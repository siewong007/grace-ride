package org.pjstatecm.graceride.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Event
import androidx.compose.material.icons.outlined.HowToReg
import androidx.compose.material.icons.outlined.QrCode2
import androidx.compose.material.icons.outlined.Repeat
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.theme.Spacing

enum class TimelineKind { SIGNUP, RIDE, RETURN, DRIVE, EVENT, CHECKIN }

enum class TimelineState { ACTIVE, UPCOMING, DONE }

data class TimelineBadge(
    val text: String,
    val kind: RideStatusKind,
)

data class TimelineNode(
    val id: String,
    val timeLabel: String,
    val title: String,
    val subtitle: String? = null,
    val kind: TimelineKind,
    val state: TimelineState,
    val badge: TimelineBadge? = null,
    val onClick: (() -> Unit)? = null,
)

data class DayOption(
    val key: String,
    val weekday: String,
    val dateLabel: String,
)

private fun iconFor(kind: TimelineKind) = when (kind) {
    TimelineKind.SIGNUP -> Icons.Outlined.HowToReg
    TimelineKind.CHECKIN -> Icons.Outlined.QrCode2
    TimelineKind.RETURN -> Icons.Outlined.Repeat
    TimelineKind.EVENT -> Icons.Outlined.Event
    TimelineKind.RIDE, TimelineKind.DRIVE -> Icons.Outlined.DirectionsCar
}

/** Vertical chronological rail uniting signup, rides, driving, and event milestones. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun ScheduleTimeline(
    items: List<TimelineNode>,
    modifier: Modifier = Modifier,
) {
    if (items.isEmpty()) return

    Column(modifier = modifier.fillMaxWidth()) {
        items.forEachIndexed { index, node ->
            val isLast = index == items.lastIndex
            val rowModifier = Modifier
                .fillMaxWidth()
                .height(IntrinsicSize.Min)
                .clip(MaterialTheme.shapes.small)
                .let { base ->
                    if (node.onClick != null) {
                        base.defaultMinSize(minHeight = 48.dp).clickable(onClick = node.onClick)
                    } else {
                        base
                    }
                }

            Row(modifier = rowModifier) {
                Text(
                    text = node.timeLabel,
                    style = MaterialTheme.typography.labelMedium,
                    fontWeight = FontWeight.Bold,
                    textAlign = TextAlign.End,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    color = if (node.state == TimelineState.ACTIVE) {
                        MaterialTheme.colorScheme.onSurface
                    } else {
                        MaterialTheme.colorScheme.onSurfaceVariant
                    },
                    modifier = Modifier
                        .width(64.dp)
                        .padding(top = 12.dp, end = Spacing.xs),
                )

                Column(
                    horizontalAlignment = Alignment.CenterHorizontally,
                    modifier = Modifier
                        .width(20.dp)
                        .fillMaxHeight(),
                ) {
                    Spacer(Modifier.height(14.dp))
                    val dotModifier = when (node.state) {
                        TimelineState.ACTIVE -> Modifier
                            .size(12.dp)
                            .background(MaterialTheme.colorScheme.primary, CircleShape)
                        TimelineState.DONE -> Modifier
                            .size(12.dp)
                            .background(MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f), CircleShape)
                        TimelineState.UPCOMING -> Modifier
                            .size(12.dp)
                            .background(MaterialTheme.colorScheme.surface, CircleShape)
                            .border(2.dp, MaterialTheme.colorScheme.outlineVariant, CircleShape)
                    }
                    Box(modifier = dotModifier)
                    if (!isLast) {
                        Box(
                            modifier = Modifier
                                .padding(top = 4.dp)
                                .width(2.dp)
                                .weight(1f)
                                .defaultMinSize(minHeight = 16.dp)
                                .background(MaterialTheme.colorScheme.outlineVariant),
                        )
                    }
                }

                Column(
                    modifier = Modifier
                        .weight(1f)
                        .padding(
                            start = Spacing.xs,
                            end = Spacing.xs,
                            top = 10.dp,
                            bottom = if (isLast) Spacing.xs else Spacing.sm,
                        ),
                    verticalArrangement = Arrangement.spacedBy(2.dp),
                ) {
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        verticalArrangement = Arrangement.Center,
                    ) {
                        Text(
                            text = node.title,
                            style = MaterialTheme.typography.titleSmall,
                            color = if (node.onClick != null && node.state == TimelineState.ACTIVE) {
                                MaterialTheme.colorScheme.primary
                            } else {
                                MaterialTheme.colorScheme.onSurface
                            },
                        )
                        node.badge?.let { badge ->
                            StatusPill(text = badge.text, kind = badge.kind)
                        }
                    }
                    if (!node.subtitle.isNullOrBlank()) {
                        Text(
                            text = node.subtitle,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            maxLines = 2,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }

                val iconBg = if (node.state == TimelineState.ACTIVE) {
                    MaterialTheme.colorScheme.primaryContainer
                } else {
                    MaterialTheme.colorScheme.surfaceVariant
                }
                val iconFg = if (node.state == TimelineState.ACTIVE) {
                    MaterialTheme.colorScheme.primary
                } else {
                    MaterialTheme.colorScheme.onSurfaceVariant
                }
                Surface(
                    color = iconBg,
                    contentColor = iconFg,
                    shape = CircleShape,
                    modifier = Modifier
                        .padding(top = 8.dp)
                        .size(34.dp),
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        Icon(
                            imageVector = iconFor(node.kind),
                            contentDescription = null,
                            modifier = Modifier.size(16.dp),
                        )
                    }
                }
            }
        }
    }
}

/** Horizontal day switcher for multi-day event schedules without pill bloat. */
@Composable
fun DayStrip(
    days: List<DayOption>,
    selectedDay: String,
    onSelect: (String) -> Unit,
    allLabel: String,
    modifier: Modifier = Modifier,
) {
    Column(modifier = modifier.fillMaxWidth()) {
        Row(
            horizontalArrangement = Arrangement.spacedBy(Spacing.md),
            modifier = Modifier
                .fillMaxWidth()
                .horizontalScroll(rememberScrollState()),
        ) {
            DayStripItem(
                weekday = allLabel,
                dateLabel = null,
                selected = selectedDay == "all",
                onClick = { onSelect("all") },
            )
            days.forEach { day ->
                DayStripItem(
                    weekday = day.weekday,
                    dateLabel = day.dateLabel,
                    selected = selectedDay == day.key,
                    onClick = { onSelect(day.key) },
                )
            }
        }
        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
    }
}

@Composable
private fun DayStripItem(
    weekday: String,
    dateLabel: String?,
    selected: Boolean,
    onClick: () -> Unit,
) {
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
        modifier = Modifier
            .defaultMinSize(minHeight = 48.dp)
            .clickable(onClick = onClick)
            .padding(horizontal = Spacing.xxs),
    ) {
        Text(
            text = weekday,
            style = MaterialTheme.typography.labelLarge,
            fontWeight = FontWeight.Bold,
            color = if (selected) MaterialTheme.colorScheme.onSurface else MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(top = Spacing.xs),
        )
        if (!dateLabel.isNullOrBlank()) {
            Text(
                text = dateLabel,
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Spacer(Modifier.height(6.dp))
        Box(
            modifier = Modifier
                .width(36.dp)
                .height(2.dp)
                .background(if (selected) MaterialTheme.colorScheme.primary else Color.Transparent),
        )
    }
}
