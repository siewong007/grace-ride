package org.pjstatecm.graceride.data.api

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonObject

// ---------------- Settlements & Shares Management ----------------

@Serializable
data class SettlementEventRefDto(
    val id: Long,
    val title: String = "",
    @SerialName("starts_at") val startsAt: String = "",
)

@Serializable
data class SettlementItemDto(
    val id: Long = 0,
    val label: String = "",
    @SerialName("amount_cents") val amountCents: Long = 0,
    @SerialName("receipt_file_id") val receiptFileId: Long? = null,
)

@Serializable
data class SettlementShareDto(
    val id: Long,
    @SerialName("user_id") val userId: Long,
    val name: String = "",
    @SerialName("amount_cents") val amountCents: Long = 0,
    val status: String = "unpaid",
    @SerialName("proof_file_id") val proofFileId: Long? = null,
    @SerialName("txn_ref") val txnRef: String = "",
    @SerialName("paid_date") val paidDate: String? = null,
    val note: String = "",
    @SerialName("updated_at") val updatedAt: String = "",
)

@Serializable
data class SettlementSummaryDto(
    val id: Long,
    val title: String = "",
    @SerialName("event_title") val eventTitle: String = "",
    val status: String = "draft",
    @SerialName("total_cents") val totalCents: Long = 0,
    val confirmed: Long = 0,
    val count: Long = 0,
)

@Serializable
data class SettlementsPageDto(
    val items: List<SettlementSummaryDto> = emptyList(),
    val total: Long = 0,
    val page: Int = 1,
    @SerialName("page_size") val pageSize: Int = 25,
    @SerialName("total_pages") val totalPages: Int = 1,
)

@Serializable
data class SettlementDetailDto(
    val id: Long,
    val event: SettlementEventRefDto,
    @SerialName("created_by") val createdBy: UserRefDto,
    val title: String = "",
    @SerialName("split_mode") val splitMode: String = "equal",
    @SerialName("subsidy_cents") val subsidyCents: Long = 0,
    @SerialName("payee_name") val payeeName: String = "",
    @SerialName("payee_account") val payeeAccount: String = "",
    @SerialName("payee_qr_file_id") val payeeQrFileId: Long? = null,
    @SerialName("due_date") val dueDate: String? = null,
    val status: String = "draft",
    val version: Int = 1,
    @SerialName("created_at") val createdAt: String = "",
    @SerialName("gross_cents") val grossCents: Long = 0,
    @SerialName("total_cents") val totalCents: Long = 0,
    val items: List<SettlementItemDto> = emptyList(),
    val shares: List<SettlementShareDto> = emptyList(),
    @SerialName("can_manage") val canManage: Boolean = false,
)

@Serializable
data class SettlementItemInReq(
    val label: String,
    val amount_cents: Long,
    val receipt_file_id: Long? = null,
)

@Serializable
data class SettlementParticipantInReq(
    val user_id: Long,
    val amount_cents: Long? = null,
)

@Serializable
data class SettlementInReq(
    val title: String,
    val items: List<SettlementItemInReq>,
    val subsidy_cents: Long? = null,
    val split_mode: String = "equal",
    val participants: List<SettlementParticipantInReq>,
    val payee_name: String,
    val payee_account: String,
    val payee_qr_file_id: Long? = null,
    val due_date: String? = null,
    val reason: String? = null,
)

@Serializable
data class SettlementRevisionDto(
    val version: Int = 1,
    val snapshot: JsonObject? = null,
    val reason: String = "",
    val by: String = "",
    @SerialName("created_at") val createdAt: String = "",
)

@Serializable
data class VerifyShareReq(
    val action: String,
    val note: String? = null,
)

// ---------------- Reimbursements & Payout Batches ----------------

@Serializable
data class ReimbursementDto(
    val id: Long,
    val user: UserRefDto,
    @SerialName("ride_id") val rideId: Long? = null,
    @SerialName("ride_at") val rideAt: String? = null,
    val kind: String = "grab",
    @SerialName("ride_route") val rideRoute: String? = null,
    @SerialName("event_id") val eventId: Long? = null,
    @SerialName("event_title") val eventTitle: String? = null,
    @SerialName("event_starts_at") val eventStartsAt: String? = null,
    @SerialName("amount_cents") val amountCents: Long = 0,
    val description: String = "",
    @SerialName("receipt_file_id") val receiptFileId: Long? = null,
    @SerialName("payee_name") val payeeName: String? = null,
    @SerialName("payee_phone") val payeePhone: String? = null,
    val status: String = "pending",
    @SerialName("review_note") val reviewNote: String = "",
    val reviewer: String? = null,
    @SerialName("payee_verified") val payeeVerified: Boolean = false,
    @SerialName("paid_ref") val paidRef: String? = null,
    @SerialName("paid_at") val paidAt: String? = null,
    @SerialName("created_at") val createdAt: String = "",
    val visitor: UserRefDto? = null,
    @SerialName("special_reason") val specialReason: String? = null,
    @SerialName("high_amount") val highAmount: Boolean = false,
    @SerialName("requested_by") val requestedBy: UserRefDto? = null,
)

@Serializable
data class ReimbursementsPageDto(
    val items: List<ReimbursementDto> = emptyList(),
    val total: Long = 0,
    val page: Int = 1,
    @SerialName("page_size") val pageSize: Int = 25,
    @SerialName("total_pages") val totalPages: Int = 1,
)

@Serializable
data class CreateReimbursementReq(
    val ride_id: Long? = null,
    val event_id: Long? = null,
    val amount_cents: Long,
    val description: String,
    val receipt_file_id: Long? = null,
    val payee_name: String,
    val payee_phone: String,
    val kind: String? = "grab",
)

@Serializable
data class ReviewReimbursementReq(
    val action: String,
    val note: String? = null,
)

@Serializable
data class PayReimbursementReq(
    val payee_verified: Boolean,
    val paid_ref: String,
)

@Serializable
data class PayoutBatchDto(
    val id: Long,
    val month: String = "",
    val title: String = "",
    @SerialName("total_cents") val totalCents: Long = 0,
    @SerialName("claim_count") val claimCount: Long = 0,
    val status: String = "open",
    @SerialName("paid_ref") val paidRef: String = "",
    @SerialName("created_by_id") val createdById: Long = 0,
    @SerialName("created_by") val createdBy: String = "",
    @SerialName("paid_by") val paidBy: String? = null,
    @SerialName("paid_at") val paidAt: String? = null,
    @SerialName("created_at") val createdAt: String = "",
)

@Serializable
data class CreatePayoutBatchReq(
    val month: String? = null,
    val title: String? = null,
    val claim_ids: List<Long>? = null,
)

@Serializable
data class CreatePayoutBatchResp(
    val id: Long,
    val month: String = "",
    val title: String = "",
    @SerialName("total_cents") val totalCents: Long = 0,
    @SerialName("claim_count") val claimCount: Long = 0,
)
