package org.pjstatecm.graceride.data.api

import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.descriptors.buildClassSerialDescriptor
import kotlinx.serialization.descriptors.element
import kotlinx.serialization.encoding.CompositeDecoder
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder
import kotlinx.serialization.json.JsonDecoder
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonEncoder
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonPrimitive

// ---------------- Admin Users, Roles, Groups, Invites & Recovery ----------------

@Serializable
data class AdminUserDto(
    val id: Long,
    val name: String,
    val email: String = "",
    val phone: String = "",
    val roles: List<String> = emptyList(),
    val status: String = "active",
    @SerialName("created_at") val createdAt: String = "",
    val passkeys: Long = 0,
    val groups: String = "",
    @SerialName("photo_file_id") val photoFileId: Long? = null,
    @SerialName("avatar_color") val avatarColor: String? = null,
    @SerialName("avatar_icon") val avatarIcon: String? = null,
    @SerialName("family_name") val familyName: String? = null,
)

@Serializable
data class AdminUsersPageDto(
    val items: List<AdminUserDto> = emptyList(),
    val total: Long = 0,
    val page: Int = 1,
    @SerialName("page_size") val pageSize: Int = 50,
    @SerialName("total_pages") val totalPages: Int = 1,
)

@Serializable
data class UpdateAdminUserReq(
    val roles: List<String>,
    val status: String,
    val group_ids: List<Long>? = null,
    val president_acknowledged: Boolean,
)

@Serializable
data class AdminRoleDefDto(
    val key: String,
    val name: String = "",
    val description: String = "",
    val system: Boolean = false,
    val implicit: Boolean = false,
    val fixed: Boolean = false,
    val assignable: Boolean = true,
    val permissions: List<String> = emptyList(),
    val locked: List<String> = emptyList(),
    val members: Long = 0,
)

@Serializable
data class RoleCatalogEntryDto(
    val page: String,
    val area: String = "",
    val actions: List<String> = emptyList(),
)

@Serializable
data class AdminRolesRespDto(
    val catalog: List<RoleCatalogEntryDto> = emptyList(),
    val roles: List<AdminRoleDefDto> = emptyList(),
)

@Serializable
data class RoleSaveReq(
    val name: String,
    val description: String = "",
    val permissions: List<String>,
)

@Serializable
data class RoleKeyRespDto(
    val key: String,
)

@Serializable
data class GroupDto(
    val id: Long,
    val name: String,
    val description: String = "",
    @SerialName("is_all") val isAll: Boolean = false,
    @SerialName("member_count") val memberCount: Long = 0,
    @SerialName("parent_id") val parentId: Long? = null,
    @SerialName("is_visitor") val isVisitor: Boolean = false,
)

@Serializable
data class AdminInviteDto(
    val id: Long,
    val kind: String = "invite",
    val email: String = "",
    val name: String = "",
    val roles: List<String> = emptyList(),
    @SerialName("expires_at") val expiresAt: String = "",
    @SerialName("used_at") val usedAt: String? = null,
    @SerialName("created_at") val createdAt: String = "",
    val status: String = "open",
    val url: String? = null,
)

@Serializable
data class CreateInviteResDto(
    val id: Long,
    val token: String = "",
    val url: String = "",
)

@Serializable
data class RecoveryRequestDto(
    val id: Long,
    val email: String,
    val reason: String = "",
    val status: String = "pending",
    @SerialName("created_at") val createdAt: String = "",
    val name: String? = null,
)

@Serializable
data class ApproveRecoveryResDto(
    val url: String? = null,
    @SerialName("awaiting_second") val awaitingSecond: Boolean = false,
)

// ---------------- Announcements Admin Requests ----------------

@Serializable
data class CreateAnnouncementReq(
    val title: String,
    val body: String,
    val pinned: Boolean,
    val notify_all: Boolean,
    val group_ids: List<Long>,
)

/**
 * Update payload for an existing announcement. Keep [group_ids] `null` when the
 * audience is unchanged or when toggling pin so `explicitNulls = false` omits the
 * field and preserves existing targeted groups on the backend.
 */
@Serializable
data class UpdateAnnouncementReq(
    val title: String,
    val body: String,
    val pinned: Boolean,
    val group_ids: List<Long>? = null,
)

// ---------------- Audit Log ----------------

@Serializable
data class AuditAttendanceDto(
    @SerialName("member_id") val memberId: Long,
    @SerialName("member_name") val memberName: String? = null,
    val status: String = "",
)

@Serializable
data class AuditItemDto(
    val id: Long,
    @SerialName("actor_id") val actorId: Long? = null,
    val actor: String? = null,
    val action: String,
    val entity: String = "",
    @SerialName("entity_id") val entityId: Long? = null,
    @SerialName("entity_name") val entityName: String? = null,
    @SerialName("entity_at") val entityAt: String? = null,
    val detail: String = "",
    val attendance: AuditAttendanceDto? = null,
    @SerialName("created_at") val createdAt: String = "",
)

@Serializable
data class AuditPageDto(
    val items: List<AuditItemDto> = emptyList(),
    @SerialName("next_before") val nextBefore: Long? = null,
    @SerialName("retention_days") val retentionDays: Int? = null,
)

// ---------------- Admin System Settings, Alerts & Groups ----------------

@Serializable
data class SystemSettingsDto(
    @SerialName("audit_retention_days") val auditRetentionDays: Int = 365,
    @SerialName("chat_retention_days") val chatRetentionDays: Int = 90,
    @SerialName("max_live_drivers") val maxLiveDrivers: Int = 10,
    @SerialName("session_timeout_mins") val sessionTimeoutMins: Int = 15,
    @SerialName("auto_lock_mins") val autoLockMins: Int? = null,
)

@Serializable
data class UpdateSystemSettingsReq(
    @SerialName("audit_retention_days") val auditRetentionDays: Int,
    @SerialName("chat_retention_days") val chatRetentionDays: Int,
    @SerialName("max_live_drivers") val maxLiveDrivers: Int,
    @SerialName("session_timeout_mins") val sessionTimeoutMins: Int,
)

@Serializable
data class DeliveryChannelStatsDto(
    val delivered: Long = 0,
    val pending: Long = 0,
    val retry: Long = 0,
    val failed: Long = 0,
    val gone: Long = 0,
)

@Serializable
data class AlertSettingsDto(
    @SerialName("telegram_bot_configured") val telegramBotConfigured: Boolean = false,
    @SerialName("telegram_bot_username") val telegramBotUsername: String = "",
    @SerialName("telegram_webhook_path") val telegramWebhookPath: String = "",
    @SerialName("telegram_webhook_header_path") val telegramWebhookHeaderPath: String = "",
    @SerialName("telegram_webhook_secret") val telegramWebhookSecret: String = "",
    @SerialName("alert_webhook_configured") val alertWebhookConfigured: Boolean = false,
    @SerialName("alert_webhook_url") val alertWebhookUrl: String = "",
    @SerialName("alert_webhook_secret_configured") val alertWebhookSecretConfigured: Boolean = false,
    @SerialName("mobile_push_webhook_configured") val mobilePushWebhookConfigured: Boolean = false,
    @SerialName("mobile_push_webhook_url") val mobilePushWebhookUrl: String = "",
    @SerialName("mobile_push_subscriptions_count") val mobilePushSubscriptionsCount: Long = 0,
    @SerialName("vapid_public_key") val vapidPublicKey: String = "",
    @SerialName("vapid_private_configured") val vapidPrivateConfigured: Boolean = false,
    @SerialName("push_subscriptions_count") val pushSubscriptionsCount: Long = 0,
    @SerialName("delivery_stats_24h") val deliveryStats24h: Map<String, DeliveryChannelStatsDto> = emptyMap(),
)

@Serializable
data class UpdateAlertSettingsReq(
    @SerialName("telegram_bot_token") val telegramBotToken: String? = null,
    @SerialName("clear_telegram_bot_token") val clearTelegramBotToken: Boolean? = null,
    @SerialName("telegram_bot_username") val telegramBotUsername: String? = null,
    @SerialName("alert_webhook_url") val alertWebhookUrl: String? = null,
    @SerialName("clear_alert_webhook_url") val clearAlertWebhookUrl: Boolean? = null,
    @SerialName("alert_webhook_secret") val alertWebhookSecret: String? = null,
    @SerialName("clear_alert_webhook_secret") val clearAlertWebhookSecret: Boolean? = null,
    @SerialName("mobile_push_webhook_url") val mobilePushWebhookUrl: String? = null,
    @SerialName("clear_mobile_push_webhook_url") val clearMobilePushWebhookUrl: Boolean? = null,
    @SerialName("vapid_public_key") val vapidPublicKey: String? = null,
    @SerialName("vapid_private_key") val vapidPrivateKey: String? = null,
    @SerialName("rotate_vapid_keys") val rotateVapidKeys: Boolean? = null,
)

@Serializable
data class TestWebhookRespDto(
    val ok: Boolean = false,
    @SerialName("status_code") val statusCode: Int = 0,
    @SerialName("latency_ms") val latencyMs: Long = 0,
    val error: String? = null,
)

/**
 * Group create/update payload. Uses [GroupUpsertReqSerializer] so clearing a parent
 * explicitly serializes `"parent_id": null` despite `explicitNulls = false` on [ApiClient].
 */
@Serializable(with = GroupUpsertReqSerializer::class)
data class GroupUpsertReq(
    val name: String,
    val description: String = "",
    @SerialName("parent_id") val parentId: JsonElement = JsonNull,
)

object GroupUpsertReqSerializer : KSerializer<GroupUpsertReq> {
    override val descriptor: SerialDescriptor = buildClassSerialDescriptor("GroupUpsertReq") {
        element<String>("name")
        element<String>("description")
        element("parent_id", JsonElement.serializer().descriptor)
    }

    override fun serialize(encoder: Encoder, value: GroupUpsertReq) {
        val jsonEncoder = encoder as? JsonEncoder
        if (jsonEncoder != null) {
            jsonEncoder.encodeJsonElement(
                buildJsonObject {
                    put("name", JsonPrimitive(value.name))
                    put("description", JsonPrimitive(value.description))
                    put("parent_id", value.parentId)
                },
            )
            return
        }
        val composite = encoder.beginStructure(descriptor)
        composite.encodeStringElement(descriptor, 0, value.name)
        composite.encodeStringElement(descriptor, 1, value.description)
        composite.encodeSerializableElement(descriptor, 2, JsonElement.serializer(), value.parentId)
        composite.endStructure(descriptor)
    }

    override fun deserialize(decoder: Decoder): GroupUpsertReq {
        val jsonDecoder = decoder as? JsonDecoder
        if (jsonDecoder != null) {
            val obj = jsonDecoder.decodeJsonElement() as? JsonObject ?: buildJsonObject {}
            return GroupUpsertReq(
                name = obj["name"]?.jsonPrimitive?.content.orEmpty(),
                description = obj["description"]?.jsonPrimitive?.content.orEmpty(),
                parentId = obj["parent_id"] ?: JsonNull,
            )
        }
        var name = ""
        var description = ""
        var parentId: JsonElement = JsonNull
        val composite = decoder.beginStructure(descriptor)
        while (true) {
            when (composite.decodeElementIndex(descriptor)) {
                0 -> name = composite.decodeStringElement(descriptor, 0)
                1 -> description = composite.decodeStringElement(descriptor, 1)
                2 -> parentId = composite.decodeSerializableElement(descriptor, 2, JsonElement.serializer())
                CompositeDecoder.DECODE_DONE -> break
            }
        }
        composite.endStructure(descriptor)
        return GroupUpsertReq(name = name, description = description, parentId = parentId)
    }
}

@Serializable
data class GroupMembersReq(
    @SerialName("user_ids") val userIds: List<Long>,
)
