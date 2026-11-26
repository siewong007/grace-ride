package org.pjstatecm.graceride.ui.screens.admin

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Call
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.WarningAmber
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.AdminRoleDefDto
import org.pjstatecm.graceride.data.api.AdminUserDto
import org.pjstatecm.graceride.data.api.GroupDto
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.domain.GroupTree
import org.pjstatecm.graceride.domain.MemberAdminRules
import org.pjstatecm.graceride.domain.RoleLabels
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.MemberAvatar
import org.pjstatecm.graceride.ui.copyToClipboard
import org.pjstatecm.graceride.ui.dialPhone
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors

@Composable
internal fun roleDisplayName(key: String, rolesCatalog: List<AdminRoleDefDto>): String =
    RoleLabels.builtInLabelRes(key)?.let { stringResource(it) }
        ?: rolesCatalog.firstOrNull { it.key == key }?.name?.takeIf { it.isNotBlank() }
        ?: key

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
internal fun MemberEditSheet(
    user: AdminUserDto,
    isApproveMode: Boolean,
    me: Me?,
    rolesCatalog: List<AdminRoleDefDto>,
    groups: List<GroupDto>,
    groupsLoaded: Boolean,
    photoUrlFor: (Long) -> String?,
    busy: Boolean,
    error: String?,
    onSave: (roles: List<String>, status: String, groupIds: List<Long>?, presidentAcknowledged: Boolean) -> Unit,
    onCopiedEmail: () -> Unit,
    onDismiss: () -> Unit,
) {
    val context = LocalContext.current
    val view = LocalView.current
    val canDismiss by rememberUpdatedState(!busy)
    val sheetState = rememberModalBottomSheetState(
        skipPartiallyExpanded = true,
        confirmValueChange = { canDismiss },
    )
    val canAssignRoles = me?.can("admin_roles", "update") == true

    var selectedRoles by remember(user.id, isApproveMode) {
        mutableStateOf(if (isApproveMode) emptyList() else user.roles)
    }
    var status by remember(user.id, isApproveMode) {
        mutableStateOf(if (isApproveMode) "active" else user.status.takeIf { it in setOf("active", "disabled") } ?: "active")
    }
    var groupsTouched by remember(user.id, isApproveMode) { mutableStateOf(false) }
    var selectedGroupIds by remember(user.id, isApproveMode) {
        mutableStateOf(
            if (isApproveMode) emptySet()
            else MemberAdminRules.parseInitialGroupIds(user.groups, groups),
        )
    }
    LaunchedEffect(user.id, isApproveMode, groupsLoaded, groups) {
        if (!isApproveMode && groupsLoaded && !groupsTouched) {
            selectedGroupIds = MemberAdminRules.parseInitialGroupIds(user.groups, groups)
        }
    }

    var showPresidentAckDialog by remember { mutableStateOf(false) }
    val orderedGroups = remember(groups) {
        GroupTree.treeOrder(GroupTree.assignableForMembers(groups))
    }
    val roleOptions = remember(rolesCatalog, canAssignRoles, selectedRoles) {
        if (!canAssignRoles) {
            selectedRoles
        } else {
            val catalogKeys = if (rolesCatalog.isNotEmpty()) {
                rolesCatalog.filter { it.assignable && !it.implicit }.map { it.key }
            } else {
                RoleLabels.BUILT_IN_ASSIGNABLE
            }
            (catalogKeys + selectedRoles).distinct()
        }
    }

    ModalBottomSheet(
        onDismissRequest = { if (!busy) onDismiss() },
        sheetState = sheetState,
        containerColor = MaterialTheme.colorScheme.surface,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .navigationBarsPadding()
                .imePadding()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
        ) {
            Text(
                text = stringResource(
                    if (isApproveMode) R.string.members_mode_approve else R.string.members_mode_edit,
                    user.name,
                ),
                style = MaterialTheme.typography.titleLarge,
                color = MaterialTheme.colorScheme.onSurface,
            )

            // Member identity summary
            Row(
                Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(Spacing.sm),
            ) {
                MemberAvatar(
                    name = user.name,
                    photoUrl = user.photoFileId?.let(photoUrlFor),
                    avatarColor = user.avatarColor,
                    avatarIcon = user.avatarIcon,
                    size = 48.dp,
                )
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Text(
                        user.name,
                        style = MaterialTheme.typography.titleMedium,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    if (user.email.isNotBlank()) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier.clickable {
                                if (copyToClipboard(context, user.name, user.email)) {
                                    onCopiedEmail()
                                }
                            },
                        ) {
                            Text(
                                user.email,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                                modifier = Modifier.weight(1f, fill = false),
                            )
                            Spacer(Modifier.width(4.dp))
                            Icon(
                                Icons.Outlined.ContentCopy,
                                contentDescription = stringResource(R.string.common_copy),
                                tint = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.size(14.dp),
                            )
                        }
                    }
                    if (!user.familyName.isNullOrBlank()) {
                        Text(
                            stringResource(R.string.members_family_label, user.familyName),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
                if (user.phone.isNotBlank()) {
                    IconButton(onClick = { dialPhone(context, user.phone) }) {
                        Icon(
                            Icons.Outlined.Call,
                            contentDescription = stringResource(R.string.common_call),
                            tint = MaterialTheme.colorScheme.primary,
                        )
                    }
                }
            }

            if (!isApproveMode) {
                Column(verticalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
                    Text(
                        stringResource(R.string.members_status_label),
                        style = MaterialTheme.typography.titleSmall,
                    )
                    SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
                        SegmentedButton(
                            selected = status == "active",
                            onClick = {
                                UiHaptics.tick(view)
                                status = "active"
                            },
                            enabled = !busy,
                            shape = SegmentedButtonDefaults.itemShape(0, 2),
                        ) {
                            Text(stringResource(R.string.members_status_active))
                        }
                        SegmentedButton(
                            selected = status == "disabled",
                            onClick = {
                                UiHaptics.tick(view)
                                status = "disabled"
                            },
                            enabled = !busy,
                            shape = SegmentedButtonDefaults.itemShape(1, 2),
                        ) {
                            Text(stringResource(R.string.members_status_disabled))
                        }
                    }
                    if (status == "disabled" && user.status != "disabled") {
                        val c = LocalStatusColors.current
                        CalloutBanner(
                            icon = Icons.Outlined.WarningAmber,
                            text = stringResource(R.string.members_disable_warning),
                            containerColor = c.warningSoft,
                            contentColor = c.warning,
                        )
                    }
                }
            }

            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)

            // Roles section
            Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                Text(
                    stringResource(R.string.members_role_label),
                    style = MaterialTheme.typography.titleSmall,
                )
                if (roleOptions.isEmpty()) {
                    Text(
                        stringResource(R.string.members_member_default),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                    ) {
                        roleOptions.forEach { roleKey ->
                            val selected = roleKey in selectedRoles
                            FilterChip(
                                selected = selected,
                                enabled = canAssignRoles && !busy,
                                onClick = {
                                    UiHaptics.tick(view)
                                    selectedRoles = if (selected) {
                                        selectedRoles - roleKey
                                    } else {
                                        selectedRoles + roleKey
                                    }
                                },
                                leadingIcon = if (selected) {
                                    { Icon(Icons.Outlined.Check, contentDescription = null, modifier = Modifier.size(16.dp)) }
                                } else {
                                    null
                                },
                                label = { Text(roleDisplayName(roleKey, rolesCatalog)) },
                            )
                        }
                    }
                }
                if (!canAssignRoles) {
                    Text(
                        stringResource(R.string.members_roles_read_only),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            // Groups section
            if (orderedGroups.isNotEmpty()) {
                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    Text(
                        stringResource(R.string.members_group_label),
                        style = MaterialTheme.typography.titleSmall,
                    )
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                    ) {
                        orderedGroups.forEach { (g, depth) ->
                            val selected = g.id in selectedGroupIds
                            val prefix = if (depth > 0) "↳ ".repeat(depth) else ""
                            FilterChip(
                                selected = selected,
                                enabled = !busy,
                                onClick = {
                                    UiHaptics.tick(view)
                                    groupsTouched = true
                                    selectedGroupIds = if (selected) {
                                        selectedGroupIds - g.id
                                    } else {
                                        selectedGroupIds + g.id
                                    }
                                },
                                leadingIcon = if (selected) {
                                    { Icon(Icons.Outlined.Check, contentDescription = null, modifier = Modifier.size(16.dp)) }
                                } else {
                                    null
                                },
                                label = { Text("$prefix${g.name}") },
                            )
                        }
                    }
                }
            }

            if (!error.isNullOrBlank()) {
                Text(
                    error,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                )
            }

            Row(
                Modifier.fillMaxWidth().padding(top = Spacing.xs),
                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                OutlinedButton(
                    onClick = onDismiss,
                    enabled = !busy,
                    modifier = Modifier.weight(1f),
                ) {
                    Text(stringResource(R.string.common_cancel))
                }
                Button(
                    onClick = {
                        val needsAck = canAssignRoles &&
                            MemberAdminRules.viceNeedsPresidentAck(me) &&
                            MemberAdminRules.promotingToAdmin(user.roles, selectedRoles, rolesCatalog)
                        if (needsAck) {
                            showPresidentAckDialog = true
                        } else {
                            UiHaptics.confirm(view)
                            onSave(
                                selectedRoles,
                                status,
                                if (groupsLoaded) selectedGroupIds.toList() else null,
                                false,
                            )
                        }
                    },
                    enabled = !busy,
                    colors = graceRidePrimaryButtonColors(),
                    modifier = Modifier.weight(1f),
                ) {
                    if (busy) {
                        InlineButtonSpinner()
                        Spacer(Modifier.width(Spacing.xs))
                    }
                    Text(
                        stringResource(
                            if (isApproveMode) R.string.members_approve_btn else R.string.common_save,
                        ),
                    )
                }
            }
            Spacer(Modifier.height(Spacing.xs))
        }
    }

    if (showPresidentAckDialog) {
        AlertDialog(
            onDismissRequest = { showPresidentAckDialog = false },
            title = { Text(stringResource(R.string.members_president_ack_title)) },
            text = { Text(stringResource(R.string.members_president_ack_body, user.name)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        showPresidentAckDialog = false
                        UiHaptics.confirm(view)
                        onSave(
                            selectedRoles,
                            status,
                            if (groupsLoaded) selectedGroupIds.toList() else null,
                            true,
                        )
                    },
                ) {
                    Text(stringResource(R.string.members_president_ack_confirm))
                }
            },
            dismissButton = {
                TextButton(onClick = { showPresidentAckDialog = false }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
internal fun BatchApproveMembersSheet(
    selectedUsers: List<AdminUserDto>,
    me: Me?,
    rolesCatalog: List<AdminRoleDefDto>,
    groups: List<GroupDto>,
    groupsLoaded: Boolean,
    busy: Boolean,
    error: String?,
    onConfirm: (roleKey: String, groupIds: List<Long>?, groupsTouched: Boolean, presidentAcknowledged: Boolean) -> Unit,
    onDismiss: () -> Unit,
) {
    val view = LocalView.current
    val canDismiss by rememberUpdatedState(!busy)
    val sheetState = rememberModalBottomSheetState(
        skipPartiallyExpanded = true,
        confirmValueChange = { canDismiss },
    )
    val canAssignRoles = me?.can("admin_roles", "update") == true

    var selectedRole by remember { mutableStateOf("member") }
    var selectedGroupIds by remember { mutableStateOf(emptySet<Long>()) }
    var groupsTouched by remember { mutableStateOf(false) }
    var showPresidentAckDialog by remember { mutableStateOf(false) }

    val orderedGroups = remember(groups) {
        GroupTree.treeOrder(GroupTree.assignableForMembers(groups))
    }
    val assignableRoleKeys = remember(rolesCatalog) {
        if (rolesCatalog.isNotEmpty()) {
            rolesCatalog.filter { it.assignable && !it.implicit }.map { it.key }
        } else {
            RoleLabels.BUILT_IN_ASSIGNABLE
        }
    }

    ModalBottomSheet(
        onDismissRequest = { if (!busy) onDismiss() },
        sheetState = sheetState,
        containerColor = MaterialTheme.colorScheme.surface,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .navigationBarsPadding()
                .imePadding()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
        ) {
            Text(
                stringResource(R.string.members_batch_title),
                style = MaterialTheme.typography.titleLarge,
            )
            Text(
                stringResource(R.string.members_batch_desc, selectedUsers.size),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            Surface(
                color = MaterialTheme.colorScheme.surfaceVariant,
                shape = MaterialTheme.shapes.medium,
                modifier = Modifier.fillMaxWidth(),
            ) {
                FlowRow(
                    modifier = Modifier.padding(Spacing.xs),
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    verticalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    selectedUsers.forEach { u ->
                        Surface(
                            color = MaterialTheme.colorScheme.surface,
                            shape = MaterialTheme.shapes.small,
                        ) {
                            Text(
                                u.name,
                                style = MaterialTheme.typography.labelMedium,
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                            )
                        }
                    }
                }
            }

            if (canAssignRoles) {
                Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    Text(
                        stringResource(R.string.members_batch_assign_role),
                        style = MaterialTheme.typography.titleSmall,
                    )
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                    ) {
                        FilterChip(
                            selected = selectedRole == "member",
                            enabled = !busy,
                            onClick = {
                                UiHaptics.tick(view)
                                selectedRole = "member"
                            },
                            label = { Text(stringResource(R.string.members_member_default)) },
                        )
                        assignableRoleKeys.forEach { roleKey ->
                            FilterChip(
                                selected = selectedRole == roleKey,
                                enabled = !busy,
                                onClick = {
                                    UiHaptics.tick(view)
                                    selectedRole = roleKey
                                },
                                label = { Text(roleDisplayName(roleKey, rolesCatalog)) },
                            )
                        }
                    }
                }
            }

            if (orderedGroups.isNotEmpty()) {
                Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    Text(
                        stringResource(R.string.members_batch_assign_groups),
                        style = MaterialTheme.typography.titleSmall,
                    )
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                    ) {
                        orderedGroups.forEach { (g, depth) ->
                            val selected = g.id in selectedGroupIds
                            val prefix = if (depth > 0) "↳ ".repeat(depth) else ""
                            FilterChip(
                                selected = selected,
                                enabled = !busy,
                                onClick = {
                                    UiHaptics.tick(view)
                                    groupsTouched = true
                                    selectedGroupIds = if (selected) {
                                        selectedGroupIds - g.id
                                    } else {
                                        selectedGroupIds + g.id
                                    }
                                },
                                leadingIcon = if (selected) {
                                    { Icon(Icons.Outlined.Check, contentDescription = null, modifier = Modifier.size(16.dp)) }
                                } else {
                                    null
                                },
                                label = { Text("$prefix${g.name}") },
                            )
                        }
                    }
                }
            }

            if (!error.isNullOrBlank()) {
                Text(
                    error,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                )
            }

            Row(
                Modifier.fillMaxWidth().padding(top = Spacing.xs),
                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                OutlinedButton(
                    onClick = onDismiss,
                    enabled = !busy,
                    modifier = Modifier.weight(1f),
                ) {
                    Text(stringResource(R.string.common_cancel))
                }
                Button(
                    onClick = {
                        val payloadRoles = if (selectedRole == "member") emptyList() else listOf(selectedRole)
                        val needsAck = canAssignRoles &&
                            MemberAdminRules.viceNeedsPresidentAck(me) &&
                            selectedUsers.any { MemberAdminRules.promotingToAdmin(it.roles, payloadRoles, rolesCatalog) }
                        if (needsAck) {
                            showPresidentAckDialog = true
                        } else {
                            UiHaptics.confirm(view)
                            onConfirm(
                                selectedRole,
                                if (groupsLoaded) selectedGroupIds.toList() else null,
                                groupsTouched,
                                false,
                            )
                        }
                    },
                    enabled = !busy,
                    colors = graceRidePrimaryButtonColors(),
                    modifier = Modifier.weight(1f),
                ) {
                    if (busy) {
                        InlineButtonSpinner()
                        Spacer(Modifier.width(Spacing.xs))
                    }
                    Text(stringResource(R.string.members_batch_confirm, selectedUsers.size))
                }
            }
            Spacer(Modifier.height(Spacing.xs))
        }
    }

    if (showPresidentAckDialog) {
        val names = remember(selectedUsers) { selectedUsers.joinToString(", ") { it.name } }
        AlertDialog(
            onDismissRequest = { showPresidentAckDialog = false },
            title = { Text(stringResource(R.string.members_president_ack_title)) },
            text = { Text(stringResource(R.string.members_president_ack_body, names)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        showPresidentAckDialog = false
                        UiHaptics.confirm(view)
                        onConfirm(
                            selectedRole,
                            if (groupsLoaded) selectedGroupIds.toList() else null,
                            groupsTouched,
                            true,
                        )
                    },
                ) {
                    Text(stringResource(R.string.members_president_ack_confirm))
                }
            },
            dismissButton = {
                TextButton(onClick = { showPresidentAckDialog = false }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }
}
