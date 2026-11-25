package org.pjstatecm.graceride.ui.screens.home

import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Notifications
import androidx.compose.material3.Badge
import androidx.compose.material3.BadgedBox
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
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
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.navigateTab
import org.pjstatecm.graceride.ui.components.AnimatedOfflineBanner
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.EventCard
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.RideCard
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.components.ShareCard
import org.pjstatecm.graceride.ui.theme.Spacing

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(container: AppContainer, nav: NavHostController) {
    val vm: HomeViewModel = viewModel(factory = HomeViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val unreadCount = (authState as? AuthState.SignedIn)?.me?.unread ?: state.me?.unread ?: 0
    val lifecycleOwner = LocalLifecycleOwner.current
    var resumedOnce by remember { mutableStateOf(false) }

    DisposableEffect(lifecycleOwner) {
        val obs = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) {
                if (resumedOnce) vm.refresh() else resumedOnce = true
            }
        }
        lifecycleOwner.lifecycle.addObserver(obs)
        onDispose { lifecycleOwner.lifecycle.removeObserver(obs) }
    }

    LaunchedEffect(Unit) {
        container.realtimeStream.refreshEvents.collect { kind ->
            if (kind != "resume") vm.refresh()
        }
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
            "loading" -> LoadingView()
            "error" -> ErrorView(state.error ?: "", onRetry = vm::refresh)
            else -> Column(Modifier.fillMaxSize()) {
                AnimatedOfflineBanner(
                    visible = state.offline,
                    modifier = Modifier.padding(horizontal = Spacing.sm, vertical = Spacing.xxs),
                )
                PullToRefreshBox(isRefreshing = state.refreshing, onRefresh = vm::refresh, modifier = Modifier.weight(1f)) {
                    LazyColumn(
                        Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
                        verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                        contentPadding = androidx.compose.foundation.layout.PaddingValues(vertical = Spacing.sm),
                    ) {
                        item {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Text(
                                    stringResource(R.string.home_greeting, state.me?.name ?: ""),
                                    style = MaterialTheme.typography.headlineSmall,
                                    color = MaterialTheme.colorScheme.onSurface,
                                    modifier = Modifier.weight(1f),
                                )
                                IconButton(onClick = { nav.navigate(Routes.NOTIFICATIONS) }) {
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
                            }
                        }
                        state.offlineSnapshotAt?.let { savedAt ->
                            val cachedTime = KlTime.formatEpochMillis(savedAt)
                            item {
                                OutlinedCard(Modifier.fillMaxWidth()) {
                                    Text(
                                        stringResource(R.string.drive_offline_snapshot, cachedTime),
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        modifier = Modifier.padding(Spacing.sm),
                                    )
                                }
                            }
                        }

                        state.nextRide?.let { ride ->
                            item { SectionHeader(stringResource(R.string.home_next_ride)) }
                            item {
                                RideCard(ride, onClick = { nav.navigate(Routes.rideDetail(ride.id)) })
                            }
                        }

                        if (state.unpaidShares.isNotEmpty()) {
                            item {
                                SectionHeader(
                                    stringResource(R.string.home_unpaid_shares),
                                    stringResource(R.string.home_view_all),
                                ) { nav.navigate(Routes.COSTS) }
                            }
                            items(state.unpaidShares, key = { "share-${it.id}" }) { share ->
                                ShareCard(
                                    share,
                                    onPay = { nav.navigate(Routes.COSTS) },
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
                                        onClick = { nav.navigate(Routes.rideDetail(ride.id)) },
                                        modifier = Modifier.animateItem(),
                                    )
                                }
                            } else if (state.nextUpcomingDrive != null) {
                                val upcoming = state.nextUpcomingDrive!!
                                item {
                                    OutlinedCard(
                                        onClick = { nav.navigate(Routes.rideDetail(upcoming.id)) },
                                        modifier = Modifier.fillMaxWidth(),
                                    ) {
                                        Column(Modifier.padding(Spacing.sm)) {
                                            Text(
                                                stringResource(R.string.home_next_drive_upcoming, KlTime.displayDate(upcoming.pickupAt)),
                                                style = MaterialTheme.typography.titleSmall,
                                                color = MaterialTheme.colorScheme.onSurface,
                                            )
                                            Text(
                                                "${KlTime.displayTime(upcoming.pickupAt)} · ${upcoming.passenger.name} · ${upcoming.area.name} → ${upcoming.destination}",
                                                style = MaterialTheme.typography.bodySmall,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                modifier = Modifier.padding(top = Spacing.xxs),
                                            )
                                        }
                                    }
                                }
                            }
                            val remainingUpcoming = if (state.drivingToday.isNotEmpty()) state.upcomingDrives else state.upcomingDrives.drop(1)
                            if (remainingUpcoming.isNotEmpty()) {
                                items(remainingUpcoming.take(3), key = { "upcoming-drive-${it.id}" }) { ride ->
                                    OutlinedCard(
                                        onClick = { nav.navigate(Routes.rideDetail(ride.id)) },
                                        modifier = Modifier.fillMaxWidth().animateItem(),
                                    ) {
                                        Column(Modifier.padding(Spacing.sm)) {
                                            Text(
                                                stringResource(R.string.home_drive_for, ride.passenger.name),
                                                style = MaterialTheme.typography.titleSmall,
                                                color = MaterialTheme.colorScheme.onSurface,
                                            )
                                            Text(
                                                "${KlTime.displayDate(ride.pickupAt)} · ${KlTime.displayTime(ride.pickupAt)} · ${ride.destination}",
                                                style = MaterialTheme.typography.bodySmall,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                modifier = Modifier.padding(top = Spacing.xxs),
                                            )
                                        }
                                    }
                                }
                            }
                        }

                        if (state.events.isNotEmpty()) {
                            item {
                                SectionHeader(
                                    stringResource(R.string.home_upcoming_events),
                                    stringResource(R.string.home_view_all),
                                ) { nav.navigateTab(Routes.EVENTS) }
                            }
                            items(state.events, key = { "event-${it.id}" }) { event ->
                                EventCard(
                                    event,
                                    onClick = { nav.navigate(Routes.eventDetail(event.id)) },
                                    modifier = Modifier.animateItem(),
                                )
                            }
                        }

                        if (state.announcements.isNotEmpty()) {
                            item { SectionHeader(stringResource(R.string.home_announcements)) }
                            items(state.announcements, key = { "ann-${it.id}" }) { ann ->
                                OutlinedCard(Modifier.fillMaxWidth().animateItem()) {
                                    Column(Modifier.padding(Spacing.sm)) {
                                        Text(ann.title, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.onSurface)
                                        if (ann.body.isNotBlank()) {
                                            Text(
                                                ann.body,
                                                style = MaterialTheme.typography.bodyMedium,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                maxLines = 4,
                                                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                                                modifier = Modifier.padding(top = Spacing.xxs),
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
                            item { EmptyView(stringResource(R.string.home_no_ride)) }
                        }
                    }
                }
            }
        }
    }
}
