package org.pjstatecm.graceride.ui.screens.events

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material.icons.outlined.Share
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.AdminUserDto
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.PostEventClaimReq
import org.pjstatecm.graceride.data.api.PostEventDriversDto
import org.pjstatecm.graceride.data.api.PostEventReimbursementsReq
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.platform.QrCodeCanvas
import org.pjstatecm.graceride.ui.copyToClipboard
import org.pjstatecm.graceride.ui.shareText
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import kotlin.math.roundToLong

@Composable
fun VenueCheckinQrDialog(
    event: EventItem,
    checkinUrl: String,
    busy: Boolean,
    onResetCode: () -> Unit,
    onDismiss: () -> Unit,
    onCopied: () -> Unit,
) {
    org.pjstatecm.graceride.ui.SecureScreenEffect()
    val context = LocalContext.current
    var confirmReset by remember { mutableStateOf(false) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.event_venue_qr_btn)) },
        text = {
            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                Text(event.title, style = MaterialTheme.typography.titleSmall)
                Text(
                    stringResource(R.string.event_venue_qr_desc),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                QrCodeCanvas(
                    content = checkinUrl,
                    modifier = Modifier.size(210.dp),
                )
                event.checkinCode?.let { code ->
                    Text(
                        code,
                        style = MaterialTheme.typography.titleLarge.copy(fontFamily = FontFamily.Monospace),
                    )
                }
                Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    OutlinedButton(
                        onClick = {
                            if (copyToClipboard(context, "Check-in URL", checkinUrl, sensitive = true)) {
                                onCopied()
                            }
                        },
                    ) {
                        Icon(Icons.Outlined.ContentCopy, contentDescription = null, modifier = Modifier.size(16.dp))
                        Spacer(Modifier.width(Spacing.xxs))
                        Text(stringResource(R.string.common_copy))
                    }
                    OutlinedButton(onClick = { shareText(context, checkinUrl, subject = event.title) }) {
                        Icon(Icons.Outlined.Share, contentDescription = null, modifier = Modifier.size(16.dp))
                        Spacer(Modifier.width(Spacing.xxs))
                        Text(stringResource(R.string.family_share_link))
                    }
                }
                TextButton(
                    onClick = { confirmReset = true },
                    enabled = !busy,
                ) {
                    Icon(Icons.Outlined.Refresh, contentDescription = null, modifier = Modifier.size(16.dp))
                    Spacer(Modifier.width(Spacing.xxs))
                    Text(stringResource(R.string.event_venue_qr_reset))
                }
            }
        },
        confirmButton = {
            TextButton(onClick = onDismiss) {
                Text(stringResource(R.string.common_close))
            }
        },
    )

    if (confirmReset) {
        AlertDialog(
            onDismissRequest = { confirmReset = false },
            title = { Text(stringResource(R.string.event_venue_qr_reset)) },
            text = { Text(stringResource(R.string.event_venue_qr_reset_confirm)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmReset = false
                        onResetCode()
                    },
                ) {
                    Text(stringResource(R.string.common_confirm), color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmReset = false }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }
}

@Composable
fun WalkInCheckinDialog(
    query: String,
    candidates: List<AdminUserDto>,
    busy: Boolean,
    onQueryChange: (String) -> Unit,
    onPickUser: (Long) -> Unit,
    onDismiss: () -> Unit,
) {
    org.pjstatecm.graceride.ui.SecureScreenEffect()
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.event_walk_in_btn)) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                OutlinedTextField(
                    value = query,
                    onValueChange = onQueryChange,
                    label = { Text(stringResource(R.string.event_walk_in_search)) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                Column(
                    Modifier
                        .fillMaxWidth()
                        .height(220.dp)
                        .verticalScroll(rememberScrollState()),
                ) {
                    candidates.take(12).forEach { user ->
                        Surface(
                            onClick = { if (!busy) onPickUser(user.id) },
                            color = MaterialTheme.colorScheme.surface,
                        ) {
                            Row(
                                Modifier
                                    .fillMaxWidth()
                                    .padding(vertical = Spacing.xs, horizontal = Spacing.xxs),
                                horizontalArrangement = Arrangement.SpaceBetween,
                            ) {
                                Text(user.name, style = MaterialTheme.typography.bodyMedium)
                                if (user.groups.isNotBlank()) {
                                    Text(
                                        user.groups,
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                            }
                        }
                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                    }
                }
            }
        },
        confirmButton = {
            TextButton(onClick = onDismiss) {
                Text(stringResource(R.string.common_close))
            }
        },
    )
}

private data class DriverClaimRowUi(
    val selected: Boolean,
    val tripType: String,
    val destAreaId: Long?,
    val amountInput: String,
)

private fun parseRmToCents(raw: String): Long {
    val v = raw.trim().toDoubleOrNull() ?: return 0L
    return (v * 100.0).roundToLong().coerceAtLeast(0L)
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun PostEventReimbSheet(
    data: PostEventDriversDto,
    busy: Boolean,
    onSubmit: (PostEventReimbursementsReq) -> Unit,
    onDismiss: () -> Unit,
) {
    org.pjstatecm.graceride.ui.SecureScreenEffect()
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    var supperAreaId by remember { mutableStateOf<Long?>(null) }
    var supperAreaExpanded by remember { mutableStateOf(false) }
    var supperNote by remember { mutableStateOf("") }
    val rowMap = remember { mutableStateMapOf<Long, DriverClaimRowUi>() }

    LaunchedEffect(data.eventId, data.drivers) {
        data.drivers.forEach { d ->
            val destId = d.defaultArea?.id
            val fareCents = data.availableAreas.firstOrNull { it.id == destId }?.fareCents ?: 0L
            rowMap[d.id] = DriverClaimRowUi(
                selected = !d.alreadyClaimed && !d.reimbursementExcluded,
                tripType = d.suggestedTripType,
                destAreaId = destId,
                amountInput = "%.2f".format(fareCents / 100.0),
            )
        }
    }

    val claims = data.drivers.mapNotNull { d ->
        val row = rowMap[d.id] ?: return@mapNotNull null
        val areaId = row.destAreaId
        val cents = parseRmToCents(row.amountInput)
        if (!d.alreadyClaimed && row.selected && areaId != null && cents > 0L) {
            PostEventClaimReq(
                driver_id = d.id,
                trip_type = row.tripType,
                destination_area_id = areaId,
                amount_cents = cents,
            )
        } else {
            null
        }
    }
    val totalCents = claims.sumOf { it.amount_cents }

    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
        Column(
            Modifier
                .fillMaxWidth()
                .navigationBarsPadding()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = Spacing.sm)
                .padding(bottom = Spacing.md),
            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
        ) {
            Text(stringResource(R.string.event_post_reimb_title), style = MaterialTheme.typography.titleLarge)
            Text(
                "${data.eventTitle}${data.eventArea?.name?.let { " · $it" }.orEmpty()}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            if (data.drivers.isEmpty()) {
                Text(
                    stringResource(R.string.event_post_reimb_empty),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            } else {
                ExposedDropdownMenuBox(
                    expanded = supperAreaExpanded,
                    onExpandedChange = { supperAreaExpanded = it },
                ) {
                    val selectedSupper = data.availableAreas.firstOrNull { it.id == supperAreaId }
                    OutlinedTextField(
                        value = selectedSupper?.let { "${it.name}${it.fareCents?.let { f -> " (${Money.rm(f)})" }.orEmpty()}" } ?: "",
                        onValueChange = {},
                        readOnly = true,
                        label = { Text(stringResource(R.string.event_post_reimb_supper_area)) },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = supperAreaExpanded) },
                        modifier = Modifier
                            .menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable)
                            .fillMaxWidth(),
                    )
                    ExposedDropdownMenu(
                        expanded = supperAreaExpanded,
                        onDismissRequest = { supperAreaExpanded = false },
                    ) {
                        data.availableAreas.forEach { a ->
                            DropdownMenuItem(
                                text = { Text("${a.name}${a.fareCents?.let { f -> " (${Money.rm(f)})" }.orEmpty()}") },
                                onClick = {
                                    supperAreaId = a.id
                                    supperAreaExpanded = false
                                    val fare = a.fareCents ?: 0L
                                    data.drivers.forEach { d ->
                                        val cur = rowMap[d.id]
                                        if (cur != null && cur.tripType == "supper") {
                                            rowMap[d.id] = cur.copy(
                                                destAreaId = a.id,
                                                amountInput = "%.2f".format(fare / 100.0),
                                            )
                                        }
                                    }
                                },
                            )
                        }
                    }
                }
                OutlinedTextField(
                    value = supperNote,
                    onValueChange = { supperNote = it },
                    label = { Text(stringResource(R.string.event_post_reimb_supper_note)) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )

                data.drivers.forEach { d ->
                    val row = rowMap[d.id] ?: return@forEach
                    var destExpanded by remember(d.id) { mutableStateOf(false) }
                    OutlinedCard(
                        modifier = Modifier.fillMaxWidth(),
                        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                    ) {
                        Column(
                            Modifier.padding(Spacing.sm),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            Row(
                                Modifier.fillMaxWidth(),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.SpaceBetween,
                            ) {
                                Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.weight(1f)) {
                                    Checkbox(
                                        checked = row.selected,
                                        enabled = !d.alreadyClaimed && !busy,
                                        onCheckedChange = { checked -> rowMap[d.id] = row.copy(selected = checked) },
                                    )
                                    Column {
                                        Text(d.name, style = MaterialTheme.typography.titleSmall)
                                        d.vehicle?.let { v ->
                                            Text(v, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                        }
                                    }
                                }
                                when {
                                    d.alreadyClaimed -> Text(
                                        stringResource(R.string.event_post_reimb_already_claimed, Money.rm(d.claimedAmountCents ?: 0L)),
                                        style = MaterialTheme.typography.labelMedium,
                                        color = MaterialTheme.colorScheme.primary,
                                    )
                                    d.reimbursementExcluded -> Text(
                                        stringResource(R.string.event_post_reimb_excluded),
                                        style = MaterialTheme.typography.labelSmall,
                                        color = MaterialTheme.colorScheme.error,
                                    )
                                }
                            }
                            if (!d.alreadyClaimed) {
                                FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                    FilterChip(
                                        selected = row.tripType == "supper",
                                        onClick = {
                                            val targetArea = supperAreaId ?: d.defaultArea?.id
                                            val fare = data.availableAreas.firstOrNull { it.id == targetArea }?.fareCents ?: 0L
                                            rowMap[d.id] = row.copy(
                                                tripType = "supper",
                                                destAreaId = targetArea,
                                                amountInput = "%.2f".format(fare / 100.0),
                                            )
                                        },
                                        enabled = row.selected && !busy,
                                        label = { Text(stringResource(R.string.event_post_reimb_trip_supper)) },
                                    )
                                    FilterChip(
                                        selected = row.tripType == "direct_return",
                                        onClick = {
                                            val targetArea = d.defaultArea?.id
                                            val fare = data.availableAreas.firstOrNull { it.id == targetArea }?.fareCents ?: 0L
                                            rowMap[d.id] = row.copy(
                                                tripType = "direct_return",
                                                destAreaId = targetArea,
                                                amountInput = "%.2f".format(fare / 100.0),
                                            )
                                        },
                                        enabled = row.selected && !busy,
                                        label = { Text(stringResource(R.string.event_post_reimb_trip_direct)) },
                                    )
                                }
                                ExposedDropdownMenuBox(
                                    expanded = destExpanded,
                                    onExpandedChange = { if (row.selected && !busy) destExpanded = it },
                                ) {
                                    val destArea = data.availableAreas.firstOrNull { it.id == row.destAreaId }
                                    OutlinedTextField(
                                        value = destArea?.name ?: "",
                                        onValueChange = {},
                                        readOnly = true,
                                        enabled = row.selected && !busy,
                                        label = { Text(stringResource(R.string.visitor_pickup_area)) },
                                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = destExpanded) },
                                        modifier = Modifier
                                            .menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable)
                                            .fillMaxWidth(),
                                    )
                                    ExposedDropdownMenu(
                                        expanded = destExpanded,
                                        onDismissRequest = { destExpanded = false },
                                    ) {
                                        data.availableAreas.forEach { a ->
                                            DropdownMenuItem(
                                                text = { Text("${a.name}${a.fareCents?.let { f -> " (${Money.rm(f)})" }.orEmpty()}") },
                                                onClick = {
                                                    val fare = a.fareCents ?: 0L
                                                    rowMap[d.id] = row.copy(
                                                        destAreaId = a.id,
                                                        amountInput = "%.2f".format(fare / 100.0),
                                                    )
                                                    destExpanded = false
                                                },
                                            )
                                        }
                                    }
                                }
                                OutlinedTextField(
                                    value = row.amountInput,
                                    onValueChange = { txt -> rowMap[d.id] = row.copy(amountInput = txt) },
                                    enabled = row.selected && !busy,
                                    label = { Text(stringResource(R.string.event_post_reimb_amount)) },
                                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                                    singleLine = true,
                                    modifier = Modifier.fillMaxWidth(),
                                )
                            }
                        }
                    }
                }

                Button(
                    onClick = {
                        onSubmit(
                            PostEventReimbursementsReq(
                                supper_note = supperNote.trim().takeIf { it.isNotEmpty() },
                                claims = claims,
                            ),
                        )
                    },
                    enabled = claims.isNotEmpty() && !busy,
                    colors = graceRidePrimaryButtonColors(),
                    modifier = Modifier.fillMaxWidth().height(50.dp),
                ) {
                    Text(stringResource(R.string.event_post_reimb_submit, claims.size, Money.rm(totalCents)))
                }
            }
        }
    }
}
