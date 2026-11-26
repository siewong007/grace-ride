package org.pjstatecm.graceride.ui.screens.public

import androidx.annotation.StringRes
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.CareStatusDto
import org.pjstatecm.graceride.di.AppContainer

data class CarePassUiState(
    val loading: Boolean = true,
    val status: CareStatusDto? = null,
    val updating: Boolean = false,
    val closedError: Boolean = false,
    val loadError: String? = null,
    val actionError: String? = null,
    @param:StringRes @get:StringRes val toastRes: Int? = null,
)

class CarePassViewModel(
    private val container: AppContainer,
    private val token: String,
) : ViewModel() {
    private val _state = MutableStateFlow(CarePassUiState())
    val state: StateFlow<CarePassUiState> = _state.asStateFlow()

    init {
        refresh(initial = true)
        viewModelScope.launch {
            while (true) {
                delay(15_000L)
                if (!_state.value.closedError && _state.value.loadError == null) {
                    refresh(initial = false)
                }
            }
        }
    }

    fun refresh(initial: Boolean = true) {
        if (initial) _state.value = CarePassUiState(loading = true)
        viewModelScope.launch {
            try {
                val dto = container.publicFlow.careStatus(token)
                _state.value = _state.value.copy(
                    loading = false,
                    status = dto,
                    closedError = false,
                    loadError = null,
                )
            } catch (e: ApiException) {
                if (e.status == 404 || e.status == 410) {
                    _state.value = CarePassUiState(loading = false, closedError = true, loadError = e.message)
                } else {
                    _state.value = _state.value.copy(loading = false, loadError = e.message)
                }
            } catch (e: Exception) {
                if (initial) {
                    _state.value = _state.value.copy(loading = false, loadError = e.message)
                }
            }
        }
    }

    fun sendReadiness(status: String) {
        if (_state.value.updating) return
        _state.value = _state.value.copy(updating = true, actionError = null)
        viewModelScope.launch {
            try {
                container.publicFlow.careReadiness(token, status)
                val updated = container.publicFlow.careStatus(token)
                _state.value = _state.value.copy(
                    updating = false,
                    status = updated,
                    toastRes = if (status == "ready") R.string.visit_ready_sent else R.string.visit_late_sent,
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(updating = false, actionError = e.message)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _state.value = _state.value.copy(updating = false, actionError = e.message)
            }
        }
    }

    fun clearToast() {
        _state.value = _state.value.copy(toastRes = null)
    }

    fun clearActionError() {
        _state.value = _state.value.copy(actionError = null)
    }

    companion object {
        fun factory(container: AppContainer, token: String) = viewModelFactory {
            initializer { CarePassViewModel(container, token) }
        }
    }
}
