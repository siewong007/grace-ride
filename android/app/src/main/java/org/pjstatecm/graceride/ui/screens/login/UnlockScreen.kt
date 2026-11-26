package org.pjstatecm.graceride.ui.screens.login

import android.content.Context
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
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
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Fingerprint
import androidx.compose.material.icons.outlined.Key
import androidx.compose.material.icons.outlined.Lock
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
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
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import androidx.credentials.CredentialManager
import androidx.credentials.GetCredentialRequest
import androidx.credentials.GetPublicKeyCredentialOption
import androidx.credentials.PublicKeyCredential
import androidx.credentials.exceptions.GetCredentialCancellationException
import androidx.credentials.exceptions.GetCredentialException
import androidx.credentials.exceptions.NoCredentialException
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.BuildConfig
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.BiometricEnrollment
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.platform.AndroidBiometricCryptoDriver
import org.pjstatecm.graceride.platform.BiometricAvailability
import org.pjstatecm.graceride.platform.BiometricSignResult
import org.pjstatecm.graceride.platform.findFragmentActivity
import org.pjstatecm.graceride.ui.theme.Spacing

data class UnlockUiState(val busy: Boolean = false, val error: String? = null, val useBackup: Boolean = false)

class UnlockViewModel(private val container: AppContainer) : ViewModel() {
    private val _state = MutableStateFlow(UnlockUiState())
    val state: StateFlow<UnlockUiState> = _state.asStateFlow()
    val biometricEnrollment: StateFlow<BiometricEnrollment?> = container.biometric.enrollment
    private var hasAutoPromptedBiometric = false

    fun autoPromptBiometricOnce(context: Context) {
        if (hasAutoPromptedBiometric || _state.value.busy) return
        if (container.biometric.availability(context) != BiometricAvailability.AVAILABLE) return
        if (container.biometric.driver is AndroidBiometricCryptoDriver && context.findFragmentActivity() == null) {
            return
        }
        hasAutoPromptedBiometric = true
        unlockWithBiometric(context)
    }

    fun unlockWithBiometric(context: Context) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, error = null)
        viewModelScope.launch {
            try {
                when (
                    val res = container.auth.reauthWithBiometric(
                        context = context,
                        title = context.getString(R.string.biometric_prompt_unlock_title),
                        subtitle = context.getString(R.string.biometric_prompt_unlock_subtitle),
                        cancelText = context.getString(R.string.common_cancel),
                    )
                ) {
                    is BiometricSignResult.Success,
                    BiometricSignResult.Cancelled -> {}
                    BiometricSignResult.KeyInvalidated -> {
                        _state.value = _state.value.copy(error = context.getString(R.string.biometric_invalidated_notice))
                    }
                    is BiometricSignResult.Error -> {
                        _state.value = _state.value.copy(error = res.message)
                    }
                }
            } catch (e: ApiException) {
                _state.value = _state.value.copy(error = e.message)
            } catch (e: Exception) {
                _state.value = _state.value.copy(error = e.message ?: context.getString(R.string.biometric_failed))
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun toggleBackup() { _state.value = _state.value.copy(useBackup = !_state.value.useBackup, error = null) }

    fun unlock(code: String) {
        if (_state.value.busy || code.isBlank()) return
        _state.value = _state.value.copy(busy = true, error = null)
        viewModelScope.launch {
            try {
                container.auth.unlock(code.trim(), _state.value.useBackup)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(error = e.message)
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
                // User dismissed the passkey prompt cleanly.
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

    fun devUnlock() {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, error = null)
        viewModelScope.launch {
            try {
                container.auth.devUnlock()
            } catch (e: ApiException) {
                _state.value = _state.value.copy(error = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun signOut() { viewModelScope.launch { container.auth.logout() } }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory { initializer { UnlockViewModel(container) } }
    }
}

@Composable
fun UnlockScreen(container: AppContainer) {
    org.pjstatecm.graceride.ui.SecureScreenEffect()
    val vm: UnlockViewModel = viewModel(factory = UnlockViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val biometricEnrollment by vm.biometricEnrollment.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val view = LocalView.current
    var code by remember { mutableStateOf("") }
    val meId = container.auth.me?.id
    val showBiometricUnlock = biometricEnrollment != null && (meId == null || biometricEnrollment?.userId == meId)
    val showPasskeyUnlock = container.auth.me == null || (container.auth.me?.passkeyCount ?: 0) > 0

    LaunchedEffect(showBiometricUnlock) {
        if (showBiometricUnlock) {
            vm.autoPromptBiometricOnce(context)
        }
    }

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
        Surface(
            color = MaterialTheme.colorScheme.primaryContainer,
            shape = CircleShape,
            modifier = Modifier.size(64.dp),
        ) {
            Box(contentAlignment = Alignment.Center) {
                Icon(
                    Icons.Outlined.Lock,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.size(30.dp),
                )
            }
        }
        Text(
            stringResource(R.string.unlock_title),
            style = MaterialTheme.typography.headlineSmall,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.padding(top = Spacing.sm),
        )
        Text(
            stringResource(R.string.unlock_subtitle),
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
                if (showBiometricUnlock) {
                    Button(
                        onClick = {
                            UiHaptics.confirm(view)
                            vm.unlockWithBiometric(context)
                        },
                        enabled = !state.busy,
                        modifier = Modifier.fillMaxWidth().height(52.dp),
                    ) {
                        if (state.busy) {
                            InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                        } else {
                            Icon(Icons.Outlined.Fingerprint, contentDescription = null, modifier = Modifier.size(20.dp))
                            Spacer(Modifier.width(Spacing.xs))
                            Text(stringResource(R.string.biometric_unlock_btn))
                        }
                    }
                    HorizontalDivider(Modifier.padding(vertical = Spacing.xs), color = MaterialTheme.colorScheme.outlineVariant)
                }
                if (showPasskeyUnlock) {
                    Button(
                        onClick = {
                            UiHaptics.confirm(view)
                            vm.unlockWithPasskey(context)
                        },
                        enabled = !state.busy,
                        modifier = Modifier.fillMaxWidth().height(52.dp),
                    ) {
                        if (state.busy) {
                            InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                        } else {
                            Icon(Icons.Outlined.Key, contentDescription = null, modifier = Modifier.size(18.dp))
                            Spacer(Modifier.width(Spacing.xs))
                            Text(stringResource(R.string.unlock_passkey))
                        }
                    }
                    HorizontalDivider(Modifier.padding(vertical = Spacing.xs), color = MaterialTheme.colorScheme.outlineVariant)
                }
                OutlinedTextField(
                    value = code,
                    onValueChange = { code = it },
                    label = { Text(stringResource(if (state.useBackup) R.string.unlock_backup else R.string.unlock_code)) },
                    singleLine = true,
                    visualTransformation = if (state.useBackup) {
                        androidx.compose.ui.text.input.PasswordVisualTransformation()
                    } else {
                        androidx.compose.ui.text.input.VisualTransformation.None
                    },
                    keyboardOptions = KeyboardOptions(
                        keyboardType = if (state.useBackup) KeyboardType.Password else KeyboardType.NumberPassword,
                        imeAction = ImeAction.Done,
                    ),
                    keyboardActions = KeyboardActions(onDone = { vm.unlock(code) }),
                    modifier = Modifier.fillMaxWidth(),
                )
                state.error?.let {
                    Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium)
                }
                Button(
                    onClick = {
                        UiHaptics.confirm(view)
                        vm.unlock(code)
                    },
                    enabled = !state.busy && code.isNotBlank(),
                    modifier = Modifier.fillMaxWidth().height(52.dp),
                ) {
                    if (state.busy) {
                        InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                    } else {
                        Text(stringResource(R.string.unlock_submit))
                    }
                }
                TextButton(onClick = { vm.toggleBackup() }) {
                    Text(stringResource(if (state.useBackup) R.string.unlock_code else R.string.unlock_backup))
                }
                if (BuildConfig.DEBUG) {
                    OutlinedButton(
                        onClick = { vm.devUnlock() },
                        enabled = !state.busy,
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Text(stringResource(R.string.unlock_dev_btn))
                    }
                }
            }
        }
        TextButton(onClick = { vm.signOut() }, modifier = Modifier.padding(top = Spacing.sm)) {
            Text(stringResource(R.string.profile_logout))
        }
    }
}
