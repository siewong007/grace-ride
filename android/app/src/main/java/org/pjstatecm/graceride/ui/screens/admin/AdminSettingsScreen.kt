package org.pjstatecm.graceride.ui.screens.admin

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.DeleteOutline
import androidx.compose.material.icons.outlined.Edit
import androidx.compose.material.icons.outlined.PeopleOutline
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material.icons.outlined.Settings
import androidx.compose.material.icons.outlined.Visibility
import androidx.compose.material.icons.outlined.VisibilityOff
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.GroupDto
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.SecureScreenEffect
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.copyToClipboard
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun AdminSettingsScreen(
    container: AppContainer,
    nav: NavHostController,
    initialTab: String? = null,
) {
    SecureScreenEffect()
    val vm: AdminSettingsViewModel = viewModel(
        key = "admin-settings-${initialTab ?: "system"}",
        factory = AdminSettingsViewModel.factory(container, initialTab),
    )
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val me = (authState as? AuthState.SignedIn)?.me ?: container.auth.me
    val context = LocalContext.current
    val view = LocalView.current
    val scope = rememberCoroutineScope()
    val snackbar = remember { SnackbarHostState() }

    val canSettings = me?.can("admin_settings") == true
    val canUpdateSettings = me?.can("admin_settings", "update") == true
    val canGroups = me?.can("admin_groups") == true

    val systemSavedMsg = stringResource(R.string.settings_system_saved)
    val alertsSavedMsg = stringResource(R.string.settings_alerts_saved)
    val groupAddedMsg = stringResource(R.string.settings_added)
    val groupSavedMsg = stringResource(R.string.settings_saved)
    val groupDeletedMsg = stringResource(R.string.settings_deleted)
    val copiedMsg = stringResource(R.string.common_copied)

    LaunchedEffect(state.message, state.error) {
        val msg = state.message ?: state.error
        if (!msg.isNullOrBlank()) {
            snackbar.showSnackbar(msg)
            vm.clearMessage()
        }
    }

    val tabs = remember(canSettings, canGroups) {
        buildList {
            if (canSettings) {
                add("system" to R.string.settings_tab_rates)
                add("alerts" to R.string.settings_tab_alerts)
            }
            if (canGroups) {
                add("groups" to R.string.settings_tab_groups)
            }
        }
    }

    var editingGroup by remember { mutableStateOf<GroupDto?>(null) }
    var showCreateGroup by remember { mutableStateOf(false) }
    var deleteGroupTarget by remember { mutableStateOf<GroupDto?>(null) }
    var groupNameInput by remember { mutableStateOf("") }
    var groupDescInput by remember { mutableStateOf("") }
    var groupParentId by remember { mutableStateOf<Long?>(null) }
    var groupMembersSearch by remember { mutableStateOf("") }

    val auditVal = state.auditDaysText.trim().toIntOrNull()
    val chatVal = state.chatDaysText.trim().toIntOrNull()
    val driversVal = state.maxDriversText.trim().toIntOrNull()
    val timeoutVal = state.sessionTimeoutText.trim().toIntOrNull()
    val systemInvalid = auditVal !in 30..3650 || chatVal !in 7..3650 || driversVal !in 1..100 || timeoutVal !in 0..1440
    val isInitialLoading = state.loading && state.systemSettings == null && state.alertSettings == null && state.groups.isEmpty()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.settings_title), maxLines = 1, overflow = TextOverflow.Ellipsis) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
                actions = {
                    if (canGroups && state.tab == "groups" && me.can("admin_groups", "create")) {
                        IconButton(
                            onClick = {
                                UiHaptics.tick(view)
                                groupNameInput = ""
                                groupDescInput = ""
                                groupParentId = null
                                showCreateGroup = true
                            },
                        ) {
                            Icon(Icons.Outlined.Add, contentDescription = stringResource(R.string.settings_add_group))
                        }
                    }
                    IconButton(onClick = { UiHaptics.tick(view); vm.refresh() }) {
                        Icon(Icons.Outlined.Refresh, contentDescription = stringResource(R.string.common_refresh))
                    }
                },
            )
        },
        bottomBar = {
            if (tabs.isNotEmpty() && !isInitialLoading && canUpdateSettings && state.tab != "groups") {
                Surface(color = MaterialTheme.colorScheme.surface, modifier = Modifier.fillMaxWidth()) {
                    Column(Modifier.navigationBarsPadding().imePadding()) {
                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                        Box(Modifier.fillMaxWidth().padding(horizontal = Spacing.sm, vertical = Spacing.xs)) {
                            Button(
                                onClick = {
                                    UiHaptics.confirm(view)
                                    if (state.tab == "alerts") vm.saveAlertSettings(alertsSavedMsg)
                                    else vm.saveSystemSettings(systemSavedMsg)
                                },
                                enabled = !state.busy && (state.tab == "alerts" || !systemInvalid),
                                colors = graceRidePrimaryButtonColors(),
                                modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
                            ) {
                                if (state.busy) {
                                    InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                    Spacer(Modifier.width(Spacing.xs))
                                }
                                Text(stringResource(R.string.common_save))
                            }
                        }
                    }
                }
            }
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        if (tabs.isEmpty()) {
            Box(Modifier.fillMaxSize().padding(padding)) {
                EmptyView(text = stringResource(R.string.members_no_permission), icon = Icons.Outlined.Settings)
            }
            return@Scaffold
        }

        Column(Modifier.fillMaxSize().padding(padding)) {
            if (tabs.size > 1) {
                val selIdx = tabs.indexOfFirst { it.first == state.tab }.coerceAtLeast(0)
                PrimaryTabRow(selectedTabIndex = selIdx) {
                    tabs.forEach { (key, res) ->
                        Tab(
                            selected = state.tab == key,
                            onClick = { UiHaptics.tick(view); vm.setTab(key) },
                            text = { Text(stringResource(res), maxLines = 1, overflow = TextOverflow.Ellipsis) },
                        )
                    }
                }
            }

            if (isInitialLoading) {
                Box(Modifier.fillMaxSize()) { LoadingView() }
            } else {
                when (state.tab) {
                    "alerts" -> AlertsSettingsTab(
                        state = state,
                        canUpdate = canUpdateSettings,
                        vm = vm,
                        onCopy = { label, value ->
                            copyToClipboard(context, label, value)
                            scope.launch { snackbar.showSnackbar(copiedMsg) }
                        },
                    )
                    "groups" -> GroupsSettingsTab(
                        state = state,
                        canCreate = me?.can("admin_groups", "create") == true,
                        canUpdate = me?.can("admin_groups", "update") == true,
                        canDelete = me?.can("admin_groups", "delete") == true,
                        onOpenCreate = {
                            groupNameInput = ""
                            groupDescInput = ""
                            groupParentId = null
                            showCreateGroup = true
                        },
                        onEditGroup = { g ->
                            groupNameInput = g.name
                            groupDescInput = g.description
                            groupParentId = g.parentId
                            editingGroup = g
                        },
                        onManageMembers = { g ->
                            groupMembersSearch = ""
                            vm.openGroupMembers(g)
                        },
                        onDeleteGroup = { g -> deleteGroupTarget = g },
                    )
                    else -> SystemSettingsTab(
                        state = state,
                        canUpdate = canUpdateSettings,
                        me = me,
                        vm = vm,
                        nav = nav,
                    )
                }
            }
        }
    }

    if (showCreateGroup || editingGroup != null) {
        val current = editingGroup
        val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
        val parentChoices = remember(state.groups, current) {
            state.groups.filter { !it.isAll && !it.isVisitor && it.id != current?.id }
        }
        var parentExpanded by remember { mutableStateOf(false) }

        ModalBottomSheet(
            onDismissRequest = {
                showCreateGroup = false
                editingGroup = null
            },
            sheetState = sheetState,
        ) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .verticalScroll(rememberScrollState())
                    .navigationBarsPadding()
                    .imePadding()
                    .padding(horizontal = Spacing.sm)
                    .padding(bottom = Spacing.md),
                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                Text(
                    stringResource(if (current == null) R.string.settings_add_group else R.string.settings_edit_group),
                    style = MaterialTheme.typography.titleMedium,
                )
                OutlinedTextField(
                    value = groupNameInput,
                    onValueChange = { groupNameInput = it },
                    label = { Text(stringResource(R.string.settings_group_name)) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                OutlinedTextField(
                    value = groupDescInput,
                    onValueChange = { groupDescInput = it },
                    label = { Text(stringResource(R.string.roles_description)) },
                    maxLines = 2,
                    modifier = Modifier.fillMaxWidth(),
                )
                ExposedDropdownMenuBox(
                    expanded = parentExpanded,
                    onExpandedChange = { parentExpanded = !parentExpanded },
                ) {
                    OutlinedTextField(
                        value = state.groups.firstOrNull { it.id == groupParentId }?.name
                            ?: stringResource(R.string.settings_top_level),
                        onValueChange = {},
                        readOnly = true,
                        label = { Text(stringResource(R.string.settings_part_of)) },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(parentExpanded) },
                        modifier = Modifier.menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
                    )
                    ExposedDropdownMenu(expanded = parentExpanded, onDismissRequest = { parentExpanded = false }) {
                        DropdownMenuItem(
                            text = { Text(stringResource(R.string.settings_top_level)) },
                            onClick = {
                                groupParentId = null
                                parentExpanded = false
                            },
                        )
                        parentChoices.forEach { choice ->
                            DropdownMenuItem(
                                text = { Text(choice.name) },
                                onClick = {
                                    groupParentId = choice.id
                                    parentExpanded = false
                                },
                            )
                        }
                    }
                }
                Spacer(Modifier.height(Spacing.xxs))
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    OutlinedButton(
                        onClick = {
                            showCreateGroup = false
                            editingGroup = null
                        },
                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                    ) { Text(stringResource(R.string.common_cancel)) }
                    Button(
                        onClick = {
                            UiHaptics.confirm(view)
                            if (current == null) {
                                vm.createGroup(groupNameInput, groupDescInput, groupParentId, groupAddedMsg)
                            } else {
                                vm.updateGroup(current.id, groupNameInput, groupDescInput, groupParentId, groupSavedMsg)
                            }
                            showCreateGroup = false
                            editingGroup = null
                        },
                        enabled = groupNameInput.isNotBlank() && !state.busy,
                        colors = graceRidePrimaryButtonColors(),
                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                    ) { Text(stringResource(R.string.common_save)) }
                }
            }
        }
    }

    deleteGroupTarget?.let { target ->
        AlertDialog(
            onDismissRequest = { deleteGroupTarget = null },
            title = { Text(stringResource(R.string.common_delete)) },
            text = { Text(stringResource(R.string.settings_delete_group_confirm, target.name)) },
            confirmButton = {
                Button(
                    onClick = {
                        vm.deleteGroup(target.id, groupDeletedMsg)
                        deleteGroupTarget = null
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error),
                ) { Text(stringResource(R.string.common_delete)) }
            },
            dismissButton = {
                TextButton(onClick = { deleteGroupTarget = null }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }

    state.groupMembersTarget?.let { group ->
        val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
        val filteredUsers = remember(state.activeUsers, groupMembersSearch) {
            val q = groupMembersSearch.trim()
            if (q.isEmpty()) state.activeUsers
            else state.activeUsers.filter { it.name.contains(q, ignoreCase = true) }
        }
        ModalBottomSheet(onDismissRequest = vm::closeGroupMembers, sheetState = sheetState) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .navigationBarsPadding()
                    .imePadding()
                    .padding(horizontal = Spacing.sm)
                    .padding(bottom = Spacing.md),
                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                Text(
                    stringResource(R.string.settings_group_members_title, group.name, state.editingGroupMembersIds.size),
                    style = MaterialTheme.typography.titleMedium,
                )
                Row(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    OutlinedTextField(
                        value = groupMembersSearch,
                        onValueChange = { groupMembersSearch = it },
                        placeholder = { Text(stringResource(R.string.members_search_hint)) },
                        leadingIcon = { Icon(Icons.Outlined.Search, contentDescription = null) },
                        trailingIcon = if (groupMembersSearch.isNotEmpty()) {
                            {
                                IconButton(onClick = { groupMembersSearch = "" }) {
                                    Icon(Icons.Outlined.Close, contentDescription = stringResource(R.string.a11y_clear))
                                }
                            }
                        } else null,
                        singleLine = true,
                        modifier = Modifier.weight(1f),
                    )
                    TextButton(
                        onClick = {
                            UiHaptics.tick(view)
                            val allFilteredIds = filteredUsers.map { it.id }.toSet()
                            val allSelected = allFilteredIds.isNotEmpty() && allFilteredIds.all { it in state.editingGroupMembersIds }
                            if (allSelected) {
                                vm.setAllGroupMembers(state.editingGroupMembersIds - allFilteredIds)
                            } else {
                                vm.setAllGroupMembers(state.editingGroupMembersIds + allFilteredIds)
                            }
                        },
                        enabled = !state.groupMembersLoading && filteredUsers.isNotEmpty(),
                        modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                    ) {
                        val allSelected = filteredUsers.isNotEmpty() && filteredUsers.all { it.id in state.editingGroupMembersIds }
                        Text(stringResource(if (allSelected) R.string.settings_clear_all else R.string.settings_select_all))
                    }
                }
                if (state.groupMembersLoading) {
                    Box(Modifier.fillMaxWidth().height(160.dp)) { LoadingView() }
                } else {
                    LazyColumn(Modifier.fillMaxWidth().heightIn(max = 360.dp)) {
                        items(filteredUsers, key = { it.id }) { u ->
                            Row(
                                Modifier
                                    .fillMaxWidth()
                                    .defaultMinSize(minHeight = 48.dp)
                                    .toggleable(
                                        value = u.id in state.editingGroupMembersIds,
                                        role = Role.Checkbox,
                                        onValueChange = {
                                            UiHaptics.tick(view)
                                            vm.toggleGroupMember(u.id)
                                        },
                                    )
                                    .padding(horizontal = Spacing.xxs),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Checkbox(checked = u.id in state.editingGroupMembersIds, onCheckedChange = null)
                                Text(u.name, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.padding(start = Spacing.xs))
                            }
                        }
                    }
                }
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    OutlinedButton(
                        onClick = vm::closeGroupMembers,
                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                    ) { Text(stringResource(R.string.common_cancel)) }
                    Button(
                        onClick = {
                            UiHaptics.confirm(view)
                            vm.saveGroupMembers(groupSavedMsg)
                        },
                        enabled = !state.busy && !state.groupMembersLoading,
                        colors = graceRidePrimaryButtonColors(),
                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                    ) { Text(stringResource(R.string.common_save)) }
                }
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun SystemSettingsTab(
    state: AdminSettingsUiState,
    canUpdate: Boolean,
    me: org.pjstatecm.graceride.data.api.Me?,
    vm: AdminSettingsViewModel,
    nav: NavHostController,
) {
    val view = LocalView.current
    val auditVal = state.auditDaysText.trim().toIntOrNull()
    val chatVal = state.chatDaysText.trim().toIntOrNull()
    val driversVal = state.maxDriversText.trim().toIntOrNull()
    val timeoutVal = state.sessionTimeoutText.trim().toIntOrNull()
    val auditErr = auditVal !in 30..3650
    val chatErr = chatVal !in 7..3650
    val driversErr = driversVal !in 1..100
    val timeoutErr = timeoutVal !in 0..1440

    LazyColumn(
        Modifier.fillMaxSize().imePadding(),
        contentPadding = PaddingValues(Spacing.sm),
        verticalArrangement = Arrangement.spacedBy(Spacing.sm),
    ) {
        if (me?.can("admin_fares") == true || me?.can("admin_areas") == true || me?.can("admin_area_photos") == true) {
            item {
                OutlinedCard(Modifier.fillMaxWidth()) {
                    Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        SectionHeader(stringResource(R.string.settings_traffic_shortcuts))
                        FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            if (me.can("admin_fares")) {
                                OutlinedButton(onClick = { nav.navigate(AppRoute.TrafficHub(tab = "areas", areaTab = "fares")) }) {
                                    Text(stringResource(R.string.settings_tab_fares))
                                }
                            }
                            if (me.can("admin_area_photos")) {
                                OutlinedButton(onClick = { nav.navigate(AppRoute.TrafficHub(tab = "areas", areaTab = "photos")) }) {
                                    Text(stringResource(R.string.traffic_areas_sub_photos))
                                }
                            }
                            if (me.can("admin_areas")) {
                                OutlinedButton(onClick = { nav.navigate(AppRoute.TrafficHub(tab = "areas", areaTab = "areas")) }) {
                                    Text(stringResource(R.string.settings_areas))
                                }
                            }
                        }
                    }
                }
            }
        }

        item {
            OutlinedCard(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    SectionHeader(stringResource(R.string.settings_system_title))

                    OutlinedTextField(
                        value = state.auditDaysText,
                        onValueChange = vm::setAuditDays,
                        label = { Text(stringResource(R.string.settings_audit_retention)) },
                        supportingText = { Text(stringResource(R.string.settings_range_hint, 30, 3650)) },
                        isError = auditErr,
                        enabled = canUpdate && !state.busy,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        listOf(90, 365, 730, 1825).forEach { days ->
                            FilterChip(
                                selected = auditVal == days,
                                onClick = { if (canUpdate) { UiHaptics.tick(view); vm.setAuditDays(days.toString()) } },
                                label = { Text("${days}d") },
                            )
                        }
                    }

                    OutlinedTextField(
                        value = state.chatDaysText,
                        onValueChange = vm::setChatDays,
                        label = { Text(stringResource(R.string.settings_chat_retention)) },
                        supportingText = { Text(stringResource(R.string.settings_range_hint, 7, 3650)) },
                        isError = chatErr,
                        enabled = canUpdate && !state.busy,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        listOf(30, 90, 180, 365).forEach { days ->
                            FilterChip(
                                selected = chatVal == days,
                                onClick = { if (canUpdate) { UiHaptics.tick(view); vm.setChatDays(days.toString()) } },
                                label = { Text("${days}d") },
                            )
                        }
                    }

                    OutlinedTextField(
                        value = state.maxDriversText,
                        onValueChange = vm::setMaxDrivers,
                        label = { Text(stringResource(R.string.settings_max_live_drivers)) },
                        supportingText = { Text(stringResource(R.string.settings_range_hint, 1, 100)) },
                        isError = driversErr,
                        enabled = canUpdate && !state.busy,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )

                    OutlinedTextField(
                        value = state.sessionTimeoutText,
                        onValueChange = vm::setSessionTimeout,
                        label = { Text(stringResource(R.string.settings_session_timeout)) },
                        supportingText = { Text(stringResource(R.string.settings_range_hint, 0, 1440)) },
                        isError = timeoutErr,
                        enabled = canUpdate && !state.busy,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        listOf(0, 15, 30, 60, 240).forEach { mins ->
                            FilterChip(
                                selected = timeoutVal == mins,
                                onClick = { if (canUpdate) { UiHaptics.tick(view); vm.setSessionTimeout(mins.toString()) } },
                                label = { Text(if (mins == 0) stringResource(R.string.settings_mins_disabled) else "${mins}m") },
                            )
                        }
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun AlertsSettingsTab(
    state: AdminSettingsUiState,
    canUpdate: Boolean,
    vm: AdminSettingsViewModel,
    onCopy: (String, String) -> Unit,
) {
    SecureScreenEffect()
    val cfg = state.alertSettings
    val view = LocalView.current
    var showTgToken by remember { mutableStateOf(false) }
    var showWebhookSecret by remember { mutableStateOf(false) }
    var showVapidPriv by remember { mutableStateOf(false) }

    LazyColumn(
        Modifier.fillMaxSize().imePadding(),
        contentPadding = PaddingValues(Spacing.sm),
        verticalArrangement = Arrangement.spacedBy(Spacing.sm),
    ) {
        item {
            OutlinedCard(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    SectionHeader(stringResource(R.string.settings_alerts_status))
                    if (cfg != null) {
                        Text(
                            stringResource(
                                R.string.settings_alerts_summary,
                                cfg.pushSubscriptionsCount,
                                cfg.mobilePushSubscriptionsCount,
                            ),
                            style = MaterialTheme.typography.bodySmall.tabular(),
                        )
                        cfg.deliveryStats24h.forEach { (ch, stats) ->
                            val queued = stats.pending + stats.retry
                            val failed = stats.failed + stats.gone
                            Row(
                                Modifier.fillMaxWidth(),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                Text(ch, style = MaterialTheme.typography.labelLarge, modifier = Modifier.width(76.dp))
                                FlowRow(
                                    horizontalArrangement = Arrangement.spacedBy(Spacing.xxs),
                                    verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                                ) {
                                    StatusPill(stringResource(R.string.settings_delivery_ok, stats.delivered), RideStatusKind.ACTIVE)
                                    if (queued > 0) {
                                        StatusPill(stringResource(R.string.settings_delivery_queue, queued), RideStatusKind.PENDING)
                                    }
                                    if (failed > 0) {
                                        StatusPill(stringResource(R.string.settings_delivery_fail, failed), RideStatusKind.CANCELLED)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        item {
            OutlinedCard(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    Row(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(stringResource(R.string.settings_alerts_telegram), style = MaterialTheme.typography.titleMedium)
                        if (cfg?.telegramBotConfigured == true) {
                            StatusPill(stringResource(R.string.settings_configured_badge), RideStatusKind.ACTIVE)
                        }
                    }
                    OutlinedTextField(
                        value = state.tgTokenInput,
                        onValueChange = vm::setTgToken,
                        label = { Text(stringResource(R.string.settings_tg_token_hint)) },
                        visualTransformation = if (showTgToken) VisualTransformation.None else PasswordVisualTransformation(),
                        trailingIcon = {
                            IconButton(onClick = { showTgToken = !showTgToken }, modifier = Modifier.size(48.dp)) {
                                Icon(
                                    if (showTgToken) Icons.Outlined.VisibilityOff else Icons.Outlined.Visibility,
                                    contentDescription = stringResource(R.string.settings_reveal_secret),
                                )
                            }
                        },
                        enabled = canUpdate && !state.busy,
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    OutlinedTextField(
                        value = state.tgUsernameInput,
                        onValueChange = vm::setTgUsername,
                        label = { Text(stringResource(R.string.settings_tg_username)) },
                        enabled = canUpdate && !state.busy,
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    if (cfg?.telegramWebhookPath?.isNotBlank() == true) {
                        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                            Text(
                                cfg.telegramWebhookPath,
                                style = MaterialTheme.typography.bodySmall,
                                modifier = Modifier.weight(1f),
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                            IconButton(
                                onClick = { onCopy("webhook-path", cfg.telegramWebhookPath) },
                                modifier = Modifier.size(48.dp),
                            ) {
                                Icon(Icons.Outlined.ContentCopy, contentDescription = stringResource(R.string.common_copied))
                            }
                        }
                    }
                    if (canUpdate && cfg?.telegramBotConfigured == true) {
                        Row(
                            Modifier
                                .fillMaxWidth()
                                .defaultMinSize(minHeight = 48.dp)
                                .toggleable(
                                    value = state.clearTgToken,
                                    role = Role.Switch,
                                    onValueChange = { UiHaptics.tick(view); vm.setClearTgToken(it) },
                                ),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween,
                        ) {
                            Text(stringResource(R.string.settings_clear_secret), style = MaterialTheme.typography.bodySmall)
                            Switch(checked = state.clearTgToken, onCheckedChange = null)
                        }
                    }
                }
            }
        }

        item {
            OutlinedCard(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    Row(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(stringResource(R.string.settings_alerts_webhooks), style = MaterialTheme.typography.titleMedium)
                        if (cfg?.alertWebhookConfigured == true || cfg?.vapidPrivateConfigured == true) {
                            StatusPill(stringResource(R.string.settings_configured_badge), RideStatusKind.ACTIVE)
                        }
                    }
                    OutlinedTextField(
                        value = state.webhookUrlInput,
                        onValueChange = vm::setWebhookUrl,
                        label = { Text(stringResource(R.string.settings_webhook_url)) },
                        placeholder = { Text(cfg?.alertWebhookUrl.orEmpty()) },
                        enabled = canUpdate && !state.busy,
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    OutlinedTextField(
                        value = state.webhookSecretInput,
                        onValueChange = vm::setWebhookSecret,
                        label = { Text(stringResource(R.string.settings_webhook_secret)) },
                        visualTransformation = if (showWebhookSecret) VisualTransformation.None else PasswordVisualTransformation(),
                        trailingIcon = {
                            IconButton(onClick = { showWebhookSecret = !showWebhookSecret }, modifier = Modifier.size(48.dp)) {
                                Icon(
                                    if (showWebhookSecret) Icons.Outlined.VisibilityOff else Icons.Outlined.Visibility,
                                    contentDescription = stringResource(R.string.settings_reveal_secret),
                                )
                            }
                        },
                        enabled = canUpdate && !state.busy,
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    OutlinedTextField(
                        value = state.mobileWebhookUrlInput,
                        onValueChange = vm::setMobileWebhookUrl,
                        label = { Text(stringResource(R.string.settings_mobile_webhook_url)) },
                        placeholder = { Text(cfg?.mobilePushWebhookUrl.orEmpty()) },
                        enabled = canUpdate && !state.busy,
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    if (canUpdate && cfg?.alertWebhookConfigured == true) {
                        Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs), verticalAlignment = Alignment.CenterVertically) {
                            OutlinedButton(
                                onClick = { UiHaptics.tick(view); vm.testWebhook() },
                                enabled = !state.testingWebhook && !state.busy,
                                modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                            ) {
                                if (state.testingWebhook) {
                                    InlineButtonSpinner()
                                    Spacer(Modifier.width(Spacing.xxs))
                                }
                                Text(stringResource(R.string.settings_test_webhook))
                            }
                            state.lastWebhookTest?.let { res ->
                                StatusPill(
                                    text = if (res.ok) "OK (${res.statusCode} · ${res.latencyMs}ms)" else "ERR (${res.statusCode}) ${res.error.orEmpty()}",
                                    kind = if (res.ok) RideStatusKind.ACTIVE else RideStatusKind.CANCELLED,
                                )
                            }
                        }
                    }
                    OutlinedTextField(
                        value = state.vapidPubInput,
                        onValueChange = vm::setVapidPub,
                        label = { Text(stringResource(R.string.settings_vapid_pub)) },
                        enabled = canUpdate && !state.busy && !state.rotateVapidKeys,
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    OutlinedTextField(
                        value = state.vapidPrivInput,
                        onValueChange = vm::setVapidPriv,
                        label = { Text(stringResource(R.string.settings_vapid_priv)) },
                        visualTransformation = if (showVapidPriv) VisualTransformation.None else PasswordVisualTransformation(),
                        trailingIcon = {
                            IconButton(onClick = { showVapidPriv = !showVapidPriv }, modifier = Modifier.size(48.dp)) {
                                Icon(
                                    if (showVapidPriv) Icons.Outlined.VisibilityOff else Icons.Outlined.Visibility,
                                    contentDescription = stringResource(R.string.settings_reveal_secret),
                                )
                            }
                        },
                        enabled = canUpdate && !state.busy && !state.rotateVapidKeys,
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    if (canUpdate) {
                        Row(
                            Modifier
                                .fillMaxWidth()
                                .defaultMinSize(minHeight = 48.dp)
                                .toggleable(
                                    value = state.rotateVapidKeys,
                                    role = Role.Switch,
                                    onValueChange = { UiHaptics.tick(view); vm.setRotateVapidKeys(it) },
                                ),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween,
                        ) {
                            Text(stringResource(R.string.settings_rotate_vapid), style = MaterialTheme.typography.bodySmall)
                            Switch(checked = state.rotateVapidKeys, onCheckedChange = null)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun GroupsSettingsTab(
    state: AdminSettingsUiState,
    canCreate: Boolean,
    canUpdate: Boolean,
    canDelete: Boolean,
    onOpenCreate: () -> Unit,
    onEditGroup: (GroupDto) -> Unit,
    onManageMembers: (GroupDto) -> Unit,
    onDeleteGroup: (GroupDto) -> Unit,
) {
    val ordered = remember(state.groups) { buildTreeOrder(state.groups) }
    val byId = remember(state.groups) { state.groups.associateBy { it.id } }

    LazyColumn(
        Modifier.fillMaxSize().navigationBarsPadding(),
        contentPadding = PaddingValues(Spacing.sm),
        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
    ) {
        if (canCreate) {
            item {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                    Button(
                        onClick = onOpenCreate,
                        colors = graceRidePrimaryButtonColors(),
                        modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                    ) {
                        Icon(Icons.Outlined.Add, contentDescription = null, modifier = Modifier.size(18.dp))
                        Spacer(Modifier.width(Spacing.xxs))
                        Text(stringResource(R.string.settings_add_group))
                    }
                }
            }
        }
        if (ordered.isEmpty()) {
            item { EmptyView(text = stringResource(R.string.settings_empty_groups)) }
        } else {
            items(ordered, key = { it.first.id }) { (group, depth) ->
                OutlinedCard(
                    modifier = Modifier.fillMaxWidth().padding(start = (depth * 14).dp),
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                ) {
                    Row(Modifier.fillMaxWidth().height(IntrinsicSize.Min)) {
                        if (depth > 0) {
                            Box(Modifier.width(4.dp).fillMaxHeight().background(MaterialTheme.colorScheme.primary))
                        }
                        Column(Modifier.weight(1f).padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
                            Text(
                                text = group.name,
                                style = MaterialTheme.typography.titleMedium,
                            )
                            val parentName = group.parentId?.let { byId[it]?.name }
                            val meta = buildList {
                                if (group.isAll) add(stringResource(R.string.settings_all_members))
                                else if (group.isVisitor) add(stringResource(R.string.settings_visitor_pill))
                                else add(stringResource(R.string.settings_member_count, group.memberCount))
                                if (!parentName.isNullOrBlank()) add(stringResource(R.string.settings_part_of_name, parentName))
                                if (group.description.isNotBlank() && !group.isVisitor) add(group.description)
                            }.joinToString(" · ")
                            Text(meta, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)

                            if (!group.isAll && !group.isVisitor && (canUpdate || canDelete)) {
                                Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                    if (canUpdate) {
                                        OutlinedButton(
                                            onClick = { onEditGroup(group) },
                                            modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                                        ) {
                                            Icon(Icons.Outlined.Edit, contentDescription = null, modifier = Modifier.size(16.dp))
                                            Spacer(Modifier.width(4.dp))
                                            Text(stringResource(R.string.settings_edit))
                                        }
                                        OutlinedButton(
                                            onClick = { onManageMembers(group) },
                                            modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                                        ) {
                                            Icon(Icons.Outlined.PeopleOutline, contentDescription = null, modifier = Modifier.size(16.dp))
                                            Spacer(Modifier.width(4.dp))
                                            Text(stringResource(R.string.settings_members_btn))
                                        }
                                    }
                                    if (canDelete) {
                                        IconButton(
                                            onClick = { onDeleteGroup(group) },
                                            modifier = Modifier.size(48.dp),
                                        ) {
                                            Icon(
                                                Icons.Outlined.DeleteOutline,
                                                contentDescription = stringResource(R.string.common_delete),
                                                tint = MaterialTheme.colorScheme.error,
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

private fun buildTreeOrder(groups: List<GroupDto>): List<Pair<GroupDto, Int>> {
    val childrenByParent = groups.groupBy { it.parentId }
    val visited = mutableSetOf<Long>()
    val result = mutableListOf<Pair<GroupDto, Int>>()
    fun walk(parentId: Long?, depth: Int) {
        val list = childrenByParent[parentId].orEmpty()
        list.forEach { g ->
            if (visited.add(g.id)) {
                result += g to depth
                walk(g.id, (depth + 1).coerceAtMost(3))
            }
        }
    }
    walk(null, 0)
    groups.filter { it.id !in visited }.forEach { result += it to 0 }
    return result
}
