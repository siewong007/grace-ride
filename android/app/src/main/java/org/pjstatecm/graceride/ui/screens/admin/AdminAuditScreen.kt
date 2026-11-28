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
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.OpenInNew
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.Download
import androidx.compose.material.icons.outlined.FilterList
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material.icons.outlined.Shield
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FilledTonalButton
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
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
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
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.AuditItemDto
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.platform.CsvBuilder
import org.pjstatecm.graceride.platform.ExportShare
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.SecureScreenEffect
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.AdaptiveContentFrame
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular
import java.time.LocalDate

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun AdminAuditScreen(
    container: AppContainer,
    nav: NavHostController,
) {
    SecureScreenEffect()
    val vm: AdminAuditViewModel = viewModel(factory = AdminAuditViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val me = (authState as? AuthState.SignedIn)?.me ?: container.auth.me
    val context = LocalContext.current
    val view = LocalView.current
    val scope = rememberCoroutineScope()
    val snackbar = remember { SnackbarHostState() }
    val statusColors = LocalStatusColors.current

    var selectedItem by remember { mutableStateOf<AuditItemDto?>(null) }
    var showFilterSheet by remember { mutableStateOf(false) }

    val exportFailedMsg = stringResource(R.string.audit_export_failed)
    val canView = me?.can("admin_audit") == true
    val activeSheetFilters = listOf(state.entity, state.action).count { it.isNotBlank() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.audit_title), maxLines = 1, overflow = TextOverflow.Ellipsis) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
                actions = {
                    if (canView) {
                        IconButton(
                            onClick = {
                                if (state.items.isNotEmpty()) {
                                    UiHaptics.tick(view)
                                    scope.launch {
                                        val bytes = CsvBuilder.buildBytes(
                                            headers = listOf("ID", "Time", "Actor", "Action", "Entity", "Entity ID", "Entity Name", "Detail"),
                                            rows = state.items.map { a ->
                                                listOf(
                                                    a.id.toString(),
                                                    a.createdAt,
                                                    a.actor ?: "System",
                                                    a.action,
                                                    a.entity,
                                                    a.entityId?.toString().orEmpty(),
                                                    a.entityName.orEmpty(),
                                                    a.detail,
                                                )
                                            },
                                        )
                                        ExportShare.writeAndShareFile(context, "grace-ride-audit.csv", "text/csv", bytes)
                                    }
                                }
                            },
                            enabled = state.items.isNotEmpty(),
                        ) {
                            Icon(Icons.Outlined.Download, contentDescription = stringResource(R.string.common_export_csv))
                        }
                        IconButton(onClick = { UiHaptics.tick(view); vm.refresh() }) {
                            Icon(Icons.Outlined.Refresh, contentDescription = stringResource(R.string.common_refresh))
                        }
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        if (!canView) {
            Box(Modifier.fillMaxSize().padding(padding)) {
                EmptyView(text = stringResource(R.string.members_no_permission), icon = Icons.Outlined.Shield)
            }
            return@Scaffold
        }

        AdaptiveContentFrame(maxWidth = 920.dp, compactHorizontalPadding = 0.dp, modifier = Modifier.fillMaxSize().padding(padding)) {
            LazyColumn(
                modifier = Modifier.fillMaxSize().navigationBarsPadding(),
                contentPadding = PaddingValues(Spacing.sm),
                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                item {
                    Row(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        OutlinedTextField(
                            value = state.q,
                            onValueChange = vm::setQuery,
                            placeholder = { Text(stringResource(R.string.audit_search_hint), maxLines = 1, overflow = TextOverflow.Ellipsis) },
                            leadingIcon = { Icon(Icons.Outlined.Search, contentDescription = null) },
                            trailingIcon = if (state.q.isNotEmpty()) {
                                {
                                    IconButton(onClick = { vm.setQuery("") }) {
                                        Icon(Icons.Outlined.Close, contentDescription = stringResource(R.string.a11y_clear))
                                    }
                                }
                            } else null,
                            singleLine = true,
                            modifier = Modifier.weight(1f),
                        )
                        FilledTonalButton(
                            onClick = {
                                UiHaptics.tick(view)
                                showFilterSheet = true
                            },
                            modifier = Modifier.defaultMinSize(minHeight = 52.dp),
                        ) {
                            Icon(Icons.Outlined.FilterList, contentDescription = null, modifier = Modifier.size(18.dp))
                            Spacer(Modifier.width(Spacing.xxs))
                            Text(
                                if (activeSheetFilters > 0) stringResource(R.string.audit_filters_active, activeSheetFilters)
                                else stringResource(R.string.audit_filters_btn),
                            )
                        }
                    }
                }

                item {
                    val todayStr = KlTime.today()
                    val today = remember(todayStr) { LocalDate.parse(todayStr) }
                    val yesterday = remember(today) { today.minusDays(1) }
                    val last7 = remember(today) { today.minusDays(6) }
                    val monthStart = remember(today) { today.withDayOfMonth(1) }

                    LazyRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        item {
                            FilterChip(
                                selected = state.fromDate == today.toString() && state.toDate == today.toString(),
                                onClick = { UiHaptics.tick(view); vm.setFilter(fromDate = today.toString(), toDate = today.toString()) },
                                label = { Text(stringResource(R.string.audit_today)) },
                            )
                        }
                        item {
                            FilterChip(
                                selected = state.fromDate == yesterday.toString() && state.toDate == yesterday.toString(),
                                onClick = { UiHaptics.tick(view); vm.setFilter(fromDate = yesterday.toString(), toDate = yesterday.toString()) },
                                label = { Text(stringResource(R.string.audit_yesterday)) },
                            )
                        }
                        item {
                            FilterChip(
                                selected = state.fromDate == last7.toString() && state.toDate == today.toString(),
                                onClick = { UiHaptics.tick(view); vm.setFilter(fromDate = last7.toString(), toDate = today.toString()) },
                                label = { Text(stringResource(R.string.audit_last_7_days)) },
                            )
                        }
                        item {
                            FilterChip(
                                selected = state.fromDate == monthStart.toString() && state.toDate == today.toString(),
                                onClick = { UiHaptics.tick(view); vm.setFilter(fromDate = monthStart.toString(), toDate = today.toString()) },
                                label = { Text(stringResource(R.string.audit_this_month)) },
                            )
                        }
                        item {
                            FilterChip(
                                selected = state.q == "failed",
                                onClick = { UiHaptics.tick(view); vm.toggleSecurityOnly() },
                                label = { Text(stringResource(R.string.audit_security_only)) },
                            )
                        }
                        if (state.hasFilters) {
                            item {
                                TextButton(
                                    onClick = { UiHaptics.tick(view); vm.clearFilters() },
                                    modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                                ) {
                                    Text(stringResource(R.string.audit_clear_filters))
                                }
                            }
                        }
                    }
                }

                item {
                    FlowRow(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                        itemVerticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            stringResource(
                                R.string.audit_stats_summary,
                                state.items.size,
                                state.securityCount,
                                state.escalationCount,
                            ),
                            style = MaterialTheme.typography.bodySmall.tabular(),
                        )
                        OutlinedButton(
                            onClick = {
                                UiHaptics.tick(view)
                                vm.downloadStreamCsv { res ->
                                    res.onSuccess { bytes ->
                                        scope.launch {
                                            ExportShare.writeAndShareFile(
                                                context = context,
                                                filename = "grace-ride-audit-full.csv",
                                                mimeType = "text/csv",
                                                bytes = bytes,
                                            )
                                        }
                                    }.onFailure { err ->
                                        scope.launch { snackbar.showSnackbar(err.message ?: exportFailedMsg) }
                                    }
                                }
                            },
                            enabled = !state.exportingStream,
                            modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                        ) {
                            if (state.exportingStream) {
                                InlineButtonSpinner()
                                Spacer(Modifier.width(Spacing.xxs))
                            }
                            Text(stringResource(R.string.audit_export_full_csv))
                        }
                    }
                }

                when {
                    state.loading && state.items.isEmpty() -> item { LoadingView() }
                    state.error != null && state.items.isEmpty() -> item {
                        ErrorView(state.error ?: "", onRetry = vm::refresh)
                    }
                    state.items.isEmpty() -> item { EmptyView(text = stringResource(R.string.audit_none)) }
                    else -> {
                        items(state.items, key = { it.id }) { item ->
                            val kind = when (item.action) {
                                in AdminAuditUiState.SECURITY_ACTIONS -> RideStatusKind.CANCELLED
                                in AdminAuditUiState.WARNING_ACTIONS -> RideStatusKind.PENDING
                                in AdminAuditUiState.INFO_ACTIONS -> RideStatusKind.ACTIVE
                                else -> RideStatusKind.DONE
                            }
                            val stripeColor = when (kind) {
                                RideStatusKind.CANCELLED -> statusColors.danger
                                RideStatusKind.PENDING -> statusColors.warning
                                RideStatusKind.ACTIVE -> statusColors.success
                                RideStatusKind.DONE -> statusColors.info
                            }
                            OutlinedCard(
                                onClick = { UiHaptics.tick(view); selectedItem = item },
                                modifier = Modifier.fillMaxWidth(),
                                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                            ) {
                                Row(Modifier.fillMaxWidth().height(IntrinsicSize.Min)) {
                                    Box(Modifier.width(4.dp).fillMaxHeight().background(stripeColor))
                                    Column(Modifier.weight(1f).padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                        Row(
                                            Modifier.fillMaxWidth(),
                                            horizontalArrangement = Arrangement.SpaceBetween,
                                            verticalAlignment = Alignment.CenterVertically,
                                        ) {
                                            StatusPill(text = item.action, kind = kind)
                                            Text(
                                                KlTime.display(item.createdAt),
                                                style = MaterialTheme.typography.labelSmall.tabular(),
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                            )
                                        }
                                        Text(
                                            buildString {
                                                append(item.actor ?: stringResource(R.string.audit_system_actor))
                                                append(" · ")
                                                append(item.entity)
                                                if (item.entityId != null) append(" #${item.entityId}")
                                                if (!item.entityName.isNullOrBlank()) append(" (${item.entityName})")
                                            },
                                            style = MaterialTheme.typography.titleSmall,
                                        )
                                        if (item.detail.isNotBlank()) {
                                            Text(
                                                item.detail,
                                                style = MaterialTheme.typography.bodySmall,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                maxLines = 2,
                                                overflow = TextOverflow.Ellipsis,
                                            )
                                        }
                                    }
                                }
                            }
                        }

                        if (state.nextBefore != null) {
                            item {
                                Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
                                    OutlinedButton(
                                        onClick = vm::loadMore,
                                        enabled = !state.loadingMore,
                                        modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                                    ) {
                                        if (state.loadingMore) {
                                            InlineButtonSpinner()
                                            Spacer(Modifier.width(Spacing.xxs))
                                        }
                                        Text(stringResource(R.string.common_load_more))
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if (showFilterSheet) {
        val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
        var draftEntity by remember(showFilterSheet) { mutableStateOf(state.entity) }
        var draftAction by remember(showFilterSheet) { mutableStateOf(state.action) }
        var entityExpanded by remember { mutableStateOf(false) }
        var actionExpanded by remember { mutableStateOf(false) }

        ModalBottomSheet(onDismissRequest = { showFilterSheet = false }, sheetState = sheetState) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .navigationBarsPadding()
                    .imePadding()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = Spacing.sm)
                    .padding(bottom = Spacing.md),
                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
            ) {
                Text(stringResource(R.string.audit_filters_btn), style = MaterialTheme.typography.titleMedium)

                ExposedDropdownMenuBox(
                    expanded = entityExpanded,
                    onExpandedChange = { entityExpanded = !entityExpanded },
                ) {
                    OutlinedTextField(
                        value = draftEntity.ifBlank { stringResource(R.string.audit_all_objects) },
                        onValueChange = {},
                        readOnly = true,
                        singleLine = true,
                        label = { Text(stringResource(R.string.audit_object)) },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(entityExpanded) },
                        modifier = Modifier.menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
                    )
                    ExposedDropdownMenu(expanded = entityExpanded, onDismissRequest = { entityExpanded = false }) {
                        DropdownMenuItem(
                            text = { Text(stringResource(R.string.audit_all_objects)) },
                            onClick = { draftEntity = ""; entityExpanded = false },
                        )
                        AdminAuditUiState.ENTITIES.forEach { ent ->
                            DropdownMenuItem(
                                text = { Text(ent) },
                                onClick = { draftEntity = ent; entityExpanded = false },
                            )
                        }
                    }
                }

                ExposedDropdownMenuBox(
                    expanded = actionExpanded,
                    onExpandedChange = { actionExpanded = !actionExpanded },
                ) {
                    OutlinedTextField(
                        value = draftAction.ifBlank { stringResource(R.string.audit_all_actions) },
                        onValueChange = {},
                        readOnly = true,
                        singleLine = true,
                        label = { Text(stringResource(R.string.audit_action)) },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(actionExpanded) },
                        modifier = Modifier.menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
                    )
                    ExposedDropdownMenu(expanded = actionExpanded, onDismissRequest = { actionExpanded = false }) {
                        DropdownMenuItem(
                            text = { Text(stringResource(R.string.audit_all_actions)) },
                            onClick = { draftAction = ""; actionExpanded = false },
                        )
                        AdminAuditUiState.ACTION_GROUPS.flatMap { it.second }.forEach { act ->
                            DropdownMenuItem(
                                text = { Text(act) },
                                onClick = { draftAction = act; actionExpanded = false },
                            )
                        }
                    }
                }

                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    OutlinedButton(
                        onClick = {
                            UiHaptics.tick(view)
                            vm.setFilter(entity = "", entityId = "", action = "")
                            showFilterSheet = false
                        },
                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                    ) {
                        Text(stringResource(R.string.audit_clear_filters))
                    }
                    Button(
                        onClick = {
                            UiHaptics.confirm(view)
                            vm.setFilter(
                                entity = draftEntity,
                                entityId = if (draftEntity == state.entity) state.entityId else "",
                                action = draftAction,
                            )
                            showFilterSheet = false
                        },
                        colors = graceRidePrimaryButtonColors(),
                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                    ) {
                        Text(stringResource(R.string.audit_apply_filters))
                    }
                }
            }
        }
    }

    selectedItem?.let { item ->
        val route = remember(item) { auditObjectRoute(item) }
        val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
        ModalBottomSheet(onDismissRequest = { selectedItem = null }, sheetState = sheetState) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .navigationBarsPadding()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = Spacing.sm)
                    .padding(bottom = Spacing.md),
                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                Row(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text("${item.action} (#${item.id})", style = MaterialTheme.typography.titleMedium)
                    Text(
                        KlTime.display(item.createdAt),
                        style = MaterialTheme.typography.labelSmall.tabular(),
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Text(
                    "${item.actor ?: stringResource(R.string.audit_system_actor)} · ${item.entity}${item.entityId?.let { " #$it" } ?: ""}${item.entityName?.let { " · $it" } ?: ""}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)

                val parts = remember(item.detail) {
                    item.detail.split(';').map { it.trim() }.filter { it.isNotEmpty() }
                }
                if (parts.isEmpty()) {
                    Text("-", style = MaterialTheme.typography.bodySmall)
                } else {
                    parts.forEach { part ->
                        val diffMatch = Regex("^(.*?)(?:→|->)(.*)$").find(part)
                        if (diffMatch != null) {
                            Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                Surface(
                                    color = statusColors.dangerSoft,
                                    contentColor = statusColors.danger,
                                    shape = MaterialTheme.shapes.small,
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Text(
                                        "− ${diffMatch.groupValues[1].trim()}",
                                        style = MaterialTheme.typography.bodySmall.copy(
                                            fontFamily = FontFamily.Monospace,
                                            textDecoration = TextDecoration.LineThrough,
                                        ),
                                        modifier = Modifier.padding(horizontal = Spacing.xs, vertical = 6.dp),
                                    )
                                }
                                Surface(
                                    color = statusColors.successSoft,
                                    contentColor = statusColors.success,
                                    shape = MaterialTheme.shapes.small,
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Text(
                                        "+ ${diffMatch.groupValues[2].trim()}",
                                        style = MaterialTheme.typography.bodySmall.copy(
                                            fontFamily = FontFamily.Monospace,
                                            fontWeight = FontWeight.Bold,
                                        ),
                                        modifier = Modifier.padding(horizontal = Spacing.xs, vertical = 6.dp),
                                    )
                                }
                            }
                        } else {
                            Surface(
                                color = MaterialTheme.colorScheme.surfaceContainerLow,
                                shape = MaterialTheme.shapes.small,
                                modifier = Modifier.fillMaxWidth(),
                            ) {
                                Text(
                                    part,
                                    style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                                    modifier = Modifier.padding(horizontal = Spacing.xs, vertical = 6.dp),
                                )
                            }
                        }
                    }
                }

                Spacer(Modifier.height(Spacing.xs))
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    item.actor?.takeIf { it.isNotBlank() }?.let { actorName ->
                        OutlinedButton(
                            onClick = {
                                UiHaptics.tick(view)
                                selectedItem = null
                                vm.filterByActor(actorName)
                            },
                            modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                        ) {
                            Text(stringResource(R.string.audit_filter_by_actor), maxLines = 1, overflow = TextOverflow.Ellipsis)
                        }
                    }
                    if (route != null) {
                        Button(
                            onClick = {
                                UiHaptics.tick(view)
                                selectedItem = null
                                nav.navigate(route)
                            },
                            colors = graceRidePrimaryButtonColors(),
                            modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                        ) {
                            Icon(Icons.AutoMirrored.Outlined.OpenInNew, contentDescription = null, modifier = Modifier.size(16.dp))
                            Spacer(Modifier.width(4.dp))
                            Text(stringResource(R.string.audit_view_object))
                        }
                    } else {
                        OutlinedButton(
                            onClick = { selectedItem = null },
                            modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                        ) {
                            Text(stringResource(R.string.common_close))
                        }
                    }
                }
            }
        }
    }
}

private fun auditObjectRoute(item: AuditItemDto): AppRoute? {
    val id = item.entityId
    return when (item.entity) {
        "ride" -> id?.let { AppRoute.RideDetail(it) }
        "event" -> id?.let { AppRoute.EventDetail(it) }
        "help_ticket" -> id?.let { AppRoute.HelpDetail(it) }
        "settlement" -> id?.let { AppRoute.SettlementDetail(it) }
        "user" -> AppRoute.AdminMembers()
        "reimbursement" -> AppRoute.AdminReimbursements()
        "group", "area" -> AppRoute.AdminSettings()
        "role" -> AppRoute.AdminRoles
        "announcement" -> AppRoute.Announcements(manage = true)
        else -> null
    }
}
