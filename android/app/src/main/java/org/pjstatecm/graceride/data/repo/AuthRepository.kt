package org.pjstatecm.graceride.data.repo

import android.content.Context
import android.os.Build
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.ApiProvider
import org.pjstatecm.graceride.data.api.ApiService
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.BackupCodesRespDto
import org.pjstatecm.graceride.data.api.BiometricEnrollment
import org.pjstatecm.graceride.data.api.BiometricFinishReq
import org.pjstatecm.graceride.data.api.BiometricInfo
import org.pjstatecm.graceride.data.api.BiometricLoginStartReq
import org.pjstatecm.graceride.data.api.BiometricReauthStartReq
import org.pjstatecm.graceride.data.api.BiometricRegisterFinishReq
import org.pjstatecm.graceride.data.api.DevUser
import org.pjstatecm.graceride.data.api.LogoutReq
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.OkResponse
import org.pjstatecm.graceride.data.api.PasskeyFinishReq
import org.pjstatecm.graceride.data.api.PasskeyStart
import org.pjstatecm.graceride.data.api.PasskeyStartReq
import org.pjstatecm.graceride.data.api.TotpCodeReq
import org.pjstatecm.graceride.data.api.TotpEnableRespDto
import org.pjstatecm.graceride.data.api.TotpSetupDto
import org.pjstatecm.graceride.data.api.TurnstileConfigDto
import org.pjstatecm.graceride.data.api.UnlockReq
import org.pjstatecm.graceride.data.api.apiCall
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.session.OfflineSnapshotStore
import org.pjstatecm.graceride.data.session.PersistentCookieJar
import org.pjstatecm.graceride.data.session.SecureStore
import org.pjstatecm.graceride.platform.BiometricAuthManager
import org.pjstatecm.graceride.platform.BiometricSignResult

sealed interface AuthState {
    data object Loading : AuthState
    data object SignedOut : AuthState
    data class SignedIn(val me: Me) : AuthState
    data class Locked(val me: Me?) : AuthState
    data class Pending(val me: Me) : AuthState
}

class AuthRepository(
    private val apiProvider: ApiProvider,
    private val bus: AuthEventBus,
    private val cookies: PersistentCookieJar,
    private val _state: MutableStateFlow<AuthState> = MutableStateFlow(AuthState.Loading),
    private val store: SecureStore? = null,
    private val onCleared: (() -> Unit)? = null,
    private val biometric: BiometricAuthManager? = null,
    private val onLockedCallback: (() -> Unit)? = null,
    private val onUserSwitched: ((Long) -> Unit)? = null,
) {
    private val api: ApiService get() = apiProvider.get()
    val state: StateFlow<AuthState> = _state.asStateFlow()

    fun userIdOf(st: AuthState = _state.value): Long? = when (st) {
        is AuthState.SignedIn -> st.me.id
        is AuthState.Locked -> st.me?.id
        is AuthState.Pending -> st.me.id
        else -> null
    }

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
        store?.putString(CACHED_ME_AT_KEY, System.currentTimeMillis().toString())
    }

    private fun loadCachedMe(): Me? {
        val raw = store?.getString(CACHED_ME_KEY) ?: return null
        return runCatching { apiJson.decodeFromString(Me.serializer(), raw) }.getOrNull()
    }

    private fun isCachedIdleExpired(m: Me): Boolean {
        val effectiveMins = when {
            m.autoLockMins > 0 -> m.autoLockMins
            m.autoLockMins == 0 -> m.sessionTimeoutMins
            else -> 0
        }
        val hasUnlockMethod = m.passkeyCount > 0 || m.biometricCount > 0 || m.totpEnabled || m.devMode
        if (effectiveMins <= 0 || m.status != "active" || !hasUnlockMethod) return false
        val savedAt = store?.getString(CACHED_ME_AT_KEY)?.toLongOrNull() ?: return true
        val elapsedMs = System.currentTimeMillis() - savedAt
        if (elapsedMs < 0L) return true
        return (elapsedMs / 60_000L) >= effectiveMins
    }

    @Synchronized
    private fun clearLocalSessionIfNeeded(preserveVisitCookies: Boolean = true) {
        val hasLocal = _state.value !is AuthState.SignedOut ||
            (if (preserveVisitCookies) cookies.hasMemberCookies() else cookies.hasCookies()) ||
            store?.getString(CACHED_ME_KEY) != null ||
            store?.getString(HIDE_LOCK_DETAILS_KEY) != null ||
            store?.getString(NOTIF_PREFS_KEY) != null
        _state.value = AuthState.SignedOut
        if (!hasLocal) return
        if (preserveVisitCookies) cookies.clearMemberCookies() else cookies.clear()
        store?.remove(CACHED_ME_KEY)
        store?.remove(CACHED_ME_AT_KEY)
        store?.remove(HIDE_LOCK_DETAILS_KEY)
        store?.remove(NOTIF_PREFS_KEY)
        store?.let { OfflineSnapshotStore.clear(it) }
        biometric?.resetAppUnlocked()
        biometric?.clearPendingOptIn()
        onCleared?.invoke()
    }

    suspend fun refresh(throwOnError: Boolean = false) {
        try {
            val m = apiCall(bus) { api.me() }
            withContext(NonCancellable + Dispatchers.IO) {
                val prevId = userIdOf() ?: loadCachedMe()?.id
                if (prevId != null && prevId != m.id) {
                    store?.remove(HIDE_LOCK_DETAILS_KEY)
                    store?.remove(NOTIF_PREFS_KEY)
                    store?.let { OfflineSnapshotStore.clear(it) }
                    biometric?.clearStaleEnrollmentIfOtherUser(m.id)
                    onUserSwitched?.invoke(prevId)
                }
                cacheMe(m)
                biometric?.ensureLoaded()
            }
            _state.value = stateFor(m)
        } catch (e: ApiException) {
            if (e.status == 401) {
                withContext(NonCancellable + Dispatchers.IO) { clearLocalSessionIfNeeded(preserveVisitCookies = true) }
                if (throwOnError) throw e
            } else {
                // Offline / transient error on cold-start: restore last verified profile when cookies
                // exist, or purge any stale PII if session cookies are absent/expired.
                if (_state.value is AuthState.Loading) {
                    val (cached, expired) = withContext(NonCancellable + Dispatchers.IO) {
                        biometric?.ensureLoaded()
                        if (!cookies.hasMemberCookies()) {
                            clearLocalSessionIfNeeded(preserveVisitCookies = true)
                            null to false
                        } else if (e.status == 0 || e.status >= 500 || e.status == 429) {
                            loadCachedMe()?.let { it to isCachedIdleExpired(it) } ?: (null to false)
                        } else {
                            null to false
                        }
                    }
                    _state.value = when {
                        cached == null -> AuthState.SignedOut
                        expired && !cached.locked -> AuthState.Locked(cached.copy(locked = true))
                        else -> stateFor(cached)
                    }
                }
                if (throwOnError) throw e
            }
        }
    }

    fun onUnauthorized() {
        clearLocalSessionIfNeeded(preserveVisitCookies = true)
    }

    @Synchronized
    fun onLocked() {
        val current = _state.value
        val lockedMe = me?.copy(locked = true)
        if (current is AuthState.Locked && (current.me != null || lockedMe == null)) {
            return
        }
        lockedMe?.let(::cacheMe)
        store?.let { OfflineSnapshotStore.clear(it) }
        onLockedCallback?.invoke()
        _state.value = AuthState.Locked(lockedMe)
    }

    fun onPending() {
        val m = me?.copy(status = "pending")
        if (m != null) {
            cacheMe(m)
            _state.value = AuthState.Pending(m)
        }
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
        biometric?.markAppUnlocked()
        withContext(NonCancellable) {
            refresh(throwOnError = true)
            me?.let { m ->
                withContext(Dispatchers.IO) {
                    biometric?.clearStaleEnrollmentIfOtherUser(m.id)
                    biometric?.offerOptInIfEligible(m.id)
                }
            }
        }
    }

    suspend fun turnstileConfig(): TurnstileConfigDto =
        apiCall(bus) { api.turnstileConfig() }

    suspend fun passkeyLoginStart(email: String, turnstileToken: String? = null): PasskeyStart =
        apiCall(bus) { api.passkeyLoginStart(PasskeyStartReq(email.trim(), turnstileToken)) }

    suspend fun passkeyLoginFinish(challengeId: String, credentialJson: String) {
        apiCall(bus) {
            api.passkeyLoginFinish(
                PasskeyFinishReq(challengeId, apiJson.parseToJsonElement(credentialJson))
            )
        }
        biometric?.markAppUnlocked()
        withContext(NonCancellable) {
            refresh(throwOnError = true)
            me?.let { m ->
                withContext(Dispatchers.IO) {
                    biometric?.clearStaleEnrollmentIfOtherUser(m.id)
                    biometric?.offerOptInIfEligible(m.id)
                }
            }
        }
    }

    suspend fun passkeyReauthStart(): PasskeyStart =
        apiCall(bus) { api.passkeyReauthStart() }

    suspend fun passkeyReauthFinish(challengeId: String, credentialJson: String) {
        apiCall(bus) {
            api.passkeyReauthFinish(
                PasskeyFinishReq(challengeId, apiJson.parseToJsonElement(credentialJson))
            )
        }
        biometric?.markAppUnlocked()
        refresh(throwOnError = true)
    }

    suspend fun passkeyRegisterStart(): PasskeyStart = apiCall(bus) { api.passkeyRegisterStart() }

    suspend fun passkeyRegisterFinish(
        challengeId: String,
        credentialJson: String,
        label: String = defaultBiometricDeviceLabel(),
    ) {
        apiCall(bus) {
            api.passkeyRegisterFinish(
                PasskeyFinishReq(
                    challenge_id = challengeId,
                    credential = apiJson.parseToJsonElement(credentialJson),
                    label = label.takeIf { it.isNotBlank() },
                )
            )
        }
        refresh(throwOnError = true)
    }

    suspend fun enrollBiometric(
        context: Context,
        title: String,
        subtitle: String,
        cancelText: String,
        label: String = defaultBiometricDeviceLabel(),
    ): BiometricSignResult {
        val bio = biometric ?: return BiometricSignResult.Error("Biometric manager unavailable")
        val currentMe = me ?: return BiometricSignResult.Error("Not signed in")
        val start = apiCall(bus) { api.biometricRegisterStart() }
        val payload = bio.buildSignedPayload("register", start.challenge)
        val signed = bio.driver.generateAndSign(
            context = context,
            payload = payload,
            title = title,
            subtitle = subtitle,
            cancelText = cancelText,
            attestationChallenge = start.challenge.toByteArray(Charsets.UTF_8),
        )
        return when (signed) {
            is BiometricSignResult.Success -> {
                val res = try {
                    apiCall(bus) {
                        api.biometricRegisterFinish(
                            BiometricRegisterFinishReq(
                                challenge_id = start.challenge_id,
                                public_key = signed.publicKeyB64Url,
                                signature = signed.signatureB64Url,
                                label = label,
                                attestation_chain = signed.attestationChain,
                            )
                        )
                    }
                } catch (t: Throwable) {
                    withContext(NonCancellable + Dispatchers.IO) {
                        bio.clearEnrollmentLocal()
                    }
                    throw t
                }
                val enrollment = BiometricEnrollment(
                    id = res.id,
                    credentialId = res.credential_id,
                    userId = currentMe.id,
                    email = currentMe.email,
                    userName = currentMe.name,
                    label = res.label,
                    enrolledAtMs = System.currentTimeMillis(),
                )
                withContext(NonCancellable + Dispatchers.IO) {
                    bio.saveEnrollment(enrollment)
                }
                bio.markAppUnlocked()
                refresh(throwOnError = true)
                signed
            }
            BiometricSignResult.Cancelled -> {
                if (bio.enrollment.value != null) {
                    withContext(NonCancellable + Dispatchers.IO) {
                        bio.clearEnrollmentLocal()
                    }
                }
                BiometricSignResult.Cancelled
            }
            BiometricSignResult.KeyInvalidated -> {
                withContext(NonCancellable + Dispatchers.IO) {
                    bio.handleKeyInvalidated()
                }
                BiometricSignResult.KeyInvalidated
            }
            is BiometricSignResult.Error -> {
                if (bio.enrollment.value != null) {
                    withContext(NonCancellable + Dispatchers.IO) {
                        bio.clearEnrollmentLocal()
                    }
                }
                signed
            }
        }
    }

    suspend fun loginWithBiometric(
        context: Context,
        turnstileToken: String? = null,
        title: String,
        subtitle: String,
        cancelText: String,
    ): BiometricSignResult {
        val bio = biometric ?: return BiometricSignResult.Error("Biometric manager unavailable")
        withContext(Dispatchers.IO) { bio.ensureLoaded() }
        val enrolled = bio.enrollment.value
            ?: return BiometricSignResult.Error("No biometric enrollment found on this device")
        val start = apiCall(bus) {
            api.biometricLoginStart(BiometricLoginStartReq(enrolled.credentialId, turnstileToken))
        }
        val payload = bio.buildSignedPayload("login", start.challenge)
        return when (
            val signed = bio.driver.signWithEnrolledKey(context, payload, title, subtitle, cancelText)
        ) {
            is BiometricSignResult.Success -> {
                apiCall(bus) {
                    api.biometricLoginFinish(
                        BiometricFinishReq(
                            challenge_id = start.challenge_id,
                            credential_id = enrolled.credentialId,
                            signature = signed.signatureB64Url,
                        )
                    )
                }
                bio.markAppUnlocked()
                withContext(NonCancellable) {
                    refresh(throwOnError = true)
                    val latest = me
                    withContext(Dispatchers.IO) {
                        val finalEmail = latest?.email ?: enrolled.email
                        if (finalEmail.isNotBlank()) {
                            store?.putString("last_email", finalEmail)
                        }
                        if (latest != null) {
                            bio.saveEnrollment(
                                enrolled.copy(
                                    userId = latest.id,
                                    email = latest.email,
                                    userName = latest.name,
                                )
                            )
                        }
                    }
                }
                signed
            }
            BiometricSignResult.Cancelled -> BiometricSignResult.Cancelled
            BiometricSignResult.KeyInvalidated -> {
                withContext(NonCancellable + Dispatchers.IO) {
                    bio.handleKeyInvalidated()
                }
                BiometricSignResult.KeyInvalidated
            }
            is BiometricSignResult.Error -> signed
        }
    }

    suspend fun reauthWithBiometric(
        context: Context,
        title: String,
        subtitle: String,
        cancelText: String,
    ): BiometricSignResult {
        val bio = biometric ?: return BiometricSignResult.Error("Biometric manager unavailable")
        withContext(Dispatchers.IO) { bio.ensureLoaded() }
        val enrolled = bio.enrollment.value
            ?: return BiometricSignResult.Error("No biometric enrollment found on this device")
        val start = apiCall(bus) {
            api.biometricReauthStart(BiometricReauthStartReq(enrolled.credentialId))
        }
        val payload = bio.buildSignedPayload("reauth", start.challenge)
        return when (
            val signed = bio.driver.signWithEnrolledKey(context, payload, title, subtitle, cancelText)
        ) {
            is BiometricSignResult.Success -> {
                apiCall(bus) {
                    api.biometricReauthFinish(
                        BiometricFinishReq(
                            challenge_id = start.challenge_id,
                            credential_id = enrolled.credentialId,
                            signature = signed.signatureB64Url,
                        )
                    )
                }
                bio.markAppUnlocked()
                refresh(throwOnError = true)
                signed
            }
            BiometricSignResult.Cancelled -> BiometricSignResult.Cancelled
            BiometricSignResult.KeyInvalidated -> {
                withContext(NonCancellable + Dispatchers.IO) {
                    bio.handleKeyInvalidated()
                }
                BiometricSignResult.KeyInvalidated
            }
            is BiometricSignResult.Error -> signed
        }
    }

    suspend fun biometrics(): List<BiometricInfo> = apiCall(bus) { api.biometrics() }

    suspend fun deleteBiometric(id: Long) {
        apiCall(bus) { api.deleteBiometric(id) }
        val enrolled = biometric?.enrollment?.value
        if (enrolled != null && enrolled.id == id) {
            withContext(NonCancellable + Dispatchers.IO) {
                biometric.clearEnrollmentLocal()
            }
        }
        refresh(throwOnError = false)
    }

    suspend fun unlock(code: String, isBackupCode: Boolean) {
        apiCall(bus) {
            api.unlock(
                if (isBackupCode) UnlockReq(backup_code = code) else UnlockReq(totp_code = code)
            )
        }
        biometric?.markAppUnlocked()
        refresh(throwOnError = true)
    }

    suspend fun devUnlock() {
        apiCall(bus) { api.unlock(UnlockReq(dev_unlock = true)) }
        biometric?.markAppUnlocked()
        refresh(throwOnError = true)
    }

    suspend fun logout(mobileToken: String? = null) {
        withContext(NonCancellable + Dispatchers.IO) {
            try {
                apiCall(bus) {
                    api.logout(LogoutReq(mobile_token = mobileToken?.takeIf { it.isNotBlank() }))
                }
            } catch (_: ApiException) {
                // Even if the server call fails, drop the local credential.
            } finally {
                clearLocalSessionIfNeeded(preserveVisitCookies = false)
            }
        }
    }

    suspend fun logoutAll() {
        withContext(NonCancellable + Dispatchers.IO) {
            try {
                apiCall(bus) { api.logoutAll() }
            } catch (_: ApiException) {
            } finally {
                clearLocalSessionIfNeeded(preserveVisitCookies = false)
            }
        }
    }

    suspend fun lockSession() {
        val res = apiCall(bus) { api.lock() }
        if (res.loggedOut) {
            withContext(NonCancellable + Dispatchers.IO) {
                clearLocalSessionIfNeeded(preserveVisitCookies = true)
            }
        } else {
            onLocked()
        }
    }

    suspend fun totpSetup(): TotpSetupDto = apiCall(bus) { api.totpSetup() }

    suspend fun totpEnable(code: String): TotpEnableRespDto {
        val res = apiCall(bus) { api.totpEnable(TotpCodeReq(code.trim())) }
        refresh(throwOnError = false)
        return res
    }

    suspend fun totpVerify(code: String): OkResponse {
        val res = apiCall(bus) { api.totpVerify(TotpCodeReq(code.trim())) }
        refresh(throwOnError = false)
        return res
    }

    suspend fun regenerateBackupCodes(): BackupCodesRespDto {
        val res = apiCall(bus) { api.regenerateBackupCodes() }
        refresh(throwOnError = false)
        return res
    }

    suspend fun acceptTerms() {
        apiCall(bus) { api.acceptTerms(org.pjstatecm.graceride.data.api.TermsReq(accepted = true)) }
        refresh(throwOnError = true)
    }

    companion object {
        private const val CACHED_ME_KEY = "cached_me_v1"
        private const val CACHED_ME_AT_KEY = "cached_me_at_ms"
        private const val HIDE_LOCK_DETAILS_KEY = "notif_hide_lock_details"
        const val NOTIF_PREFS_KEY = "notif_prefs_v1"

        fun defaultBiometricDeviceLabel(): String {
            val mfg = Build.MANUFACTURER?.trim().orEmpty()
            val model = Build.MODEL?.trim().orEmpty()
            val combined = when {
                model.isBlank() && mfg.isBlank() -> ""
                model.isBlank() -> mfg
                mfg.isBlank() || model.startsWith(mfg, ignoreCase = true) -> model
                else -> "$mfg $model"
            }
            return combined.take(60).ifBlank { "Android device" }
        }
    }
}
