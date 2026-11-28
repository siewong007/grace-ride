package org.pjstatecm.graceride.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Notifications
import androidx.compose.material3.Badge
import androidx.compose.material3.BadgedBox
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.PrimaryScrollableTabRow
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import org.pjstatecm.graceride.ui.theme.Spacing

/**
 * Centers screen content within a readable maximum width on tablet and unfolded foldable displays
 * while preserving full width on compact phones.
 */
@Composable
fun AdaptiveContentFrame(
    modifier: Modifier = Modifier,
    maxWidth: Dp = 840.dp,
    compactHorizontalPadding: Dp = 0.dp,
    expandedHorizontalPadding: Dp = Spacing.md,
    content: @Composable BoxScope.(isExpanded: Boolean) -> Unit,
) {
    BoxWithConstraints(modifier = modifier.fillMaxWidth(), contentAlignment = Alignment.TopCenter) {
        val isExpanded = this.maxWidth >= 600.dp
        val hPad = if (isExpanded) expandedHorizontalPadding else compactHorizontalPadding
        Box(
            modifier = Modifier
                .widthIn(max = maxWidth)
                .fillMaxWidth()
                .padding(horizontal = hPad),
        ) {
            content(isExpanded)
        }
    }
}

/**
 * Renders two composables side-by-side when available width is at least [twoColumnBreakpoint],
 * or stacks them vertically on compact phones. Keeping this inside a single `LazyColumn` item
 * preserves `hasScrollAction()` semantics across tests.
 */
@Composable
fun AdaptiveTwoColumnRow(
    modifier: Modifier = Modifier,
    twoColumnBreakpoint: Dp = 660.dp,
    horizontalSpacing: Dp = Spacing.sm,
    verticalSpacing: Dp = Spacing.sm,
    first: @Composable ColumnScope.() -> Unit,
    second: @Composable ColumnScope.() -> Unit,
) {
    BoxWithConstraints(modifier = modifier.fillMaxWidth()) {
        if (maxWidth >= twoColumnBreakpoint) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(horizontalSpacing),
                verticalAlignment = Alignment.Top,
            ) {
                Column(
                    modifier = Modifier.weight(1f),
                    verticalArrangement = Arrangement.spacedBy(verticalSpacing),
                    content = first,
                )
                Column(
                    modifier = Modifier.weight(1f),
                    verticalArrangement = Arrangement.spacedBy(verticalSpacing),
                    content = second,
                )
            }
        } else {
            Column(
                modifier = Modifier.fillMaxWidth(),
                verticalArrangement = Arrangement.spacedBy(verticalSpacing),
            ) {
                first()
                second()
            }
        }
    }
}

/**
 * Pairs items across two equal columns inside a single `LazyColumn` when wide enough.
 */
@Composable
fun AdaptivePairRow(
    left: @Composable BoxScope.() -> Unit,
    right: (@Composable BoxScope.() -> Unit)?,
    modifier: Modifier = Modifier,
    spacing: Dp = Spacing.xs,
) {
    Row(
        modifier = modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(spacing),
        verticalAlignment = Alignment.Top,
    ) {
        Box(modifier = Modifier.weight(1f), content = left)
        if (right != null) {
            Box(modifier = Modifier.weight(1f), content = right)
        } else {
            Spacer(modifier = Modifier.weight(1f))
        }
    }
}

/**
 * Switches between a scrollable tab row on compact phones and a fixed-width tab row on wider canvases
 * so multi-word tab labels do not truncate into ellipses on 411dp displays.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AdaptiveTabRow(
    selectedTabIndex: Int,
    modifier: Modifier = Modifier,
    scrollableBelow: Dp = 600.dp,
    edgePadding: Dp = Spacing.xs,
    tabs: @Composable () -> Unit,
) {
    BoxWithConstraints(modifier = modifier.fillMaxWidth()) {
        if (maxWidth < scrollableBelow) {
            PrimaryScrollableTabRow(
                selectedTabIndex = selectedTabIndex,
                edgePadding = edgePadding,
                tabs = tabs,
            )
        } else {
            PrimaryTabRow(
                selectedTabIndex = selectedTabIndex,
                tabs = tabs,
            )
        }
    }
}

/**
 * Notification bell action with extra trailing badge breathing room so 2+ digit unread counts
 * never clip against the right screen edge inside `TopAppBar`.
 */
@Composable
fun NotificationBellButton(
    unreadCount: Int,
    onClick: () -> Unit,
    contentDescription: String,
    modifier: Modifier = Modifier,
) {
    IconButton(
        onClick = onClick,
        modifier = modifier.padding(end = 4.dp),
    ) {
        BadgedBox(
            modifier = Modifier.padding(end = 6.dp, top = 2.dp),
            badge = {
                if (unreadCount > 0) {
                    Badge { Text(if (unreadCount > 99) "99+" else unreadCount.toString()) }
                }
            },
        ) {
            Icon(
                Icons.Outlined.Notifications,
                contentDescription = contentDescription,
            )
        }
    }
}
