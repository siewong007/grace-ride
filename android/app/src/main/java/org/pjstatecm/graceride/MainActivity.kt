package org.pjstatecm.graceride

import android.app.Activity
import android.app.LocaleManager
import android.content.Context
import android.content.Intent
import android.content.res.Configuration
import android.graphics.Color
import android.os.Build
import android.os.Bundle
import android.os.LocaleList
import androidx.activity.SystemBarStyle
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.fragment.app.FragmentActivity
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import kotlinx.coroutines.flow.MutableStateFlow
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.notifications.NotificationHelper
import org.pjstatecm.graceride.ui.GraceRideAppRoot
import org.pjstatecm.graceride.ui.NotifLinkRouter
import org.pjstatecm.graceride.ui.theme.GraceRideTheme
import org.pjstatecm.graceride.ui.theme.ThemeMode
import java.util.Locale

class MainActivity : FragmentActivity() {

    private val container: AppContainer
        get() = (application as GraceRideApp).container

    private val pendingDeepLink = MutableStateFlow<String?>(null)

    override fun attachBaseContext(newBase: Context) {
        val appContainer = (newBase.applicationContext as GraceRideApp).container
        var lang = appContainer.lang
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            val sysLocales = newBase.getSystemService(LocaleManager::class.java)?.applicationLocales
            if (sysLocales != null && !sysLocales.isEmpty) {
                val sysLang = if (sysLocales[0]?.language == "zh") "zh" else "en"
                if (sysLang != lang) {
                    appContainer.lang = sysLang
                    lang = sysLang
                }
            }
        }
        val locale = Locale.forLanguageTag(lang)
        Locale.setDefault(locale)
        val config = Configuration(newBase.resources.configuration)
        config.setLocale(locale)
        super.attachBaseContext(newBase.createConfigurationContext(config))
    }

    private fun consumeDeepLink(intent: Intent?) {
        val extraLink = intent?.getStringExtra(NotificationHelper.EXTRA_NOTIF_LINK)
        val extraToken = intent?.getStringExtra(NotificationHelper.EXTRA_NOTIF_TOKEN)
        intent?.removeExtra(NotificationHelper.EXTRA_NOTIF_LINK)
        intent?.removeExtra(NotificationHelper.EXTRA_NOTIF_TOKEN)
        if (!extraLink.isNullOrBlank() && extraToken == NotificationHelper.internalToken(this)) {
            pendingDeepLink.value = extraLink
            return
        }
        val uriStr = intent?.data?.toString() ?: return
        intent.data = null
        val extraHost = runCatching { java.net.URI(container.baseUrl).host?.lowercase() }.getOrNull()
        val extraHosts = if (extraHost != null) setOf(extraHost) else emptySet()
        NotifLinkRouter.extractPathFromIntentUri(uriStr, extraHosts)?.let { path ->
            pendingDeepLink.value = path
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        val splash = installSplashScreen()
        splash.setOnExitAnimationListener { provider -> provider.remove() }
        super.onCreate(savedInstanceState)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            setRecentsScreenshotEnabled(false)
        }
        window?.decorView?.filterTouchesWhenObscured = true
        NotificationHelper.ensureChannels(this, container.lang)
        enableEdgeToEdge()
        if (savedInstanceState == null) {
            consumeDeepLink(intent)
        }
        setContent {
            val themeModeKey by container.themeMode.collectAsStateWithLifecycle()
            val darkTheme = when (ThemeMode.fromKey(themeModeKey)) {
                ThemeMode.LIGHT -> false
                ThemeMode.DARK -> true
                ThemeMode.SYSTEM -> isSystemInDarkTheme()
            }
            LaunchedEffect(darkTheme) {
                enableEdgeToEdge(
                    statusBarStyle = if (darkTheme) {
                        SystemBarStyle.dark(Color.TRANSPARENT)
                    } else {
                        SystemBarStyle.light(Color.TRANSPARENT, Color.TRANSPARENT)
                    },
                    navigationBarStyle = if (darkTheme) {
                        SystemBarStyle.dark(Color.TRANSPARENT)
                    } else {
                        SystemBarStyle.light(Color.TRANSPARENT, Color.TRANSPARENT)
                    },
                )
            }
            GraceRideTheme(darkTheme = darkTheme) {
                GraceRideAppRoot(container, pendingDeepLink)
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        consumeDeepLink(intent)
    }
}

/**
 * Applies the selected app language (`en` / `zh`): syncs system "App languages" via native
 * `LocaleManager` on Android 13+ (API 33+) and refreshes notification channels + activity config.
 */
fun applyAppLocale(context: Context, lang: String) {
    NotificationHelper.ensureChannels(context, lang)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
        val lm = context.getSystemService(LocaleManager::class.java)
        val target = LocaleList.forLanguageTags(lang)
        if (lm != null && lm.applicationLocales != target) {
            lm.applicationLocales = target
            return
        }
    }
    (context as? Activity)?.recreate()
}
