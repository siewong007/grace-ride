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
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.MonthReportDto
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import java.time.YearMonth
import java.util.Locale

data class ReportUiState(
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val month: String = KlTime.today().take(7),
    val range: String = "monthly",
    val type: String = "all",
    val report: MonthReportDto? = null,
    val error: String? = null,
)

class ReportViewModel(private val container: AppContainer) : ViewModel() {
    private val _state = MutableStateFlow(ReportUiState())
    val state: StateFlow<ReportUiState> = _state.asStateFlow()
    private var loadJob: Job? = null

    init {
        load(first = true)
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

    fun setType(type: String) {
        if (_state.value.type == type) return
        _state.value = _state.value.copy(type = type)
        load(first = false)
    }

    fun refresh() = load(first = false)

    fun load(first: Boolean = false) {
        loadJob?.cancel()
        val s = _state.value
        val lang = if (Locale.getDefault().language == "zh") "zh" else "en"
        loadJob = viewModelScope.launch {
            _state.value = _state.value.copy(
                loading = first && _state.value.report == null,
                refreshing = !first,
                error = null,
            )
            try {
                val r = container.help.monthlyReport(
                    month = s.month,
                    range = s.range,
                    type = s.type,
                    lang = lang,
                )
                _state.value = _state.value.copy(loading = false, refreshing = false, report = r)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(loading = false, refreshing = false, error = e.message)
            }
        }
    }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory {
            initializer { ReportViewModel(container) }
        }
    }
}
