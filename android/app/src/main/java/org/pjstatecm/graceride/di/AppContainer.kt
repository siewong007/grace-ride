package org.pjstatecm.graceride.di

import android.content.Context
import android.content.SharedPreferences
import androidx.core.content.edit
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.launch
import okhttp3.Cache
import okhttp3.HttpUrl.Companion.toHttpUrlOrNull
import okhttp3.Interceptor
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import org.pjstatecm.graceride.BuildConfig
import org.pjstatecm.graceride.data.api.ApiProvider
import org.pjstatecm.graceride.data.api.ApiService
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.api.runSuspendCatching
import org.pjstatecm.graceride.data.cache.OfflineActionQueue
import org.pjstatecm.graceride.data.cache.OfflineCache
import org.pjstatecm.graceride.data.repo.AdminRepository
import org.pjstatecm.graceride.data.repo.AuthRepository
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.data.repo.CommunityRepository
import org.pjstatecm.graceride.data.repo.EventsRepository
import org.pjstatecm.graceride.data.repo.FinanceRepository
import org.pjstatecm.graceride.data.repo.HelpRepository
import org.pjstatecm.graceride.data.repo.PublicFlowRepository
import org.pjstatecm.graceride.data.repo.RidesRepository
import org.pjstatecm.graceride.data.repo.SharesRepository
import org.pjstatecm.graceride.data.repo.TrafficRepository
import org.pjstatecm.graceride.data.session.EncryptedPrefsStore
import org.pjstatecm.graceride.data.session.PersistentCookieJar
import org.pjstatecm.graceride.data.session.SecureStore
import org.pjstatecm.graceride.notifications.RealtimeStreamClient
import org.pjstatecm.graceride.platform.AndroidBiometricCryptoDriver
import org.pjstatecm.graceride.platform.BiometricAuthManager
import org.pjstatecm.graceride.platform.BiometricCryptoDriver
import org.pjstatecm.graceride.platform.NetworkMonitor
import retrofit2.Retrofit
import com.jakewharton.retrofit2.converter.kotlinx.serialization.asConverterFactory
import java.io.File
import java.util.Locale
import java.util.concurrent.TimeUnit

/**
 * Manual dependency container (application-scoped singletons). Keeps the graph
 * explicit and unit-testable without annotation processing.
 */
class AppContainer(
    context: Context?,
    private val storeOverride: SecureStore? = null,
    biometricDriverOverride: BiometricCryptoDriver? = null,
) {

    internal val appContext: Context? = context?.applicationContext
    private val uiPrefs: SharedPreferences? =
        appContext?.getSharedPreferences("gr_ui_prefs", Context.MODE_PRIVATE)
    val appScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    val secureStore: SecureStore = storeOverride ?: EncryptedPrefsStore(context!!.applicationContext)
    val biometric: BiometricAuthManager = BiometricAuthManager(
        store = secureStore,
        driver = biometricDriverOverride ?: AndroidBiometricCryptoDriver(),
    )
    val cookieJar = PersistentCookieJar(secureStore)
    val authEvents = AuthEventBus()

    @Volatile
    var launchSplashShown: Boolean = false

    @Volatile
    private var cachedLang: String =
        uiPrefs?.getString("lang", null)
            ?: storeOverride?.getString("lang")
            ?: if (Locale.getDefault().language == "zh") "zh" else "en"

    private val _langFlow = MutableStateFlow(cachedLang)
    val langFlow: StateFlow<String> = _langFlow.asStateFlow()

    /** UI + Accept-Language language: "zh" or "en". Defaults to the device locale. */
    var lang: String
        get() = cachedLang
        set(value) {
            cachedLang = value
            _langFlow.value = value
            if (uiPrefs != null) {
                uiPrefs.edit { putString("lang", value) }
            } else {
                secureStore.putString("lang", value)
            }
        }

    fun isLangPendingSync(): Boolean =
        uiPrefs?.getBoolean(KEY_LANG_PENDING_SYNC, false)
            ?: (secureStore.getString(KEY_LANG_PENDING_SYNC) == "1")

    fun markLangPendingSync(pending: Boolean) {
        if (uiPrefs != null) {
            uiPrefs.edit { putBoolean(KEY_LANG_PENDING_SYNC, pending) }
        } else {
            if (pending) secureStore.putString(KEY_LANG_PENDING_SYNC, "1")
            else secureStore.remove(KEY_LANG_PENDING_SYNC)
        }
    }

    private val _themeMode = MutableStateFlow(
        (uiPrefs?.getString("theme_mode", null) ?: storeOverride?.getString("theme_mode"))
            ?.takeIf { it in THEME_MODES }
            ?: "system",
    )
    val themeMode: StateFlow<String> = _themeMode.asStateFlow()

    /** Persist the user's preferred appearance ("system", "light", or "dark"). */
    fun setThemeMode(mode: String) {
        val normalized = mode.takeIf { it in THEME_MODES } ?: "system"
        if (uiPrefs != null) {
            uiPrefs.edit { putString("theme_mode", normalized) }
        } else {
            secureStore.putString("theme_mode", normalized)
        }
        _themeMode.value = normalized
    }

    private val _textScale = MutableStateFlow(
        (uiPrefs?.getString("text_scale", null) ?: storeOverride?.getString("text_scale"))
            ?.takeIf { it in TEXT_SCALE_MODES }
            ?: "normal",
    )
    val textScale: StateFlow<String> = _textScale.asStateFlow()

    /** Persist the user's preferred in-app text size ("normal", "large", or "xl"). */
    fun setTextScale(mode: String) {
        val normalized = mode.takeIf { it in TEXT_SCALE_MODES } ?: "normal"
        if (uiPrefs != null) {
            uiPrefs.edit { putString("text_scale", normalized) }
        } else {
            secureStore.putString("text_scale", normalized)
        }
        _textScale.value = normalized
    }

    private val _navAppPref = MutableStateFlow(
        (uiPrefs?.getString("nav_app_pref", null) ?: storeOverride?.getString("nav_app_pref"))
            ?.takeIf { it in NAV_APP_MODES }
            ?: "ask",
    )
    val navAppPref: StateFlow<String> = _navAppPref.asStateFlow()

    /** Persist the driver's preferred turn-by-turn navigation app ("ask", "waze", or "google"). */
    fun setNavAppPref(mode: String) {
        val normalized = mode.takeIf { it in NAV_APP_MODES } ?: "ask"
        if (uiPrefs != null) {
            uiPrefs.edit { putString("nav_app_pref", normalized) }
        } else {
            secureStore.putString("nav_app_pref", normalized)
        }
        _navAppPref.value = normalized
    }

    private val _autoArriveGate = MutableStateFlow(
        uiPrefs?.let { prefs ->
            if (prefs.contains("auto_arrive_gate")) prefs.getBoolean("auto_arrive_gate", true) else true
        } ?: (storeOverride?.getString("auto_arrive_gate") != "0"),
    )
    val autoArriveGate: StateFlow<Boolean> = _autoArriveGate.asStateFlow()

    /** Toggle automatic gate-arrival alert within ~80 m during active driver trips. */
    fun setAutoArriveGate(enabled: Boolean) {
        if (uiPrefs != null) {
            uiPrefs.edit { putBoolean("auto_arrive_gate", enabled) }
        } else {
            secureStore.putString("auto_arrive_gate", if (enabled) "1" else "0")
        }
        _autoArriveGate.value = enabled
    }

    /** Debug builds may point at a LAN dev server; release is fixed to HTTPS production. */
    @Volatile
    var baseUrl: String = (uiPrefs?.getString("base_url_override", null) ?: storeOverride?.getString("base_url_override"))
        ?.takeIf { BuildConfig.DEBUG }
        ?: BuildConfig.API_BASE_URL
        private set

    fun setBaseUrl(url: String, clearSessionIfChanged: Boolean = false) {
        require(BuildConfig.DEBUG) { "Server URL override is only available in debug builds" }
        val trimmed = url.trim()
        val parsed = trimmed.toHttpUrlOrNull()
        require(parsed != null && parsed.scheme in setOf("http", "https") && parsed.host.isNotBlank()) {
            "Invalid server URL"
        }
        val normalized = if (trimmed.endsWith("/")) trimmed else "$trimmed/"
        val changed = normalized != baseUrl
        baseUrl = normalized
        if (BuildConfig.DEBUG) {
            if (uiPrefs != null) {
                uiPrefs.edit { putString("base_url_override", normalized) }
            } else {
                secureStore.putString("base_url_override", normalized)
            }
        }
        if (clearSessionIfChanged && changed) {
            val hasSession = auth.me != null ||
                cookieJar.hasCookies() ||
                secureStore.getString("cached_me_v1") != null
            if (hasSession) {
                auth.onUnauthorized()
                runCatching { okHttp.cache?.evictAll() }
            }
        }
        rebuildApi()
    }

    /** Warms up Keystore-backed storage and migrates legacy non-secret UI prefs off the main thread. */
    fun warmUpStorage() {
        (secureStore as? EncryptedPrefsStore)?.warmUp()
        biometric.ensureLoaded()
        cookieJar.hasCookies()
        uiPrefs?.let { prefs ->
            if (!prefs.contains("lang")) {
                secureStore.getString("lang")?.let { saved ->
                    prefs.edit { putString("lang", saved) }
                    cachedLang = saved
                    _langFlow.value = saved
                    appContext?.let { ctx ->
                        org.pjstatecm.graceride.notifications.NotificationHelper.ensureChannels(ctx, saved)
                    }
                }
            }
            if (!prefs.contains("theme_mode")) {
                secureStore.getString("theme_mode")?.takeIf { it in THEME_MODES }?.let { saved ->
                    prefs.edit { putString("theme_mode", saved) }
                    _themeMode.value = saved
                }
            }
            if (BuildConfig.DEBUG && !prefs.contains("base_url_override")) {
                secureStore.getString("base_url_override")?.let { saved ->
                    val parsed = saved.trim().toHttpUrlOrNull()
                    if (parsed != null && parsed.scheme in setOf("http", "https") && parsed.host.isNotBlank()) {
                        val normalized = if (saved.trim().endsWith("/")) saved.trim() else "${saved.trim()}/"
                        prefs.edit { putString("base_url_override", normalized) }
                        if (normalized != baseUrl) {
                            baseUrl = normalized
                            rebuildApi()
                        }
                    }
                }
            }
        }
    }

    val okHttp: OkHttpClient = OkHttpClient.Builder()
        .cookieJar(cookieJar)
        .apply {
            appContext?.let { cache(Cache(File(it.cacheDir, "http_cache"), HTTP_CACHE_BYTES)) }
        }
        // No Origin header: the backend's origin guard admits non-browser clients.
        .addInterceptor(Interceptor { chain ->
            chain.proceed(
                chain.request().newBuilder()
                    .header("Accept-Language", cachedLang)
                    .build()
            )
        })
        .connectTimeout(20, TimeUnit.SECONDS)
        .readTimeout(40, TimeUnit.SECONDS)
        .writeTimeout(60, TimeUnit.SECONDS)
        .retryOnConnectionFailure(true)
        .build()

    val apiProvider: ApiProvider = ApiProvider(buildApi())

    val api: ApiService
        get() = apiProvider.get()

    val networkMonitor: NetworkMonitor = NetworkMonitor(appContext)
    val offlineCache: OfflineCache = OfflineCache(appContext)
    val offlineQueue: OfflineActionQueue = OfflineActionQueue(secureStore)

    private fun buildApi(): ApiService =
        Retrofit.Builder()
            .baseUrl(baseUrl)
            .client(okHttp)
            .addConverterFactory(apiJson.asConverterFactory("application/json".toMediaType()))
            .build()
            .create(ApiService::class.java)

    private val sharedAuthState = MutableStateFlow<AuthState>(AuthState.Loading)

    /** Callback wired by [org.pjstatecm.graceride.GraceRideApp] to evict image caches on sign-out. */
    @Volatile
    var onSessionCleared: (() -> Unit)? = null
    @Volatile
    var onSessionLocked: (() -> Unit)? = null

    val rides: RidesRepository = RidesRepository(apiProvider, authEvents)
    val events: EventsRepository = EventsRepository(apiProvider, authEvents)
    val shares: SharesRepository = SharesRepository(apiProvider, authEvents)
    val community: CommunityRepository = CommunityRepository(apiProvider, authEvents)
    val traffic: TrafficRepository = TrafficRepository(apiProvider, authEvents)
    val admin: AdminRepository = AdminRepository(apiProvider, authEvents)
    val finance: FinanceRepository = FinanceRepository(apiProvider, authEvents)
    val help: HelpRepository = HelpRepository(apiProvider, authEvents)
    val publicFlow: PublicFlowRepository = PublicFlowRepository(apiProvider)
    val auth: AuthRepository = AuthRepository(
        apiProvider,
        authEvents,
        cookieJar,
        sharedAuthState,
        store = secureStore,
        onCleared = {
            rides.clearAreasCache()
            offlineQueue.clear()
            appScope.launch {
                offlineCache.clearAll()
                appContext?.let { ctx ->
                    runCatching { androidx.core.app.NotificationManagerCompat.from(ctx).cancelAll() }
                    runCatching {
                        org.pjstatecm.graceride.service.LiveTripLocationService.stop(ctx, notifyServer = false)
                    }
                    runCatching {
                        androidx.core.content.pm.ShortcutManagerCompat.removeDynamicShortcuts(ctx, listOf("drive_hub"))
                    }
                    org.pjstatecm.graceride.widget.WidgetSnapshotStore.clear(ctx)
                    org.pjstatecm.graceride.widget.refreshGraceRideWidget(ctx)
                }
            }
            onSessionCleared?.invoke()
        },
        biometric = biometric,
        onLockedCallback = {
            rides.clearAreasCache()
            appScope.launch {
                offlineCache.clearAll()
                appContext?.let { ctx ->
                    runCatching { androidx.core.app.NotificationManagerCompat.from(ctx).cancelAll() }
                    runCatching {
                        org.pjstatecm.graceride.service.LiveTripLocationService.stop(ctx, notifyServer = false)
                    }
                    runCatching {
                        androidx.core.content.pm.ShortcutManagerCompat.removeDynamicShortcuts(ctx, listOf("drive_hub"))
                    }
                    org.pjstatecm.graceride.widget.WidgetSnapshotStore.clear(ctx)
                    org.pjstatecm.graceride.widget.refreshGraceRideWidget(ctx)
                }
            }
            onSessionLocked?.invoke()
        },
        onUserSwitched = { prevId ->
            rides.clearAreasCache()
            offlineQueue.clear()
            offlineCache.clearUser(prevId)
            appContext?.let { ctx ->
                runCatching { androidx.core.app.NotificationManagerCompat.from(ctx).cancelAll() }
                runCatching {
                    org.pjstatecm.graceride.service.LiveTripLocationService.stop(ctx, notifyServer = false)
                }
                runCatching {
                    androidx.core.content.pm.ShortcutManagerCompat.removeDynamicShortcuts(ctx, listOf("drive_hub"))
                }
                org.pjstatecm.graceride.widget.WidgetSnapshotStore.clear(ctx)
                appScope.launch { org.pjstatecm.graceride.widget.refreshGraceRideWidget(ctx) }
            }
            onSessionCleared?.invoke()
        },
    )

    init {
        appScope.launch {
            authEvents.events.collect { event ->
                when (event) {
                    org.pjstatecm.graceride.data.api.AuthEvent.Unauthorized -> {
                        appContext?.let {
                            runCatching {
                                org.pjstatecm.graceride.service.LiveTripLocationService.stop(it, notifyServer = false)
                            }
                        }
                        auth.onUnauthorized()
                    }
                    org.pjstatecm.graceride.data.api.AuthEvent.Locked -> {
                        appContext?.let {
                            runCatching {
                                org.pjstatecm.graceride.service.LiveTripLocationService.stop(it, notifyServer = false)
                            }
                        }
                        auth.onLocked()
                    }
                    org.pjstatecm.graceride.data.api.AuthEvent.Pending -> auth.onPending()
                }
            }
        }
        appScope.launch {
            networkMonitor.isOnline.drop(1).filter { it }.collect {
                if (auth.state.value is AuthState.SignedIn && offlineQueue.pending.value.isNotEmpty()) {
                    runSuspendCatching { offlineQueue.flush(rides) }
                }
            }
        }
    }
    val realtimeStream: RealtimeStreamClient = RealtimeStreamClient(
        okHttp,
        { baseUrl },
        auth,
        community,
        appContext,
        authEvents = authEvents,
        langProvider = { cachedLang },
        hideLockDetailsProvider = { secureStore.getString("notif_hide_lock_details") == "1" },
        store = secureStore,
        biometric = biometric,
    )

    private fun rebuildApi() {
        realtimeStream.stop()
        apiProvider.swap(buildApi())
        rides.clearAreasCache()
        if (appContext != null) {
            realtimeStream.start()
        }
    }

    fun fileUrl(fileId: Long): String = baseUrl + "api/files/" + fileId

    fun webUrl(path: String): String = baseUrl.trimEnd('/') + "/" + path.trimStart('/')

    companion object {
        private val THEME_MODES = setOf("system", "light", "dark")
        private val TEXT_SCALE_MODES = setOf("normal", "large", "xl")
        private val NAV_APP_MODES = setOf("ask", "waze", "google")
        private const val HTTP_CACHE_BYTES = 10L * 1024L * 1024L
        private const val KEY_LANG_PENDING_SYNC = "lang_pending_sync"
    }
}
