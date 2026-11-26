package org.pjstatecm.graceride.ui.screens.home

import androidx.compose.animation.Crossfade
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Call
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.History
import androidx.compose.material.icons.outlined.Navigation
import androidx.compose.material.icons.outlined.Notifications
import androidx.compose.material.icons.outlined.Payments
import androidx.compose.material.icons.outlined.Place
import androidx.compose.material.icons.outlined.PushPin
import androidx.compose.material.icons.outlined.Schedule
import androidx.compose.material.icons.outlined.TouchApp
import androidx.compose.material.icons.outlined.WarningAmber
import androidx.compose.material3.Badge
import androidx.compose.material3.BadgedBox
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.LocalBottomBarClearance
import org.pjstatecm.graceride.ui.components.AnimatedOfflineBanner
import org.pjstatecm.graceride.ui.components.BrandMarkMode
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.EventCard
import org.pjstatecm.graceride.ui.components.GraceRideBrandMark
import org.pjstatecm.graceride.ui.components.GraceRideWordmark
import org.pjstatecm.graceride.ui.components.HomeSkeleton
import org.pjstatecm.graceride.ui.components.MemberAvatar
import org.pjstatecm.graceride.ui.components.RideCard
import org.pjstatecm.graceride.ui.components.RideStatusPill
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.components.ShareCard
import org.pjstatecm.graceride.ui.components.friendlyTime
import org.pjstatecm.graceride.ui.dialPhone
import org.pjstatecm.graceride.ui.navigateTab
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.tabular

private data class QuickActionItem(
    val label: String,
    val icon: ImageVector,
    val badgeCount: Int = 0,
    val onClick: () -> Unit,
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(container: AppContainer, nav: NavHostController) {
    val vm: HomeViewModel = viewModel(factory = HomeViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val unreadCount = (authState as? AuthState.SignedIn)?.me?.unread ?: state.me?.unread ?: 0
    val lifecycleOwner = LocalLifecycleOwner.current
    var resumedOnce by remember { mutableStateOf(false) }
    var expandedAnnIds by remember { mutableStateOf(emptySet<Long>()) }

    DisposableEffect(lifecycleOwner) {
        val obs = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) {
                if (resumedOnce) vm.refresh() else resumedOnce = true
            }
        }
        lifecycleOwner.lifecycle.addObserver(obs)
        onDispose { lifecycleOwner.lifecycle.removeObserver(obs) }
    }

    val viewMode = when {
        state.loading -> "loading"
        state.error != null &&
            state.nextRide == null &&
            state.unpaidShares.isEmpty() &&
            state.events.isEmpty() &&
            state.announcements.isEmpty() &&
            state.drivingToday.isEmpty() &&
            state.nextUpcomingDrive == null -> "error"
        else -> "content"
    }

    Crossfade(targetState = viewMode, animationSpec = tween(200), label = "home_state") { mode ->
        when (mode) {
            "loading" -> HomeSkeleton()
            "error" -> ErrorView(state.error ?: "", onRetry = vm::refresh)
            else -> Column(Modifier.fillMaxSize()) {
                AnimatedOfflineBanner(
                    visible = state.offline,
                    modifier = Modifier.padding(horizontal = Spacing.sm, vertical = Spacing.xxs),
                )
                PullToRefreshBox(isRefreshing = state.refreshing, onRefresh = vm::refresh, modifier = Modifier.weight(1f)) {
                    val bottomClearance = LocalBottomBarClearance.current
                    LazyColumn(
                        Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
                        verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                        contentPadding = PaddingValues(top = Spacing.sm, bottom = Spacing.sm + bottomClearance),
                    ) {
                        item {
                            val me = state.me
                            Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Row(
                                        verticalAlignment = Alignment.CenterVertically,
                                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    ) {
                                        GraceRideBrandMark(
                                            modifier = Modifier.size(28.dp),
                                            mode = BrandMarkMode.IDLE_HOP,
                                        )
                                        GraceRideWordmark(fontSize = 20.sp)
                                    }
                                    Row(
                                        verticalAlignment = Alignment.CenterVertically,
                                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    ) {
                                        IconButton(onClick = { nav.navigate(AppRoute.Notifications) }) {
                                            BadgedBox(
                                                badge = {
                                                    if (unreadCount > 0) {
                                                        Badge {
                                                            Text(if (unreadCount > 99) "99+" else unreadCount.toString())
                                                        }
                                                    }
                                                },
                                            ) {
                                                Icon(
                                                    Icons.Outlined.Notifications,
                                                    contentDescription = stringResource(R.string.notif_title),
                                                )
                                            }
                                        }
                                        Box(Modifier.clickable { nav.navigateTab(AppRoute.Profile) }) {
                                            MemberAvatar(
                                                name = me?.name ?: "",
                                                photoUrl = me?.photoFileId?.let(container::fileUrl),
                                                avatarColor = me?.avatarColor,
                                                avatarIcon = me?.avatarIcon,
                                                size = 34.dp,
                                            )
                                        }
                                    }
                                }
                                Column {
                                    Text(
                                        stringResource(R.string.home_greeting, me?.name ?: ""),
                                        style = MaterialTheme.typography.headlineSmall,
                                        color = MaterialTheme.colorScheme.onSurface,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                    )
                                    Text(
                                        KlTime.displayDate(KlTime.today()),
                                        style = MaterialTheme.typography.bodyMedium,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                            }
                        }

                        val me = state.me
                        val canShowQuick = me?.can("rides") == true || me?.can("costs") == true || me?.can("drive") == true
                        if (canShowQuick) {
                            item {
                                val quickActions = buildList {
                                    if (me?.can("rides") == true) {
                                        add(
                                            QuickActionItem(
                                                label = stringResource(R.string.home_quick_rides),
                                                icon = Icons.Outlined.DirectionsCar,
                                                onClick = { nav.navigate(AppRoute.Rides) },
                                            ),
                                        )
                                    }
                                    if (me?.can("costs") == true) {
                                        add(
                                            QuickActionItem(
                                                label = stringResource(R.string.home_quick_costs),
                                                icon = Icons.Outlined.Payments,
                                                badgeCount = state.unpaidShareCount,
                                                onClick = { nav.navigate(AppRoute.Costs) },
                                            ),
                                        )
                                    }
                                    if (me?.can("drive") == true) {
                                        add(
                                            QuickActionItem(
                                                label = stringResource(R.string.shortcut_drive_short),
                                                icon = Icons.Outlined.Navigation,
                                                onClick = { nav.navigate(AppRoute.DriveHub) },
                                            ),
                                        )
                                    }
                                    if (me?.can("rides") == true && size < 4) {
                                        add(
                                            QuickActionItem(
                                                label = stringResource(R.string.history_title),
                                                icon = Icons.Outlined.History,
                                                onClick = { nav.navigate(AppRoute.History) },
                                            ),
                                        )
                                    }
                                }
                                Row(
                                    Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                ) {
                                    quickActions.forEach { action ->
                                        QuickActionTile(
                                            item = action,
                                            modifier = Modifier.weight(1f),
                                        )
                                    }
                                }
                            }
                        }

                        state.offlineSnapshotAt?.let { savedAt ->
                            val cachedTime = KlTime.formatEpochMillis(savedAt)
                            item {
                                CalloutBanner(
                                    icon = Icons.Outlined.Schedule,
                                    text = stringResource(R.string.drive_offline_snapshot, cachedTime),
                                    containerColor = MaterialTheme.colorScheme.surfaceVariant,
                                    contentColor = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }

                        state.nextRide?.let { ride ->
                            item { SectionHeader(stringResource(R.string.home_next_ride)) }
                            item {
                                NextRideHeroCard(
                                    ride = ride,
                                    container = container,
                                    onClick = { nav.navigate(AppRoute.RideDetail(ride.id)) },
                                )
                            }
                        }

                        if (state.unpaidShares.isNotEmpty()) {
                            item {
                                SectionHeader(
                                    title = stringResource(R.string.home_unpaid_shares),
                                    actionText = stringResource(R.string.home_view_all),
                                    count = state.unpaidShareCount,
                                ) { nav.navigate(AppRoute.Costs) }
                            }
                            items(state.unpaidShares, key = { "share-${it.id}" }) { share ->
                                ShareCard(
                                    share,
                                    canPay = state.me?.can("costs", "update") == true,
                                    onPay = { nav.navigate(AppRoute.Costs) },
                                    modifier = Modifier.animateItem(),
                                )
                            }
                        }

                        if (state.me?.can("drive") == true && (state.drivingToday.isNotEmpty() || state.nextUpcomingDrive != null)) {
                            item { SectionHeader(stringResource(R.string.home_today_driving)) }
                            if (state.drivingToday.isNotEmpty()) {
                                items(state.drivingToday, key = { "drive-${it.id}" }) { ride ->
                                    RideCard(
                                        ride,
                                        onClick = { nav.navigate(AppRoute.RideDetail(ride.id)) },
                                        photoUrlFor = container::fileUrl,
                                        modifier = Modifier.animateItem(),
                                    )
                                }
                            } else if (state.nextUpcomingDrive != null) {
                                val upcoming = state.nextUpcomingDrive!!
                                item {
                                    OutlinedCard(
                                        onClick = { nav.navigate(AppRoute.RideDetail(upcoming.id)) },
                                        modifier = Modifier.fillMaxWidth(),
                                    ) {
                                        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
                                            Row(
                                                Modifier.fillMaxWidth(),
                                                horizontalArrangement = Arrangement.SpaceBetween,
                                                verticalAlignment = Alignment.CenterVertically,
                                            ) {
                                                Text(
                                                    stringResource(R.string.home_next_drive_upcoming, KlTime.displayDate(upcoming.pickupAt)),
                                                    style = MaterialTheme.typography.titleSmall,
                                                    color = MaterialTheme.colorScheme.onSurface,
                                                    modifier = Modifier.weight(1f),
                                                )
                                                RideStatusPill(upcoming.status)
                                            }
                                            Text(
                                                "${KlTime.displayTime(upcoming.pickupAt)} · ${upcoming.passenger.name} · ${upcoming.area.name} → ${upcoming.destination}",
                                                style = MaterialTheme.typography.bodyMedium.tabular(),
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                            )
                                        }
                                    }
                                }
                            }
                            val remainingUpcoming = if (state.drivingToday.isNotEmpty()) state.upcomingDrives else state.upcomingDrives.drop(1)
                            if (remainingUpcoming.isNotEmpty()) {
                                items(remainingUpcoming.take(3), key = { "upcoming-drive-${it.id}" }) { ride ->
                                    OutlinedCard(
                                        onClick = { nav.navigate(AppRoute.RideDetail(ride.id)) },
                                        modifier = Modifier.fillMaxWidth().animateItem(),
                                    ) {
                                        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
                                            Row(
                                                Modifier.fillMaxWidth(),
                                                horizontalArrangement = Arrangement.SpaceBetween,
                                                verticalAlignment = Alignment.CenterVertically,
                                            ) {
                                                Text(
                                                    stringResource(R.string.home_drive_for, ride.passenger.name),
                                                    style = MaterialTheme.typography.titleSmall,
                                                    color = MaterialTheme.colorScheme.onSurface,
                                                    modifier = Modifier.weight(1f),
                                                )
                                                RideStatusPill(ride.status)
                                            }
                                            Text(
                                                "${KlTime.displayDate(ride.pickupAt)} · ${KlTime.displayTime(ride.pickupAt)} · ${ride.destination}",
                                                style = MaterialTheme.typography.bodyMedium.tabular(),
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                            )
                                        }
                                    }
                                }
                            }
                        }

                        if (state.events.isNotEmpty()) {
                            item {
                                SectionHeader(
                                    title = stringResource(R.string.home_upcoming_events),
                                    actionText = stringResource(R.string.home_view_all),
                                ) { nav.navigateTab(AppRoute.Events) }
                            }
                            items(state.events, key = { "event-${it.id}" }) { event ->
                                EventCard(
                                    event,
                                    onClick = { nav.navigate(AppRoute.EventDetail(event.id)) },
                                    modifier = Modifier.animateItem(),
                                )
                            }
                        }

                        if (state.announcements.isNotEmpty()) {
                            item { SectionHeader(stringResource(R.string.home_announcements)) }
                            items(state.announcements, key = { "ann-${it.id}" }) { ann ->
                                val expanded = ann.id in expandedAnnIds
                                val isLong = ann.body.length > 140 || ann.body.count { it == '\n' } >= 3
                                OutlinedCard(
                                    onClick = {
                                        if (isLong) {
                                            expandedAnnIds = if (expanded) expandedAnnIds - ann.id else expandedAnnIds + ann.id
                                        }
                                    },
                                    modifier = Modifier.fillMaxWidth().animateItem(),
                                ) {
                                    Column(
                                        Modifier.padding(Spacing.sm).animateContentSize(tween(200)),
                                        verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                                    ) {
                                        Row(
                                            verticalAlignment = Alignment.CenterVertically,
                                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                                        ) {
                                            if (ann.pinned) {
                                                Icon(
                                                    Icons.Outlined.PushPin,
                                                    contentDescription = null,
                                                    tint = MaterialTheme.colorScheme.primary,
                                                    modifier = Modifier.size(16.dp),
                                                )
                                            }
                                            Text(
                                                ann.title,
                                                style = MaterialTheme.typography.titleSmall,
                                                color = MaterialTheme.colorScheme.onSurface,
                                            )
                                        }
                                        if (ann.body.isNotBlank()) {
                                            Text(
                                                ann.body,
                                                style = MaterialTheme.typography.bodyMedium,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                maxLines = if (expanded) Int.MAX_VALUE else 3,
                                                overflow = TextOverflow.Ellipsis,
                                            )
                                        }
                                        if (isLong) {
                                            Text(
                                                stringResource(if (expanded) R.string.common_show_less else R.string.common_read_more),
                                                style = MaterialTheme.typography.labelMedium,
                                                color = MaterialTheme.colorScheme.primary,
                                                modifier = Modifier.padding(top = 2.dp),
                                            )
                                        }
                                    }
                                }
                            }
                        }

                        if (state.nextRide == null &&
                            state.unpaidShares.isEmpty() &&
                            state.drivingToday.isEmpty() &&
                            state.nextUpcomingDrive == null &&
                            state.events.isEmpty() &&
                            state.announcements.isEmpty()
                        ) {
                            item {
                                EmptyView(
                                    text = stringResource(R.string.home_no_ride),
                                    icon = Icons.Outlined.DirectionsCar,
                                    actionLabel = stringResource(R.string.ride_new_browse_events),
                                    onAction = { nav.navigateTab(AppRoute.Events) },
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun QuickActionTile(
    item: QuickActionItem,
    modifier: Modifier = Modifier,
) {
    Surface(
        onClick = item.onClick,
        color = MaterialTheme.colorScheme.surface,
        shape = MaterialTheme.shapes.medium,
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
        modifier = modifier.defaultMinSize(minHeight = 68.dp),
    ) {
        Column(
            Modifier.padding(horizontal = Spacing.xs, vertical = Spacing.xs),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(4.dp, Alignment.CenterVertically),
        ) {
            BadgedBox(
                badge = {
                    if (item.badgeCount > 0) {
                        Badge { Text(item.badgeCount.toString()) }
                    }
                },
            ) {
                Icon(
                    imageVector = item.icon,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.size(22.dp),
                )
            }
            Text(
                item.label,
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurface,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

@Composable
private fun NextRideHeroCard(
    ride: Ride,
    container: AppContainer,
    onClick: () -> Unit,
) {
    val context = LocalContext.current
    val c = LocalStatusColors.current
    val isLive = ride.status == "in_progress"
    OutlinedCard(
        onClick = onClick,
        border = BorderStroke(
            width = if (isLive) 1.5.dp else 1.dp,
            color = if (isLive) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outlineVariant,
        ),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.animateContentSize(tween(200))) {
            Row(
                Modifier.fillMaxWidth().padding(horizontal = Spacing.sm, vertical = Spacing.xs),
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
                        tint = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.size(18.dp),
                    )
                    Text(
                        friendlyTime(ride.pickupAt),
                        style = MaterialTheme.typography.titleMedium.tabular(),
                        color = MaterialTheme.colorScheme.onSurface,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                Row(
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xxs),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
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
                    RideStatusPill(ride.status)
                }
            }
            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)

            Column(
                Modifier.padding(Spacing.sm),
                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                // Visual route connector
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        modifier = Modifier.padding(top = 6.dp).width(16.dp),
                    ) {
                        Box(
                            Modifier
                                .size(10.dp)
                                .clip(CircleShape)
                                .background(MaterialTheme.colorScheme.primary),
                        )
                        Box(
                            Modifier
                                .padding(vertical = 3.dp)
                                .width(2.dp)
                                .height(20.dp)
                                .background(MaterialTheme.colorScheme.outlineVariant),
                        )
                        Icon(
                            Icons.Outlined.Place,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.size(16.dp),
                        )
                    }
                    Column(
                        Modifier.weight(1f),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        Text(
                            ride.area.name + (ride.pickupAddress?.let { " — $it" } ?: ""),
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.onSurface,
                            maxLines = 2,
                            overflow = TextOverflow.Ellipsis,
                        )
                        Text(
                            ride.destination,
                            style = MaterialTheme.typography.titleMedium,
                            color = MaterialTheme.colorScheme.onSurface,
                            maxLines = 2,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }

                if (ride.delayMinutes > 0) {
                    CalloutBanner(
                        icon = Icons.Outlined.Schedule,
                        text = stringResource(R.string.ride_driver_delay, ride.delayMinutes),
                        containerColor = c.warningSoft,
                        contentColor = c.warning,
                    )
                }
                ride.arrivedAtGateAt?.takeIf { it.isNotBlank() }?.let { arrived ->
                    CalloutBanner(
                        icon = Icons.Outlined.DirectionsCar,
                        text = stringResource(R.string.ride_arrived_gate, KlTime.time(arrived)),
                        containerColor = c.infoSoft,
                        contentColor = c.info,
                    )
                }
                if (ride.needsAttention) {
                    CalloutBanner(
                        icon = Icons.Outlined.WarningAmber,
                        text = stringResource(R.string.ride_attention) +
                            (if (ride.attentionReason.isNotBlank()) ": ${ride.attentionReason}" else ""),
                        containerColor = c.warningSoft,
                        contentColor = c.warning,
                    )
                }

                ride.driver?.let { driver ->
                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                    Row(
                        Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        MemberAvatar(
                            name = driver.name ?: "",
                            photoUrl = driver.photoFileId?.let(container::fileUrl),
                            avatarColor = driver.avatarColor,
                            avatarIcon = driver.avatarIcon,
                            size = 36.dp,
                        )
                        Column(Modifier.weight(1f)) {
                            Text(
                                driver.name?.takeIf { it.isNotBlank() } ?: "—",
                                style = MaterialTheme.typography.titleSmall,
                                color = MaterialTheme.colorScheme.onSurface,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                            val vehicleSummary = driver.vehicle?.let { v ->
                                listOfNotNull(
                                    v.color?.takeIf { it.isNotBlank() },
                                    v.model.takeIf { it.isNotBlank() },
                                    v.plate?.takeIf { it.isNotBlank() },
                                ).joinToString(" · ")
                            }.orEmpty()
                            if (vehicleSummary.isNotBlank()) {
                                Text(
                                    vehicleSummary,
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                )
                            }
                        }
                        driver.phone?.takeIf { it.isNotBlank() }?.let { phone ->
                            FilledTonalIconButton(onClick = { dialPhone(context, phone) }) {
                                Icon(
                                    Icons.Outlined.Call,
                                    contentDescription = stringResource(R.string.common_call),
                                )
                            }
                        }
                    }
                }

                val actionHintRes = when {
                    ride.viewer.isPassenger && ride.status == "pending_passenger" -> R.string.home_action_respond
                    ride.viewer.isPassenger && (ride.status == "confirmed" || ride.status == "in_progress") && ride.pickedUpAt == null ->
                        R.string.home_action_readiness
                    else -> null
                }
                if (actionHintRes != null) {
                    Surface(
                        color = MaterialTheme.colorScheme.surfaceContainerLow,
                        shape = MaterialTheme.shapes.small,
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Row(
                            Modifier.padding(horizontal = Spacing.xs, vertical = Spacing.xxs + 2.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                        ) {
                            Icon(
                                Icons.Outlined.TouchApp,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.primary,
                                modifier = Modifier.size(16.dp),
                            )
                            Text(
                                stringResource(actionHintRes),
                                style = MaterialTheme.typography.labelMedium,
                                color = MaterialTheme.colorScheme.primary,
                            )
                        }
                    }
                }
            }
        }
    }
}
