package org.pjstatecm.graceride.domain

import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.AdminRoleDefDto
import org.pjstatecm.graceride.data.api.AdminUserDto
import org.pjstatecm.graceride.data.api.GroupDto
import org.pjstatecm.graceride.data.api.Me

/** Group hierarchy & audience helpers mirroring frontend/src/groupTree.ts. */
object GroupTree {

    data class OrderedGroup(val group: GroupDto, val depth: Int)

    /** Ordinary groups (excludes the built-in visitor group). */
    fun ordinary(groups: List<GroupDto>): List<GroupDto> = groups.filter { !it.isVisitor }

    /** Groups members can be assigned to directly (excludes All Members and Visitors). */
    fun assignableForMembers(groups: List<GroupDto>): List<GroupDto> =
        groups.filter { !it.isAll && !it.isVisitor }

    /**
     * Groups in DFS tree order (each parent followed by its sub-groups) with cycle protection.
     * Groups whose parent is missing from [groups] are promoted to root (`depth = 0`).
     */
    fun treeOrder(groups: List<GroupDto>): List<OrderedGroup> {
        val ids = groups.map { it.id }.toHashSet()
        val kids = LinkedHashMap<Long?, MutableList<GroupDto>>()
        for (g in groups) {
            val parent = g.parentId?.takeIf { it in ids }
            kids.getOrPut(parent) { mutableListOf() }.add(g)
        }
        val out = ArrayList<OrderedGroup>(groups.size)
        val seen = HashSet<Long>(groups.size)

        fun walk(parent: Long?, depth: Int) {
            val children = kids[parent] ?: return
            for (g in children) {
                if (!seen.add(g.id)) continue
                out.add(OrderedGroup(g, depth))
                walk(g.id, depth + 1)
            }
        }
        walk(null, 0)
        // Guard against disconnected cycles so no group is ever silently dropped.
        for (g in groups) {
            if (seen.add(g.id)) {
                out.add(OrderedGroup(g, 0))
                walk(g.id, 1)
            }
        }
        return out
    }

    /** All group ids including descendants of [rootId] within [groups]. */
    fun withDescendants(groups: List<GroupDto>, rootId: Long): Set<Long> {
        val result = linkedSetOf(rootId)
        val byParent = groups.groupBy { it.parentId }
        val queue = ArrayDeque<Long>()
        queue.add(rootId)
        while (queue.isNotEmpty()) {
            val cur = queue.removeFirst()
            byParent[cur].orEmpty().forEach { child ->
                if (result.add(child.id)) queue.add(child.id)
            }
        }
        return result
    }
}

/** Shared built-in role display labels matching backend `perms::SYSTEM_ROLES`. */
object RoleLabels {

    val BUILT_IN_ASSIGNABLE: List<String> = listOf(
        "admin",
        "president",
        "vice_president",
        "secretary",
        "traffic_head",
        "traffic_member",
        "cell_leader",
        "transport",
        "finance",
        "auditor",
        "leader",
        "driver",
    )

    fun builtInLabelRes(key: String): Int? = when (key) {
        "member" -> R.string.role_member
        "superadmin" -> R.string.role_superadmin
        "admin" -> R.string.role_admin
        "president" -> R.string.role_president
        "vice_president" -> R.string.role_vice_president
        "secretary" -> R.string.role_secretary
        "traffic_head" -> R.string.role_traffic_head
        "traffic_member" -> R.string.role_traffic_member
        "cell_leader" -> R.string.role_cell_leader
        "transport" -> R.string.role_transport
        "finance" -> R.string.role_finance
        "auditor" -> R.string.role_auditor
        "leader" -> R.string.role_leader
        "driver" -> R.string.role_driver
        else -> null
    }
}

/** Pure authorization & payload guards mirroring frontend Members.tsx and Announcements.tsx. */
object MemberAdminRules {

    /** Superadmin accounts can only be edited by another superadmin (backend enforces this). */
    fun isEditable(me: Me?, target: AdminUserDto): Boolean =
        me != null &&
            me.can("admin_members", "update") &&
            (me.roles.contains("superadmin") || !target.roles.contains("superadmin"))

    /** Vice-Presidents promoting a member to administrator must acknowledge President approval. */
    fun viceNeedsPresidentAck(me: Me?): Boolean =
        me != null &&
            me.roles.contains("vice_president") &&
            !me.roles.contains("president") &&
            !me.roles.contains("superadmin")

    fun isAdminPowerRole(roleKey: String, rolesCatalog: List<AdminRoleDefDto>): Boolean =
        roleKey == "admin" ||
            rolesCatalog.firstOrNull { it.key == roleKey }?.permissions?.contains("admin_roles.update") == true

    fun promotingToAdmin(
        oldRoles: List<String>,
        newRoles: List<String>,
        rolesCatalog: List<AdminRoleDefDto>,
    ): Boolean = !oldRoles.any { isAdminPowerRole(it, rolesCatalog) } &&
        newRoles.any { isAdminPowerRole(it, rolesCatalog) }

    /**
     * Parses `u.groups` (server joins group names with `'、'`) back into assignable group IDs.
     */
    fun parseInitialGroupIds(userGroupsText: String, groups: List<GroupDto>): Set<Long> {
        if (userGroupsText.isBlank()) return emptySet()
        val names = userGroupsText.split('、', ',')
            .map { it.trim() }
            .filter { it.isNotEmpty() }
            .toSet()
        return GroupTree.assignableForMembers(groups)
            .filter { it.name in names }
            .map { it.id }
            .toSet()
    }

    /**
     * Returns true when the selected announcement audience differs from the saved audience.
     * Keep `group_ids` omitted (`null`) in `PUT /api/announcements/{id}` when false so targeted
     * announcements whose groups were deleted are never silently opened to everyone.
     */
    fun announcementAudienceChanged(originalIds: List<Long>, currentIds: List<Long>): Boolean =
        originalIds.toSet() != currentIds.toSet()
}
