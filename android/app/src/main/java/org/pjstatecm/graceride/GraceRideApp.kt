package org.pjstatecm.graceride

import android.app.Application
import coil.Coil
import coil.ImageLoader
import org.pjstatecm.graceride.di.AppContainer

class GraceRideApp : Application() {
    lateinit var container: AppContainer
        private set

    override fun onCreate() {
        super.onCreate()
        container = AppContainer(this)
        // Images (avatars, DuitNow QRs, receipts) are cookie-authenticated:
        // share the session OkHttp client with Coil.
        Coil.setImageLoader(
            ImageLoader.Builder(this)
                .okHttpClient(container.okHttp)
                .crossfade(true)
                .build()
        )
    }
}
