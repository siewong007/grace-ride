package org.pjstatecm.graceride.ui.screens.help

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.GuideDetailDto
import org.pjstatecm.graceride.di.AppContainer

data class HelpGuideDetailUiState(
    val loading: Boolean = true,
    val guide: GuideDetailDto? = null,
    val myFeedback: Boolean? = null,
    val feedbackBusy: Boolean = false,
    val error: String? = null,
    val message: String? = null,
)

class HelpGuideDetailViewModel(
    private val container: AppContainer,
    private val key: String,
) : ViewModel() {
    private val _state = MutableStateFlow(HelpGuideDetailUiState())
    val state: StateFlow<HelpGuideDetailUiState> = _state.asStateFlow()

    init {
        load()
    }

    fun load() {
        _state.value = _state.value.copy(loading = _state.value.guide == null, error = null)
        viewModelScope.launch {
            try {
                val g = container.help.guideDetail(key)
                _state.value = _state.value.copy(
                    loading = false,
                    guide = g,
                    myFeedback = g.myFeedback,
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(loading = false, error = e.message)
            }
        }
    }

    fun sendFeedback(helpful: Boolean, thanksMsg: String) {
        val guideId = _state.value.guide?.id ?: return
        if (_state.value.feedbackBusy) return
        _state.value = _state.value.copy(feedbackBusy = true)
        viewModelScope.launch {
            try {
                container.help.guideFeedback(guideId, helpful)
                _state.value = _state.value.copy(
                    feedbackBusy = false,
                    myFeedback = helpful,
                    message = thanksMsg,
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(feedbackBusy = false, message = e.message)
            }
        }
    }

    fun clearMessage() {
        _state.value = _state.value.copy(message = null)
    }

    companion object {
        fun factory(container: AppContainer, key: String) = viewModelFactory {
            initializer { HelpGuideDetailViewModel(container, key) }
        }
    }
}
