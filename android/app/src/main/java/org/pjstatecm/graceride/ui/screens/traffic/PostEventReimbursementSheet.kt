package org.pjstatecm.graceride.ui.screens.traffic

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
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
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
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
import org.pjstatecm.graceride.data.api.PostEventClaimReq
import org.pjstatecm.graceride.data.api.PostEventDriversDto
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.GraceRideSpringEasing
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.components.animatorsEnabled
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular
import java.math.BigDecimal

private data class DriverClaimDraftState(
    val selected: Boolean,
    val tripType: String,
    val areaId: Long,
    val amountText: String,
    val note: String,
)

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
internal fun PostEventReimbursementSheet(
    loading: Boolean,
    busy: Boolean,
    data: PostEventDriversDto?,
    onSubmit: (supperNote: String?, claims: List<PostEventClaimReq>) -> Unit,
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
                .heightIn(max = 580.dp),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Text(stringResource(R.string.plan_post_event_title), style = MaterialTheme.typography.titleLarge)
            when {
                loading || data == null -> LoadingView(Modifier.heightIn(min = 200.dp))
                data.drivers.isEmpty() -> EmptyView(text = stringResource(R.string.plan_post_event_no_drivers))
                else -> {
                    var supperNote by remember(data.eventId) { mutableStateOf("") }
                    val drafts = remember(data.eventId, data.drivers.size) {
                        mutableStateMapOf<Long, DriverClaimDraftState>().apply {
                            val fallbackArea = data.eventArea ?: data.availableAreas.firstOrNull()
                            data.drivers.forEach { d ->
                                val defArea = d.defaultArea ?: fallbackArea
                                val cents = defArea?.fareCents ?: 0L
                                put(
                                    d.id,
                                    DriverClaimDraftState(
                                        selected = !d.alreadyClaimed && !d.reimbursementExcluded && cents > 0L,
                                        tripType = if (d.suggestedTripType == "direct_return") "direct_return" else "supper",
                                        areaId = defArea?.id ?: 0L,
                                        amountText = if (cents > 0L) BigDecimal(cents).movePointLeft(2).stripTrailingZeros().toPlainString() else "",
                                        note = "",
                                    ),
                                )
                            }
                        }
                    }

                    OutlinedTextField(
                        value = supperNote,
                        onValueChange = { supperNote = it },
                        label = { Text(stringResource(R.string.plan_post_event_supper_note)) },
                        modifier = Modifier.fillMaxWidth(),
                    )

                    LazyColumn(
                        Modifier.weight(1f, fill = false),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        items(data.drivers, key = { "pe-drv-${it.id}" }) { drv ->
                            val rowState = drafts[drv.id] ?: return@items
                            var areaExpanded by remember { mutableStateOf(false) }
                            val chosenArea = data.availableAreas.firstOrNull { it.id == rowState.areaId } ?: drv.defaultArea
                            val c = LocalStatusColors.current

                            OutlinedCard(
                                Modifier
                                    .fillMaxWidth()
                                    .animateItem(),
                                border = BorderStroke(
                                    1.dp,
                                    if (rowState.selected && !drv.alreadyClaimed) MaterialTheme.colorScheme.primary.copy(alpha = 0.6f)
                                    else MaterialTheme.colorScheme.outlineVariant,
                                ),
                            ) {
                                Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                                    Row(
                                        Modifier.fillMaxWidth(),
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        Checkbox(
                                            checked = rowState.selected,
                                            onCheckedChange = { checked -> drafts[drv.id] = rowState.copy(selected = checked) },
                                            enabled = !drv.alreadyClaimed,
                                        )
                                        Column(Modifier.weight(1f)) {
                                            Text(drv.name, style = MaterialTheme.typography.titleMedium)
                                            drv.vehicle?.takeIf { it.isNotBlank() }?.let { v ->
                                                Text(v, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                            }
                                            Text(
                                                stringResource(
                                                    R.string.plan_post_event_pax_stats,
                                                    drv.passengerStats.total,
                                                    drv.passengerStats.supper,
                                                    drv.passengerStats.directHome,
                                                ),
                                                style = MaterialTheme.typography.labelSmall.tabular(),
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                            )
                                        }
                                        if (!rowState.selected && !drv.alreadyClaimed) {
                                            val previewCents = parseRmToCents(rowState.amountText)
                                            if (previewCents != null && previewCents > 0L) {
                                                Text(
                                                    Money.rm(previewCents),
                                                    style = MaterialTheme.typography.labelMedium.tabular(),
                                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                )
                                            }
                                        }
                                    }
                                    if (drv.alreadyClaimed) {
                                        FlowRow(
                                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                            verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                                        ) {
                                            StatusPill(
                                                reimbursementStatusLabel(drv.claimedStatus),
                                                reimbursementStatusKind(drv.claimedStatus),
                                            )
                                            Text(
                                                Money.rm(drv.claimedAmountCents ?: 0L),
                                                style = MaterialTheme.typography.labelMedium.tabular(),
                                                color = MaterialTheme.colorScheme.primary,
                                            )
                                        }
                                    } else {
                                        if (drv.excludedRideCount > 0) {
                                            Text(
                                                stringResource(R.string.plan_post_event_excluded, drv.excludedRideCount, drv.totalRideCount),
                                                style = MaterialTheme.typography.labelSmall,
                                                color = c.warning,
                                            )
                                        }
                                        AnimatedVisibility(
                                            visible = rowState.selected,
                                            enter = if (animatorsEnabled()) {
                                                expandVertically(tween(200, easing = GraceRideSpringEasing)) + fadeIn(tween(160))
                                            } else {
                                                fadeIn(tween(100))
                                            },
                                            exit = if (animatorsEnabled()) {
                                                shrinkVertically(tween(180, easing = GraceRideSpringEasing)) + fadeOut(tween(120))
                                            } else {
                                                fadeOut(tween(80))
                                            },
                                        ) {
                                            Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                                                if (data.eventArea != null && chosenArea != null) {
                                                    Text(
                                                        stringResource(
                                                            R.string.plan_post_event_route_fmt,
                                                            data.eventArea.name,
                                                            chosenArea.name,
                                                        ),
                                                        style = MaterialTheme.typography.labelMedium,
                                                        color = MaterialTheme.colorScheme.primary,
                                                    )
                                                }
                                                SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
                                                    SegmentedButton(
                                                        selected = rowState.tripType == "supper",
                                                        onClick = { drafts[drv.id] = rowState.copy(tripType = "supper") },
                                                        shape = SegmentedButtonDefaults.itemShape(0, 2),
                                                    ) {
                                                        Text(
                                                            stringResource(R.string.plan_post_event_trip_supper),
                                                            maxLines = 1,
                                                            overflow = TextOverflow.Ellipsis,
                                                        )
                                                    }
                                                    SegmentedButton(
                                                        selected = rowState.tripType == "direct_return",
                                                        onClick = { drafts[drv.id] = rowState.copy(tripType = "direct_return") },
                                                        shape = SegmentedButtonDefaults.itemShape(1, 2),
                                                    ) {
                                                        Text(
                                                            stringResource(R.string.plan_post_event_trip_direct),
                                                            maxLines = 1,
                                                            overflow = TextOverflow.Ellipsis,
                                                        )
                                                    }
                                                }
                                                ExposedDropdownMenuBox(
                                                    expanded = areaExpanded,
                                                    onExpandedChange = { areaExpanded = !areaExpanded },
                                                    modifier = Modifier.fillMaxWidth(),
                                                ) {
                                                    OutlinedTextField(
                                                        value = chosenArea?.name.orEmpty(),
                                                        onValueChange = {},
                                                        readOnly = true,
                                                        label = { Text(stringResource(R.string.traffic_fare_area_b)) },
                                                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = areaExpanded) },
                                                        modifier = Modifier
                                                            .menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable)
                                                            .fillMaxWidth(),
                                                    )
                                                    ExposedDropdownMenu(expanded = areaExpanded, onDismissRequest = { areaExpanded = false }) {
                                                        data.availableAreas.forEach { areaOpt ->
                                                            DropdownMenuItem(
                                                                text = {
                                                                    Text(
                                                                        "${areaOpt.name}${areaOpt.fareCents?.let { " · ${Money.rm(it)}" }.orEmpty()}",
                                                                    )
                                                                },
                                                                onClick = {
                                                                    val nextAmt = areaOpt.fareCents?.let {
                                                                        BigDecimal(it).movePointLeft(2).stripTrailingZeros().toPlainString()
                                                                    } ?: rowState.amountText
                                                                    drafts[drv.id] = rowState.copy(areaId = areaOpt.id, amountText = nextAmt)
                                                                    areaExpanded = false
                                                                },
                                                            )
                                                        }
                                                    }
                                                }
                                                OutlinedTextField(
                                                    value = rowState.amountText,
                                                    onValueChange = { drafts[drv.id] = rowState.copy(amountText = it) },
                                                    label = { Text(stringResource(R.string.plan_special_reimb_amount)) },
                                                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                                                    singleLine = true,
                                                    modifier = Modifier.fillMaxWidth(),
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    val readyClaims = drafts.entries.mapNotNull { (driverId, st) ->
                        if (!st.selected || st.areaId <= 0L) null
                        else {
                            val cents = parseRmToCents(st.amountText) ?: return@mapNotNull null
                            if (cents <= 0L) null
                            else PostEventClaimReq(
                                driver_id = driverId,
                                trip_type = st.tripType,
                                destination_area_id = st.areaId,
                                amount_cents = cents,
                                note = st.note.trim().ifBlank { null },
                            )
                        }
                    }
                    val totalCents = readyClaims.sumOf { it.amount_cents }

                    Button(
                        onClick = { onSubmit(supperNote, readyClaims) },
                        enabled = readyClaims.isNotEmpty() && !busy,
                        colors = graceRidePrimaryButtonColors(),
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        if (busy) {
                            InlineButtonSpinner()
                            Spacer(Modifier.width(Spacing.xs))
                        }
                        Text(
                            if (readyClaims.isNotEmpty()) {
                                stringResource(R.string.plan_post_event_submit_total, readyClaims.size, Money.rm(totalCents))
                            } else {
                                stringResource(R.string.plan_post_event_submit, 0)
                            },
                        )
                    }
                }
            }
        }
    }
}
