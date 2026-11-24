package org.pjstatecm.graceride

import android.content.Context
import android.content.Intent
import android.content.res.Configuration
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import kotlinx.coroutines.flow.MutableStateFlow
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.notifications.NotificationHelper
import org.pjstatecm.graceride.ui.GraceRideAppRoot
import org.pjstatecm.graceride.ui.theme.GraceRideTheme
import java.util.Locale

class MainActivity : ComponentActivity() {

    private val container: AppContainer
        get() = (application as GraceRideApp).container

    private val pendingDeepLink = MutableStateFlow<String?>(null)

    override fun attachBaseContext(newBase: Context) {
        // Per-app language (zh/en), independent of the device locale on API < 33.
        val lang = (newBase.applicationContext as GraceRideApp).container.lang
        val locale = Locale(lang)
        Locale.setDefault(locale)
        val config = Configuration(newBase.resources.configuration)
        config.setLocale(locale)
        super.attachBaseContext(newBase.createConfigurationContext(config))
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        installSplashScreen()
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        intent?.getStringExtra(NotificationHelper.EXTRA_NOTIF_LINK)?.let {
            pendingDeepLink.value = it
        }
        setContent {
            GraceRideTheme {
                GraceRideAppRoot(container, pendingDeepLink)
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        intent.getStringExtra(NotificationHelper.EXTRA_NOTIF_LINK)?.let {
            pendingDeepLink.value = it
        }
    }
}
