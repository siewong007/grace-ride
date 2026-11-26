package org.pjstatecm.graceride.ui.components

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.Accessible
import androidx.compose.material.icons.outlined.ChevronRight
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Event
import androidx.compose.material.icons.outlined.Person
import androidx.compose.material.icons.outlined.Place
import androidx.compose.material.icons.outlined.QrCode2
import androidx.compose.material.icons.outlined.Schedule
import androidx.compose.material.icons.outlined.WarningAmber
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.Share
import org.pjstatecm.graceride.domain.Assistance
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.domain.RideStatus
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.tabular

@Composable
fun friendlyTime(text: String): String {
    val day = when (KlTime.dayLabel(text)) {
        KlTime.DayLabel.TODAY -> stringResource(R.string.today)
        KlTime.DayLabel.TOMORROW -> stringResource(R.string.tomorrow)
        KlTime.DayLabel.OTHER -> null
    }
    return if (day != null) "$day · ${KlTime.time(text)}" else KlTime.display(text)
}

@Composable
fun StatusPill(text: String, kind: RideStatusKind, modifier: Modifier = Modifier) {
    val c = LocalStatusColors.current
    val (fg, bg) = when (kind) {
        RideStatusKind.ACTIVE -> c.success to c.successSoft
        RideStatusKind.PENDING -> c.warning to c.warningSoft
        RideStatusKind.DONE -> c.info to c.infoSoft
        RideStatusKind.CANCELLED -> c.danger to c.dangerSoft
    }
    val animBg by animateColorAsState(bg, tween(200), label = "pill_bg")
    val animFg by animateColorAsState(fg, tween(200), label = "pill_fg")
    Surface(color = animBg, contentColor = animFg, shape = CircleShape, modifier = modifier) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(6.dp),
            modifier = Modifier.padding(horizontal = Spacing.xs + Spacing.xxs, vertical = Spacing.xxs),
        ) {
            Box(
                Modifier
                    .size(6.dp)
                    .clip(CircleShape)
                    .background(animFg),
            )
            Text(
                text,
                style = MaterialTheme.typography.labelMedium,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

@Composable
fun RideStatusPill(status: String, modifier: Modifier = Modifier) {
    StatusPill(stringResource(RideStatus.labelRes(status)), RideStatus.kind(status), modifier)
}

@Composable
fun SectionHeader(
    title: String,
    actionText: String? = null,
    count: Int? = null,
    onAction: (() -> Unit)? = null,
) {
    Row(
        Modifier.fillMaxWidth().semantics { heading() },
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
            modifier = Modifier.weight(1f),
        ) {
            Text(title, style = MaterialTheme.typography.titleMedium, color = MaterialTheme.colorScheme.onSurface)
            if (count != null && count > 0) {
                Surface(
                    color = MaterialTheme.colorScheme.surfaceVariant,
                    contentColor = MaterialTheme.colorScheme.onSurfaceVariant,
                    shape = CircleShape,
                ) {
                    Text(
                        count.toString(),
                        style = MaterialTheme.typography.labelMedium.tabular(),
                        modifier = Modifier.padding(horizontal = Spacing.xs, vertical = 2.dp),
                    )
                }
            }
        }
        if (actionText != null && onAction != null) {
            TextButton(onClick = onAction) {
                Text(actionText)
                Icon(
                    Icons.Outlined.ChevronRight,
                    contentDescription = null,
                    modifier = Modifier.size(18.dp),
                )
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun RideCard(
    ride: Ride,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    photoUrlFor: (Long) -> String? = { null },
) {
    val isLive = ride.status == "in_progress"
    val border = if (isLive) {
        BorderStroke(1.5.dp, MaterialTheme.colorScheme.primary.copy(alpha = 0.55f))
    } else {
        BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)
    }
    OutlinedCard(
        onClick = onClick,
        border = border,
        modifier = modifier.fillMaxWidth(),
    ) {
        Column(
            Modifier.padding(Spacing.sm).animateContentSize(tween(200)),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    modifier = Modifier.weight(1f),
                ) {
                    Icon(
                        Icons.Outlined.Schedule,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.size(16.dp),
                    )
                    Text(
                        friendlyTime(ride.pickupAt),
                        style = MaterialTheme.typography.titleSmall.tabular(),
                        color = MaterialTheme.colorScheme.onSurface,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                RideStatusPill(ride.status)
            }
            Text(
                ride.area.name + " → " + ride.destination,
                style = MaterialTheme.typography.titleMedium,
                color = MaterialTheme.colorScheme.onSurface,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            val c = LocalStatusColors.current
            if (!ride.viewer.isPassenger) {
                val riderName = when {
                    ride.isProxy && !ride.proxy?.riderName.isNullOrBlank() && !ride.proxy?.bookerName.isNullOrBlank() ->
                        "${ride.proxy!!.riderName} (${ride.proxy.bookerName})"
                    ride.isProxy && !ride.proxy?.riderName.isNullOrBlank() ->
                        ride.proxy!!.riderName
                    else -> ride.passenger.name
                }
                val eventSubtitle = ride.event?.title?.takeIf { it.isNotBlank() }
                FlowRow(
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                ) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        Icon(
                            Icons.Outlined.Person,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.size(16.dp),
                        )
                        Text(
                            riderName,
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                    if (eventSubtitle != null) {
                        Text(
                            "· $eventSubtitle",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                    if (ride.seats > 1) {
                        Surface(
                            color = MaterialTheme.colorScheme.surfaceVariant,
                            contentColor = MaterialTheme.colorScheme.onSurfaceVariant,
                            shape = CircleShape,
                        ) {
                            Text(
                                "${ride.seats}×",
                                style = MaterialTheme.typography.labelMedium.tabular(),
                                modifier = Modifier.padding(horizontal = Spacing.xs, vertical = 2.dp),
                            )
                        }
                    }
                    if (ride.assistance.isNotBlank()) {
                        val assistText = Assistance.labelRes(ride.assistance)?.let { stringResource(it) } ?: ride.assistance
                        Surface(
                            color = c.infoSoft,
                            contentColor = c.info,
                            shape = CircleShape,
                        ) {
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(4.dp),
                                modifier = Modifier.padding(horizontal = Spacing.xs, vertical = 2.dp),
                            ) {
                                Icon(Icons.AutoMirrored.Outlined.Accessible, contentDescription = null, modifier = Modifier.size(14.dp))
                                Text(assistText, style = MaterialTheme.typography.labelMedium)
                            }
                        }
                    }
                    if (ride.leg == "return") {
                        Surface(
                            color = MaterialTheme.colorScheme.primaryContainer,
                            contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                            shape = CircleShape,
                        ) {
                            Text(
                                stringResource(R.string.ride_leg_return),
                                style = MaterialTheme.typography.labelMedium,
                                modifier = Modifier.padding(horizontal = Spacing.xs, vertical = 2.dp),
                            )
                        }
                    }
                }
            } else {
                val proxyLabel = if (ride.isProxy && !ride.proxy?.riderName.isNullOrBlank()) {
                    stringResource(R.string.ride_proxy_label, ride.proxy!!.riderName)
                } else {
                    null
                }
                val subtitle = buildString {
                    proxyLabel?.let { append(it) }
                    ride.event?.title?.let { if (isNotEmpty()) append(" · "); append(it) }
                }
                if (ride.driver != null || subtitle.isNotEmpty() || ride.leg == "return") {
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                    ) {
                        ride.driver?.let { d ->
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(6.dp),
                            ) {
                                MemberAvatar(
                                    name = d.name ?: "",
                                    photoUrl = d.photoFileId?.let(photoUrlFor),
                                    avatarColor = d.avatarColor,
                                    avatarIcon = d.avatarIcon,
                                    size = 20.dp,
                                )
                                Text(
                                    d.name?.takeIf { it.isNotBlank() } ?: "—",
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                )
                            }
                        }
                        if (subtitle.isNotEmpty()) {
                            Text(
                                subtitle,
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                        }
                        if (ride.leg == "return") {
                            Surface(
                                color = MaterialTheme.colorScheme.primaryContainer,
                                contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                                shape = CircleShape,
                            ) {
                                Text(
                                    stringResource(R.string.ride_leg_return),
                                    style = MaterialTheme.typography.labelMedium,
                                    modifier = Modifier.padding(horizontal = Spacing.xs, vertical = 2.dp),
                                )
                            }
                        }
                    }
                }
            }
            val showLiveNotices = !RideStatus.isTerminal(ride.status) && ride.pickedUpAt.isNullOrBlank()
            if (showLiveNotices && ride.delayMinutes > 0) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    Icon(Icons.Outlined.Schedule, contentDescription = null, tint = c.warning, modifier = Modifier.size(16.dp))
                    Text(
                        stringResource(R.string.ride_driver_delay, ride.delayMinutes),
                        style = MaterialTheme.typography.labelMedium,
                        color = c.warning,
                    )
                }
            }
            if (showLiveNotices) {
                ride.arrivedAtGateAt?.takeIf { it.isNotBlank() }?.let { arrived ->
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        Icon(Icons.Outlined.DirectionsCar, contentDescription = null, tint = c.info, modifier = Modifier.size(16.dp))
                        Text(
                            stringResource(R.string.ride_arrived_gate, KlTime.time(arrived)),
                            style = MaterialTheme.typography.labelMedium,
                            color = c.info,
                        )
                    }
                }
            }
            if (ride.needsAttention) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    Icon(Icons.Outlined.WarningAmber, contentDescription = null, tint = c.warning, modifier = Modifier.size(16.dp))
                    Text(
                        stringResource(R.string.ride_attention),
                        style = MaterialTheme.typography.labelMedium,
                        color = c.warning,
                    )
                }
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun EventCard(
    event: EventItem,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    ridesNeeded: Int = 0,
    quickRsvpLabel: String? = null,
    onQuickRsvp: (() -> Unit)? = null,
) {
    val c = LocalStatusColors.current
    OutlinedCard(onClick = onClick, modifier = modifier.fillMaxWidth()) {
        Column(
            Modifier.padding(Spacing.sm).animateContentSize(tween(200)),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(Spacing.sm),
                verticalAlignment = Alignment.Top,
            ) {
                Surface(
                    color = MaterialTheme.colorScheme.surfaceVariant,
                    shape = MaterialTheme.shapes.medium,
                    modifier = Modifier.size(44.dp),
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        EventIcon(icon = event.icon, kind = event.kind, size = 28.dp)
                    }
                }
                Column(
                    Modifier.weight(1f),
                    verticalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    Text(
                        event.title,
                        style = MaterialTheme.typography.titleMedium,
                        color = MaterialTheme.colorScheme.onSurface,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        Icon(
                            Icons.Outlined.Schedule,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.size(15.dp),
                        )
                        Text(
                            friendlyTime(event.startsAt),
                            style = MaterialTheme.typography.bodyMedium.tabular(),
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                    if (event.location.isNotBlank()) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(4.dp),
                        ) {
                            Icon(
                                Icons.Outlined.Place,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.size(15.dp),
                            )
                            Text(
                                event.location,
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                        }
                    }
                }
            }
            val signupBadge = event.mySignup?.takeIf { it.status == "going" || it.status == "waitlist" }
            if (event.status == "draft" || ridesNeeded > 0 || signupBadge != null || event.spotsLeft != null) {
                FlowRow(
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                ) {
                    signupBadge?.let {
                        StatusPill(
                            text = stringResource(if (it.status == "going") R.string.event_going else R.string.event_waitlist),
                            kind = if (it.status == "going") RideStatusKind.ACTIVE else RideStatusKind.PENDING,
                        )
                    }
                    if (event.status == "draft") {
                        Surface(color = c.warningSoft, contentColor = c.warning, shape = CircleShape) {
                            Text(
                                stringResource(R.string.event_status_draft),
                                style = MaterialTheme.typography.labelMedium,
                                modifier = Modifier.padding(horizontal = Spacing.xs, vertical = Spacing.xxs),
                            )
                        }
                    }
                    if (ridesNeeded > 0) {
                        Surface(
                            color = MaterialTheme.colorScheme.primaryContainer,
                            contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                            shape = CircleShape,
                        ) {
                            Text(
                                stringResource(R.string.event_rides_needed, ridesNeeded),
                                style = MaterialTheme.typography.labelMedium.tabular(),
                                modifier = Modifier.padding(horizontal = Spacing.xs, vertical = Spacing.xxs),
                            )
                        }
                    }
                    event.spotsLeft?.let { spots ->
                        val lowSpots = spots in 1L..5L
                        Surface(
                            color = if (lowSpots) c.warningSoft else MaterialTheme.colorScheme.surfaceVariant,
                            contentColor = if (lowSpots) c.warning else MaterialTheme.colorScheme.onSurfaceVariant,
                            shape = CircleShape,
                        ) {
                            Text(
                                stringResource(R.string.event_spots_left, spots),
                                style = MaterialTheme.typography.labelMedium.tabular(),
                                modifier = Modifier.padding(horizontal = Spacing.xs, vertical = Spacing.xxs),
                            )
                        }
                    }
                    if (quickRsvpLabel != null && onQuickRsvp != null) {
                        FilledTonalButton(
                            onClick = onQuickRsvp,
                            contentPadding = androidx.compose.foundation.layout.PaddingValues(
                                horizontal = Spacing.sm,
                                vertical = 2.dp,
                            ),
                        ) {
                            Text(quickRsvpLabel, style = MaterialTheme.typography.labelMedium)
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun ShareCard(
    share: Share,
    onPay: () -> Unit,
    modifier: Modifier = Modifier,
    canPay: Boolean = true,
) {
    val c = LocalStatusColors.current
    OutlinedCard(modifier = modifier.fillMaxWidth()) {
        Column(
            Modifier.padding(Spacing.sm).animateContentSize(tween(200)),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    Money.rm(share.amountCents),
                    style = MaterialTheme.typography.titleLarge.tabular(),
                    color = MaterialTheme.colorScheme.onSurface,
                )
                val (kind, label) = when (share.status) {
                    "confirmed" -> RideStatusKind.ACTIVE to R.string.costs_status_confirmed
                    "pending" -> RideStatusKind.DONE to R.string.costs_status_submitted
                    "needs_info" -> RideStatusKind.PENDING to R.string.share_needs_info
                    else -> RideStatusKind.PENDING to R.string.costs_status_pending
                }
                StatusPill(text = stringResource(label), kind = kind)
            }
            Text(
                share.title + (if (share.eventTitle.isNotBlank()) " · " + share.eventTitle else ""),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            share.dueDate?.takeIf { it.isNotBlank() }?.let {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    Icon(
                        Icons.Outlined.Event,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.size(15.dp),
                    )
                    Text(
                        stringResource(R.string.costs_due, KlTime.date(it)),
                        style = MaterialTheme.typography.labelMedium.tabular(),
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            share.note.takeIf { it.isNotBlank() }?.let { noteText ->
                CalloutBanner(
                    icon = Icons.Outlined.WarningAmber,
                    text = noteText,
                    containerColor = c.warningSoft,
                    contentColor = c.warning,
                )
            }
            if (canPay && (share.status == "unpaid" || share.status == "needs_info")) {
                FilledTonalButton(
                    onClick = onPay,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Icon(Icons.Outlined.QrCode2, contentDescription = null, modifier = Modifier.size(18.dp))
                    Spacer(Modifier.width(Spacing.xs))
                    Text(
                        stringResource(
                            if (share.status == "needs_info") R.string.costs_resubmit else R.string.costs_pay,
                        ),
                    )
                }
            }
        }
    }
}
