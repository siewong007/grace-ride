package org.pjstatecm.graceride.data.api

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonObject

@Serializable
data class OkResponse(val ok: Boolean = true)

@Serializable
data class GroupRef(val id: Long, val name: String)

@Serializable
data class Me(
    val id: Long,
    val name: String,
    val email: String,
    val phone: String = "",
    @SerialName("photo_file_id") val photoFileId: Long? = null,
    @SerialName("avatar_color") val avatarColor: String? = null,
    @SerialName("avatar_icon") val avatarIcon: String? = null,
    val roles: List<String> = emptyList(),
    @SerialName("role_names") val roleNames: Map<String, String> = emptyMap(),
    val permissions: List<String> = emptyList(),
    val status: String = "active",
    val lang: String = "zh",
    @SerialName("terms_accepted") val termsAccepted: Boolean = true,
    @SerialName("mfa_ok") val mfaOk: Boolean = false,
    @SerialName("totp_enabled") val totpEnabled: Boolean = false,
    @SerialName("has_password") val hasPassword: Boolean = false,
    @SerialName("passkey_count") val passkeyCount: Int = 0,
    @SerialName("auto_lock_mins") val autoLockMins: Int = 0,
    @SerialName("session_timeout_mins") val sessionTimeoutMins: Int = 0,
    @SerialName("backup_codes_count") val backupCodesCount: Int = 0,
    val locked: Boolean = false,
    val privileged: Boolean = false,
    val unread: Int = 0,
    val groups: List<GroupRef> = emptyList(),
    @SerialName("dev_mode") val devMode: Boolean = false,
    @SerialName("push_opt_in") val pushOptIn: Boolean = true,
    @SerialName("onboarding_dismissed") val onboardingDismissed: Boolean = false,
    @SerialName("telegram_chat_id") val telegramChatId: String? = null,
    @SerialName("whatsapp_opt_in") val whatsappOptIn: Boolean = false,
    @SerialName("badges_opt_out") val badgesOptOut: Boolean = false,
    @SerialName("readiness_prompt_mins") val readinessPromptMins: Int? = null,
) {
    fun can(page: String, act: String = "view"): Boolean = permissions.contains("$page.$act")
}

@Serializable
data class DevUser(val name: String, val email: String, val roles: String = "")

@Serializable
data class PasskeyStart(val challenge_id: String, val options: JsonObject)

@Serializable
data class PasskeyFinishReq(val challenge_id: String, val credential: kotlinx.serialization.json.JsonElement)

@Serializable
data class PasskeyInfo(val id: Long, val label: String = "", val created_at: String = "", val last_used: String? = null)

@Serializable
data class UnlockReq(
    val totp_code: String? = null,
    val backup_code: String? = null,
    val dev_unlock: Boolean? = null,
)

@Serializable
data class Area(val id: Long, val name: String, val active: Boolean = true)

@Serializable
data class Place(val label: String, val lat: Double = 0.0, val lng: Double = 0.0)

@Serializable
data class SavedPlace(
    val id: Long,
    val label: String,
    @SerialName("area_id") val areaId: Long? = null,
    @SerialName("area_name") val areaName: String? = null,
    val address: String = "",
    @SerialName("gate_notes") val gateNotes: String = "",
    @SerialName("is_default") val isDefault: Int = 0,
    val lat: Double? = null,
    val lng: Double? = null,
)

@Serializable
data class SavedPlaceReq(
    val label: String,
    val area_id: Long? = null,
    val address: String,
    val gate_notes: String? = null,
    val is_default: Boolean? = null,
    val lat: Double? = null,
    val lng: Double? = null,
)

@Serializable
data class CreateSavedPlaceRes(val id: Long, val ok: Boolean = true)

@Serializable
data class RideArea(val id: Long, val name: String)

@Serializable
data class RideEvent(
    val id: Long? = null,
    val title: String? = null,
    val private: Boolean = false,
    @SerialName("mileage_claimable") val mileageClaimable: Boolean = false,
)

@Serializable
data class Vehicle(
    val model: String = "",
    val color: String? = null,
    val plate: String? = null,
    val type: String? = null,
    @SerialName("wheelchair_ok") val wheelchairOk: Boolean = false,
    @SerialName("low_step") val lowStep: Boolean = false,
    @SerialName("child_seat") val childSeat: Boolean = false,
)

@Serializable
data class RideDriver(
    val id: Long,
    val name: String? = null,
    @SerialName("photo_file_id") val photoFileId: Long? = null,
    @SerialName("avatar_color") val avatarColor: String? = null,
    @SerialName("avatar_icon") val avatarIcon: String? = null,
    val phone: String? = null,
    val vehicle: Vehicle? = null,
)

@Serializable
data class RidePassenger(val id: Long, val name: String, val phone: String? = null)

@Serializable
data class RideProxy(
    @SerialName("is_proxy") val isProxy: Boolean = true,
    @SerialName("rider_name") val riderName: String = "",
    @SerialName("rider_phone") val riderPhone: String? = null,
    @SerialName("rider_relationship") val riderRelationship: String = "",
    @SerialName("booker_name") val bookerName: String = "",
    @SerialName("booker_phone") val bookerPhone: String? = null,
)

@Serializable
data class RideViewer(
    @SerialName("is_passenger") val isPassenger: Boolean = false,
    @SerialName("is_driver") val isDriver: Boolean = false,
    @SerialName("is_staff") val isStaff: Boolean = false,
)

@Serializable
data class RouteFare(val from: String = "", val to: String = "", @SerialName("amount_cents") val amountCents: Long? = null)

@Serializable
data class Ride(
    val id: Long,
    val status: String,
    val leg: String = "outbound",
    @SerialName("paired_ride_id") val pairedRideId: Long? = null,
    @SerialName("pickup_at") val pickupAt: String,
    val area: RideArea,
    @SerialName("pickup_address") val pickupAddress: String? = null,
    val destination: String = "",
    val seats: Long = 1,
    val assistance: String = "",
    val notes: String = "",
    val event: RideEvent? = null,
    @SerialName("as_group") val asGroup: GroupRef? = null,
    @SerialName("trip_id") val tripId: Long? = null,
    @SerialName("route_fare") val routeFare: RouteFare? = null,
    @SerialName("distance_km") val distanceKm: Long? = null,
    @SerialName("reimbursement_amount_cents") val reimbursementAmountCents: Long? = null,
    @SerialName("mileage_receipt_needed") val mileageReceiptNeeded: Boolean = false,
    @SerialName("readiness_status") val readinessStatus: String = "unconfirmed",
    @SerialName("readiness_updated_at") val readinessUpdatedAt: String? = null,
    @SerialName("delay_minutes") val delayMinutes: Int = 0,
    @SerialName("arrived_at_gate_at") val arrivedAtGateAt: String? = null,
    @SerialName("supper_choice") val supperChoice: String = "unspecified",
    @SerialName("is_proxy") val isProxy: Boolean = false,
    val proxy: RideProxy? = null,
    val passenger: RidePassenger,
    val driver: RideDriver? = null,
    @SerialName("needs_attention") val needsAttention: Boolean = false,
    @SerialName("attention_reason") val attentionReason: String = "",
    @SerialName("started_at") val startedAt: String? = null,
    @SerialName("picked_up_at") val pickedUpAt: String? = null,
    @SerialName("completed_at") val completedAt: String? = null,
    @SerialName("created_at") val createdAt: String = "",
    @SerialName("updated_at") val updatedAt: String = "",
    val viewer: RideViewer = RideViewer(),
)

@Serializable
data class CreateRideReq(
    val pickup_at: String,
    val pickup_area_id: Long,
    val pickup_address: String,
    val destination: String,
    val seats: Long? = null,
    val assistance: String? = null,
    val notes: String? = null,
    val event_id: Long? = null,
    val leg: String? = null,
    val is_proxy: Boolean? = null,
    val rider_name: String? = null,
    val rider_phone: String? = null,
    val rider_relationship: String? = null,
    val save_proxy_rider: Boolean? = null,
    val return_pickup_at: String? = null,
    val return_pickup_area_id: Long? = null,
    val return_pickup_address: String? = null,
    val return_destination: String? = null,
)

@Serializable
data class ReadinessReq(val status: String, val notes: String? = null)

@Serializable
data class RespondReq(val accept: Boolean)

@Serializable
data class CancelReq(val reason: String? = null)

@Serializable
data class DriverDelayReq(val delay_minutes: Int)

@Serializable
data class WithdrawReq(val reason: String? = null)

@Serializable
data class EventCreator(val id: Long, val name: String)

@Serializable
data class EventRideRef(val id: Long, val status: String)

@Serializable
data class MySignup(
    val status: String,
    @SerialName("need_ride") val needRide: Boolean = false,
    @SerialName("offer_drive") val offerDrive: Boolean = false,
    val attended: Boolean? = null,
    @SerialName("pass_code") val passCode: String? = null,
    @SerialName("as_group") val asGroup: GroupRef? = null,
)

@Serializable
data class EventItem(
    val id: Long,
    val title: String,
    val kind: String = "",
    val icon: String? = null,
    val description: String = "",
    @SerialName("starts_at") val startsAt: String,
    val location: String = "",
    val capacity: Long = 0,
    @SerialName("cost_note") val costNote: String = "",
    @SerialName("signup_deadline") val signupDeadline: String? = null,
    val status: String = "published",
    @SerialName("created_by") val createdBy: EventCreator? = null,
    val going: Long = 0,
    val waitlist: Long = 0,
    @SerialName("spots_left") val spotsLeft: Long? = null,
    @SerialName("my_signup") val mySignup: MySignup? = null,
    @SerialName("my_ride") val myRide: EventRideRef? = null,
    @SerialName("my_return_ride") val myReturnRide: EventRideRef? = null,
    /** Display names of the target groups (a string in event_json, not an array). */
    val groups: String = "",
    @SerialName("photo_file_id") val photoFileId: Long? = null,
    @SerialName("attendance_enabled") val attendanceEnabled: Boolean = false,
    @SerialName("mileage_claimable") val mileageClaimable: Boolean = false,
    val area: RideArea? = null,
    @SerialName("checkin_code") val checkinCode: String? = null,
    @SerialName("can_manage") val canManage: Boolean = false,
    @SerialName("can_publish") val canPublish: Boolean = false,
    @SerialName("can_unpublish") val canUnpublish: Boolean = false,
    @SerialName("can_cancel") val canCancel: Boolean = false,
    @SerialName("can_see_signups") val canSeeSignups: Boolean = false,
    @SerialName("join_as") val joinAs: List<GroupRef> = emptyList(),
)

@Serializable
data class SignupReq(
    val need_ride: Boolean? = null,
    val offer_drive: Boolean? = null,
    val offer_seats: Long? = null,
    val as_group_id: Long? = null,
)

@Serializable
data class Share(
    val id: Long,
    @SerialName("settlement_id") val settlementId: Long,
    val title: String,
    @SerialName("event_title") val eventTitle: String = "",
    @SerialName("amount_cents") val amountCents: Long,
    val status: String,
    @SerialName("payee_name") val payeeName: String = "",
    @SerialName("payee_account") val payeeAccount: String = "",
    @SerialName("payee_qr_file_id") val payeeQrFileId: Long? = null,
    @SerialName("due_date") val dueDate: String? = null,
    val note: String = "",
    @SerialName("txn_ref") val txnRef: String = "",
    @SerialName("proof_file_id") val proofFileId: Long? = null,
)

@Serializable
data class ProofReq(val proof_file_id: Long, val txn_ref: String, val paid_date: String? = null)

@Serializable
data class Announcement(
    val id: Long,
    val title: String,
    val body: String = "",
    val pinned: Boolean = false,
    val by: String = "",
    @SerialName("created_at") val createdAt: String = "",
    val targeted: Boolean = false,
    val groups: List<GroupRef> = emptyList(),
)

@Serializable
data class NotificationItem(
    val id: Long,
    val title: String,
    val body: String = "",
    val link: String? = null,
    val read: Boolean = false,
    @SerialName("created_at") val createdAt: String = "",
)

@Serializable
data class FileRef(val id: Long, val mime: String = "", val name: String = "")

@Serializable
data class ProfileUpdateReq(
    val name: String? = null,
    val phone: String? = null,
    val lang: String? = null,
    val auto_lock_mins: Int? = null,
    val telegram_chat_id: String? = null,
    val whatsapp_opt_in: Boolean? = null,
    val push_opt_in: Boolean? = null,
    val badges_opt_out: Boolean? = null,
    val readiness_prompt_mins: Int? = null,
    val photo_file_id: Long? = null,
    val clear_photo: Boolean? = null,
    val avatar_color: String? = null,
    val avatar_icon: String? = null,
)

@Serializable
data class ProxyRider(
    val id: Long,
    val name: String,
    val phone: String = "",
    val relationship: String = "",
    val notes: String = "",
    @SerialName("area_id") val areaId: Long? = null,
    @SerialName("pickup_address") val pickupAddress: String = "",
    @SerialName("gate_notes") val gateNotes: String = "",
)

@Serializable
data class BadgeTierStep(
    val tier: String,
    val target: Long,
    val reached: Boolean = false,
)

@Serializable
data class Badge(
    val key: String,
    val category: String? = null,
    @SerialName("role_match") val roleMatch: Boolean = true,
    val char: String,
    val icon: String? = null,
    val name: String = "",
    val rule: String = "",
    @SerialName("char_zh") val charZh: String? = null,
    @SerialName("name_zh") val nameZh: String? = null,
    @SerialName("rule_zh") val ruleZh: String? = null,
    val progress: Long = 0,
    val target: Long = 0,
    val earned: Boolean = false,
    val tier: String = "none",
    @SerialName("tier_level") val tierLevel: Int = 0,
    @SerialName("next_tier") val nextTier: String? = null,
    @SerialName("next_target") val nextTarget: Long? = null,
    val tiers: List<BadgeTierStep> = emptyList(),
)

@Serializable
data class CreateRideRes(val id: Long, val return_ride_id: Long? = null)

@Serializable
data class TermsReq(val accepted: Boolean)

@Serializable
data class DriverVehicle(
    val id: Long? = null,
    val model: String = "",
    val color: String = "",
    val plate: String = "",
    val seats: Int = 5,
    @SerialName("vehicle_type") val vehicleType: String = "standard",
    @SerialName("wheelchair_ok") val wheelchairOk: Boolean = false,
    @SerialName("low_step") val lowStep: Boolean = false,
    @SerialName("child_seat") val childSeat: Boolean = false,
    @SerialName("is_active") val isActive: Boolean = false,
)

@Serializable
data class DriverProfile(
    @SerialName("carpool_enabled") val carpoolEnabled: Boolean = false,
    @SerialName("spare_seats") val spareSeats: Int = 1,
    @SerialName("gate_wait_mins") val gateWaitMins: Int = 5,
    @SerialName("area_ids") val areaIds: List<Long> = emptyList(),
    val vehicle: DriverVehicle? = null,
    val vehicles: List<DriverVehicle> = emptyList(),
    @SerialName("active_vehicle_id") val activeVehicleId: Long? = null,
)

@Serializable
data class DriverProfileReq(
    val carpool_enabled: Boolean,
    val spare_seats: Int,
    val gate_wait_mins: Int,
    val area_ids: List<Long>,
    val active_vehicle_id: Long? = null,
)

@Serializable
data class DriverVehicleReq(
    val model: String,
    val color: String,
    val plate: String,
    val seats: Int,
    val vehicle_type: String = "standard",
    val wheelchair_ok: Boolean = false,
    val low_step: Boolean = false,
    val child_seat: Boolean = false,
    val is_active: Boolean = false,
)

@Serializable
data class SelectVehicleRes(
    val ok: Boolean = true,
    @SerialName("active_vehicle_id") val activeVehicleId: Long? = null,
)

@Serializable
data class ProxyRiderReq(
    val name: String,
    val phone: String? = null,
    val relationship: String? = null,
    val notes: String? = null,
    val area_id: Long? = null,
    val pickup_address: String? = null,
    val gate_notes: String? = null,
)

@Serializable
data class CreateProxyRiderRes(val id: Long, val ok: Boolean = true)
