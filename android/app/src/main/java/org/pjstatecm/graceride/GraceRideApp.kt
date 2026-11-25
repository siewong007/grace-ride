package org.pjstatecm.graceride

import android.app.Application
import coil.Coil
import coil.ImageLoader
import coil.annotation.ExperimentalCoilApi
import coil.decode.ImageDecoderDecoder
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.notifications.NotificationHelper

class GraceRideApp : Application() {
    lateinit var container: AppContainer
        private set

    @OptIn(ExperimentalCoilApi::class)
    override fun onCreate() {
        super.onCreate()
        container = AppContainer(this)
        NotificationHelper.ensureChannels(this, container.lang)
        container.realtimeStream.start()
        // Images (avatars, DuitNow QRs, receipts) are cookie-authenticated:
        // share the session OkHttp client with Coil.
        val imageLoader = ImageLoader.Builder(this)
            .okHttpClient(container.okHttp)
            .components { add(ImageDecoderDecoder.Factory()) }
            .crossfade(true)
            .build()
        Coil.setImageLoader(imageLoader)
        container.onSessionCleared = {
            imageLoader.memoryCache?.clear()
            runCatching { imageLoader.diskCache?.clear() }
        }
    }
}
