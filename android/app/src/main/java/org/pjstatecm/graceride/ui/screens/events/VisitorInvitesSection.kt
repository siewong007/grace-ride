package org.pjstatecm.graceride.ui.screens.events

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.PersonAdd
import androidx.compose.material.icons.outlined.Share
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
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
import org.pjstatecm.graceride.data.api.Area
import org.pjstatecm.graceride.data.api.CreateVisitorPassReq
import org.pjstatecm.graceride.data.api.VisitorPassDto
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.copyToClipboard
import org.pjstatecm.graceride.ui.shareText
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun VisitorInvitesSection(
    eventTitle: String,
    eventStartsAt: String,
    visitors: List<VisitorPassDto>,
    areas: List<Area>,
    busy: Boolean,
    lastSharePair: Pair<String, String>?,
    onCreateVisitor: (CreateVisitorPassReq) -> Unit,
    onReissueVisitor: (VisitorPassDto) -> Unit,
    onRevokeVisitor: (Long) -> Unit,
    onOpenRide: (Long) -> Unit,
    onDismissSharePair: () -> Unit,
    onCopiedToast: () -> Unit,
) {
    val context = LocalContext.current
    var createOpen by remember { mutableStateOf(false) }
    var pendingRevoke by remember { mutableStateOf<VisitorPassDto?>(null) }

    OutlinedCard(
        modifier = Modifier.fillMaxWidth(),
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
    ) {
        Column(
            Modifier.padding(Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            SectionHeader(title = stringResource(R.string.visitor_section_title), count = visitors.size)
            Text(
                stringResource(R.string.visitor_section_desc),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            visitors.forEachIndexed { idx, pass ->
                if (idx > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                val (stateLabel, stateKind) = when (pass.state) {
                    "claimed" -> stringResource(R.string.visitor_state_claimed) to RideStatusKind.ACTIVE
                    "not_yet" -> stringResource(R.string.visitor_state_not_yet, KlTime.display(pass.validFrom)) to RideStatusKind.PENDING
                    "closed" -> stringResource(R.string.visitor_state_closed) to RideStatusKind.CANCELLED
                    else -> stringResource(R.string.visitor_state_open) to RideStatusKind.PENDING
                }
                Column(verticalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
                    Row(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(pass.name, style = MaterialTheme.typography.titleSmall)
                        StatusPill(stateLabel, stateKind)
                    }
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        pass.rideId?.let { rid ->
                            TextButton(onClick = { onOpenRide(rid) }) {
                                Text(stringResource(R.string.visitor_view_ride))
                            }
                        }
                        if (pass.state != "closed") {
                            TextButton(
                                onClick = { onReissueVisitor(pass) },
                                enabled = !busy,
                            ) {
                                Text(stringResource(R.string.visitor_reissue))
                            }
                            TextButton(
                                onClick = { pendingRevoke = pass },
                                enabled = !busy,
                                colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                            ) {
                                Text(stringResource(R.string.visitor_revoke))
                            }
                        }
                    }
                }
            }

            OutlinedButton(
                onClick = { createOpen = true },
                enabled = !busy,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Icon(Icons.Outlined.PersonAdd, contentDescription = null, modifier = Modifier.size(18.dp))
                Spacer(Modifier.width(Spacing.xs))
                Text(stringResource(R.string.visitor_invite_btn))
            }
        }
    }

    if (createOpen) {
        val defaultDt = remember(eventStartsAt) {
            KlTime.parse(eventStartsAt)?.minusHours(1)
        }
        var name by remember { mutableStateOf("") }
        var phone by remember { mutableStateOf("") }
        var date by remember {
            mutableStateOf(
                defaultDt?.toLocalDate()?.toString() ?: eventStartsAt.take(10),
            )
        }
        var time by remember {
            mutableStateOf(
                defaultDt?.toLocalTime()?.toString()?.take(5) ?: "09:00",
            )
        }
        val activeAreas = remember(areas) { areas.filter { it.active } }
        var areaId by remember { mutableStateOf(activeAreas.firstOrNull()?.id) }
        var areaExpanded by remember { mutableStateOf(false) }
        var address by remember { mutableStateOf("") }
        var seats by remember { mutableIntStateOf(1) }

        AlertDialog(
            onDismissRequest = { createOpen = false },
            title = { Text(stringResource(R.string.visitor_invite_btn)) },
            text = {
                Column(
                    Modifier.verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    OutlinedTextField(
                        value = name,
                        onValueChange = { name = it },
                        label = { Text(stringResource(R.string.visitor_name_label)) },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    OutlinedTextField(
                        value = phone,
                        onValueChange = { phone = it },
                        label = { Text(stringResource(R.string.visitor_phone_label)) },
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Phone),
                        modifier = Modifier.fillMaxWidth(),
                    )
                    Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        OutlinedTextField(
                            value = date,
                            onValueChange = { date = it },
                            label = { Text(stringResource(R.string.visitor_pickup_date)) },
                            singleLine = true,
                            modifier = Modifier.weight(1f),
                        )
                        OutlinedTextField(
                            value = time,
                            onValueChange = { time = it },
                            label = { Text(stringResource(R.string.visitor_pickup_time)) },
                            singleLine = true,
                            modifier = Modifier.weight(1f),
                        )
                    }
                    ExposedDropdownMenuBox(
                        expanded = areaExpanded,
                        onExpandedChange = { areaExpanded = it },
                    ) {
                        OutlinedTextField(
                            value = activeAreas.firstOrNull { it.id == areaId }?.name ?: "",
                            onValueChange = {},
                            readOnly = true,
                            label = { Text(stringResource(R.string.visitor_pickup_area)) },
                            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = areaExpanded) },
                            modifier = Modifier
                                .menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable)
                                .fillMaxWidth(),
                        )
                        ExposedDropdownMenu(
                            expanded = areaExpanded,
                            onDismissRequest = { areaExpanded = false },
                        ) {
                            activeAreas.forEach { area ->
                                DropdownMenuItem(
                                    text = { Text(area.name) },
                                    onClick = {
                                        areaId = area.id
                                        areaExpanded = false
                                    },
                                )
                            }
                        }
                    }
                    OutlinedTextField(
                        value = address,
                        onValueChange = { address = it },
                        label = { Text(stringResource(R.string.visitor_pickup_address)) },
                        maxLines = 2,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    Row(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(stringResource(R.string.visitor_seats_label), style = MaterialTheme.typography.bodyMedium)
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            TextButton(onClick = { if (seats > 1) seats-- }, enabled = seats > 1) { Text("-") }
                            Text("$seats", style = MaterialTheme.typography.titleSmall)
                            TextButton(onClick = { if (seats < 6) seats++ }, enabled = seats < 6) { Text("+") }
                        }
                    }
                }
            },
            confirmButton = {
                val selectedArea = areaId
                Button(
                    onClick = {
                        if (selectedArea != null) {
                            onCreateVisitor(
                                CreateVisitorPassReq(
                                    name = name.trim(),
                                    phone = phone.trim(),
                                    pickup_at = "${date.trim()}T${time.trim()}",
                                    pickup_area_id = selectedArea,
                                    pickup_address = address.trim(),
                                    seats = seats,
                                ),
                            )
                            createOpen = false
                        }
                    },
                    enabled = name.isNotBlank() && address.isNotBlank() && selectedArea != null && !busy,
                    colors = graceRidePrimaryButtonColors(),
                ) {
                    Text(stringResource(R.string.visitor_create_btn))
                }
            },
            dismissButton = {
                TextButton(onClick = { createOpen = false }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }

    lastSharePair?.let { (vName, url) ->
        AlertDialog(
            onDismissRequest = onDismissSharePair,
            title = { Text(stringResource(R.string.visitor_link_ready_title, vName)) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    Text(
                        stringResource(R.string.visitor_link_ready_desc),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Text(
                        url,
                        style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                        color = MaterialTheme.colorScheme.primary,
                    )
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        shareText(context, "$vName · $eventTitle\n$url")
                    },
                    colors = graceRidePrimaryButtonColors(),
                ) {
                    Icon(Icons.Outlined.Share, contentDescription = null, modifier = Modifier.size(16.dp))
                    Spacer(Modifier.width(Spacing.xxs))
                    Text(stringResource(R.string.family_share_link))
                }
            },
            dismissButton = {
                Row {
                    TextButton(
                        onClick = {
                            if (copyToClipboard(context, "Visitor pass", url)) {
                                onCopiedToast()
                            }
                        },
                    ) {
                        Icon(Icons.Outlined.ContentCopy, contentDescription = null, modifier = Modifier.size(16.dp))
                        Spacer(Modifier.width(Spacing.xxs))
                        Text(stringResource(R.string.common_copy))
                    }
                    TextButton(onClick = onDismissSharePair) {
                        Text(stringResource(R.string.common_close))
                    }
                }
            },
        )
    }

    pendingRevoke?.let { pass ->
        AlertDialog(
            onDismissRequest = { pendingRevoke = null },
            title = { Text(stringResource(R.string.visitor_revoke)) },
            text = { Text(stringResource(R.string.visitor_revoke_confirm, pass.name)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        pendingRevoke = null
                        onRevokeVisitor(pass.id)
                    },
                ) {
                    Text(stringResource(R.string.visitor_revoke), color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { pendingRevoke = null }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }
}
