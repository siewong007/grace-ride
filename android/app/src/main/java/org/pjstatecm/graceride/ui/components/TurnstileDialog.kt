package org.pjstatecm.graceride.ui.components

import android.annotation.SuppressLint
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.net.toUri
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R

private val ALLOWED_TURNSTILE_ACTIONS = setOf(
    "passkey_login",
    "biometric_login",
    "recovery_request",
    "invite_accept",
    "visitor_claim",
)

/**
 * Reusable Cloudflare Turnstile challenge dialog shared by Login, Account Recovery,
 * Invite acceptance, and Visitor Pass claiming.
 */
@SuppressLint("SetJavaScriptEnabled")
@Composable
fun TurnstileDialog(
    siteKey: String,
    baseUrl: String,
    action: String,
    onToken: (String) -> Unit,
    onDismiss: () -> Unit,
) {
    val safeAction = if (action in ALLOWED_TURNSTILE_ACTIONS) action else "passkey_login"
    val locale = LocalConfiguration.current.locales[0]
    val bg = MaterialTheme.colorScheme.background
    val darkTheme = (0.2126f * bg.red + 0.7152f * bg.green + 0.0722f * bg.blue) < 0.5f
    val cfTheme = if (darkTheme) "dark" else "light"
    val cfLang = remember(locale) {
        if (locale?.language?.startsWith("zh", ignoreCase = true) == true) "zh-cn" else "en"
    }
    var loadError by remember { mutableStateOf(false) }
    val webViewHolder = remember { arrayOfNulls<WebView>(1) }
    val scope = rememberCoroutineScope()
    DisposableEffect(loadError) {
        onDispose {
            webViewHolder[0]?.destroy()
            webViewHolder[0] = null
        }
    }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.login_turnstile_title)) },
        text = {
            if (loadError) {
                Text(
                    stringResource(R.string.login_turnstile_error),
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodyMedium,
                )
            } else {
                AndroidView(
                    modifier = Modifier.fillMaxWidth().height(120.dp),
                    factory = { ctx ->
                        WebView(ctx).apply {
                            setBackgroundColor(android.graphics.Color.TRANSPARENT)
                            val baseOrigin = runCatching {
                                val parsed = baseUrl.toUri()
                                val scheme = parsed.scheme?.lowercase()
                                val host = parsed.host?.lowercase()
                                if ((scheme == "https" || scheme == "http") && !host.isNullOrBlank()) {
                                    scheme to host
                                } else {
                                    null
                                }
                            }.getOrNull()
                            val safeBaseUrl = if (baseOrigin != null) baseUrl else "https://graceride.saliminn.my/"
                            settings.javaScriptEnabled = true
                            settings.domStorageEnabled = true
                            settings.allowFileAccess = false
                            settings.allowContentAccess = false
                            settings.javaScriptCanOpenWindowsAutomatically = false
                            settings.setSupportMultipleWindows(false)
                            webViewClient = object : android.webkit.WebViewClient() {
                                override fun shouldOverrideUrlLoading(
                                    view: WebView?,
                                    request: android.webkit.WebResourceRequest?,
                                ): Boolean {
                                    val u = request?.url ?: return true
                                    val scheme = u.scheme?.lowercase()
                                    val host = u.host?.lowercase()
                                    val isCloudflare = scheme == "https" && host == "challenges.cloudflare.com"
                                    val isBase = baseOrigin != null && scheme == baseOrigin.first && host == baseOrigin.second
                                    return !(isCloudflare || isBase)
                                }

                                override fun onReceivedError(
                                    view: WebView?,
                                    request: android.webkit.WebResourceRequest?,
                                    error: android.webkit.WebResourceError?,
                                ) {
                                    if (request?.isForMainFrame == true || request?.url?.host == "challenges.cloudflare.com") {
                                        scope.launch(Dispatchers.Main.immediate) { loadError = true }
                                    }
                                }
                            }
                            addJavascriptInterface(
                                object {
                                    @JavascriptInterface
                                    fun onToken(token: String) {
                                        scope.launch(Dispatchers.Main.immediate) { onToken(token) }
                                    }

                                    @JavascriptInterface
                                    fun onError(@Suppress("UNUSED_PARAMETER") reason: String?) {
                                        scope.launch(Dispatchers.Main.immediate) { loadError = true }
                                    }
                                },
                                "GraceTurnstile",
                            )
                            val html = """
                                <!doctype html>
                                <html>
                                <head>
                                  <meta name="viewport" content="width=device-width,initial-scale=1"/>
                                  <script>
                                    var loadTimer = setTimeout(function() {
                                      if (!window.turnstile && window.GraceTurnstile) {
                                        window.GraceTurnstile.onError('load_timeout');
                                      }
                                    }, 10000);
                                    function onLoadTurnstile() {
                                      clearTimeout(loadTimer);
                                      if (!window.turnstile) {
                                        if (window.GraceTurnstile) window.GraceTurnstile.onError('missing_turnstile');
                                        return;
                                      }
                                      window.turnstile.render('#cf-turnstile', {
                                        sitekey: '$siteKey',
                                        action: '$safeAction',
                                        theme: '$cfTheme',
                                        language: '$cfLang',
                                        callback: function(token) {
                                          if (window.GraceTurnstile && token) {
                                            window.GraceTurnstile.onToken(token);
                                          }
                                        },
                                        'error-callback': function(code) {
                                          if (window.GraceTurnstile) window.GraceTurnstile.onError(String(code || 'error'));
                                        },
                                        'expired-callback': function() {
                                          if (window.GraceTurnstile) window.GraceTurnstile.onError('expired');
                                        },
                                        'timeout-callback': function() {
                                          if (window.GraceTurnstile) window.GraceTurnstile.onError('timeout');
                                        }
                                      });
                                    }
                                  </script>
                                  <script src="https://challenges.cloudflare.com/turnstile/v0/api.js?onload=onLoadTurnstile&render=explicit" async defer onerror="clearTimeout(loadTimer);if(window.GraceTurnstile){window.GraceTurnstile.onError('script_error');}"></script>
                                </head>
                                <body style="margin:0;background:transparent;display:flex;justify-content:center;align-items:center;min-height:96px;">
                                  <div id="cf-turnstile"></div>
                                </body>
                                </html>
                            """.trimIndent()
                            loadDataWithBaseURL(safeBaseUrl, html, "text/html", "UTF-8", null)
                            webViewHolder[0] = this
                        }
                    },
                )
            }
        },
        confirmButton = {
            if (loadError) {
                TextButton(onClick = { loadError = false }) {
                    Text(stringResource(R.string.common_retry))
                }
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text(stringResource(R.string.common_cancel))
            }
        },
    )
}
