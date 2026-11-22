package org.pjstatecm.graceride.ui.screens.login

import android.content.Context
import androidx.credentials.CredentialManager
import androidx.credentials.GetCredentialRequest
import androidx.credentials.GetPublicKeyCredentialOption
import androidx.credentials.PublicKeyCredential
import androidx.credentials.exceptions.GetCredentialException
import androidx.credentials.exceptions.NoCredentialException
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.BuildConfig
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.DevUser
import org.pjstatecm.graceride.di.AppContainer

data class LoginUiState(
    val signingIn: Boolean = false,
    val error: String? = null,
    val noPasskeyHint: Boolean = false,
    val devUsers: List<DevUser> = emptyList(),
    val serverUrl: String = "",
    val devAvailable: Boolean = BuildConfig.DEV_LOGIN_AVAILABLE,
)

class LoginViewModel(
    private val container: AppContainer,
    private val devAvailable: Boolean = BuildConfig.DEV_LOGIN_AVAILABLE,
) : ViewModel() {

    private val _state = MutableStateFlow(LoginUiState(serverUrl = container.baseUrl, devAvailable = devAvailable))
    val state: StateFlow<LoginUiState> = _state.asStateFlow()

    init {
        if (devAvailable) loadDevUsers()
    }

    fun loadDevUsers() {
        viewModelScope.launch {
            try {
                val users = container.auth.devUsers()
                _state.value = _state.value.copy(devUsers = users, error = null)
            } catch (_: ApiException) {
                _state.value = _state.value.copy(devUsers = emptyList())
            }
        }
    }

    fun applyServer(url: String) {
        container.setBaseUrl(url.trim())
        _state.value = _state.value.copy(serverUrl = container.baseUrl)
        loadDevUsers()
    }

    fun clearError() { _state.value = _state.value.copy(error = null, noPasskeyHint = false) }

    /**
     * Passkey sign-in: the server's WebAuthn challenge is handed to Credential
     * Manager verbatim; the authenticator response goes back to /auth/passkey/login/finish.
     */
    fun signInWithPasskey(context: Context) {
        if (_state.value.signingIn) return
        _state.value = _state.value.copy(signingIn = true, error = null, noPasskeyHint = false)
        viewModelScope.launch {
            try {
                val start = container.auth.passkeyLoginStart()
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
            } catch (e: NoCredentialException) {
                _state.value = _state.value.copy(noPasskeyHint = true)
            } catch (e: GetCredentialException) {
                _state.value = _state.value.copy(error = e.message ?: "Sign-in cancelled")
            } catch (e: ApiException) {
                _state.value = _state.value.copy(error = e.message)
            } catch (e: Exception) {
                _state.value = _state.value.copy(error = e.message ?: "Sign-in failed")
            } finally {
                _state.value = _state.value.copy(signingIn = false)
            }
        }
    }

    fun devLogin(email: String) {
        if (_state.value.signingIn) return
        _state.value = _state.value.copy(signingIn = true, error = null)
        viewModelScope.launch {
            try {
                container.auth.devLogin(email)
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
