package org.pjstatecm.graceride.data.api

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

// ---- Recovery, Invites, Visit & Care Links ----

@Serializable
data class RecoveryRequestIn(
    val email: String,
    val reason: String = "",
    val turnstile_token: String? = null,
)

@Serializable
data class InvitePreviewDto(
    val kind: String = "member",
    val email: String = "",
    val name: String = "",
    @SerialName("family_name") val familyName: String? = null,
    val phone: String? = null,
)

@Serializable
data class InviteAcceptReq(
    val name: String? = null,
    val email: String? = null,
    val phone: String? = null,
    val terms: Boolean = true,
    val turnstile_token: String? = null,
)

@Serializable
data class PublicEventBriefDto(
    val title: String = "",
    @SerialName("starts_at") val startsAt: String = "",
    val location: String = "",
)

@Serializable
data class VisitPreviewDto(
    val state: String = "closed",
    @SerialName("closed_reason") val closedReason: String? = null,
    @SerialName("opens_at") val opensAt: String = "",
    val name: String = "",
    val host: String = "",
    val event: PublicEventBriefDto = PublicEventBriefDto(),
)

@Serializable
data class VisitClaimReq(val turnstile_token: String? = null)

@Serializable
data class VisitHostDto(
    val name: String = "",
    val phone: String = "",
)

@Serializable
data class VisitDriverVehicleDto(
    val model: String? = null,
    val color: String? = null,
    val plate: String? = null,
)

@Serializable
data class VisitDriverDto(
    val name: String = "",
    val phone: String? = null,
    val vehicle: VisitDriverVehicleDto? = null,
)

@Serializable
data class VisitRideDetailDto(
    val status: String = "",
    @SerialName("pickup_at") val pickupAt: String = "",
    @SerialName("pickup_address") val pickupAddress: String = "",
    val area: String = "",
    val destination: String = "",
    val seats: Int = 1,
    @SerialName("readiness_status") val readinessStatus: String = "unconfirmed",
    @SerialName("delay_minutes") val delayMinutes: Int? = null,
    @SerialName("arrived_at_gate_at") val arrivedAtGateAt: String? = null,
    @SerialName("picked_up_at") val pickedUpAt: String? = null,
    val driver: VisitDriverDto? = null,
)

@Serializable
data class VisitRideDto(
    val name: String = "",
    val host: VisitHostDto = VisitHostDto(),
    val event: PublicEventBriefDto = PublicEventBriefDto(),
    val ride: VisitRideDetailDto = VisitRideDetailDto(),
)

@Serializable
data class CareLocationEtaDto(
    @SerialName("duration_s") val durationS: Double? = null,
    @SerialName("distance_m") val distanceM: Double? = null,
    val eta: String? = null,
)

@Serializable
data class CareLocationDto(
    val sharing: Boolean = false,
    val lat: Double? = null,
    val lng: Double? = null,
    @SerialName("updated_at") val updatedAt: String? = null,
    val eta: CareLocationEtaDto? = null,
)

@Serializable
data class CareStatusDto(
    @SerialName("ride_id") val rideId: Long = 0,
    val label: String = "",
    @SerialName("rider_name") val riderName: String = "",
    @SerialName("booker_name") val bookerName: String = "",
    val status: String = "",
    val leg: String? = null,
    @SerialName("pickup_at") val pickupAt: String = "",
    @SerialName("opens_at") val opensAt: String? = null,
    val area: Area = Area(0, ""),
    val destination: String = "",
    @SerialName("readiness_status") val readinessStatus: String = "unconfirmed",
    @SerialName("delay_minutes") val delayMinutes: Int = 0,
    @SerialName("arrived_at_gate_at") val arrivedAtGateAt: String? = null,
    @SerialName("started_at") val startedAt: String? = null,
    @SerialName("picked_up_at") val pickedUpAt: String? = null,
    @SerialName("completed_at") val completedAt: String? = null,
    val driver: VisitDriverDto? = null,
    val location: CareLocationDto = CareLocationDto(),
)

@Serializable
data class LicensePackageDto(
    val name: String,
    val version: String = "",
    val license: String = "",
    val text: String = "",
)

// ---- Auth TOTP & Lock ----

@Serializable
data class LockRespDto(
    val ok: Boolean = true,
    @SerialName("logged_out") val loggedOut: Boolean = false,
)

@Serializable
data class TotpSetupDto(
    val secret: String = "",
    val url: String = "",
)

@Serializable
data class TotpCodeReq(val code: String)

@Serializable
data class TotpEnableRespDto(
    val ok: Boolean = true,
    @SerialName("backup_codes") val backupCodes: List<String> = emptyList(),
)

@Serializable
data class BackupCodesRespDto(
    val ok: Boolean = true,
    @SerialName("backup_codes") val backupCodes: List<String> = emptyList(),
)

// ---- Calendar Feed, Push Devices & Telegram ----

@Serializable
data class CalendarFeedDto(val enabled: Boolean = false)

@Serializable
data class CalendarFeedRotateDto(
    val token: String = "",
    @SerialName("ics_path") val icsPath: String = "",
)

@Serializable
data class PushConfigDto(
    val enabled: Boolean = true,
    val configured: Boolean = false,
    @SerialName("vapid_public_key") val vapidPublicKey: String = "",
    @SerialName("subscription_count") val subscriptionCount: Int = 0,
    @SerialName("mobile_subscription_count") val mobileSubscriptionCount: Int = 0,
    @SerialName("whatsapp_configured") val whatsappConfigured: Boolean = false,
    @SerialName("telegram_configured") val telegramConfigured: Boolean = false,
    @SerialName("telegram_bot_username") val telegramBotUsername: String = "",
)

@Serializable
data class WebPushSubDto(
    val id: Long,
    @SerialName("device_label") val deviceLabel: String = "",
    @SerialName("endpoint_preview") val endpointPreview: String = "",
    @SerialName("created_at") val createdAt: String = "",
    @SerialName("last_delivered_at") val lastDeliveredAt: String? = null,
)

@Serializable
data class MobilePushSubDto(
    val id: Long,
    val platform: String = "",
    val provider: String = "",
    @SerialName("device_label") val deviceLabel: String = "",
    @SerialName("created_at") val createdAt: String = "",
    @SerialName("last_delivered_at") val lastDeliveredAt: String? = null,
)

@Serializable
data class PushSubscriptionsDto(
    val web: List<WebPushSubDto> = emptyList(),
    val mobile: List<MobilePushSubDto> = emptyList(),
)

@Serializable
data class PushTestRespDto(
    val ok: Boolean = true,
    val delivered: Int = 0,
    val total: Int = 0,
)

@Serializable
data class TelegramLinkTokenDto(
    val url: String = "",
    @SerialName("expires_at") val expiresAt: String = "",
)

@Serializable
data class TelegramTestRespDto(
    val ok: Boolean = true,
    val delivered: Boolean = false,
    val error: String? = null,
)

// ---- Family Circle ----

@Serializable
data class FamilyInfoDto(
    val id: Long,
    val name: String = "",
    @SerialName("owner_id") val ownerId: Long = 0,
)

@Serializable
data class FamilyMemberDto(
    @SerialName("user_id") val userId: Long,
    val name: String = "",
    val status: String = "active",
)

@Serializable
data class FamilyRequestDto(
    val id: Long,
    @SerialName("user_id") val userId: Long = 0,
    val name: String = "",
)

@Serializable
data class FamilyInviteDto(
    val id: Long,
    val phone: String = "",
    @SerialName("expires_at") val expiresAt: String = "",
    val url: String = "",
)

@Serializable
data class FamilyIncomingDto(
    val id: Long,
    @SerialName("family_name") val familyName: String = "",
    @SerialName("owner_name") val ownerName: String = "",
)

@Serializable
data class FamilyStateDto(
    val family: FamilyInfoDto? = null,
    val members: List<FamilyMemberDto> = emptyList(),
    val requests: List<FamilyRequestDto> = emptyList(),
    val invites: List<FamilyInviteDto> = emptyList(),
    val incoming: FamilyIncomingDto? = null,
)

@Serializable
data class FamilyNameReq(val name: String)

@Serializable
data class FamilyCreateRespDto(val id: Long, val name: String = "")

@Serializable
data class FamilyPhoneReq(val phone: String)

@Serializable
data class FamilyAddPersonRespDto(
    val kind: String = "",
    val id: Long = 0,
    val name: String? = null,
    val phone: String? = null,
    @SerialName("expires_at") val expiresAt: String? = null,
    val url: String? = null,
)
