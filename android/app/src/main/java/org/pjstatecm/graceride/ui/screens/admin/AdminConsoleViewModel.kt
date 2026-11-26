package org.pjstatecm.graceride.ui.screens.admin

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.AdminDashboardStatsDto
import org.pjstatecm.graceride.data.api.AuditItemDto
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime

enum class TodayRideFilter {
    ALL,
    UNASSIGNED,
    CONFIRMED,
    IN_PROGRESS,
}

data class AdminConsoleUiState(
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val error: String? = null,
    val dashboard: AdminDashboardStatsDto? = null,
    val todayRides: List<Ride> = emptyList(),
    val todayFilter: TodayRideFilter = TodayRideFilter.ALL,
    val recentAudit: List<AuditItemDto> = emptyList(),
)

class AdminConsoleViewModel(
    private val container: AppContainer,
) : ViewModel() {

    private val _state = MutableStateFlow(AdminConsoleUiState())
    val state: StateFlow<AdminConsoleUiState> = _state.asStateFlow()

    init {
        refresh(silent = false)
    }

    fun refresh(silent: Boolean = false) {
        val me = container.auth.me
        viewModelScope.launch {
            _state.update {
                it.copy(
                    loading = !silent && it.dashboard == null,
                    refreshing = silent || it.dashboard != null,
                    error = null,
                )
            }
            if (me?.can("admin_dashboard") != true) {
                _state.update { it.copy(loading = false, refreshing = false) }
                return@launch
            }

            coroutineScope {
                val dashDef = async { container.admin.dashboardStats(passive = false) }
                val todayDef = if (me.can("admin_rides")) {
                    async { container.traffic.adminRides(date = KlTime.today(), limit = 50) }
                } else null
                val auditDef = if (me.can("admin_audit")) {
                    async { container.admin.auditLog(limit = 5) }
                } else null

                val dashRes = dashDef.await()
                val todayRes = todayDef?.await()
                val auditRes = auditDef?.await()

                _state.update { cur ->
                    val nextDash = dashRes.getOrNull() ?: cur.dashboard
                    cur.copy(
                        loading = false,
                        refreshing = false,
                        error = if (nextDash == null) dashRes.exceptionOrNull()?.message else null,
                        dashboard = nextDash,
                        todayRides = todayRes?.getOrNull()?.items ?: cur.todayRides,
                        recentAudit = auditRes?.getOrNull()?.items ?: cur.recentAudit,
                    )
                }
            }
        }
    }

    fun setTodayFilter(filter: TodayRideFilter) {
        _state.update { it.copy(todayFilter = filter) }
    }

    companion object {
        fun factory(container: AppContainer): ViewModelProvider.Factory = viewModelFactory {
            initializer { AdminConsoleViewModel(container) }
        }
    }
}
