package org.pjstatecm.graceride

import android.app.Application
import android.os.StrictMode
import coil.Coil
import coil.ImageLoader
import coil.annotation.ExperimentalCoilApi
import coil.decode.ImageDecoderDecoder
import coil.disk.DiskCache
import coil.memory.MemoryCache
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

    @OptIn(ExperimentalCoilApi::class)
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
        // share the session OkHttp client with Coil.
        val imageLoader = ImageLoader.Builder(this)
            .okHttpClient(container.okHttp)
            .components { add(ImageDecoderDecoder.Factory()) }
            .memoryCache {
                MemoryCache.Builder(this)
                    .maxSizePercent(0.20)
                    .build()
            }
            .diskCache {
                DiskCache.Builder()
                    .directory(File(cacheDir, "image_cache"))
                    .maxSizeBytes(32L * 1024L * 1024L)
                    .build()
            }
            .crossfade(true)
            .build()
        Coil.setImageLoader(imageLoader)
        container.onSessionCleared = {
            imageLoader.memoryCache?.clear()
            container.appScope.launch {
                runCatching { imageLoader.diskCache?.clear() }
                runCatching { container.okHttp.cache?.evictAll() }
            }
            runCatching { BackgroundNotificationWorker.cancel(this) }
        }
        container.onSessionLocked = {
            imageLoader.memoryCache?.clear()
            container.appScope.launch {
                runCatching { imageLoader.diskCache?.clear() }
                runCatching { container.okHttp.cache?.evictAll() }
            }
        }
    }
}
