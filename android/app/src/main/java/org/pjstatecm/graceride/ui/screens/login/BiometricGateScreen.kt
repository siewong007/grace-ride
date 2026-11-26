package org.pjstatecm.graceride.ui.screens.login

import android.content.Context
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Fingerprint
import androidx.compose.material.icons.outlined.Key
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.credentials.CredentialManager
import androidx.credentials.GetCredentialRequest
import androidx.credentials.GetPublicKeyCredentialOption
import androidx.credentials.PublicKeyCredential
import androidx.credentials.exceptions.GetCredentialCancellationException
import androidx.credentials.exceptions.GetCredentialException
import androidx.credentials.exceptions.NoCredentialException
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.platform.AndroidBiometricCryptoDriver
import org.pjstatecm.graceride.platform.BiometricAvailability
import org.pjstatecm.graceride.platform.BiometricSignResult
import org.pjstatecm.graceride.platform.findFragmentActivity
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.theme.Spacing

data class BiometricGateUiState(
    val busy: Boolean = false,
    val error: String? = null,
)

class BiometricGateViewModel(private val container: AppContainer) : ViewModel() {
    private val _state = MutableStateFlow(BiometricGateUiState())
    val state: StateFlow<BiometricGateUiState> = _state.asStateFlow()
    private var hasAutoPrompted = false

    fun autoPromptOnce(context: Context) {
        if (hasAutoPrompted || _state.value.busy) return
        if (container.biometric.availability(context) != BiometricAvailability.AVAILABLE) return
        if (container.biometric.driver is AndroidBiometricCryptoDriver && context.findFragmentActivity() == null) {
            return
        }
        hasAutoPrompted = true
        unlockWithBiometrics(context)
    }

    fun unlockWithBiometrics(context: Context) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, error = null)
        viewModelScope.launch {
            try {
                when (
                    val res = container.biometric.driver.verifyLocalUnlock(
                        context = context,
                        title = context.getString(R.string.biometric_prompt_unlock_title),
                        subtitle = context.getString(R.string.biometric_prompt_unlock_subtitle),
                        cancelText = context.getString(R.string.common_cancel),
                    )
                ) {
                    is BiometricSignResult.Success -> container.biometric.unlockWithVerifiedResult(res)
                    BiometricSignResult.Cancelled -> {}
                    BiometricSignResult.KeyInvalidated -> {
                        container.biometric.handleKeyInvalidated()
                        container.auth.logout()
                    }
                    is BiometricSignResult.Error -> {
                        _state.value = _state.value.copy(error = res.message)
                    }
                }
            } catch (e: Exception) {
                _state.value = _state.value.copy(error = e.message ?: context.getString(R.string.biometric_failed))
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun unlockWithPasskey(context: Context) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, error = null)
        viewModelScope.launch {
            try {
                val start = container.auth.passkeyReauthStart()
                val manager = CredentialManager.create(context)
                val request = GetCredentialRequest.Builder()
                    .addCredentialOption(
                        GetPublicKeyCredentialOption(
                            org.pjstatecm.graceride.domain.PasskeyOptions.publicKey(start.options).toString()
                        )
                    )
                    .build()
                val result = manager.getCredential(context, request)
                val credential = result.credential as? PublicKeyCredential
                    ?: throw IllegalStateException("Unexpected credential type")
                container.auth.passkeyReauthFinish(start.challenge_id, credential.authenticationResponseJson)
            } catch (_: GetCredentialCancellationException) {
            } catch (_: NoCredentialException) {
                _state.value = _state.value.copy(error = context.getString(R.string.login_no_passkey))
            } catch (e: GetCredentialException) {
                _state.value = _state.value.copy(error = e.message ?: context.getString(R.string.reauth_failed))
            } catch (e: ApiException) {
                _state.value = _state.value.copy(error = e.message)
            } catch (e: Exception) {
                _state.value = _state.value.copy(error = e.message ?: context.getString(R.string.reauth_failed))
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun signOut() {
        viewModelScope.launch { container.auth.logout() }
    }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory {
            initializer { BiometricGateViewModel(container) }
        }
    }
}

@Composable
fun BiometricGateScreen(container: AppContainer, me: Me) {
    org.pjstatecm.graceride.ui.SecureScreenEffect()
    val vm: BiometricGateViewModel = viewModel(
        key = "biometric_gate_${me.id}",
        factory = BiometricGateViewModel.factory(container),
    )
    val state by vm.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val view = LocalView.current

    LaunchedEffect(me.id) {
        vm.autoPromptOnce(context)
    }

    Column(
        Modifier
            .fillMaxSize()
            .statusBarsPadding()
            .navigationBarsPadding()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = Spacing.md, vertical = Spacing.md),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Surface(
            color = MaterialTheme.colorScheme.primaryContainer,
            shape = CircleShape,
            modifier = Modifier.size(64.dp),
        ) {
            Box(contentAlignment = Alignment.Center) {
                Icon(
                    Icons.Outlined.Fingerprint,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.size(32.dp),
                )
            }
        }
        Text(
            stringResource(R.string.biometric_gate_title),
            style = MaterialTheme.typography.headlineSmall,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.padding(top = Spacing.sm),
        )
        Text(
            stringResource(R.string.biometric_gate_subtitle, me.name.ifBlank { me.email }),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(top = Spacing.xs, bottom = Spacing.md),
        )
        OutlinedCard(Modifier.fillMaxWidth()) {
            Column(
                Modifier.padding(Spacing.sm),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                Button(
                    onClick = {
                        UiHaptics.confirm(view)
                        vm.unlockWithBiometrics(context)
                    },
                    enabled = !state.busy,
                    modifier = Modifier.fillMaxWidth().height(52.dp),
                ) {
                    if (state.busy) {
                        InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                    } else {
                        Icon(Icons.Outlined.Fingerprint, contentDescription = null, modifier = Modifier.size(20.dp))
                        Spacer(Modifier.width(Spacing.xs))
                        Text(stringResource(R.string.biometric_gate_unlock_btn))
                    }
                }
                if (me.passkeyCount > 0) {
                    OutlinedButton(
                        onClick = {
                            UiHaptics.confirm(view)
                            vm.unlockWithPasskey(context)
                        },
                        enabled = !state.busy,
                        modifier = Modifier.fillMaxWidth().height(48.dp),
                    ) {
                        Icon(Icons.Outlined.Key, contentDescription = null, modifier = Modifier.size(18.dp))
                        Spacer(Modifier.width(Spacing.xs))
                        Text(stringResource(R.string.biometric_gate_passkey_btn))
                    }
                }
                state.error?.let {
                    Text(
                        it,
                        color = MaterialTheme.colorScheme.error,
                        style = MaterialTheme.typography.bodyMedium,
                        textAlign = TextAlign.Center,
                    )
                }
            }
        }
        TextButton(onClick = { vm.signOut() }, modifier = Modifier.padding(top = Spacing.sm)) {
            Text(stringResource(R.string.profile_logout))
        }
    }
}
