package org.pjstatecm.graceride.ui.screens.admin

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
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
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material.icons.outlined.DeleteOutline
import androidx.compose.material.icons.outlined.ExpandLess
import androidx.compose.material.icons.outlined.ExpandMore
import androidx.compose.material.icons.outlined.Info
import androidx.compose.material.icons.outlined.Lock
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material.icons.outlined.Shield
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.AdminRoleDefDto
import org.pjstatecm.graceride.data.api.RoleCatalogEntryDto
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.domain.RoleLabels
import org.pjstatecm.graceride.ui.SecureScreenEffect
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun AdminRolesScreen(
    container: AppContainer,
    nav: NavHostController,
) {
    SecureScreenEffect()
    val vm: AdminRolesViewModel = viewModel(factory = AdminRolesViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val me = (authState as? AuthState.SignedIn)?.me ?: container.auth.me
    val snackbar = remember { SnackbarHostState() }
    val view = LocalView.current

    var pendingPickKey by remember { mutableStateOf<String?>(null) }
    var showDeleteConfirm by remember { mutableStateOf(false) }
    val expandedAreas = remember { mutableStateMapOf<String, Boolean>() }

    val createdMsg = stringResource(R.string.roles_created)
    val savedMsg = stringResource(R.string.roles_saved)
    val deletedMsg = stringResource(R.string.roles_deleted)

    LaunchedEffect(state.message) {
        state.message?.let { msg ->
            snackbar.showSnackbar(msg)
            vm.clearMessage()
        }
    }

    val canView = me?.can("admin_roles") == true
    val role = state.selectedRole
    val editable = role?.fixed != true && (
        if (state.isNew) me?.can("admin_roles", "create") == true
        else me?.can("admin_roles", "update") == true
    )

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.roles_title), maxLines = 1, overflow = TextOverflow.Ellipsis) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
                actions = {
                    if (me?.can("admin_roles", "create") == true) {
                        IconButton(
                            onClick = {
                                UiHaptics.tick(view)
                                if (state.dirty) pendingPickKey = AdminRolesUiState.NEW_ROLE_KEY
                                else vm.startNewRole()
                            },
                        ) {
                            Icon(Icons.Outlined.Add, contentDescription = stringResource(R.string.roles_new_role))
                        }
                    }
                    IconButton(onClick = { UiHaptics.tick(view); vm.load() }) {
                        Icon(Icons.Outlined.Refresh, contentDescription = stringResource(R.string.common_refresh))
                    }
                },
            )
        },
        bottomBar = {
            if (canView && editable && !(state.loading && state.roles.isEmpty())) {
                Surface(color = MaterialTheme.colorScheme.surface, modifier = Modifier.fillMaxWidth()) {
                    Column(Modifier.navigationBarsPadding().imePadding()) {
                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                        Row(
                            Modifier
                                .fillMaxWidth()
                                .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            if (state.dirty) {
                                StatusPill(text = stringResource(R.string.roles_unsaved_changes), kind = RideStatusKind.PENDING)
                            }
                            if (!state.isNew && role != null && !role.system && me.can("admin_roles", "delete")) {
                                OutlinedButton(
                                    onClick = {
                                        UiHaptics.tick(view)
                                        showDeleteConfirm = true
                                    },
                                    enabled = !state.busy,
                                    colors = ButtonDefaults.outlinedButtonColors(contentColor = MaterialTheme.colorScheme.error),
                                    modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                                ) {
                                    Icon(Icons.Outlined.DeleteOutline, contentDescription = stringResource(R.string.common_delete), modifier = Modifier.size(18.dp))
                                }
                            }
                            Button(
                                onClick = {
                                    UiHaptics.confirm(view)
                                    vm.saveCurrentRole(createdMsg, savedMsg)
                                },
                                enabled = !state.busy && (role?.system == true || state.draftName.trim().isNotEmpty()),
                                colors = graceRidePrimaryButtonColors(),
                                modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                            ) {
                                if (state.busy) {
                                    InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                    Spacer(Modifier.width(Spacing.xs))
                                }
                                Text(stringResource(if (state.isNew) R.string.roles_create else R.string.roles_save))
                            }
                        }
                    }
                }
            }
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        if (!canView) {
            Box(Modifier.fillMaxSize().padding(padding)) {
                EmptyView(text = stringResource(R.string.members_no_permission), icon = Icons.Outlined.Shield)
            }
            return@Scaffold
        }

        when {
            state.loading && state.roles.isEmpty() -> Box(Modifier.fillMaxSize().padding(padding)) { LoadingView() }
            state.error != null && state.roles.isEmpty() -> Box(Modifier.fillMaxSize().padding(padding)) {
                ErrorView(state.error ?: "", onRetry = { vm.load() })
            }
            else -> {
                val groupedCatalog = remember(state.catalog) {
                    state.catalog.groupBy { it.area.ifBlank { "other" } }
                }
                LazyColumn(
                    modifier = Modifier.fillMaxSize().padding(padding).imePadding(),
                    contentPadding = PaddingValues(Spacing.sm),
                    verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                ) {
                    item {
                        LazyRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            if (state.isNew) {
                                item {
                                    FilterChip(
                                        selected = true,
                                        onClick = {},
                                        label = { Text(stringResource(R.string.roles_new_role)) },
                                    )
                                }
                            }
                            items(state.roles, key = { it.key }) { r ->
                                val label = roleDisplayLabel(r)
                                val sub = if (r.implicit) stringResource(R.string.roles_everyone)
                                else stringResource(R.string.roles_member_count, r.members)
                                FilterChip(
                                    selected = !state.isNew && state.selectedKey == r.key,
                                    onClick = {
                                        if (state.selectedKey != r.key) {
                                            UiHaptics.tick(view)
                                            if (state.dirty) pendingPickKey = r.key else vm.selectRole(r.key)
                                        }
                                    },
                                    label = { Text("$label ($sub)") },
                                )
                            }
                        }
                    }

                    role?.let { r ->
                        item {
                            Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                val bannerBg = MaterialTheme.colorScheme.secondaryContainer
                                val bannerFg = MaterialTheme.colorScheme.onSecondaryContainer
                                if (r.implicit) {
                                    CalloutBanner(
                                        icon = Icons.Outlined.Info,
                                        text = stringResource(R.string.roles_everyone_hint),
                                        containerColor = bannerBg,
                                        contentColor = bannerFg,
                                    )
                                }
                                if (r.fixed) {
                                    CalloutBanner(
                                        icon = Icons.Outlined.Shield,
                                        text = stringResource(R.string.roles_fixed_hint),
                                        containerColor = bannerBg,
                                        contentColor = bannerFg,
                                    )
                                }
                                if (!editable && !r.fixed) {
                                    CalloutBanner(
                                        icon = Icons.Outlined.Info,
                                        text = stringResource(R.string.roles_read_only),
                                        containerColor = bannerBg,
                                        contentColor = bannerFg,
                                    )
                                }
                            }
                        }
                    }

                    state.error?.let { err ->
                        item {
                            Text(err, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                        }
                    }

                    if (state.isNew || role?.system == false) {
                        item {
                            OutlinedCard(Modifier.fillMaxWidth()) {
                                Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                    OutlinedTextField(
                                        value = state.draftName,
                                        onValueChange = vm::setDraftName,
                                        label = { Text(stringResource(R.string.roles_name)) },
                                        placeholder = { Text(stringResource(R.string.roles_name_placeholder)) },
                                        enabled = editable && !state.busy,
                                        singleLine = true,
                                        modifier = Modifier.fillMaxWidth(),
                                    )
                                    OutlinedTextField(
                                        value = state.draftDescription,
                                        onValueChange = vm::setDraftDescription,
                                        label = { Text(stringResource(R.string.roles_description)) },
                                        enabled = editable && !state.busy,
                                        maxLines = 3,
                                        modifier = Modifier.fillMaxWidth(),
                                    )
                                }
                            }
                        }
                    }

                    groupedCatalog.forEach { (area, pages) ->
                        val isExpanded = expandedAreas[area] ?: true
                        val totalPerms = pages.sumOf { it.actions.size }
                        val grantedPerms = pages.sumOf { p ->
                            p.actions.count { act -> "${p.page}.$act" in state.draftPermissions }
                        }
                        item(key = "area-card-$area") {
                            OutlinedCard(
                                modifier = Modifier.fillMaxWidth(),
                                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                            ) {
                                Column(Modifier.fillMaxWidth()) {
                                    Surface(
                                        onClick = {
                                            UiHaptics.tick(view)
                                            expandedAreas[area] = !isExpanded
                                        },
                                        color = MaterialTheme.colorScheme.surfaceContainerLow,
                                    ) {
                                        Row(
                                            Modifier
                                                .fillMaxWidth()
                                                .defaultMinSize(minHeight = 52.dp)
                                                .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                                            horizontalArrangement = Arrangement.SpaceBetween,
                                            verticalAlignment = Alignment.CenterVertically,
                                        ) {
                                            Text(
                                                text = areaLabel(area),
                                                style = MaterialTheme.typography.titleSmall,
                                                fontWeight = FontWeight.SemiBold,
                                                modifier = Modifier.weight(1f),
                                            )
                                            Row(
                                                verticalAlignment = Alignment.CenterVertically,
                                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                            ) {
                                                StatusPill(
                                                    text = stringResource(R.string.roles_granted_count, grantedPerms, totalPerms),
                                                    kind = if (grantedPerms > 0) RideStatusKind.DONE else RideStatusKind.PENDING,
                                                )
                                                Icon(
                                                    imageVector = if (isExpanded) Icons.Outlined.ExpandLess else Icons.Outlined.ExpandMore,
                                                    contentDescription = null,
                                                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                                                )
                                            }
                                        }
                                    }
                                    if (isExpanded) {
                                        pages.forEachIndexed { idx, entry ->
                                            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                            RolePageSection(
                                                entry = entry,
                                                granted = state.draftPermissions,
                                                locked = role?.locked.orEmpty().toSet(),
                                                editable = editable && !state.busy,
                                                onTogglePage = {
                                                    UiHaptics.tick(view)
                                                    vm.togglePageRow(entry)
                                                },
                                                onTogglePerm = { key ->
                                                    UiHaptics.tick(view)
                                                    vm.togglePermission(key)
                                                },
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }

                    item { Spacer(Modifier.height(Spacing.md)) }
                }
            }
        }
    }

    pendingPickKey?.let { nextKey ->
        AlertDialog(
            onDismissRequest = { pendingPickKey = null },
            title = { Text(stringResource(R.string.roles_discard_title)) },
            text = { Text(stringResource(R.string.roles_discard_body)) },
            confirmButton = {
                Button(
                    onClick = {
                        pendingPickKey = null
                        vm.selectRole(nextKey)
                    },
                ) { Text(stringResource(R.string.roles_discard)) }
            },
            dismissButton = {
                TextButton(onClick = { pendingPickKey = null }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }

    if (showDeleteConfirm && role != null) {
        AlertDialog(
            onDismissRequest = { showDeleteConfirm = false },
            title = { Text(stringResource(R.string.roles_delete)) },
            text = { Text(stringResource(R.string.roles_delete_confirm, roleDisplayLabel(role), role.members)) },
            confirmButton = {
                Button(
                    onClick = {
                        showDeleteConfirm = false
                        vm.deleteCurrentRole(deletedMsg)
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error),
                ) { Text(stringResource(R.string.common_delete)) }
            },
            dismissButton = {
                TextButton(onClick = { showDeleteConfirm = false }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun RolePageSection(
    entry: RoleCatalogEntryDto,
    granted: Set<String>,
    locked: Set<String>,
    editable: Boolean,
    onTogglePage: () -> Unit,
    onTogglePerm: (String) -> Unit,
) {
    Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
        Row(
            Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                entry.page.replace('_', ' ').replaceFirstChar { it.uppercaseChar() },
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.SemiBold,
                modifier = Modifier.weight(1f),
            )
            if (editable) {
                TextButton(
                    onClick = onTogglePage,
                    modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                ) {
                    Text(stringResource(R.string.roles_toggle_row))
                }
            }
        }
        FlowRow(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
            verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
        ) {
            entry.actions.forEach { act ->
                val permKey = "${entry.page}.$act"
                val isLocked = permKey in locked
                val isGranted = permKey in granted
                val label = act.replace('_', ' ').replaceFirstChar { it.uppercaseChar() }
                FilterChip(
                    selected = isGranted,
                    onClick = { onTogglePerm(permKey) },
                    enabled = editable && !isLocked,
                    leadingIcon = when {
                        isLocked -> {
                            { Icon(Icons.Outlined.Lock, contentDescription = stringResource(R.string.roles_locked_hint_short), modifier = Modifier.size(16.dp)) }
                        }
                        isGranted -> {
                            { Icon(Icons.Outlined.Check, contentDescription = null, modifier = Modifier.size(16.dp)) }
                        }
                        else -> null
                    },
                    label = { Text(label) },
                )
            }
        }
    }
}

@Composable
private fun roleDisplayLabel(role: AdminRoleDefDto): String =
    role.name.ifBlank {
        RoleLabels.builtInLabelRes(role.key)?.let { stringResource(it) } ?: role.key
    }

@Composable
private fun areaLabel(area: String): String = when (area) {
    "rides" -> stringResource(R.string.roles_area_rides)
    "finance" -> stringResource(R.string.roles_area_finance)
    "people" -> stringResource(R.string.roles_area_people)
    "admin" -> stringResource(R.string.roles_area_admin)
    else -> area.replaceFirstChar { it.uppercaseChar() }
}
