package org.pjstatecm.graceride.ui.screens.public

import android.content.Context
import androidx.annotation.StringRes
import androidx.credentials.CreatePublicKeyCredentialRequest
import androidx.credentials.CredentialManager
import androidx.credentials.exceptions.CreateCredentialCancellationException
import androidx.credentials.exceptions.CreateCredentialException
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.InviteAcceptReq
import org.pjstatecm.graceride.data.api.InvitePreviewDto
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.PasskeyOptions

data class InviteUiState(
    val loading: Boolean = true,
    val preview: InvitePreviewDto? = null,
    val step: Int = 1,
    val name: String = "",
    val email: String = "",
    val phone: String = "",
    val termsAccepted: Boolean = false,
    val submitting: Boolean = false,
    val registeringPasskey: Boolean = false,
    val confirmSkipOpen: Boolean = false,
    val done: Boolean = false,
    val emailTaken: Boolean = false,
    val error: String? = null,
    @param:StringRes @get:StringRes val errorRes: Int? = null,
    val loadError: String? = null,
    val turnstileEnabled: Boolean = false,
    val turnstileSiteKey: String? = null,
    val turnstilePrompted: Boolean = false,
)

class InviteViewModel(
    private val container: AppContainer,
    private val token: String,
) : ViewModel() {
    private val siteKeyRe = Regex("^[A-Za-z0-9_-]+$")
    private val _state = MutableStateFlow(InviteUiState())
    val state: StateFlow<InviteUiState> = _state.asStateFlow()

    init {
        load()
    }

    fun load() {
        _state.value = InviteUiState(loading = true)
        viewModelScope.launch {
            try {
                val preview = container.publicFlow.invitePreview(token)
                val cfg = runCatching { container.publicFlow.turnstileConfig() }.getOrNull()
                val validKey = cfg?.siteKey?.takeIf { siteKeyRe.matches(it) }
                val startStep = if (preview.kind == "recovery") 2 else 1
                _state.value = InviteUiState(
                    loading = false,
                    preview = preview,
                    step = startStep,
                    name = preview.name,
                    email = preview.email,
                    phone = preview.phone.orEmpty(),
                    turnstileEnabled = cfg?.enabled == true && validKey != null,
                    turnstileSiteKey = validKey,
                )
            } catch (e: ApiException) {
                _state.value = InviteUiState(loading = false, loadError = e.message)
            } catch (e: Exception) {
                _state.value = InviteUiState(loading = false, loadError = e.message)
            }
        }
    }

    fun setName(value: String) {
        _state.value = _state.value.copy(name = value, error = null, errorRes = null)
    }

    fun setEmail(value: String) {
        _state.value = _state.value.copy(email = value, emailTaken = false, error = null, errorRes = null)
    }

    fun setPhone(value: String) {
        _state.value = _state.value.copy(phone = value, error = null, errorRes = null)
    }

    fun setTermsAccepted(value: Boolean) {
        _state.value = _state.value.copy(termsAccepted = value, error = null, errorRes = null)
    }

    fun dismissTurnstile() {
        _state.value = _state.value.copy(turnstilePrompted = false)
    }

    fun continueFromDetails() {
        val s = _state.value
        val trimmedName = s.name.trim()
        val trimmedEmail = s.email.trim()
        val trimmedPhone = s.phone.trim()
        val isFamily = s.preview?.kind == "family"
        when {
            trimmedName.isEmpty() -> {
                _state.value = s.copy(error = null, errorRes = R.string.invite_name_required)
                return
            }
            trimmedName.length > 80 -> {
                _state.value = s.copy(error = null, errorRes = R.string.invite_name_too_long)
                return
            }
            trimmedEmail.isEmpty() || !trimmedEmail.contains('@') -> {
                _state.value = s.copy(error = null, errorRes = R.string.invite_email_invalid)
                return
            }
            !isFamily && trimmedPhone.filter { it.isDigit() }.length < 6 -> {
                _state.value = s.copy(error = null, errorRes = R.string.invite_phone_invalid)
                return
            }
        }
        _state.value = s.copy(step = 2, error = null, errorRes = null)
    }

    fun backToDetails() {
        if (_state.value.preview?.kind != "recovery") {
            _state.value = _state.value.copy(step = 1, error = null, errorRes = null)
        }
    }

    fun acceptTermsAndJoin(turnstileToken: String? = null) {
        val s = _state.value
        if (s.submitting) return
        if (!s.termsAccepted) {
            _state.value = s.copy(error = null, errorRes = R.string.invite_terms_required)
            return
        }
        if (s.turnstileEnabled && turnstileToken.isNullOrBlank()) {
            _state.value = s.copy(error = null, errorRes = null, turnstilePrompted = true)
            return
        }
        _state.value = s.copy(submitting = true, error = null, errorRes = null, emailTaken = false, turnstilePrompted = false)
        viewModelScope.launch {
            try {
                val isRecovery = s.preview?.kind == "recovery"
                val isFamily = s.preview?.kind == "family"
                val req = if (isRecovery) {
                    InviteAcceptReq(
                        terms = true,
                        turnstile_token = turnstileToken,
                    )
                } else {
                    InviteAcceptReq(
                        name = s.name.trim(),
                        email = s.email.trim(),
                        phone = if (isFamily) null else s.phone.trim(),
                        terms = true,
                        turnstile_token = turnstileToken,
                    )
                }
                container.publicFlow.inviteAccept(token, req)
                _state.value = _state.value.copy(step = 3, submitting = false)
            } catch (e: ApiException) {
                if (e.code == "TURNSTILE_REQUIRED" && turnstileToken.isNullOrBlank()) {
                    val cfg = runCatching { container.publicFlow.turnstileConfig() }.getOrNull()
                    val validKey = cfg?.siteKey?.takeIf { siteKeyRe.matches(it) }
                    if (cfg?.enabled == true && validKey != null) {
                        _state.value = _state.value.copy(
                            submitting = false,
                            turnstileEnabled = true,
                            turnstileSiteKey = validKey,
                            turnstilePrompted = true,
                        )
                        return@launch
                    }
                }
                val duplicate = e.code == "EMAIL_EXISTS" || e.status == 409
                _state.value = _state.value.copy(
                    submitting = false,
                    emailTaken = duplicate,
                    error = e.message,
                )
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _state.value = _state.value.copy(submitting = false, error = e.message)
            }
        }
    }

    fun registerPasskey(context: Context) {
        if (_state.value.registeringPasskey) return
        _state.value = _state.value.copy(registeringPasskey = true, error = null, errorRes = null)
        viewModelScope.launch {
            try {
                val start = container.auth.passkeyRegisterStart()
                val req = CreatePublicKeyCredentialRequest(
                    PasskeyOptions.publicKey(start.options).toString()
                )
                val res = CredentialManager.create(context).createCredential(context, req)
                val cred = res as? androidx.credentials.CreatePublicKeyCredentialResponse
                    ?: throw IllegalStateException("Unexpected credential response")
                container.auth.passkeyRegisterFinish(start.challenge_id, cred.registrationResponseJson)
                val email = _state.value.email.trim().ifBlank { _state.value.preview?.email.orEmpty() }
                if (email.isNotBlank()) {
                    container.secureStore.putString("last_email", email)
                }
                container.auth.refresh(throwOnError = false)
                _state.value = _state.value.copy(registeringPasskey = false, done = true)
            } catch (_: CreateCredentialCancellationException) {
                _state.value = _state.value.copy(registeringPasskey = false)
            } catch (e: CreateCredentialException) {
                _state.value = _state.value.copy(registeringPasskey = false, error = e.message)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(registeringPasskey = false, error = e.message)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _state.value = _state.value.copy(registeringPasskey = false, error = e.message)
            }
        }
    }

    fun promptSkipPasskey() {
        _state.value = _state.value.copy(confirmSkipOpen = true)
    }

    fun dismissSkipPasskey() {
        _state.value = _state.value.copy(confirmSkipOpen = false)
    }

    fun confirmSkipPasskey() {
        _state.value = _state.value.copy(confirmSkipOpen = false)
        viewModelScope.launch {
            container.auth.refresh(throwOnError = false)
            _state.value = _state.value.copy(done = true)
        }
    }

    companion object {
        fun factory(container: AppContainer, token: String) = viewModelFactory {
            initializer { InviteViewModel(container, token) }
        }
    }
}
