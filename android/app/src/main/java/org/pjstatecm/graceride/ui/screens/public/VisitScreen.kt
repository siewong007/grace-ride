package org.pjstatecm.graceride.ui.screens.public

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
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.CalendarMonth
import androidx.compose.material.icons.outlined.Call
import androidx.compose.material.icons.outlined.CheckCircle
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Map
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material.icons.outlined.Schedule
import androidx.compose.material.icons.outlined.WarningAmber
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
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
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.platform.addEventToCalendar
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.GraceRideLiveMap
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.LiveMapPoint
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.MapMarkerKind
import org.pjstatecm.graceride.ui.components.RideStatusPill
import org.pjstatecm.graceride.ui.components.TurnstileDialog
import org.pjstatecm.graceride.ui.dialPhone
import org.pjstatecm.graceride.ui.openInMaps
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun VisitScreen(
    container: AppContainer,
    token: String,
    onBack: () -> Unit,
) {
    org.pjstatecm.graceride.ui.SecureScreenEffect()
    val vm: VisitViewModel = viewModel(
        key = "visit_$token",
        factory = VisitViewModel.factory(container, token),
    )
    val state by vm.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val view = LocalView.current
    val snackbar = remember { SnackbarHostState() }
    val statusColors = LocalStatusColors.current
    var unableOpen by remember { mutableStateOf(false) }
    var unableNotes by remember { mutableStateOf("") }

    val toastMsg = state.readinessToast ?: state.readinessToastRes?.let { stringResource(it) }
    LaunchedEffect(toastMsg) {
        toastMsg?.let {
            snackbar.showSnackbar(it)
            vm.clearToast()
        }
    }
    LaunchedEffect(state.error) {
        state.error?.let {
            snackbar.showSnackbar(it)
            vm.clearError()
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.visit_title)) },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.common_back))
                    }
                },
                actions = {
                    if (state.ride != null) {
                        IconButton(onClick = { vm.refreshClaimed(silent = false) }) {
                            Icon(Icons.Outlined.Refresh, contentDescription = stringResource(R.string.common_retry))
                        }
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background),
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
        containerColor = MaterialTheme.colorScheme.background,
    ) { padding ->
        when {
            state.loading -> LoadingView(Modifier.fillMaxSize().padding(padding))
            state.loadError != null -> ErrorView(
                message = state.loadError ?: stringResource(R.string.visit_invalid),
                modifier = Modifier.padding(padding),
                onRetry = vm::load,
            )
            state.ride != null -> {
                val rideData = state.ride!!
                val detail = rideData.ride
                val loc = state.location
                val vehicleText = detail.driver?.vehicle?.let { v ->
                    listOfNotNull(
                        v.color?.takeIf { it.isNotBlank() },
                        v.model?.takeIf { it.isNotBlank() },
                        v.plate?.takeIf { it.isNotBlank() }?.let { "($it)" },
                    ).joinToString(" ")
                }.orEmpty()

                LazyColumn(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding)
                        .padding(horizontal = Spacing.sm),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    contentPadding = PaddingValues(vertical = Spacing.sm),
                    verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                ) {
                    item {
                        OutlinedCard(
                            modifier = Modifier.widthIn(max = 560.dp).fillMaxWidth(),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                        ) {
                            Column(
                                Modifier.padding(Spacing.sm),
                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                Row(
                                    Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Text(
                                        stringResource(R.string.visit_hello, rideData.name),
                                        style = MaterialTheme.typography.titleLarge,
                                        maxLines = 2,
                                        overflow = TextOverflow.Ellipsis,
                                        modifier = Modifier.weight(1f),
                                    )
                                    RideStatusPill(detail.status)
                                }
                                Text(
                                    stringResource(
                                        R.string.visit_ride_to,
                                        detail.destination.ifBlank { rideData.event.title },
                                    ),
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                                if (detail.arrivedAtGateAt != null && detail.pickedUpAt == null) {
                                    CalloutBanner(
                                        icon = Icons.Outlined.DirectionsCar,
                                        text = stringResource(R.string.visit_at_gate),
                                        containerColor = statusColors.successSoft,
                                        contentColor = statusColors.success,
                                    )
                                }
                                detail.delayMinutes?.takeIf { it > 0 }?.let { mins ->
                                    CalloutBanner(
                                        icon = Icons.Outlined.Schedule,
                                        text = stringResource(R.string.visit_driver_late, mins),
                                        containerColor = statusColors.warningSoft,
                                        contentColor = statusColors.warning,
                                    )
                                }
                            }
                        }
                    }

                    item {
                        OutlinedCard(
                            modifier = Modifier.widthIn(max = 560.dp).fillMaxWidth(),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                        ) {
                            Column(
                                Modifier.padding(Spacing.sm),
                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                InfoLine(
                                    label = stringResource(R.string.visit_time),
                                    value = KlTime.display(detail.pickupAt),
                                )
                                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                InfoLine(
                                    label = stringResource(R.string.visit_pickup),
                                    value = detail.pickupAddress.ifBlank { detail.area },
                                )
                                detail.driver?.let { driver ->
                                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                    InfoLine(
                                        label = stringResource(R.string.visit_driver),
                                        value = driver.name,
                                    )
                                    if (vehicleText.isNotBlank()) {
                                        InfoLine(
                                            label = stringResource(R.string.visit_car),
                                            value = vehicleText,
                                        )
                                    }
                                    if (!driver.phone.isNullOrBlank()) {
                                        FilledTonalButton(
                                            onClick = { dialPhone(context, driver.phone) },
                                            modifier = Modifier.fillMaxWidth(),
                                        ) {
                                            Icon(Icons.Outlined.Call, contentDescription = null, modifier = Modifier.size(18.dp))
                                            Spacer(Modifier.width(Spacing.xs))
                                            Text(stringResource(R.string.visit_call_driver))
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if (loc?.sharing == true && loc.lat != null && loc.lng != null) {
                        item {
                            OutlinedCard(
                                modifier = Modifier.widthIn(max = 560.dp).fillMaxWidth(),
                                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                            ) {
                                Column(
                                    Modifier.padding(Spacing.sm),
                                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                ) {
                                    Text(
                                        stringResource(R.string.visit_driver_here),
                                        style = MaterialTheme.typography.titleSmall,
                                    )
                                    val points = buildList {
                                        add(
                                            LiveMapPoint(
                                                id = "driver",
                                                lat = loc.lat,
                                                lng = loc.lng,
                                                label = detail.driver?.name.orEmpty(),
                                                kind = MapMarkerKind.CAR,
                                            ),
                                        )
                                        loc.passenger?.let { p ->
                                            add(
                                                LiveMapPoint(
                                                    id = "pickup",
                                                    lat = p.lat,
                                                    lng = p.lng,
                                                    label = detail.pickupAddress.ifBlank { rideData.name },
                                                    kind = MapMarkerKind.STOP,
                                                ),
                                            )
                                        }
                                    }
                                    GraceRideLiveMap(
                                        points = points,
                                        showZoomControls = true,
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .height(220.dp),
                                    )
                                    Row(
                                        Modifier.fillMaxWidth(),
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        loc.updatedAt?.let { updated ->
                                            Text(
                                                stringResource(R.string.visit_updated, KlTime.displayTime(updated)),
                                                style = MaterialTheme.typography.bodySmall,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                            )
                                        }
                                        TextButton(onClick = {
                                            openInMaps(context, "${loc.lat},${loc.lng}")
                                        }) {
                                            Icon(Icons.Outlined.Map, contentDescription = null, modifier = Modifier.size(16.dp))
                                            Spacer(Modifier.width(Spacing.xxs))
                                            Text(stringResource(R.string.visit_open_map))
                                        }
                                    }
                                }
                            }
                        }
                    }

                    item {
                        OutlinedCard(
                            modifier = Modifier.widthIn(max = 560.dp).fillMaxWidth(),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                        ) {
                            Column(
                                Modifier.padding(Spacing.sm),
                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                Text(
                                    stringResource(R.string.visit_let_driver_know),
                                    style = MaterialTheme.typography.titleSmall,
                                )
                                FlowRow(
                                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                ) {
                                    Button(
                                        onClick = {
                                            UiHaptics.confirm(view)
                                            vm.sendReadiness("ready")
                                        },
                                        enabled = !state.updatingReadiness,
                                        colors = graceRidePrimaryButtonColors(),
                                    ) {
                                        Icon(Icons.Outlined.CheckCircle, contentDescription = null, modifier = Modifier.size(18.dp))
                                        Spacer(Modifier.width(Spacing.xxs))
                                        Text(stringResource(R.string.visit_ready))
                                    }
                                    OutlinedButton(
                                        onClick = {
                                            UiHaptics.tick(view)
                                            vm.sendReadiness("delayed_5m")
                                        },
                                        enabled = !state.updatingReadiness,
                                    ) {
                                        Text(stringResource(R.string.visit_late))
                                    }
                                    OutlinedButton(
                                        onClick = {
                                            UiHaptics.tick(view)
                                            unableOpen = true
                                        },
                                        enabled = !state.updatingReadiness,
                                        colors = ButtonDefaults.outlinedButtonColors(
                                            contentColor = MaterialTheme.colorScheme.error,
                                        ),
                                    ) {
                                        Text(stringResource(R.string.visit_cant_make_it))
                                    }
                                }
                                if (rideData.host.phone.isNotBlank()) {
                                    TextButton(
                                        onClick = { dialPhone(context, rideData.host.phone) },
                                    ) {
                                        Icon(Icons.Outlined.Call, contentDescription = null, modifier = Modifier.size(16.dp))
                                        Spacer(Modifier.width(Spacing.xxs))
                                        Text(stringResource(R.string.visit_ask_host, rideData.host.name))
                                    }
                                }
                            }
                        }
                    }
                }
            }

            else -> {
                val preview = state.preview
                LazyColumn(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding)
                        .padding(horizontal = Spacing.sm),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    contentPadding = PaddingValues(vertical = Spacing.md),
                    verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                ) {
                    item {
                        OutlinedCard(
                            modifier = Modifier.widthIn(max = 560.dp).fillMaxWidth(),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                        ) {
                            Column(
                                Modifier.padding(Spacing.md),
                                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                            ) {
                                when (preview?.state) {
                                    "open" -> {
                                        Text(
                                            stringResource(R.string.visit_open_title, preview.host),
                                            style = MaterialTheme.typography.headlineSmall,
                                        )
                                        Text(
                                            "${preview.event.title} · ${KlTime.display(preview.event.startsAt)}",
                                            style = MaterialTheme.typography.titleSmall,
                                            color = MaterialTheme.colorScheme.primary,
                                        )
                                        if (preview.event.location.isNotBlank()) {
                                            Text(
                                                preview.event.location,
                                                style = MaterialTheme.typography.bodyMedium,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                            )
                                        }
                                        Text(
                                            stringResource(R.string.visit_open_one_phone),
                                            style = MaterialTheme.typography.bodyMedium,
                                        )
                                        Text(
                                            stringResource(R.string.visit_privacy_note),
                                            style = MaterialTheme.typography.bodySmall,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                        Button(
                                            onClick = { vm.claim() },
                                            enabled = !state.claiming,
                                            colors = graceRidePrimaryButtonColors(),
                                            modifier = Modifier.fillMaxWidth().height(52.dp),
                                        ) {
                                            if (state.claiming) {
                                                InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                            } else {
                                                Text(stringResource(R.string.visit_open_button))
                                            }
                                        }
                                    }

                                    "not_yet" -> {
                                        Text(
                                            stringResource(R.string.visit_not_yet_title, KlTime.display(preview.opensAt)),
                                            style = MaterialTheme.typography.titleLarge,
                                        )
                                        Text(
                                            "${preview.event.title} · ${KlTime.display(preview.event.startsAt)}",
                                            style = MaterialTheme.typography.bodyMedium,
                                        )
                                        Text(
                                            stringResource(R.string.visit_not_yet_body),
                                            style = MaterialTheme.typography.bodyMedium,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                        OutlinedButton(
                                            onClick = {
                                                addEventToCalendar(
                                                    context = context,
                                                    title = preview.event.title,
                                                    startsAt = preview.event.startsAt,
                                                    location = preview.event.location,
                                                    description = preview.host,
                                                )
                                            },
                                            modifier = Modifier.fillMaxWidth(),
                                        ) {
                                            Icon(Icons.Outlined.CalendarMonth, contentDescription = null, modifier = Modifier.size(18.dp))
                                            Spacer(Modifier.width(Spacing.xs))
                                            Text(stringResource(R.string.event_add_to_calendar))
                                        }
                                    }

                                    "used" -> {
                                        Text(
                                            stringResource(R.string.visit_used_title),
                                            style = MaterialTheme.typography.titleLarge,
                                        )
                                        Text(
                                            stringResource(R.string.visit_used_body, preview.host),
                                            style = MaterialTheme.typography.bodyMedium,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                    }

                                    else -> {
                                        val closedText = when (preview?.closedReason) {
                                            "completed" -> stringResource(R.string.visit_ended_thanks)
                                            "cancelled" -> stringResource(R.string.visit_ended_cancelled)
                                            else -> stringResource(
                                                R.string.visit_ended_stopped,
                                                preview?.host.orEmpty(),
                                            )
                                        }
                                        Text(
                                            closedText,
                                            style = MaterialTheme.typography.titleMedium,
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

    if (unableOpen) {
        AlertDialog(
            onDismissRequest = { unableOpen = false },
            title = { Text(stringResource(R.string.visit_cant_make_it)) },
            text = {
                OutlinedTextField(
                    value = unableNotes,
                    onValueChange = { unableNotes = it },
                    label = { Text(stringResource(R.string.login_recovery_reason_label)) },
                    maxLines = 3,
                    modifier = Modifier.fillMaxWidth(),
                )
            },
            confirmButton = {
                Button(
                    onClick = {
                        vm.sendReadiness("unable", unableNotes)
                        unableOpen = false
                        unableNotes = ""
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error),
                ) {
                    Text(stringResource(R.string.visit_cant_make_it))
                }
            },
            dismissButton = {
                TextButton(onClick = { unableOpen = false }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }

    if (state.turnstilePrompted && !state.turnstileSiteKey.isNullOrBlank()) {
        TurnstileDialog(
            siteKey = state.turnstileSiteKey!!,
            baseUrl = container.baseUrl,
            action = "visitor_claim",
            onToken = { tok -> vm.claim(tok) },
            onDismiss = vm::dismissTurnstile,
        )
    }
}

@Composable
private fun InfoLine(label: String, value: String) {
    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Text(
            label,
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(value, style = MaterialTheme.typography.bodyLarge)
    }
}
