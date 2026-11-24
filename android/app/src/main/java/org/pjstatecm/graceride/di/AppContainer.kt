package org.pjstatecm.graceride.di

import android.content.Context
import okhttp3.Interceptor
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import org.pjstatecm.graceride.BuildConfig
import org.pjstatecm.graceride.data.api.ApiService
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.repo.AuthRepository
import org.pjstatecm.graceride.data.repo.CommunityRepository
import org.pjstatecm.graceride.data.repo.EventsRepository
import org.pjstatecm.graceride.data.repo.RidesRepository
import org.pjstatecm.graceride.data.repo.SharesRepository
import org.pjstatecm.graceride.data.session.EncryptedPrefsStore
import org.pjstatecm.graceride.data.session.PersistentCookieJar
import org.pjstatecm.graceride.data.session.SecureStore
import org.pjstatecm.graceride.notifications.RealtimeStreamClient
import retrofit2.Retrofit
import com.jakewharton.retrofit2.converter.kotlinx.serialization.asConverterFactory
import java.util.Locale
import java.util.concurrent.TimeUnit

/**
 * Manual dependency container (application-scoped singletons). Keeps the graph
 * explicit and unit-testable without annotation processing.
 */
class AppContainer(context: Context?, storeOverride: SecureStore? = null) {

    private val appContext: Context? = context?.applicationContext
    val secureStore: SecureStore = storeOverride ?: EncryptedPrefsStore(context!!.applicationContext)
    val cookieJar = PersistentCookieJar(secureStore)
    val authEvents = AuthEventBus()

    /** UI + Accept-Language language: "zh" or "en". Defaults to the device locale. */
    var lang: String
        get() = secureStore.getString("lang")
            ?: if (Locale.getDefault().language == "zh") "zh" else "en"
        set(value) { secureStore.putString("lang", value) }

    /** Debug builds may point at a LAN dev server; release is fixed to HTTPS production. */
    var baseUrl: String = secureStore.getString("base_url_override")
        ?.takeIf { BuildConfig.DEBUG }
        ?: BuildConfig.API_BASE_URL
        private set

    fun setBaseUrl(url: String) {
        val normalized = if (url.endsWith("/")) url else "$url/"
        baseUrl = normalized
        if (BuildConfig.DEBUG) secureStore.putString("base_url_override", normalized)
        rebuildApi()
    }

    val okHttp: OkHttpClient = OkHttpClient.Builder()
        .cookieJar(cookieJar)
        // No Origin header: the backend's origin guard admits non-browser clients.
        .addInterceptor(Interceptor { chain ->
            chain.proceed(
                chain.request().newBuilder()
                    .header("Accept-Language", lang)
                    .build()
            )
        })
        .connectTimeout(20, TimeUnit.SECONDS)
        .readTimeout(40, TimeUnit.SECONDS)
        .writeTimeout(60, TimeUnit.SECONDS)
        .retryOnConnectionFailure(true)
        .build()

    @Volatile
    var api: ApiService = buildApi()
        private set

    private fun buildApi(): ApiService =
        Retrofit.Builder()
            .baseUrl(baseUrl)
            .client(okHttp)
            .addConverterFactory(apiJson.asConverterFactory("application/json".toMediaType()))
            .build()
            .create(ApiService::class.java)

    // Repositories are rebuilt with the API when the base URL changes (debug
    // server override); holding stale Retrofit instances would keep the old host.
    var auth: AuthRepository = AuthRepository(api, authEvents, cookieJar)
        private set
    var rides: RidesRepository = RidesRepository(api, authEvents)
        private set
    var events: EventsRepository = EventsRepository(api, authEvents)
        private set
    var shares: SharesRepository = SharesRepository(api, authEvents)
        private set
    var community: CommunityRepository = CommunityRepository(api, authEvents)
        private set
    var realtimeStream: RealtimeStreamClient =
        RealtimeStreamClient(okHttp, { baseUrl }, auth, community, appContext)
        private set

    private fun rebuildApi() {
        realtimeStream.stop()
        api = buildApi()
        auth = AuthRepository(api, authEvents, cookieJar)
        rides = RidesRepository(api, authEvents)
        events = EventsRepository(api, authEvents)
        shares = SharesRepository(api, authEvents)
        community = CommunityRepository(api, authEvents)
        realtimeStream = RealtimeStreamClient(okHttp, { baseUrl }, auth, community, appContext)
        if (appContext != null) {
            realtimeStream.start()
        }
    }

    fun fileUrl(fileId: Long): String = baseUrl + "api/files/" + fileId

    fun webUrl(path: String): String = baseUrl.trimEnd('/') + "/" + path.trimStart('/')
}
