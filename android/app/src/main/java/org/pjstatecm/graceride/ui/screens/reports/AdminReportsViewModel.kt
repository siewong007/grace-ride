package org.pjstatecm.graceride.ui.screens.reports

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.AdminUserDto
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.DemandReportDto
import org.pjstatecm.graceride.data.api.EventsReportDto
import org.pjstatecm.graceride.data.api.FinanceReportDto
import org.pjstatecm.graceride.data.api.MonthReportDto
import org.pjstatecm.graceride.data.api.OverviewReportDto
import org.pjstatecm.graceride.data.api.ReportCapabilitiesDto
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import java.time.YearMonth
import java.util.Locale

data class AdminReportsUiState(
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val caps: ReportCapabilitiesDto = ReportCapabilitiesDto(),
    val tab: String = "member",
    val month: String = KlTime.today().take(7),
    val range: String = "monthly",
    val members: List<AdminUserDto> = emptyList(),
    val selectedUserId: Long? = null,
    val memberReport: MonthReportDto? = null,
    val overview: OverviewReportDto? = null,
    val eventsReport: EventsReportDto? = null,
    val demandReport: DemandReportDto? = null,
    val financeReport: FinanceReportDto? = null,
    val busy: Boolean = false,
    val error: String? = null,
    val message: String? = null,
)

class AdminReportsViewModel(
    private val container: AppContainer,
    initialTab: String? = null,
) : ViewModel() {
    private val _state = MutableStateFlow(
        AdminReportsUiState(
            tab = initialTab?.takeIf { it in setOf("member", "overview", "events", "demand", "finance") } ?: "member",
            selectedUserId = container.auth.me?.id,
        ),
    )
    val state: StateFlow<AdminReportsUiState> = _state.asStateFlow()
    private var loadJob: Job? = null

    init {
        load(first = true)
    }

    fun selectTab(tab: String) {
        if (_state.value.tab == tab) return
        _state.value = _state.value.copy(tab = tab)
        load(first = false)
    }

    fun selectMember(userId: Long) {
        if (_state.value.selectedUserId == userId) return
        _state.value = _state.value.copy(selectedUserId = userId)
        load(first = false)
    }

    fun shiftMonth(deltaMonths: Long) {
        val ym = runCatching { YearMonth.parse(_state.value.month) }.getOrNull() ?: return
        _state.value = _state.value.copy(month = ym.plusMonths(deltaMonths).toString())
        load(first = false)
    }

    fun setRange(range: String) {
        if (_state.value.range == range) return
        _state.value = _state.value.copy(range = range)
        load(first = false)
    }

    fun refresh() = load(first = false)

    fun load(first: Boolean = false) {
        loadJob?.cancel()
        loadJob = viewModelScope.launch {
            _state.value = _state.value.copy(
                loading = first && _state.value.memberReport == null && _state.value.overview == null,
                refreshing = !first,
                error = null,
            )
            try {
                val caps = if (first || !_state.value.caps.let { it.overview || it.events || it.demand || it.finance || it.memberReportsGlobal }) {
                    runCatching { container.help.reportCapabilities() }.getOrDefault(_state.value.caps)
                } else {
                    _state.value.caps
                }
                val s = _state.value
                val validTab = when (s.tab) {
                    "overview" -> if (caps.overview) "overview" else "member"
                    "events" -> if (caps.events) "events" else "member"
                    "demand" -> if (caps.demand) "demand" else "member"
                    "finance" -> if (caps.finance) "finance" else "member"
                    else -> "member"
                }
                val lang = if (Locale.getDefault().language == "zh") "zh" else "en"
                val membersList = if (caps.memberReportsGlobal && s.members.isEmpty()) {
                    container.admin.users(page = 1, limit = 100).getOrNull()?.items ?: emptyList()
                } else {
                    s.members
                }
                when (validTab) {
                    "overview" -> {
                        val ov = container.help.overviewReport(month = s.month, range = s.range)
                        _state.value = _state.value.copy(
                            loading = false, refreshing = false, caps = caps, tab = validTab,
                            members = membersList, overview = ov,
                        )
                    }
                    "events" -> {
                        val ev = container.help.eventsReport(month = s.month, range = s.range)
                        _state.value = _state.value.copy(
                            loading = false, refreshing = false, caps = caps, tab = validTab,
                            members = membersList, eventsReport = ev,
                        )
                    }
                    "demand" -> {
                        val dm = container.help.demandReport(month = s.month, range = s.range)
                        _state.value = _state.value.copy(
                            loading = false, refreshing = false, caps = caps, tab = validTab,
                            members = membersList, demandReport = dm,
                        )
                    }
                    "finance" -> {
                        val fn = container.help.financeReport(month = s.month, range = s.range)
                        _state.value = _state.value.copy(
                            loading = false, refreshing = false, caps = caps, tab = validTab,
                            members = membersList, financeReport = fn,
                        )
                    }
                    else -> {
                        val targetId = if (caps.memberReportsGlobal) s.selectedUserId else null
                        val mr = container.help.monthlyReport(
                            month = s.month,
                            range = s.range,
                            userId = targetId,
                            lang = lang,
                        )
                        _state.value = _state.value.copy(
                            loading = false, refreshing = false, caps = caps, tab = validTab,
                            members = membersList, memberReport = mr,
                        )
                    }
                }
            } catch (e: ApiException) {
                _state.value = _state.value.copy(loading = false, refreshing = false, error = e.message)
            }
        }
    }

    fun updateBadgeIcon(userId: Long, badgeKey: String, icon: String) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                container.help.updateUserBadgeIcon(userId, badgeKey, icon)
                _state.value = _state.value.copy(busy = false)
                load(first = false)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, message = e.message)
            }
        }
    }

    fun clearMessage() {
        _state.value = _state.value.copy(message = null)
    }

    companion object {
        fun factory(container: AppContainer, initialTab: String?) = viewModelFactory {
            initializer { AdminReportsViewModel(container, initialTab) }
        }
    }
}
