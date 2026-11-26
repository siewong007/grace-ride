package org.pjstatecm.graceride.ui.screens.costs

import android.content.Context
import android.net.Uri
import androidx.annotation.StringRes
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Share
import org.pjstatecm.graceride.data.media.prepareImageUpload
import org.pjstatecm.graceride.di.AppContainer

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
    @StringRes val sheetErrorRes: Int? = null,
    val doneMessage: String? = null,
)

class CostsViewModel(private val container: AppContainer) : ViewModel() {
    private val _state = MutableStateFlow(CostsUiState())
    val state: StateFlow<CostsUiState> = _state.asStateFlow()
    private var loadJob: Job? = null
    private var uploadedForUri: Uri? = null
    private var uploadedFileId: Long? = null

    init { load(first = true) }

    fun refresh() = load(first = false)

    private fun load(first: Boolean) {
        loadJob?.cancel()
        if (container.auth.me?.can("costs", "view") != true) {
            _state.value = _state.value.copy(loading = false, refreshing = false)
            return
        }
        loadJob = viewModelScope.launch {
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
        uploadedForUri = null
        uploadedFileId = null
        _state.value = _state.value.copy(
            paying = share,
            proofUri = null,
            txnRef = share.txnRef,
            sheetError = null,
            sheetErrorRes = null,
        )
    }

    fun closePay() {
        uploadedForUri = null
        uploadedFileId = null
        _state.value = _state.value.copy(paying = null, sheetError = null, sheetErrorRes = null)
    }

    fun setProof(uri: Uri?) {
        if (uri != uploadedForUri) {
            uploadedForUri = null
            uploadedFileId = null
        }
        _state.value = _state.value.copy(proofUri = uri, sheetError = null, sheetErrorRes = null)
    }

    fun setTxnRef(value: String) {
        _state.value = _state.value.copy(txnRef = value, sheetError = null, sheetErrorRes = null)
    }

    fun clearDone() { _state.value = _state.value.copy(doneMessage = null) }

    /**
     * The app never moves money: the member pays in their banking app via the
     * payee's DuitNow QR, then uploads the transfer proof for staff verification.
     */
    fun submitProof(context: Context) {
        val s = _state.value
        val share = s.paying ?: return
        if (container.auth.me?.can("costs", "update") != true) {
            _state.value = s.copy(sheetError = null, sheetErrorRes = R.string.role_costs_access_required)
            return
        }
        val uri = s.proofUri
        val validationErr = validateProofInput(uri != null, s.txnRef)
        if (validationErr != null) {
            _state.value = s.copy(sheetError = null, sheetErrorRes = validationErr)
            return
        }
        uri!!
        val cleanRef = s.txnRef.trim()
        if (s.submitting) return
        _state.value = s.copy(submitting = true, sheetError = null, sheetErrorRes = null)
        viewModelScope.launch {
            try {
                val fileId = if (uploadedForUri == uri && uploadedFileId != null) {
                    uploadedFileId!!
                } else {
                    val prepared = withContext(Dispatchers.IO) {
                        try {
                            prepareImageUpload(
                                context = context,
                                uri = uri,
                                prefix = "proof",
                                maxDimensionPx = 1600,
                                quality = 85,
                            )
                        } catch (e: org.pjstatecm.graceride.data.media.FileTooLargeException) {
                            throw IllegalStateException(context.getString(R.string.costs_err_file_too_large), e)
                        } catch (e: Exception) {
                            throw IllegalStateException(context.getString(R.string.costs_err_unreadable), e)
                        }
                    }
                    val uploaded = container.shares.upload(prepared.bytes, prepared.filename, prepared.mime)
                    uploadedForUri = uri
                    uploadedFileId = uploaded.id
                    uploaded.id
                }
                container.shares.submitProof(share.id, fileId, cleanRef)
                uploadedForUri = null
                uploadedFileId = null
                _state.value = _state.value.copy(submitting = false, paying = null, doneMessage = "ok")
                load(first = false)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(submitting = false, sheetError = e.message, sheetErrorRes = null)
            } catch (e: Exception) {
                _state.value = _state.value.copy(
                    submitting = false,
                    sheetError = e.message,
                    sheetErrorRes = if (e.message == null) R.string.costs_err_upload else null,
                )
            }
        }
    }

    companion object {
        internal fun validateProofInput(hasPhoto: Boolean, txnRef: String): Int? = when {
            !hasPhoto -> R.string.costs_err_missing_photo
            txnRef.trim().isEmpty() -> R.string.costs_err_missing_ref
            txnRef.trim().length < 4 -> R.string.costs_err_ref_short
            else -> null
        }

        fun factory(container: AppContainer) = viewModelFactory { initializer { CostsViewModel(container) } }
    }
}
