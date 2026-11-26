package org.pjstatecm.graceride.data.api

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class EventUpsertReq(
    val title: String,
    val kind: String,
    val icon: String? = null,
    val description: String = "",
    val starts_at: String,
    val location: String,
    val capacity: Long = 0,
    val cost_note: String = "",
    val signup_deadline: String? = null,
    val group_ids: List<Long>,
    val as_group_ids: List<Long> = emptyList(),
    val photo_file_id: Long? = null,
    val attendance_enabled: Boolean = true,
    val mileage_claimable: Boolean = false,
    val area_id: Long? = null,
    val repeat_frequency: String? = null,
    val repeat_count: Long? = null,
    val status: String? = null,
)

@Serializable
data class EventCreateRespDto(
    val id: Long,
    @SerialName("created_ids") val createdIds: List<Long> = emptyList(),
    @SerialName("series_id") val seriesId: String? = null,
    val status: String = "published",
)

@Serializable
data class EventDuplicateReq(
    val starts_at: String,
    val signup_deadline: String? = null,
    val title: String? = null,
    val status: String? = null,
)

@Serializable
data class EventSignupRespDto(
    val ok: Boolean = true,
    val status: String = "going",
)

@Serializable
data class EventCheckinResetRespDto(
    @SerialName("checkin_code") val checkinCode: String = "",
)

@Serializable
data class EventSignupItemDto(
    val id: Long,
    @SerialName("user_id") val userId: Long,
    val name: String = "",
    val status: String = "going",
    @SerialName("need_ride") val needRide: Boolean = false,
    @SerialName("offer_drive") val offerDrive: Boolean = false,
    @SerialName("offer_seats") val offerSeats: Long = 0,
    @SerialName("self_transport") val selfTransport: Boolean = false,
    val attended: Long? = null,
    @SerialName("ride_status") val rideStatus: String? = null,
    @SerialName("as_group") val asGroup: String? = null,
)

// ---- Event Visitor Passes ----

@Serializable
data class VisitorPassHostDto(
    val id: Long = 0,
    val name: String = "",
)

@Serializable
data class VisitorPassDto(
    val id: Long,
    val name: String = "",
    val phone: String = "",
    @SerialName("ride_id") val rideId: Long? = null,
    @SerialName("ride_status") val rideStatus: String? = null,
    val host: VisitorPassHostDto = VisitorPassHostDto(),
    val state: String = "open",
    @SerialName("closed_reason") val closedReason: String? = null,
    @SerialName("valid_from") val validFrom: String = "",
    @SerialName("valid_until") val validUntil: String = "",
)

@Serializable
data class CreateVisitorPassReq(
    val name: String,
    val phone: String = "",
    val pickup_at: String,
    val pickup_area_id: Long,
    val pickup_address: String,
    val seats: Int = 1,
    val assistance: String? = null,
    val notes: String? = null,
)

@Serializable
data class CreateVisitorPassRespDto(
    val id: Long = 0,
    val url: String = "",
    @SerialName("ride_id") val rideId: Long = 0,
    @SerialName("valid_from") val validFrom: String = "",
    @SerialName("valid_until") val validUntil: String = "",
)

@Serializable
data class VisitorUrlRespDto(val url: String = "")

// ---- Post-event Driver Mileage ----
// Shared PostEvent* request/driver DTOs live in TrafficDtos.kt.

@Serializable
data class PostEventReimbBatchRespDto(
    val ok: Boolean = true,
    @SerialName("created_count") val createdCount: Int = 0,
    @SerialName("auto_approved_count") val autoApprovedCount: Int = 0,
    @SerialName("pending_review_count") val pendingReviewCount: Int = 0,
    @SerialName("total_cents") val totalCents: Long = 0,
)
