package org.pjstatecm.graceride.ui.screens.events

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.OpenInNew
import androidx.compose.material.icons.outlined.People
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MenuAnchorType
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.MySignup
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.RideStatus
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.ScheduleTimeline
import org.pjstatecm.graceride.ui.components.TimelineBadge
import org.pjstatecm.graceride.ui.components.TimelineKind
import org.pjstatecm.graceride.ui.components.TimelineNode
import org.pjstatecm.graceride.ui.components.TimelineState
import org.pjstatecm.graceride.ui.openInWebApp
import org.pjstatecm.graceride.ui.theme.Spacing

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EventDetailScreen(container: AppContainer, nav: NavHostController, id: Long) {
    val vm: EventDetailViewModel = viewModel(factory = EventDetailViewModel.factory(container, id))
    val state by vm.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val snackbar = remember { SnackbarHostState() }
    var showSignup by remember { mutableStateOf(false) }
    var confirmWithdraw by remember { mutableStateOf(false) }

    LaunchedEffect(state.message) {
        state.message?.let {
            snackbar.showSnackbar(it)
            vm.clearMessage()
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.events_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        when {
            state.loading -> LoadingView(Modifier.padding(padding))
            state.event == null -> ErrorView(state.error ?: "", vm::load, Modifier.padding(padding))
            else -> {
                val event = state.event!!
                // A withdrawn signup stays in the payload with status "cancelled";
                // treat it as no signup so the member can sign up again.
                val signup = event.mySignup?.takeIf { it.status == "going" || it.status == "waitlist" }
                val open = event.status == "published"
                val myRide = event.myRide
                val isGoing = signup?.status == "going"
                val hasTransport = myRide != null || state.drivingForEvent.isNotEmpty()
                val canCreateRide = container.auth.me?.can("rides", "create") == true
                val timelineNodes = buildEventWorkflowNodes(
                    event = event,
                    signup = signup,
                    outboundRide = state.outboundRide,
                    returnRide = state.returnRide,
                    drivingForEvent = state.drivingForEvent,
                    onOpenRide = { rideId -> nav.navigate(Routes.rideDetail(rideId)) },
                )

                Column(
                    Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState()).padding(Spacing.sm),
                    verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                ) {
                    Text(event.title, style = MaterialTheme.typography.headlineSmall, color = MaterialTheme.colorScheme.onSurface)
                    Text(KlTime.display(event.startsAt), style = MaterialTheme.typography.titleMedium, color = MaterialTheme.colorScheme.primary)

                    OutlinedCard(Modifier.fillMaxWidth()) {
                        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            Text(
                                stringResource(
                                    if (isGoing && state.drivingForEvent.isNotEmpty() && myRide == null) {
                                        R.string.event_you_are_driving
                                    } else {
                                        R.string.event_schedule_title
                                    },
                                ),
                                style = MaterialTheme.typography.labelLarge,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            ScheduleTimeline(items = timelineNodes)
                        }
                    }

                    if (open && signup == null) {
                        Button(onClick = { showSignup = true }, enabled = !state.busy, modifier = Modifier.fillMaxWidth()) {
                            Text(stringResource(R.string.event_signup))
                        }
                    } else if (open && signup != null) {
                        OutlinedButton(onClick = { confirmWithdraw = true }, enabled = !state.busy, modifier = Modifier.fillMaxWidth()) {
                            Text(stringResource(R.string.event_withdraw), color = MaterialTheme.colorScheme.error)
                        }
                    }

                    if (open && isGoing && canCreateRide) {
                        if (!hasTransport) {
                            Button(
                                onClick = { nav.navigate(Routes.rideNew(event.id)) },
                                enabled = !state.busy,
                                modifier = Modifier.fillMaxWidth(),
                            ) { Text(stringResource(R.string.ride_request)) }
                        } else {
                            OutlinedButton(
                                onClick = { nav.navigate(Routes.rideNew(event.id, proxy = true)) },
                                enabled = !state.busy,
                                modifier = Modifier.fillMaxWidth(),
                            ) {
                                Icon(Icons.Outlined.People, contentDescription = null, modifier = Modifier.padding(end = Spacing.xs))
                                Text(stringResource(R.string.event_book_for_family))
                            }
                        }
                    }

                    OutlinedCard(Modifier.fillMaxWidth()) {
                        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            if (event.location.isNotBlank()) InfoRow(stringResource(R.string.event_location), event.location)
                            if (event.capacity > 0) {
                                val spots = event.spotsLeft
                                InfoRow(
                                    stringResource(R.string.event_going),
                                    event.going.toString() + " / " + event.capacity +
                                        (spots?.let { " · " + stringResource(R.string.event_spots_left, it) } ?: ""),
                                )
                            }
                            if (event.costNote.isNotBlank()) InfoRow(stringResource(R.string.event_cost), event.costNote)
                            event.signupDeadline?.takeIf { it.isNotBlank() }?.let {
                                InfoRow(stringResource(R.string.event_deadline), KlTime.display(it))
                            }
                            signup?.let { mine ->
                                InfoRow(
                                    stringResource(R.string.event_my_status),
                                    stringResource(if (mine.status == "going") R.string.event_going else R.string.event_waitlist),
                                )
                                mine.passCode?.let { InfoRow(stringResource(R.string.event_checkin_code), it) }
                            }
                        }
                    }

                    if (event.description.isNotBlank()) {
                        Text(event.description, style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurface)
                    }

                    OutlinedButton(
                        onClick = { openInWebApp(context, container.webUrl("events/${event.id}")) },
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Icon(Icons.AutoMirrored.Outlined.OpenInNew, contentDescription = null, modifier = Modifier.padding(end = Spacing.xs))
                        Text(stringResource(R.string.event_open_web_full))
                    }
                }
            }
        }
    }

    val event = state.event
    if (showSignup && event != null) {
        var needRide by remember { mutableStateOf(false) }
        var offerDrive by remember { mutableStateOf(false) }
        var groupId by remember { mutableStateOf(event.joinAs.firstOrNull()?.id) }
        var groupExpanded by remember { mutableStateOf(false) }

        AlertDialog(
            onDismissRequest = { showSignup = false },
            title = { Text(stringResource(R.string.event_signup)) },
            text = {
                Column {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier
                            .fillMaxWidth()
                            .toggleable(
                                value = needRide,
                                onValueChange = { needRide = it },
                            ),
                    ) {
                        Checkbox(checked = needRide, onCheckedChange = null)
                        Text(stringResource(R.string.event_need_ride), modifier = Modifier.padding(start = Spacing.xs))
                    }
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier
                            .fillMaxWidth()
                            .toggleable(
                                value = offerDrive,
                                onValueChange = { offerDrive = it },
                            ),
                    ) {
                        Checkbox(checked = offerDrive, onCheckedChange = null)
                        Text(stringResource(R.string.event_offer_drive), modifier = Modifier.padding(start = Spacing.xs))
                    }
                    if (event.joinAs.size > 1) {
                        ExposedDropdownMenuBox(expanded = groupExpanded, onExpandedChange = { groupExpanded = it }) {
                            OutlinedTextField(
                                value = event.joinAs.firstOrNull { it.id == groupId }?.name ?: "",
                                onValueChange = {},
                                readOnly = true,
                                label = { Text(stringResource(R.string.event_join_as_group)) },
                                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = groupExpanded) },
                                modifier = Modifier.menuAnchor(MenuAnchorType.PrimaryNotEditable).fillMaxWidth().padding(top = Spacing.xs),
                            )
                            ExposedDropdownMenu(expanded = groupExpanded, onDismissRequest = { groupExpanded = false }) {
                                event.joinAs.forEach { g ->
                                    DropdownMenuItem(text = { Text(g.name) }, onClick = { groupId = g.id; groupExpanded = false })
                                }
                            }
                        }
                    }
                }
            },
            confirmButton = {
                TextButton(onClick = {
                    vm.signup(needRide, offerDrive, if (event.joinAs.size == 1) event.joinAs.first().id else groupId)
                    showSignup = false
                }) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = { TextButton(onClick = { showSignup = false }) { Text(stringResource(R.string.common_cancel)) } },
        )
    }

    if (confirmWithdraw) {
        AlertDialog(
            onDismissRequest = { confirmWithdraw = false },
            title = { Text(stringResource(R.string.event_withdraw)) },
            text = { Text(stringResource(R.string.event_withdraw_confirm)) },
            confirmButton = {
                TextButton(onClick = { vm.withdraw(); confirmWithdraw = false }) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = { TextButton(onClick = { confirmWithdraw = false }) { Text(stringResource(R.string.common_cancel)) } },
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

@Composable
private fun buildEventWorkflowNodes(
    event: EventItem,
    signup: MySignup?,
    outboundRide: Ride?,
    returnRide: Ride?,
    drivingForEvent: List<Ride>,
    onOpenRide: (Long) -> Unit,
): List<TimelineNode> {
    val open = event.status == "published"
    val going = signup?.status == "going"
    val hasTransport = event.myRide != null || drivingForEvent.isNotEmpty()
    val eventTime = KlTime.time(event.startsAt)
    val nodes = mutableListOf<TimelineNode>()

    if (!open) {
        appendTransportTimelineNodes(
            nodes = nodes,
            event = event,
            outboundRide = outboundRide,
            returnRide = returnRide,
            drivingForEvent = drivingForEvent,
            doneAll = true,
            onOpenRide = onOpenRide,
        )
        return nodes
    }

    if (signup == null) {
        val full = event.capacity > 0 && event.going >= event.capacity
        nodes += TimelineNode(
            id = "wf-signup",
            timeLabel = stringResource(R.string.event_workflow_rsvp),
            title = stringResource(if (full) R.string.event_full else R.string.event_workflow_signup_title),
            subtitle = event.signupDeadline?.takeIf { it.isNotBlank() }?.let {
                stringResource(R.string.event_workflow_signup_deadline, KlTime.display(it))
            } ?: stringResource(R.string.event_workflow_signup_hint),
            kind = TimelineKind.SIGNUP,
            state = TimelineState.ACTIVE,
        )
        nodes += TimelineNode(
            id = "wf-transport",
            timeLabel = stringResource(R.string.event_workflow_before),
            title = stringResource(R.string.event_workflow_transport_title),
            subtitle = stringResource(R.string.event_workflow_transport_unlocks),
            kind = TimelineKind.RIDE,
            state = TimelineState.UPCOMING,
        )
        nodes += TimelineNode(
            id = "evt-${event.id}",
            timeLabel = eventTime,
            title = stringResource(R.string.event_workflow_event_start),
            subtitle = event.location.ifBlank { null },
            kind = TimelineKind.EVENT,
            state = TimelineState.UPCOMING,
        )
        if (event.attendanceEnabled) {
            nodes += TimelineNode(
                id = "wf-checkin",
                timeLabel = stringResource(R.string.event_workflow_at_venue),
                title = stringResource(R.string.event_workflow_checkin_title),
                subtitle = stringResource(R.string.event_workflow_checkin_hint),
                kind = TimelineKind.CHECKIN,
                state = TimelineState.UPCOMING,
            )
        }
        return nodes
    }

    if (!going) {
        nodes += TimelineNode(
            id = "wf-waitlist",
            timeLabel = stringResource(R.string.event_workflow_rsvp),
            title = stringResource(R.string.event_waitlist),
            subtitle = stringResource(R.string.event_workflow_waitlist_note),
            kind = TimelineKind.SIGNUP,
            state = TimelineState.ACTIVE,
            badge = TimelineBadge(stringResource(R.string.event_waitlist), RideStatusKind.PENDING),
        )
        nodes += TimelineNode(
            id = "evt-${event.id}",
            timeLabel = eventTime,
            title = stringResource(R.string.event_workflow_event_start),
            subtitle = event.location.ifBlank { null },
            kind = TimelineKind.EVENT,
            state = TimelineState.UPCOMING,
        )
        return nodes
    }

    if (!hasTransport) {
        val attended = signup.attended == true
        nodes += TimelineNode(
            id = "wf-transport",
            timeLabel = stringResource(R.string.event_workflow_before),
            title = stringResource(R.string.event_workflow_transport_title),
            subtitle = stringResource(R.string.event_workflow_transport_pending),
            kind = TimelineKind.RIDE,
            state = TimelineState.ACTIVE,
        )
        nodes += TimelineNode(
            id = "evt-${event.id}",
            timeLabel = eventTime,
            title = stringResource(R.string.event_workflow_event_start),
            subtitle = event.location.ifBlank { null },
            kind = TimelineKind.EVENT,
            state = TimelineState.ACTIVE,
            badge = TimelineBadge(stringResource(R.string.event_going), RideStatusKind.ACTIVE),
        )
        if (event.attendanceEnabled) {
            nodes += TimelineNode(
                id = "wf-checkin",
                timeLabel = stringResource(R.string.event_workflow_at_venue),
                title = stringResource(R.string.event_workflow_checkin_title),
                subtitle = if (attended) null else stringResource(R.string.event_workflow_checkin_hint),
                kind = TimelineKind.CHECKIN,
                state = if (attended) TimelineState.DONE else TimelineState.UPCOMING,
                badge = if (attended) TimelineBadge(stringResource(R.string.event_status_attended), RideStatusKind.ACTIVE) else null,
            )
        }
        return nodes
    }

    appendTransportTimelineNodes(
        nodes = nodes,
        event = event,
        outboundRide = outboundRide,
        returnRide = returnRide,
        drivingForEvent = drivingForEvent,
        doneAll = false,
        onOpenRide = onOpenRide,
    )
    return nodes
}

@Composable
private fun appendTransportTimelineNodes(
    nodes: MutableList<TimelineNode>,
    event: EventItem,
    outboundRide: Ride?,
    returnRide: Ride?,
    drivingForEvent: List<Ride>,
    doneAll: Boolean,
    onOpenRide: (Long) -> Unit,
) {
    val preDrives = drivingForEvent
        .filter { it.leg != "return" && it.pickupAt <= event.startsAt }
        .sortedBy { it.pickupAt }
    val postDrives = drivingForEvent
        .filter { it.leg == "return" || it.pickupAt > event.startsAt }
        .sortedBy { it.pickupAt }

    event.myRide?.let { ref ->
        nodes += TimelineNode(
            id = "out-${ref.id}",
            timeLabel = outboundRide?.pickupAt?.let(KlTime::time) ?: stringResource(R.string.event_outbound_step),
            title = stringResource(R.string.event_my_ride),
            subtitle = outboundRide?.driver?.name,
            kind = TimelineKind.RIDE,
            state = if (doneAll || ref.status == "completed") TimelineState.DONE else TimelineState.ACTIVE,
            badge = TimelineBadge(stringResource(RideStatus.labelRes(ref.status)), RideStatus.kind(ref.status)),
            onClick = { onOpenRide(ref.id) },
        )
    }

    preDrives.forEach { ride ->
        val riderName = if (ride.isProxy && !ride.proxy?.riderName.isNullOrBlank()) ride.proxy!!.riderName else ride.passenger.name
        nodes += TimelineNode(
            id = "drv-${ride.id}",
            timeLabel = KlTime.time(ride.pickupAt),
            title = riderName,
            subtitle = "${ride.area.name} → ${ride.destination}",
            kind = TimelineKind.DRIVE,
            state = if (doneAll || ride.status == "completed") TimelineState.DONE else TimelineState.ACTIVE,
            badge = TimelineBadge(stringResource(RideStatus.labelRes(ride.status)), RideStatus.kind(ride.status)),
            onClick = { onOpenRide(ride.id) },
        )
    }

    val attended = event.attendanceEnabled && event.mySignup?.attended == true
    nodes += TimelineNode(
        id = "evt-${event.id}",
        timeLabel = KlTime.time(event.startsAt),
        title = stringResource(R.string.event_workflow_event_start),
        subtitle = event.location.ifBlank { null },
        kind = TimelineKind.EVENT,
        state = if (doneAll) TimelineState.DONE else TimelineState.ACTIVE,
        badge = when {
            doneAll -> TimelineBadge(
                stringResource(if (event.status == "cancelled") R.string.event_status_cancelled else R.string.event_status_finished),
                RideStatusKind.DONE,
            )
            attended -> TimelineBadge(stringResource(R.string.event_status_attended), RideStatusKind.ACTIVE)
            else -> null
        },
    )

    postDrives.forEach { ride ->
        val riderName = if (ride.isProxy && !ride.proxy?.riderName.isNullOrBlank()) ride.proxy!!.riderName else ride.passenger.name
        nodes += TimelineNode(
            id = "drv-${ride.id}",
            timeLabel = KlTime.time(ride.pickupAt),
            title = riderName,
            subtitle = "${ride.area.name} → ${ride.destination}",
            kind = TimelineKind.RETURN,
            state = if (doneAll || ride.status == "completed") TimelineState.DONE else TimelineState.UPCOMING,
            badge = TimelineBadge(stringResource(RideStatus.labelRes(ride.status)), RideStatus.kind(ride.status)),
            onClick = { onOpenRide(ride.id) },
        )
    }

    val retRef = event.myReturnRide
    if (retRef != null && retRef.id != event.myRide?.id) {
        nodes += TimelineNode(
            id = "ret-${retRef.id}",
            timeLabel = returnRide?.pickupAt?.let(KlTime::time) ?: stringResource(R.string.event_return_step),
            title = stringResource(R.string.event_return_ride),
            subtitle = returnRide?.driver?.name,
            kind = TimelineKind.RETURN,
            state = if (doneAll || retRef.status == "completed") TimelineState.DONE else TimelineState.UPCOMING,
            badge = TimelineBadge(stringResource(RideStatus.labelRes(retRef.status)), RideStatus.kind(retRef.status)),
            onClick = { onOpenRide(retRef.id) },
        )
    }
}
