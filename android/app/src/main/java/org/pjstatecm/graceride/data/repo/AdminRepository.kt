package org.pjstatecm.graceride.data.repo

import org.pjstatecm.graceride.data.api.AdminDashboardStatsDto
import org.pjstatecm.graceride.data.api.AdminInviteDto
import org.pjstatecm.graceride.data.api.AdminRolesRespDto
import org.pjstatecm.graceride.data.api.AdminUsersPageDto
import org.pjstatecm.graceride.data.api.ApiProvider
import org.pjstatecm.graceride.data.api.ApproveRecoveryResDto
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.CreateInviteResDto
import org.pjstatecm.graceride.data.api.GroupDto
import org.pjstatecm.graceride.data.api.RecoveryRequestDto
import org.pjstatecm.graceride.data.api.UpdateAdminUserReq
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
}
