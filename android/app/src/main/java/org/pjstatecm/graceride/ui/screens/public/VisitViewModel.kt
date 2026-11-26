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
import org.pjstatecm.graceride.data.api.RideLocationDto
import org.pjstatecm.graceride.data.api.VisitPreviewDto
import org.pjstatecm.graceride.data.api.VisitRideDto
import org.pjstatecm.graceride.di.AppContainer

data class VisitUiState(
    val loading: Boolean = true,
    val preview: VisitPreviewDto? = null,
    val ride: VisitRideDto? = null,
    val location: RideLocationDto? = null,
    val claiming: Boolean = false,
    val updatingReadiness: Boolean = false,
    val readinessToast: String? = null,
    @param:StringRes @get:StringRes val readinessToastRes: Int? = null,
    val error: String? = null,
    val loadError: String? = null,
    val turnstileEnabled: Boolean = false,
    val turnstileSiteKey: String? = null,
    val turnstilePrompted: Boolean = false,
)

class VisitViewModel(
    private val container: AppContainer,
    private val token: String,
) : ViewModel() {
    private val siteKeyRe = Regex("^[A-Za-z0-9_-]+$")
    private val _state = MutableStateFlow(VisitUiState())
    val state: StateFlow<VisitUiState> = _state.asStateFlow()

    init {
        load()
        viewModelScope.launch {
            while (true) {
                delay(15_000L)
                if (_state.value.ride != null) {
                    refreshClaimed(silent = true)
                }
            }
        }
    }

    fun load() {
        _state.value = VisitUiState(loading = true)
        viewModelScope.launch {
            try {
                val cfg = runCatching { container.publicFlow.turnstileConfig() }.getOrNull()
                val validKey = cfg?.siteKey?.takeIf { siteKeyRe.matches(it) }
                val preview = container.publicFlow.visitPreview(token)
                _state.value = _state.value.copy(
                    loading = false,
                    preview = preview,
                    turnstileEnabled = cfg?.enabled == true && validKey != null,
                    turnstileSiteKey = validKey,
                )
                if (preview.state == "mine") {
                    refreshClaimed(silent = false)
                }
            } catch (e: ApiException) {
                _state.value = VisitUiState(loading = false, loadError = e.message)
            } catch (e: Exception) {
                _state.value = VisitUiState(loading = false, loadError = e.message)
            }
        }
    }

    fun dismissTurnstile() {
        _state.value = _state.value.copy(turnstilePrompted = false)
    }

    fun claim(turnstileToken: String? = null) {
        val s = _state.value
        if (s.claiming) return
        if (s.turnstileEnabled && turnstileToken.isNullOrBlank()) {
            _state.value = s.copy(error = null, turnstilePrompted = true)
            return
        }
        _state.value = s.copy(claiming = true, error = null, turnstilePrompted = false)
        viewModelScope.launch {
            try {
                container.publicFlow.visitClaim(token, turnstileToken)
                refreshClaimed(silent = false)
            } catch (e: ApiException) {
                if (e.code == "TURNSTILE_REQUIRED" && turnstileToken.isNullOrBlank()) {
                    val cfg = runCatching { container.publicFlow.turnstileConfig() }.getOrNull()
                    val validKey = cfg?.siteKey?.takeIf { siteKeyRe.matches(it) }
                    if (cfg?.enabled == true && validKey != null) {
                        _state.value = _state.value.copy(
                            claiming = false,
                            turnstileEnabled = true,
                            turnstileSiteKey = validKey,
                            turnstilePrompted = true,
                        )
                        return@launch
                    }
                }
                _state.value = _state.value.copy(claiming = false, error = e.message)
                load()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _state.value = _state.value.copy(claiming = false, error = e.message)
            }
        }
    }

    fun refreshClaimed(silent: Boolean = false) {
        viewModelScope.launch {
            try {
                val ride = container.publicFlow.visitMe()
                val loc = runCatching { container.publicFlow.visitLocation() }.getOrNull()
                _state.value = _state.value.copy(
                    loading = false,
                    claiming = false,
                    ride = ride,
                    location = loc ?: _state.value.location,
                )
            } catch (e: ApiException) {
                if (e.status == 401 || e.status == 403 || e.status == 410) {
                    _state.value = _state.value.copy(ride = null, location = null)
                    if (!silent) load()
                } else if (!silent) {
                    _state.value = _state.value.copy(claiming = false, error = e.message)
                }
            } catch (_: Exception) {
            }
        }
    }

    fun sendReadiness(status: String, notes: String? = null) {
        if (_state.value.updatingReadiness) return
        _state.value = _state.value.copy(updatingReadiness = true, error = null)
        viewModelScope.launch {
            try {
                container.publicFlow.visitReadiness(status, notes?.trim()?.takeIf { it.isNotEmpty() })
                val ride = container.publicFlow.visitMe()
                val toastRes = when (status) {
                    "ready" -> R.string.visit_ready_sent
                    "delayed_5m" -> R.string.visit_late_sent
                    else -> null
                }
                _state.value = _state.value.copy(
                    updatingReadiness = false,
                    ride = ride,
                    readinessToastRes = toastRes,
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(updatingReadiness = false, error = e.message)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _state.value = _state.value.copy(updatingReadiness = false, error = e.message)
            }
        }
    }

    fun clearToast() {
        _state.value = _state.value.copy(readinessToast = null, readinessToastRes = null)
    }

    fun clearError() {
        _state.value = _state.value.copy(error = null)
    }

    companion object {
        fun factory(container: AppContainer, token: String) = viewModelFactory {
            initializer { VisitViewModel(container, token) }
        }
    }
}
