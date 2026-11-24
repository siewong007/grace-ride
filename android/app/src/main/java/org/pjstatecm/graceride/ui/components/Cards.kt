package org.pjstatecm.graceride.ui.components

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.Share
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.domain.RideStatus
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing

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
        Text(
            text,
            style = MaterialTheme.typography.labelMedium,
            modifier = Modifier.padding(horizontal = Spacing.xs + Spacing.xxs, vertical = Spacing.xxs),
        )
    }
}

@Composable
fun RideStatusPill(status: String, modifier: Modifier = Modifier) {
    StatusPill(stringResource(RideStatus.labelRes(status)), RideStatus.kind(status), modifier)
}

@Composable
fun SectionHeader(title: String, actionText: String? = null, onAction: (() -> Unit)? = null) {
    Row(
        Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(title, style = MaterialTheme.typography.titleMedium, color = MaterialTheme.colorScheme.onSurface)
        if (actionText != null && onAction != null) {
            TextButton(onClick = onAction) { Text(actionText) }
        }
    }
}

@Composable
fun RideCard(ride: Ride, onClick: () -> Unit, modifier: Modifier = Modifier) {
    OutlinedCard(onClick = onClick, modifier = modifier.fillMaxWidth()) {
        Column(Modifier.padding(Spacing.sm).animateContentSize(tween(200))) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Text(
                    friendlyTime(ride.pickupAt),
                    style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                RideStatusPill(ride.status)
            }
            Text(
                ride.area.name + " → " + ride.destination,
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurface,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.padding(top = Spacing.xxs),
            )
            val proxyLabel = if (ride.isProxy && !ride.proxy?.riderName.isNullOrBlank()) {
                stringResource(R.string.ride_proxy_label, ride.proxy!!.riderName)
            } else null
            val subtitle = buildString {
                proxyLabel?.let { append(it) }
                ride.driver?.name?.let { if (isNotEmpty()) append(" · "); append(it) }
                ride.event?.title?.let { if (isNotEmpty()) append(" · "); append(it) }
                if (ride.leg == "return") { if (isNotEmpty()) append(" · "); append(stringResource(R.string.ride_leg_return)) }
            }
            if (subtitle.isNotEmpty()) {
                Text(
                    subtitle,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.padding(top = Spacing.xxs),
                )
            }
            if (ride.needsAttention) {
                Text(
                    stringResource(R.string.ride_attention),
                    style = MaterialTheme.typography.labelMedium,
                    color = LocalStatusColors.current.warning,
                    modifier = Modifier.padding(top = Spacing.xxs),
                )
            }
        }
    }
}

@Composable
fun EventCard(
    event: EventItem,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    ridesNeeded: Int = 0,
) {
    OutlinedCard(onClick = onClick, modifier = modifier.fillMaxWidth()) {
        Column(Modifier.padding(Spacing.sm).animateContentSize(tween(200))) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Row(
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.weight(1f),
                ) {
                    EventIcon(icon = event.icon, kind = event.kind, size = 22.dp)
                    Text(
                        event.title,
                        style = MaterialTheme.typography.titleSmall,
                        color = MaterialTheme.colorScheme.onSurface,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.weight(1f, fill = false),
                    )
                }
                Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xxs), verticalAlignment = Alignment.CenterVertically) {
                    if (event.status == "draft") {
                        val c = LocalStatusColors.current
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
                                style = MaterialTheme.typography.labelMedium,
                                modifier = Modifier.padding(horizontal = Spacing.xs, vertical = Spacing.xxs),
                            )
                        }
                    }
                    event.mySignup?.takeIf { it.status == "going" || it.status == "waitlist" }?.let {
                        val c = LocalStatusColors.current
                        val (fg, bg) = if (it.status == "going") c.success to c.successSoft else c.warning to c.warningSoft
                        Surface(color = bg, contentColor = fg, shape = CircleShape) {
                            Text(
                                stringResource(if (it.status == "going") R.string.event_going else R.string.event_waitlist),
                                style = MaterialTheme.typography.labelMedium,
                                modifier = Modifier.padding(horizontal = Spacing.xs, vertical = Spacing.xxs),
                            )
                        }
                    }
                }
            }
            Text(
                friendlyTime(event.startsAt) + (if (event.location.isNotBlank()) " · " + event.location else ""),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.padding(top = Spacing.xxs),
            )
            event.spotsLeft?.let {
                Text(
                    stringResource(R.string.event_spots_left, it),
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(top = Spacing.xxs),
                )
            }
        }
    }
}

@Composable
fun ShareCard(share: Share, onPay: () -> Unit, modifier: Modifier = Modifier) {
    val c = LocalStatusColors.current
    OutlinedCard(modifier = modifier.fillMaxWidth()) {
        Column(Modifier.padding(Spacing.sm).animateContentSize(tween(200))) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Text(
                    Money.rm(share.amountCents),
                    style = MaterialTheme.typography.titleMedium,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                val (fg, bg, label) = when (share.status) {
                    "confirmed" -> Triple(c.success, c.successSoft, R.string.costs_status_confirmed)
                    "pending" -> Triple(c.info, c.infoSoft, R.string.costs_status_submitted)
                    else -> Triple(c.warning, c.warningSoft, R.string.costs_status_pending)
                }
                Surface(color = bg, contentColor = fg, shape = CircleShape) {
                    Text(
                        stringResource(label),
                        style = MaterialTheme.typography.labelMedium,
                        modifier = Modifier.padding(horizontal = Spacing.xs, vertical = Spacing.xxs),
                    )
                }
            }
            Text(
                share.title + (if (share.eventTitle.isNotBlank()) " · " + share.eventTitle else ""),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = Spacing.xxs),
            )
            share.dueDate?.takeIf { it.isNotBlank() }?.let {
                Text(
                    stringResource(R.string.costs_due, KlTime.date(it)),
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(top = Spacing.xxs),
                )
            }
            if (share.status == "unpaid") {
                TextButton(onClick = onPay, modifier = Modifier.padding(top = Spacing.xxs)) {
                    Text(stringResource(R.string.costs_pay))
                }
            }
        }
    }
}
