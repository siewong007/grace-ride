package org.pjstatecm.graceride.data.repo

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.ApiService
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.DevUser
import org.pjstatecm.graceride.data.api.LogoutReq
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.OkResponse
import org.pjstatecm.graceride.data.api.PasskeyFinishReq
import org.pjstatecm.graceride.data.api.PasskeyStart
import org.pjstatecm.graceride.data.api.PasskeyStartReq
import org.pjstatecm.graceride.data.api.UnlockReq
import org.pjstatecm.graceride.data.api.apiCall
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.session.OfflineSnapshotStore
import org.pjstatecm.graceride.data.session.PersistentCookieJar
import org.pjstatecm.graceride.data.session.SecureStore

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
    private val _state: MutableStateFlow<AuthState> = MutableStateFlow(AuthState.Loading),
    private val store: SecureStore? = null,
    private val onCleared: (() -> Unit)? = null,
) {
    val state: StateFlow<AuthState> = _state.asStateFlow()

    val me: Me?
        get() = when (val s = _state.value) {
            is AuthState.SignedIn -> s.me
            is AuthState.Locked -> s.me
            is AuthState.Pending -> s.me
            else -> null
        }

    private fun stateFor(m: Me): AuthState = when {
        m.locked -> AuthState.Locked(m)
        m.status == "pending" -> AuthState.Pending(m)
        else -> AuthState.SignedIn(m)
    }

    private fun cacheMe(m: Me) {
        store?.putString(CACHED_ME_KEY, apiJson.encodeToString(Me.serializer(), m))
    }

    private fun loadCachedMe(): Me? {
        val raw = store?.getString(CACHED_ME_KEY) ?: return null
        return runCatching { apiJson.decodeFromString(Me.serializer(), raw) }.getOrNull()
    }

    private fun clearLocalSession() {
        cookies.clear()
        store?.remove(CACHED_ME_KEY)
        store?.remove(HIDE_LOCK_DETAILS_KEY)
        store?.let { OfflineSnapshotStore.clear(it) }
        onCleared?.invoke()
    }

    suspend fun refresh() {
        try {
            val m = apiCall(bus) { api.me() }
            cacheMe(m)
            _state.value = stateFor(m)
        } catch (e: ApiException) {
            if (e.status == 401 || e.status == 403) {
                clearLocalSession()
                _state.value = AuthState.SignedOut
            }
            // Offline cold-start with valid session cookies: restore last verified profile
            // so the member can still view cached today's rides until connectivity returns.
            else if (_state.value is AuthState.Loading) {
                val cached = if (e.status == 0 && cookies.hasCookies()) loadCachedMe() else null
                _state.value = if (cached != null) stateFor(cached) else AuthState.SignedOut
            }
        }
    }

    fun onUnauthorized() {
        clearLocalSession()
        _state.value = AuthState.SignedOut
    }

    fun onLocked() {
        _state.value = AuthState.Locked(me)
    }

    fun onPending() {
        val m = me
        if (m != null) _state.value = AuthState.Pending(m)
    }

    fun updateUnread(count: Int) {
        val safe = count.coerceAtLeast(0)
        when (val s = _state.value) {
            is AuthState.SignedIn -> if (s.me.unread != safe) {
                _state.value = AuthState.SignedIn(s.me.copy(unread = safe))
            }
            is AuthState.Locked -> s.me?.let { m ->
                if (m.unread != safe) _state.value = AuthState.Locked(m.copy(unread = safe))
            }
            is AuthState.Pending -> if (s.me.unread != safe) {
                _state.value = AuthState.Pending(s.me.copy(unread = safe))
            }
            else -> {}
        }
    }

    suspend fun devUsers(): List<DevUser> = apiCall(bus) { api.devUsers() }

    suspend fun devLogin(email: String) {
        apiCall(bus) { api.devLogin(mapOf("email" to email)) }
        refresh()
    }

    suspend fun passkeyLoginStart(email: String): PasskeyStart =
        apiCall(bus) { api.passkeyLoginStart(PasskeyStartReq(email.trim())) }

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

    suspend fun logout(mobileToken: String? = null) {
        try {
            apiCall(bus) {
                api.logout(LogoutReq(mobile_token = mobileToken?.takeIf { it.isNotBlank() }))
            }
        } catch (_: ApiException) {
            // Even if the server call fails, drop the local credential.
        }
        clearLocalSession()
        _state.value = AuthState.SignedOut
    }

    suspend fun logoutAll() {
        try {
            apiCall(bus) { api.logoutAll() }
        } catch (_: ApiException) {
        }
        clearLocalSession()
        _state.value = AuthState.SignedOut
    }

    suspend fun acceptTerms() {
        apiCall(bus) { api.acceptTerms(org.pjstatecm.graceride.data.api.TermsReq(accepted = true)) }
        refresh()
    }

    companion object {
        private const val CACHED_ME_KEY = "cached_me_v1"
        private const val HIDE_LOCK_DETAILS_KEY = "notif_hide_lock_details"
    }
}
