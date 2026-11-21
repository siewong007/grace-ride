package org.pjstatecm.graceride.ui.screens.costs

import android.content.Context
import android.net.Uri
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Share
import org.pjstatecm.graceride.di.AppContainer
import java.io.File

data class CostsUiState(
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val offline: Boolean = false,
    val error: String? = null,
    val shares: List<Share> = emptyList(),
    val paying: Share? = null,
    val proofUri: Uri? = null,
    val txnRef: String = "",
    val submitting: Boolean = false,
    val sheetError: String? = null,
    val doneMessage: String? = null,
)

class CostsViewModel(private val container: AppContainer) : ViewModel() {
    private val _state = MutableStateFlow(CostsUiState())
    val state: StateFlow<CostsUiState> = _state.asStateFlow()

    init { load(first = true) }

    fun refresh() = load(first = false)

    private fun load(first: Boolean) {
        viewModelScope.launch {
            _state.value = _state.value.copy(loading = first, refreshing = !first)
            try {
                val shares = container.shares.mine()
                _state.value = _state.value.copy(loading = false, refreshing = false, shares = shares, error = null, offline = false)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(
                    loading = false, refreshing = false,
                    error = if (e.status == 0) null else e.message,
                    offline = e.status == 0,
                )
            }
        }
    }

    fun openPay(share: Share) {
        _state.value = _state.value.copy(paying = share, proofUri = null, txnRef = share.txnRef, sheetError = null)
    }

    fun closePay() { _state.value = _state.value.copy(paying = null, sheetError = null) }

    fun setProof(uri: Uri?) { _state.value = _state.value.copy(proofUri = uri) }
    fun setTxnRef(value: String) { _state.value = _state.value.copy(txnRef = value) }
    fun clearDone() { _state.value = _state.value.copy(doneMessage = null) }

    /**
     * The app never moves money: the member pays in their banking app via the
     * payee's DuitNow QR, then uploads the transfer proof for staff verification.
     */
    fun submitProof(context: Context) {
        val s = _state.value
        val share = s.paying ?: return
        val uri = s.proofUri
        if (uri == null) {
            _state.value = s.copy(sheetError = "Please attach the transfer screenshot.")
            return
        }
        if (s.txnRef.isBlank()) {
            _state.value = s.copy(sheetError = "Please enter the transaction reference.")
            return
        }
        if (s.submitting) return
        _state.value = s.copy(submitting = true, sheetError = null)
        viewModelScope.launch {
            try {
                val file = withContext(Dispatchers.IO) {
                    val tmp = File.createTempFile("proof-", ".jpg", context.cacheDir)
                    context.contentResolver.openInputStream(uri)?.use { input ->
                        tmp.outputStream().use { output -> input.copyTo(output) }
                    } ?: throw IllegalStateException("Cannot read the selected image")
                    tmp
                }
                val mime = context.contentResolver.getType(uri) ?: "image/jpeg"
                val uploaded = container.shares.upload(file, mime)
                file.delete()
                container.shares.submitProof(share.id, uploaded.id, s.txnRef.trim())
                _state.value = _state.value.copy(submitting = false, paying = null, doneMessage = "ok")
                load(first = false)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(submitting = false, sheetError = e.message)
            } catch (e: Exception) {
                _state.value = _state.value.copy(submitting = false, sheetError = e.message ?: "Upload failed")
            }
        }
    }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory { initializer { CostsViewModel(container) } }
    }
}
