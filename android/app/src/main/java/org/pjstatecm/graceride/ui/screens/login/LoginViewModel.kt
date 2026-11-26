package org.pjstatecm.graceride.ui.screens.login

import android.content.Context
import androidx.credentials.CredentialManager
import androidx.credentials.GetCredentialRequest
import androidx.credentials.GetPublicKeyCredentialOption
import androidx.credentials.PublicKeyCredential
import androidx.credentials.exceptions.GetCredentialCancellationException
import androidx.credentials.exceptions.GetCredentialException
import androidx.credentials.exceptions.NoCredentialException
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import androidx.annotation.StringRes
import org.pjstatecm.graceride.BuildConfig
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.BiometricEnrollment
import org.pjstatecm.graceride.data.api.DevUser
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.platform.BiometricSignResult

data class LoginUiState(
    val email: String = "",
    val signingIn: Boolean = false,
    val error: String? = null,
    @param:StringRes @get:StringRes val errorRes: Int? = null,
    val noPasskeyHint: Boolean = false,
    val devUsers: List<DevUser> = emptyList(),
    val serverUrl: String = "",
    val devAvailable: Boolean = BuildConfig.DEV_LOGIN_AVAILABLE,
    val turnstileEnabled: Boolean = false,
    val turnstileSiteKey: String? = null,
    val turnstilePrompted: Boolean = false,
    val turnstileAction: String = "passkey_login",
    val recoveryOpen: Boolean = false,
    val recoveryEmail: String = "",
    val recoveryReason: String = "",
    val recoverySubmitting: Boolean = false,
    val recoverySent: Boolean = false,
    val recoveryError: String? = null,
    @param:StringRes @get:StringRes val recoveryErrorRes: Int? = null,
)

class LoginViewModel(
    private val container: AppContainer,
    private val devAvailable: Boolean = BuildConfig.DEV_LOGIN_AVAILABLE,
) : ViewModel() {

    private val siteKeyRe = Regex("^[A-Za-z0-9_-]+$")
    private var turnstileConfigLoaded = false
    private var turnstileConfigJob: Job? = null
    val biometricEnrollment: StateFlow<BiometricEnrollment?> = container.biometric.enrollment
    val keyInvalidatedNotice: StateFlow<Boolean> = container.biometric.keyInvalidatedNotice

    private val _state = MutableStateFlow(
        LoginUiState(
            email = if (container.secureStore is org.pjstatecm.graceride.data.session.InMemoryStore) {
                container.secureStore.getString("last_email").orEmpty()
            } else {
                ""
            },
            serverUrl = container.baseUrl,
            devAvailable = devAvailable,
        )
    )
    val state: StateFlow<LoginUiState> = _state.asStateFlow()

    init {
        if (container.secureStore !is org.pjstatecm.graceride.data.session.InMemoryStore) {
            viewModelScope.launch {
                val savedEmail = kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
                    container.secureStore.getString("last_email")
                }
                if (!savedEmail.isNullOrBlank() && _state.value.email.isBlank()) {
                    _state.value = _state.value.copy(email = savedEmail)
                }
            }
        }
        if (devAvailable) loadDevUsers()
    }

    fun loadTurnstileConfig() {
        turnstileConfigJob?.cancel()
        turnstileConfigJob = viewModelScope.launch {
            fetchTurnstileConfig(resetPrompt = true)
        }
    }

    private suspend fun awaitTurnstileConfigIfPending() {
        if (!turnstileConfigLoaded) {
            val job = turnstileConfigJob
            if (job != null && job.isActive) {
                job.join()
            }
        }
    }

    private suspend fun fetchTurnstileConfig(resetPrompt: Boolean = false): Boolean {
        return try {
            val cfg = container.auth.turnstileConfig()
            val validKey = cfg.siteKey?.takeIf { siteKeyRe.matches(it) }
            val enabled = cfg.enabled && validKey != null
            turnstileConfigLoaded = true
            _state.value = _state.value.copy(
                turnstileEnabled = enabled,
                turnstileSiteKey = validKey,
                turnstilePrompted = if (resetPrompt) false else _state.value.turnstilePrompted,
            )
            enabled
        } catch (_: ApiException) {
            turnstileConfigLoaded = true
            _state.value = _state.value.copy(
                turnstileEnabled = false,
                turnstileSiteKey = null,
                turnstilePrompted = if (resetPrompt) false else _state.value.turnstilePrompted,
            )
            false
        }
    }

    fun setEmail(value: String) {
        _state.value = _state.value.copy(email = value, error = null, errorRes = null)
    }

    fun loadDevUsers() {
        viewModelScope.launch {
            try {
                val users = container.auth.devUsers()
                _state.value = _state.value.copy(devUsers = users, error = null, errorRes = null)
            } catch (_: ApiException) {
                _state.value = _state.value.copy(devUsers = emptyList())
            }
        }
    }

    fun applyServer(url: String) {
        try {
            container.setBaseUrl(url, clearSessionIfChanged = true)
            turnstileConfigLoaded = false
            _state.value = _state.value.copy(
                serverUrl = container.baseUrl,
                turnstileEnabled = false,
                turnstileSiteKey = null,
                turnstilePrompted = false,
                error = null,
                errorRes = null,
            )
            loadTurnstileConfig()
            loadDevUsers()
        } catch (_: IllegalArgumentException) {
            _state.value = _state.value.copy(error = null, errorRes = R.string.login_invalid_server)
        }
    }

    fun clearError() { _state.value = _state.value.copy(error = null, errorRes = null, noPasskeyHint = false) }

    fun dismissTurnstile() { _state.value = _state.value.copy(turnstilePrompted = false) }

    fun openRecovery() {
        _state.value = _state.value.copy(
            recoveryOpen = true,
            recoveryEmail = _state.value.recoveryEmail.ifBlank { _state.value.email },
            recoverySent = false,
            recoveryError = null,
            recoveryErrorRes = null,
        )
    }

    fun closeRecovery() {
        _state.value = _state.value.copy(
            recoveryOpen = false,
            recoveryError = null,
            recoveryErrorRes = null,
            recoverySent = false,
        )
    }

    fun setRecoveryEmail(value: String) {
        _state.value = _state.value.copy(recoveryEmail = value, recoveryError = null, recoveryErrorRes = null)
    }

    fun setRecoveryReason(value: String) {
        _state.value = _state.value.copy(recoveryReason = value, recoveryError = null, recoveryErrorRes = null)
    }

    fun submitRecovery(turnstileToken: String? = null) {
        if (_state.value.recoverySubmitting) return
        val email = _state.value.recoveryEmail.trim()
        if (email.isEmpty() || !email.contains('@')) {
            _state.value = _state.value.copy(
                recoveryError = null,
                recoveryErrorRes = R.string.login_recovery_email_required,
            )
            return
        }
        if (_state.value.turnstileEnabled && turnstileToken.isNullOrBlank()) {
            _state.value = _state.value.copy(
                recoveryError = null,
                recoveryErrorRes = null,
                turnstilePrompted = true,
                turnstileAction = "recovery_request",
            )
            return
        }
        _state.value = _state.value.copy(
            recoverySubmitting = true,
            recoveryError = null,
            recoveryErrorRes = null,
            turnstilePrompted = false,
        )
        viewModelScope.launch {
            try {
                awaitTurnstileConfigIfPending()
                if (_state.value.turnstileEnabled && turnstileToken.isNullOrBlank()) {
                    _state.value = _state.value.copy(
                        turnstilePrompted = true,
                        turnstileAction = "recovery_request",
                    )
                    return@launch
                }
                container.publicFlow.recoveryRequest(
                    email = email,
                    reason = _state.value.recoveryReason.trim(),
                    turnstileToken = turnstileToken,
                )
                _state.value = _state.value.copy(recoverySent = true, recoveryReason = "")
            } catch (e: ApiException) {
                if (e.code == "TURNSTILE_REQUIRED" && turnstileToken.isNullOrBlank() && fetchTurnstileConfig()) {
                    _state.value = _state.value.copy(
                        turnstilePrompted = true,
                        turnstileAction = "recovery_request",
                    )
                } else {
                    _state.value = _state.value.copy(recoveryError = e.message)
                }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _state.value = _state.value.copy(recoveryError = e.message)
            } finally {
                _state.value = _state.value.copy(recoverySubmitting = false)
            }
        }
    }

    fun dismissKeyInvalidatedNotice() { container.biometric.clearKeyInvalidatedNotice() }

    fun forgetBiometricOnDevice() {
        container.biometric.clearEnrollmentLocal()
        _state.value = _state.value.copy(error = null, errorRes = null)
    }

    /**
     * Direct hardware-backed biometric sign-in: fetches a one-time challenge for the
     * enrolled device credential_id, unlocks the AndroidKeyStore EC P-256 key via
     * BiometricPrompt, and verifies the signature on the backend without requiring email input.
     */
    fun signInWithBiometrics(context: Context, turnstileToken: String? = null) {
        if (_state.value.signingIn) return
        if (container.biometric.enrollment.value == null) return
        if (_state.value.turnstileEnabled && turnstileToken.isNullOrBlank()) {
            _state.value = _state.value.copy(
                error = null,
                errorRes = null,
                noPasskeyHint = false,
                turnstilePrompted = true,
                turnstileAction = "biometric_login",
            )
            return
        }
        _state.value = _state.value.copy(
            signingIn = true,
            error = null,
            errorRes = null,
            noPasskeyHint = false,
            turnstilePrompted = false,
        )
        viewModelScope.launch {
            try {
                awaitTurnstileConfigIfPending()
                if (_state.value.turnstileEnabled && turnstileToken.isNullOrBlank()) {
                    _state.value = _state.value.copy(
                        turnstilePrompted = true,
                        turnstileAction = "biometric_login",
                    )
                    return@launch
                }
                when (
                    val res = container.auth.loginWithBiometric(
                        context = context,
                        turnstileToken = turnstileToken,
                        title = context.getString(R.string.biometric_prompt_login_title),
                        subtitle = context.getString(R.string.biometric_prompt_login_subtitle),
                        cancelText = context.getString(R.string.common_cancel),
                    )
                ) {
                    is BiometricSignResult.Success -> {
                        container.biometric.clearKeyInvalidatedNotice()
                    }
                    BiometricSignResult.Cancelled,
                    BiometricSignResult.KeyInvalidated -> {}
                    is BiometricSignResult.Error -> {
                        _state.value = _state.value.copy(error = res.message)
                    }
                }
            } catch (e: ApiException) {
                if (e.code == "TURNSTILE_REQUIRED" && turnstileToken.isNullOrBlank() && fetchTurnstileConfig()) {
                    _state.value = _state.value.copy(
                        turnstilePrompted = true,
                        turnstileAction = "biometric_login",
                    )
                } else {
                    _state.value = _state.value.copy(error = e.message)
                }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _state.value = _state.value.copy(error = e.message ?: context.getString(R.string.login_failed))
            } finally {
                _state.value = _state.value.copy(signingIn = false)
            }
        }
    }

    /**
     * Passkey sign-in: the server's WebAuthn challenge is handed to Credential
     * Manager verbatim; the authenticator response goes back to /auth/passkey/login/finish.
     */
    fun signInWithPasskey(context: Context, turnstileToken: String? = null) {
        if (_state.value.signingIn) return
        val email = _state.value.email.trim()
        if (email.isEmpty() || !email.contains('@')) {
            _state.value = _state.value.copy(error = null, errorRes = R.string.login_email_required, noPasskeyHint = false)
            return
        }
        if (_state.value.turnstileEnabled && turnstileToken.isNullOrBlank()) {
            _state.value = _state.value.copy(
                error = null,
                errorRes = null,
                noPasskeyHint = false,
                turnstilePrompted = true,
                turnstileAction = "passkey_login",
            )
            return
        }
        _state.value = _state.value.copy(
            signingIn = true,
            error = null,
            errorRes = null,
            noPasskeyHint = false,
            turnstilePrompted = false,
        )
        viewModelScope.launch {
            try {
                awaitTurnstileConfigIfPending()
                if (_state.value.turnstileEnabled && turnstileToken.isNullOrBlank()) {
                    _state.value = _state.value.copy(
                        turnstilePrompted = true,
                        turnstileAction = "passkey_login",
                    )
                    return@launch
                }
                val start = container.auth.passkeyLoginStart(email, turnstileToken)
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
                container.auth.passkeyLoginFinish(start.challenge_id, credential.authenticationResponseJson)
                container.biometric.clearKeyInvalidatedNotice()
                kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
                    container.secureStore.putString("last_email", email)
                }
            } catch (_: NoCredentialException) {
                _state.value = _state.value.copy(noPasskeyHint = true)
            } catch (_: GetCredentialCancellationException) {
                // User dismissed the passkey prompt cleanly.
            } catch (e: GetCredentialException) {
                _state.value = _state.value.copy(error = e.message ?: context.getString(R.string.login_failed))
            } catch (e: ApiException) {
                if (e.code == "TURNSTILE_REQUIRED" && turnstileToken.isNullOrBlank() && fetchTurnstileConfig()) {
                    _state.value = _state.value.copy(
                        turnstilePrompted = true,
                        turnstileAction = "passkey_login",
                    )
                } else {
                    _state.value = _state.value.copy(error = e.message)
                }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _state.value = _state.value.copy(error = e.message ?: context.getString(R.string.login_failed))
            } finally {
                _state.value = _state.value.copy(signingIn = false)
            }
        }
    }

    fun devLogin(email: String) {
        if (_state.value.signingIn) return
        _state.value = _state.value.copy(signingIn = true, error = null, errorRes = null)
        viewModelScope.launch {
            try {
                container.auth.devLogin(email)
                container.biometric.clearKeyInvalidatedNotice()
                kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
                    container.secureStore.putString("last_email", email.trim())
                }
            } catch (e: ApiException) {
                _state.value = _state.value.copy(error = e.message)
            } finally {
                _state.value = _state.value.copy(signingIn = false)
            }
        }
    }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory {
            initializer { LoginViewModel(container) }
        }
    }
}
