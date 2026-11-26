package org.pjstatecm.graceride.ui.screens.events


import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.OpenInNew
import androidx.compose.material.icons.outlined.CalendarToday
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Map
import androidx.compose.material.icons.outlined.People
import androidx.compose.material.icons.outlined.Place
import androidx.compose.material.icons.outlined.QrCode2
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
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
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
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
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.MySignup
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.RideStatus
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.DetailSkeleton
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.EventIcon
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.ScheduleTimeline
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.components.TimelineBadge
import org.pjstatecm.graceride.ui.components.TimelineKind
import org.pjstatecm.graceride.ui.components.TimelineNode
import org.pjstatecm.graceride.ui.components.TimelineState
import org.pjstatecm.graceride.ui.copyToClipboard
import org.pjstatecm.graceride.ui.openInMaps
import org.pjstatecm.graceride.ui.openInWebApp
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.tabular

@Composable
internal fun InfoRow(label: String, value: String) {
    Column {
        Text(label, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(value, style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurface)
    }
}

@Composable
internal fun buildEventWorkflowNodes(
    event: EventItem,
    signup: MySignup?,
    outboundRide: Ride?,
    returnRide: Ride?,
    drivingForEvent: List<Ride>,
    onOpenRide: (Long) -> Unit,
): List<TimelineNode> {
    val nowKl = KlTime.nowStamp()
    val startDt = KlTime.parse(event.startsAt)
    val endedByTime = startDt != null && KlTime.now().isAfter(startDt.plusHours(12))
    val isLiveOrFuture = event.status == "published" && !endedByTime
    val deadlinePassed = !event.signupDeadline.isNullOrBlank() &&
        event.signupDeadline.take(16).replace('T', ' ') < nowKl
    val going = signup?.status == "going"
    val hasTransport = event.myRide != null ||
        event.myReturnRide != null ||
        drivingForEvent.isNotEmpty() ||
        signup?.selfTransport == true
    val eventTime = KlTime.time(event.startsAt)
    val nodes = mutableListOf<TimelineNode>()

    if (!isLiveOrFuture) {
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
            title = stringResource(
                when {
                    deadlinePassed -> R.string.event_signup_closed
                    full -> R.string.event_full
                    else -> R.string.event_workflow_signup_title
                },
            ),
            subtitle = if (deadlinePassed) {
                null
            } else {
                event.signupDeadline?.takeIf { it.isNotBlank() }?.let {
                    stringResource(R.string.event_workflow_signup_deadline, KlTime.display(it))
                } ?: stringResource(R.string.event_workflow_signup_hint)
            },
            kind = TimelineKind.SIGNUP,
            state = if (deadlinePassed) TimelineState.DONE else TimelineState.ACTIVE,
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
            eventIcon = event.icon,
            eventKind = event.kind,
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
            eventIcon = event.icon,
            eventKind = event.kind,
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
            eventIcon = event.icon,
            eventKind = event.kind,
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
internal fun appendTransportTimelineNodes(
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

    val outboundRef = event.myRide?.takeIf { it.id != event.myReturnRide?.id }
    outboundRef?.let { ref ->
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
    val isDraft = event.status == "draft"
    val isCancelled = event.status == "cancelled"
    nodes += TimelineNode(
        id = "evt-${event.id}",
        timeLabel = KlTime.time(event.startsAt),
        title = stringResource(R.string.event_workflow_event_start),
        subtitle = event.location.ifBlank { null },
        kind = TimelineKind.EVENT,
        state = when {
            isDraft -> TimelineState.UPCOMING
            doneAll -> TimelineState.DONE
            else -> TimelineState.ACTIVE
        },
        badge = when {
            isDraft -> TimelineBadge(stringResource(R.string.event_status_draft), RideStatusKind.PENDING)
            isCancelled -> TimelineBadge(stringResource(R.string.event_status_cancelled), RideStatusKind.CANCELLED)
            attended -> TimelineBadge(stringResource(R.string.event_status_attended), RideStatusKind.ACTIVE)
            doneAll -> TimelineBadge(stringResource(R.string.event_status_finished), RideStatusKind.DONE)
            else -> null
        },
        eventIcon = event.icon,
        eventKind = event.kind,
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
    if (retRef != null) {
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
