package org.pjstatecm.graceride.ui.screens.rides

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.AdminPanelSettings
import androidx.compose.material.icons.outlined.ChevronRight
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.DeleteOutline
import androidx.compose.material.icons.outlined.Edit
import androidx.compose.material.icons.outlined.EditNote
import androidx.compose.material.icons.outlined.Lock
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.AdminDriverDto
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.RideChangeDraftDto
import org.pjstatecm.graceride.data.api.RideDraftInReq
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.GraceRideSpringEasing
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.components.animatorsEnabled
import org.pjstatecm.graceride.ui.screens.traffic.PlanDraftStatusPill
import org.pjstatecm.graceride.ui.screens.traffic.ReturnNotesDialog
import org.pjstatecm.graceride.ui.screens.traffic.SmallBadgePill
import org.pjstatecm.graceride.ui.screens.traffic.rideDraftActionLabel
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors

private enum class StaffReasonMode { RETURN, ALT_TRANSPORT, CANCEL }

internal fun legalRideDraftActions(ride: Ride, includeAction: String? = null): List<String> {
    val active = ride.status !in setOf("completed", "cancelled", "no_show")
    val list = buildList {
        if (ride.status == "pending_approval") add("approve")
        if (active && ride.driver == null) add("assign")
        if (active && ride.driver != null) add("reassign")
        if (ride.status in setOf("driver_assigned", "confirmed")) add("return")
        if (ride.status in setOf("pending_approval", "pending_driver", "driver_assigned", "confirmed")) add("alt_transport")
        if (active) add("cancel")
        add("reimbursement_excluded")
    }
    return if (includeAction != null && includeAction !in list) list + includeAction else list
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun RideDetailStaffAndDraftsSection(
    ride: Ride,
    me: Me?,
    state: RideDetailUiState,
    vm: RideDetailViewModel,
    onOpenRide: (Long) -> Unit,
) {
    if (me == null || !me.can("admin_rides")) return

    val canUpdateRides = me.can("admin_rides", "update")
    val canEditDraft = me.can("admin_plan", "update") || me.can("admin_rides", "update")
    val canApproveLead = me.canApprovePlan()
    val canToggleReimb = canUpdateRides || canApproveLead

    var showReassignSheet by remember { mutableStateOf(false) }
    var reasonMode by remember { mutableStateOf<StaffReasonMode?>(null) }
    var reasonText by remember { mutableStateOf("") }

    var editingDraft by remember { mutableStateOf<RideChangeDraftDto?>(null) }
    var showDraftEditor by remember { mutableStateOf(false) }
    var returningDraft by remember { mutableStateOf<RideChangeDraftDto?>(null) }
    var returnNotes by remember { mutableStateOf("") }
    var confirmDeleteDraft by remember { mutableStateOf<RideChangeDraftDto?>(null) }

    val c = LocalStatusColors.current

    // 1. Direct Staff Controls
    if (ride.viewer.isStaff) {
        OutlinedCard(
            Modifier.fillMaxWidth(),
            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
        ) {
            Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    Icon(Icons.Outlined.AdminPanelSettings, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                    Text(stringResource(R.string.ride_staff_section), style = MaterialTheme.typography.titleMedium)
                }

                if (ride.reimbursementExcluded) {
                    StatusPill(stringResource(R.string.ride_staff_reimb_excluded_badge), RideStatusKind.PENDING)
                }

                if (canUpdateRides) {
                    if (ride.status == "pending_approval") {
                        Button(
                            onClick = { vm.staffApprove() },
                            enabled = !state.busy,
                            colors = graceRidePrimaryButtonColors(),
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            if (state.busy) {
                                InlineButtonSpinner()
                                Spacer(Modifier.width(Spacing.xs))
                            }
                            Text(stringResource(R.string.ride_staff_approve))
                        }
                    }
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        if (ride.status in setOf("pending_approval", "pending_driver", "driver_assigned", "confirmed")) {
                            OutlinedButton(
                                onClick = { showReassignSheet = true },
                                enabled = !state.busy,
                            ) {
                                Text(stringResource(R.string.ride_staff_reassign))
                            }
                        }
                        if (ride.status in setOf("driver_assigned", "confirmed")) {
                            OutlinedButton(
                                onClick = {
                                    reasonText = ""
                                    reasonMode = StaffReasonMode.RETURN
                                },
                                enabled = !state.busy,
                            ) {
                                Text(stringResource(R.string.ride_staff_return))
                            }
                        }
                        if (ride.status in setOf("pending_approval", "pending_driver", "driver_assigned", "confirmed")) {
                            OutlinedButton(
                                onClick = {
                                    reasonText = ""
                                    reasonMode = StaffReasonMode.ALT_TRANSPORT
                                },
                                enabled = !state.busy,
                            ) {
                                Text(stringResource(R.string.ride_staff_alt_transport))
                            }
                        }
                    }
                    if (ride.status !in setOf("completed", "cancelled", "no_show")) {
                        OutlinedButton(
                            onClick = {
                                reasonText = ""
                                reasonMode = StaffReasonMode.CANCEL
                            },
                            enabled = !state.busy,
                            colors = ButtonDefaults.outlinedButtonColors(contentColor = MaterialTheme.colorScheme.error),
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Text(stringResource(R.string.ride_staff_cancel))
                        }
                    }
                }

                if (canToggleReimb) {
                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                    Row(
                        Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text(
                            stringResource(R.string.ride_staff_reimb_exclude),
                            style = MaterialTheme.typography.bodyMedium,
                            modifier = Modifier.weight(1f),
                        )
                        Switch(
                            checked = ride.reimbursementExcluded,
                            onCheckedChange = { vm.staffSetReimbursementExcluded(it) },
                            enabled = !state.busy,
                        )
                    }
                }
            }
        }
    }

    // 2. Proposed Single-Ride Change Drafts
    if (canEditDraft || canApproveLead || state.drafts.isNotEmpty()) {
        OutlinedCard(
            Modifier.fillMaxWidth(),
            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
        ) {
            Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                Row(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        modifier = Modifier.weight(1f),
                    ) {
                        Icon(Icons.Outlined.EditNote, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                        Text(stringResource(R.string.ride_draft_section), style = MaterialTheme.typography.titleMedium)
                    }
                    if (canEditDraft) {
                        OutlinedButton(
                            onClick = {
                                editingDraft = null
                                showDraftEditor = true
                            },
                            enabled = !state.draftBusy,
                        ) {
                            Icon(Icons.Outlined.Add, contentDescription = null, modifier = Modifier.size(16.dp))
                            Spacer(Modifier.width(4.dp))
                            Text(stringResource(R.string.ride_draft_new))
                        }
                    }
                }

                state.drafts.forEach { draft ->
                    val lockedByOther = draft.lockedBy != null && !draft.lockIsMine
                    val editable = draft.status in setOf("draft", "returned") && !lockedByOther && canEditDraft

                    OutlinedCard(
                        Modifier.fillMaxWidth(),
                        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                    ) {
                        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                            Row(
                                Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Text(
                                    "v${draft.version} · ${rideDraftActionLabel(draft.action)}${draft.driverName?.let { " → $it" } ?: ""}",
                                    style = MaterialTheme.typography.titleSmall,
                                    modifier = Modifier.weight(1f),
                                )
                                PlanDraftStatusPill(draft.status)
                            }
                            if (draft.reason.isNotBlank()) {
                                Text(draft.reason, style = MaterialTheme.typography.bodySmall)
                            }
                            draft.reviewNotes?.takeIf { it.isNotBlank() }?.let { notes ->
                                CalloutBanner(
                                    icon = Icons.Outlined.EditNote,
                                    text = stringResource(
                                        R.string.plan_returned_banner,
                                        draft.reviewedBy?.name.orEmpty(),
                                        notes,
                                    ),
                                    containerColor = c.warningSoft,
                                    contentColor = c.warning,
                                )
                            }
                            if (lockedByOther && draft.status in setOf("draft", "returned")) {
                                Row(
                                    Modifier.fillMaxWidth(),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                                ) {
                                    Icon(Icons.Outlined.Lock, contentDescription = null, tint = c.warning, modifier = Modifier.size(16.dp))
                                    Text(
                                        stringResource(
                                            R.string.plan_lock_other,
                                            draft.lockedBy?.name.orEmpty(),
                                            draft.lockExpiresAt?.let(KlTime::displayTime).orEmpty(),
                                        ),
                                        style = MaterialTheme.typography.bodySmall,
                                        modifier = Modifier.weight(1f),
                                    )
                                    TextButton(onClick = { vm.acquireRideDraftLock(draft.id, force = true) }) {
                                        Text(stringResource(R.string.plan_lock_takeover))
                                    }
                                }
                            } else if (draft.lockIsMine && draft.status in setOf("draft", "returned")) {
                                Text(
                                    stringResource(R.string.plan_lock_mine),
                                    style = MaterialTheme.typography.labelSmall,
                                    color = c.info,
                                )
                            }

                            FlowRow(
                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                            ) {
                                if (editable) {
                                    OutlinedButton(
                                        onClick = {
                                            editingDraft = draft
                                            showDraftEditor = true
                                        },
                                        enabled = !state.draftBusy,
                                    ) {
                                        Icon(Icons.Outlined.Edit, contentDescription = null, modifier = Modifier.size(16.dp))
                                        Spacer(Modifier.width(4.dp))
                                        Text(stringResource(R.string.ride_draft_edit))
                                    }
                                    Button(
                                        onClick = { vm.requestRideDraftApproval(draft.id) },
                                        enabled = !state.draftBusy,
                                    ) {
                                        Text(stringResource(R.string.ride_draft_submit))
                                    }
                                    IconButton(
                                        onClick = { confirmDeleteDraft = draft },
                                        enabled = !state.draftBusy,
                                    ) {
                                        Icon(Icons.Outlined.DeleteOutline, contentDescription = stringResource(R.string.ride_draft_delete))
                                    }
                                }
                                if (canApproveLead && draft.status == "pending_approval") {
                                    Button(
                                        onClick = { vm.approveRideDraft(draft.id) },
                                        enabled = !state.draftBusy,
                                        colors = graceRidePrimaryButtonColors(),
                                    ) {
                                        Text(stringResource(R.string.ride_draft_approve))
                                    }
                                    OutlinedButton(
                                        onClick = {
                                            returnNotes = ""
                                            returningDraft = draft
                                        },
                                        enabled = !state.draftBusy,
                                    ) {
                                        Text(stringResource(R.string.ride_draft_return))
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if (showReassignSheet) {
        StaffDriverPickerSheet(
            drivers = state.drivers,
            currentDriverId = ride.driver?.id,
            rideAreaName = ride.area.name,
            onSelect = { driverId ->
                vm.staffReassign(driverId)
                showReassignSheet = false
            },
            onDismiss = { showReassignSheet = false },
        )
    }

    reasonMode?.let { mode ->
        val titleRes = when (mode) {
            StaffReasonMode.RETURN -> R.string.ride_staff_return
            StaffReasonMode.ALT_TRANSPORT -> R.string.ride_staff_alt_transport
            StaffReasonMode.CANCEL -> R.string.ride_staff_cancel
        }
        AlertDialog(
            onDismissRequest = { reasonMode = null },
            title = { Text(stringResource(titleRes)) },
            text = {
                OutlinedTextField(
                    value = reasonText,
                    onValueChange = { reasonText = it },
                    label = { Text(stringResource(R.string.ride_cancel_reason_hint)) },
                    minLines = 2,
                    modifier = Modifier.fillMaxWidth(),
                )
            },
            confirmButton = {
                Button(
                    onClick = {
                        when (mode) {
                            StaffReasonMode.RETURN -> vm.staffReturn(reasonText)
                            StaffReasonMode.ALT_TRANSPORT -> vm.staffAltTransport(reasonText)
                            StaffReasonMode.CANCEL -> vm.cancel(reasonText)
                        }
                        reasonMode = null
                    },
                ) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = {
                TextButton(onClick = { reasonMode = null }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }

    if (showDraftEditor) {
        RideDraftEditorSheet(
            initial = editingDraft,
            ride = ride,
            drivers = state.drivers,
            busy = state.draftBusy,
            onSave = { req ->
                val cur = editingDraft
                if (cur == null) vm.createRideDraft(req) else vm.updateRideDraft(cur.id, req)
                showDraftEditor = false
            },
            onDismiss = { showDraftEditor = false },
        )
    }

    returningDraft?.let { draft ->
        ReturnNotesDialog(
            notes = returnNotes,
            onNotesChange = { returnNotes = it },
            quickReasons = listOf(
                stringResource(R.string.ride_draft_quick_reason_driver),
                stringResource(R.string.ride_draft_quick_reason_area),
                stringResource(R.string.ride_draft_quick_reason_details),
            ),
            onConfirm = {
                vm.returnRideDraft(draft.id, returnNotes)
                returningDraft = null
            },
            onDismiss = { returningDraft = null },
        )
    }

    confirmDeleteDraft?.let { draft ->
        AlertDialog(
            onDismissRequest = { confirmDeleteDraft = null },
            title = { Text(stringResource(R.string.ride_draft_delete)) },
            text = { Text(stringResource(R.string.ride_draft_delete_confirm, draft.version)) },
            confirmButton = {
                Button(
                    onClick = {
                        vm.deleteRideDraft(draft.id)
                        confirmDeleteDraft = null
                    },
                ) { Text(stringResource(R.string.common_delete)) }
            },
            dismissButton = {
                TextButton(onClick = { confirmDeleteDraft = null }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }

    state.nextPendingRide?.let { nextRide ->
        AlertDialog(
            onDismissRequest = vm::dismissNextPending,
            title = { Text(stringResource(R.string.ride_staff_next_pending_title)) },
            text = { Text(stringResource(R.string.ride_staff_next_pending_body, nextRide.id, nextRide.passenger.name)) },
            confirmButton = {
                Button(
                    onClick = {
                        vm.dismissNextPending()
                        onOpenRide(nextRide.id)
                    },
                ) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = {
                TextButton(onClick = vm::dismissNextPending) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
private fun StaffDriverPickerSheet(
    drivers: List<AdminDriverDto>,
    currentDriverId: Long?,
    rideAreaName: String = "",
    onSelect: (Long) -> Unit,
    onDismiss: () -> Unit,
) {
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
        Column(
            Modifier
                .fillMaxWidth()
                .imePadding()
                .navigationBarsPadding()
                .padding(horizontal = Spacing.md)
                .padding(bottom = Spacing.md)
                .heightIn(max = 520.dp),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Text(stringResource(R.string.ride_staff_reassign), style = MaterialTheme.typography.titleLarge)
            StaffDriverPickerContent(
                drivers = drivers,
                currentDriverId = currentDriverId,
                rideAreaName = rideAreaName,
                onSelect = onSelect,
            )
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun StaffDriverPickerContent(
    drivers: List<AdminDriverDto>,
    currentDriverId: Long?,
    rideAreaName: String,
    onSelect: (Long) -> Unit,
    modifier: Modifier = Modifier,
) {
    var query by remember { mutableStateOf("") }
    val filteredDrivers = remember(drivers, query, rideAreaName, currentDriverId) {
        val q = query.trim()
        val matching = if (q.isEmpty()) {
            drivers
        } else {
            drivers.filter { drv ->
                drv.name.contains(q, ignoreCase = true) ||
                    drv.vehicle?.model?.contains(q, ignoreCase = true) == true ||
                    drv.areas.any { it.contains(q, ignoreCase = true) }
            }
        }
        matching.sortedWith(
            compareByDescending<AdminDriverDto> {
                rideAreaName.isNotBlank() && it.areas.any { a -> a.equals(rideAreaName, ignoreCase = true) }
            }
                .thenByDescending { it.id == currentDriverId }
                .thenByDescending { it.spareSeats },
        )
    }
    val c = LocalStatusColors.current
    Column(modifier = modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
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
        if (filteredDrivers.isEmpty()) {
            EmptyView(text = stringResource(R.string.common_empty_title))
        } else {
            LazyColumn(
                modifier = Modifier.weight(1f, fill = false),
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                items(filteredDrivers, key = { "stf-drv-${it.id}" }) { drv ->
                    val sameArea = rideAreaName.isNotBlank() &&
                        drv.areas.any { it.equals(rideAreaName, ignoreCase = true) }
                    val isCurrent = drv.id == currentDriverId
                    OutlinedCard(
                        onClick = { onSelect(drv.id) },
                        modifier = Modifier
                            .fillMaxWidth()
                            .animateItem(),
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
                                    drv.name,
                                    style = MaterialTheme.typography.titleMedium,
                                    modifier = Modifier.weight(1f),
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                )
                                Text(
                                    stringResource(R.string.ride_staff_spare_seats, drv.spareSeats),
                                    style = MaterialTheme.typography.labelMedium,
                                    color = MaterialTheme.colorScheme.primary,
                                )
                            }
                            val subtitle = buildList {
                                drv.vehicle?.model?.takeIf { it.isNotBlank() }?.let { add(it) }
                                if (drv.areas.isNotEmpty()) add(drv.areas.joinToString(", "))
                            }.joinToString(" · ")
                            if (subtitle.isNotBlank()) {
                                Text(
                                    subtitle,
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                            if (isCurrent || sameArea) {
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
                                    if (sameArea) {
                                        SmallBadgePill(
                                            stringResource(R.string.plan_driver_same_area),
                                            c.successSoft,
                                            c.success,
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

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
private fun RideDraftEditorSheet(
    initial: RideChangeDraftDto?,
    ride: Ride,
    drivers: List<AdminDriverDto>,
    busy: Boolean,
    onSave: (RideDraftInReq) -> Unit,
    onDismiss: () -> Unit,
) {
    val actions = remember(ride.status, ride.driver?.id, initial?.action) {
        legalRideDraftActions(ride, initial?.action)
    }
    val defaultAction = initial?.action ?: actions.firstOrNull() ?: "assign"
    var action by remember(initial) { mutableStateOf(defaultAction) }
    var driverId by remember(initial) { mutableStateOf(initial?.driverId ?: ride.driver?.id) }
    var reason by remember(initial) { mutableStateOf(initial?.reason.orEmpty()) }
    var reimbExcluded by remember(initial) { mutableStateOf(initial?.reimbursementExcluded ?: ride.reimbursementExcluded) }
    var showDriverPicker by remember { mutableStateOf(false) }

    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
        BackHandler(enabled = showDriverPicker) { showDriverPicker = false }
        AnimatedContent(
            targetState = showDriverPicker,
            transitionSpec = {
                if (animatorsEnabled()) {
                    val dir = if (targetState) 1 else -1
                    (slideInHorizontally(tween(220, easing = GraceRideSpringEasing)) { it * dir } +
                        fadeIn(tween(160)))
                        .togetherWith(
                            slideOutHorizontally(tween(200, easing = GraceRideSpringEasing)) { (-it / 3) * dir } +
                                fadeOut(tween(120)),
                        )
                } else {
                    fadeIn(tween(120)).togetherWith(fadeOut(tween(100)))
                }
            },
            label = "ride_draft_step",
        ) { pickerOpen ->
            if (pickerOpen) {
                Column(
                    Modifier
                        .fillMaxWidth()
                        .imePadding()
                        .navigationBarsPadding()
                        .padding(horizontal = Spacing.md)
                        .padding(bottom = Spacing.md)
                        .heightIn(max = 520.dp),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    Row(
                        Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xxs),
                    ) {
                        IconButton(onClick = { showDriverPicker = false }) {
                            Icon(
                                Icons.AutoMirrored.Filled.ArrowBack,
                                contentDescription = stringResource(R.string.a11y_back),
                            )
                        }
                        Text(
                            stringResource(R.string.ride_staff_reassign),
                            style = MaterialTheme.typography.titleLarge,
                        )
                    }
                    StaffDriverPickerContent(
                        drivers = drivers,
                        currentDriverId = driverId,
                        rideAreaName = ride.area.name,
                        onSelect = { selected ->
                            driverId = selected
                            showDriverPicker = false
                        },
                        modifier = Modifier.weight(1f, fill = false),
                    )
                }
            } else {
                Column(
                    Modifier
                        .fillMaxWidth()
                        .imePadding()
                        .navigationBarsPadding()
                        .padding(horizontal = Spacing.md)
                        .padding(bottom = Spacing.md),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    Text(
                        stringResource(if (initial == null) R.string.ride_draft_new else R.string.ride_draft_edit),
                        style = MaterialTheme.typography.titleLarge,
                    )
                    Column(
                        Modifier
                            .weight(1f, fill = false)
                            .verticalScroll(rememberScrollState()),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        FlowRow(
                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                            verticalArrangement = Arrangement.spacedBy(6.dp),
                        ) {
                            actions.forEach { act ->
                                FilterChip(
                                    selected = action == act,
                                    onClick = { action = act },
                                    label = { Text(rideDraftActionLabel(act), maxLines = 1, overflow = TextOverflow.Ellipsis) },
                                )
                            }
                        }
                        if (action == "assign" || action == "reassign") {
                            val chosen = drivers.firstOrNull { it.id == driverId }
                            OutlinedCard(
                                onClick = { showDriverPicker = true },
                                modifier = Modifier.fillMaxWidth(),
                                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                            ) {
                                Row(
                                    Modifier
                                        .fillMaxWidth()
                                        .padding(Spacing.sm),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Column(Modifier.weight(1f)) {
                                        Text(
                                            stringResource(R.string.ride_staff_reassign),
                                            style = MaterialTheme.typography.labelMedium,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                        Text(
                                            chosen?.name ?: stringResource(R.string.plan_driver_tap_to_assign),
                                            style = MaterialTheme.typography.titleSmall,
                                            color = if (chosen != null) MaterialTheme.colorScheme.onSurface else MaterialTheme.colorScheme.primary,
                                        )
                                    }
                                    Icon(
                                        Icons.Outlined.ChevronRight,
                                        contentDescription = null,
                                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                            }
                        }
                        if (action == "reimbursement_excluded") {
                            Row(
                                Modifier.fillMaxWidth(),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.SpaceBetween,
                            ) {
                                Text(stringResource(R.string.ride_staff_reimb_exclude), style = MaterialTheme.typography.bodyMedium)
                                Switch(checked = reimbExcluded, onCheckedChange = { reimbExcluded = it })
                            }
                        }
                        OutlinedTextField(
                            value = reason,
                            onValueChange = { reason = it },
                            label = { Text(stringResource(R.string.ride_cancel_reason_hint)) },
                            minLines = 2,
                            modifier = Modifier.fillMaxWidth(),
                        )
                    }
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        OutlinedButton(
                            onClick = {
                                onSave(
                                    RideDraftInReq(
                                        action = action,
                                        driver_id = if (action in setOf("assign", "reassign")) driverId else null,
                                        reason = reason.trim(),
                                        reimbursement_excluded = if (action == "reimbursement_excluded") reimbExcluded else null,
                                        submit = false,
                                    ),
                                )
                            },
                            enabled = !busy && (action !in setOf("assign", "reassign") || driverId != null),
                            modifier = Modifier.weight(1f),
                        ) { Text(stringResource(R.string.ride_draft_save)) }
                        Button(
                            onClick = {
                                onSave(
                                    RideDraftInReq(
                                        action = action,
                                        driver_id = if (action in setOf("assign", "reassign")) driverId else null,
                                        reason = reason.trim(),
                                        reimbursement_excluded = if (action == "reimbursement_excluded") reimbExcluded else null,
                                        submit = true,
                                    ),
                                )
                            },
                            enabled = !busy && (action !in setOf("assign", "reassign") || driverId != null),
                            colors = graceRidePrimaryButtonColors(),
                            modifier = Modifier.weight(1f),
                        ) { Text(stringResource(R.string.ride_draft_submit)) }
                    }
                }
            }
        }
    }
}
