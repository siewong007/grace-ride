package org.pjstatecm.graceride.ui.screens.traffic

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.DeleteOutline
import androidx.compose.material.icons.outlined.Edit
import androidx.compose.material.icons.outlined.Payments
import androidx.compose.material.icons.outlined.WarningAmber
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.Area
import org.pjstatecm.graceride.data.api.PlanDataDto
import org.pjstatecm.graceride.data.api.PlanDriverDto
import org.pjstatecm.graceride.data.api.PlanVisitorDto
import org.pjstatecm.graceride.data.api.VisitorReq
import org.pjstatecm.graceride.domain.EffectiveVisitor
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular
import java.math.BigDecimal

private const val SPECIAL_HIGH_CENTS = 10_000L

@Composable
internal fun PlanVisitorsHeader(
    count: Int,
    canEdit: Boolean,
    onAddClick: () -> Unit,
) {
    Row(
        Modifier.fillMaxWidth().padding(top = Spacing.xs),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            stringResource(R.string.plan_visitors_section, count),
            style = MaterialTheme.typography.titleMedium,
        )
        if (canEdit) {
            OutlinedButton(onClick = onAddClick) {
                Icon(Icons.Outlined.Add, contentDescription = null, modifier = Modifier.size(18.dp))
                Spacer(Modifier.width(Spacing.xxs))
                Text(stringResource(R.string.plan_visitor_add))
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun EffectiveVisitorCard(
    visitor: EffectiveVisitor,
    canEdit: Boolean,
    canRequestSpecial: Boolean,
    modifier: Modifier = Modifier,
    onEdit: () -> Unit,
    onDelete: () -> Unit,
    onSpecialReimb: () -> Unit,
) {
    val c = LocalStatusColors.current
    val legLabel = stringResource(if (visitor.leg == "return") R.string.plan_leg_return else R.string.plan_leg_outbound)
    OutlinedCard(
        modifier.fillMaxWidth(),
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
    ) {
        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(Modifier.weight(1f)) {
                    Text(visitor.name, style = MaterialTheme.typography.titleSmall)
                    val meta = buildList {
                        add(legLabel)
                        add("${visitor.seats}")
                        visitor.areaName?.takeIf { it.isNotBlank() }?.let { add(it) }
                    }.joinToString(" · ")
                    Text(meta, style = MaterialTheme.typography.bodySmall.tabular(), color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                when (visitor.draftOp) {
                    "add" -> SmallBadgePill(stringResource(R.string.plan_visitor_staged_add), c.successSoft, c.success)
                    "update" -> SmallBadgePill(stringResource(R.string.plan_visitor_staged_update), c.infoSoft, c.info)
                    "delete" -> SmallBadgePill(stringResource(R.string.plan_visitor_staged_delete), c.dangerSoft, c.danger)
                }
            }
            if (visitor.pickupAddress.isNotBlank()) {
                Text(visitor.pickupAddress, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            visitor.driverName?.takeIf { it.isNotBlank() }?.let { dName ->
                Text(
                    dName,
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.primary,
                )
            }
            if (visitor.notes.isNotBlank()) {
                Text(visitor.notes, style = MaterialTheme.typography.bodySmall)
            }
            visitor.liveSource?.specialReimbursement?.let { reimb ->
                val cents = reimb.amountCents ?: 0L
                FlowRow(
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                ) {
                    StatusPill(reimbursementStatusLabel(reimb.status), reimbursementStatusKind(reimb.status))
                    Text(
                        "${Money.rm(cents)}${reimb.reason?.takeIf { it.isNotBlank() }?.let { " · $it" }.orEmpty()}",
                        style = MaterialTheme.typography.labelMedium.tabular(),
                        color = if (reimb.highAmount) c.warning else MaterialTheme.colorScheme.primary,
                    )
                }
            }
            FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs), verticalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
                if (canEdit && visitor.draftOp != "delete") {
                    IconButton(onClick = onEdit) {
                        Icon(Icons.Outlined.Edit, contentDescription = stringResource(R.string.plan_visitor_edit))
                    }
                    IconButton(onClick = onDelete) {
                        Icon(Icons.Outlined.DeleteOutline, contentDescription = stringResource(R.string.common_delete))
                    }
                }
                if (canRequestSpecial && visitor.liveSource != null && visitor.draftOp != "delete") {
                    val existing = visitor.liveSource.specialReimbursement
                    OutlinedButton(onClick = onSpecialReimb) {
                        Icon(Icons.Outlined.Payments, contentDescription = null, modifier = Modifier.size(16.dp))
                        Spacer(Modifier.width(Spacing.xxs))
                        Text(
                            stringResource(
                                if (existing != null) R.string.plan_special_reimb_resubmit else R.string.plan_special_reimb_btn,
                            ),
                        )
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun VisitorEditorDialog(
    initial: EffectiveVisitor?,
    areas: List<Area>,
    drivers: List<PlanDriverDto>,
    onDismiss: () -> Unit,
    onSave: (name: String, phone: String, notes: String, area: Area?, address: String, seats: Long, leg: String, driver: PlanDriverDto?) -> Unit,
) {
    var name by remember(initial) { mutableStateOf(initial?.name.orEmpty()) }
    var phone by remember(initial) { mutableStateOf(initial?.phone.orEmpty()) }
    var notes by remember(initial) { mutableStateOf(initial?.notes.orEmpty()) }
    var areaId by remember(initial) { mutableStateOf(initial?.areaId ?: areas.firstOrNull { it.active }?.id ?: areas.firstOrNull()?.id) }
    var address by remember(initial) { mutableStateOf(initial?.pickupAddress.orEmpty()) }
    var seats by remember(initial) { mutableIntStateOf((initial?.seats ?: 1L).toInt().coerceIn(1, 8)) }
    var leg by remember(initial) { mutableStateOf(initial?.leg ?: "outbound") }
    var driverId by remember(initial) { mutableStateOf(initial?.driverId) }
    var driverExpanded by remember { mutableStateOf(false) }

    val selectedDriver = drivers.firstOrNull { it.id == driverId }
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
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
                stringResource(if (initial == null) R.string.plan_visitor_add else R.string.plan_visitor_edit),
                style = MaterialTheme.typography.titleLarge,
            )
            Column(
                Modifier
                    .weight(1f, fill = false)
                    .verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text(stringResource(R.string.plan_visitor_name_hint)) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                OutlinedTextField(
                    value = phone,
                    onValueChange = { phone = it },
                    label = { Text(stringResource(R.string.plan_visitor_phone_hint)) },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Phone),
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                if (areas.isNotEmpty()) {
                    AreaDropdownField(
                        label = stringResource(R.string.traffic_area_name_hint),
                        areas = areas,
                        selectedId = areaId,
                        onSelect = { areaId = it },
                    )
                }
                OutlinedTextField(
                    value = address,
                    onValueChange = { address = it },
                    label = { Text(stringResource(R.string.plan_visitor_address_hint)) },
                    modifier = Modifier.fillMaxWidth(),
                )
                SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
                    SegmentedButton(
                        selected = leg == "outbound",
                        onClick = { leg = "outbound" },
                        shape = SegmentedButtonDefaults.itemShape(0, 2),
                    ) { Text(stringResource(R.string.plan_leg_outbound), maxLines = 1, overflow = TextOverflow.Ellipsis) }
                    SegmentedButton(
                        selected = leg == "return",
                        onClick = { leg = "return" },
                        shape = SegmentedButtonDefaults.itemShape(1, 2),
                    ) { Text(stringResource(R.string.plan_leg_return), maxLines = 1, overflow = TextOverflow.Ellipsis) }
                }
                Row(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(stringResource(R.string.plan_visitor_seats_label), style = MaterialTheme.typography.bodyMedium)
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        TextButton(onClick = { if (seats > 1) seats-- }, enabled = seats > 1) { Text("−") }
                        Text("$seats", style = MaterialTheme.typography.titleSmall.tabular())
                        TextButton(onClick = { if (seats < 8) seats++ }, enabled = seats < 8) { Text("+") }
                    }
                }
                ExposedDropdownMenuBox(
                    expanded = driverExpanded,
                    onExpandedChange = { driverExpanded = !driverExpanded },
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    OutlinedTextField(
                        value = selectedDriver?.name ?: "",
                        onValueChange = {},
                        readOnly = true,
                        label = { Text(stringResource(R.string.plan_visitor_driver_optional)) },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = driverExpanded) },
                        modifier = Modifier.menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
                    )
                    ExposedDropdownMenu(expanded = driverExpanded, onDismissRequest = { driverExpanded = false }) {
                        DropdownMenuItem(
                            text = { Text("—") },
                            onClick = {
                                driverId = null
                                driverExpanded = false
                            },
                        )
                        drivers.forEach { d ->
                            DropdownMenuItem(
                                text = { Text(d.name) },
                                onClick = {
                                    driverId = d.id
                                    driverExpanded = false
                                },
                            )
                        }
                    }
                }
                OutlinedTextField(
                    value = notes,
                    onValueChange = { notes = it },
                    label = { Text(stringResource(R.string.plan_visitor_notes_hint)) },
                    modifier = Modifier.fillMaxWidth(),
                )
            }
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                OutlinedButton(onClick = onDismiss, modifier = Modifier.weight(1f)) {
                    Text(stringResource(R.string.common_cancel))
                }
                Button(
                    onClick = {
                        val chosenArea = areas.firstOrNull { it.id == areaId }
                        onSave(name, phone, notes, chosenArea, address, seats.toLong(), leg, selectedDriver)
                    },
                    enabled = name.isNotBlank(),
                    colors = graceRidePrimaryButtonColors(),
                    modifier = Modifier.weight(1f),
                ) { Text(stringResource(R.string.common_save)) }
            }
        }
    }
}

@Composable
internal fun reimbursementStatusLabel(status: String?): String = when (status?.lowercase()) {
    "pending" -> stringResource(R.string.plan_reimb_status_pending)
    "approved" -> stringResource(R.string.plan_reimb_status_approved)
    "paid" -> stringResource(R.string.plan_reimb_status_paid)
    "rejected", "returned" -> stringResource(R.string.plan_reimb_status_rejected)
    else -> status.orEmpty()
}

internal fun reimbursementStatusKind(status: String?): RideStatusKind = when (status?.lowercase()) {
    "approved", "paid" -> RideStatusKind.ACTIVE
    "rejected", "returned" -> RideStatusKind.CANCELLED
    "pending" -> RideStatusKind.PENDING
    else -> RideStatusKind.DONE
}

@Composable
internal fun SpecialReimbursementDialog(
    visitor: PlanVisitorDto,
    onDismiss: () -> Unit,
    onSubmit: (amountCents: Long, reason: String) -> Unit,
) {
    val existing = visitor.specialReimbursement
    var amountText by remember(visitor.id) {
        mutableStateOf(
            existing?.amountCents?.let {
                BigDecimal(it).movePointLeft(2).stripTrailingZeros().toPlainString()
            }.orEmpty(),
        )
    }
    var reason by remember(visitor.id) { mutableStateOf(existing?.reason.orEmpty()) }
    val parsedCents = parseRmToCents(amountText)
    val c = LocalStatusColors.current

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("${stringResource(R.string.plan_special_reimb_title)} · ${visitor.name}") },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                OutlinedTextField(
                    value = amountText,
                    onValueChange = { amountText = it },
                    label = { Text(stringResource(R.string.plan_special_reimb_amount)) },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                if (parsedCents != null && parsedCents > SPECIAL_HIGH_CENTS) {
                    CalloutBanner(
                        icon = Icons.Outlined.WarningAmber,
                        text = stringResource(R.string.plan_special_reimb_high_warn),
                        containerColor = c.warningSoft,
                        contentColor = c.warning,
                    )
                }
                OutlinedTextField(
                    value = reason,
                    onValueChange = { reason = it },
                    label = { Text(stringResource(R.string.plan_special_reimb_reason)) },
                    minLines = 2,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        },
        confirmButton = {
            Button(
                onClick = { if (parsedCents != null && parsedCents > 0L) onSubmit(parsedCents, reason) },
                enabled = parsedCents != null && parsedCents > 0L,
            ) { Text(stringResource(R.string.common_confirm)) }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text(stringResource(R.string.common_cancel)) }
        },
    )
}

internal fun buildVisitorReq(
    name: String,
    phone: String,
    notes: String,
    area: Area?,
    address: String,
    seats: Long,
    leg: String,
    driver: PlanDriverDto?,
): VisitorReq = VisitorReq(
    name = name.trim(),
    phone = phone.trim(),
    notes = notes.trim(),
    area_id = area?.id,
    pickup_address = address.trim(),
    seats = seats,
    leg = leg,
    driver_id = driver?.id,
)

@Suppress("unused")
internal fun effectiveDrivers(plan: PlanDataDto): List<PlanDriverDto> = plan.drivers
