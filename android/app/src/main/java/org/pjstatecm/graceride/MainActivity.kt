package org.pjstatecm.graceride

import android.content.Context
import android.content.res.Configuration
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.GraceRideAppRoot
import org.pjstatecm.graceride.ui.theme.GraceRideTheme
import java.util.Locale

class MainActivity : ComponentActivity() {

    private val container: AppContainer
        get() = (application as GraceRideApp).container

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
        setContent {
            GraceRideTheme {
                GraceRideAppRoot(container)
            }
        }
    }
}
