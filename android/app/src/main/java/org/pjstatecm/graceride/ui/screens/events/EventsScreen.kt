package org.pjstatecm.graceride.ui.screens.events

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.RideStatus
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.components.DayOption
import org.pjstatecm.graceride.ui.components.DayStrip
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.EventCard
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.OfflineBanner
import org.pjstatecm.graceride.ui.components.ScheduleTimeline
import org.pjstatecm.graceride.ui.components.TimelineBadge
import org.pjstatecm.graceride.ui.components.TimelineKind
import org.pjstatecm.graceride.ui.components.TimelineNode
import org.pjstatecm.graceride.ui.components.TimelineState
import org.pjstatecm.graceride.ui.theme.Spacing

private data class DatedNode(
    val sortAt: String,
    val dayKey: String,
    val node: TimelineNode,
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EventsScreen(container: AppContainer, nav: NavHostController) {
    val vm: EventsViewModel = viewModel(factory = EventsViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()

    Column(Modifier.fillMaxSize()) {
        PrimaryTabRow(selectedTabIndex = state.tab) {
            Tab(selected = state.tab == 0, onClick = { vm.setTab(0) }, text = { Text(stringResource(R.string.events_tab_upcoming)) })
            Tab(selected = state.tab == 1, onClick = { vm.setTab(1) }, text = { Text(stringResource(R.string.events_tab_my_schedule)) })
            Tab(selected = state.tab == 2, onClick = { vm.setTab(2) }, text = { Text(stringResource(R.string.events_tab_past)) })
        }
        when {
            state.loading -> LoadingView()
            state.error != null && state.events.isEmpty() -> ErrorView(state.error!!, vm::refresh)
            state.tab == 1 -> MyScheduleContent(state, vm::refresh, vm::selectDay, { vm.setTab(0) }, nav)
            else -> PullToRefreshBox(isRefreshing = state.refreshing, onRefresh = vm::refresh) {
                LazyColumn(
                    Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    contentPadding = PaddingValues(vertical = Spacing.xs),
                ) {
                    if (state.offline) item { OfflineBanner() }
                    if (state.events.isEmpty()) item { EmptyView(stringResource(R.string.events_empty)) }
                    items(state.events, key = { it.id }) { event ->
                        val needed = if (state.tab == 0) state.requestsByEvent[event.id] ?: 0 else 0
                        EventCard(event, onClick = { nav.navigate(Routes.eventDetail(event.id)) }, ridesNeeded = needed)
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun MyScheduleContent(
    state: EventsUiState,
    onRefresh: () -> Unit,
    onSelectDay: (String) -> Unit,
    onBrowseUpcoming: () -> Unit,
    nav: NavHostController,
) {
    val dayGroups = buildScheduleGroups(state, nav)
    val days = dayGroups.map { (dayKey, _) ->
        DayOption(
            key = dayKey,
            weekday = KlTime.weekdayShort(dayKey),
            dateLabel = KlTime.dayMonthShort(dayKey),
        )
    }
    val effectiveDay = if (state.selectedDay != "all" && days.any { it.key == state.selectedDay }) {
        state.selectedDay
    } else {
        "all"
    }
    val visibleGroups = if (effectiveDay == "all") dayGroups else dayGroups.filter { it.first == effectiveDay }

    PullToRefreshBox(isRefreshing = state.refreshing, onRefresh = onRefresh) {
        if (dayGroups.isEmpty()) {
            LazyColumn(
                Modifier.fillMaxSize().padding(Spacing.sm),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center,
            ) {
                if (state.offline) item { OfflineBanner() }
                item {
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                        modifier = Modifier.fillMaxWidth().padding(vertical = Spacing.lg),
                    ) {
                        EmptyView(stringResource(R.string.events_schedule_empty_title))
                        Text(
                            stringResource(R.string.events_schedule_empty_desc),
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            textAlign = TextAlign.Center,
                        )
                        Button(onClick = onBrowseUpcoming, modifier = Modifier.padding(top = Spacing.xs)) {
                            Text(stringResource(R.string.events_browse_upcoming))
                        }
                    }
                }
            }
        } else {
            LazyColumn(
                Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                contentPadding = PaddingValues(vertical = Spacing.xs),
            ) {
                if (state.offline) item { OfflineBanner() }
                if (days.size >= 2) {
                    item {
                        DayStrip(
                            days = days,
                            selectedDay = effectiveDay,
                            onSelect = onSelectDay,
                            allLabel = stringResource(R.string.events_all_days),
                        )
                    }
                }
                items(visibleGroups, key = { it.first }) { (dayKey, nodes) ->
                    Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        if (effectiveDay == "all") {
                            Text(
                                KlTime.displayDate(dayKey),
                                style = MaterialTheme.typography.labelLarge,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        OutlinedCard(Modifier.fillMaxWidth()) {
                            ScheduleTimeline(
                                items = nodes,
                                modifier = Modifier.padding(Spacing.sm),
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun buildScheduleGroups(
    state: EventsUiState,
    nav: NavHostController,
): List<Pair<String, List<TimelineNode>>> {
    val dated = mutableListOf<DatedNode>()
    val ridesByEvent = state.myRides.mapNotNull { ride -> ride.event?.id?.let { it to ride } }.toMap()
    val drivesByEvent = state.myDrives.mapNotNull { ride -> ride.event?.id?.let { it to ride } }.toMap()

    state.events
        .filter { it.mySignup?.status == "going" || it.mySignup?.status == "waitlist" }
        .forEach { event ->
            val going = event.mySignup?.status == "going"
            val hasTransport = event.myRide != null || event.myReturnRide != null ||
                ridesByEvent.containsKey(event.id) || drivesByEvent.containsKey(event.id)
            val subtitle = buildString {
                if (event.location.isNotBlank()) append(event.location)
                if (going && !hasTransport) {
                    if (isNotEmpty()) append(" · ")
                    append(stringResource(R.string.event_workflow_transport_pending))
                }
            }.ifBlank { null }
            val dayKey = KlTime.date(event.startsAt)
            if (dayKey.length == 10) {
                dated += DatedNode(
                    sortAt = event.startsAt,
                    dayKey = dayKey,
                    node = TimelineNode(
                        id = "event-${event.id}",
                        timeLabel = KlTime.time(event.startsAt),
                        title = event.title,
                        subtitle = subtitle,
                        kind = TimelineKind.EVENT,
                        state = if (going) TimelineState.ACTIVE else TimelineState.UPCOMING,
                        badge = TimelineBadge(
                            stringResource(if (going) R.string.event_going else R.string.event_waitlist),
                            if (going) RideStatusKind.ACTIVE else RideStatusKind.PENDING,
                        ),
                        onClick = { nav.navigate(Routes.eventDetail(event.id)) },
                    ),
                )
            }
        }

    val seenRides = mutableSetOf<Long>()
    state.myRides.forEach { ride ->
        if (!seenRides.add(ride.id)) return@forEach
        val isReturn = ride.leg == "return"
        val dayKey = KlTime.date(ride.pickupAt)
        if (dayKey.length == 10) {
            dated += DatedNode(
                sortAt = ride.pickupAt,
                dayKey = dayKey,
                node = TimelineNode(
                    id = "ride-${ride.id}",
                    timeLabel = KlTime.time(ride.pickupAt),
                    title = if (isReturn) stringResource(R.string.event_return_ride)
                    else stringResource(R.string.event_timeline_ride_to, ride.destination),
                    subtitle = listOfNotNull(ride.driver?.name, ride.area.name).joinToString(" · ").ifBlank { null },
                    kind = if (isReturn) TimelineKind.RETURN else TimelineKind.RIDE,
                    state = scheduleStateForRide(ride.status),
                    badge = TimelineBadge(stringResource(RideStatus.labelRes(ride.status)), RideStatus.kind(ride.status)),
                    onClick = { nav.navigate(Routes.rideDetail(ride.id)) },
                ),
            )
        }
    }

    state.myDrives.forEach { ride ->
        val riderName = if (ride.isProxy && !ride.proxy?.riderName.isNullOrBlank()) ride.proxy!!.riderName else ride.passenger.name
        val dayKey = KlTime.date(ride.pickupAt)
        if (dayKey.length == 10) {
            dated += DatedNode(
                sortAt = ride.pickupAt,
                dayKey = dayKey,
                node = TimelineNode(
                    id = "drive-${ride.id}",
                    timeLabel = KlTime.time(ride.pickupAt),
                    title = stringResource(R.string.event_timeline_drive_for, riderName),
                    subtitle = "${ride.area.name} → ${ride.destination}",
                    kind = TimelineKind.DRIVE,
                    state = scheduleStateForRide(ride.status),
                    badge = TimelineBadge(stringResource(RideStatus.labelRes(ride.status)), RideStatus.kind(ride.status)),
                    onClick = { nav.navigate(Routes.rideDetail(ride.id)) },
                ),
            )
        }
    }

    dated.sortBy { it.sortAt }
    val grouped = linkedMapOf<String, MutableList<TimelineNode>>()
    dated.forEach { item ->
        grouped.getOrPut(item.dayKey) { mutableListOf() }.add(item.node)
    }
    return grouped.entries.map { it.key to it.value }
}

private fun scheduleStateForRide(status: String): TimelineState = when (status) {
    "completed" -> TimelineState.DONE
    "confirmed", "in_progress" -> TimelineState.ACTIVE
    else -> TimelineState.UPCOMING
}
