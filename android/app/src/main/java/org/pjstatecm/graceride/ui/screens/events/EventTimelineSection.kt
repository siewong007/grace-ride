package org.pjstatecm.graceride.ui.screens.events

import androidx.compose.foundation.layout.Column
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.MySignup
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.RideStatus
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.components.TimelineBadge
import org.pjstatecm.graceride.ui.components.TimelineKind
import org.pjstatecm.graceride.ui.components.TimelineNode
import org.pjstatecm.graceride.ui.components.TimelineState

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
        return buildTransportTimelineNodes(
            event = event,
            outboundRide = outboundRide,
            returnRide = returnRide,
            drivingForEvent = drivingForEvent,
            doneAll = true,
            onOpenRide = onOpenRide,
        )
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

    nodes += buildTransportTimelineNodes(
        event = event,
        outboundRide = outboundRide,
        returnRide = returnRide,
        drivingForEvent = drivingForEvent,
        doneAll = false,
        onOpenRide = onOpenRide,
    )
    if (event.attendanceEnabled) {
        val attended = signup.attended == true
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

@Composable
internal fun buildTransportTimelineNodes(
    event: EventItem,
    outboundRide: Ride?,
    returnRide: Ride?,
    drivingForEvent: List<Ride>,
    doneAll: Boolean,
    onOpenRide: (Long) -> Unit,
): List<TimelineNode> {
    val nodes = mutableListOf<TimelineNode>()
    val eventKey = KlTime.sortKey(event.startsAt)
    val preDrives = drivingForEvent
        .filter { it.leg != "return" && KlTime.sortKey(it.pickupAt) <= eventKey }
        .sortedBy { KlTime.sortKey(it.pickupAt) }
    val postDrives = drivingForEvent
        .filter { it.leg == "return" || KlTime.sortKey(it.pickupAt) > eventKey }
        .sortedBy { KlTime.sortKey(it.pickupAt) }

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

    if (event.mySignup?.selfTransport == true && outboundRef == null && preDrives.isEmpty()) {
        nodes += TimelineNode(
            id = "wf-self-transport",
            timeLabel = stringResource(R.string.event_workflow_before),
            title = stringResource(R.string.event_going_own_way),
            kind = TimelineKind.RIDE,
            state = if (doneAll) TimelineState.DONE else TimelineState.ACTIVE,
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
    return nodes
}
