package org.pjstatecm.graceride.ui.screens.rides

import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.OpenInNew
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.Assistance
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.ProxyRelationships
import org.pjstatecm.graceride.domain.Readiness
import org.pjstatecm.graceride.domain.RideStatus
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.MemberAvatar
import org.pjstatecm.graceride.ui.components.RideStatusPill
import org.pjstatecm.graceride.ui.openInWebApp
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing

@OptIn(ExperimentalMaterial3Api::class, androidx.compose.foundation.layout.ExperimentalLayoutApi::class)
@Composable
fun RideDetailScreen(
    container: AppContainer,
    nav: NavHostController,
    id: Long,
    justBooked: Boolean = false,
) {
    val vm: RideDetailViewModel = viewModel(factory = RideDetailViewModel.factory(container, id))
    val state by vm.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val snackbar = remember { SnackbarHostState() }
    var confirmCancel by remember { mutableStateOf(false) }
    var confirmWithdraw by remember { mutableStateOf(false) }
    var withdrawReason by remember { mutableStateOf("") }
    val staleMsg = stringResource(R.string.error_ride_changed)
    val bookedMsg = stringResource(R.string.ride_new_success)
    val lifecycleOwner = LocalLifecycleOwner.current
    var resumedOnce by remember { mutableStateOf(false) }

    DisposableEffect(lifecycleOwner) {
        val obs = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) {
                if (resumedOnce) vm.load() else resumedOnce = true
            }
        }
        lifecycleOwner.lifecycle.addObserver(obs)
        onDispose { lifecycleOwner.lifecycle.removeObserver(obs) }
    }

    LaunchedEffect(justBooked) {
        if (justBooked) {
            snackbar.showSnackbar(bookedMsg, duration = SnackbarDuration.Long)
        }
    }

    LaunchedEffect(Unit) {
        container.realtimeStream.refreshEvents.collect { kind ->
            if (kind != "resume") vm.load()
        }
    }

    LaunchedEffect(state.message, state.staleReload) {
        if (state.staleReload) {
            snackbar.showSnackbar(staleMsg)
            vm.clearMessage()
        } else if (state.message != null) {
            snackbar.showSnackbar(state.message!!)
            vm.clearMessage()
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.ride_detail_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        val viewMode = when {
            state.loading -> "loading"
            state.ride == null -> "error"
            else -> "content"
        }
        Crossfade(
            targetState = viewMode,
            animationSpec = tween(200),
            label = "ride_detail_state",
            modifier = Modifier.padding(padding),
        ) { mode ->
            when (mode) {
                "loading" -> LoadingView()
                "error" -> ErrorView(state.error ?: "", vm::load)
                else -> {
                    val ride = state.ride!!
                    Column(
                        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(Spacing.sm),
                        verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                    ) {
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                        Text(KlTime.display(ride.pickupAt), style = MaterialTheme.typography.titleLarge, color = MaterialTheme.colorScheme.onSurface)
                        RideStatusPill(ride.status)
                    }
                    if (ride.leg == "return") {
                        Text(stringResource(R.string.ride_leg_return), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.primary)
                    }
                    if (ride.delayMinutes > 0) {
                        Text(
                            stringResource(R.string.ride_driver_delay, ride.delayMinutes),
                            style = MaterialTheme.typography.bodyMedium,
                            color = LocalStatusColors.current.warning,
                        )
                    }
                    ride.arrivedAtGateAt?.takeIf { it.isNotBlank() }?.let { arrived ->
                        Text(
                            stringResource(R.string.ride_arrived_gate, KlTime.time(arrived)),
                            style = MaterialTheme.typography.bodyMedium,
                            color = LocalStatusColors.current.info,
                        )
                    }
                    if (ride.needsAttention) {
                        Text(
                            stringResource(R.string.ride_attention) + (if (ride.attentionReason.isNotBlank()) ": " + ride.attentionReason else ""),
                            style = MaterialTheme.typography.bodyMedium,
                            color = LocalStatusColors.current.warning,
                        )
                    }

                    OutlinedCard(Modifier.fillMaxWidth()) {
                        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            InfoRow(stringResource(R.string.ride_pickup), ride.area.name + (ride.pickupAddress?.let { " — $it" } ?: ""))
                            InfoRow(stringResource(R.string.ride_destination), ride.destination)
                            InfoRow(stringResource(R.string.ride_seats), ride.seats.toString())
                            if (ride.assistance.isNotBlank()) {
                                InfoRow(
                                    stringResource(R.string.ride_assistance),
                                    Assistance.labelRes(ride.assistance)?.let { stringResource(it) } ?: ride.assistance,
                                )
                            }
                            if (ride.notes.isNotBlank()) InfoRow(stringResource(R.string.ride_notes), ride.notes)
                            ride.event?.title?.let { InfoRow(stringResource(R.string.ride_event), it) }
                            if (ride.readinessStatus in Readiness.values) {
                                InfoRow(
                                    stringResource(R.string.ride_readiness_status),
                                    stringResource(Readiness.labelRes(ride.readinessStatus)),
                                )
                            }
                            if (ride.isProxy && ride.proxy != null) {
                                val relLabel = ride.proxy.riderRelationship.takeIf { it.isNotBlank() }?.let { rel ->
                                    ProxyRelationships.labelRes(rel)?.let { res -> stringResource(res) } ?: rel
                                }
                                val proxySummary = listOfNotNull(
                                    ride.proxy.riderName.takeIf { it.isNotBlank() },
                                    relLabel,
                                ).joinToString(" · ")
                                if (proxySummary.isNotBlank()) {
                                    InfoRow(stringResource(R.string.ride_new_rider_name), proxySummary)
                                }
                                ride.proxy.riderPhone?.takeIf { it.isNotBlank() }?.let {
                                    InfoRow(stringResource(R.string.ride_new_rider_phone), it)
                                }
                                if (!ride.viewer.isPassenger && ride.proxy.bookerName.isNotBlank()) {
                                    InfoRow(
                                        stringResource(R.string.ride_proxy_booker, ride.proxy.bookerName),
                                        ride.proxy.bookerPhone?.takeIf { it.isNotBlank() } ?: "—",
                                    )
                                }
                            }
                            ride.driver?.let { d ->
                                Row(
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                ) {
                                    MemberAvatar(
                                        name = d.name ?: "",
                                        photoUrl = d.photoFileId?.let(container::fileUrl),
                                        avatarColor = d.avatarColor,
                                        avatarIcon = d.avatarIcon,
                                        size = 40.dp,
                                    )
                                    InfoRow(stringResource(R.string.ride_driver), d.name?.takeIf { it.isNotBlank() } ?: "—")
                                }
                                d.vehicle?.let { v ->
                                    val vSummary = listOfNotNull(
                                        v.model.takeIf { it.isNotBlank() },
                                        v.color?.takeIf { it.isNotBlank() },
                                        v.plate?.takeIf { it.isNotBlank() },
                                    ).joinToString(" · ")
                                    if (vSummary.isNotBlank()) {
                                        InfoRow(stringResource(R.string.ride_vehicle), vSummary)
                                    }
                                }
                                d.phone?.takeIf { it.isNotBlank() }?.let { InfoRow(stringResource(R.string.profile_phone), it) }
                            }
                            if (!ride.viewer.isPassenger) {
                                InfoRow(stringResource(R.string.ride_passenger), ride.passenger.name)
                                ride.passenger.phone?.takeIf { it.isNotBlank() }?.let { InfoRow(stringResource(R.string.profile_phone), it) }
                            }
                        }
                    }

                    ride.pairedRideId?.let { pairedId ->
                        OutlinedButton(
                            onClick = { nav.navigate(Routes.rideDetail(pairedId)) },
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Text(stringResource(R.string.ride_view_paired))
                        }
                    }

                    // Passenger actions, mirroring the web's state-gated controls.
                    if (ride.viewer.isPassenger && ride.status == "pending_passenger") {
                        Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            Button(onClick = { vm.respond(true) }, enabled = !state.busy) { Text(stringResource(R.string.ride_respond_accept)) }
                            OutlinedButton(onClick = { vm.respond(false) }, enabled = !state.busy) { Text(stringResource(R.string.ride_respond_decline)) }
                        }
                    }
                    if (ride.viewer.isPassenger &&
                        (ride.status == "confirmed" || ride.status == "in_progress") &&
                        ride.pickedUpAt == null
                    ) {
                        OutlinedCard(Modifier.fillMaxWidth()) {
                            Column(Modifier.padding(Spacing.sm)) {
                                Text(stringResource(R.string.ride_readiness), style = MaterialTheme.typography.titleSmall)
                                androidx.compose.foundation.layout.FlowRow(
                                    Modifier.padding(top = Spacing.xs),
                                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                ) {
                                    org.pjstatecm.graceride.domain.Readiness.values.forEach { value ->
                                        val selected = ride.readinessStatus == value
                                        if (selected) {
                                            Button(onClick = { vm.readiness(value) }, enabled = !state.busy) {
                                                Text(stringResource(org.pjstatecm.graceride.domain.Readiness.labelRes(value)))
                                            }
                                        } else {
                                            OutlinedButton(onClick = { vm.readiness(value) }, enabled = !state.busy) {
                                                Text(stringResource(org.pjstatecm.graceride.domain.Readiness.labelRes(value)))
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if (ride.viewer.isPassenger && RideStatus.cancellable(ride.status)) {
                        OutlinedButton(
                            onClick = { confirmCancel = true },
                            enabled = !state.busy,
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Text(stringResource(R.string.ride_cancel), color = MaterialTheme.colorScheme.error)
                        }
                    }

                    // Driver execution controls, matching the web's ride lifecycle actions.
                    if (ride.viewer.isDriver && ride.status in setOf(
                            "pending_driver_confirm",
                            "confirmed",
                            "in_progress",
                            "pending_passenger",
                            "pending_approval",
                        )
                    ) {
                        OutlinedCard(Modifier.fillMaxWidth()) {
                            Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                Text(stringResource(R.string.driver_actions_title), style = MaterialTheme.typography.titleSmall)
                                when (ride.status) {
                                    "pending_driver_confirm" -> {
                                        Button(
                                            onClick = { vm.confirmAssignment() },
                                            enabled = !state.busy,
                                            modifier = Modifier.fillMaxWidth(),
                                        ) { Text(stringResource(R.string.driver_confirm_assignment)) }
                                    }
                                    "confirmed" -> {
                                        Button(
                                            onClick = { vm.startRide() },
                                            enabled = !state.busy,
                                            modifier = Modifier.fillMaxWidth(),
                                        ) { Text(stringResource(R.string.driver_start_trip)) }
                                        FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                            OutlinedButton(onClick = { vm.driverDelay(5) }, enabled = !state.busy) {
                                                Text(stringResource(R.string.driver_delay_5m))
                                            }
                                            OutlinedButton(onClick = { vm.driverDelay(10) }, enabled = !state.busy) {
                                                Text(stringResource(R.string.driver_delay_10m))
                                            }
                                        }
                                    }
                                    "in_progress" -> {
                                        if (ride.pickedUpAt == null) {
                                            Button(
                                                onClick = { vm.pickup() },
                                                enabled = !state.busy,
                                                modifier = Modifier.fillMaxWidth(),
                                            ) { Text(stringResource(R.string.driver_picked_up)) }
                                            FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                                OutlinedButton(onClick = { vm.arrivedGate() }, enabled = !state.busy) {
                                                    Text(stringResource(R.string.driver_arrived_gate))
                                                }
                                                OutlinedButton(onClick = { vm.driverDelay(5) }, enabled = !state.busy) {
                                                    Text(stringResource(R.string.driver_delay_5m))
                                                }
                                                OutlinedButton(onClick = { vm.driverDelay(10) }, enabled = !state.busy) {
                                                    Text(stringResource(R.string.driver_delay_10m))
                                                }
                                            }
                                        } else {
                                            Button(
                                                onClick = { vm.complete() },
                                                enabled = !state.busy,
                                                modifier = Modifier.fillMaxWidth(),
                                            ) { Text(stringResource(R.string.driver_complete_trip)) }
                                        }
                                    }
                                }
                                if (ride.status in setOf("pending_passenger", "pending_approval", "pending_driver_confirm", "confirmed")) {
                                    OutlinedButton(
                                        onClick = { confirmWithdraw = true },
                                        enabled = !state.busy,
                                        modifier = Modifier.fillMaxWidth(),
                                    ) {
                                        Text(stringResource(R.string.driver_withdraw), color = MaterialTheme.colorScheme.error)
                                    }
                                }
                            }
                        }
                    }

                    if (ride.tripId != null || ride.status in setOf("confirmed", "in_progress")) {
                        OutlinedButton(
                            onClick = { openInWebApp(context, container.webUrl("rides/${ride.id}")) },
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Icon(Icons.AutoMirrored.Outlined.OpenInNew, contentDescription = null, modifier = Modifier.padding(end = Spacing.xs))
                            Text(stringResource(R.string.ride_open_web_live))
                        }
                    }
                    }
                }
            }
        }
    }

    if (confirmCancel) {
        AlertDialog(
            onDismissRequest = { confirmCancel = false },
            title = { Text(stringResource(R.string.ride_cancel)) },
            text = { Text(stringResource(R.string.ride_cancel_confirm)) },
            confirmButton = {
                TextButton(onClick = { vm.cancel(null); confirmCancel = false }) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = {
                TextButton(onClick = { confirmCancel = false }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }

    if (confirmWithdraw) {
        AlertDialog(
            onDismissRequest = { confirmWithdraw = false },
            title = { Text(stringResource(R.string.driver_withdraw)) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    Text(stringResource(R.string.driver_withdraw_confirm))
                    OutlinedTextField(
                        value = withdrawReason,
                        onValueChange = { withdrawReason = it },
                        label = { Text(stringResource(R.string.driver_withdraw_reason)) },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            },
            confirmButton = {
                TextButton(onClick = {
                    vm.withdraw(withdrawReason.trim().ifBlank { null })
                    withdrawReason = ""
                    confirmWithdraw = false
                }) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = {
                TextButton(onClick = { confirmWithdraw = false }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }
}

@Composable
private fun InfoRow(label: String, value: String) {
    Column {
        Text(label, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(value, style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurface)
    }
}
