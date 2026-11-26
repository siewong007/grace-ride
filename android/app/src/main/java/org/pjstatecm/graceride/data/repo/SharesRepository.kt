package org.pjstatecm.graceride.data.repo

import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.RequestBody.Companion.asRequestBody
import okhttp3.RequestBody.Companion.toRequestBody
import org.pjstatecm.graceride.data.api.ApiProvider
import org.pjstatecm.graceride.data.api.ApiService
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.FileRef
import org.pjstatecm.graceride.data.api.ProofReq
import org.pjstatecm.graceride.data.api.Share
import org.pjstatecm.graceride.data.api.apiCall
import java.io.File

class SharesRepository(private val apiProvider: ApiProvider, private val bus: AuthEventBus) {
    private val api: ApiService get() = apiProvider.get()
    suspend fun mine(): List<Share> = apiCall(bus) { api.myShares() }

    suspend fun upload(file: File, mime: String): FileRef {
        val part = MultipartBody.Part.createFormData(
            "file", file.name, file.asRequestBody(mime.toMediaType())
        )
        return apiCall(bus) { api.upload(part) }
    }

    suspend fun upload(bytes: ByteArray, filename: String, mime: String): FileRef {
        val part = MultipartBody.Part.createFormData(
            "file", filename, bytes.toRequestBody(mime.toMediaType())
        )
        return apiCall(bus) { api.upload(part) }
    }

    suspend fun submitProof(
        shareId: Long,
        fileId: Long,
        txnRef: String,
        paidDate: String = org.pjstatecm.graceride.domain.KlTime.today(),
    ) = apiCall(bus) { api.submitProof(shareId, ProofReq(fileId, txnRef, paid_date = paidDate)) }
}
