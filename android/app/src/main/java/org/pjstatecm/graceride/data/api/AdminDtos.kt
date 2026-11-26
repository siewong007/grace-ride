package org.pjstatecm.graceride.data.api

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

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
data class AdminRolesRespDto(
    val roles: List<AdminRoleDefDto> = emptyList(),
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
