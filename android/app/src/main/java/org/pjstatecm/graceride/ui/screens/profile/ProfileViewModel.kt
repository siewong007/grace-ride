package org.pjstatecm.graceride.ui.screens.profile

import android.content.Context
import android.net.Uri
import androidx.credentials.CreatePublicKeyCredentialRequest
import androidx.credentials.CredentialManager
import androidx.credentials.GetCredentialRequest
import androidx.credentials.GetPublicKeyCredentialOption
import androidx.credentials.PublicKeyCredential
import androidx.credentials.exceptions.CreateCredentialCancellationException
import androidx.credentials.exceptions.CreateCredentialException
import androidx.credentials.exceptions.GetCredentialCancellationException
import androidx.credentials.exceptions.GetCredentialException
import org.pjstatecm.graceride.R
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.BiometricEnrollment
import org.pjstatecm.graceride.data.api.BiometricInfo
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.NotificationPrefsDto
import org.pjstatecm.graceride.data.api.PasskeyInfo
import org.pjstatecm.graceride.data.api.ProfileUpdateReq
import org.pjstatecm.graceride.data.api.UpdateNotificationPrefsReq
import org.pjstatecm.graceride.data.media.prepareImageUpload
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.platform.BiometricSignResult

data class ProfileUiState(
    val me: Me? = null,
    val passkeys: List<PasskeyInfo> = emptyList(),
    val biometrics: List<BiometricInfo> = emptyList(),
    val notifPrefs: NotificationPrefsDto? = null,
    val busy: Boolean = false,
    val message: String? = null,
    val error: String? = null,
    val lang: String = "zh",
)

class ProfileViewModel(private val container: AppContainer) : ViewModel() {

    private val _state = MutableStateFlow(ProfileUiState(me = container.auth.me, lang = container.lang))
    val state: StateFlow<ProfileUiState> = _state.asStateFlow()
    val biometricEnrollment: StateFlow<BiometricEnrollment?> = container.biometric.enrollment

    init {
        loadPasskeys()
    }

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

    fun loadBiometrics() {
        viewModelScope.launch {
            try {
                val list = container.auth.biometrics()
                val enrolled = container.biometric.enrollment.value
                val currentId = container.auth.me?.id
                if (enrolled != null && currentId != null && enrolled.userId == currentId &&
                    list.none { it.credential_id == enrolled.credentialId }
                ) {
                    withContext(Dispatchers.IO) { container.biometric.clearEnrollmentLocal() }
                }
                _state.value = _state.value.copy(biometrics = list)
            } catch (_: ApiException) { }
        }
    }

    fun loadNotificationPrefs() {
        viewModelScope.launch {
            try {
                val prefs = container.community.notificationPrefs()
                kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
                    org.pjstatecm.graceride.notifications.RealtimeStreamClient.writeCachedPrefs(container.secureStore, prefs)
                }
                _state.value = _state.value.copy(notifPrefs = prefs)
            } catch (_: ApiException) { }
        }
    }

    fun clearMessage() {
        _state.value = _state.value.copy(message = null)
    }

    fun clearError() {
        _state.value = _state.value.copy(error = null)
    }

    fun saveProfile(name: String, phone: String) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, error = null, message = null)
        viewModelScope.launch {
            try {
                container.community.updateProfile(
                    ProfileUpdateReq(
                        name = name.trim(),
                        phone = phone.trim(),
                    )
                )
                container.auth.refresh()
                _state.value = _state.value.copy(
                    busy = false,
                    me = container.auth.me,
                    message = "saved",
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, error = e.message)
            }
        }
    }

    fun saveNotificationPrefs(req: UpdateNotificationPrefsReq) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, error = null, message = null)
        viewModelScope.launch {
            try {
                container.community.updateNotificationPrefs(req)
                val prefs = container.community.notificationPrefs()
                kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
                    org.pjstatecm.graceride.notifications.RealtimeStreamClient.writeCachedPrefs(container.secureStore, prefs)
                }
                container.auth.refresh()
                _state.value = _state.value.copy(
                    busy = false,
                    me = container.auth.me,
                    notifPrefs = prefs,
                    message = "prefs_saved",
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, error = e.message)
            }
        }
    }

    fun saveAvatarStyle(avatarIcon: String, avatarColor: String, onDone: () -> Unit = {}) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, error = null, message = null)
        viewModelScope.launch {
            try {
                container.community.updateProfile(
                    ProfileUpdateReq(
                        clear_photo = true,
                        avatar_icon = avatarIcon,
                        avatar_color = avatarColor,
                    )
                )
                container.auth.refresh()
                _state.value = _state.value.copy(
                    busy = false,
                    me = container.auth.me,
                    message = "saved",
                )
                onDone()
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, error = e.message)
            }
        }
    }

    fun saveAvatarPhotoColor(photoFileId: Long, avatarColor: String, onDone: () -> Unit = {}) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, error = null, message = null)
        viewModelScope.launch {
            try {
                container.community.updateProfile(
                    ProfileUpdateReq(
                        photo_file_id = photoFileId,
                        avatar_color = avatarColor,
                    )
                )
                container.auth.refresh()
                _state.value = _state.value.copy(
                    busy = false,
                    me = container.auth.me,
                    message = "saved",
                )
                onDone()
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, error = e.message)
            }
        }
    }

    fun saveAvatarPhoto(context: Context, uri: Uri, avatarColor: String, onDone: () -> Unit = {}) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, error = null, message = null)
        viewModelScope.launch {
            try {
                val prepared = withContext(Dispatchers.IO) {
                    prepareImageUpload(
                        context = context,
                        uri = uri,
                        prefix = "avatar",
                        maxDimensionPx = 512,
                        quality = 85,
                    )
                }
                val uploaded = container.community.upload(prepared.bytes, prepared.filename, prepared.mime)
                container.community.updateProfile(
                    ProfileUpdateReq(
                        photo_file_id = uploaded.id,
                        avatar_color = avatarColor,
                    )
                )
                container.auth.refresh()
                _state.value = _state.value.copy(
                    busy = false,
                    me = container.auth.me,
                    message = "saved",
                )
                onDone()
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, error = e.message)
            } catch (e: Exception) {
                _state.value = _state.value.copy(busy = false, error = e.message ?: "Upload failed")
            }
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

    private suspend fun reauthWithPasskey(context: Context): Boolean {
        return try {
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
            true
        } catch (_: GetCredentialCancellationException) {
            false
        } catch (e: GetCredentialException) {
            _state.value = _state.value.copy(error = e.message ?: context.getString(R.string.reauth_failed))
            false
        } catch (e: ApiException) {
            _state.value = _state.value.copy(error = e.message)
            false
        } catch (e: Exception) {
            _state.value = _state.value.copy(error = e.message ?: context.getString(R.string.reauth_failed))
            false
        }
    }

    private suspend fun registerPasskeyOnce(context: Context) {
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
    }

    private suspend fun reauthWithStrongAuth(context: Context): Boolean {
        val currentId = container.auth.me?.id
        val enrolled = container.biometric.enrollment.value
        val hasBiometricHere = enrolled != null && (currentId == null || enrolled.userId == currentId)
        val hasPasskeys = (container.auth.me?.passkeyCount ?: _state.value.passkeys.size) > 0
        if (hasBiometricHere) {
            try {
                when (
                    val res = container.auth.reauthWithBiometric(
                        context = context,
                        title = context.getString(R.string.biometric_prompt_unlock_title),
                        subtitle = context.getString(R.string.biometric_prompt_unlock_subtitle),
                        cancelText = context.getString(R.string.common_cancel),
                    )
                ) {
                    is BiometricSignResult.Success -> return true
                    BiometricSignResult.Cancelled -> return false
                    BiometricSignResult.KeyInvalidated -> {
                        if (hasPasskeys) return reauthWithPasskey(context)
                        _state.value = _state.value.copy(error = context.getString(R.string.biometric_invalidated_notice))
                        return false
                    }
                    is BiometricSignResult.Error -> {
                        if (hasPasskeys) return reauthWithPasskey(context)
                        _state.value = _state.value.copy(error = res.message)
                        return false
                    }
                }
            } catch (e: ApiException) {
                if (hasPasskeys) return reauthWithPasskey(context)
                _state.value = _state.value.copy(error = e.message)
                return false
            } catch (e: Exception) {
                if (hasPasskeys) return reauthWithPasskey(context)
                _state.value = _state.value.copy(error = e.message ?: context.getString(R.string.biometric_failed))
                return false
            }
        }
        return if (hasPasskeys) reauthWithPasskey(context) else false
    }

    private suspend fun enrollBiometricOnce(context: Context): BiometricSignResult {
        return container.auth.enrollBiometric(
            context = context,
            title = context.getString(R.string.biometric_prompt_enroll_title),
            subtitle = context.getString(R.string.biometric_prompt_enroll_subtitle),
            cancelText = context.getString(R.string.common_cancel),
        )
    }

    fun enableBiometric(context: Context) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, error = null, message = null)
        viewModelScope.launch {
            suspend fun handleResult(res: BiometricSignResult) {
                when (res) {
                    is BiometricSignResult.Success -> {
                        val updated = runCatching { container.auth.biometrics() }.getOrNull()
                        _state.value = _state.value.copy(
                            me = container.auth.me,
                            biometrics = updated ?: _state.value.biometrics,
                            message = "biometric_enabled",
                        )
                    }
                    BiometricSignResult.Cancelled -> {}
                    BiometricSignResult.KeyInvalidated -> {
                        _state.value = _state.value.copy(error = context.getString(R.string.biometric_failed))
                    }
                    is BiometricSignResult.Error -> {
                        _state.value = _state.value.copy(error = res.message)
                    }
                }
            }
            try {
                handleResult(enrollBiometricOnce(context))
            } catch (e: ApiException) {
                val hasStrongAuth = (container.auth.me?.passkeyCount ?: _state.value.passkeys.size) > 0 ||
                    (container.auth.me?.biometricCount ?: _state.value.biometrics.size) > 0
                if (e.code == "RECENT_SIGN_IN_REQUIRED" && hasStrongAuth) {
                    if (reauthWithStrongAuth(context)) {
                        try {
                            handleResult(enrollBiometricOnce(context))
                        } catch (retryErr: ApiException) {
                            _state.value = _state.value.copy(error = retryErr.message)
                        } catch (retryErr: Exception) {
                            _state.value = _state.value.copy(error = retryErr.message ?: context.getString(R.string.biometric_failed))
                        }
                    }
                } else {
                    _state.value = _state.value.copy(error = e.message)
                }
            } catch (e: Exception) {
                _state.value = _state.value.copy(error = e.message ?: context.getString(R.string.biometric_failed))
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun disableCurrentBiometric(context: Context) {
        if (_state.value.busy) return
        val enrolled = container.biometric.enrollment.value
        if (enrolled == null) {
            container.biometric.clearEnrollmentLocal()
            return
        }
        val serverId = _state.value.biometrics
            .firstOrNull { it.credential_id == enrolled.credentialId }
            ?.id
            ?: enrolled.id.takeIf { it > 0L }
        if (serverId == null) {
            container.biometric.clearEnrollmentLocal()
            _state.value = _state.value.copy(message = "biometric_disabled")
            return
        }
        _state.value = _state.value.copy(busy = true, error = null, message = null)
        viewModelScope.launch {
            suspend fun syncAfterDisable() {
                withContext(Dispatchers.IO) { container.biometric.clearEnrollmentLocal() }
                val updated = runCatching { container.auth.biometrics() }.getOrNull()
                _state.value = _state.value.copy(
                    me = container.auth.me,
                    biometrics = updated ?: _state.value.biometrics.filterNot { it.id == serverId },
                    message = "biometric_disabled",
                )
            }
            try {
                container.auth.deleteBiometric(serverId)
                syncAfterDisable()
            } catch (e: ApiException) {
                if (e.code == "RECENT_SIGN_IN_REQUIRED") {
                    if (reauthWithStrongAuth(context)) {
                        try {
                            container.auth.deleteBiometric(serverId)
                            syncAfterDisable()
                        } catch (retryErr: ApiException) {
                            _state.value = _state.value.copy(error = retryErr.message)
                        }
                    }
                } else {
                    _state.value = _state.value.copy(error = e.message)
                }
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun deleteBiometric(id: Long, context: Context) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, error = null, message = null)
        viewModelScope.launch {
            suspend fun syncAfterDelete() {
                val updated = runCatching { container.auth.biometrics() }.getOrNull()
                _state.value = _state.value.copy(
                    me = container.auth.me,
                    biometrics = updated ?: _state.value.biometrics.filterNot { it.id == id },
                )
            }
            try {
                container.auth.deleteBiometric(id)
                syncAfterDelete()
            } catch (e: ApiException) {
                if (e.code == "RECENT_SIGN_IN_REQUIRED") {
                    if (reauthWithStrongAuth(context)) {
                        try {
                            container.auth.deleteBiometric(id)
                            syncAfterDelete()
                        } catch (retryErr: ApiException) {
                            _state.value = _state.value.copy(error = retryErr.message)
                        }
                    }
                } else {
                    _state.value = _state.value.copy(error = e.message)
                }
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun addPasskey(context: Context) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, error = null)
        viewModelScope.launch {
            try {
                registerPasskeyOnce(context)
            } catch (_: CreateCredentialCancellationException) {
                // User dismissed the passkey creation prompt cleanly.
            } catch (e: CreateCredentialException) {
                _state.value = _state.value.copy(error = e.message)
            } catch (e: ApiException) {
                val hasStrongAuth = (container.auth.me?.passkeyCount ?: _state.value.passkeys.size) > 0 ||
                    (container.auth.me?.biometricCount ?: _state.value.biometrics.size) > 0
                if (e.code == "RECENT_SIGN_IN_REQUIRED" && hasStrongAuth) {
                    if (reauthWithStrongAuth(context)) {
                        try {
                            registerPasskeyOnce(context)
                        } catch (_: CreateCredentialCancellationException) {
                        } catch (retryErr: Exception) {
                            _state.value = _state.value.copy(error = retryErr.message)
                        }
                    }
                } else {
                    _state.value = _state.value.copy(error = e.message)
                }
            } catch (e: Exception) {
                _state.value = _state.value.copy(error = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun deletePasskey(id: Long, context: Context) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, error = null)
        viewModelScope.launch {
            suspend fun syncAfterDelete() {
                container.auth.refresh()
                val updated = runCatching { container.community.passkeys() }.getOrNull()
                _state.value = _state.value.copy(
                    me = container.auth.me,
                    passkeys = updated ?: _state.value.passkeys.filterNot { it.id == id },
                )
            }
            try {
                container.community.deletePasskey(id)
                syncAfterDelete()
            } catch (e: ApiException) {
                if (e.code == "RECENT_SIGN_IN_REQUIRED") {
                    if (reauthWithStrongAuth(context)) {
                        try {
                            container.community.deletePasskey(id)
                            syncAfterDelete()
                        } catch (retryErr: ApiException) {
                            _state.value = _state.value.copy(error = retryErr.message)
                        }
                    }
                } else {
                    _state.value = _state.value.copy(error = e.message)
                }
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun logout() { viewModelScope.launch { container.auth.logout() } }
    fun logoutAll() { viewModelScope.launch { container.auth.logoutAll() } }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory { initializer { ProfileViewModel(container) } }
    }
}
