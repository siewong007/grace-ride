package org.pjstatecm.graceride.data.repo

import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.RequestBody.Companion.asRequestBody
import org.pjstatecm.graceride.data.api.ApiService
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.FileRef
import org.pjstatecm.graceride.data.api.ProofReq
import org.pjstatecm.graceride.data.api.Share
import org.pjstatecm.graceride.data.api.apiCall
import java.io.File

class SharesRepository(private val api: ApiService, private val bus: AuthEventBus) {
    suspend fun mine(): List<Share> = apiCall(bus) { api.myShares() }

    suspend fun upload(file: File, mime: String): FileRef {
        val part = MultipartBody.Part.createFormData(
            "file", file.name, file.asRequestBody(mime.toMediaType())
        )
        return apiCall(bus) { api.upload(part) }
    }

    suspend fun submitProof(shareId: Long, fileId: Long, txnRef: String) =
        apiCall(bus) { api.submitProof(shareId, ProofReq(fileId, txnRef)) }
}
