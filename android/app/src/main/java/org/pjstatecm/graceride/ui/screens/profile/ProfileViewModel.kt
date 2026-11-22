package org.pjstatecm.graceride.ui.screens.profile

import android.content.Context
import android.net.Uri
import androidx.credentials.CreatePublicKeyCredentialRequest
import androidx.credentials.CredentialManager
import androidx.credentials.exceptions.CreateCredentialException
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
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.PasskeyInfo
import org.pjstatecm.graceride.data.api.ProfileUpdateReq
import org.pjstatecm.graceride.di.AppContainer
import java.io.File

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

    fun clearMessage() {
        _state.value = _state.value.copy(message = null)
    }

    fun saveProfile(name: String, phone: String, telegramChatId: String) {
        if (_state.value.busy) return
        val normalizedTg = telegramChatId.trim().removePrefix("@")
        _state.value = _state.value.copy(busy = true, error = null, message = null)
        viewModelScope.launch {
            try {
                container.community.updateProfile(
                    ProfileUpdateReq(
                        name = name.trim(),
                        phone = phone.trim(),
                        telegram_chat_id = normalizedTg,
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

    fun saveAvatarStyle(avatarIcon: String, avatarColor: String) {
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
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, error = e.message)
            }
        }
    }

    fun saveAvatarPhotoColor(photoFileId: Long, avatarColor: String) {
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
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, error = e.message)
            }
        }
    }

    fun saveAvatarPhoto(context: Context, uri: Uri, avatarColor: String) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, error = null, message = null)
        viewModelScope.launch {
            var tmpFile: File? = null
            try {
                val mime = context.contentResolver.getType(uri) ?: "image/jpeg"
                val ext = when {
                    mime.equals("image/gif", ignoreCase = true) -> ".gif"
                    mime.equals("image/png", ignoreCase = true) -> ".png"
                    mime.equals("image/webp", ignoreCase = true) -> ".webp"
                    else -> ".jpg"
                }
                tmpFile = withContext(Dispatchers.IO) {
                    val f = File.createTempFile("avatar-", ext, context.cacheDir)
                    context.contentResolver.openInputStream(uri)?.use { input ->
                        f.outputStream().use { output -> input.copyTo(output) }
                    } ?: throw IllegalStateException("Cannot read selected image")
                    f
                }
                val uploaded = container.community.upload(tmpFile, mime)
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
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, error = e.message)
            } catch (e: Exception) {
                _state.value = _state.value.copy(busy = false, error = e.message ?: "Upload failed")
            } finally {
                tmpFile?.delete()
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
