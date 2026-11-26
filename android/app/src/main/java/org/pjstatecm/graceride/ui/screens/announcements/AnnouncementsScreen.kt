package org.pjstatecm.graceride.ui.screens.announcements

import androidx.compose.animation.Crossfade
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.snap
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
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
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.Campaign
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.Delete
import androidx.compose.material.icons.outlined.Edit
import androidx.compose.material.icons.outlined.Info
import androidx.compose.material.icons.outlined.PushPin
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material.icons.outlined.Visibility
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
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
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.Announcement
import org.pjstatecm.graceride.data.api.GroupDto
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.GroupTree
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.GraceRideSpringEasing
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.animatorsEnabled
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AnnouncementsScreen(
    container: AppContainer,
    nav: NavHostController,
    manage: Boolean = false,
) {
    val vm: AnnouncementsViewModel = viewModel(
        factory = AnnouncementsViewModel.factory(container, manage),
    )
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val currentMe = (authState as? AuthState.SignedIn)?.me ?: state.me
    val snackbar = remember { SnackbarHostState() }
    val view = LocalView.current

    val canManageView = currentMe?.can("admin_announcements") == true
    val canCreate = currentMe?.can("admin_announcements", "create") == true
    val canDelete = currentMe?.can("admin_announcements", "delete") == true

    var editorOpen by remember { mutableStateOf(false) }
    var editingItem by remember { mutableStateOf<Announcement?>(null) }
    var deletingItem by remember { mutableStateOf<Announcement?>(null) }
    var expandedIds by remember { mutableStateOf(emptySet<Long>()) }

    LaunchedEffect(currentMe) { vm.syncMe(currentMe) }

    val publishedMsg = stringResource(R.string.announcements_published_msg)
    val updatedMsg = stringResource(R.string.announcements_updated_msg)
    val pinnedMsg = stringResource(R.string.announcements_pinned_msg)
    val unpinnedMsg = stringResource(R.string.announcements_unpinned_msg)
    val deletedMsg = stringResource(R.string.announcements_deleted_msg)

    LaunchedEffect(state.notice, state.actionError) {
        val msg = when (state.notice) {
            AnnouncementNotice.PUBLISHED -> publishedMsg
            AnnouncementNotice.UPDATED -> updatedMsg
            AnnouncementNotice.PINNED -> pinnedMsg
            AnnouncementNotice.UNPINNED -> unpinnedMsg
            AnnouncementNotice.DELETED -> deletedMsg
            null -> if (!editorOpen) state.actionError else null
        }
        if (!msg.isNullOrBlank()) {
            snackbar.showSnackbar(msg)
            vm.clearNotice()
        }
    }

    val filtered = remember(state.items, state.query, state.pinnedOnly) {
        val q = state.query.trim().lowercase()
        state.items.filter { a ->
            (!state.pinnedOnly || a.pinned) &&
                (q.isEmpty() ||
                    a.title.lowercase().contains(q) ||
                    a.body.lowercase().contains(q) ||
                    a.by.lowercase().contains(q))
        }
    }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        snackbarHost = { SnackbarHost(snackbar) },
        topBar = {
            TopAppBar(
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background),
                title = {
                    Text(
                        stringResource(R.string.announcements_title),
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
                    IconButton(onClick = { vm.refresh() }) {
                        Icon(
                            Icons.Outlined.Refresh,
                            contentDescription = stringResource(R.string.common_refresh),
                        )
                    }
                },
            )
        },
        floatingActionButton = {
            if (canCreate) {
                ExtendedFloatingActionButton(
                    onClick = {
                        vm.clearActionError()
                        editingItem = null
                        editorOpen = true
                    },
                    containerColor = MaterialTheme.colorScheme.primary,
                    contentColor = MaterialTheme.colorScheme.onPrimary,
                ) {
                    Icon(Icons.Outlined.Campaign, contentDescription = null)
                    Spacer(Modifier.width(Spacing.xs))
                    Text(stringResource(R.string.announcements_publish_fab))
                }
            }
        },
    ) { padding ->
        Column(
            Modifier
                .fillMaxSize()
                .padding(padding),
        ) {
            OutlinedTextField(
                value = state.query,
                onValueChange = { vm.setQuery(it) },
                placeholder = {
                    Text(
                        stringResource(R.string.announcements_search_hint),
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                },
                leadingIcon = { Icon(Icons.Outlined.Search, contentDescription = null) },
                trailingIcon = {
                    if (state.query.isNotEmpty()) {
                        IconButton(onClick = { vm.setQuery("") }) {
                            Icon(Icons.Outlined.Close, contentDescription = stringResource(R.string.a11y_clear))
                        }
                    }
                },
                singleLine = true,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
            )

            LazyRow(
                contentPadding = PaddingValues(horizontal = Spacing.sm),
                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                item {
                    FilterChip(
                        selected = !state.pinnedOnly,
                        onClick = {
                            UiHaptics.tick(view)
                            vm.setPinnedOnly(false)
                        },
                        label = { Text(stringResource(R.string.announcements_filter_all)) },
                    )
                }
                item {
                    FilterChip(
                        selected = state.pinnedOnly,
                        onClick = {
                            UiHaptics.tick(view)
                            vm.setPinnedOnly(true)
                        },
                        leadingIcon = { Icon(Icons.Outlined.PushPin, contentDescription = null, modifier = Modifier.size(16.dp)) },
                        label = { Text(stringResource(R.string.announcements_filter_pinned)) },
                    )
                }
                if (canManageView) {
                    item {
                        FilterChip(
                            selected = state.manageMode,
                            onClick = {
                                UiHaptics.tick(view)
                                vm.setManageMode(!state.manageMode)
                            },
                            leadingIcon = { Icon(Icons.Outlined.Visibility, contentDescription = null, modifier = Modifier.size(16.dp)) },
                            label = { Text(stringResource(R.string.announcements_manage_badge)) },
                        )
                    }
                }
            }

            val viewMode = when {
                state.loading && state.items.isEmpty() -> "loading"
                state.error != null && state.items.isEmpty() -> "error"
                else -> "content"
            }
            Crossfade(
                targetState = viewMode,
                animationSpec = if (animatorsEnabled()) tween(200) else snap(),
                label = "announcements_state",
                modifier = Modifier.weight(1f),
            ) { mode ->
                when (mode) {
                    "loading" -> LoadingView()
                    "error" -> ErrorView(
                        message = state.error ?: "",
                        onRetry = { vm.refresh() },
                    )
                    else -> PullToRefreshBox(
                        isRefreshing = state.refreshing,
                        onRefresh = { vm.refresh() },
                        modifier = Modifier.fillMaxSize(),
                    ) {
                        LazyColumn(
                            Modifier.fillMaxSize(),
                            contentPadding = PaddingValues(
                                start = Spacing.sm,
                                end = Spacing.sm,
                                top = Spacing.xs,
                                bottom = if (canCreate) 88.dp else Spacing.lg,
                            ),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            if (filtered.isEmpty()) {
                                item {
                                    EmptyView(
                                        text = stringResource(R.string.announcements_empty),
                                        subtitle = if (canCreate) stringResource(R.string.announcements_empty_manage) else null,
                                        icon = Icons.Outlined.Campaign,
                                    )
                                }
                            } else {
                                items(filtered, key = { "ann-${it.id}" }) { ann ->
                                    AnnouncementItemCard(
                                        ann = ann,
                                        expanded = ann.id in expandedIds,
                                        canCreate = canCreate,
                                        canDelete = canDelete,
                                        busy = state.busyId == ann.id,
                                        modifier = Modifier.animateItem(),
                                        onToggleExpand = {
                                            expandedIds = if (ann.id in expandedIds) {
                                                expandedIds - ann.id
                                            } else {
                                                expandedIds + ann.id
                                            }
                                        },
                                        onEdit = {
                                            vm.clearActionError()
                                            editingItem = ann
                                            editorOpen = true
                                        },
                                        onTogglePin = { vm.togglePin(ann) },
                                        onDelete = { deletingItem = ann },
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if (editorOpen) {
        AnnouncementEditorSheet(
            editing = editingItem,
            groups = state.groups,
            authorName = currentMe?.name ?: "",
            busy = state.busy,
            error = state.actionError,
            onCreate = { title, body, pinned, notifyAll, groupIds ->
                vm.createAnnouncement(
                    title = title,
                    body = body,
                    pinned = pinned,
                    notifyAll = notifyAll,
                    groupIds = groupIds,
                    onSuccess = {
                        editorOpen = false
                        editingItem = null
                    },
                )
            },
            onUpdate = { item, title, body, pinned, groupIds ->
                vm.updateAnnouncement(
                    item = item,
                    title = title,
                    body = body,
                    pinned = pinned,
                    groupIds = groupIds,
                    onSuccess = {
                        editorOpen = false
                        editingItem = null
                    },
                )
            },
            onDismiss = {
                vm.clearActionError()
                editorOpen = false
                editingItem = null
            },
        )
    }

    deletingItem?.let { target ->
        AlertDialog(
            onDismissRequest = { deletingItem = null },
            title = { Text(stringResource(R.string.common_delete)) },
            text = { Text(stringResource(R.string.announcements_delete_confirm, target.title)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        vm.deleteAnnouncement(target.id)
                        deletingItem = null
                    },
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                ) {
                    Text(stringResource(R.string.common_delete))
                }
            },
            dismissButton = {
                TextButton(onClick = { deletingItem = null }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun AnnouncementItemCard(
    ann: Announcement,
    expanded: Boolean,
    canCreate: Boolean,
    canDelete: Boolean,
    busy: Boolean,
    onToggleExpand: () -> Unit,
    onEdit: () -> Unit,
    onTogglePin: () -> Unit,
    onDelete: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val isLong = ann.body.length > 180 || ann.body.count { it == '\n' } >= 4
    val groupNames = remember(ann.groups) { ann.groups.joinToString(", ") { it.name } }
    val hiddenAudience = ann.targeted && ann.groups.isEmpty()

    OutlinedCard(
        modifier = modifier.fillMaxWidth(),
        border = BorderStroke(
            width = if (ann.pinned) 1.5.dp else 1.dp,
            color = if (ann.pinned) MaterialTheme.colorScheme.primary.copy(alpha = 0.5f) else MaterialTheme.colorScheme.outlineVariant,
        ),
    ) {
        Column(
            Modifier
                .padding(Spacing.sm)
                .animateContentSize(
                    animationSpec = if (animatorsEnabled()) tween(200, easing = GraceRideSpringEasing) else snap(),
                ),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Row(
                Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                if (ann.pinned) {
                    Icon(
                        Icons.Outlined.PushPin,
                        contentDescription = stringResource(R.string.announcements_pinned_label),
                        tint = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.size(16.dp),
                    )
                }
                Text(
                    ann.title,
                    style = MaterialTheme.typography.titleMedium,
                    color = MaterialTheme.colorScheme.onSurface,
                    modifier = Modifier.weight(1f),
                )
            }

            if (ann.body.isNotBlank()) {
                Text(
                    ann.body,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = if (expanded || !isLong) Int.MAX_VALUE else 4,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            if (isLong) {
                Text(
                    stringResource(if (expanded) R.string.common_show_less else R.string.common_read_more),
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.clickable { onToggleExpand() },
                )
            }

            FlowRow(
                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
            ) {
                AudiencePill(names = groupNames, hidden = hiddenAudience)
                Text(
                    listOfNotNull(
                        ann.by.takeIf { it.isNotBlank() },
                        ann.createdAt.takeIf { it.isNotBlank() }?.let { KlTime.display(it) },
                    ).joinToString(" · "),
                    style = MaterialTheme.typography.bodySmall.tabular(),
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.align(Alignment.CenterVertically),
                )
            }

            if (canCreate || canDelete) {
                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                Row(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    if (canCreate) {
                        TextButton(onClick = onEdit, enabled = !busy) {
                            Icon(Icons.Outlined.Edit, contentDescription = null, modifier = Modifier.size(16.dp))
                            Spacer(Modifier.width(4.dp))
                            Text(stringResource(R.string.members_edit_btn))
                        }
                        TextButton(onClick = onTogglePin, enabled = !busy) {
                            if (busy) {
                                InlineButtonSpinner()
                            } else {
                                Icon(Icons.Outlined.PushPin, contentDescription = null, modifier = Modifier.size(16.dp))
                            }
                            Spacer(Modifier.width(4.dp))
                            Text(
                                stringResource(
                                    if (ann.pinned) R.string.announcements_unpin else R.string.announcements_pin,
                                ),
                            )
                        }
                    }
                    if (canDelete) {
                        Spacer(Modifier.weight(1f))
                        TextButton(
                            onClick = onDelete,
                            enabled = !busy,
                            colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                        ) {
                            Icon(Icons.Outlined.Delete, contentDescription = null, modifier = Modifier.size(16.dp))
                            Spacer(Modifier.width(4.dp))
                            Text(stringResource(R.string.common_delete))
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun AudiencePill(names: String, hidden: Boolean) {
    val c = LocalStatusColors.current
    val (bg, fg, label) = when {
        hidden -> Triple(c.warningSoft, c.warning, stringResource(R.string.announcements_audience_gone))
        names.isNotBlank() -> Triple(c.infoSoft, c.info, stringResource(R.string.announcements_for_groups, names))
        else -> Triple(
            MaterialTheme.colorScheme.surfaceVariant,
            MaterialTheme.colorScheme.onSurfaceVariant,
            stringResource(R.string.announcements_for_everyone),
        )
    }
    Surface(color = bg, contentColor = fg, shape = CircleShape) {
        Text(
            label,
            style = MaterialTheme.typography.labelMedium,
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp),
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
private fun AnnouncementEditorSheet(
    editing: Announcement?,
    groups: List<GroupDto>,
    authorName: String,
    busy: Boolean,
    error: String?,
    onCreate: (title: String, body: String, pinned: Boolean, notifyAll: Boolean, groupIds: List<Long>) -> Unit,
    onUpdate: (item: Announcement, title: String, body: String, pinned: Boolean, groupIds: List<Long>) -> Unit,
    onDismiss: () -> Unit,
) {
    val view = LocalView.current
    val c = LocalStatusColors.current
    val canDismiss by rememberUpdatedState(!busy)
    val sheetState = rememberModalBottomSheetState(
        skipPartiallyExpanded = true,
        confirmValueChange = { canDismiss },
    )
    val isEdit = editing != null

    var title by remember(editing?.id) { mutableStateOf(editing?.title.orEmpty()) }
    var body by remember(editing?.id) { mutableStateOf(editing?.body.orEmpty()) }
    var pinned by remember(editing?.id) { mutableStateOf(editing?.pinned ?: false) }
    var notifyAll by remember(editing?.id) { mutableStateOf(true) }
    var selectedGroupIds by remember(editing?.id) {
        mutableStateOf(editing?.groups?.map { it.id }?.toSet() ?: emptySet())
    }
    var localError by remember { mutableStateOf<String?>(null) }
    var showNotifyConfirm by remember { mutableStateOf(false) }

    val orderedGroups = remember(groups) {
        GroupTree.treeOrder(GroupTree.ordinary(groups))
    }
    val chosenNames = remember(orderedGroups, selectedGroupIds) {
        orderedGroups.filter { it.group.id in selectedGroupIds }.joinToString(", ") { it.group.name }
    }
    val requiredMsg = stringResource(R.string.announcements_required_error)

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
                stringResource(if (isEdit) R.string.announcements_edit_title else R.string.announcements_create_title),
                style = MaterialTheme.typography.titleLarge,
            )

            if (isEdit) {
                CalloutBanner(
                    icon = Icons.Outlined.Info,
                    text = stringResource(R.string.announcements_edit_hint),
                    containerColor = c.infoSoft,
                    contentColor = c.info,
                )
            }

            OutlinedTextField(
                value = title,
                onValueChange = {
                    title = it
                    localError = null
                },
                label = { Text(stringResource(R.string.announcements_title_label)) },
                singleLine = true,
                enabled = !busy,
                modifier = Modifier.fillMaxWidth(),
            )

            OutlinedTextField(
                value = body,
                onValueChange = {
                    body = it
                    localError = null
                },
                label = { Text(stringResource(R.string.announcements_body_label)) },
                minLines = 4,
                enabled = !busy,
                modifier = Modifier.fillMaxWidth(),
            )

            if (orderedGroups.isNotEmpty()) {
                Column(verticalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
                    Text(
                        stringResource(R.string.announcements_audience_title),
                        style = MaterialTheme.typography.titleSmall,
                    )
                    Text(
                        stringResource(R.string.announcements_audience_hint),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                    ) {
                        orderedGroups.forEach { (g, depth) ->
                            val selected = g.id in selectedGroupIds
                            val prefix = if (depth > 0) "↳ ".repeat(depth) else ""
                            val countSuffix = if (!g.isAll) {
                                " · " + stringResource(R.string.announcements_group_people, g.memberCount)
                            } else {
                                ""
                            }
                            FilterChip(
                                selected = selected,
                                enabled = !busy,
                                onClick = {
                                    UiHaptics.tick(view)
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
                                label = { Text("$prefix${g.name}$countSuffix") },
                            )
                        }
                    }
                }
            }

            Row(
                Modifier
                    .fillMaxWidth()
                    .clickable(enabled = !busy) { pinned = !pinned }
                    .padding(vertical = 2.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text(
                    stringResource(R.string.announcements_pinned_label),
                    style = MaterialTheme.typography.bodyLarge,
                )
                Switch(checked = pinned, onCheckedChange = { pinned = it }, enabled = !busy)
            }

            if (!isEdit) {
                Row(
                    Modifier
                        .fillMaxWidth()
                        .clickable(enabled = !busy) { notifyAll = !notifyAll }
                        .padding(vertical = 2.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Text(
                        stringResource(
                            if (selectedGroupIds.isNotEmpty()) {
                                R.string.announcements_notify_groups
                            } else {
                                R.string.announcements_notify_all
                            },
                        ),
                        style = MaterialTheme.typography.bodyLarge,
                        modifier = Modifier.weight(1f).padding(end = Spacing.xs),
                    )
                    Switch(checked = notifyAll, onCheckedChange = { notifyAll = it }, enabled = !busy)
                }

                // Live member preview
                OutlinedCard(
                    modifier = Modifier.fillMaxWidth(),
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                ) {
                    Column(
                        Modifier.padding(Spacing.sm),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                    ) {
                        Row(
                            Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Text(
                                stringResource(R.string.announcements_preview),
                                style = MaterialTheme.typography.labelMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            AudiencePill(names = chosenNames, hidden = false)
                        }
                        Text(
                            title.ifBlank { stringResource(R.string.announcements_title_label) },
                            style = MaterialTheme.typography.titleSmall,
                        )
                        Text(
                            body.ifBlank { stringResource(R.string.announcements_body_label) },
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            maxLines = 3,
                            overflow = TextOverflow.Ellipsis,
                        )
                        if (authorName.isNotBlank()) {
                            Text(
                                authorName,
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }
            }

            val displayError = localError ?: error
            if (!displayError.isNullOrBlank()) {
                Text(
                    displayError,
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
                        if (title.trim().isEmpty() || body.trim().isEmpty()) {
                            localError = requiredMsg
                            return@Button
                        }
                        if (editing != null) {
                            UiHaptics.confirm(view)
                            onUpdate(editing, title, body, pinned, selectedGroupIds.toList())
                        } else if (notifyAll) {
                            showNotifyConfirm = true
                        } else {
                            UiHaptics.confirm(view)
                            onCreate(title, body, pinned, false, selectedGroupIds.toList())
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
                            if (isEdit) R.string.common_save else R.string.announcements_publish_fab,
                        ),
                    )
                }
            }
            Spacer(Modifier.height(Spacing.xs))
        }
    }

    if (showNotifyConfirm) {
        val hasGroups = selectedGroupIds.isNotEmpty()
        AlertDialog(
            onDismissRequest = { showNotifyConfirm = false },
            title = {
                Text(
                    stringResource(
                        if (hasGroups) {
                            R.string.announcements_notify_groups_confirm_title
                        } else {
                            R.string.announcements_notify_all_confirm_title
                        },
                    ),
                )
            },
            text = {
                Text(
                    if (hasGroups) {
                        stringResource(
                            R.string.announcements_notify_groups_confirm_body,
                            title.trim(),
                            chosenNames,
                        )
                    } else {
                        stringResource(
                            R.string.announcements_notify_all_confirm_body,
                            title.trim(),
                        )
                    },
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        showNotifyConfirm = false
                        UiHaptics.confirm(view)
                        onCreate(title, body, pinned, true, selectedGroupIds.toList())
                    },
                ) {
                    Text(stringResource(R.string.announcements_publish_fab))
                }
            },
            dismissButton = {
                TextButton(onClick = { showNotifyConfirm = false }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }
}
