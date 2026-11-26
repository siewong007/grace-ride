package org.pjstatecm.graceride.data.api

import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.descriptors.PrimitiveKind
import kotlinx.serialization.descriptors.PrimitiveSerialDescriptor
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder
import kotlinx.serialization.json.JsonDecoder
import kotlinx.serialization.json.JsonEncoder
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.longOrNull

object FlexibleBooleanSerializer : KSerializer<Boolean?> {
    override val descriptor: SerialDescriptor =
        PrimitiveSerialDescriptor("FlexibleBoolean", PrimitiveKind.STRING)

    override fun deserialize(decoder: Decoder): Boolean? {
        val jsonDecoder = decoder as? JsonDecoder
            ?: return decoder.decodeBoolean()
        return when (val el = jsonDecoder.decodeJsonElement()) {
            is JsonNull -> null
            is JsonPrimitive -> el.booleanOrNull ?: el.longOrNull?.let { it != 0L }
            else -> null
        }
    }

    override fun serialize(encoder: Encoder, value: Boolean?) {
        val jsonEncoder = encoder as? JsonEncoder
        if (jsonEncoder != null) {
            jsonEncoder.encodeJsonElement(if (value == null) JsonNull else JsonPrimitive(value))
        } else if (value != null) {
            encoder.encodeBoolean(value)
        }
    }
}

object NullableNonBlankStringSerializer : KSerializer<String?> {
    override val descriptor: SerialDescriptor =
        PrimitiveSerialDescriptor("NullableNonBlankString", PrimitiveKind.STRING)

    override fun deserialize(decoder: Decoder): String? {
        val jsonDecoder = decoder as? JsonDecoder
        if (jsonDecoder != null) {
            return when (val el = jsonDecoder.decodeJsonElement()) {
                is JsonNull -> null
                is JsonPrimitive -> el.contentOrNull?.takeIf { it.isNotBlank() }
                else -> null
            }
        }
        return decoder.decodeString().takeIf { it.isNotBlank() }
    }

    override fun serialize(encoder: Encoder, value: String?) {
        val jsonEncoder = encoder as? JsonEncoder
        if (jsonEncoder != null) {
            jsonEncoder.encodeJsonElement(if (value == null) JsonNull else JsonPrimitive(value))
        } else if (value != null) {
            encoder.encodeString(value)
        }
    }
}

@Serializable
data class OkResponse(val ok: Boolean = true)

@Serializable
data class GroupRef(val id: Long, val name: String = "")

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
    @SerialName("biometric_count") val biometricCount: Int = 0,
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
    @SerialName("whatsapp_opt_in") val whatsappOptIn: Boolean = false,
    @SerialName("badges_opt_out") val badgesOptOut: Boolean = false,
    @SerialName("readiness_prompt_mins") val readinessPromptMins: Int? = null,
) {
    fun can(page: String, act: String = "view"): Boolean = permissions.contains("$page.$act")

    /** True when this account holds any staff/admin (`admin_*`) permission. */
    fun hasStaffAccess(): Boolean = permissions.any { it.startsWith("admin_") }

    /** Traffic leads can approve/return plan & ride drafts and edit live visitors directly. */
    fun canApprovePlan(): Boolean =
        roles.any { it in setOf("traffic_head", "admin", "superadmin", "president", "vice_president") }

    /** Executive / superadmin roles allowed to toggle event mileage claimability. */
    fun canAuthoriseMileage(): Boolean =
        roles.any { it in setOf("superadmin", "president", "vice_president") }
}

@Serializable
data class DevUser(val name: String, val email: String, val roles: String = "")

@Serializable
data class TurnstileConfigDto(
    val enabled: Boolean = false,
    @SerialName("site_key") val siteKey: String? = null,
)

@Serializable
data class PasskeyStartReq(val email: String, val turnstile_token: String? = null)

@Serializable
data class PasskeyStart(val challenge_id: String, val options: JsonObject)

@Serializable
data class PasskeyFinishReq(
    val challenge_id: String,
    val credential: kotlinx.serialization.json.JsonElement,
    val label: String? = null,
)

@Serializable
data class PasskeyInfo(val id: Long, val label: String = "", val created_at: String = "", val last_used: String? = null)

@Serializable
data class BiometricEnrollment(
    val id: Long,
    val credentialId: String,
    val userId: Long,
    val email: String,
    val userName: String,
    val label: String = "",
    val enrolledAtMs: Long = 0L,
)

@Serializable
data class BiometricChallengeRes(val challenge_id: String, val challenge: String)

@Serializable
data class BiometricRegisterFinishReq(
    val challenge_id: String,
    val public_key: String,
    val signature: String,
    val label: String? = null,
    val attestation_chain: List<String>? = null,
)

@Serializable
data class BiometricRegisterFinishRes(
    val ok: Boolean = true,
    val id: Long = 0L,
    val credential_id: String = "",
    val label: String = "",
)

@Serializable
data class BiometricLoginStartReq(val credential_id: String, val turnstile_token: String? = null)

@Serializable
data class BiometricReauthStartReq(val credential_id: String)

@Serializable
data class BiometricFinishReq(
    val challenge_id: String,
    val credential_id: String,
    val signature: String,
)

@Serializable
data class BiometricInfo(
    val id: Long,
    val credential_id: String = "",
    val label: String = "",
    val created_at: String = "",
    val last_used: String? = null,
)

@Serializable
data class UnlockReq(
    val totp_code: String? = null,
    val backup_code: String? = null,
    val dev_unlock: Boolean? = null,
)

@Serializable
data class Area(
    val id: Long,
    val name: String,
    val active: Boolean = true,
    @SerialName("photo_count") val photoCount: Int = 0,
)

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
    val clear_coords: Boolean? = null,
)

@Serializable
data class CreateSavedPlaceRes(val id: Long, val ok: Boolean = true)

@Serializable
data class RideArea(val id: Long, val name: String = "")

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
    @SerialName("reimbursement_excluded") val reimbursementExcluded: Boolean = false,
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
data class SupperChoiceReq(val choice: String)

@Serializable
data class EventCreator(val id: Long, val name: String)

@Serializable
data class EventRideRef(val id: Long, val status: String)

@Serializable
data class MySignup(
    val status: String,
    @SerialName("need_ride") val needRide: Boolean = false,
    @SerialName("offer_drive") val offerDrive: Boolean = false,
    @SerialName("self_transport") val selfTransport: Boolean = false,
    @Serializable(with = FlexibleBooleanSerializer::class) val attended: Boolean? = null,
    @SerialName("pass_code") val passCode: String? = null,
    @SerialName("as_group") val asGroup: GroupRef? = null,
)

@Serializable
data class EventSettlementRef(
    val id: Long,
    val status: String = "",
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
    @SerialName("series_id") val seriesId: String? = null,
    val going: Long = 0,
    val waitlist: Long = 0,
    @SerialName("attended_count") val attendedCount: Long? = null,
    @SerialName("unassigned_rides") val unassignedRides: Long? = null,
    @SerialName("spots_left") val spotsLeft: Long? = null,
    @SerialName("my_signup") val mySignup: MySignup? = null,
    @SerialName("my_ride") val myRide: EventRideRef? = null,
    @SerialName("my_return_ride") val myReturnRide: EventRideRef? = null,
    /** Display names of the target groups (a string in event_json, not an array). */
    val groups: String = "",
    @SerialName("group_ids") val groupIds: List<Long> = emptyList(),
    @SerialName("as_group_ids") val asGroupIds: List<Long> = emptyList(),
    @SerialName("photo_file_id") val photoFileId: Long? = null,
    @SerialName("attendance_enabled") val attendanceEnabled: Boolean = false,
    @SerialName("mileage_claimable") val mileageClaimable: Boolean = false,
    val area: RideArea? = null,
    @SerialName("checkin_code") val checkinCode: String? = null,
    val settlement: EventSettlementRef? = null,
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
    val self_transport: Boolean? = null,
)

@Serializable
data class EventTransportReq(val self_transport: Boolean)

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
    @Serializable(with = NullableNonBlankStringSerializer::class) val link: String? = null,
    val read: Boolean = false,
    @SerialName("created_at") val createdAt: String = "",
    val category: String = "general",
    @SerialName("event_type") val eventType: String = "general",
    val priority: String = "normal",
    @SerialName("entity_type") val entityType: String = "",
    @SerialName("entity_id") val entityId: Long? = null,
)

@Serializable
data class NotificationPageDto(
    val items: List<NotificationItem> = emptyList(),
    @SerialName("next_before") val nextBefore: Long? = null,
    val unread: Int = 0,
)

@Serializable
data class NotificationActionResp(
    val ok: Boolean = true,
    val unread: Int = 0,
)

@Serializable
data class ReadAllReq(val category: String? = null)

@Serializable
data class NotificationPrefsDto(
    @SerialName("push_opt_in") val pushOptIn: Boolean = true,
    @SerialName("whatsapp_opt_in") val whatsappOptIn: Boolean = false,
    @SerialName("telegram_chat_id") val telegramChatId: String = "",
    @SerialName("telegram_blocked") val telegramBlocked: Boolean = false,
    @SerialName("readiness_prompt_mins") val readinessPromptMins: Int = 45,
    @SerialName("driver_remind_mins") val driverRemindMins: Int = 60,
    @SerialName("muted_categories") val mutedCategories: List<String> = emptyList(),
    @SerialName("quiet_hours_start") val quietHoursStart: String = "",
    @SerialName("quiet_hours_end") val quietHoursEnd: String = "",
    @SerialName("hide_lock_details") val hideLockDetails: Boolean = false,
)

@Serializable
data class UpdateNotificationPrefsReq(
    val push_opt_in: Boolean? = null,
    val whatsapp_opt_in: Boolean? = null,
    val telegram_chat_id: String? = null,
    val readiness_prompt_mins: Int? = null,
    val driver_remind_mins: Int? = null,
    val muted_categories: List<String>? = null,
    val quiet_hours_start: String? = null,
    val quiet_hours_end: String? = null,
    val hide_lock_details: Boolean? = null,
)

@Serializable
data class MobilePushTokenReq(
    val platform: String = "android",
    val provider: String = "unifiedpush",
    val token: String,
    val device_label: String = "",
)

@Serializable
data class MobilePushTokenDel(val token: String)

@Serializable
data class LogoutReq(
    val push_endpoint: String? = null,
    val mobile_token: String? = null,
)

@Serializable
data class FileRef(val id: Long, val mime: String = "", val name: String = "")

@Serializable
data class ProfileUpdateReq(
    val name: String? = null,
    val phone: String? = null,
    val lang: String? = null,
    val auto_lock_mins: Int? = null,
    val whatsapp_opt_in: Boolean? = null,
    val push_opt_in: Boolean? = null,
    val telegram_chat_id: String? = null,
    val badges_opt_out: Boolean? = null,
    val onboarding_dismissed: Boolean? = null,
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
    @SerialName("caregiver_user_id") val caregiverUserId: Long? = null,
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

@OptIn(kotlinx.serialization.ExperimentalSerializationApi::class)
@Serializable
data class DriverVehicleReq(
    val model: String,
    val color: String,
    val plate: String,
    val seats: Int,
    @kotlinx.serialization.EncodeDefault val vehicle_type: String = "standard",
    @kotlinx.serialization.EncodeDefault val wheelchair_ok: Boolean = false,
    @kotlinx.serialization.EncodeDefault val low_step: Boolean = false,
    @kotlinx.serialization.EncodeDefault val child_seat: Boolean = false,
    @kotlinx.serialization.EncodeDefault val is_active: Boolean = false,
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
    val caregiver_user_id: Long? = null,
)

@Serializable
data class CreateProxyRiderRes(val id: Long, val ok: Boolean = true)

// ---- Trip Group Chat ----

@Serializable
data class ChatReactionDto(
    val emoji: String,
    val count: Int = 0,
    val mine: Boolean = false,
    val names: List<String> = emptyList(),
)

@Serializable
data class ChatMessageDto(
    val id: Long,
    @SerialName("user_id") val userId: Long? = null,
    val name: String? = null,
    val body: String = "",
    @SerialName("created_at") val createdAt: String = "",
    val mine: Boolean = false,
    val status: String? = null,
    val reactions: List<ChatReactionDto> = emptyList(),
)

@Serializable
data class ChatThreadDto(
    val open: Boolean = false,
    @SerialName("can_post") val canPost: Boolean = true,
    val members: List<String> = emptyList(),
    @SerialName("reactions_allowed") val reactionsAllowed: List<String> = emptyList(),
    val messages: List<ChatMessageDto> = emptyList(),
)

@Serializable
data class PostMessageReq(val body: String)

@Serializable
data class PostMessageRes(val ok: Boolean = true, val id: Long = 0)

@Serializable
data class MarkReadReq(val up_to: Long)

@Serializable
data class ReactReq(val emoji: String)

@Serializable
data class ReactRes(val ok: Boolean = true, val emoji: String? = null)

// ---- Driver Hub Route & Live Location ----

@Serializable
data class RouteOriginDto(
    val lat: Double,
    val lng: Double,
    val heading: Double? = null,
    @SerialName("updated_at") val updatedAt: String = "",
)

@Serializable
data class RouteStopDto(
    @SerialName("ride_id") val rideId: Long,
    val passenger: String = "",
    val address: String = "",
    @SerialName("pickup_at") val pickupAt: String = "",
    val seats: Long = 1,
    val started: Boolean = false,
    val lat: Double? = null,
    val lng: Double? = null,
    val location: String? = null,
    @SerialName("leg_distance_m") val legDistanceM: Double? = null,
    @SerialName("leg_duration_s") val legDurationS: Double? = null,
    @SerialName("distance_m") val distanceM: Double? = null,
    @SerialName("duration_s") val durationS: Double? = null,
    val eta: String? = null,
)

@Serializable
data class RouteDestinationDto(
    val name: String = "",
    val lat: Double? = null,
    val lng: Double? = null,
    val approx: Boolean = false,
    @SerialName("distance_m") val distanceM: Double? = null,
    @SerialName("duration_s") val durationS: Double? = null,
    val eta: String? = null,
)

@Serializable
data class TripRouteDto(
    @SerialName("trip_id") val tripId: Long,
    val live: Boolean = false,
    val origin: RouteOriginDto? = null,
    val stops: List<RouteStopDto> = emptyList(),
    val onboard: Long = 0,
    val destination: RouteDestinationDto = RouteDestinationDto(),
    @SerialName("distance_m") val distanceM: Double = 0.0,
    @SerialName("duration_s") val durationS: Double = 0.0,
    val approximate: Boolean = false,
    @SerialName("computed_at") val computedAt: String = "",
    val geometry: List<List<Double>> = emptyList(),
)

@Serializable
data class DriverRouteResp(val route: TripRouteDto? = null)

@Serializable
data class DriverLocationReq(val lat: Double, val lng: Double, val heading: Double? = null)

@Serializable
data class PassengerLocationReq(val lat: Double, val lng: Double)

@Serializable
data class LocationShareResp(val ok: Boolean = true, val sharing: Boolean = false)

@Serializable
data class PassengerSpotDto(
    val lat: Double,
    val lng: Double,
    @SerialName("updated_at") val updatedAt: String = "",
)

@Serializable
data class RideEtaDto(
    val phase: String = "pickup",
    val located: Boolean = false,
    @SerialName("stops_before") val stopsBefore: Int = 0,
    val approximate: Boolean = false,
    @SerialName("computed_at") val computedAt: String = "",
    @SerialName("distance_m") val distanceM: Double? = null,
    @SerialName("duration_s") val durationS: Double? = null,
    val eta: String? = null,
    val destination: String? = null,
    val location: String? = null,
    val target: List<Double>? = null,
    val path: List<List<Double>> = emptyList(),
)

@Serializable
data class RideLocationDto(
    val sharing: Boolean = false,
    val lat: Double? = null,
    val lng: Double? = null,
    val heading: Double? = null,
    @SerialName("updated_at") val updatedAt: String? = null,
    val passenger: PassengerSpotDto? = null,
    val eta: RideEtaDto? = null,
)

// ---- Event Check-In & Attendance ----

@Serializable
data class CheckinInfoDto(
    val id: Long,
    val title: String = "",
    @SerialName("starts_at") val startsAt: String = "",
    val location: String = "",
    @SerialName("attendance_enabled") val attendanceEnabled: Boolean = false,
    @SerialName("my_attended") val myAttended: Boolean = false,
    @SerialName("is_member") val isMember: Boolean = false,
)

@Serializable
data class CheckinReq(val code: String? = null)

@Serializable
data class CheckinByCodeReq(val code: String)

@Serializable
data class CheckinResp(
    val ok: Boolean = true,
    @SerialName("already_attended") val alreadyAttended: Boolean = false,
    @SerialName("event_id") val eventId: Long = 0,
    val title: String = "",
    @SerialName("starts_at") val startsAt: String = "",
    val location: String = "",
)

@Serializable
data class ScanMemberReq(val pass_code: String)

@Serializable
data class ScanMemberResp(
    val ok: Boolean = true,
    @SerialName("user_id") val userId: Long = 0,
    @SerialName("member_name") val memberName: String = "",
    @SerialName("already_attended") val alreadyAttended: Boolean = false,
)

@Serializable
data class MarkAttendanceReq(val user_id: Long, val attended: Boolean? = null)

@Serializable
data class RidesPageDto(
    val items: List<Ride> = emptyList(),
    @SerialName("next_before") val nextBefore: String? = null,
)

@Serializable
data class CareLinkReq(val label: String? = null)

@Serializable
data class CareLinkResp(
    val token: String = "",
    val url: String = "",
    @SerialName("valid_until") val validUntil: String = "",
)

@Serializable
data class ExtractRefReq(val text: String? = null)

@Serializable
data class ExtractRefResp(
    val ref: String = "",
    val candidates: List<String> = emptyList(),
)
