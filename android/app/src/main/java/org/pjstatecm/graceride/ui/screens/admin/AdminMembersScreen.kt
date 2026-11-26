package org.pjstatecm.graceride.ui.screens.admin

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.snap
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.Sort
import androidx.compose.material.icons.outlined.AddLink
import androidx.compose.material.icons.outlined.AdminPanelSettings
import androidx.compose.material.icons.outlined.Badge
import androidx.compose.material.icons.outlined.Call
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.Download
import androidx.compose.material.icons.outlined.Info
import androidx.compose.material.icons.outlined.Key
import androidx.compose.material.icons.outlined.MarkEmailUnread
import androidx.compose.material.icons.outlined.PeopleOutline
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material.icons.outlined.Share
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.AdminInviteDto
import org.pjstatecm.graceride.data.api.AdminUserDto
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.RecoveryRequestDto
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.MemberAdminRules
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.domain.RoleLabels
import org.pjstatecm.graceride.platform.CsvBuilder
import org.pjstatecm.graceride.platform.ExportShare
import org.pjstatecm.graceride.ui.SecureScreenEffect
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.GraceRideSpringEasing
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.MemberAvatar
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.components.animatorsEnabled
import org.pjstatecm.graceride.ui.copyToClipboard
import org.pjstatecm.graceride.ui.dialPhone
import org.pjstatecm.graceride.ui.shareText
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AdminMembersScreen(
    container: AppContainer,
    nav: NavHostController,
    initialTab: String = "members",
    initialStatus: String? = null,
) {
    SecureScreenEffect()

    val vm: AdminMembersViewModel = viewModel(
        factory = AdminMembersViewModel.factory(
            container = container,
            initialTab = initialTab,
            initialStatus = initialStatus,
        ),
    )
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val currentMe = (authState as? AuthState.SignedIn)?.me ?: state.me
    val snackbar = remember { SnackbarHostState() }
    val scope = rememberCoroutineScope()
    val context = LocalContext.current
    val view = LocalView.current

    var editingUser by remember { mutableStateOf<Pair<AdminUserDto, Boolean>?>(null) }
    var batchSheetOpen by remember { mutableStateOf(false) }

    LaunchedEffect(currentMe) { vm.syncMe(currentMe) }

    val approvedNotice = stringResource(R.string.members_approved_msg)
    val updatedNotice = stringResource(R.string.members_updated_msg)
    val inviteCreatedNotice = stringResource(R.string.members_invite_created)
    val inviteRevokedNotice = stringResource(R.string.members_revoked_msg)
    val recoveryApprovedNotice = stringResource(R.string.members_recovery_approved_msg)
    val recoveryAwaitingNotice = stringResource(R.string.members_recovery_awaiting_msg)
    val recoveryRejectedNotice = stringResource(R.string.members_recovery_rejected_msg)
    val copiedMsg = stringResource(R.string.common_copied)

    LaunchedEffect(state.notice, state.batchSuccessCount, state.batchFailureCount, state.messageText) {
        val msg = when {
            state.notice != null -> when (state.notice!!) {
                AdminMembersNotice.MEMBER_APPROVED -> approvedNotice
                AdminMembersNotice.MEMBER_UPDATED -> updatedNotice
                AdminMembersNotice.INVITE_CREATED -> inviteCreatedNotice
                AdminMembersNotice.INVITE_REVOKED -> inviteRevokedNotice
                AdminMembersNotice.RECOVERY_APPROVED -> recoveryApprovedNotice
                AdminMembersNotice.RECOVERY_AWAITING_SECOND -> recoveryAwaitingNotice
                AdminMembersNotice.RECOVERY_REJECTED -> recoveryRejectedNotice
            }
            state.batchSuccessCount != null -> {
                val ok = state.batchSuccessCount!!
                val fail = state.batchFailureCount
                when {
                    fail == 0 -> context.getString(R.string.members_batch_success, ok)
                    ok > 0 -> context.getString(R.string.members_batch_partial, ok, fail)
                    else -> state.messageText
                }
            }
            state.messageText != null -> state.messageText
            else -> null
        }
        if (!msg.isNullOrBlank()) {
            snackbar.showSnackbar(msg)
            vm.clearNotices()
        }
    }

    val permittedTabs = remember(currentMe, state.stats) {
        buildList {
            if (currentMe?.can("admin_members") == true) {
                add(Triple("members", R.string.members_tab_members, (state.stats?.pendingMembers ?: 0L).toInt()))
            }
            if (currentMe?.can("admin_invites") == true) {
                add(Triple("invites", R.string.members_tab_invites, (state.stats?.openInvites ?: 0L).toInt()))
            }
            if (currentMe?.can("admin_recovery") == true) {
                add(Triple("recovery", R.string.members_tab_recovery, (state.stats?.pendingRecovery ?: 0L).toInt()))
            }
        }
    }
    val canEditMembers = currentMe?.can("admin_members", "update") == true
    val selectedUsers = remember(state.usersPage.items, state.selectedUserIds, currentMe) {
        state.usersPage.items.filter { MemberAdminRules.isEditable(currentMe, it) && it.id in state.selectedUserIds }
    }
    val hasBatchBar = state.tab == "members" && selectedUsers.isNotEmpty() && canEditMembers
    var lastSelectedCount by remember { mutableIntStateOf(0) }
    if (selectedUsers.isNotEmpty()) {
        lastSelectedCount = selectedUsers.size
    }
    val snackbarBottomPadding by animateDpAsState(
        targetValue = if (hasBatchBar) 76.dp else 0.dp,
        animationSpec = if (animatorsEnabled()) tween(220, easing = GraceRideSpringEasing) else snap(),
        label = "members_snackbar_bottom",
    )

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        snackbarHost = {
            SnackbarHost(
                hostState = snackbar,
                modifier = Modifier.padding(bottom = snackbarBottomPadding),
            )
        },
        topBar = {
            TopAppBar(
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background),
                title = {
                    Text(
                        stringResource(R.string.members_title),
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(
                            Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = stringResource(R.string.a11y_back),
                        )
                    }
                },
                actions = {
                    if (state.tab == "members" && state.usersPage.items.isNotEmpty()) {
                        IconButton(
                            onClick = {
                                scope.launch {
                                    val bytes = CsvBuilder.buildBytes(
                                        headers = listOf("ID", "Name", "Email", "Phone", "Status", "Roles", "Groups", "Passkeys", "Created"),
                                        rows = state.usersPage.items.map { u ->
                                            listOf(
                                                u.id.toString(),
                                                u.name,
                                                u.email,
                                                u.phone,
                                                u.status,
                                                u.roles.joinToString("; "),
                                                u.groups,
                                                u.passkeys.toString(),
                                                u.createdAt,
                                            )
                                        },
                                    )
                                    ExportShare.writeAndShareFile(
                                        context = context,
                                        filename = "grace-ride-members.csv",
                                        mimeType = "text/csv",
                                        bytes = bytes,
                                    )
                                }
                            },
                        ) {
                            Icon(
                                Icons.Outlined.Download,
                                contentDescription = stringResource(R.string.common_export_csv),
                            )
                        }
                    }
                    if (permittedTabs.isNotEmpty()) {
                        IconButton(onClick = { vm.refreshActive(silent = false) }) {
                            Icon(
                                Icons.Outlined.Refresh,
                                contentDescription = stringResource(R.string.common_refresh),
                            )
                        }
                    }
                },
            )
        },
    ) { padding ->
        if (permittedTabs.isEmpty()) {
            Box(Modifier.fillMaxSize().padding(padding)) {
                EmptyView(
                    text = stringResource(R.string.members_no_permission),
                    icon = Icons.Outlined.AdminPanelSettings,
                )
            }
            return@Scaffold
        }

        Box(Modifier.fillMaxSize().padding(padding)) {
            Column(Modifier.fillMaxSize()) {
                if (permittedTabs.size > 1) {
                    val selectedIdx = permittedTabs.indexOfFirst { it.first == state.tab }.coerceAtLeast(0)
                    PrimaryTabRow(selectedTabIndex = selectedIdx) {
                        permittedTabs.forEach { (key, labelRes, count) ->
                            Tab(
                                selected = state.tab == key,
                                onClick = {
                                    UiHaptics.tick(view)
                                    vm.setTab(key)
                                },
                                text = {
                                    Row(
                                        verticalAlignment = Alignment.CenterVertically,
                                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                                    ) {
                                        Text(
                                            stringResource(labelRes),
                                            maxLines = 1,
                                            overflow = TextOverflow.Ellipsis,
                                        )
                                        if (count > 0) {
                                            Surface(
                                                color = MaterialTheme.colorScheme.primaryContainer,
                                                contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                                                shape = CircleShape,
                                            ) {
                                                Text(
                                                    count.toString(),
                                                    style = MaterialTheme.typography.labelSmall.tabular(),
                                                    modifier = Modifier.padding(horizontal = 6.dp, vertical = 1.dp),
                                                )
                                            }
                                        }
                                    }
                                },
                            )
                        }
                    }
                }

                AnimatedContent(
                    targetState = state.tab,
                    transitionSpec = {
                        val anim = animatorsEnabled()
                        if (anim) {
                            val fromIdx = permittedTabs.indexOfFirst { it.first == initialState }
                            val toIdx = permittedTabs.indexOfFirst { it.first == targetState }
                            val dir = if (fromIdx != -1 && toIdx != -1 && toIdx != fromIdx) {
                                if (toIdx > fromIdx) 1 else -1
                            } else {
                                1
                            }
                            (slideInHorizontally(tween(240, easing = GraceRideSpringEasing)) { it * dir } +
                                fadeIn(tween(180)))
                                .togetherWith(
                                    slideOutHorizontally(tween(220, easing = GraceRideSpringEasing)) { (-it / 3) * dir } +
                                        fadeOut(tween(140)),
                                )
                        } else {
                            fadeIn(tween(120)).togetherWith(fadeOut(tween(100)))
                        }
                    },
                    label = "admin_members_tab",
                    modifier = Modifier.weight(1f),
                ) { activeTab ->
                    when (activeTab) {
                        "members" -> MembersTab(
                            state = state,
                            me = currentMe,
                            canEdit = canEditMembers,
                            photoUrlFor = container::fileUrl,
                            vm = vm,
                            onEditUser = { user, isApprove ->
                                vm.clearSaveError()
                                editingUser = user to isApprove
                            },
                            hasBatchBar = hasBatchBar,
                        )
                        "invites" -> InvitesTab(
                            state = state,
                            me = currentMe,
                            vm = vm,
                            onCopied = { scope.launch { snackbar.showSnackbar(copiedMsg) } },
                        )
                        "recovery" -> RecoveryTab(
                            state = state,
                            me = currentMe,
                            vm = vm,
                            onCopied = { scope.launch { snackbar.showSnackbar(copiedMsg) } },
                        )
                    }
                }
            }

            // Sticky batch approval bar
            AnimatedVisibility(
                visible = hasBatchBar,
                enter = if (animatorsEnabled()) {
                    slideInVertically(tween(220, easing = GraceRideSpringEasing)) { it } + fadeIn(tween(160))
                } else {
                    fadeIn(tween(100))
                },
                exit = if (animatorsEnabled()) {
                    slideOutVertically(tween(180, easing = GraceRideSpringEasing)) { it } + fadeOut(tween(120))
                } else {
                    fadeOut(tween(80))
                },
                modifier = Modifier
                    .align(Alignment.BottomCenter)
                    .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
            ) {
                Surface(
                    color = MaterialTheme.colorScheme.primaryContainer,
                    contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                    shape = MaterialTheme.shapes.large,
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.primary.copy(alpha = 0.45f)),
                    tonalElevation = 4.dp,
                    shadowElevation = 4.dp,
                ) {
                    Row(
                        Modifier
                            .fillMaxWidth()
                            .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        Text(
                            stringResource(R.string.members_selected_count, lastSelectedCount),
                            style = MaterialTheme.typography.titleSmall.tabular(),
                            modifier = Modifier.weight(1f),
                        )
                        TextButton(onClick = { vm.clearSelection() }, enabled = !state.batchBusy) {
                            Text(stringResource(R.string.common_cancel))
                        }
                        Button(
                            onClick = {
                                vm.clearSaveError()
                                batchSheetOpen = true
                            },
                            enabled = !state.batchBusy,
                            colors = graceRidePrimaryButtonColors(),
                        ) {
                            Text(stringResource(R.string.members_batch_approve, lastSelectedCount))
                        }
                    }
                }
            }
        }
    }

    editingUser?.let { (user, isApprove) ->
        MemberEditSheet(
            user = user,
            isApproveMode = isApprove,
            me = currentMe,
            rolesCatalog = state.roles,
            groups = state.groups,
            groupsLoaded = state.groupsLoaded,
            photoUrlFor = container::fileUrl,
            busy = state.savingUser,
            error = state.saveError,
            onSave = { roles, status, groupIds, presidentAck ->
                vm.saveUser(
                    user = user,
                    roles = roles,
                    status = status,
                    groupIds = groupIds,
                    presidentAcknowledged = presidentAck,
                    isApproveMode = isApprove,
                    onSuccess = { editingUser = null },
                )
            },
            onCopiedEmail = { scope.launch { snackbar.showSnackbar(copiedMsg) } },
            onDismiss = {
                vm.clearSaveError()
                editingUser = null
            },
        )
    }

    if (batchSheetOpen && selectedUsers.isNotEmpty()) {
        BatchApproveMembersSheet(
            selectedUsers = selectedUsers,
            me = currentMe,
            rolesCatalog = state.roles,
            groups = state.groups,
            groupsLoaded = state.groupsLoaded,
            busy = state.batchBusy,
            error = state.saveError,
            onConfirm = { roleKey, groupIds, groupsTouched, presidentAck ->
                vm.batchApprove(
                    users = selectedUsers,
                    roleKey = roleKey,
                    groupIds = groupIds,
                    groupsTouched = groupsTouched,
                    presidentAcknowledged = presidentAck,
                    onDone = { batchSheetOpen = false },
                )
            },
            onDismiss = {
                vm.clearSaveError()
                batchSheetOpen = false
            },
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
private fun MembersTab(
    state: AdminMembersUiState,
    me: Me?,
    canEdit: Boolean,
    photoUrlFor: (Long) -> String?,
    vm: AdminMembersViewModel,
    onEditUser: (AdminUserDto, Boolean) -> Unit,
    hasBatchBar: Boolean,
) {
    val view = LocalView.current
    var showRolePicker by remember { mutableStateOf(false) }
    var showSortPicker by remember { mutableStateOf(false) }

    val users = state.usersPage.items
    val editableOnPage = remember(users, me) {
        users.filter { MemberAdminRules.isEditable(me, it) }
    }
    val hasFilters = state.query.isNotBlank() || state.statusFilter.isNotBlank() || state.roleFilter.isNotBlank()

    Column(Modifier.fillMaxSize()) {
        // Search field
        OutlinedTextField(
            value = state.query,
            onValueChange = { vm.setQuery(it) },
            placeholder = {
                Text(
                    stringResource(R.string.members_search_hint),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            },
            leadingIcon = { Icon(Icons.Outlined.Search, contentDescription = null) },
            trailingIcon = {
                if (state.query.isNotEmpty()) {
                    IconButton(onClick = { vm.clearQuery() }) {
                        Icon(Icons.Outlined.Close, contentDescription = stringResource(R.string.a11y_clear))
                    }
                }
            },
            singleLine = true,
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
            keyboardActions = KeyboardActions(onSearch = { vm.submitSearch() }),
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
        )

        // Status chips + Role & Sort pickers
        LazyRow(
            contentPadding = PaddingValues(horizontal = Spacing.sm),
            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            val statuses = listOf(
                "" to R.string.members_status_all,
                "pending" to R.string.members_status_pending,
                "active" to R.string.members_status_active,
                "disabled" to R.string.members_status_disabled,
            )
            items(statuses) { (key, labelRes) ->
                FilterChip(
                    selected = state.statusFilter == key,
                    onClick = {
                        UiHaptics.tick(view)
                        vm.setStatusFilter(key)
                    },
                    label = { Text(stringResource(labelRes), maxLines = 1) },
                )
            }
            item {
                FilterChip(
                    selected = state.roleFilter.isNotBlank(),
                    onClick = { showRolePicker = true },
                    leadingIcon = { Icon(Icons.Outlined.Badge, contentDescription = null, modifier = Modifier.size(16.dp)) },
                    label = {
                        Text(
                            if (state.roleFilter.isNotBlank()) roleDisplayName(state.roleFilter, state.roles)
                            else stringResource(R.string.members_role_all),
                            maxLines = 1,
                        )
                    },
                )
            }
            item {
                FilterChip(
                    selected = state.sortKey != "name" || state.sortDir != "asc",
                    onClick = { showSortPicker = true },
                    leadingIcon = { Icon(Icons.AutoMirrored.Outlined.Sort, contentDescription = null, modifier = Modifier.size(16.dp)) },
                    label = { Text(sortOptionLabel(state.sortKey, state.sortDir), maxLines = 1) },
                )
            }
            if (hasFilters) {
                item {
                    FilterChip(
                        selected = false,
                        onClick = { vm.clearFilters() },
                        trailingIcon = { Icon(Icons.Outlined.Close, contentDescription = null, modifier = Modifier.size(16.dp)) },
                        label = { Text(stringResource(R.string.members_clear_filters)) },
                    )
                }
            }
        }

        val membersMode = when {
            state.membersLoading && users.isEmpty() -> "loading"
            state.membersError != null && users.isEmpty() -> "error"
            else -> "content"
        }
        Crossfade(
            targetState = membersMode,
            animationSpec = if (animatorsEnabled()) tween(200) else snap(),
            label = "members_tab_state",
            modifier = Modifier.weight(1f),
        ) { mode ->
            when (mode) {
                "loading" -> LoadingView()
                "error" -> ErrorView(
                    message = state.membersError ?: "",
                    onRetry = { vm.refreshActive(silent = false) },
                )
                else -> PullToRefreshBox(
                    isRefreshing = state.refreshing,
                    onRefresh = { vm.refreshActive(silent = false) },
                    modifier = Modifier.fillMaxSize(),
                ) {
                    LazyColumn(
                        Modifier.fillMaxSize(),
                        contentPadding = PaddingValues(
                            start = Spacing.sm,
                            end = Spacing.sm,
                            top = Spacing.xs,
                            bottom = (if (hasBatchBar) 88.dp else Spacing.lg),
                        ),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        if (users.isEmpty()) {
                            item {
                                EmptyView(
                                    text = stringResource(R.string.members_empty),
                                    icon = Icons.Outlined.PeopleOutline,
                                    actionLabel = if (hasFilters) stringResource(R.string.members_clear_filters) else null,
                                    onAction = if (hasFilters) { { vm.clearFilters() } } else null,
                                )
                            }
                        } else {
                            item {
                                Row(
                                    Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Text(
                                        stringResource(
                                            R.string.members_total_page,
                                            state.usersPage.total,
                                            state.usersPage.page,
                                            state.usersPage.totalPages,
                                        ),
                                        style = MaterialTheme.typography.bodySmall.tabular(),
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                    if (canEdit && editableOnPage.isNotEmpty()) {
                                        val allSelected = editableOnPage.all { it.id in state.selectedUserIds }
                                        TextButton(
                                            onClick = {
                                                if (allSelected) vm.clearSelection() else vm.selectAllEditableOnPage()
                                            },
                                        ) {
                                            Text(
                                                stringResource(
                                                    if (allSelected) R.string.common_cancel else R.string.members_select_all,
                                                ),
                                            )
                                        }
                                    }
                                }
                            }

                            items(users, key = { "member-${it.id}" }) { user ->
                                MemberCard(
                                    user = user,
                                    editable = MemberAdminRules.isEditable(me, user),
                                    selected = user.id in state.selectedUserIds,
                                    rolesCatalog = state.roles,
                                    photoUrlFor = photoUrlFor,
                                    modifier = Modifier.animateItem(),
                                    onToggleSelect = { vm.toggleSelectUser(user.id) },
                                    onActionClick = { onEditUser(user, user.status == "pending") },
                                )
                            }

                            if (state.usersPage.totalPages > 1) {
                                item {
                                    Row(
                                        Modifier.fillMaxWidth().padding(top = Spacing.xs),
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        OutlinedButton(
                                            onClick = { vm.goToPage(state.usersPage.page - 1) },
                                            enabled = state.usersPage.page > 1 && !state.membersLoading,
                                        ) {
                                            Text(stringResource(R.string.traffic_prev_page))
                                        }
                                        Text(
                                            "${state.usersPage.page} / ${state.usersPage.totalPages}",
                                            style = MaterialTheme.typography.bodySmall.tabular(),
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                        OutlinedButton(
                                            onClick = { vm.goToPage(state.usersPage.page + 1) },
                                            enabled = state.usersPage.page < state.usersPage.totalPages && !state.membersLoading,
                                        ) {
                                            Text(stringResource(R.string.traffic_next_page))
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

    if (showRolePicker) {
        val assignable = remember(state.roles) {
            if (state.roles.isNotEmpty()) {
                state.roles.filter { it.assignable && !it.implicit }.map { it.key }
            } else {
                RoleLabels.BUILT_IN_ASSIGNABLE
            }
        }
        val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
        ModalBottomSheet(
            onDismissRequest = { showRolePicker = false },
            sheetState = sheetState,
            containerColor = MaterialTheme.colorScheme.surface,
        ) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .navigationBarsPadding()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
            ) {
                Text(
                    stringResource(R.string.members_role_label),
                    style = MaterialTheme.typography.titleLarge,
                    modifier = Modifier.padding(bottom = Spacing.xs),
                )
                Surface(
                    onClick = {
                        vm.setRoleFilter("")
                        showRolePicker = false
                    },
                    color = MaterialTheme.colorScheme.surface,
                ) {
                    Row(
                        Modifier.fillMaxWidth().padding(vertical = 10.dp, horizontal = Spacing.xs),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        RadioButton(
                            selected = state.roleFilter.isEmpty(),
                            onClick = null,
                        )
                        Text(
                            stringResource(R.string.members_role_all),
                            style = MaterialTheme.typography.bodyLarge,
                        )
                    }
                }
                assignable.forEach { roleKey ->
                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                    Surface(
                        onClick = {
                            vm.setRoleFilter(roleKey)
                            showRolePicker = false
                        },
                        color = MaterialTheme.colorScheme.surface,
                    ) {
                        Row(
                            Modifier.fillMaxWidth().padding(vertical = 10.dp, horizontal = Spacing.xs),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            RadioButton(
                                selected = state.roleFilter == roleKey,
                                onClick = null,
                            )
                            Text(
                                roleDisplayName(roleKey, state.roles),
                                style = MaterialTheme.typography.bodyLarge,
                            )
                        }
                    }
                }
                Spacer(Modifier.height(Spacing.sm))
            }
        }
    }

    if (showSortPicker) {
        val options = listOf(
            ("name" to "asc") to R.string.members_sort_name_asc,
            ("name" to "desc") to R.string.members_sort_name_desc,
            ("created_at" to "desc") to R.string.members_sort_newest,
            ("status" to "asc") to R.string.members_sort_status,
        )
        val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
        ModalBottomSheet(
            onDismissRequest = { showSortPicker = false },
            sheetState = sheetState,
            containerColor = MaterialTheme.colorScheme.surface,
        ) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .navigationBarsPadding()
                    .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
            ) {
                Text(
                    stringResource(R.string.members_sort_label),
                    style = MaterialTheme.typography.titleLarge,
                    modifier = Modifier.padding(bottom = Spacing.xs),
                )
                options.forEachIndexed { idx, (pair, labelRes) ->
                    if (idx > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                    val selected = state.sortKey == pair.first && state.sortDir == pair.second
                    Surface(
                        onClick = {
                            vm.setSort(pair.first, pair.second)
                            showSortPicker = false
                        },
                        color = MaterialTheme.colorScheme.surface,
                    ) {
                        Row(
                            Modifier.fillMaxWidth().padding(vertical = 12.dp, horizontal = Spacing.xs),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            RadioButton(selected = selected, onClick = null)
                            Text(stringResource(labelRes), style = MaterialTheme.typography.bodyLarge)
                        }
                    }
                }
                Spacer(Modifier.height(Spacing.sm))
            }
        }
    }
}

@Composable
private fun sortOptionLabel(sortKey: String, sortDir: String): String = when {
    sortKey == "name" && sortDir == "desc" -> stringResource(R.string.members_sort_name_desc)
    sortKey == "created_at" -> stringResource(R.string.members_sort_newest)
    sortKey == "status" -> stringResource(R.string.members_sort_status)
    else -> stringResource(R.string.members_sort_name_asc)
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun MemberCard(
    user: AdminUserDto,
    editable: Boolean,
    selected: Boolean,
    rolesCatalog: List<org.pjstatecm.graceride.data.api.AdminRoleDefDto>,
    photoUrlFor: (Long) -> String?,
    onToggleSelect: () -> Unit,
    onActionClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    val statusKind = when (user.status) {
        "active" -> RideStatusKind.ACTIVE
        "pending" -> RideStatusKind.PENDING
        else -> RideStatusKind.CANCELLED
    }
    val statusLabel = when (user.status) {
        "active" -> stringResource(R.string.members_status_active)
        "pending" -> stringResource(R.string.members_status_pending)
        else -> stringResource(R.string.members_status_disabled)
    }

    OutlinedCard(
        modifier = modifier.fillMaxWidth(),
        border = BorderStroke(
            1.dp,
            if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outlineVariant,
        ),
    ) {
        Column(
            Modifier.padding(Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Row(
                Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                if (editable) {
                    Checkbox(
                        checked = selected,
                        onCheckedChange = { onToggleSelect() },
                    )
                }
                MemberAvatar(
                    name = user.name,
                    photoUrl = user.photoFileId?.let(photoUrlFor),
                    avatarColor = user.avatarColor,
                    avatarIcon = user.avatarIcon,
                    size = 42.dp,
                )
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Text(
                        user.name,
                        style = MaterialTheme.typography.titleMedium,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    if (user.email.isNotBlank()) {
                        Text(
                            user.email,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }
                StatusPill(statusLabel, statusKind)
            }

            // Role pills
            FlowRow(
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                if (user.roles.isEmpty()) {
                    Surface(
                        color = MaterialTheme.colorScheme.surfaceVariant,
                        contentColor = MaterialTheme.colorScheme.onSurfaceVariant,
                        shape = CircleShape,
                    ) {
                        Text(
                            stringResource(R.string.members_member_default),
                            style = MaterialTheme.typography.labelMedium,
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp),
                        )
                    }
                } else {
                    user.roles.forEach { r ->
                        Surface(
                            color = MaterialTheme.colorScheme.primaryContainer,
                            contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                            shape = CircleShape,
                        ) {
                            Text(
                                roleDisplayName(r, rolesCatalog),
                                style = MaterialTheme.typography.labelMedium,
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp),
                            )
                        }
                    }
                }
                if (user.groups.isNotBlank()) {
                    Surface(
                        color = MaterialTheme.colorScheme.secondaryContainer,
                        contentColor = MaterialTheme.colorScheme.onSecondaryContainer,
                        shape = CircleShape,
                    ) {
                        Text(
                            user.groups,
                            style = MaterialTheme.typography.labelMedium,
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp),
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }
            }

            // Meta + action row
            Row(
                Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        Text(
                            stringResource(R.string.members_passkeys_count, user.passkeys),
                            style = MaterialTheme.typography.bodySmall.tabular(),
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        if (!user.familyName.isNullOrBlank()) {
                            Text(
                                "· ${user.familyName}",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                        }
                    }
                    if (user.phone.isNotBlank()) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(4.dp),
                        ) {
                            Icon(
                                Icons.Outlined.Call,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.primary,
                                modifier = Modifier.size(14.dp),
                            )
                            TextButton(
                                onClick = { dialPhone(context, user.phone) },
                                contentPadding = PaddingValues(0.dp),
                                modifier = Modifier.height(28.dp),
                            ) {
                                Text(user.phone, style = MaterialTheme.typography.bodySmall.tabular())
                            }
                        }
                    }
                }
                if (editable) {
                    if (user.status == "pending") {
                        Button(
                            onClick = onActionClick,
                            colors = graceRidePrimaryButtonColors(),
                        ) {
                            Text(stringResource(R.string.members_approve_btn))
                        }
                    } else {
                        OutlinedButton(onClick = onActionClick) {
                            Text(stringResource(R.string.members_edit_btn))
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun InvitesTab(
    state: AdminMembersUiState,
    me: Me?,
    vm: AdminMembersViewModel,
    onCopied: () -> Unit,
) {
    val context = LocalContext.current
    val view = LocalView.current
    val c = LocalStatusColors.current
    val canCreate = me?.can("admin_invites", "create") == true
    val canDelete = me?.can("admin_invites", "delete") == true
    var revokingTarget by remember { mutableStateOf<AdminInviteDto?>(null) }
    val shareTemplate = stringResource(R.string.members_invite_share_text)

    val invitesMode = when {
        state.invitesLoading && state.invites.isEmpty() -> "loading"
        state.invitesError != null && state.invites.isEmpty() -> "error"
        else -> "content"
    }
    Crossfade(
        targetState = invitesMode,
        animationSpec = if (animatorsEnabled()) tween(200) else snap(),
        label = "invites_tab_state",
        modifier = Modifier.fillMaxSize(),
    ) { mode ->
        when (mode) {
            "loading" -> LoadingView()
            "error" -> ErrorView(state.invitesError ?: "", onRetry = { vm.refreshActive(silent = false) })
            else -> PullToRefreshBox(
                isRefreshing = state.refreshing,
                onRefresh = { vm.refreshActive(silent = false) },
                modifier = Modifier.fillMaxSize(),
            ) {
                LazyColumn(
                    Modifier.fillMaxSize(),
                    contentPadding = PaddingValues(Spacing.sm),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    item {
                        OutlinedCard(
                            modifier = Modifier.fillMaxWidth(),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                        ) {
                            Column(
                                Modifier.padding(Spacing.sm),
                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                Text(
                                    stringResource(R.string.members_invite_hint),
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                                if (canCreate) {
                                    Button(
                                        onClick = {
                                            UiHaptics.confirm(view)
                                            vm.createInvite()
                                        },
                                        enabled = !state.inviteBusy,
                                        colors = graceRidePrimaryButtonColors(),
                                        modifier = Modifier.fillMaxWidth(),
                                    ) {
                                        if (state.inviteBusy) {
                                            InlineButtonSpinner()
                                        } else {
                                            Icon(Icons.Outlined.AddLink, contentDescription = null, modifier = Modifier.size(18.dp))
                                        }
                                        Spacer(Modifier.width(Spacing.xs))
                                        Text(stringResource(R.string.members_create_invite))
                                    }
                                }
                            }
                        }
                    }

                    state.lastCreatedInviteUrl?.let { link ->
                        item(key = "invite-banner") {
                            OneTimeLinkBanner(
                                title = stringResource(R.string.members_invite_link_label),
                                url = link,
                                containerColor = c.successSoft,
                                contentColor = c.success,
                                modifier = Modifier.animateItem(),
                                onCopy = {
                                    if (copyToClipboard(context, "Grace Ride Invite", link, sensitive = true)) {
                                        onCopied()
                                    }
                                },
                                onShare = {
                                    shareText(
                                        context = context,
                                        text = "$shareTemplate\n\n$link",
                                        chooserTitle = context.getString(R.string.members_share_btn),
                                    )
                                },
                                onDismiss = { vm.dismissCreatedInviteUrl() },
                            )
                        }
                    }

                    if (state.invites.isEmpty()) {
                        item {
                            EmptyView(
                                text = stringResource(R.string.members_invites_empty),
                                icon = Icons.Outlined.MarkEmailUnread,
                            )
                        }
                    } else {
                        items(state.invites, key = { "invite-${it.id}" }) { invite ->
                            val statusKind = when (invite.status) {
                                "open" -> RideStatusKind.PENDING
                                "used" -> RideStatusKind.ACTIVE
                                else -> RideStatusKind.DONE
                            }
                            val statusLabel = when (invite.status) {
                                "open" -> stringResource(R.string.members_invite_status_open)
                                "used" -> stringResource(R.string.members_invite_status_used)
                                else -> stringResource(R.string.members_invite_status_expired)
                            }
                            val recipient = invite.email.ifBlank { stringResource(R.string.members_open_invite) }
                            val revoking = state.revokingInviteId == invite.id
                            OutlinedCard(
                                modifier = Modifier.fillMaxWidth().animateItem(),
                                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                            ) {
                                Column(
                                    Modifier.padding(Spacing.sm),
                                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                ) {
                                    Row(
                                        Modifier.fillMaxWidth(),
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                            Row(
                                                verticalAlignment = Alignment.CenterVertically,
                                                horizontalArrangement = Arrangement.spacedBy(6.dp),
                                            ) {
                                                Text(
                                                    recipient,
                                                    style = MaterialTheme.typography.titleSmall,
                                                    maxLines = 1,
                                                    overflow = TextOverflow.Ellipsis,
                                                    modifier = Modifier.weight(1f, fill = false),
                                                )
                                                if (invite.kind == "recovery") {
                                                    Surface(
                                                        color = c.infoSoft,
                                                        contentColor = c.info,
                                                        shape = CircleShape,
                                                    ) {
                                                        Text(
                                                            stringResource(R.string.members_tab_recovery),
                                                            style = MaterialTheme.typography.labelSmall,
                                                            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
                                                        )
                                                    }
                                                }
                                            }
                                            val rolesText = if (invite.roles.isEmpty()) {
                                                stringResource(R.string.members_member_default)
                                            } else {
                                                buildList {
                                                    for (roleKey in invite.roles) {
                                                        add(roleDisplayName(roleKey, state.roles))
                                                    }
                                                }.joinToString(" · ")
                                            }
                                            Text(
                                                "$rolesText · ${KlTime.display(invite.createdAt)}",
                                                style = MaterialTheme.typography.bodySmall.tabular(),
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                            )
                                        }
                                        StatusPill(statusLabel, statusKind)
                                    }
                                    if (invite.status == "open") {
                                        Row(
                                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                            modifier = Modifier.fillMaxWidth(),
                                        ) {
                                            invite.url?.takeIf { it.isNotBlank() }?.let { url ->
                                                OutlinedButton(
                                                    onClick = {
                                                        if (copyToClipboard(context, recipient, url, sensitive = true)) {
                                                            onCopied()
                                                        }
                                                    },
                                                    enabled = !revoking,
                                                ) {
                                                    Icon(Icons.Outlined.ContentCopy, contentDescription = null, modifier = Modifier.size(16.dp))
                                                    Spacer(Modifier.width(4.dp))
                                                    Text(stringResource(R.string.common_copy))
                                                }
                                                OutlinedButton(
                                                    onClick = {
                                                        shareText(
                                                            context = context,
                                                            text = "$shareTemplate\n\n$url",
                                                            chooserTitle = context.getString(R.string.members_share_btn),
                                                        )
                                                    },
                                                    enabled = !revoking,
                                                ) {
                                                    Icon(Icons.Outlined.Share, contentDescription = null, modifier = Modifier.size(16.dp))
                                                    Spacer(Modifier.width(4.dp))
                                                    Text(stringResource(R.string.members_share_btn))
                                                }
                                            }
                                            if (canDelete && invite.kind == "invite") {
                                                Spacer(Modifier.weight(1f))
                                                TextButton(
                                                    onClick = { revokingTarget = invite },
                                                    enabled = !revoking,
                                                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                                                ) {
                                                    if (revoking) InlineButtonSpinner()
                                                    else Text(stringResource(R.string.members_revoke_btn))
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
    }

    revokingTarget?.let { target ->
        val label = target.email.ifBlank { stringResource(R.string.members_open_invite) }
        AlertDialog(
            onDismissRequest = { revokingTarget = null },
            title = { Text(stringResource(R.string.members_revoke_btn)) },
            text = { Text(stringResource(R.string.members_revoke_confirm, label)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        vm.revokeInvite(target.id)
                        revokingTarget = null
                    },
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                ) {
                    Text(stringResource(R.string.members_revoke_btn))
                }
            },
            dismissButton = {
                TextButton(onClick = { revokingTarget = null }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }
}

@Composable
private fun RecoveryTab(
    state: AdminMembersUiState,
    me: Me?,
    vm: AdminMembersViewModel,
    onCopied: () -> Unit,
) {
    val context = LocalContext.current
    val c = LocalStatusColors.current
    val canDecide = me?.can("admin_recovery", "update") == true
    var approvingTarget by remember { mutableStateOf<RecoveryRequestDto?>(null) }
    var rejectingTarget by remember { mutableStateOf<RecoveryRequestDto?>(null) }

    val recoveryMode = when {
        state.recoveryLoading && state.recovery.isEmpty() -> "loading"
        state.recoveryError != null && state.recovery.isEmpty() -> "error"
        else -> "content"
    }
    Crossfade(
        targetState = recoveryMode,
        animationSpec = if (animatorsEnabled()) tween(200) else snap(),
        label = "recovery_tab_state",
        modifier = Modifier.fillMaxSize(),
    ) { mode ->
        when (mode) {
            "loading" -> LoadingView()
            "error" -> ErrorView(state.recoveryError ?: "", onRetry = { vm.refreshActive(silent = false) })
            else -> PullToRefreshBox(
                isRefreshing = state.refreshing,
                onRefresh = { vm.refreshActive(silent = false) },
                modifier = Modifier.fillMaxSize(),
            ) {
                LazyColumn(
                    Modifier.fillMaxSize(),
                    contentPadding = PaddingValues(Spacing.sm),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    item {
                        CalloutBanner(
                            icon = Icons.Outlined.Info,
                            text = stringResource(R.string.members_recovery_notice),
                            containerColor = c.infoSoft,
                            contentColor = c.info,
                        )
                    }

                    state.lastRecoveryUrl?.let { url ->
                        item(key = "recovery-banner") {
                            OneTimeLinkBanner(
                                title = stringResource(R.string.members_recovery_link_label),
                                url = url,
                                containerColor = c.successSoft,
                                contentColor = c.success,
                                modifier = Modifier.animateItem(),
                                onCopy = {
                                    if (copyToClipboard(context, "Grace Ride Recovery", url, sensitive = true)) {
                                        onCopied()
                                    }
                                },
                                onShare = {
                                    shareText(
                                        context = context,
                                        text = url,
                                        chooserTitle = context.getString(R.string.members_share_btn),
                                    )
                                },
                                onDismiss = { vm.dismissRecoveryUrl() },
                            )
                        }
                    }

                    if (state.recovery.isEmpty()) {
                        item {
                            EmptyView(
                                text = stringResource(R.string.members_recovery_empty),
                                icon = Icons.Outlined.Key,
                            )
                        }
                    } else {
                        items(state.recovery, key = { "recovery-${it.id}" }) { req ->
                            val busy = state.recoveryBusyId == req.id
                            val pendingAction = req.status == "pending" || req.status == "awaiting_second"
                            val statusKind = when (req.status) {
                                "approved" -> RideStatusKind.ACTIVE
                                "pending", "awaiting_second" -> RideStatusKind.PENDING
                                else -> RideStatusKind.CANCELLED
                            }
                            val statusLabel = when (req.status) {
                                "approved" -> stringResource(R.string.members_recovery_status_approved)
                                "rejected" -> stringResource(R.string.members_recovery_status_rejected)
                                "awaiting_second" -> stringResource(R.string.members_recovery_status_awaiting_second)
                                "superseded" -> stringResource(R.string.members_recovery_status_superseded)
                                else -> stringResource(R.string.members_recovery_status_pending)
                            }
                            OutlinedCard(
                                modifier = Modifier.fillMaxWidth().animateItem(),
                                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                            ) {
                                Column(
                                    Modifier.padding(Spacing.sm),
                                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                ) {
                                    Row(
                                        Modifier.fillMaxWidth(),
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                            Text(
                                                "${req.name ?: stringResource(R.string.members_non_member)} · ${req.email}",
                                                style = MaterialTheme.typography.titleSmall,
                                                maxLines = 2,
                                                overflow = TextOverflow.Ellipsis,
                                            )
                                            Text(
                                                KlTime.display(req.createdAt),
                                                style = MaterialTheme.typography.bodySmall.tabular(),
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                            )
                                        }
                                        StatusPill(statusLabel, statusKind)
                                    }
                                    if (req.reason.isNotBlank()) {
                                        Text(
                                            req.reason,
                                            style = MaterialTheme.typography.bodyMedium,
                                            color = MaterialTheme.colorScheme.onSurface,
                                        )
                                    }
                                    if (req.status == "awaiting_second") {
                                        Text(
                                            stringResource(R.string.members_recovery_second_notice),
                                            style = MaterialTheme.typography.bodySmall,
                                            color = c.warning,
                                        )
                                    }
                                    if (pendingAction) {
                                        Row(
                                            Modifier.fillMaxWidth(),
                                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                        ) {
                                            Button(
                                                onClick = { approvingTarget = req },
                                                enabled = canDecide && req.name != null && !busy && state.recoveryBusyId == null,
                                                colors = graceRidePrimaryButtonColors(),
                                            ) {
                                                if (busy) {
                                                    InlineButtonSpinner()
                                                    Spacer(Modifier.width(Spacing.xs))
                                                }
                                                Text(
                                                    stringResource(
                                                        if (req.status == "awaiting_second") {
                                                            R.string.members_recovery_confirm_second
                                                        } else {
                                                            R.string.members_recovery_approve
                                                        },
                                                    ),
                                                )
                                            }
                                            OutlinedButton(
                                                onClick = { rejectingTarget = req },
                                                enabled = canDecide && !busy && state.recoveryBusyId == null,
                                            ) {
                                                Text(stringResource(R.string.members_recovery_reject))
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

    approvingTarget?.let { req ->
        val who = "${req.name ?: req.email} (${req.email})"
        val second = req.status == "awaiting_second"
        AlertDialog(
            onDismissRequest = { approvingTarget = null },
            title = {
                Text(
                    stringResource(
                        if (second) R.string.members_recovery_confirm_second else R.string.members_recovery_approve,
                    ),
                )
            },
            text = {
                Text(
                    stringResource(
                        if (second) R.string.members_recovery_second_confirm else R.string.members_recovery_approve_confirm,
                        who,
                    ),
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        vm.approveRecovery(req.id)
                        approvingTarget = null
                    },
                ) {
                    Text(
                        stringResource(
                            if (second) R.string.members_recovery_confirm_second else R.string.members_recovery_approve,
                        ),
                    )
                }
            },
            dismissButton = {
                TextButton(onClick = { approvingTarget = null }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }

    rejectingTarget?.let { req ->
        val who = "${req.name ?: req.email} (${req.email})"
        AlertDialog(
            onDismissRequest = { rejectingTarget = null },
            title = { Text(stringResource(R.string.members_recovery_reject)) },
            text = { Text(stringResource(R.string.members_recovery_reject_confirm, who)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        vm.rejectRecovery(req.id)
                        rejectingTarget = null
                    },
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                ) {
                    Text(stringResource(R.string.members_recovery_reject))
                }
            },
            dismissButton = {
                TextButton(onClick = { rejectingTarget = null }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }
}

@Composable
private fun OneTimeLinkBanner(
    title: String,
    url: String,
    containerColor: androidx.compose.ui.graphics.Color,
    contentColor: androidx.compose.ui.graphics.Color,
    onCopy: () -> Unit,
    onShare: () -> Unit,
    onDismiss: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Surface(
        color = containerColor,
        contentColor = contentColor,
        shape = MaterialTheme.shapes.medium,
        border = BorderStroke(1.dp, contentColor.copy(alpha = 0.25f)),
        modifier = modifier.fillMaxWidth(),
    ) {
        Column(
            Modifier.padding(Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Text(title, style = MaterialTheme.typography.titleSmall, color = contentColor)
            Text(
                url,
                style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                color = MaterialTheme.colorScheme.onSurface,
            )
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                OutlinedButton(onClick = onCopy) {
                    Icon(Icons.Outlined.ContentCopy, contentDescription = null, modifier = Modifier.size(16.dp))
                    Spacer(Modifier.width(4.dp))
                    Text(stringResource(R.string.common_copy))
                }
                OutlinedButton(onClick = onShare) {
                    Icon(Icons.Outlined.Share, contentDescription = null, modifier = Modifier.size(16.dp))
                    Spacer(Modifier.width(4.dp))
                    Text(stringResource(R.string.members_share_btn))
                }
                Spacer(Modifier.weight(1f))
                TextButton(onClick = onDismiss) {
                    Text(stringResource(R.string.common_close))
                }
            }
        }
    }
}
