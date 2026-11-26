package org.pjstatecm.graceride.ui.screens.rides

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.ChatMessageDto
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.notifications.RealtimeEvent

data class TripChatUiState(
    val loading: Boolean = true,
    val sending: Boolean = false,
    val open: Boolean = true,
    val canPost: Boolean = true,
    val members: List<String> = emptyList(),
    val reactionsAllowed: List<String> = listOf("👍", "❤️", "😂", "😮", "😢", "🙏"),
    val messages: List<ChatMessageDto> = emptyList(),
    val error: String? = null,
)

class TripChatViewModel(
    private val container: AppContainer,
    private val rideId: Long,
) : ViewModel() {

    private val _state = MutableStateFlow(TripChatUiState())
    val state: StateFlow<TripChatUiState> = _state.asStateFlow()
    private var loadJob: Job? = null
    @Volatile
    private var sheetVisible: Boolean = true

    init {
        load()
        viewModelScope.launch {
            container.realtimeStream.events.collect { ev ->
                if (sheetVisible && (ev == RealtimeEvent.Chat || ev == RealtimeEvent.Resume)) {
                    load(silent = true)
                }
            }
        }
    }

    fun setSheetVisible(visible: Boolean) {
        val becameVisible = !sheetVisible && visible
        sheetVisible = visible
        if (becameVisible) {
            load(silent = _state.value.messages.isNotEmpty())
        }
    }

    fun load(silent: Boolean = false) {
        loadJob?.cancel()
        loadJob = viewModelScope.launch {
            if (!silent && _state.value.messages.isEmpty()) {
                _state.value = _state.value.copy(loading = true, error = null)
            }
            try {
                val thread = container.rides.rideMessages(rideId)
                _state.value = _state.value.copy(
                    loading = false,
                    open = thread.open,
                    canPost = thread.canPost,
                    members = thread.members,
                    reactionsAllowed = thread.reactionsAllowed.ifEmpty { _state.value.reactionsAllowed },
                    messages = thread.messages,
                    error = null,
                )
                val maxId = thread.messages.lastOrNull()?.id ?: 0L
                if (sheetVisible && maxId > 0L) {
                    runCatching { container.rides.markChatRead(rideId, maxId) }
                }
            } catch (e: ApiException) {
                _state.value = _state.value.copy(loading = false, error = e.message)
            }
        }
    }

    fun send(body: String, onSent: () -> Unit = {}) {
        val clean = body.trim()
        if (clean.isEmpty() || _state.value.sending) return
        _state.value = _state.value.copy(sending = true, error = null)
        viewModelScope.launch {
            try {
                container.rides.postMessage(rideId, clean.take(1000))
                load(silent = true)
                onSent()
            } catch (e: ApiException) {
                _state.value = _state.value.copy(error = e.message)
            } finally {
                _state.value = _state.value.copy(sending = false)
            }
        }
    }

    fun react(messageId: Long, emoji: String) {
        viewModelScope.launch {
            try {
                container.rides.reactMessage(rideId, messageId, emoji)
                load(silent = true)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(error = e.message)
            }
        }
    }

    companion object {
        fun factory(container: AppContainer, rideId: Long): ViewModelProvider.Factory =
            object : ViewModelProvider.Factory {
                @Suppress("UNCHECKED_CAST")
                override fun <T : ViewModel> create(modelClass: Class<T>): T =
                    TripChatViewModel(container, rideId) as T
            }
    }
}
