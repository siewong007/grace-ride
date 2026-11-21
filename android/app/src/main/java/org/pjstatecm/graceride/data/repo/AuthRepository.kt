package org.pjstatecm.graceride.data.repo

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.ApiService
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.DevUser
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.OkResponse
import org.pjstatecm.graceride.data.api.PasskeyFinishReq
import org.pjstatecm.graceride.data.api.PasskeyStart
import org.pjstatecm.graceride.data.api.UnlockReq
import org.pjstatecm.graceride.data.api.apiCall
import org.pjstatecm.graceride.data.session.PersistentCookieJar

sealed interface AuthState {
    data object Loading : AuthState
    data object SignedOut : AuthState
    data class SignedIn(val me: Me) : AuthState
    data class Locked(val me: Me?) : AuthState
    data class Pending(val me: Me) : AuthState
}

class AuthRepository(
    private val api: ApiService,
    private val bus: AuthEventBus,
    private val cookies: PersistentCookieJar,
) {
    private val _state = MutableStateFlow<AuthState>(AuthState.Loading)
    val state: StateFlow<AuthState> = _state.asStateFlow()

    val me: Me?
        get() = when (val s = _state.value) {
            is AuthState.SignedIn -> s.me
            is AuthState.Locked -> s.me
            is AuthState.Pending -> s.me
            else -> null
        }

    suspend fun refresh() {
        try {
            val m = apiCall(bus) { api.me() }
            _state.value = when {
                m.locked -> AuthState.Locked(m)
                m.status == "pending" -> AuthState.Pending(m)
                else -> AuthState.SignedIn(m)
            }
        } catch (e: ApiException) {
            if (e.status == 401 || e.status == 403) {
                _state.value = AuthState.SignedOut
            }
            // Network failure while loading: stay signed out; the login screen
            // surfaces the error when the member tries to sign in.
            else if (_state.value is AuthState.Loading) {
                _state.value = AuthState.SignedOut
            }
        }
    }

    fun onUnauthorized() {
        cookies.clear()
        _state.value = AuthState.SignedOut
    }

    fun onLocked() {
        _state.value = AuthState.Locked(me)
    }

    fun onPending() {
        val m = me
        if (m != null) _state.value = AuthState.Pending(m)
    }

    suspend fun devUsers(): List<DevUser> = apiCall(bus) { api.devUsers() }

    suspend fun devLogin(email: String) {
        apiCall(bus) { api.devLogin(mapOf("email" to email)) }
        refresh()
    }

    suspend fun passkeyLoginStart(): PasskeyStart = apiCall(bus) { api.passkeyLoginStart() }

    suspend fun passkeyLoginFinish(challengeId: String, credentialJson: String) {
        apiCall(bus) {
            api.passkeyLoginFinish(
                PasskeyFinishReq(challengeId, org.pjstatecm.graceride.data.api.apiJson.parseToJsonElement(credentialJson))
            )
        }
        refresh()
    }

    suspend fun passkeyRegisterStart(): PasskeyStart = apiCall(bus) { api.passkeyRegisterStart() }

    suspend fun passkeyRegisterFinish(challengeId: String, credentialJson: String) {
        apiCall(bus) {
            api.passkeyRegisterFinish(
                PasskeyFinishReq(challengeId, org.pjstatecm.graceride.data.api.apiJson.parseToJsonElement(credentialJson))
            )
        }
        refresh()
    }

    suspend fun unlock(code: String, isBackupCode: Boolean) {
        apiCall(bus) {
            api.unlock(
                if (isBackupCode) UnlockReq(backup_code = code) else UnlockReq(totp_code = code)
            )
        }
        refresh()
    }

    suspend fun devUnlock() {
        apiCall(bus) { api.unlock(UnlockReq(dev_unlock = true)) }
        refresh()
    }

    suspend fun logout() {
        try {
            apiCall(bus) { api.logout() }
        } catch (_: ApiException) {
            // Even if the server call fails, drop the local credential.
        }
        cookies.clear()
        _state.value = AuthState.SignedOut
    }

    suspend fun logoutAll() {
        try {
            apiCall(bus) { api.logoutAll() }
        } catch (_: ApiException) {
        }
        cookies.clear()
        _state.value = AuthState.SignedOut
    }

    suspend fun acceptTerms() {
        apiCall(bus) { api.acceptTerms(org.pjstatecm.graceride.data.api.TermsReq(accepted = true)) }
        refresh()
    }
}
