package org.pjstatecm.graceride.ui.screens.events

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.clickable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.EventAvailable
import androidx.compose.material.icons.outlined.Notifications
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Badge
import androidx.compose.material3.BadgedBox
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.PrimaryScrollableTabRow
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.RadioButton
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.rememberCoroutineScope
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.RideStatus
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.LocalBottomBarClearance
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.AnimatedOfflineBanner
import org.pjstatecm.graceride.ui.components.DayOption
import org.pjstatecm.graceride.ui.components.DayStrip
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.EventCard
import org.pjstatecm.graceride.ui.components.EventIcon
import org.pjstatecm.graceride.ui.components.GraceRideSpringEasing
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.ScheduleTimeline
import org.pjstatecm.graceride.ui.components.SkeletonLoadingList
import org.pjstatecm.graceride.ui.components.TimelineBadge
import org.pjstatecm.graceride.ui.components.TimelineKind
import org.pjstatecm.graceride.ui.components.TimelineNode
import org.pjstatecm.graceride.ui.components.TimelineState
import org.pjstatecm.graceride.ui.components.animatorsEnabled
import org.pjstatecm.graceride.ui.components.friendlyTime
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
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
    val snackbar = remember { SnackbarHostState() }
    val view = LocalView.current
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val currentMe = (authState as? org.pjstatecm.graceride.data.repo.AuthState.SignedIn)?.me ?: container.auth.me
    val canCreateEvent = currentMe?.can("events", "create") == true
    LaunchedEffect(canCreateEvent, state.tab) {
        if (!canCreateEvent && state.tab == 3) {
            vm.setTab(0)
        }
    }
    val publishedNotice = stringResource(R.string.event_published_notice)
    val unpublishedNotice = stringResource(R.string.event_unpublished_notice)
    var pendingPublish by remember { mutableStateOf<EventItem?>(null) }
    var pendingUnpublish by remember { mutableStateOf<EventItem?>(null) }
    var onlyGoing by remember { mutableStateOf(false) }
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

    LaunchedEffect(state.message) {
        state.message?.let {
            snackbar.showSnackbar(it)
            vm.clearMessage()
        }
    }
    LaunchedEffect(state.actionNotice) {
        when (state.actionNotice) {
            EventActionNotice.PUBLISHED -> snackbar.showSnackbar(publishedNotice)
            EventActionNotice.UNPUBLISHED -> snackbar.showSnackbar(unpublishedNotice)
            null -> Unit
        }
        if (state.actionNotice != null) vm.clearNotice()
    }

    Box(Modifier.fillMaxSize()) {
        Column(Modifier.fillMaxSize()) {
            val unreadCount = currentMe?.unread ?: 0
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(start = Spacing.sm, end = Spacing.xxs, top = Spacing.xs, bottom = Spacing.xxs),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    stringResource(R.string.events_title),
                    style = MaterialTheme.typography.headlineSmall,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                val unreadState = if (unreadCount > 0) {
                    stringResource(R.string.notif_a11y_unread_count, unreadCount)
                } else {
                    null
                }
                Row(verticalAlignment = Alignment.CenterVertically) {
                    if (canCreateEvent) {
                        FilledTonalButton(
                            onClick = { nav.navigate(AppRoute.EventNew()) },
                            contentPadding = PaddingValues(horizontal = Spacing.sm, vertical = 4.dp),
                        ) {
                            Icon(Icons.Outlined.Add, contentDescription = null, modifier = Modifier.size(18.dp))
                            Spacer(Modifier.width(4.dp))
                            Text(stringResource(R.string.event_create_btn), style = MaterialTheme.typography.labelLarge)
                        }
                    }
                    IconButton(
                        onClick = { nav.navigate(AppRoute.Notifications) },
                        modifier = Modifier.semantics {
                            if (unreadState != null) stateDescription = unreadState
                        },
                    ) {
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
            val tabItems: @Composable () -> Unit = {
                Tab(
                    selected = state.tab == 0,
                    onClick = { UiHaptics.tick(view); vm.setTab(0) },
                    text = { Text(stringResource(R.string.events_tab_upcoming), maxLines = 1, overflow = TextOverflow.Ellipsis) },
                )
                Tab(
                    selected = state.tab == 1,
                    onClick = { UiHaptics.tick(view); vm.setTab(1) },
                    text = { Text(stringResource(R.string.events_tab_my_schedule), maxLines = 1, overflow = TextOverflow.Ellipsis) },
                )
                Tab(
                    selected = state.tab == 2,
                    onClick = { UiHaptics.tick(view); vm.setTab(2) },
                    text = { Text(stringResource(R.string.events_tab_past), maxLines = 1, overflow = TextOverflow.Ellipsis) },
                )
                if (canCreateEvent) {
                    Tab(
                        selected = state.tab == 3,
                        onClick = { UiHaptics.tick(view); vm.setTab(3) },
                        text = { Text(stringResource(R.string.events_tab_managing), maxLines = 1, overflow = TextOverflow.Ellipsis) },
                    )
                }
            }
            if (canCreateEvent) {
                PrimaryScrollableTabRow(selectedTabIndex = state.tab.coerceIn(0, 3), edgePadding = Spacing.xs) {
                    tabItems()
                }
            } else {
                PrimaryTabRow(selectedTabIndex = state.tab.coerceIn(0, 2)) {
                    tabItems()
                }
            }

            AnimatedOfflineBanner(
                visible = !state.loading && state.offline,
                modifier = Modifier.padding(horizontal = Spacing.sm, vertical = Spacing.xxs),
            )

            val tabSnapshots = remember { mutableStateMapOf<Int, EventsUiState>() }
            if (!state.loading) {
                tabSnapshots[state.tab] = state
            }

            AnimatedContent(
                targetState = state.tab,
                transitionSpec = {
                    if (animatorsEnabled()) {
                        val dir = if (targetState > initialState) 1 else -1
                        (slideInHorizontally(tween(240, easing = GraceRideSpringEasing)) { it * dir } +
                            fadeIn(tween(180)))
                            .togetherWith(
                                slideOutHorizontally(tween(220, easing = GraceRideSpringEasing)) { (-it / 3) * dir } +
                                    fadeOut(tween(140)),
                            )
                    } else {
                        fadeIn(tween(120)).togetherWith(fadeOut(tween(100)))
                    }
                },
                label = "events_tab",
                modifier = Modifier.weight(1f),
            ) { tab ->
                val tabState = if (tab == state.tab) state else (tabSnapshots[tab] ?: state)
                val innerMode = when {
                    tabState.loading -> "loading"
                    tabState.error != null && tabState.events.isEmpty() -> "error"
                    else -> "content"
                }
                Crossfade(
                    targetState = innerMode,
                    animationSpec = tween(180),
                    label = "events_tab_state_$tab",
                ) { mode ->
                    when (mode) {
                        "loading" -> SkeletonLoadingList(count = 4)
                        "error" -> ErrorView(tabState.error ?: "", onRetry = vm::refresh)
                        else -> when (tab) {
                            1 -> MyScheduleContent(tabState, vm::refresh, { UiHaptics.tick(view); vm.selectDay(it) }, { vm.setTab(0) }, nav)
                            3 -> ManagingContent(
                                state = tabState,
                                onRefresh = vm::refresh,
                                onToggleHideDrafts = vm::toggleHideDrafts,
                                onOpenEvent = { nav.navigate(AppRoute.EventDetail(it)) },
                                onPublishClick = { pendingPublish = it },
                                onUnpublishClick = { pendingUnpublish = it },
                            )
                            else -> {
                                val isUpcomingTab = tab == 0
                                val hasSignedUp = isUpcomingTab && tabState.events.any {
                                    it.mySignup?.status == "going" || it.mySignup?.status == "waitlist"
                                }
                                val visibleEvents = if (isUpcomingTab && onlyGoing && hasSignedUp) {
                                    tabState.events.filter { it.mySignup?.status == "going" || it.mySignup?.status == "waitlist" }
                                } else {
                                    tabState.events
                                }
                                PullToRefreshBox(isRefreshing = tabState.refreshing, onRefresh = vm::refresh) {
                                    val bottomClearance = LocalBottomBarClearance.current
                                    LazyColumn(
                                        Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
                                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                        contentPadding = PaddingValues(top = Spacing.xs, bottom = Spacing.xs + bottomClearance),
                                    ) {
                                        if (hasSignedUp) {
                                            item(key = "filter-going") {
                                                FilterChip(
                                                    selected = onlyGoing,
                                                    onClick = { UiHaptics.tick(view); onlyGoing = !onlyGoing },
                                                    label = { Text(stringResource(R.string.events_filter_going)) },
                                                )
                                            }
                                        }
                                        if (visibleEvents.isEmpty()) item { EmptyView(stringResource(R.string.events_empty)) }
                                        items(visibleEvents, key = { it.id }) { event ->
                                            val needed = if (isUpcomingTab) tabState.requestsByEvent[event.id] ?: 0 else 0
                                            val unsigned = event.mySignup == null || event.mySignup.status == "cancelled"
                                            val canQuickRsvp = isUpcomingTab &&
                                                unsigned &&
                                                event.status == "published" &&
                                                (event.signupDeadline.isNullOrBlank() ||
                                                    event.signupDeadline.take(16).replace('T', ' ') >= KlTime.nowStamp())
                                            EventCard(
                                                event,
                                                onClick = { nav.navigate(AppRoute.EventDetail(event.id)) },
                                                modifier = Modifier.animateItem(),
                                                ridesNeeded = needed,
                                                quickRsvpLabel = if (canQuickRsvp) stringResource(R.string.quick_rsvp_btn) else null,
                                                onQuickRsvp = if (canQuickRsvp) ({ vm.openQuickRsvp(event) }) else null,
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        SnackbarHost(
            hostState = snackbar,
            modifier = Modifier.align(Alignment.BottomCenter).padding(bottom = LocalBottomBarClearance.current),
        )
    }

    pendingPublish?.let { target ->
        AlertDialog(
            onDismissRequest = { pendingPublish = null },
            title = { Text(stringResource(R.string.event_publish)) },
            text = { Text(stringResource(R.string.event_publish_confirm)) },
            confirmButton = {
                TextButton(onClick = {
                    vm.publishEvent(target.id)
                    pendingPublish = null
                }) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = {
                TextButton(onClick = { pendingPublish = null }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }

    pendingUnpublish?.let { target ->
        AlertDialog(
            onDismissRequest = { pendingUnpublish = null },
            title = { Text(stringResource(R.string.event_unpublish)) },
            text = { Text(stringResource(R.string.event_unpublish_confirm)) },
            confirmButton = {
                TextButton(onClick = {
                    vm.unpublishEvent(target.id)
                    pendingUnpublish = null
                }) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = {
                TextButton(onClick = { pendingUnpublish = null }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }

    val quickTarget = state.quickRsvpTarget
    if (quickTarget != null) {
        val scope = rememberCoroutineScope()
        val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
        var selectedGroupId by remember(quickTarget.id, quickTarget.joinAs) {
            mutableStateOf(quickTarget.joinAs.firstOrNull()?.id)
        }
        val joinedMsg = stringResource(R.string.quick_rsvp_joined)
        val waitlistMsg = stringResource(R.string.quick_rsvp_waitlist)
        val canRideCreate = currentMe?.can("rides", "create") == true
        val canDriveCreate = currentMe?.can("drive", "create") == true

        ModalBottomSheet(onDismissRequest = vm::dismissQuickRsvp, sheetState = sheetState) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .navigationBarsPadding()
                    .padding(horizontal = Spacing.sm)
                    .padding(bottom = Spacing.md),
                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
            ) {
                Text(quickTarget.title, style = MaterialTheme.typography.titleLarge)
                if (state.quickRsvpStep == "group") {
                    if (state.quickRsvpLoading) {
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            InlineButtonSpinner()
                            Text(stringResource(R.string.common_loading), style = MaterialTheme.typography.bodyMedium)
                        }
                    } else {
                        if (quickTarget.joinAs.isNotEmpty()) {
                            Text(stringResource(R.string.event_join_as_group), style = MaterialTheme.typography.titleSmall)
                            Column {
                                quickTarget.joinAs.forEach { g ->
                                    Row(
                                        Modifier
                                            .fillMaxWidth()
                                            .clickable { selectedGroupId = g.id }
                                            .padding(vertical = 4.dp),
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        RadioButton(
                                            selected = selectedGroupId == g.id,
                                            onClick = { selectedGroupId = g.id },
                                        )
                                        Text(g.name, style = MaterialTheme.typography.bodyMedium)
                                    }
                                }
                            }
                        }
                        state.quickRsvpError?.let { err ->
                            Text(err, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                        }
                        Button(
                            onClick = {
                                vm.confirmQuickRsvp(
                                    asGroupId = selectedGroupId,
                                    onWaitlistDone = { status ->
                                        scope.launch {
                                            snackbar.showSnackbar(if (status == "waitlist") waitlistMsg else joinedMsg)
                                        }
                                    },
                                    onJoinedNoTransport = {
                                        scope.launch { snackbar.showSnackbar(joinedMsg) }
                                    },
                                )
                            },
                            enabled = !state.quickRsvpBusy && (quickTarget.joinAs.isEmpty() || selectedGroupId != null),
                            colors = graceRidePrimaryButtonColors(),
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            if (state.quickRsvpBusy) {
                                InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                            } else {
                                val full = quickTarget.capacity > 0 && quickTarget.going >= quickTarget.capacity
                                Text(stringResource(if (full) R.string.event_waitlist else R.string.event_signup))
                            }
                        }
                    }
                } else {
                    Text(stringResource(R.string.quick_rsvp_transport_title), style = MaterialTheme.typography.titleSmall)
                    if (canRideCreate) {
                        Button(
                            onClick = {
                                val eid = quickTarget.id
                                vm.dismissQuickRsvp()
                                nav.navigate(AppRoute.RideNew(eventId = eid))
                            },
                            colors = graceRidePrimaryButtonColors(),
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Icon(Icons.Outlined.DirectionsCar, contentDescription = null, modifier = Modifier.size(18.dp))
                            Spacer(Modifier.width(Spacing.xs))
                            Text(stringResource(R.string.event_need_ride))
                        }
                    }
                    if (canDriveCreate) {
                        OutlinedButton(
                            onClick = {
                                val eid = quickTarget.id
                                vm.dismissQuickRsvp()
                                nav.navigate(AppRoute.EventDetail(eid))
                            },
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Text(stringResource(R.string.event_offer_drive))
                        }
                    }
                    TextButton(
                        onClick = vm::quickRsvpSelfTransport,
                        enabled = !state.quickRsvpBusy,
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Text(stringResource(R.string.event_going_own_way))
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ManagingContent(
    state: EventsUiState,
    onRefresh: () -> Unit,
    onToggleHideDrafts: () -> Unit,
    onOpenEvent: (Long) -> Unit,
    onPublishClick: (EventItem) -> Unit,
    onUnpublishClick: (EventItem) -> Unit,
) {
    PullToRefreshBox(isRefreshing = state.refreshing, onRefresh = onRefresh) {
        val bottomClearance = LocalBottomBarClearance.current
        LazyColumn(
            Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
            contentPadding = PaddingValues(top = Spacing.xs, bottom = Spacing.xs + bottomClearance),
        ) {
            item {
                val filterLabel = if (!state.hideDrafts && state.draftCount > 0) {
                    stringResource(R.string.events_hide_drafts_count, state.draftCount)
                } else {
                    stringResource(R.string.events_hide_drafts)
                }
                FilterChip(
                    selected = state.hideDrafts,
                    onClick = onToggleHideDrafts,
                    label = { Text(filterLabel) },
                )
            }
            if (state.events.isEmpty()) {
                item {
                    if (state.hideDrafts) {
                        Column(
                            horizontalAlignment = Alignment.CenterHorizontally,
                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            modifier = Modifier.fillMaxWidth().padding(vertical = Spacing.lg),
                        ) {
                            EmptyView(stringResource(R.string.events_drafts_hidden_empty))
                            OutlinedButton(onClick = onToggleHideDrafts) {
                                Text(stringResource(R.string.events_show_drafts))
                            }
                        }
                    } else {
                        EmptyView(stringResource(R.string.events_empty))
                    }
                }
            }
            items(state.events, key = { it.id }) { event ->
                val busy = state.busyEventId == event.id
                ManagingEventCard(
                    event = event,
                    busy = busy,
                    onClick = { onOpenEvent(event.id) },
                    onPublishClick = { onPublishClick(event) },
                    onUnpublishClick = { onUnpublishClick(event) },
                    modifier = Modifier.animateItem(),
                )
            }
        }
    }
}

@Composable
private fun ManagingEventCard(
    event: EventItem,
    busy: Boolean,
    onClick: () -> Unit,
    onPublishClick: () -> Unit,
    onUnpublishClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = LocalStatusColors.current
    OutlinedCard(onClick = onClick, modifier = modifier.fillMaxWidth()) {
        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
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
                if (event.status == "draft") {
                    Surface(color = c.warningSoft, contentColor = c.warning, shape = CircleShape) {
                        Text(
                            stringResource(R.string.event_status_draft),
                            style = MaterialTheme.typography.labelMedium,
                            modifier = Modifier.padding(horizontal = Spacing.xs, vertical = Spacing.xxs),
                        )
                    }
                } else if (event.status == "cancelled") {
                    Surface(color = c.dangerSoft, contentColor = c.danger, shape = CircleShape) {
                        Text(
                            stringResource(R.string.event_status_cancelled),
                            style = MaterialTheme.typography.labelMedium,
                            modifier = Modifier.padding(horizontal = Spacing.xs, vertical = Spacing.xxs),
                        )
                    }
                } else if (event.status == "finished") {
                    Surface(color = c.infoSoft, contentColor = c.info, shape = CircleShape) {
                        Text(
                            stringResource(R.string.event_status_finished),
                            style = MaterialTheme.typography.labelMedium,
                            modifier = Modifier.padding(horizontal = Spacing.xs, vertical = Spacing.xxs),
                        )
                    }
                }
            }
            Text(
                friendlyTime(event.startsAt) + (if (event.location.isNotBlank()) " · " + event.location else ""),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            if (event.canPublish || event.canUnpublish) {
                Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    if (event.canPublish) {
                        Button(onClick = onPublishClick, enabled = !busy) {
                            if (busy) {
                                InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                            } else {
                                Text(stringResource(R.string.event_publish))
                            }
                        }
                    }
                    if (event.canUnpublish) {
                        OutlinedButton(onClick = onUnpublishClick, enabled = !busy) {
                            if (busy) {
                                InlineButtonSpinner()
                            } else {
                                Text(stringResource(R.string.event_unpublish))
                            }
                        }
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
        val bottomClearance = LocalBottomBarClearance.current
        if (dayGroups.isEmpty()) {
            LazyColumn(
                Modifier.fillMaxSize().padding(Spacing.sm),
                contentPadding = PaddingValues(bottom = bottomClearance),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center,
            ) {
                item {
                    EmptyView(
                        text = stringResource(R.string.events_schedule_empty_title),
                        icon = Icons.Outlined.EventAvailable,
                        subtitle = stringResource(R.string.events_schedule_empty_desc),
                        actionLabel = stringResource(R.string.events_browse_upcoming),
                        onAction = onBrowseUpcoming,
                    )
                }
            }
        } else {
            LazyColumn(
                Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                contentPadding = PaddingValues(top = Spacing.xs, bottom = Spacing.xs + bottomClearance),
            ) {
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
                    Column(
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                        modifier = Modifier.animateItem(),
                    ) {
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
    val ridesByEvent = state.myRides.filter { !it.isProxy }.mapNotNull { ride -> ride.event?.id?.let { it to ride } }.toMap()
    val drivesByEvent = state.myDrives.mapNotNull { ride -> ride.event?.id?.let { it to ride } }.toMap()

    state.events
        .filter { it.mySignup?.status == "going" || it.mySignup?.status == "waitlist" }
        .forEach { event ->
            val going = event.mySignup?.status == "going"
            val hasTransport = event.myRide != null ||
                event.myReturnRide != null ||
                event.mySignup?.selfTransport == true ||
                ridesByEvent.containsKey(event.id) ||
                drivesByEvent.containsKey(event.id)
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
                        eventIcon = event.icon,
                        eventKind = event.kind,
                        onClick = { nav.navigate(AppRoute.EventDetail(event.id)) },
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
                    onClick = { nav.navigate(AppRoute.RideDetail(ride.id)) },
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
                    onClick = { nav.navigate(AppRoute.RideDetail(ride.id)) },
                ),
            )
        }
    }

    dated.sortBy { KlTime.sortKey(it.sortAt) }
    val grouped = linkedMapOf<String, MutableList<TimelineNode>>()
    dated.forEach { item ->
        grouped.getOrPut(item.dayKey) { mutableListOf() }.add(item.node)
    }
    return grouped.entries.map { it.key to it.value }
}

private fun scheduleStateForRide(status: String): TimelineState = when (status) {
    "completed", "no_show", "alt_transport", "cancelled" -> TimelineState.DONE
    "confirmed", "in_progress" -> TimelineState.ACTIVE
    else -> TimelineState.UPCOMING
}
