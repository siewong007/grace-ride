package org.pjstatecm.graceride

import android.app.Application
import coil.Coil
import coil.ImageLoader
import coil.decode.ImageDecoderDecoder
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.notifications.NotificationHelper

class GraceRideApp : Application() {
    lateinit var container: AppContainer
        private set

    override fun onCreate() {
        super.onCreate()
        NotificationHelper.ensureChannels(this)
        container = AppContainer(this)
        container.realtimeStream.start()
        // Images (avatars, DuitNow QRs, receipts) are cookie-authenticated:
        // share the session OkHttp client with Coil.
        Coil.setImageLoader(
            ImageLoader.Builder(this)
                .okHttpClient(container.okHttp)
                .components { add(ImageDecoderDecoder.Factory()) }
                .crossfade(true)
                .build()
        )
    }
}
