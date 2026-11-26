package org.pjstatecm.graceride.ui.screens.profile

import androidx.annotation.StringRes
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
import org.pjstatecm.graceride.data.api.FamilyAddPersonRespDto
import org.pjstatecm.graceride.data.api.FamilyStateDto
import org.pjstatecm.graceride.di.AppContainer

data class FamilyUiState(
    val loading: Boolean = true,
    val data: FamilyStateDto = FamilyStateDto(),
    val createName: String = "",
    val renameName: String = "",
    val addPhone: String = "",
    val busy: Boolean = false,
    val lastAdded: FamilyAddPersonRespDto? = null,
    @param:StringRes @get:StringRes val messageRes: Int? = null,
    val messageArg: String? = null,
    val error: String? = null,
)

class FamilyViewModel(private val container: AppContainer) : ViewModel() {
    private val _state = MutableStateFlow(FamilyUiState())
    val state: StateFlow<FamilyUiState> = _state.asStateFlow()

    init {
        load()
    }

    fun load() {
        viewModelScope.launch {
            if (_state.value.data.family == null && _state.value.data.incoming == null) {
                _state.value = _state.value.copy(loading = true, error = null)
            }
            try {
                val res = container.community.family()
                _state.value = _state.value.copy(
                    loading = false,
                    data = res,
                    renameName = res.family?.name.orEmpty(),
                    error = null,
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(loading = false, error = e.message)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _state.value = _state.value.copy(loading = false, error = e.message)
            }
        }
    }

    fun setCreateName(value: String) {
        _state.value = _state.value.copy(createName = value, error = null)
    }

    fun setRenameName(value: String) {
        _state.value = _state.value.copy(renameName = value, error = null)
    }

    fun setAddPhone(value: String) {
        _state.value = _state.value.copy(addPhone = value, error = null)
    }

    fun clearFeedback() {
        _state.value = _state.value.copy(error = null, messageRes = null, messageArg = null)
    }

    fun createFamily() {
        val name = _state.value.createName.trim()
        if (name.isEmpty() || _state.value.busy) return
        runAction {
            container.community.createFamily(name)
            _state.value = _state.value.copy(createName = "")
        }
    }

    fun renameFamily() {
        val name = _state.value.renameName.trim()
        if (name.isEmpty() || _state.value.busy) return
        runAction { container.community.renameFamily(name) }
    }

    fun deleteFamily() {
        if (_state.value.busy) return
        runAction { container.community.deleteFamily() }
    }

    fun leaveFamily() {
        if (_state.value.busy) return
        runAction { container.community.leaveFamily() }
    }

    fun addPerson() {
        val phone = _state.value.addPhone.trim()
        if (phone.isEmpty() || _state.value.busy) return
        _state.value = _state.value.copy(busy = true, error = null, lastAdded = null)
        viewModelScope.launch {
            try {
                val res = container.community.addFamilyPerson(phone)
                val updated = container.community.family()
                val msgRes = if (res.kind == "request") {
                    R.string.family_person_requested
                } else {
                    R.string.family_invite_ready
                }
                _state.value = _state.value.copy(
                    busy = false,
                    addPhone = "",
                    data = updated,
                    renameName = updated.family?.name.orEmpty(),
                    lastAdded = res,
                    messageRes = msgRes,
                    messageArg = res.name ?: res.phone ?: phone,
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, error = e.message)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _state.value = _state.value.copy(busy = false, error = e.message)
            }
        }
    }

    fun removeMember(userId: Long) {
        if (_state.value.busy) return
        runAction { container.community.removeFamilyMember(userId) }
    }

    fun cancelRequest(id: Long) {
        if (_state.value.busy) return
        runAction { container.community.cancelFamilyRequest(id) }
    }

    fun acceptIncoming(id: Long) {
        if (_state.value.busy) return
        runAction { container.community.acceptFamilyRequest(id) }
    }

    fun declineIncoming(id: Long) {
        if (_state.value.busy) return
        runAction { container.community.declineFamilyRequest(id) }
    }

    fun revokeInvite(id: Long) {
        if (_state.value.busy) return
        runAction { container.community.revokeFamilyInvite(id) }
    }

    private fun runAction(block: suspend () -> Unit) {
        _state.value = _state.value.copy(busy = true, error = null)
        viewModelScope.launch {
            try {
                block()
                val updated = container.community.family()
                _state.value = _state.value.copy(
                    busy = false,
                    data = updated,
                    renameName = updated.family?.name.orEmpty(),
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, error = e.message)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _state.value = _state.value.copy(busy = false, error = e.message)
            }
        }
    }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory {
            initializer { FamilyViewModel(container) }
        }
    }
}
