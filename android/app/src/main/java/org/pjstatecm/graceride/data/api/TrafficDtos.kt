package org.pjstatecm.graceride.data.api

import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.descriptors.PrimitiveKind
import kotlinx.serialization.descriptors.PrimitiveSerialDescriptor
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonDecoder
import kotlinx.serialization.json.JsonEncoder
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.contentOrNull

/** Staff dashboard summary counters surfaced on Home / Traffic badges. */
@Serializable
data class AdminDashboardStatsDto(
    val today: String = "",
    @SerialName("today_total") val todayTotal: Long = 0,
    @SerialName("today_confirmed") val todayConfirmed: Long = 0,
    @SerialName("today_changes") val todayChanges: Long = 0,
    @SerialName("pending_approval") val pendingApproval: Long = 0,
    @SerialName("no_driver") val noDriver: Long = 0,
    @SerialName("needs_attention") val needsAttention: Long = 0,
    @SerialName("live_drivers") val liveDrivers: Long = 0,
    @SerialName("max_live_drivers") val maxLiveDrivers: Long = 0,
    val members: Long = 0,
    @SerialName("pending_members") val pendingMembers: Long = 0,
    @SerialName("pending_recovery") val pendingRecovery: Long = 0,
    @SerialName("open_invites") val openInvites: Long = 0,
    @SerialName("open_help") val openHelp: Long = 0,
    @SerialName("pending_reimbursements") val pendingReimbursements: Long = 0,
    @SerialName("awaiting_payment") val awaitingPayment: Long = 0,
    @SerialName("oldest_approval_at") val oldestApprovalAt: String? = null,
    @SerialName("next_no_driver_at") val nextNoDriverAt: String? = null,
    @SerialName("oldest_help_at") val oldestHelpAt: String? = null,
    @SerialName("help_claimed_by_me") val helpClaimedByMe: Long = 0,
    @SerialName("pending_plan_drafts") val pendingPlanDrafts: Long = 0,
    @SerialName("pending_ride_drafts") val pendingRideDrafts: Long = 0,
    @SerialName("my_returned_plan_drafts") val myReturnedPlanDrafts: Long = 0,
    @SerialName("my_returned_ride_drafts") val myReturnedRideDrafts: Long = 0,
)

@Serializable
data class UserRefDto(
    val id: Long,
    val name: String = "",
)

@Serializable
data class AdminRidesPageDto(
    val items: List<Ride> = emptyList(),
    val total: Long = 0,
    val page: Int = 1,
    @SerialName("page_size") val pageSize: Int = 50,
    @SerialName("total_pages") val totalPages: Int = 1,
)

// ---------------- Traffic Plans & Drafts ----------------

@Serializable
data class PlanDraftEntryDto(
    @SerialName("ride_id") val rideId: Long,
    @SerialName("driver_id") val driverId: Long? = null,
)

@Serializable
data class DraftVisitorEntryDto(
    @SerialName("temp_id") val tempId: String = "",
    val op: String = "add", // "add" | "update" | "delete"
    @SerialName("live_visitor_id") val liveVisitorId: Long? = null,
    val name: String = "",
    val phone: String = "",
    val notes: String = "",
    @SerialName("area_id") val areaId: Long? = null,
    @SerialName("area_name") val areaName: String? = null,
    @SerialName("pickup_address") val pickupAddress: String = "",
    val seats: Long? = 1,
    val leg: String? = "outbound",
    @SerialName("driver_id") val driverId: Long? = null,
    @SerialName("driver_name") val driverName: String? = null,
)

@Serializable
data class PlanDraftDto(
    val id: Long,
    @SerialName("event_id") val eventId: Long,
    val version: Int = 1,
    val label: String = "",
    val status: String = "draft", // "draft" | "pending_approval" | "returned" | "approved" | "superseded"
    val assignments: List<PlanDraftEntryDto> = emptyList(),
    val visitors: List<DraftVisitorEntryDto> = emptyList(),
    @SerialName("created_by") val createdBy: UserRefDto? = null,
    @SerialName("updated_by") val updatedBy: UserRefDto? = null,
    @SerialName("created_at") val createdAt: String = "",
    @SerialName("updated_at") val updatedAt: String = "",
    @SerialName("submitted_by") val submittedBy: UserRefDto? = null,
    @SerialName("submitted_at") val submittedAt: String? = null,
    @SerialName("reviewed_by") val reviewedBy: UserRefDto? = null,
    @SerialName("reviewed_at") val reviewedAt: String? = null,
    @SerialName("review_notes") val reviewNotes: String? = null,
    @SerialName("locked_by") val lockedBy: UserRefDto? = null,
    @SerialName("locked_at") val lockedAt: String? = null,
    @SerialName("lock_expires_at") val lockExpiresAt: String? = null,
    @SerialName("lock_is_mine") val lockIsMine: Boolean = false,
)

@Serializable
data class TrafficEventPlanItemDto(
    val id: Long,
    val title: String,
    val kind: String = "",
    val icon: String? = null,
    @SerialName("starts_at") val startsAt: String,
    val location: String = "",
    val status: String = "published",
    @SerialName("area_name") val areaName: String? = null,
    val groups: String = "",
    @SerialName("mileage_claimable") val mileageClaimable: Boolean = false,
    @SerialName("ride_count") val rideCount: Long = 0,
    @SerialName("unassigned_count") val unassignedCount: Long = 0,
    @SerialName("pending_confirm_count") val pendingConfirmCount: Long = 0,
    @SerialName("visitor_count") val visitorCount: Long = 0,
    @SerialName("draft_count") val draftCount: Long = 0,
    @SerialName("pending_draft_count") val pendingDraftCount: Long = 0,
    @SerialName("returned_draft_count") val returnedDraftCount: Long = 0,
    @SerialName("latest_draft") val latestDraft: PlanDraftDto? = null,
)

@Serializable
data class PlanEventDto(
    val id: Long,
    val title: String,
    @SerialName("starts_at") val startsAt: String = "",
    val location: String = "",
    val status: String = "published",
)

@Serializable
data class PlanLastMatchDto(
    @SerialName("driver_id") val driverId: Long,
    @SerialName("driver_name") val driverName: String,
    val times: Long = 1,
    val at: String = "",
)

@Serializable
data class PlanRideDto(
    val id: Long,
    val passenger: UserRefDto,
    val seats: Long = 1,
    val area: RideArea,
    @SerialName("pickup_at") val pickupAt: String = "",
    @SerialName("pickup_address") val pickupAddress: String = "",
    val assistance: String = "",
    val notes: String = "",
    val status: String = "pending_driver",
    @SerialName("driver_id") val driverId: Long? = null,
    val leg: String = "outbound",
    @SerialName("paired_ride_id") val pairedRideId: Long? = null,
    @SerialName("match_group_id") val matchGroupId: Long? = null,
    @SerialName("reimbursement_excluded") val reimbursementExcluded: Boolean = false,
    val last: PlanLastMatchDto? = null,
)

@Serializable
data class PlanVehicleDto(
    val model: String? = null,
    val plate: String? = null,
    val seats: Long? = null,
    @SerialName("wheelchair_ok") val wheelchairOk: Boolean = false,
    @SerialName("low_step") val lowStep: Boolean = false,
    @SerialName("child_seat") val childSeat: Boolean = false,
)

@Serializable
data class PlanDriverDto(
    val id: Long,
    val name: String,
    val carpool: Boolean = true,
    val cap: Long = 4,
    @SerialName("other_seats") val otherSeats: Long = 0,
    @SerialName("other_rides") val otherRides: Long = 0,
    @SerialName("other_outbound_seats") val otherOutboundSeats: Long = 0,
    @SerialName("other_outbound_rides") val otherOutboundRides: Long = 0,
    @SerialName("other_return_seats") val otherReturnSeats: Long = 0,
    @SerialName("other_return_rides") val otherReturnRides: Long = 0,
    @SerialName("area_ids") val areaIds: List<Long> = emptyList(),
    @SerialName("drive_group_ids") val driveGroupIds: List<Long> = emptyList(),
    val vehicle: PlanVehicleDto? = null,
)

@Serializable
data class SpecialReimbursementDto(
    val id: Long,
    val status: String = "pending",
    @SerialName("amount_cents") val amountCents: Long? = null,
    @SerialName("high_amount") val highAmount: Boolean = false,
    val reason: String? = null,
    @SerialName("review_note") val reviewNote: String? = null,
    @SerialName("payee_name") val payeeName: String? = null,
)

@Serializable
data class PlanVisitorDto(
    val id: Long,
    val name: String,
    val phone: String = "",
    val notes: String = "",
    val area: RideArea? = null,
    @SerialName("pickup_address") val pickupAddress: String = "",
    val seats: Long = 1,
    val leg: String = "outbound",
    @SerialName("driver_id") val driverId: Long? = null,
    @SerialName("driver_name") val driverName: String? = null,
    @SerialName("created_by") val createdBy: UserRefDto? = null,
    @SerialName("created_at") val createdAt: String = "",
    @SerialName("updated_at") val updatedAt: String = "",
    @SerialName("can_edit") val canEdit: Boolean = false,
    @SerialName("special_reimbursement") val specialReimbursement: SpecialReimbursementDto? = null,
)

@Serializable
data class PlanDataDto(
    val event: PlanEventDto,
    val draft: PlanDraftDto? = null,
    val drafts: List<PlanDraftDto> = emptyList(),
    @SerialName("can_approve") val canApprove: Boolean = false,
    val visitors: List<PlanVisitorDto> = emptyList(),
    val rides: List<PlanRideDto> = emptyList(),
    val drivers: List<PlanDriverDto> = emptyList(),
)

@Serializable
data class PlanDraftMutationResp(
    val ok: Boolean = true,
    val status: String? = null,
    @SerialName("draft_id") val draftId: Long? = null,
    val draft: PlanDraftDto? = null,
)

@Serializable
data class PlanChangeDto(
    @SerialName("ride_id") val rideId: Long,
    @SerialName("from_driver_id") val fromDriverId: Long? = null,
    @SerialName("driver_id") val driverId: Long? = null,
)

@Serializable
data class SavePlanReq(
    val changes: List<PlanChangeDto>,
    val draft_id: Long? = null,
)

@Serializable
data class CreatePlanDraftReq(
    val label: String? = null,
    val clone_from_draft_id: Long? = null,
    val assignments: List<PlanDraftEntryDto>? = null,
    val visitors: List<DraftVisitorEntryDto>? = null,
)

@Serializable
data class UpdatePlanDraftReq(
    val label: String? = null,
    val assignments: List<PlanDraftEntryDto>? = null,
    val visitors: List<DraftVisitorEntryDto>? = null,
)

@Serializable
data class SavePlanDraftReq(
    val draft_id: Long? = null,
    val label: String? = null,
    val assignments: List<PlanDraftEntryDto>,
    val visitors: List<DraftVisitorEntryDto>? = null,
)

@Serializable
data class DraftLockReq(val force: Boolean = false)

@Serializable
data class RequestPlanApprovalReq(
    val draft_id: Long? = null,
    val assignments: List<PlanDraftEntryDto>? = null,
    val visitors: List<DraftVisitorEntryDto>? = null,
    val label: String? = null,
)

@Serializable
data class ReturnPlanDraftReq(
    val draft_id: Long? = null,
    val notes: String? = null,
)

@Serializable
data class ArrangePlanReq(val draft: List<PlanDraftEntryDto>)

@Serializable
data class ArrangeNearDto(
    @SerialName("ride_id") val rideId: Long,
    val name: String? = null,
    val km: Double? = null,
)

@Serializable
data class ArrangePlacedDto(
    @SerialName("ride_id") val rideId: Long,
    @SerialName("driver_id") val driverId: Long,
    val near: ArrangeNearDto? = null,
    @SerialName("in_area") val inArea: Boolean = false,
    val together: Boolean = false,
)

@Serializable
data class ArrangeRespDto(
    val placed: List<ArrangePlacedDto> = emptyList(),
    val unplaced: List<Long> = emptyList(),
    val approximate: Boolean = false,
)

// ---------------- Plan Visitors & Post-Event Reimbursements ----------------

@Serializable
data class VisitorReq(
    val name: String,
    val phone: String = "",
    val notes: String = "",
    val area_id: Long? = null,
    val pickup_address: String = "",
    val seats: Long = 1,
    val leg: String = "outbound",
    val driver_id: Long? = null,
)

@Serializable
data class IdRespDto(val id: Long)

@Serializable
data class SpecialReimbReq(
    val amount_cents: Long,
    val reason: String = "",
)

@Serializable
data class PostEventAreaOptionDto(
    val id: Long,
    val name: String = "",
    @SerialName("fare_cents") val fareCents: Long? = null,
)

@Serializable
data class PostEventPassengerStatsDto(
    val total: Long = 0,
    val supper: Long = 0,
    @SerialName("direct_return") val directReturn: Long = 0,
    @SerialName("direct_home") val directHome: Long = 0,
    val unspecified: Long = 0,
)

@Serializable
data class PostEventDriverDto(
    val id: Long,
    val name: String = "",
    val phone: String = "",
    val vehicle: String? = null,
    @SerialName("default_area") val defaultArea: PostEventAreaOptionDto? = null,
    @SerialName("passenger_stats") val passengerStats: PostEventPassengerStatsDto = PostEventPassengerStatsDto(),
    @SerialName("suggested_trip_type") val suggestedTripType: String = "supper",
    @SerialName("already_claimed") val alreadyClaimed: Boolean = false,
    @SerialName("claimed_amount_cents") val claimedAmountCents: Long? = null,
    @SerialName("claimed_status") val claimedStatus: String? = null,
    @SerialName("reimbursement_excluded") val reimbursementExcluded: Boolean = false,
    @SerialName("excluded_ride_count") val excludedRideCount: Long = 0,
    @SerialName("total_ride_count") val totalRideCount: Long = 0,
)

@Serializable
data class PostEventDriversDto(
    @SerialName("event_id") val eventId: Long,
    @SerialName("event_title") val eventTitle: String = "",
    @SerialName("mileage_claimable") val mileageClaimable: Boolean = false,
    @SerialName("event_area") val eventArea: PostEventAreaOptionDto? = null,
    @SerialName("available_areas") val availableAreas: List<PostEventAreaOptionDto> = emptyList(),
    val drivers: List<PostEventDriverDto> = emptyList(),
)

@Serializable
data class PostEventClaimReq(
    val driver_id: Long,
    val trip_type: String,
    val destination_area_id: Long,
    val amount_cents: Long,
    val note: String? = null,
)

@Serializable
data class PostEventReimbursementsReq(
    val supper_note: String? = null,
    val claims: List<PostEventClaimReq>,
)

// ---------------- Live Drivers & Admin Drivers ----------------

@Serializable
data class LiveDriverDto(
    @SerialName("driver_id") val driverId: Long,
    val name: String,
    val lat: Double,
    val lng: Double,
    @SerialName("updated_at") val updatedAt: String = "",
    val rides: Long = 0,
    @SerialName("first_pickup") val firstPickup: String? = null,
)

@Serializable
data class LiveDriversDto(
    val max: Long = 0,
    val drivers: List<LiveDriverDto> = emptyList(),
)

@Serializable
data class AdminDriverVehicleDto(
    val model: String = "",
    val plate: String? = null,
    val seats: Long? = null,
)

/** Decodes driver areas returned either as a comma-separated string or as a JSON string array. */
object StringOrListSerializer : KSerializer<List<String>> {
    override val descriptor: SerialDescriptor =
        PrimitiveSerialDescriptor("StringOrList", PrimitiveKind.STRING)

    override fun deserialize(decoder: Decoder): List<String> {
        val jsonDecoder = decoder as? JsonDecoder
            ?: return decoder.decodeString().split(',', '，').map { it.trim() }.filter { it.isNotEmpty() }
        return when (val el = jsonDecoder.decodeJsonElement()) {
            is JsonNull -> emptyList()
            is JsonArray -> el.mapNotNull { (it as? JsonPrimitive)?.contentOrNull?.trim()?.takeIf(String::isNotEmpty) }
            is JsonPrimitive -> (el.contentOrNull ?: "").split(',', '，').map { it.trim() }.filter { it.isNotEmpty() }
            else -> emptyList()
        }
    }

    override fun serialize(encoder: Encoder, value: List<String>) {
        val jsonEncoder = encoder as? JsonEncoder
        if (jsonEncoder != null) {
            jsonEncoder.encodeJsonElement(JsonArray(value.map(::JsonPrimitive)))
        } else {
            encoder.encodeString(value.joinToString(", "))
        }
    }
}

@Serializable
data class AdminDriverDto(
    val id: Long,
    val name: String,
    @SerialName("carpool_enabled") val carpoolEnabled: Boolean = true,
    @SerialName("spare_seats") val spareSeats: Long = 3,
    val vehicle: AdminDriverVehicleDto? = null,
    @Serializable(with = StringOrListSerializer::class) val areas: List<String> = emptyList(),
)

// ---------------- Single-Ride Change Drafts & Staff Actions ----------------

@Serializable
data class RideDraftRideRefDto(
    val id: Long,
    val status: String = "",
    @SerialName("pickup_at") val pickupAt: String = "",
    @SerialName("passenger_name") val passengerName: String = "",
    @SerialName("area_name") val areaName: String = "",
    @SerialName("current_driver_name") val currentDriverName: String? = null,
    @SerialName("event_id") val eventId: Long? = null,
    @SerialName("event_title") val eventTitle: String? = null,
)

@Serializable
data class RideChangeDraftDto(
    val id: Long,
    @SerialName("ride_id") val rideId: Long,
    val version: Int = 1,
    val action: String, // "assign" | "reassign" | "return" | "alt_transport" | "cancel" | "approve" | "reimbursement_excluded"
    @SerialName("driver_id") val driverId: Long? = null,
    @SerialName("driver_name") val driverName: String? = null,
    val reason: String = "",
    @SerialName("reimbursement_excluded") val reimbursementExcluded: Boolean? = null,
    val status: String = "draft",
    @SerialName("created_by") val createdBy: UserRefDto? = null,
    @SerialName("updated_by") val updatedBy: UserRefDto? = null,
    @SerialName("submitted_by") val submittedBy: UserRefDto? = null,
    @SerialName("submitted_at") val submittedAt: String? = null,
    @SerialName("reviewed_by") val reviewedBy: UserRefDto? = null,
    @SerialName("reviewed_at") val reviewedAt: String? = null,
    @SerialName("review_notes") val reviewNotes: String? = null,
    @SerialName("created_at") val createdAt: String = "",
    @SerialName("updated_at") val updatedAt: String = "",
    @SerialName("locked_by") val lockedBy: UserRefDto? = null,
    @SerialName("locked_at") val lockedAt: String? = null,
    @SerialName("lock_expires_at") val lockExpiresAt: String? = null,
    @SerialName("lock_is_mine") val lockIsMine: Boolean = false,
    val ride: RideDraftRideRefDto? = null,
)

@Serializable
data class RideDraftMutationResp(
    val ok: Boolean = true,
    val draft: RideChangeDraftDto? = null,
)

@Serializable
data class RideDraftInReq(
    val action: String,
    val driver_id: Long? = null,
    val reason: String = "",
    val reimbursement_excluded: Boolean? = null,
    val submit: Boolean? = null,
)

@Serializable
data class ReturnRideDraftReq(val notes: String? = null)

@Serializable
data class StaffReassignReq(val driver_id: Long)

@Serializable
data class StaffReimbExcludedReq(val excluded: Boolean)

@Serializable
data class ReimbExcludedRespDto(
    val ok: Boolean = true,
    @SerialName("reimbursement_excluded") val reimbursementExcluded: Boolean = false,
)

// ---------------- Areas, Photos & Fares ----------------

@Serializable
data class AreaPhotoDto(
    val id: Long,
    @SerialName("file_id") val fileId: Long,
    val url: String = "",
    val caption: String = "",
    @SerialName("sort_order") val sortOrder: Int = 0,
    @SerialName("can_edit") val canEdit: Boolean = false,
    @SerialName("can_delete") val canDelete: Boolean = false,
    val mine: Boolean = false,
    @SerialName("created_by") val createdBy: UserRefDto? = null,
    @SerialName("created_at") val createdAt: String = "",
)

@Serializable
data class AreaPhotosDto(
    val area: RideArea,
    val photos: List<AreaPhotoDto> = emptyList(),
    val max: Int = 20,
    @SerialName("can_add") val canAdd: Boolean = false,
    @SerialName("can_reorder") val canReorder: Boolean = false,
)

@Serializable
data class AddAreaPhotoReq(
    val file_id: Long,
    val caption: String? = null,
)

@Serializable
data class UpdateAreaPhotoReq(val caption: String? = null)

@Serializable
data class ReorderAreaPhotosReq(val photo_ids: List<Long>)

@Serializable
data class AreaFareDto(
    val id: Long,
    @SerialName("area_a") val areaA: RideArea,
    @SerialName("area_b") val areaB: RideArea,
    @SerialName("amount_cents") val amountCents: Long = 0,
    @SerialName("updated_at") val updatedAt: String = "",
)

@Serializable
data class AreaFaresRespDto(val fares: List<AreaFareDto> = emptyList())

@Serializable
data class PutFareReq(
    val area_a_id: Long,
    val area_b_id: Long,
    val amount_cents: Long,
)

@Serializable
data class AreaUpsertReq(
    val name: String,
    val active: Boolean = true,
)

@Serializable
data class BulkFareItemReq(
    @SerialName("area_a") val areaA: String,
    @SerialName("area_b") val areaB: String,
    @SerialName("amount_cents") val amountCents: Long,
)

@Serializable
data class BulkFaresReq(
    val fares: List<BulkFareItemReq>,
    @SerialName("create_missing_areas") val createMissingAreas: Boolean = false,
)

@Serializable
data class BulkFaresRespDto(
    val created: Int = 0,
    val updated: Int = 0,
    val unchanged: Int = 0,
    val total: Int = 0,
    @SerialName("areas_created") val areasCreated: List<String> = emptyList(),
)
