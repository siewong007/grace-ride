package org.pjstatecm.graceride.data.repo

import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.RequestBody.Companion.toRequestBody
import org.pjstatecm.graceride.data.api.ApiProvider
import org.pjstatecm.graceride.data.api.ApiService
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.CreatePayoutBatchReq
import org.pjstatecm.graceride.data.api.CreatePayoutBatchResp
import org.pjstatecm.graceride.data.api.CreateReimbursementReq
import org.pjstatecm.graceride.data.api.FileRef
import org.pjstatecm.graceride.data.api.IdRespDto
import org.pjstatecm.graceride.data.api.PayReimbursementReq
import org.pjstatecm.graceride.data.api.PayoutBatchDto
import org.pjstatecm.graceride.data.api.ReimbursementDto
import org.pjstatecm.graceride.data.api.ReimbursementsPageDto
import org.pjstatecm.graceride.data.api.ReviewReimbursementReq
import org.pjstatecm.graceride.data.api.SettlementDetailDto
import org.pjstatecm.graceride.data.api.SettlementInReq
import org.pjstatecm.graceride.data.api.SettlementRevisionDto
import org.pjstatecm.graceride.data.api.SettlementsPageDto
import org.pjstatecm.graceride.data.api.VerifyShareReq
import org.pjstatecm.graceride.data.api.apiCall

class FinanceRepository(private val apiProvider: ApiProvider, private val bus: AuthEventBus) {
    private val api: ApiService get() = apiProvider.get()

    suspend fun uploadFile(bytes: ByteArray, filename: String, mime: String): FileRef {
        val part = MultipartBody.Part.createFormData("file", filename, bytes.toRequestBody(mime.toMediaType()))
        return apiCall(bus) { api.upload(part) }
    }

    // ---- Settlements ----
    suspend fun listSettlements(page: Int = 1, limit: Int = 25): SettlementsPageDto =
        apiCall(bus) { api.settlements(page, limit) }

    suspend fun createSettlement(eventId: Long, req: SettlementInReq): IdRespDto =
        apiCall(bus) { api.createSettlement(eventId, req) }

    suspend fun settlementDetail(id: Long): SettlementDetailDto =
        apiCall(bus) { api.settlementDetail(id) }

    suspend fun updateSettlement(id: Long, req: SettlementInReq) =
        apiCall(bus) { api.updateSettlement(id, req) }

    suspend fun deleteSettlement(id: Long) =
        apiCall(bus) { api.deleteSettlement(id) }

    suspend fun publishSettlement(id: Long) =
        apiCall(bus) { api.publishSettlement(id) }

    suspend fun closeSettlement(id: Long) =
        apiCall(bus) { api.closeSettlement(id) }

    suspend fun settlementRevisions(id: Long): List<SettlementRevisionDto> =
        apiCall(bus) { api.settlementRevisions(id) }

    suspend fun verifyShare(shareId: Long, action: String, note: String? = null) =
        apiCall(bus) { api.verifyShare(shareId, VerifyShareReq(action = action, note = note?.trim()?.ifBlank { null })) }

    // ---- Reimbursements ----
    suspend fun listReimbursements(
        scope: String = "mine",
        q: String? = null,
        status: String? = null,
        sort: String? = null,
        dir: String? = null,
        page: Int = 1,
        limit: Int = 25,
    ): ReimbursementsPageDto = apiCall(bus) {
        api.reimbursements(
            scope = scope,
            q = q?.trim()?.ifBlank { null },
            status = status?.takeIf { it.isNotBlank() && it != "all" },
            sort = sort,
            dir = dir,
            page = page,
            limit = limit,
        )
    }

    suspend fun reimbursementDetail(id: Long): ReimbursementDto =
        apiCall(bus) { api.reimbursementDetail(id) }

    suspend fun createReimbursement(req: CreateReimbursementReq): IdRespDto =
        apiCall(bus) { api.createReimbursement(req) }

    suspend fun updateReimbursement(id: Long, req: CreateReimbursementReq) =
        apiCall(bus) { api.updateReimbursement(id, req) }

    suspend fun reviewReimbursement(id: Long, action: String, note: String? = null) =
        apiCall(bus) { api.reviewReimbursement(id, ReviewReimbursementReq(action = action, note = note?.trim()?.ifBlank { null })) }

    suspend fun payReimbursement(id: Long, payeeVerified: Boolean, paidRef: String) =
        apiCall(bus) { api.payReimbursement(id, PayReimbursementReq(payee_verified = payeeVerified, paid_ref = paidRef.trim())) }

    // ---- Payout Batches ----
    suspend fun listPayoutBatches(): List<PayoutBatchDto> =
        apiCall(bus) { api.payoutBatches() }

    suspend fun createPayoutBatch(req: CreatePayoutBatchReq): CreatePayoutBatchResp =
        apiCall(bus) { api.createPayoutBatch(req) }

    suspend fun cancelPayoutBatch(id: Long) =
        apiCall(bus) { api.cancelPayoutBatch(id) }

    suspend fun downloadBatchCsv(id: Long): ByteArray =
        apiCall(bus) { api.payoutBatchCsv(id).use { it.bytes() } }

    suspend fun payPayoutBatch(id: Long, payeeVerified: Boolean, paidRef: String) =
        apiCall(bus) { api.payPayoutBatch(id, PayReimbursementReq(payee_verified = payeeVerified, paid_ref = paidRef.trim())) }
}
