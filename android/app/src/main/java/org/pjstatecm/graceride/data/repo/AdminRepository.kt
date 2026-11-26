package org.pjstatecm.graceride.data.repo

import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonPrimitive
import org.pjstatecm.graceride.data.api.AdminDashboardStatsDto
import org.pjstatecm.graceride.data.api.AdminInviteDto
import org.pjstatecm.graceride.data.api.AdminRolesRespDto
import org.pjstatecm.graceride.data.api.AdminUsersPageDto
import org.pjstatecm.graceride.data.api.AlertSettingsDto
import org.pjstatecm.graceride.data.api.ApiProvider
import org.pjstatecm.graceride.data.api.ApproveRecoveryResDto
import org.pjstatecm.graceride.data.api.AuditPageDto
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.CreateInviteResDto
import org.pjstatecm.graceride.data.api.GroupDto
import org.pjstatecm.graceride.data.api.GroupMembersReq
import org.pjstatecm.graceride.data.api.GroupUpsertReq
import org.pjstatecm.graceride.data.api.RecoveryRequestDto
import org.pjstatecm.graceride.data.api.RoleSaveReq
import org.pjstatecm.graceride.data.api.SystemSettingsDto
import org.pjstatecm.graceride.data.api.TestWebhookRespDto
import org.pjstatecm.graceride.data.api.UpdateAdminUserReq
import org.pjstatecm.graceride.data.api.UpdateAlertSettingsReq
import org.pjstatecm.graceride.data.api.UpdateSystemSettingsReq
import org.pjstatecm.graceride.data.api.UserRefDto
import org.pjstatecm.graceride.data.api.apiCall
import org.pjstatecm.graceride.data.api.runSuspendCatching

/** Repository for Admin member directory, roles, groups, invite links, and recovery approvals. */
class AdminRepository(
    private val apiProvider: ApiProvider,
    private val bus: AuthEventBus,
) {
    private val api get() = apiProvider.get()

    suspend fun dashboardStats(passive: Boolean = false): Result<AdminDashboardStatsDto> =
        runSuspendCatching { apiCall(bus) { api.adminDashboard(if (passive) "1" else null) } }

    suspend fun users(
        q: String? = null,
        role: String? = null,
        status: String? = null,
        sort: String? = null,
        dir: String? = null,
        page: Int = 1,
        limit: Int = 50,
    ): Result<AdminUsersPageDto> = runSuspendCatching {
        apiCall(bus) {
            api.adminUsers(
                q = q?.trim()?.ifBlank { null },
                role = role?.trim()?.ifBlank { null },
                status = status?.trim()?.ifBlank { null },
                sort = sort?.trim()?.ifBlank { null },
                dir = dir?.trim()?.ifBlank { null },
                page = page,
                limit = limit,
            )
        }
    }

    suspend fun updateUser(id: Long, req: UpdateAdminUserReq): Result<Unit> =
        runSuspendCatching { apiCall(bus) { api.updateAdminUser(id, req) }; Unit }

    suspend fun roles(): Result<AdminRolesRespDto> =
        runSuspendCatching { apiCall(bus) { api.adminRoles() } }

    suspend fun groups(): Result<List<GroupDto>> =
        runSuspendCatching { apiCall(bus) { api.groups() } }

    suspend fun invites(): Result<List<AdminInviteDto>> =
        runSuspendCatching { apiCall(bus) { api.adminInvites() } }

    suspend fun createInvite(): Result<CreateInviteResDto> =
        runSuspendCatching { apiCall(bus) { api.createAdminInvite() } }

    suspend fun revokeInvite(id: Long): Result<Unit> =
        runSuspendCatching { apiCall(bus) { api.revokeAdminInvite(id) }; Unit }

    suspend fun recoveryRequests(): Result<List<RecoveryRequestDto>> =
        runSuspendCatching { apiCall(bus) { api.adminRecovery() } }

    suspend fun approveRecovery(id: Long): Result<ApproveRecoveryResDto> =
        runSuspendCatching { apiCall(bus) { api.approveRecovery(id) } }

    suspend fun rejectRecovery(id: Long): Result<Unit> =
        runSuspendCatching { apiCall(bus) { api.rejectRecovery(id) }; Unit }

    suspend fun createRole(name: String, description: String, permissions: List<String>): Result<String> =
        runSuspendCatching {
            apiCall(bus) {
                api.createAdminRole(RoleSaveReq(name = name, description = description, permissions = permissions)).key
            }
        }

    suspend fun updateRole(key: String, name: String, description: String, permissions: List<String>): Result<Unit> =
        runSuspendCatching {
            apiCall(bus) {
                api.updateAdminRole(key, RoleSaveReq(name = name, description = description, permissions = permissions))
            }
            Unit
        }

    suspend fun deleteRole(key: String): Result<Unit> =
        runSuspendCatching { apiCall(bus) { api.deleteAdminRole(key) }; Unit }

    suspend fun auditLog(
        q: String? = null,
        entity: String? = null,
        entityId: Long? = null,
        action: String? = null,
        actorId: Long? = null,
        from: String? = null,
        to: String? = null,
        before: Long? = null,
        limit: Int = 50,
    ): Result<AuditPageDto> = runSuspendCatching {
        apiCall(bus) {
            api.adminAudit(
                q = q?.ifBlank { null },
                entity = entity?.ifBlank { null },
                entityId = entityId,
                action = action?.ifBlank { null },
                actorId = actorId,
                from = from?.ifBlank { null },
                to = to?.ifBlank { null },
                before = before,
                limit = limit,
            )
        }
    }

    suspend fun exportAuditCsvBytes(
        q: String? = null,
        entity: String? = null,
        entityId: Long? = null,
        action: String? = null,
        actorId: Long? = null,
        from: String? = null,
        to: String? = null,
    ): Result<ByteArray> = runSuspendCatching {
        apiCall(bus) {
            api.exportAdminAuditCsv(
                q = q?.ifBlank { null },
                entity = entity?.ifBlank { null },
                entityId = entityId,
                action = action?.ifBlank { null },
                actorId = actorId,
                from = from?.ifBlank { null },
                to = to?.ifBlank { null },
            ).bytes()
        }
    }

    suspend fun systemSettings(): Result<SystemSettingsDto> =
        runSuspendCatching { apiCall(bus) { api.systemSettings() } }

    suspend fun updateSystemSettings(req: UpdateSystemSettingsReq): Result<Unit> =
        runSuspendCatching { apiCall(bus) { api.updateSystemSettings(req) }; Unit }

    suspend fun alertSettings(): Result<AlertSettingsDto> =
        runSuspendCatching { apiCall(bus) { api.alertSettings() } }

    suspend fun updateAlertSettings(req: UpdateAlertSettingsReq): Result<Unit> =
        runSuspendCatching { apiCall(bus) { api.updateAlertSettings(req) }; Unit }

    suspend fun testAlertWebhook(): Result<TestWebhookRespDto> =
        runSuspendCatching { apiCall(bus) { api.testAlertWebhook() } }

    suspend fun createGroup(name: String, description: String, parentId: Long?): Result<Long> =
        runSuspendCatching {
            val parentJson = parentId?.let { JsonPrimitive(it) } ?: JsonNull
            apiCall(bus) { api.createGroup(GroupUpsertReq(name = name, description = description, parentId = parentJson)).id }
        }

    suspend fun updateGroup(id: Long, name: String, description: String, parentId: Long?): Result<Unit> =
        runSuspendCatching {
            val parentJson = parentId?.let { JsonPrimitive(it) } ?: JsonNull
            apiCall(bus) { api.updateGroup(id, GroupUpsertReq(name = name, description = description, parentId = parentJson)) }
            Unit
        }

    suspend fun deleteGroup(id: Long): Result<Unit> =
        runSuspendCatching { apiCall(bus) { api.deleteGroup(id) }; Unit }

    suspend fun groupMembers(id: Long): Result<List<UserRefDto>> =
        runSuspendCatching { apiCall(bus) { api.groupMembers(id) } }

    suspend fun setGroupMembers(id: Long, userIds: List<Long>): Result<Unit> =
        runSuspendCatching { apiCall(bus) { api.setGroupMembers(id, GroupMembersReq(userIds)) }; Unit }
}
