package org.pjstatecm.graceride.ui.screens.help

import android.content.Context
import android.net.Uri
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
import org.pjstatecm.graceride.data.api.TicketDetailDto
import org.pjstatecm.graceride.data.media.prepareImageUpload
import org.pjstatecm.graceride.di.AppContainer

data class HelpDetailUiState(
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val ticket: TicketDetailDto? = null,
    val replyBody: String = "",
    val attachmentFileId: Long? = null,
    val uploading: Boolean = false,
    val busy: Boolean = false,
    val error: String? = null,
    val message: String? = null,
)

class HelpDetailViewModel(
    private val container: AppContainer,
    private val id: Long,
) : ViewModel() {
    private val _state = MutableStateFlow(HelpDetailUiState())
    val state: StateFlow<HelpDetailUiState> = _state.asStateFlow()

    init {
        load(first = true)
    }

    fun refresh() = load(first = false)

    fun load(first: Boolean = true) {
        viewModelScope.launch {
            _state.value = _state.value.copy(
                loading = first && _state.value.ticket == null,
                refreshing = !first,
                error = null,
            )
            try {
                val t = container.help.ticketDetail(id)
                _state.value = _state.value.copy(loading = false, refreshing = false, ticket = t)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(loading = false, refreshing = false, error = e.message)
            }
        }
    }

    fun setReplyBody(v: String) {
        _state.value = _state.value.copy(replyBody = v)
    }

    fun uploadAttachment(context: Context, uri: Uri) {
        _state.value = _state.value.copy(uploading = true, message = null)
        viewModelScope.launch {
            try {
                val prepared = withContext(Dispatchers.IO) {
                    prepareImageUpload(context, uri, prefix = "help-reply", maxDimensionPx = 1600, quality = 85)
                }
                val fileRef = container.help.uploadFile(prepared.bytes, prepared.filename, prepared.mime)
                _state.value = _state.value.copy(uploading = false, attachmentFileId = fileRef.id)
            } catch (e: Exception) {
                _state.value = _state.value.copy(uploading = false, message = e.message)
            }
        }
    }

    fun clearAttachment() {
        _state.value = _state.value.copy(attachmentFileId = null)
    }

    fun sendReply() {
        val s = _state.value
        if (s.busy || (s.replyBody.isBlank() && s.attachmentFileId == null)) return
        _state.value = s.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                container.help.reply(id, s.replyBody, s.attachmentFileId)
                val updated = container.help.ticketDetail(id)
                _state.value = _state.value.copy(
                    busy = false,
                    replyBody = "",
                    attachmentFileId = null,
                    ticket = updated,
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, message = e.message)
            }
        }
    }

    fun close() = act { container.help.close(id) }

    fun reopen() = act { container.help.reopen(id) }

    fun claim(assignToMe: Boolean) = act { container.help.claim(id, assignToMe) }

    fun clearMessage() {
        _state.value = _state.value.copy(message = null)
    }

    private fun act(block: suspend () -> Unit) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                block()
                val updated = container.help.ticketDetail(id)
                _state.value = _state.value.copy(busy = false, ticket = updated)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, message = e.message)
            }
        }
    }

    companion object {
        fun factory(container: AppContainer, id: Long) = viewModelFactory {
            initializer { HelpDetailViewModel(container, id) }
        }
    }
}
