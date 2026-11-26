package org.pjstatecm.graceride.ui.screens.login

import android.annotation.SuppressLint
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.Email
import androidx.compose.material.icons.outlined.Fingerprint
import androidx.compose.material.icons.outlined.Key
import androidx.compose.material.icons.outlined.WarningAmber
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.BrandMarkMode
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.GraceRideBrandMark
import org.pjstatecm.graceride.ui.components.GraceRideWordmark
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors

@Composable
fun LoginScreen(container: AppContainer) {
    val vm: LoginViewModel = viewModel(factory = LoginViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val biometricEnrollment by vm.biometricEnrollment.collectAsStateWithLifecycle()
    val keyInvalidatedNotice by vm.keyInvalidatedNotice.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val view = LocalView.current
    var serverField by remember { mutableStateOf(state.serverUrl) }

    LaunchedEffect(Unit) { vm.loadTurnstileConfig() }

    Column(
        Modifier
            .fillMaxSize()
            .statusBarsPadding()
            .navigationBarsPadding()
            .imePadding()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = Spacing.md, vertical = Spacing.md),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        GraceRideBrandMark(
            modifier = Modifier.size(104.dp),
            mode = BrandMarkMode.IDLE_HOP,
        )
        GraceRideWordmark(
            fontSize = 32.sp,
            modifier = Modifier.padding(top = Spacing.xs),
        )
        Text(
            stringResource(R.string.login_subtitle),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(top = Spacing.xs, bottom = Spacing.md),
        )

        OutlinedCard(Modifier.fillMaxWidth()) {
            Column(
                Modifier.padding(Spacing.sm),
                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
            ) {
                if (keyInvalidatedNotice) {
                    CalloutBanner(
                        icon = Icons.Outlined.WarningAmber,
                        text = stringResource(R.string.biometric_invalidated_notice),
                        containerColor = MaterialTheme.colorScheme.errorContainer,
                        contentColor = MaterialTheme.colorScheme.onErrorContainer,
                    )
                }
                biometricEnrollment?.let { enrolled ->
                    val displayAccount = enrolled.userName.ifBlank { enrolled.email }
                    Column(
                        Modifier.fillMaxWidth(),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            Icon(
                                Icons.Outlined.Fingerprint,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.primary,
                                modifier = Modifier.size(20.dp),
                            )
                            Text(
                                stringResource(R.string.login_biometric_saved_for, displayAccount),
                                style = MaterialTheme.typography.titleSmall,
                                color = MaterialTheme.colorScheme.onSurface,
                            )
                        }
                        if (enrolled.email.isNotBlank() && enrolled.userName.isNotBlank()) {
                            Text(
                                enrolled.email,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        Button(
                            onClick = {
                                UiHaptics.confirm(view)
                                vm.signInWithBiometrics(context)
                            },
                            enabled = !state.signingIn,
                            colors = graceRidePrimaryButtonColors(),
                            modifier = Modifier.fillMaxWidth().height(52.dp),
                        ) {
                            if (state.signingIn) {
                                InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                            } else {
                                Icon(Icons.Outlined.Fingerprint, contentDescription = null, modifier = Modifier.size(20.dp))
                                Spacer(Modifier.width(Spacing.xs))
                                Text(stringResource(R.string.login_biometric_btn))
                            }
                        }
                        TextButton(
                            onClick = { vm.forgetBiometricOnDevice() },
                            enabled = !state.signingIn,
                        ) {
                            Text(stringResource(R.string.login_biometric_forget))
                        }
                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                        Text(
                            stringResource(R.string.login_or_passkey),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
                OutlinedTextField(
                    value = state.email,
                    onValueChange = vm::setEmail,
                    label = { Text(stringResource(R.string.login_email)) },
                    leadingIcon = { Icon(Icons.Outlined.Email, contentDescription = null) },
                    trailingIcon = if (state.email.isNotEmpty()) {
                        {
                            IconButton(onClick = { vm.setEmail("") }) {
                                Icon(Icons.Outlined.Close, contentDescription = stringResource(R.string.a11y_clear))
                            }
                        }
                    } else {
                        null
                    },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Email, imeAction = ImeAction.Done),
                    keyboardActions = KeyboardActions(onDone = { vm.signInWithPasskey(context) }),
                    modifier = Modifier.fillMaxWidth(),
                )
                Button(
                    onClick = {
                        UiHaptics.confirm(view)
                        vm.signInWithPasskey(context)
                    },
                    enabled = !state.signingIn,
                    colors = graceRidePrimaryButtonColors(),
                    modifier = Modifier.fillMaxWidth().height(52.dp),
                ) {
                    if (state.signingIn) {
                        InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                    } else {
                        Icon(Icons.Outlined.Key, contentDescription = null, modifier = Modifier.size(18.dp))
                        Spacer(Modifier.width(Spacing.xs))
                        Text(stringResource(R.string.login_passkey))
                    }
                }
                if (state.noPasskeyHint) {
                    Text(
                        stringResource(R.string.login_no_passkey),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        textAlign = TextAlign.Center,
                    )
                }
                val errText = state.error ?: state.errorRes?.let { stringResource(it) }
                errText?.let {
                    CalloutBanner(
                        icon = Icons.Outlined.WarningAmber,
                        text = it,
                        containerColor = MaterialTheme.colorScheme.errorContainer,
                        contentColor = MaterialTheme.colorScheme.onErrorContainer,
                    )
                }
            }
        }

        if (state.devAvailable) {
            HorizontalDivider(Modifier.padding(vertical = Spacing.md), color = MaterialTheme.colorScheme.outlineVariant)
            OutlinedCard(Modifier.fillMaxWidth()) {
                Column(
                    Modifier.padding(Spacing.sm),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    Text(
                        stringResource(R.string.login_dev_title),
                        style = MaterialTheme.typography.titleSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    OutlinedTextField(
                        value = serverField,
                        onValueChange = { serverField = it },
                        label = { Text(stringResource(R.string.login_server_url)) },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    OutlinedButton(
                        onClick = { vm.applyServer(serverField) },
                        modifier = Modifier.fillMaxWidth(),
                    ) { Text(stringResource(R.string.login_apply_server)) }
                    state.devUsers.forEach { user ->
                        OutlinedButton(
                            onClick = { vm.devLogin(user.email) },
                            enabled = !state.signingIn,
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Text(user.name + "  ·  " + user.email)
                        }
                    }
                }
            }
        }
    }

    if (state.turnstilePrompted && !state.turnstileSiteKey.isNullOrBlank()) {
        TurnstileDialog(
            siteKey = state.turnstileSiteKey!!,
            baseUrl = container.baseUrl,
            action = state.turnstileAction,
            onToken = { token ->
                if (state.turnstileAction == "biometric_login") {
                    vm.signInWithBiometrics(context, token)
                } else {
                    vm.signInWithPasskey(context, token)
                }
            },
            onDismiss = vm::dismissTurnstile,
        )
    }
}

@SuppressLint("SetJavaScriptEnabled")
@Composable
private fun TurnstileDialog(
    siteKey: String,
    baseUrl: String,
    action: String,
    onToken: (String) -> Unit,
    onDismiss: () -> Unit,
) {
    val safeAction = if (action == "biometric_login") "biometric_login" else "passkey_login"
    val webViewHolder = remember { arrayOfNulls<WebView>(1) }
    val scope = rememberCoroutineScope()
    DisposableEffect(Unit) {
        onDispose {
            webViewHolder[0]?.destroy()
            webViewHolder[0] = null
        }
    }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.login_turnstile_title)) },
        text = {
            AndroidView(
                modifier = Modifier.fillMaxWidth().height(96.dp),
                factory = { ctx ->
                    WebView(ctx).apply {
                        val baseOrigin = runCatching {
                            val parsed = android.net.Uri.parse(baseUrl)
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
                        }
                        addJavascriptInterface(
                            object {
                                @JavascriptInterface
                                fun onToken(token: String) {
                                    scope.launch(Dispatchers.Main.immediate) { onToken(token) }
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
                                function onLoadTurnstile() {
                                  if (!window.turnstile) return;
                                  window.turnstile.render('#cf-turnstile', {
                                    sitekey: '$siteKey',
                                    action: '$safeAction',
                                    callback: function(token) {
                                      if (window.GraceTurnstile && token) {
                                        window.GraceTurnstile.onToken(token);
                                      }
                                    }
                                  });
                                }
                              </script>
                              <script src="https://challenges.cloudflare.com/turnstile/v0/api.js?onload=onLoadTurnstile&render=explicit" async defer></script>
                            </head>
                            <body style="margin:0;display:flex;justify-content:center;align-items:center;min-height:80px;">
                              <div id="cf-turnstile"></div>
                            </body>
                            </html>
                        """.trimIndent()
                        loadDataWithBaseURL(safeBaseUrl, html, "text/html", "UTF-8", null)
                        webViewHolder[0] = this
                    }
                },
            )
        },
        confirmButton = {},
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text(stringResource(R.string.common_cancel))
            }
        },
    )
}
