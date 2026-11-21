package org.pjstatecm.graceride.ui.screens.profile

import android.content.Context
import androidx.credentials.CreatePublicKeyCredentialRequest
import androidx.credentials.CredentialManager
import androidx.credentials.PublicKeyCredential
import androidx.credentials.exceptions.CreateCredentialException
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.PasskeyInfo
import org.pjstatecm.graceride.data.api.ProfileUpdateReq
import org.pjstatecm.graceride.di.AppContainer

data class ProfileUiState(
    val me: Me? = null,
    val passkeys: List<PasskeyInfo> = emptyList(),
    val busy: Boolean = false,
    val message: String? = null,
    val error: String? = null,
    val lang: String = "zh",
)

class ProfileViewModel(private val container: AppContainer) : ViewModel() {

    private val _state = MutableStateFlow(ProfileUiState(me = container.auth.me, lang = container.lang))
    val state: StateFlow<ProfileUiState> = _state.asStateFlow()

    init { loadPasskeys() }

    fun refreshMe() {
        viewModelScope.launch {
            container.auth.refresh()
            _state.value = _state.value.copy(me = container.auth.me)
        }
    }

    fun loadPasskeys() {
        viewModelScope.launch {
            try {
                _state.value = _state.value.copy(passkeys = container.community.passkeys())
            } catch (_: ApiException) { }
        }
    }

    fun setLang(lang: String, onLocaleChanged: () -> Unit) {
        container.lang = lang
        _state.value = _state.value.copy(lang = lang)
        viewModelScope.launch {
            try {
                container.community.updateProfile(ProfileUpdateReq(lang = lang))
                container.auth.refresh()
            } catch (_: ApiException) { }
            onLocaleChanged()
        }
    }

    fun setAutoLock(minutes: Int) {
        viewModelScope.launch {
            try {
                container.community.updateProfile(ProfileUpdateReq(auto_lock_mins = minutes))
                container.auth.refresh()
                _state.value = _state.value.copy(me = container.auth.me)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(error = e.message)
            }
        }
    }

    fun addPasskey(context: Context) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, error = null)
        viewModelScope.launch {
            try {
                val start = container.auth.passkeyRegisterStart()
                val manager = CredentialManager.create(context)
                val result = manager.createCredential(
                    context,
                    CreatePublicKeyCredentialRequest(
                        org.pjstatecm.graceride.domain.PasskeyOptions.publicKey(start.options).toString()
                    ),
                )
                val credential = result as? androidx.credentials.CreatePublicKeyCredentialResponse
                    ?: throw IllegalStateException("Unexpected credential type")
                container.auth.passkeyRegisterFinish(start.challenge_id, credential.registrationResponseJson)
                loadPasskeys()
            } catch (e: CreateCredentialException) {
                _state.value = _state.value.copy(error = e.message)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(error = e.message)
            } catch (e: Exception) {
                _state.value = _state.value.copy(error = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun deletePasskey(id: Long) {
        viewModelScope.launch {
            try {
                container.community.deletePasskey(id)
                loadPasskeys()
            } catch (e: ApiException) {
                _state.value = _state.value.copy(error = e.message)
            }
        }
    }

    fun logout() { viewModelScope.launch { container.auth.logout() } }
    fun logoutAll() { viewModelScope.launch { container.auth.logoutAll() } }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory { initializer { ProfileViewModel(container) } }
    }
}
