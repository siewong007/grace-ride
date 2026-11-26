package org.pjstatecm.graceride

import android.app.Application
import android.os.StrictMode
import coil.Coil
import coil.ImageLoader
import coil.decode.ImageDecoderDecoder
import coil.memory.MemoryCache
import coil.request.CachePolicy
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.ProcessLifecycleOwner
import java.io.File
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.notifications.BackgroundNotificationWorker
import org.pjstatecm.graceride.notifications.NotificationHelper

class GraceRideApp : Application() {
    lateinit var container: AppContainer
        private set

    override fun onCreate() {
        super.onCreate()
        if (BuildConfig.DEBUG) {
            StrictMode.setThreadPolicy(
                StrictMode.ThreadPolicy.Builder()
                    .detectNetwork()
                    .penaltyLog()
                    .build(),
            )
        }
        container = AppContainer(this)
        NotificationHelper.ensureChannels(this, container.lang)
        container.appScope.launch { container.warmUpStorage() }
        container.realtimeStream.start()
        ProcessLifecycleOwner.get().lifecycle.addObserver(
            LifecycleEventObserver { _, event ->
                when (event) {
                    Lifecycle.Event.ON_START -> container.biometric.onAppForegrounded()
                    Lifecycle.Event.ON_STOP -> container.biometric.onAppBackgrounded()
                    else -> {}
                }
            },
        )
        // Images (avatars, DuitNow QRs, receipts) are cookie-authenticated:
        // keep only an in-memory cache (no plaintext disk image_cache) and purge legacy files.
        container.appScope.launch {
            runCatching { File(cacheDir, "image_cache").deleteRecursively() }
        }
        val imageLoader = ImageLoader.Builder(this)
            .okHttpClient(container.okHttp.newBuilder().cache(null).build())
            .components { add(ImageDecoderDecoder.Factory()) }
            .memoryCache {
                MemoryCache.Builder(this)
                    .maxSizePercent(0.20)
                    .build()
            }
            .diskCachePolicy(CachePolicy.DISABLED)
            .crossfade(true)
            .build()
        Coil.setImageLoader(imageLoader)
        container.onSessionCleared = {
            imageLoader.memoryCache?.clear()
            container.appScope.launch {
                runCatching { File(cacheDir, "image_cache").deleteRecursively() }
                runCatching { container.okHttp.cache?.evictAll() }
            }
            runCatching { BackgroundNotificationWorker.cancel(this) }
        }
        container.onSessionLocked = {
            imageLoader.memoryCache?.clear()
            container.appScope.launch {
                runCatching { File(cacheDir, "image_cache").deleteRecursively() }
                runCatching { container.okHttp.cache?.evictAll() }
            }
        }
    }
}
