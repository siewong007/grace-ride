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
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Density
import androidx.core.content.edit
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import kotlinx.coroutines.flow.MutableStateFlow
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.notifications.NotificationHelper
import org.pjstatecm.graceride.platform.findFragmentActivity
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
        val uiPrefs = newBase.applicationContext.getSharedPreferences(UI_PREFS, Context.MODE_PRIVATE)
        var lang = appContainer.lang
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            val sysLocales = newBase.getSystemService(LocaleManager::class.java)?.applicationLocales
            if (sysLocales != null && !sysLocales.isEmpty) {
                val sysLang = if (sysLocales[0]?.language == "zh") "zh" else "en"
                uiPrefs.edit { putBoolean(KEY_LOCALE_SYNCED, true) }
                if (sysLang != lang) {
                    appContainer.lang = sysLang
                    appContainer.markLangPendingSync(true)
                    lang = sysLang
                }
            } else if (uiPrefs.getBoolean(KEY_LOCALE_SYNCED, false)) {
                val cfgLocales = newBase.resources.configuration.locales
                val sysPrimary = if (cfgLocales.isEmpty) null else cfgLocales[0]
                val osLang = if (sysPrimary?.language == "zh") "zh" else "en"
                uiPrefs.edit {
                    putBoolean(KEY_LOCALE_SYNCED, false)
                    putBoolean(KEY_LANG_EXPLICIT, false)
                }
                if (osLang != lang) {
                    appContainer.lang = osLang
                    appContainer.markLangPendingSync(true)
                    lang = osLang
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
        } else {
            window?.addFlags(android.view.WindowManager.LayoutParams.FLAG_SECURE)
        }
        window?.decorView?.filterTouchesWhenObscured = true
        NotificationHelper.ensureChannels(this, container.lang)
        enableEdgeToEdge()
        if (savedInstanceState == null) {
            consumeDeepLink(intent)
        } else {
            pendingDeepLink.value = savedInstanceState.getString(STATE_PENDING_DEEP_LINK)
        }
        setContent {
            val themeModeKey by container.themeMode.collectAsStateWithLifecycle()
            val textScaleKey by container.textScale.collectAsStateWithLifecycle()
            val darkTheme = when (ThemeMode.fromKey(themeModeKey)) {
                ThemeMode.LIGHT -> false
                ThemeMode.DARK -> true
                ThemeMode.SYSTEM -> isSystemInDarkTheme()
            }
            val baseDensity = LocalDensity.current
            val scaleMultiplier = when (textScaleKey) {
                "large" -> 1.12f
                "xl" -> 1.25f
                else -> 1.0f
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
            CompositionLocalProvider(
                LocalDensity provides Density(baseDensity.density, baseDensity.fontScale * scaleMultiplier),
            ) {
                GraceRideTheme(darkTheme = darkTheme) {
                    GraceRideAppRoot(container, pendingDeepLink)
                }
            }
        }
    }

    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState)
        outState.putString(STATE_PENDING_DEEP_LINK, pendingDeepLink.value)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        consumeDeepLink(intent)
    }

    companion object {
        private const val STATE_PENDING_DEEP_LINK = "pending_deep_link"
        private const val UI_PREFS = "gr_ui_prefs"
        private const val KEY_LANG_EXPLICIT = "lang_explicitly_set"
        private const val KEY_LOCALE_SYNCED = "locale_manager_synced"
    }
}

/**
 * Applies the selected app language (`en` / `zh`): syncs system "App languages" via native
 * `LocaleManager` on Android 13+ (API 33+) and refreshes notification channels + activity config.
 */
fun applyAppLocale(context: Context, lang: String) {
    val uiPrefs = context.applicationContext.getSharedPreferences("gr_ui_prefs", Context.MODE_PRIVATE)
    uiPrefs.edit { putBoolean("lang_explicitly_set", true) }
    NotificationHelper.ensureChannels(context, lang)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
        val lm = context.getSystemService(LocaleManager::class.java)
        val target = LocaleList.forLanguageTags(lang)
        if (lm != null) {
            uiPrefs.edit { putBoolean("locale_manager_synced", true) }
            if (lm.applicationLocales != target) {
                lm.applicationLocales = target
                return
            }
        }
    }
    context.findFragmentActivity()?.recreate()
}
