package org.pjstatecm.graceride.ui.screens.traffic

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.snap
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandHorizontally
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkHorizontally
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
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
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.Accessible
import androidx.compose.material.icons.automirrored.outlined.Undo
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.AutoFixHigh
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.DeleteOutline
import androidx.compose.material.icons.outlined.EditNote
import androidx.compose.material.icons.outlined.Image
import androidx.compose.material.icons.outlined.Lock
import androidx.compose.material.icons.outlined.Payments
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material.icons.outlined.WarningAmber
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.SnackbarResult
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.repeatOnLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.ArrangeRespDto
import org.pjstatecm.graceride.data.api.PlanDataDto
import org.pjstatecm.graceride.data.api.PlanDraftDto
import org.pjstatecm.graceride.data.api.PlanDriverDto
import org.pjstatecm.graceride.data.api.PlanRideDto
import org.pjstatecm.graceride.data.api.PlanVehicleDto
import org.pjstatecm.graceride.data.api.PlanVisitorDto
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.Assistance
import org.pjstatecm.graceride.domain.PlanDraftDiffSummary
import org.pjstatecm.graceride.domain.EffectiveVisitor
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.PlanLogic
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.GraceRideSpringEasing
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.components.animatorsEnabled
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun PlanTransportScreen(
    container: AppContainer,
    nav: NavHostController,
    eventId: Long,
    initialDraftId: Long? = null,
) {
    val vm: PlanTransportViewModel = viewModel(
        key = "plan-transport-$eventId",
        factory = PlanTransportViewModel.factory(container, eventId, initialDraftId),
    )
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val currentMe = (authState as? AuthState.SignedIn)?.me ?: container.auth.me
    val snackbar = remember { SnackbarHostState() }
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val lifecycleOwner = LocalLifecycleOwner.current

    var pickerRide by remember { mutableStateOf<PlanRideDto?>(null) }
    var showNewDraftDialog by remember { mutableStateOf(false) }
    var showReturnDialog by remember { mutableStateOf(false) }
    var returnNotes by remember { mutableStateOf("") }
    var showApproveConfirm by remember { mutableStateOf(false) }
    var confirmDeleteDraftId by remember { mutableStateOf<Long?>(null) }
    var showExitGuard by remember { mutableStateOf(false) }
    var pendingVersionTarget by remember { mutableStateOf<Pair<Boolean, Long?>>(false to null) }
    var exportingPng by remember { mutableStateOf(false) }

    var showVisitorDialog by remember { mutableStateOf(false) }
    var editingVisitor by remember { mutableStateOf<EffectiveVisitor?>(null) }
    var specialReimbTarget by remember { mutableStateOf<PlanVisitorDto?>(null) }

    BackHandler(enabled = state.hasUnsavedChanges) {
        showExitGuard = true
    }

    // 3-minute lease lock heartbeat while editing our draft and screen is STARTED.
    val activeDraft = state.activeDraft
    LaunchedEffect(lifecycleOwner, activeDraft?.id, activeDraft?.lockIsMine) {
        if (activeDraft == null || !activeDraft.lockIsMine) return@LaunchedEffect
        lifecycleOwner.repeatOnLifecycle(Lifecycle.State.STARTED) {
            while (true) {
                delay(180_000L)
                vm.renewLock()
            }
        }
    }

    val savedText = stringResource(R.string.plan_save_ok)
    val approvedText = stringResource(R.string.plan_status_approved)
    val staleText = stringResource(R.string.traffic_stale_conflict)
    val lockAcquiredText = stringResource(R.string.plan_lock_acquired)
    val lockReleasedText = stringResource(R.string.plan_lock_released)
    val postEventText = stringResource(R.string.plan_post_event_submitted)
    val exportTitle = stringResource(R.string.plan_export_title)
    val exportOutbound = stringResource(R.string.plan_leg_outbound)
    val exportReturn = stringResource(R.string.plan_leg_return)
    val exportUnassigned = stringResource(R.string.plan_unassigned_section)
    val exportFailedText = stringResource(R.string.plan_export_failed)
    val undoLabel = stringResource(R.string.plan_action_undo)

    LaunchedEffect(state.message, state.notice) {
        val noticeText = when (state.notice) {
            PlanNotice.SAVED -> savedText
            PlanNotice.APPROVED -> approvedText
            PlanNotice.STALE_RELOAD -> staleText
            PlanNotice.LOCK_ACQUIRED -> lockAcquiredText
            PlanNotice.LOCK_RELEASED -> lockReleasedText
            PlanNotice.POST_EVENT_SUBMITTED -> postEventText
            null -> null
        }
        val msg = state.message ?: noticeText
        if (msg != null) {
            snackbar.showSnackbar(msg)
            vm.clearMessage()
        }
    }

    val plan = state.plan
    val effectiveVisitors = remember(plan?.visitors, state.draftVisitors, state.activeDraftId) {
        if (plan == null) emptyList()
        else if (state.activeDraftId == null) PlanLogic.mergeEffectiveVisitors(plan.visitors, emptyList())
        else PlanLogic.mergeEffectiveVisitors(plan.visitors, state.draftVisitors)
    }
    val isLead = plan?.canApprove == true || currentMe?.canApprovePlan() == true
    val changes = remember(plan?.rides, state.assign) {
        if (plan == null) emptyList() else PlanLogic.changesOf(plan.rides, state.assign)
    }
    val diffSummary = remember(plan?.rides, state.assign, state.draftVisitors) {
        if (plan == null) PlanDraftDiffSummary(0, 0, 0, 0)
        else PlanLogic.summarizeDraftDiff(
            plan.rides,
            PlanLogic.draftEntries(plan.rides, state.assign),
            state.draftVisitors,
        )
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                        ) {
                            Text(
                                plan?.event?.title ?: stringResource(R.string.plan_title),
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                                modifier = Modifier.weight(1f, fill = false),
                            )
                            AnimatedVisibility(
                                visible = state.hasUnsavedChanges,
                                enter = if (animatorsEnabled()) {
                                    fadeIn(tween(160)) + expandHorizontally(tween(180, easing = GraceRideSpringEasing))
                                } else {
                                    fadeIn(tween(100))
                                },
                                exit = if (animatorsEnabled()) {
                                    fadeOut(tween(120)) + shrinkHorizontally(tween(150, easing = GraceRideSpringEasing))
                                } else {
                                    fadeOut(tween(80))
                                },
                            ) {
                                val c = LocalStatusColors.current
                                SmallBadgePill(stringResource(R.string.plan_unsaved_badge), c.warningSoft, c.warning)
                            }
                        }
                        plan?.event?.startsAt?.takeIf { it.isNotBlank() }?.let {
                            Text(
                                KlTime.display(it),
                                style = MaterialTheme.typography.bodySmall.tabular(),
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                },
                navigationIcon = {
                    IconButton(
                        onClick = {
                            if (state.hasUnsavedChanges) showExitGuard = true
                            else nav.popBackStack()
                        },
                    ) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
                actions = {
                    if (plan != null) {
                        IconButton(
                            enabled = !exportingPng,
                            onClick = {
                                exportingPng = true
                                scope.launch {
                                    runCatching {
                                        val uri = PlanImageExporter.exportAndCreateShareUri(
                                            context = context,
                                            plan = plan,
                                            assign = state.assign,
                                            visitors = effectiveVisitors,
                                            outboundLabel = exportOutbound,
                                            returnLabel = exportReturn,
                                            unassignedLabel = exportUnassigned,
                                        )
                                        PlanImageExporter.launchShareIntent(context, uri, exportTitle)
                                    }.onFailure {
                                        vm.reportMessage(exportFailedText)
                                    }
                                    exportingPng = false
                                }
                            },
                        ) {
                            if (exportingPng) {
                                InlineButtonSpinner()
                            } else {
                                Icon(Icons.Outlined.Image, contentDescription = stringResource(R.string.plan_action_export_png))
                            }
                        }
                        if (isLead) {
                            IconButton(onClick = { vm.openPostEventSheet() }) {
                                Icon(Icons.Outlined.Payments, contentDescription = stringResource(R.string.plan_action_post_event))
                            }
                        }
                    }
                    IconButton(onClick = { vm.refresh() }) {
                        Icon(Icons.Outlined.Refresh, contentDescription = stringResource(R.string.common_refresh))
                    }
                },
            )
        },
        bottomBar = {
            if (plan != null) {
                PlanBottomActionDock(
                    state = state,
                    activeDraft = activeDraft,
                    isLead = isLead,
                    currentUserId = currentMe?.id,
                    changesCount = changes.size,
                    diffSummary = diffSummary,
                    onUndo = { vm.undo() },
                    onReset = { vm.resetBoard() },
                    onSaveDraft = { vm.saveDraft() },
                    onRequestApproval = { vm.requestApproval() },
                    onSaveLive = { vm.saveLive() },
                    onApproveAndNotify = { showApproveConfirm = true },
                    onReturnDraft = {
                        returnNotes = ""
                        showReturnDialog = true
                    },
                    onDeleteDraft = { did -> confirmDeleteDraftId = did },
                )
            }
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        when {
            state.loading && plan == null -> LoadingView(Modifier.padding(padding))
            state.error != null && plan == null -> ErrorView(state.error!!, Modifier.padding(padding), onRetry = { vm.refresh() })
            plan != null -> {
                val driversById = remember(plan.drivers) { plan.drivers.associateBy { it.id } }
                val c = LocalStatusColors.current
                val filteredRides = remember(plan.rides, state.legFilter) {
                    if (state.legFilter == "all") plan.rides
                    else plan.rides.filter { it.leg == state.legFilter }
                }
                val unassigned = remember(filteredRides, state.assign) {
                    filteredRides.filter { state.assign[it.id] == null }
                }
                val lastMatchCount = remember(plan.rides, state.assign, driversById) {
                    val (_, count) = PlanLogic.applyLastTime(plan.rides, state.assign, driversById)
                    count
                }
                val canEditVisitorsLive = state.activeDraftId == null && (plan.visitors.any { it.canEdit } || isLead)
                val canEditVisitorsDraft = state.activeDraftId != null && state.isEditable

                PullToRefreshBox(
                    isRefreshing = state.refreshing,
                    onRefresh = { vm.refresh() },
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding),
                ) {
                    LazyColumn(
                        Modifier.fillMaxSize(),
                        contentPadding = PaddingValues(Spacing.sm),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        // Version / Draft Bar
                        item {
                            LazyRow(
                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                item {
                                    FilterChip(
                                        selected = state.activeDraftId == null,
                                        onClick = {
                                            if (state.activeDraftId != null && state.hasUnsavedChanges) {
                                                pendingVersionTarget = true to null
                                            } else {
                                                vm.selectVersion(null)
                                            }
                                        },
                                        label = { Text(stringResource(R.string.plan_version_live)) },
                                    )
                                }
                                items(plan.drafts, key = { "draft-ver-${it.id}" }) { draft ->
                                    FilterChip(
                                        selected = state.activeDraftId == draft.id,
                                        onClick = {
                                            if (state.activeDraftId != draft.id && state.hasUnsavedChanges) {
                                                pendingVersionTarget = true to draft.id
                                            } else {
                                                vm.selectVersion(draft.id)
                                            }
                                        },
                                        label = {
                                            val title = draft.label.ifBlank { "#${draft.version}" }
                                            Text(stringResource(R.string.plan_version_item, draft.version, title))
                                        },
                                    )
                                }
                                item {
                                    OutlinedButton(
                                        onClick = { showNewDraftDialog = true },
                                        enabled = !state.busy,
                                    ) {
                                        Icon(Icons.Outlined.Add, contentDescription = null, modifier = Modifier.size(16.dp))
                                        Spacer(Modifier.width(4.dp))
                                        Text(stringResource(R.string.plan_new_draft))
                                    }
                                }
                            }
                        }

                        // Draft status / lock / returned banners
                        if (activeDraft != null) {
                            item {
                                Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                                    Row(
                                        Modifier.fillMaxWidth(),
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        PlanDraftStatusPill(activeDraft.status)
                                        Text(
                                            stringResource(
                                                R.string.plan_diff_summary,
                                                diffSummary.assignmentChanges,
                                                diffSummary.visitorAdds,
                                                diffSummary.visitorUpdates,
                                                diffSummary.visitorDeletes,
                                            ),
                                            style = MaterialTheme.typography.labelMedium.tabular(),
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                    }
                                    if (activeDraft.status == "returned" && !activeDraft.reviewNotes.isNullOrBlank()) {
                                        CalloutBanner(
                                            icon = Icons.Outlined.WarningAmber,
                                            text = stringResource(
                                                R.string.plan_returned_banner,
                                                activeDraft.reviewedBy?.name.orEmpty(),
                                                activeDraft.reviewNotes,
                                            ),
                                            containerColor = c.warningSoft,
                                            contentColor = c.warning,
                                        )
                                    }
                                    if (state.lockedByOther) {
                                        OutlinedCard(
                                            Modifier.fillMaxWidth(),
                                            border = BorderStroke(1.dp, c.warning.copy(alpha = 0.5f)),
                                        ) {
                                            Row(
                                                Modifier.fillMaxWidth().padding(Spacing.sm),
                                                verticalAlignment = Alignment.CenterVertically,
                                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                            ) {
                                                Icon(Icons.Outlined.Lock, contentDescription = null, tint = c.warning)
                                                Text(
                                                    stringResource(
                                                        R.string.plan_lock_other,
                                                        activeDraft.lockedBy?.name.orEmpty(),
                                                        activeDraft.lockExpiresAt?.let(KlTime::displayTime).orEmpty(),
                                                    ),
                                                    style = MaterialTheme.typography.bodySmall,
                                                    modifier = Modifier.weight(1f),
                                                )
                                                TextButton(onClick = { vm.takeOverLock() }, enabled = !state.lockBusy) {
                                                    Text(stringResource(R.string.plan_lock_takeover))
                                                }
                                            }
                                        }
                                    } else if (activeDraft.lockIsMine) {
                                        CalloutBanner(
                                            icon = Icons.Outlined.EditNote,
                                            text = stringResource(R.string.plan_lock_mine),
                                            containerColor = c.infoSoft,
                                            contentColor = c.info,
                                        )
                                    }
                                }
                            }
                        }

                        // Compact Smart Arrange Helper Chips
                        item {
                            FlowRow(
                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                            ) {
                                OutlinedButton(
                                    onClick = { vm.autoArrange() },
                                    enabled = state.isEditable && !state.busy && unassigned.isNotEmpty(),
                                ) {
                                    Icon(Icons.Outlined.AutoFixHigh, contentDescription = null, modifier = Modifier.size(16.dp))
                                    Spacer(Modifier.width(4.dp))
                                    Text(stringResource(R.string.plan_action_auto_arrange))
                                }
                                if (lastMatchCount > 0) {
                                    OutlinedButton(
                                        onClick = { vm.applyLastTime() },
                                        enabled = state.isEditable && !state.busy,
                                    ) {
                                        Text(stringResource(R.string.plan_action_last_time, lastMatchCount))
                                    }
                                }
                                OutlinedButton(
                                    onClick = { vm.applyMatchReturnDrivers() },
                                    enabled = state.isEditable && !state.busy,
                                ) {
                                    Text(stringResource(R.string.plan_action_match_returns))
                                }
                            }
                        }

                        // Leg Filter
                        item {
                            val legs = listOf(
                                "all" to R.string.plan_leg_all,
                                "outbound" to R.string.plan_leg_outbound,
                                "return" to R.string.plan_leg_return,
                            )
                            SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
                                legs.forEachIndexed { idx, (key, res) ->
                                    SegmentedButton(
                                        selected = state.legFilter == key,
                                        onClick = { vm.setLegFilter(key) },
                                        shape = SegmentedButtonDefaults.itemShape(idx, legs.size),
                                    ) { Text(stringResource(res), maxLines = 1, overflow = TextOverflow.Ellipsis) }
                                }
                            }
                        }

                        // Unassigned Passengers Section
                        if (plan.rides.isEmpty()) {
                            item { EmptyView(text = stringResource(R.string.plan_no_rides)) }
                        } else {
                            if (unassigned.isNotEmpty()) {
                                item {
                                    SectionHeader(
                                        title = stringResource(R.string.plan_unassigned_section),
                                        count = unassigned.size,
                                    )
                                }
                                items(unassigned, key = { "unassigned-${it.id}" }) { ride ->
                                    PlanRideCard(
                                        ride = ride,
                                        driverName = null,
                                        editable = state.isEditable,
                                        modifier = Modifier.animateItem(),
                                        onClick = { pickerRide = ride },
                                    )
                                }
                            }

                            // Drivers & Loads
                            item {
                                SectionHeader(
                                    title = stringResource(R.string.plan_assigned_section),
                                    count = plan.drivers.size,
                                )
                            }
                            items(plan.drivers, key = { "plan-drv-${it.id}" }) { driver ->
                                val assignedHere = filteredRides.filter { state.assign[it.id] == driver.id }
                                val unassignFmt = stringResource(R.string.plan_unassigned_undo, "%s")
                                PlanDriverCard(
                                    driver = driver,
                                    assignedRides = assignedHere,
                                    allRides = plan.rides,
                                    assign = state.assign,
                                    driversById = driversById,
                                    editable = state.isEditable,
                                    modifier = Modifier.animateItem(),
                                    onRideClick = { pickerRide = it },
                                    onUnassign = { ride ->
                                        vm.assignRide(ride.id, null)
                                        scope.launch {
                                            val res = snackbar.showSnackbar(
                                                message = String.format(unassignFmt, ride.passenger.name),
                                                actionLabel = undoLabel,
                                                duration = SnackbarDuration.Short,
                                            )
                                            if (res == SnackbarResult.ActionPerformed) {
                                                vm.undo()
                                            }
                                        }
                                    },
                                )
                            }
                        }

                        // Visitors (live or staged inside active draft)
                        item {
                            PlanVisitorsHeader(
                                count = effectiveVisitors.count { it.draftOp != "delete" },
                                canEdit = canEditVisitorsLive || canEditVisitorsDraft,
                                onAddClick = {
                                    editingVisitor = null
                                    showVisitorDialog = true
                                },
                            )
                        }
                        items(effectiveVisitors, key = { it.key }) { visitor ->
                            EffectiveVisitorCard(
                                visitor = visitor,
                                canEdit = canEditVisitorsLive || canEditVisitorsDraft,
                                canRequestSpecial = isLead && state.activeDraftId == null && visitor.liveVisitorId != null,
                                modifier = Modifier.animateItem(),
                                onEdit = {
                                    editingVisitor = visitor
                                    showVisitorDialog = true
                                },
                                onDelete = {
                                    if (state.activeDraftId != null) {
                                        vm.stageDeleteVisitor(visitor.liveVisitorId, visitor.tempId, visitor.name)
                                    } else if (visitor.liveVisitorId != null) {
                                        vm.deleteLiveVisitor(visitor.liveVisitorId)
                                    }
                                },
                                onSpecialReimb = {
                                    visitor.liveSource?.let { specialReimbTarget = it }
                                },
                            )
                        }
                    }
                }
            }
        }
    }

    // Unsaved Changes Exit / Version-Switch Guard
    if (showExitGuard || pendingVersionTarget.first) {
        AlertDialog(
            onDismissRequest = {
                showExitGuard = false
                pendingVersionTarget = false to null
            },
            title = { Text(stringResource(R.string.plan_unsaved_title)) },
            text = { Text(stringResource(R.string.plan_unsaved_body)) },
            confirmButton = {
                Button(
                    enabled = state.isEditable && !state.busy,
                    colors = graceRidePrimaryButtonColors(),
                    onClick = {
                        val exitAfter = showExitGuard
                        val nextDraft = pendingVersionTarget.second
                        vm.saveCurrentBoard {
                            showExitGuard = false
                            pendingVersionTarget = false to null
                            if (exitAfter) nav.popBackStack()
                            else vm.selectVersion(nextDraft)
                        }
                    },
                ) { Text(stringResource(R.string.plan_unsaved_save)) }
            },
            dismissButton = {
                Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
                    TextButton(
                        onClick = {
                            val exitAfter = showExitGuard
                            val nextDraft = pendingVersionTarget.second
                            showExitGuard = false
                            pendingVersionTarget = false to null
                            if (exitAfter) nav.popBackStack()
                            else vm.selectVersion(nextDraft)
                        },
                    ) { Text(stringResource(R.string.plan_unsaved_discard)) }
                    TextButton(
                        onClick = {
                            showExitGuard = false
                            pendingVersionTarget = false to null
                        },
                    ) { Text(stringResource(R.string.plan_unsaved_stay)) }
                }
            },
        )
    }

    // Driver Picker Sheet
    val currentPickerRide = pickerRide
    if (currentPickerRide != null && plan != null) {
        PlanDriverPickerSheet(
            ride = currentPickerRide,
            plan = plan,
            assign = state.assign,
            onSelect = { driverId ->
                vm.assignRide(currentPickerRide.id, driverId)
                pickerRide = null
            },
            onDismiss = { pickerRide = null },
        )
    }

    // Diagnosis Sheet after Auto-Arrange
    state.arrangeDiag?.let { diag ->
        if (plan != null) {
            ArrangeDiagnosisSheet(
                diag = diag,
                plan = plan,
                onDismiss = { vm.dismissArrangeDiag() },
            )
        }
    }

    // Post-Event Driver Reimbursements Sheet
    if (state.showPostEventSheet) {
        PostEventReimbursementSheet(
            loading = state.postEventLoading,
            busy = state.postEventBusy,
            data = state.postEventData,
            onSubmit = vm::submitPostEventClaims,
            onDismiss = vm::closePostEventSheet,
        )
    }

    if (showNewDraftDialog) {
        NewDraftDialog(
            canCloneCurrent = plan?.rides?.isNotEmpty() == true,
            onCreate = { label, clone ->
                vm.createNewDraft(label, clone)
                showNewDraftDialog = false
            },
            onDismiss = { showNewDraftDialog = false },
        )
    }

    if (showReturnDialog) {
        ReturnNotesDialog(
            notes = returnNotes,
            onNotesChange = { returnNotes = it },
            quickReasons = listOf(
                stringResource(R.string.plan_quick_reason_route),
                stringResource(R.string.plan_quick_reason_capacity),
                stringResource(R.string.plan_quick_reason_driver),
            ),
            onConfirm = {
                vm.returnDraft(returnNotes)
                showReturnDialog = false
            },
            onDismiss = { showReturnDialog = false },
        )
    }

    if (showApproveConfirm && plan != null) {
        val diffCount = remember(plan.rides, state.assign, state.draftVisitors) {
            PlanLogic.summarizeDraftDiff(
                plan.rides,
                PlanLogic.draftEntries(plan.rides, state.assign),
                state.draftVisitors,
            ).assignmentChanges
        }
        AlertDialog(
            onDismissRequest = { showApproveConfirm = false },
            title = { Text(stringResource(R.string.plan_approve_confirm_title)) },
            text = { Text(stringResource(R.string.plan_approve_confirm_body, diffCount)) },
            confirmButton = {
                Button(
                    onClick = {
                        vm.approveAndNotify()
                        showApproveConfirm = false
                    },
                    colors = graceRidePrimaryButtonColors(),
                ) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = {
                TextButton(onClick = { showApproveConfirm = false }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }

    confirmDeleteDraftId?.let { did ->
        val ver = plan?.drafts?.firstOrNull { it.id == did }?.version ?: 1
        AlertDialog(
            onDismissRequest = { confirmDeleteDraftId = null },
            title = { Text(stringResource(R.string.plan_action_delete_draft)) },
            text = { Text(stringResource(R.string.plan_delete_draft_confirm, ver)) },
            confirmButton = {
                Button(
                    onClick = {
                        vm.deleteDraft(did)
                        confirmDeleteDraftId = null
                    },
                ) { Text(stringResource(R.string.common_delete)) }
            },
            dismissButton = {
                TextButton(onClick = { confirmDeleteDraftId = null }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }

    if (showVisitorDialog && plan != null) {
        VisitorEditorDialog(
            initial = editingVisitor,
            areas = state.areas,
            drivers = plan.drivers,
            onDismiss = { showVisitorDialog = false },
            onSave = { name, phone, notes, area, address, seats, leg, driver ->
                val cur = editingVisitor
                if (state.activeDraftId != null) {
                    if (cur == null) {
                        vm.stageAddVisitor(name, phone, notes, area, address, seats, leg, driver)
                    } else {
                        vm.stageUpdateVisitor(cur.liveVisitorId, cur.tempId, name, phone, notes, area, address, seats, leg, driver)
                    }
                } else {
                    val req = buildVisitorReq(name, phone, notes, area, address, seats, leg, driver)
                    if (cur?.liveVisitorId == null) vm.createLiveVisitor(req)
                    else vm.updateLiveVisitor(cur.liveVisitorId, req)
                }
                showVisitorDialog = false
            },
        )
    }

    specialReimbTarget?.let { visitor ->
        SpecialReimbursementDialog(
            visitor = visitor,
            onDismiss = { specialReimbTarget = null },
            onSubmit = { amountCents, reason ->
                vm.submitSpecialReimbursement(visitor.id, visitor.specialReimbursement?.id, amountCents, reason)
                specialReimbTarget = null
            },
        )
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun PlanBottomActionDock(
    state: PlanTransportUiState,
    activeDraft: PlanDraftDto?,
    isLead: Boolean,
    currentUserId: Long?,
    changesCount: Int,
    diffSummary: PlanDraftDiffSummary,
    onUndo: () -> Unit,
    onReset: () -> Unit,
    onSaveDraft: () -> Unit,
    onRequestApproval: () -> Unit,
    onSaveLive: () -> Unit,
    onApproveAndNotify: () -> Unit,
    onReturnDraft: () -> Unit,
    onDeleteDraft: (Long) -> Unit,
) {
    val canDeleteDraft = activeDraft != null &&
        activeDraft.status in setOf("draft", "returned") &&
        (isLead || activeDraft.createdBy?.id == currentUserId)
    val canEditDraft = activeDraft != null &&
        activeDraft.status in setOf("draft", "returned") &&
        !state.lockedByOther

    Surface(
        color = MaterialTheme.colorScheme.surface,
        tonalElevation = 2.dp,
    ) {
        Column(
            Modifier
                .fillMaxWidth()
                .navigationBarsPadding(),
        ) {
            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
            Column(
                Modifier
                    .fillMaxWidth()
                    .animateContentSize(
                        animationSpec = if (animatorsEnabled()) tween(200, easing = GraceRideSpringEasing) else snap(),
                    )
                    .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
            ) {
                Row(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    val summaryText = if (activeDraft != null) {
                        stringResource(
                            R.string.plan_diff_summary,
                            diffSummary.assignmentChanges,
                            diffSummary.visitorAdds,
                            diffSummary.visitorUpdates,
                            diffSummary.visitorDeletes,
                        )
                    } else {
                        stringResource(
                            R.string.plan_diff_summary,
                            changesCount,
                            0,
                            0,
                            0,
                        )
                    }
                    Text(
                        summaryText,
                        style = MaterialTheme.typography.labelMedium.tabular(),
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.weight(1f),
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        if (state.undoStack.isNotEmpty() && state.isEditable) {
                            TextButton(onClick = onUndo, enabled = !state.busy) {
                                Icon(Icons.AutoMirrored.Outlined.Undo, contentDescription = null, modifier = Modifier.size(16.dp))
                                Spacer(Modifier.width(4.dp))
                                Text("${stringResource(R.string.plan_action_undo)} (${state.undoStack.size})")
                            }
                        }
                        if (activeDraft == null && changesCount > 0) {
                            TextButton(onClick = onReset, enabled = !state.busy) {
                                Text(stringResource(R.string.plan_action_reset))
                            }
                        }
                        if (canDeleteDraft) {
                            IconButton(
                                onClick = { onDeleteDraft(activeDraft.id) },
                                enabled = !state.busy && !state.lockedByOther,
                            ) {
                                Icon(Icons.Outlined.DeleteOutline, contentDescription = stringResource(R.string.plan_action_delete_draft))
                            }
                        }
                    }
                }
                FlowRow(
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                ) {
                    if (activeDraft != null) {
                        if (canEditDraft) {
                            OutlinedButton(onClick = onSaveDraft, enabled = !state.busy) {
                                Text(stringResource(R.string.plan_action_save_draft))
                            }
                            Button(onClick = onRequestApproval, enabled = !state.busy) {
                                Text(stringResource(R.string.plan_action_submit_approval))
                            }
                        }
                        if (isLead && activeDraft.status != "approved") {
                            Button(
                                onClick = onApproveAndNotify,
                                enabled = !state.busy && !state.lockedByOther,
                                colors = graceRidePrimaryButtonColors(),
                            ) {
                                if (state.busy) {
                                    InlineButtonSpinner()
                                    Spacer(Modifier.width(Spacing.xxs))
                                }
                                Text(stringResource(R.string.plan_action_approve_notify))
                            }
                            if (activeDraft.status == "pending_approval") {
                                OutlinedButton(onClick = onReturnDraft, enabled = !state.busy) {
                                    Text(stringResource(R.string.plan_action_return))
                                }
                            }
                        }
                    } else {
                        if (changesCount > 0) {
                            OutlinedButton(onClick = onSaveLive, enabled = !state.busy) {
                                Text(stringResource(R.string.plan_action_save_live))
                            }
                        }
                        if (isLead) {
                            Button(
                                onClick = onApproveAndNotify,
                                enabled = !state.busy,
                                colors = graceRidePrimaryButtonColors(),
                            ) {
                                if (state.busy) {
                                    InlineButtonSpinner()
                                    Spacer(Modifier.width(Spacing.xxs))
                                }
                                Text(stringResource(R.string.plan_action_approve_notify))
                            }
                        }
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun PlanRideCard(
    ride: PlanRideDto,
    driverName: String?,
    editable: Boolean,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
) {
    val c = LocalStatusColors.current
    val legLabel = stringResource(if (ride.leg == "return") R.string.plan_leg_return else R.string.plan_leg_outbound)
    OutlinedCard(
        onClick = { if (editable) onClick() },
        modifier = modifier.fillMaxWidth(),
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
    ) {
        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    "${ride.passenger.name} · $legLabel (${ride.seats})",
                    style = MaterialTheme.typography.titleMedium.tabular(),
                    modifier = Modifier.weight(1f),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Spacer(Modifier.width(Spacing.xs))
                if (editable) {
                    SmallBadgePill(
                        driverName ?: stringResource(R.string.plan_driver_tap_to_assign),
                        MaterialTheme.colorScheme.primaryContainer,
                        MaterialTheme.colorScheme.onPrimaryContainer,
                    )
                } else if (driverName != null) {
                    SmallBadgePill(
                        driverName,
                        MaterialTheme.colorScheme.surfaceVariant,
                        MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            Text(
                "${ride.area.name}${if (ride.pickupAddress.isNotBlank()) " · ${ride.pickupAddress}" else ""}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            FlowRow(
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                ride.last?.let { last ->
                    SmallBadgePill(
                        stringResource(R.string.plan_last_driver_badge, last.driverName, last.times),
                        MaterialTheme.colorScheme.primaryContainer,
                        MaterialTheme.colorScheme.onPrimaryContainer,
                    )
                }
                if (ride.assistance.isNotBlank()) {
                    val assistLabel = Assistance.labelRes(ride.assistance)?.let { stringResource(it) } ?: ride.assistance
                    Surface(color = c.infoSoft, contentColor = c.info, shape = CircleShape) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(4.dp),
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp),
                        ) {
                            Icon(Icons.AutoMirrored.Outlined.Accessible, contentDescription = null, modifier = Modifier.size(14.dp))
                            Text(assistLabel, style = MaterialTheme.typography.labelMedium)
                        }
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun VehicleAccessPills(vehicle: PlanVehicleDto?) {
    if (vehicle == null) return
    val c = LocalStatusColors.current
    if (!vehicle.wheelchairOk && !vehicle.lowStep && !vehicle.childSeat) return
    FlowRow(
        horizontalArrangement = Arrangement.spacedBy(6.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        if (vehicle.wheelchairOk) {
            SmallBadgePill(stringResource(R.string.plan_driver_wheelchair), c.successSoft, c.success)
        }
        if (vehicle.lowStep) {
            SmallBadgePill(stringResource(R.string.plan_driver_low_step), c.infoSoft, c.info)
        }
        if (vehicle.childSeat) {
            SmallBadgePill(stringResource(R.string.plan_driver_child_seat), c.infoSoft, c.info)
        }
    }
}

@Composable
private fun DriverLegLoadBar(
    label: String,
    seats: Long,
    cap: Long,
    modifier: Modifier = Modifier,
) {
    val c = LocalStatusColors.current
    val rawRatio = if (cap > 0L) (seats.toFloat() / cap.toFloat()).coerceIn(0f, 1f) else 0f
    val ratio by animateFloatAsState(
        targetValue = rawRatio,
        animationSpec = if (animatorsEnabled()) tween(220, easing = GraceRideSpringEasing) else snap(),
        label = "driver_leg_load",
    )
    val barColor = when {
        seats > cap -> c.danger
        seats >= cap && cap > 0L -> c.warning
        else -> c.success
    }
    Column(modifier, verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Row(
            Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Text(label, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Text(
                stringResource(R.string.plan_driver_seats_fmt, seats, cap),
                style = MaterialTheme.typography.labelSmall.tabular(),
                color = if (seats > cap) c.danger else MaterialTheme.colorScheme.onSurface,
            )
        }
        LinearProgressIndicator(
            progress = { ratio },
            modifier = Modifier
                .fillMaxWidth()
                .height(5.dp)
                .clip(CircleShape),
            color = barColor,
            trackColor = MaterialTheme.colorScheme.surfaceVariant,
        )
    }
}

@Composable
private fun PlanDriverCard(
    driver: PlanDriverDto,
    assignedRides: List<PlanRideDto>,
    allRides: List<PlanRideDto>,
    assign: Map<Long, Long?>,
    driversById: Map<Long, PlanDriverDto>,
    editable: Boolean,
    modifier: Modifier = Modifier,
    onRideClick: (PlanRideDto) -> Unit,
    onUnassign: (PlanRideDto) -> Unit,
) {
    val outboundLoad = remember(driver.id, allRides, assign) {
        PlanLogic.driverLoad(driver.id, "outbound", allRides, assign, driversById)
    }
    val returnLoad = remember(driver.id, allRides, assign) {
        PlanLogic.driverLoad(driver.id, "return", allRides, assign, driversById)
    }
    OutlinedCard(
        modifier.fillMaxWidth(),
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
    ) {
        Column(
            Modifier
                .animateContentSize(
                    animationSpec = if (animatorsEnabled()) tween(200, easing = GraceRideSpringEasing) else snap(),
                )
                .padding(Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.Top,
            ) {
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Text(driver.name, style = MaterialTheme.typography.titleMedium)
                    driver.vehicle?.model?.takeIf { it.isNotBlank() }?.let { model ->
                        Text(
                            "$model${driver.vehicle.plate?.let { " · $it" }.orEmpty()}",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
            VehicleAccessPills(driver.vehicle)
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.sm)) {
                DriverLegLoadBar(
                    label = stringResource(R.string.plan_leg_outbound),
                    seats = outboundLoad.seats,
                    cap = driver.cap,
                    modifier = Modifier.weight(1f),
                )
                DriverLegLoadBar(
                    label = stringResource(R.string.plan_leg_return),
                    seats = returnLoad.seats,
                    cap = driver.cap,
                    modifier = Modifier.weight(1f),
                )
            }
            if (!driver.carpool) {
                Text(
                    stringResource(R.string.plan_driver_no_carpool),
                    style = MaterialTheme.typography.labelSmall,
                    color = LocalStatusColors.current.warning,
                )
            }
            if (assignedRides.isNotEmpty()) {
                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                assignedRides.forEach { ride ->
                    val legTag = stringResource(if (ride.leg == "return") R.string.plan_leg_return else R.string.plan_leg_outbound)
                    Surface(
                        onClick = { if (editable) onRideClick(ride) },
                        color = MaterialTheme.colorScheme.surfaceContainerLow,
                        shape = MaterialTheme.shapes.small,
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Row(
                            Modifier
                                .fillMaxWidth()
                                .padding(horizontal = Spacing.xs, vertical = Spacing.xxs),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween,
                        ) {
                            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                Text(
                                    "${ride.passenger.name} (${ride.seats}) · ${ride.area.name}",
                                    style = MaterialTheme.typography.titleSmall.tabular(),
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                )
                                val sub = buildString {
                                    append(legTag)
                                    if (ride.pickupAddress.isNotBlank()) append(" · ${ride.pickupAddress}")
                                    if (ride.assistance.isNotBlank()) {
                                        val assistLabel = Assistance.labelRes(ride.assistance)?.let { stringResource(it) } ?: ride.assistance
                                        append(" · $assistLabel")
                                    }
                                }
                                Text(
                                    sub,
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                )
                            }
                            if (editable) {
                                IconButton(onClick = { onUnassign(ride) }) {
                                    Icon(Icons.Outlined.Close, contentDescription = stringResource(R.string.plan_driver_unassign))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
private fun PlanDriverPickerSheet(
    ride: PlanRideDto,
    plan: PlanDataDto,
    assign: Map<Long, Long?>,
    onSelect: (Long?) -> Unit,
    onDismiss: () -> Unit,
) {
    var query by remember { mutableStateOf("") }
    var fitsOnly by remember { mutableStateOf(false) }
    val driversById = remember(plan.drivers) { plan.drivers.associateBy { it.id } }
    val ranked = remember(plan.drivers, ride, plan.rides, assign, query, fitsOnly) {
        val q = query.trim()
        plan.drivers
            .filter { d ->
                val matchesQ = q.isEmpty() ||
                    d.name.contains(q, ignoreCase = true) ||
                    d.vehicle?.model?.contains(q, ignoreCase = true) == true
                val matchesFits = !fitsOnly || PlanLogic.fits(d, ride, plan.rides, assign, driversById)
                matchesQ && matchesFits
            }
            .sortedWith(
                compareByDescending<PlanDriverDto> { PlanLogic.fits(it, ride, plan.rides, assign, driversById) }
                    .thenByDescending { ride.last?.driverId == it.id }
                    .thenByDescending { PlanLogic.servesArea(ride, it) }
                    .thenBy { PlanLogic.driverLoad(it.id, ride.leg, plan.rides, assign, driversById).seats },
            )
    }
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val c = LocalStatusColors.current
    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
        Column(
            Modifier
                .fillMaxWidth()
                .imePadding()
                .navigationBarsPadding()
                .padding(horizontal = Spacing.md)
                .padding(bottom = Spacing.md)
                .heightIn(max = 560.dp),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Text(
                "${ride.passenger.name} · ${ride.area.name} (${ride.seats})",
                style = MaterialTheme.typography.titleLarge,
            )
            OutlinedTextField(
                value = query,
                onValueChange = { query = it },
                placeholder = {
                    Text(
                        stringResource(R.string.plan_driver_search_hint),
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                },
                leadingIcon = { Icon(Icons.Outlined.Search, contentDescription = null) },
                trailingIcon = {
                    if (query.isNotEmpty()) {
                        IconButton(onClick = { query = "" }) {
                            Icon(Icons.Outlined.Close, contentDescription = stringResource(R.string.a11y_clear))
                        }
                    }
                },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                FilterChip(
                    selected = fitsOnly,
                    onClick = { fitsOnly = !fitsOnly },
                    label = { Text(stringResource(R.string.plan_driver_fits_only)) },
                )
                if (assign[ride.id] != null) {
                    OutlinedButton(
                        onClick = { onSelect(null) },
                        modifier = Modifier.weight(1f),
                    ) {
                        Text(stringResource(R.string.plan_driver_unassign), maxLines = 1, overflow = TextOverflow.Ellipsis)
                    }
                }
            }
            if (ranked.isEmpty()) {
                EmptyView(text = stringResource(R.string.common_empty_title))
            } else {
                LazyColumn(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    items(ranked, key = { "pick-${it.id}" }) { d ->
                        val fits = PlanLogic.fits(d, ride, plan.rides, assign, driversById)
                        val sameArea = PlanLogic.servesArea(ride, d)
                        val isLast = ride.last?.driverId == d.id
                        val isCurrent = assign[ride.id] == d.id
                        val load = PlanLogic.driverLoad(d.id, ride.leg, plan.rides, assign, driversById)
                        val mismatchReason = when {
                            !PlanLogic.inGroup(ride, d) -> stringResource(R.string.plan_driver_reason_group)
                            !PlanLogic.vehicleCompatible(ride, d) -> stringResource(R.string.plan_driver_reason_access)
                            !fits -> stringResource(R.string.plan_driver_reason_full)
                            !sameArea -> stringResource(R.string.plan_driver_reason_area)
                            else -> null
                        }
                        OutlinedCard(
                            onClick = { if (fits) onSelect(d.id) },
                            enabled = fits,
                            modifier = Modifier.fillMaxWidth(),
                            border = BorderStroke(
                                1.dp,
                                if (isCurrent) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outlineVariant,
                            ),
                        ) {
                            Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                Row(
                                    Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Text(
                                        d.name,
                                        style = MaterialTheme.typography.titleMedium,
                                        modifier = Modifier.weight(1f),
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                    )
                                    Text(
                                        stringResource(R.string.plan_driver_seats_fmt, load.seats, d.cap),
                                        style = MaterialTheme.typography.labelMedium.tabular(),
                                    )
                                }
                                d.vehicle?.model?.takeIf { it.isNotBlank() }?.let { model ->
                                    Text(
                                        model,
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                                FlowRow(
                                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                                    verticalArrangement = Arrangement.spacedBy(4.dp),
                                ) {
                                    if (isCurrent) {
                                        SmallBadgePill(
                                            stringResource(R.string.plan_driver_current_badge),
                                            MaterialTheme.colorScheme.primaryContainer,
                                            MaterialTheme.colorScheme.onPrimaryContainer,
                                        )
                                    }
                                    ride.last?.takeIf { isLast }?.let { lastMeta ->
                                        SmallBadgePill(
                                            stringResource(R.string.plan_driver_last_badge, lastMeta.times),
                                            MaterialTheme.colorScheme.primaryContainer,
                                            MaterialTheme.colorScheme.onPrimaryContainer,
                                        )
                                    }
                                    if (sameArea) {
                                        SmallBadgePill(
                                            stringResource(R.string.plan_driver_same_area),
                                            c.successSoft,
                                            c.success,
                                        )
                                    }
                                    if (d.vehicle?.wheelchairOk == true) {
                                        SmallBadgePill(stringResource(R.string.plan_driver_wheelchair), c.infoSoft, c.info)
                                    }
                                }
                                if (mismatchReason != null) {
                                    Text(
                                        mismatchReason,
                                        style = MaterialTheme.typography.labelSmall,
                                        color = c.warning,
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

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ArrangeDiagnosisSheet(
    diag: ArrangeRespDto,
    plan: PlanDataDto,
    onDismiss: () -> Unit,
) {
    val ridesById = remember(plan.rides) { plan.rides.associateBy { it.id } }
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
        Column(
            Modifier
                .fillMaxWidth()
                .navigationBarsPadding()
                .padding(horizontal = Spacing.md)
                .padding(bottom = Spacing.md),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    stringResource(R.string.plan_arrange_diag_title),
                    style = MaterialTheme.typography.titleLarge,
                    modifier = Modifier.weight(1f),
                )
                IconButton(onClick = onDismiss) {
                    Icon(Icons.Outlined.Close, contentDescription = stringResource(R.string.common_close))
                }
            }
            Text(stringResource(R.string.plan_arrange_placed_count, diag.placed.size), style = MaterialTheme.typography.bodyMedium)
            if (diag.unplaced.isNotEmpty()) {
                Text(
                    stringResource(R.string.plan_arrange_unplaced_count, diag.unplaced.size),
                    style = MaterialTheme.typography.bodyMedium,
                    color = LocalStatusColors.current.warning,
                )
                diag.unplaced.forEach { rideId ->
                    ridesById[rideId]?.let { r ->
                        Text("• ${r.passenger.name} (${r.area.name})", style = MaterialTheme.typography.bodySmall)
                    }
                }
            }
            if (diag.approximate) {
                CalloutBanner(
                    icon = Icons.Outlined.WarningAmber,
                    text = stringResource(R.string.plan_arrange_approx),
                    containerColor = LocalStatusColors.current.warningSoft,
                    contentColor = LocalStatusColors.current.warning,
                )
            }
        }
    }
}

@Composable
private fun NewDraftDialog(
    canCloneCurrent: Boolean,
    onCreate: (label: String, cloneCurrent: Boolean) -> Unit,
    onDismiss: () -> Unit,
) {
    var label by remember { mutableStateOf("") }
    var cloneCurrent by remember { mutableStateOf(true) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.plan_new_draft)) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                OutlinedTextField(
                    value = label,
                    onValueChange = { label = it },
                    label = { Text(stringResource(R.string.plan_new_draft_label_hint)) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                if (canCloneCurrent) {
                    DraftBaseOptionCard(
                        selected = cloneCurrent,
                        text = stringResource(R.string.plan_new_draft_clone),
                        onClick = { cloneCurrent = true },
                    )
                    DraftBaseOptionCard(
                        selected = !cloneCurrent,
                        text = stringResource(R.string.plan_new_draft_base_live),
                        onClick = { cloneCurrent = false },
                    )
                }
            }
        },
        confirmButton = {
            Button(onClick = { onCreate(label, cloneCurrent) }) {
                Text(stringResource(R.string.common_confirm))
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text(stringResource(R.string.common_cancel)) }
        },
    )
}

@Composable
private fun DraftBaseOptionCard(
    selected: Boolean,
    text: String,
    onClick: () -> Unit,
) {
    OutlinedCard(
        onClick = onClick,
        modifier = Modifier.fillMaxWidth(),
        border = BorderStroke(
            1.dp,
            if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outlineVariant,
        ),
    ) {
        Row(
            Modifier
                .fillMaxWidth()
                .padding(horizontal = Spacing.xs, vertical = Spacing.xxs),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            RadioButton(selected = selected, onClick = onClick)
            Text(text, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
        }
    }
}
