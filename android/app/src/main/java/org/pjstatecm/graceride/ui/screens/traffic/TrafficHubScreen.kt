package org.pjstatecm.graceride.ui.screens.traffic

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.snap
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.Accessible
import androidx.compose.material.icons.automirrored.outlined.AltRoute
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Download
import androidx.compose.material.icons.outlined.EditNote
import androidx.compose.material.icons.outlined.FilterList
import androidx.compose.material.icons.outlined.LocationOn
import androidx.compose.material.icons.outlined.Lock
import androidx.compose.material.icons.outlined.Notifications
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material.icons.outlined.Schedule
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material.icons.outlined.WarningAmber
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Badge
import androidx.compose.material3.BadgedBox
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.PrimaryScrollableTabRow
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.res.pluralStringResource
import org.pjstatecm.graceride.ui.components.AdaptiveContentFrame
import org.pjstatecm.graceride.ui.components.AdaptivePairRow
import org.pjstatecm.graceride.ui.components.AdaptiveTabRow
import org.pjstatecm.graceride.ui.components.AdaptiveTwoColumnRow
import org.pjstatecm.graceride.ui.components.NotificationBellButton
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.repeatOnLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.platform.CsvBuilder
import org.pjstatecm.graceride.platform.ExportShare
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.LiveDriverDto
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.RideChangeDraftDto
import org.pjstatecm.graceride.data.api.TrafficEventPlanItemDto
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.Assistance
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.LocalBottomBarClearance
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.EventIcon
import org.pjstatecm.graceride.ui.components.GraceRideLiveMap
import org.pjstatecm.graceride.ui.components.GraceRideSpringEasing
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.LiveMapPoint
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.MapMarkerKind
import org.pjstatecm.graceride.ui.components.RideStatusPill
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.components.animatorsEnabled
import org.pjstatecm.graceride.ui.components.friendlyTime
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular
import java.time.Duration

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TrafficHubScreen(
    container: AppContainer,
    nav: NavHostController,
    initialTab: String = "plans",
    initialRidesView: String = "rides",
    initialAreaTab: String = "photos",
    initialRidesStatus: String? = null,
    initialRidesAttention: Boolean = false,
    initialPlanFilter: String? = null,
) {
    val vm: TrafficHubViewModel = viewModel(
        key = "traffic-$initialTab-$initialRidesView-$initialAreaTab-${initialRidesStatus.orEmpty()}-$initialRidesAttention-${initialPlanFilter.orEmpty()}",
        factory = TrafficHubViewModel.factory(
            container = container,
            initialTab = initialTab,
            initialRidesView = initialRidesView,
            initialAreaTab = initialAreaTab,
            initialRidesStatus = initialRidesStatus,
            initialRidesAttention = initialRidesAttention,
            initialPlanFilter = initialPlanFilter,
        ),
    )
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val currentMe = (authState as? AuthState.SignedIn)?.me ?: container.auth.me
    val snackbar = remember { SnackbarHostState() }
    val view = LocalView.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val bottomClearance = LocalBottomBarClearance.current
    val statusColors = LocalStatusColors.current

    LaunchedEffect(currentMe) { vm.syncMe(currentMe) }

    val batchPartialText = state.batchApprovedCount?.let { ok ->
        if (state.batchFailedCount > 0) stringResource(R.string.traffic_rides_batch_partial, ok, state.batchFailedCount)
        else stringResource(R.string.traffic_rides_batch_done, ok)
    }
    LaunchedEffect(state.batchApprovedCount, state.batchFailedCount, state.message) {
        val snack = batchPartialText ?: state.message
        if (snack != null) {
            snackbar.showSnackbar(snack)
            vm.clearMessage()
        }
    }

    // Poll Live Map every 15s while Live tab is active and screen is STARTED.
    LaunchedEffect(lifecycleOwner, state.tab) {
        if (state.tab != "live") return@LaunchedEffect
        lifecycleOwner.repeatOnLifecycle(Lifecycle.State.STARTED) {
            while (true) {
                delay(15_000L)
                vm.loadLive(silent = true)
            }
        }
    }

    val isLead = currentMe?.canApprovePlan() == true
    val stats = state.stats
    val plansBadge = remember(isLead, stats, state.plans) {
        if (isLead) {
            if (state.plans.isNotEmpty()) state.plans.count { it.pendingDraftCount > 0 }.toLong()
            else stats?.pendingPlanDrafts ?: 0L
        } else {
            stats?.myReturnedPlanDrafts ?: 0L
        }
    }
    val ridesBadge = remember(isLead, stats, state.rideDrafts, currentMe?.id) {
        if (isLead) {
            val draftPending = if (state.rideDrafts.isNotEmpty()) {
                state.rideDrafts.count { it.status == "pending_approval" }.toLong()
            } else {
                stats?.pendingRideDrafts ?: 0L
            }
            draftPending + (stats?.pendingApproval ?: 0L) + (stats?.noDriver ?: 0L)
        } else {
            stats?.myReturnedRideDrafts ?: 0L
        }
    }
    val liveBadge = remember(state.liveData, stats) {
        state.liveData?.drivers?.size?.toLong() ?: stats?.liveDrivers ?: 0L
    }

    val permittedTabs = remember(currentMe) {
        buildList {
            if (currentMe?.can("admin_plan") == true) add("plans" to R.string.traffic_tab_plans)
            if (currentMe?.can("admin_rides") == true) add("rides" to R.string.traffic_tab_rides)
            if (currentMe?.can("admin_live") == true) add("live" to R.string.traffic_tab_live)
            if (currentMe?.let { it.can("admin_area_photos") || it.can("admin_areas") || it.can("admin_fares") } == true) {
                add("areas" to R.string.traffic_tab_areas)
            }
        }
    }

    Box(Modifier.fillMaxSize()) {
        Column(Modifier.fillMaxSize()) {
            val unread = currentMe?.unread ?: 0
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(start = Spacing.sm, end = Spacing.xxs, top = Spacing.xs, bottom = Spacing.xxs),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    stringResource(R.string.traffic_title),
                    style = MaterialTheme.typography.headlineSmall,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                Row(verticalAlignment = Alignment.CenterVertically) {
                    IconButton(onClick = { vm.refreshActive(silent = false) }) {
                        Icon(Icons.Outlined.Refresh, contentDescription = stringResource(R.string.common_refresh))
                    }
                    NotificationBellButton(
                        unreadCount = unread,
                        onClick = { nav.navigate(AppRoute.Notifications) },
                        contentDescription = stringResource(R.string.notif_title),
                    )
                }
            }

            if (permittedTabs.isNotEmpty()) {
                val selectedIdx = permittedTabs.indexOfFirst { it.first == state.tab }.coerceAtLeast(0)
                AdaptiveTabRow(selectedTabIndex = selectedIdx, scrollableBelow = 520.dp, edgePadding = Spacing.sm) {
                    permittedTabs.forEach { (key, labelRes) ->
                        val count = when (key) {
                            "plans" -> plansBadge
                            "rides" -> ridesBadge
                            "live" -> liveBadge
                            else -> 0L
                        }
                        androidx.compose.material3.Tab(
                            selected = state.tab == key,
                            onClick = {
                                UiHaptics.tick(view)
                                vm.setTab(key)
                            },
                            text = {
                                Row(
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(5.dp),
                                ) {
                                    if (key == "live" && count > 0L) {
                                        Box(
                                            Modifier
                                                .size(7.dp)
                                                .clip(CircleShape)
                                                .background(statusColors.success),
                                        )
                                    }
                                    Text(
                                        stringResource(labelRes),
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                    )
                                    if (count > 0L) {
                                        val isLiveTab = key == "live"
                                        Surface(
                                            color = if (isLiveTab) statusColors.successSoft else statusColors.warningSoft,
                                            contentColor = if (isLiveTab) statusColors.success else statusColors.warning,
                                            shape = CircleShape,
                                        ) {
                                            Text(
                                                if (count > 99L) "99+" else count.toString(),
                                                style = MaterialTheme.typography.labelSmall.tabular(),
                                                modifier = Modifier.padding(horizontal = 6.dp, vertical = 1.dp),
                                            )
                                        }
                                    }
                                }
                            },
                        )
                    }
                }
            }

            // Hoisted actionable banners visible across all Traffic tabs.
            val pendingBannerCounts = if (isLead && stats != null && (stats.pendingPlanDrafts + stats.pendingRideDrafts) > 0L) {
                stats.pendingPlanDrafts to stats.pendingRideDrafts
            } else {
                null
            }
            var lastPendingBannerCounts by remember { mutableStateOf<Pair<Long, Long>?>(null) }
            if (pendingBannerCounts != null) lastPendingBannerCounts = pendingBannerCounts

            val returnedBannerCounts = if (stats != null && (stats.myReturnedPlanDrafts + stats.myReturnedRideDrafts) > 0L) {
                stats.myReturnedPlanDrafts to stats.myReturnedRideDrafts
            } else {
                null
            }
            var lastReturnedBannerCounts by remember { mutableStateOf<Pair<Long, Long>?>(null) }
            if (returnedBannerCounts != null) lastReturnedBannerCounts = returnedBannerCounts

            val bannerEnter = if (animatorsEnabled()) {
                expandVertically(tween(200, easing = GraceRideSpringEasing)) + fadeIn(tween(160))
            } else {
                fadeIn(tween(100))
            }
            val bannerExit = if (animatorsEnabled()) {
                shrinkVertically(tween(180, easing = GraceRideSpringEasing)) + fadeOut(tween(120))
            } else {
                fadeOut(tween(80))
            }

            AnimatedVisibility(visible = pendingBannerCounts != null, enter = bannerEnter, exit = bannerExit) {
                lastPendingBannerCounts?.let { (planCnt, rideCnt) ->
                    Box(Modifier.fillMaxWidth().padding(horizontal = Spacing.sm, vertical = Spacing.xxs)) {
                        CalloutBanner(
                            icon = Icons.Outlined.EditNote,
                            text = stringResource(R.string.traffic_banner_pending_lead, planCnt, rideCnt),
                            containerColor = statusColors.warningSoft,
                            contentColor = statusColors.warning,
                            actionLabel = stringResource(R.string.traffic_review_now),
                            onAction = { vm.jumpToPendingReview(isLead = true) },
                        )
                    }
                }
            }
            AnimatedVisibility(visible = returnedBannerCounts != null, enter = bannerEnter, exit = bannerExit) {
                lastReturnedBannerCounts?.let { (planCnt, rideCnt) ->
                    Box(Modifier.fillMaxWidth().padding(horizontal = Spacing.sm, vertical = Spacing.xxs)) {
                        CalloutBanner(
                            icon = Icons.Outlined.WarningAmber,
                            text = stringResource(R.string.traffic_banner_returned, planCnt, rideCnt),
                            containerColor = statusColors.infoSoft,
                            contentColor = statusColors.info,
                            actionLabel = stringResource(R.string.traffic_review_now),
                            onAction = { vm.jumpToReturnedDrafts() },
                        )
                    }
                }
            }

            AdaptiveContentFrame(maxWidth = 920.dp, compactHorizontalPadding = 0.dp, modifier = Modifier.weight(1f)) {
                AnimatedContent(
                    targetState = state.tab,
                    transitionSpec = {
                        if (animatorsEnabled()) {
                            val fromIdx = permittedTabs.indexOfFirst { it.first == initialState }
                            val toIdx = permittedTabs.indexOfFirst { it.first == targetState }
                            val dir = if (fromIdx != -1 && toIdx != -1 && toIdx != fromIdx) {
                                if (toIdx > fromIdx) 1 else -1
                            } else {
                                1
                            }
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
                    label = "traffic_hub_tab",
                    modifier = Modifier.fillMaxSize(),
                ) { activeTab ->
                    when (activeTab) {
                        "plans" -> TrafficPlansTab(state, currentMe, vm, nav, bottomClearance)
                        "rides" -> TrafficRidesTab(state, currentMe, vm, nav, bottomClearance)
                        "live" -> TrafficLiveTab(state, vm, bottomClearance)
                        "areas" -> TrafficAreasSection(state, currentMe, vm, container::fileUrl, bottomClearance)
                    }
                }
            }
        }
        val canUpdateRidesRoot = currentMe?.can("admin_rides", "update") == true || currentMe?.can("rides", "update") == true
        val hasBatchBar = state.tab == "rides" &&
            state.ridesView == "rides" &&
            state.selectedRideIds.isNotEmpty() &&
            canUpdateRidesRoot
        val snackbarBottomPadding by animateDpAsState(
            targetValue = if (hasBatchBar) bottomClearance + 76.dp else bottomClearance,
            animationSpec = if (animatorsEnabled()) tween(220, easing = GraceRideSpringEasing) else snap(),
            label = "traffic_snackbar_bottom",
        )
        SnackbarHost(
            hostState = snackbar,
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .padding(bottom = snackbarBottomPadding),
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
private fun TrafficPlansTab(
    state: TrafficHubUiState,
    me: Me?,
    vm: TrafficHubViewModel,
    nav: NavHostController,
    bottomClearance: Dp,
) {
    if (state.plansLoading && state.plans.isEmpty()) {
        LoadingView()
        return
    }
    if (state.plansError != null && state.plans.isEmpty()) {
        ErrorView(state.plansError, onRetry = { vm.refreshActive(silent = false) })
        return
    }

    val isLead = me?.canApprovePlan() == true
    val filtered = remember(state.plans, state.planFilter) {
        when (state.planFilter) {
            "needs_work" -> state.plans.filter { it.unassignedCount > 0 || it.pendingConfirmCount > 0 }
            "drafts" -> state.plans.filter { it.draftCount > 0 }
            "pending" -> state.plans.filter { it.pendingDraftCount > 0 }
            "returned" -> state.plans.filter { it.returnedDraftCount > 0 }
            else -> state.plans
        }
    }
    val today = remember { KlTime.today() }
    val nextEvent = remember(filtered, state.planFilter) {
        if (state.planFilter == "all") filtered.firstOrNull { it.startsAt.take(10) >= today } else null
    }
    val remainingPlans = remember(filtered, nextEvent) {
        if (nextEvent == null) filtered else filtered.filterNot { it.id == nextEvent.id }
    }
    val filterCounts = remember(state.plans) {
        mapOf(
            "all" to state.plans.size,
            "needs_work" to state.plans.count { it.unassignedCount > 0 || it.pendingConfirmCount > 0 },
            "drafts" to state.plans.count { it.draftCount > 0 },
            "pending" to state.plans.count { it.pendingDraftCount > 0 },
            "returned" to state.plans.count { it.returnedDraftCount > 0 },
        )
    }

    PullToRefreshBox(
        isRefreshing = state.refreshing,
        onRefresh = { vm.refreshActive(silent = false) },
        modifier = Modifier.fillMaxSize(),
    ) {
        LazyColumn(
            Modifier.fillMaxSize(),
            contentPadding = PaddingValues(
                start = Spacing.sm,
                end = Spacing.sm,
                top = Spacing.xs,
                bottom = Spacing.lg + bottomClearance,
            ),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            if (nextEvent != null) {
                item(key = "next-plan-ev-${nextEvent.id}") {
                    TrafficNextEventHeroCard(
                        event = nextEvent,
                        isLead = isLead,
                        modifier = Modifier.animateItem(),
                        onClick = { nav.navigate(AppRoute.PlanTransport(nextEvent.id, nextEvent.latestDraft?.id)) },
                    )
                }
            }

            item {
                val chips = listOf(
                    "all" to R.string.traffic_plans_all,
                    "needs_work" to R.string.traffic_plans_needs_work,
                    "drafts" to R.string.traffic_plans_drafts,
                    "pending" to R.string.traffic_plans_pending,
                    "returned" to R.string.traffic_plans_returned,
                )
                LazyRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    items(chips) { (key, res) ->
                        val cnt = filterCounts[key] ?: 0
                        FilterChip(
                            selected = state.planFilter == key,
                            onClick = { vm.setPlanFilter(key) },
                            label = {
                                val text = stringResource(res)
                                Text(if (cnt > 0) "$text ($cnt)" else text)
                            },
                        )
                    }
                }
            }

            if (filtered.isEmpty()) {
                item {
                    EmptyView(
                        text = stringResource(R.string.traffic_plans_empty),
                        icon = Icons.AutoMirrored.Outlined.AltRoute,
                    )
                }
            } else {
                if (nextEvent != null && remainingPlans.isNotEmpty()) {
                    item {
                        SectionHeader(
                            title = stringResource(R.string.traffic_more_events),
                            count = remainingPlans.size,
                        )
                    }
                }
                items(remainingPlans, key = { "plan-ev-${it.id}" }) { ev ->
                    TrafficPlanEventCard(
                        event = ev,
                        isLead = isLead,
                        modifier = Modifier.animateItem(),
                        onClick = { nav.navigate(AppRoute.PlanTransport(ev.id, ev.latestDraft?.id)) },
                    )
                }
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun TrafficNextEventHeroCard(
    event: TrafficEventPlanItemDto,
    isLead: Boolean,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
) {
    val c = LocalStatusColors.current
    val assignedCount = (event.rideCount - event.unassignedCount).coerceAtLeast(0L)
    val rawProgress = if (event.rideCount > 0L) (assignedCount.toFloat() / event.rideCount.toFloat()).coerceIn(0f, 1f) else 1f
    val progress by animateFloatAsState(
        targetValue = rawProgress,
        animationSpec = if (animatorsEnabled()) tween(240, easing = GraceRideSpringEasing) else snap(),
        label = "hero_plan_progress",
    )

    OutlinedCard(
        onClick = onClick,
        modifier = modifier.fillMaxWidth(),
        border = BorderStroke(1.5.dp, MaterialTheme.colorScheme.primary),
    ) {
        Column(
            Modifier
                .fillMaxWidth()
                .background(MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.45f))
                .padding(Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    stringResource(R.string.traffic_next_event),
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.primary,
                )
                event.latestDraft?.lockedBy?.let { lockUser ->
                    SmallBadgePill(
                        stringResource(R.string.traffic_plan_locked_by, lockUser.name),
                        c.warningSoft,
                        c.warning,
                    )
                } ?: event.latestDraft?.let { d ->
                    PlanDraftStatusPill(d.status)
                }
            }
            Row(
                Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                EventIcon(icon = event.icon, kind = event.kind, size = 28.dp)
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Text(
                        event.title,
                        style = MaterialTheme.typography.titleMedium,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                    )
                    val meta = listOfNotNull(
                        KlTime.display(event.startsAt),
                        event.areaName?.takeIf { it.isNotBlank() } ?: event.location.takeIf { it.isNotBlank() },
                    ).joinToString(" · ")
                    Text(
                        meta,
                        style = MaterialTheme.typography.bodySmall.tabular(),
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                Button(
                    onClick = onClick,
                    colors = graceRidePrimaryButtonColors(),
                    contentPadding = PaddingValues(horizontal = 12.dp, vertical = 6.dp),
                ) {
                    Text(planCtaLabel(event, isLead), maxLines = 1)
                }
            }
            if (event.rideCount > 0L) {
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Row(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text(
                            stringResource(R.string.traffic_plan_assigned_progress, assignedCount, event.rideCount),
                            style = MaterialTheme.typography.labelMedium.tabular(),
                        )
                        if (event.unassignedCount > 0L) {
                            Text(
                                stringResource(R.string.traffic_plan_unassigned, event.unassignedCount),
                                style = MaterialTheme.typography.labelMedium.tabular(),
                                color = c.warning,
                            )
                        }
                    }
                    LinearProgressIndicator(
                        progress = { progress },
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(6.dp)
                            .clip(CircleShape),
                        color = if (event.unassignedCount > 0L) c.warning else c.success,
                        trackColor = MaterialTheme.colorScheme.surfaceVariant,
                    )
                }
            }
            FlowRow(
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                event.latestDraft?.let { d ->
                    SmallBadgePill(
                        stringResource(R.string.traffic_plan_draft_meta, d.version, d.updatedBy?.name ?: d.label.ifBlank { "#${d.version}" }),
                        c.infoSoft,
                        c.info,
                    )
                }
                if (event.visitorCount > 0L) {
                    SmallBadgePill(
                        stringResource(R.string.traffic_plan_visitors, event.visitorCount),
                        MaterialTheme.colorScheme.secondaryContainer,
                        MaterialTheme.colorScheme.onSecondaryContainer,
                    )
                }
                if (event.pendingConfirmCount > 0L) {
                    SmallBadgePill(
                        stringResource(R.string.traffic_plan_pending_confirm, event.pendingConfirmCount),
                        c.infoSoft,
                        c.info,
                    )
                }
            }
        }
    }
}

@Composable
private fun planCtaLabel(event: TrafficEventPlanItemDto, isLead: Boolean): String = when {
    isLead && event.pendingDraftCount > 0 -> stringResource(R.string.traffic_plan_cta_review)
    event.latestDraft?.status in setOf("draft", "returned") -> stringResource(R.string.traffic_plan_cta_continue)
    event.draftCount == 0L -> stringResource(R.string.traffic_plan_cta_start)
    else -> stringResource(R.string.traffic_plan_cta_open)
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun TrafficPlanEventCard(
    event: TrafficEventPlanItemDto,
    isLead: Boolean,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
) {
    val c = LocalStatusColors.current
    val assignedCount = (event.rideCount - event.unassignedCount).coerceAtLeast(0L)
    val rawProgress = if (event.rideCount > 0L) (assignedCount.toFloat() / event.rideCount.toFloat()).coerceIn(0f, 1f) else 1f
    val progress by animateFloatAsState(
        targetValue = rawProgress,
        animationSpec = if (animatorsEnabled()) tween(240, easing = GraceRideSpringEasing) else snap(),
        label = "list_plan_progress",
    )
    OutlinedCard(
        onClick = onClick,
        modifier = modifier.fillMaxWidth(),
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
    ) {
        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                EventIcon(icon = event.icon, kind = event.kind, size = 24.dp)
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Text(
                        event.title,
                        style = MaterialTheme.typography.titleMedium,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                    )
                    val meta = listOfNotNull(
                        KlTime.display(event.startsAt),
                        event.areaName?.takeIf { it.isNotBlank() } ?: event.location.takeIf { it.isNotBlank() },
                    ).joinToString(" · ")
                    Text(
                        meta,
                        style = MaterialTheme.typography.bodySmall.tabular(),
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                event.latestDraft?.let { d ->
                    PlanDraftStatusPill(d.status)
                }
            }
            if (event.rideCount > 0L) {
                LinearProgressIndicator(
                    progress = { progress },
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(5.dp)
                        .clip(CircleShape),
                    color = if (event.unassignedCount > 0L) c.warning else c.success,
                    trackColor = MaterialTheme.colorScheme.surfaceVariant,
                )
            }
            FlowRow(
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                SmallBadgePill(
                    pluralStringResource(
                        R.plurals.traffic_plan_rides_count_plural,
                        event.rideCount.toInt(),
                        event.rideCount.toInt(),
                    ),
                    MaterialTheme.colorScheme.surfaceVariant,
                    MaterialTheme.colorScheme.onSurfaceVariant,
                )
                if (event.unassignedCount > 0L) {
                    SmallBadgePill(stringResource(R.string.traffic_plan_unassigned, event.unassignedCount), c.warningSoft, c.warning)
                }
                if (event.pendingConfirmCount > 0L) {
                    SmallBadgePill(stringResource(R.string.traffic_plan_pending_confirm, event.pendingConfirmCount), c.infoSoft, c.info)
                }
                if (event.visitorCount > 0L) {
                    SmallBadgePill(
                        stringResource(R.string.traffic_plan_visitors, event.visitorCount),
                        MaterialTheme.colorScheme.secondaryContainer,
                        MaterialTheme.colorScheme.onSecondaryContainer,
                    )
                }
                if (event.draftCount > 0L) {
                    SmallBadgePill(stringResource(R.string.traffic_plan_drafts_count, event.draftCount), c.infoSoft, c.info)
                }
                event.latestDraft?.lockedBy?.let { lockUser ->
                    SmallBadgePill(
                        stringResource(R.string.traffic_plan_locked_by, lockUser.name),
                        c.warningSoft,
                        c.warning,
                    )
                }
            }
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                val noteText = event.latestDraft?.reviewNotes?.takeIf { it.isNotBlank() }
                if (noteText != null) {
                    Text(
                        noteText,
                        style = MaterialTheme.typography.bodySmall,
                        color = c.warning,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.weight(1f),
                    )
                } else {
                    Spacer(Modifier.weight(1f))
                }
                Text(
                    planCtaLabel(event, isLead),
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.primary,
                )
            }
        }
    }
}

@Composable
internal fun PlanDraftStatusPill(status: String) {
    val (res, kind) = when (status) {
        "pending_approval" -> R.string.plan_status_pending to RideStatusKind.PENDING
        "returned" -> R.string.plan_status_returned to RideStatusKind.CANCELLED
        "approved" -> R.string.plan_status_approved to RideStatusKind.ACTIVE
        else -> R.string.plan_status_draft to RideStatusKind.DONE
    }
    StatusPill(stringResource(res), kind)
}

@Composable
internal fun SmallBadgePill(text: String, bg: Color, fg: Color) {
    Surface(color = bg, contentColor = fg, shape = CircleShape) {
        Text(
            text,
            style = MaterialTheme.typography.labelMedium.tabular(),
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp),
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
private fun TrafficRidesTab(
    state: TrafficHubUiState,
    me: Me?,
    vm: TrafficHubViewModel,
    nav: NavHostController,
    bottomClearance: Dp,
) {
    var returningDraft by remember { mutableStateOf<RideChangeDraftDto?>(null) }
    var returnNotes by remember { mutableStateOf("") }
    var showFiltersSheet by remember { mutableStateOf(false) }
    val context = LocalContext.current
    val exportScope = androidx.compose.runtime.rememberCoroutineScope()
    val canUpdateRides = me?.can("admin_rides", "update") == true || me?.can("rides", "update") == true
    val pendingDraftCount = remember(state.rideDrafts, state.stats) {
        if (state.rideDrafts.isNotEmpty()) state.rideDrafts.count { it.status == "pending_approval" }
        else (state.stats?.pendingRideDrafts ?: 0L).toInt()
    }
    val activeFilterCount = remember(state.ridesDate, state.ridesStatus, state.ridesAttention) {
        listOf(
            state.ridesDate.isNotBlank(),
            state.ridesStatus.isNotBlank(),
            state.ridesAttention,
        ).count { it }
    }

    Box(Modifier.fillMaxSize()) {
        Column(Modifier.fillMaxSize()) {
            // Compact search + filter sheet trigger row
            AnimatedVisibility(
                visible = state.ridesView == "rides",
                enter = if (animatorsEnabled()) {
                    expandVertically(tween(200, easing = GraceRideSpringEasing)) + fadeIn(tween(160))
                } else {
                    fadeIn(tween(100))
                },
                exit = if (animatorsEnabled()) {
                    shrinkVertically(tween(180, easing = GraceRideSpringEasing)) + fadeOut(tween(120))
                } else {
                    fadeOut(tween(80))
                },
            ) {
                Row(
                    Modifier
                        .fillMaxWidth()
                        .padding(start = Spacing.sm, end = Spacing.sm, top = Spacing.xs),
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    OutlinedTextField(
                        value = state.ridesQuery,
                        onValueChange = { vm.setRidesQuery(it) },
                        placeholder = {
                            Text(
                                stringResource(R.string.traffic_rides_search_hint),
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                        },
                        leadingIcon = { Icon(Icons.Outlined.Search, contentDescription = null) },
                        trailingIcon = {
                            if (state.ridesQuery.isNotEmpty()) {
                                IconButton(onClick = { vm.clearRidesQuery() }) {
                                    Icon(Icons.Outlined.Close, contentDescription = stringResource(R.string.a11y_clear))
                                }
                            }
                        },
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                        keyboardActions = KeyboardActions(onSearch = { vm.applyRidesFilters() }),
                        modifier = Modifier.weight(1f),
                    )
                    FilterChip(
                        selected = activeFilterCount > 0,
                        onClick = { showFiltersSheet = true },
                        leadingIcon = { Icon(Icons.Outlined.FilterList, contentDescription = null, modifier = Modifier.size(18.dp)) },
                        label = {
                            Text(
                                if (activeFilterCount > 0) stringResource(R.string.traffic_rides_filters_active, activeFilterCount)
                                else stringResource(R.string.traffic_rides_filters),
                                maxLines = 1,
                            )
                        },
                    )
                    if (state.ridesPage.items.isNotEmpty()) {
                        IconButton(
                            onClick = {
                                exportScope.launch {
                                    val bytes = CsvBuilder.buildBytes(
                                        headers = listOf("ID", "Pickup", "Passenger", "Seats", "Area", "Destination", "Driver", "Status", "Attention"),
                                        rows = state.ridesPage.items.map { r ->
                                            listOf(
                                                r.id.toString(),
                                                r.pickupAt,
                                                r.passenger.name,
                                                r.seats.toString(),
                                                r.area.name,
                                                r.destination,
                                                r.driver?.name.orEmpty(),
                                                r.status,
                                                if (r.needsAttention) "yes" else "no",
                                            )
                                        },
                                    )
                                    ExportShare.writeAndShareFile(context, "grace-ride-rides.csv", "text/csv", bytes)
                                }
                            },
                        ) {
                            Icon(Icons.Outlined.Download, contentDescription = stringResource(R.string.common_export_csv))
                        }
                    }
                }
            }

            SingleChoiceSegmentedButtonRow(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
            ) {
                SegmentedButton(
                    selected = state.ridesView == "rides",
                    onClick = { vm.setRidesView("rides") },
                    shape = SegmentedButtonDefaults.itemShape(0, 2),
                ) { Text(stringResource(R.string.traffic_rides_list_view), maxLines = 1, overflow = TextOverflow.Ellipsis) }
                SegmentedButton(
                    selected = state.ridesView == "drafts",
                    onClick = { vm.setRidesView("drafts") },
                    shape = SegmentedButtonDefaults.itemShape(1, 2),
                ) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        Text(stringResource(R.string.traffic_rides_drafts_view), maxLines = 1, overflow = TextOverflow.Ellipsis)
                        if (pendingDraftCount > 0) {
                            SmallBadgePill(
                                pendingDraftCount.toString(),
                                LocalStatusColors.current.warningSoft,
                                LocalStatusColors.current.warning,
                            )
                        }
                    }
                }
            }

            AnimatedContent(
                targetState = state.ridesView,
                transitionSpec = {
                    if (animatorsEnabled()) {
                        val dir = if (targetState == "drafts") 1 else -1
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
                label = "traffic_rides_view",
                modifier = Modifier.weight(1f),
            ) { ridesView ->
                if (ridesView == "drafts") {
                    when {
                        state.rideDraftsLoading && state.rideDrafts.isEmpty() -> LoadingView()
                        state.rideDraftsError != null && state.rideDrafts.isEmpty() -> ErrorView(
                            message = state.rideDraftsError,
                            onRetry = { vm.refreshActive(silent = false) },
                        )
                        else -> {
                            PullToRefreshBox(
                                isRefreshing = state.refreshing,
                                onRefresh = { vm.refreshActive(silent = false) },
                                modifier = Modifier.fillMaxSize(),
                            ) {
                                LazyColumn(
                                    Modifier.fillMaxSize(),
                                    contentPadding = PaddingValues(
                                        start = Spacing.sm,
                                        end = Spacing.sm,
                                        top = Spacing.xxs,
                                        bottom = Spacing.lg + bottomClearance,
                                    ),
                                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                ) {
                                    item {
                                        val draftFilters = listOf(
                                            "" to R.string.traffic_drafts_filter_all,
                                            "pending_approval" to R.string.traffic_drafts_filter_pending,
                                            "returned" to R.string.traffic_drafts_filter_returned,
                                            "draft" to R.string.traffic_drafts_filter_draft,
                                        )
                                        LazyRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                            items(draftFilters) { (key, res) ->
                                                FilterChip(
                                                    selected = state.rideDraftsFilter == key,
                                                    onClick = { vm.setRideDraftsFilter(key) },
                                                    label = { Text(stringResource(res)) },
                                                )
                                            }
                                        }
                                    }
                                    if (state.rideDrafts.isEmpty()) {
                                        item { EmptyView(text = stringResource(R.string.traffic_ride_drafts_empty)) }
                                    } else {
                                        items(state.rideDrafts, key = { "rd-${it.id}" }) { draft ->
                                            val busy = state.rideDraftsBusyId == draft.id
                                            val c = LocalStatusColors.current
                                            OutlinedCard(
                                                onClick = { nav.navigate(AppRoute.RideDetail(draft.rideId)) },
                                                modifier = Modifier
                                                    .fillMaxWidth()
                                                    .animateItem(),
                                                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                                            ) {
                                                Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                                                    Row(
                                                        Modifier.fillMaxWidth(),
                                                        horizontalArrangement = Arrangement.SpaceBetween,
                                                        verticalAlignment = Alignment.CenterVertically,
                                                    ) {
                                                        Text(
                                                            "#${draft.rideId} · ${draft.ride?.passengerName.orEmpty()} (v${draft.version})",
                                                            style = MaterialTheme.typography.titleMedium,
                                                            modifier = Modifier.weight(1f),
                                                            maxLines = 1,
                                                            overflow = TextOverflow.Ellipsis,
                                                        )
                                                        PlanDraftStatusPill(draft.status)
                                                    }
                                                    Text(
                                                        "${rideDraftActionLabel(draft.action)}${draft.driverName?.let { " → $it" } ?: ""}",
                                                        style = MaterialTheme.typography.bodyMedium,
                                                        color = MaterialTheme.colorScheme.primary,
                                                    )
                                                    draft.ride?.let { r ->
                                                        Text(
                                                            "${KlTime.display(r.pickupAt)} · ${r.areaName}",
                                                            style = MaterialTheme.typography.bodySmall.tabular(),
                                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                        )
                                                    }
                                                    draft.lockedBy?.let { locker ->
                                                        SmallBadgePill(
                                                            stringResource(R.string.traffic_plan_locked_by, locker.name),
                                                            c.warningSoft,
                                                            c.warning,
                                                        )
                                                    }
                                                    if (draft.reason.isNotBlank()) {
                                                        Text(draft.reason, style = MaterialTheme.typography.bodySmall)
                                                    }
                                                    draft.reviewNotes?.takeIf { it.isNotBlank() }?.let { notes ->
                                                        Text(notes, style = MaterialTheme.typography.bodySmall, color = c.warning)
                                                    }
                                                    if (me?.canApprovePlan() == true && draft.status == "pending_approval") {
                                                        Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                                            Button(
                                                                onClick = { vm.approveRideDraft(draft.rideId, draft.id) },
                                                                enabled = !busy,
                                                                colors = graceRidePrimaryButtonColors(),
                                                            ) {
                                                                if (busy) {
                                                                    InlineButtonSpinner()
                                                                    Spacer(Modifier.width(Spacing.xs))
                                                                }
                                                                Text(stringResource(R.string.ride_draft_approve))
                                                            }
                                                            OutlinedButton(
                                                                onClick = {
                                                                    returnNotes = ""
                                                                    returningDraft = draft
                                                                },
                                                                enabled = !busy,
                                                            ) { Text(stringResource(R.string.ride_draft_return)) }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    when {
                        state.ridesLoading && state.ridesPage.items.isEmpty() -> LoadingView()
                        state.ridesError != null && state.ridesPage.items.isEmpty() -> ErrorView(
                            message = state.ridesError,
                            onRetry = { vm.refreshActive(silent = false) },
                        )
                        else -> {
                            val rides = state.ridesPage.items
                            val approvableOnPage = remember(rides, canUpdateRides) {
                                if (canUpdateRides) rides.filter { it.status == "pending_approval" } else emptyList()
                            }
                            val hasBatchBar = state.selectedRideIds.isNotEmpty() && canUpdateRides
                            PullToRefreshBox(
                                isRefreshing = state.refreshing,
                                onRefresh = { vm.refreshActive(silent = false) },
                                modifier = Modifier.fillMaxSize(),
                            ) {
                                androidx.compose.foundation.layout.BoxWithConstraints(Modifier.fillMaxSize()) {
                                    val useTwoCols = maxWidth >= 640.dp
                                    LazyColumn(
                                        Modifier.fillMaxSize(),
                                        contentPadding = PaddingValues(
                                            start = Spacing.sm,
                                            end = Spacing.sm,
                                            top = Spacing.xxs,
                                            bottom = (if (hasBatchBar) 88.dp else Spacing.lg) + bottomClearance,
                                        ),
                                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    ) {
                                    if (activeFilterCount > 0 || approvableOnPage.isNotEmpty()) {
                                        item {
                                            LazyRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                                if (approvableOnPage.isNotEmpty()) {
                                                    val allSelected = approvableOnPage.all { it.id in state.selectedRideIds }
                                                    item {
                                                        FilterChip(
                                                            selected = allSelected,
                                                            onClick = {
                                                                if (allSelected) vm.clearSelection() else vm.selectAllPendingRides()
                                                            },
                                                            label = {
                                                                Text(stringResource(R.string.traffic_rides_select_all_pending, approvableOnPage.size))
                                                            },
                                                        )
                                                    }
                                                }
                                                if (state.ridesDate.isNotBlank()) {
                                                    item {
                                                        val dateRes = if (state.ridesDate == "today") R.string.traffic_rides_date_today else R.string.traffic_rides_date_sunday
                                                        FilterChip(
                                                            selected = true,
                                                            onClick = { vm.applyRidesFilters(date = "") },
                                                            trailingIcon = { Icon(Icons.Outlined.Close, contentDescription = null, modifier = Modifier.size(16.dp)) },
                                                            label = { Text(stringResource(dateRes)) },
                                                        )
                                                    }
                                                }
                                                if (state.ridesAttention) {
                                                    item {
                                                        FilterChip(
                                                            selected = true,
                                                            onClick = { vm.applyRidesFilters(attention = false) },
                                                            trailingIcon = { Icon(Icons.Outlined.Close, contentDescription = null, modifier = Modifier.size(16.dp)) },
                                                            label = { Text(stringResource(R.string.traffic_rides_attention_only)) },
                                                        )
                                                    }
                                                }
                                                if (state.ridesStatus.isNotBlank()) {
                                                    item {
                                                        FilterChip(
                                                            selected = true,
                                                            onClick = { vm.applyRidesFilters(status = "") },
                                                            trailingIcon = { Icon(Icons.Outlined.Close, contentDescription = null, modifier = Modifier.size(16.dp)) },
                                                            label = { Text(stringResource(rideStatusFilterLabelRes(state.ridesStatus))) },
                                                        )
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    if (rides.isEmpty()) {
                                        item {
                                            EmptyView(
                                                text = stringResource(R.string.traffic_rides_empty),
                                                icon = Icons.Outlined.DirectionsCar,
                                                actionLabel = if (activeFilterCount > 0 || state.ridesQuery.isNotBlank()) {
                                                    stringResource(R.string.traffic_rides_filters_reset)
                                                } else {
                                                    null
                                                },
                                                onAction = if (activeFilterCount > 0 || state.ridesQuery.isNotBlank()) {
                                                    { vm.resetRidesFilters() }
                                                } else {
                                                    null
                                                },
                                            )
                                        }
                                    } else {
                                        if (useTwoCols) {
                                            val chunkedRides = rides.chunked(2)
                                            items(chunkedRides, key = { pair -> "admin-ride-pair-${pair.joinToString("-") { it.id.toString() }}" }) { pair ->
                                                AdaptivePairRow(
                                                    spacing = Spacing.xs,
                                                    left = {
                                                        val ride = pair[0]
                                                        val approvable = canUpdateRides && ride.status == "pending_approval"
                                                        AdminRideCard(
                                                            ride = ride,
                                                            approvable = approvable,
                                                            selected = ride.id in state.selectedRideIds,
                                                            quickApproving = state.quickApprovingRideId == ride.id,
                                                            actionsEnabled = !state.batchBusy && state.quickApprovingRideId == null,
                                                            modifier = Modifier.animateItem(),
                                                            onToggleSelect = { vm.toggleSelectRide(ride.id) },
                                                            onQuickApprove = { vm.quickApproveRide(ride.id) },
                                                            onClick = { nav.navigate(AppRoute.RideDetail(ride.id)) },
                                                        )
                                                    },
                                                    right = pair.getOrNull(1)?.let { rightRide ->
                                                        {
                                                            val approvable = canUpdateRides && rightRide.status == "pending_approval"
                                                            AdminRideCard(
                                                                ride = rightRide,
                                                                approvable = approvable,
                                                                selected = rightRide.id in state.selectedRideIds,
                                                                quickApproving = state.quickApprovingRideId == rightRide.id,
                                                                actionsEnabled = !state.batchBusy && state.quickApprovingRideId == null,
                                                                modifier = Modifier.animateItem(),
                                                                onToggleSelect = { vm.toggleSelectRide(rightRide.id) },
                                                                onQuickApprove = { vm.quickApproveRide(rightRide.id) },
                                                                onClick = { nav.navigate(AppRoute.RideDetail(rightRide.id)) },
                                                            )
                                                        }
                                                    },
                                                )
                                            }
                                        } else {
                                            items(rides, key = { "admin-ride-${it.id}" }) { ride ->
                                                val approvable = canUpdateRides && ride.status == "pending_approval"
                                                AdminRideCard(
                                                    ride = ride,
                                                    approvable = approvable,
                                                    selected = ride.id in state.selectedRideIds,
                                                    quickApproving = state.quickApprovingRideId == ride.id,
                                                    actionsEnabled = !state.batchBusy && state.quickApprovingRideId == null,
                                                    modifier = Modifier.animateItem(),
                                                    onToggleSelect = { vm.toggleSelectRide(ride.id) },
                                                    onQuickApprove = { vm.quickApproveRide(ride.id) },
                                                    onClick = { nav.navigate(AppRoute.RideDetail(ride.id)) },
                                                )
                                            }
                                        }

                                        if (state.ridesPage.totalPages > 1) {
                                            item {
                                                Row(
                                                    Modifier.fillMaxWidth(),
                                                    horizontalArrangement = Arrangement.SpaceBetween,
                                                    verticalAlignment = Alignment.CenterVertically,
                                                ) {
                                                    OutlinedButton(
                                                        onClick = { vm.goToRidesPage(state.ridesPage.page - 1) },
                                                        enabled = state.ridesPage.page > 1 && !state.ridesLoading,
                                                    ) { Text(stringResource(R.string.traffic_prev_page)) }
                                                    Text(
                                                        stringResource(
                                                            R.string.traffic_rides_total_page,
                                                            state.ridesPage.total,
                                                            state.ridesPage.page,
                                                            state.ridesPage.totalPages,
                                                        ),
                                                        style = MaterialTheme.typography.bodySmall.tabular(),
                                                    )
                                                    OutlinedButton(
                                                        onClick = { vm.goToRidesPage(state.ridesPage.page + 1) },
                                                        enabled = state.ridesPage.page < state.ridesPage.totalPages && !state.ridesLoading,
                                                    ) { Text(stringResource(R.string.traffic_next_page)) }
                                                }
                                            }
                                        }
                                    }
                                }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Sticky floating batch approval bar above floating bottom nav pill
        var lastSelectedRideCount by remember { mutableIntStateOf(0) }
        if (state.selectedRideIds.isNotEmpty()) {
            lastSelectedRideCount = state.selectedRideIds.size
        }
        AnimatedVisibility(
            visible = state.ridesView == "rides" && state.selectedRideIds.isNotEmpty() && canUpdateRides,
            enter = if (animatorsEnabled()) {
                slideInVertically(tween(220, easing = GraceRideSpringEasing)) { it } + fadeIn(tween(160))
            } else {
                fadeIn(tween(100))
            },
            exit = if (animatorsEnabled()) {
                slideOutVertically(tween(180, easing = GraceRideSpringEasing)) { it } + fadeOut(tween(120))
            } else {
                fadeOut(tween(80))
            },
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .padding(horizontal = Spacing.sm, vertical = Spacing.xs)
                .padding(bottom = bottomClearance),
        ) {
            Surface(
                color = MaterialTheme.colorScheme.primaryContainer,
                contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                shape = MaterialTheme.shapes.large,
                border = BorderStroke(1.dp, MaterialTheme.colorScheme.primary.copy(alpha = 0.45f)),
                tonalElevation = 4.dp,
                shadowElevation = 4.dp,
            ) {
                Row(
                    Modifier
                        .fillMaxWidth()
                        .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    Text(
                        stringResource(R.string.traffic_rides_selected_count, lastSelectedRideCount),
                        style = MaterialTheme.typography.titleSmall.tabular(),
                        modifier = Modifier.weight(1f),
                    )
                    TextButton(onClick = { vm.clearSelection() }, enabled = !state.batchBusy) {
                        Text(stringResource(R.string.traffic_rides_batch_clear))
                    }
                    Button(
                        onClick = { vm.batchApproveSelected() },
                        enabled = !state.batchBusy,
                        colors = graceRidePrimaryButtonColors(),
                    ) {
                        if (state.batchBusy) {
                            InlineButtonSpinner()
                            Spacer(Modifier.width(Spacing.xs))
                        }
                        Text(stringResource(R.string.traffic_rides_batch_approve, lastSelectedRideCount))
                    }
                }
            }
        }
    }

    if (showFiltersSheet) {
        RidesFilterBottomSheet(
            selectedDate = state.ridesDate,
            selectedStatus = state.ridesStatus,
            attentionOnly = state.ridesAttention,
            onApply = { date, status, attention ->
                vm.applyRidesFilters(status = status, date = date, attention = attention)
                showFiltersSheet = false
            },
            onReset = {
                vm.applyRidesFilters(status = "", date = "", attention = false)
                showFiltersSheet = false
            },
            onDismiss = { showFiltersSheet = false },
        )
    }

    returningDraft?.let { draft ->
        ReturnNotesDialog(
            notes = returnNotes,
            onNotesChange = { returnNotes = it },
            quickReasons = listOf(
                stringResource(R.string.ride_draft_quick_reason_driver),
                stringResource(R.string.ride_draft_quick_reason_area),
                stringResource(R.string.ride_draft_quick_reason_details),
            ),
            onConfirm = {
                vm.returnRideDraft(draft.rideId, draft.id, returnNotes)
                returningDraft = null
            },
            onDismiss = { returningDraft = null },
        )
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun AdminRideCard(
    ride: Ride,
    approvable: Boolean,
    selected: Boolean,
    quickApproving: Boolean,
    actionsEnabled: Boolean,
    modifier: Modifier = Modifier,
    onToggleSelect: () -> Unit,
    onQuickApprove: () -> Unit,
    onClick: () -> Unit,
) {
    val c = LocalStatusColors.current
    val riderDisplay = when {
        ride.isProxy && !ride.proxy?.riderName.isNullOrBlank() -> {
            val rider = ride.proxy!!.riderName.trim()
            val booker = ride.proxy.bookerName
                ?.trim()
                ?.takeIf {
                    it.isNotEmpty() &&
                        !it.equals(rider, ignoreCase = true) &&
                        !rider.contains(it, ignoreCase = true)
                }
            if (booker != null) {
                "$rider ($booker)"
            } else {
                val pName = ride.passenger.name.trim()
                if (pName.contains(rider, ignoreCase = true)) pName else rider
            }
        }
        else -> ride.passenger.name
    }
    OutlinedCard(
        onClick = onClick,
        modifier = modifier.fillMaxWidth(),
        border = BorderStroke(
            1.dp,
            if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outlineVariant,
        ),
    ) {
        Row(
            Modifier
                .fillMaxWidth()
                .padding(Spacing.sm),
            verticalAlignment = Alignment.Top,
            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            if (approvable) {
                Checkbox(
                    checked = selected,
                    onCheckedChange = { onToggleSelect() },
                )
            }
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Row(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        riderDisplay,
                        style = MaterialTheme.typography.titleMedium,
                        modifier = Modifier.weight(1f),
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Spacer(Modifier.width(Spacing.xs))
                    RideStatusPill(ride.status)
                }
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
                        "${friendlyTime(ride.pickupAt)} · ${ride.area.name} → ${ride.destination}",
                        style = MaterialTheme.typography.bodySmall.tabular(),
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                FlowRow(
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    verticalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    SmallBadgePill(
                        stringResource(if (ride.leg == "return") R.string.plan_leg_return else R.string.plan_leg_outbound),
                        MaterialTheme.colorScheme.surfaceVariant,
                        MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    if (ride.seats > 1L) {
                        SmallBadgePill(
                            stringResource(R.string.traffic_rides_seats_short, ride.seats),
                            MaterialTheme.colorScheme.surfaceVariant,
                            MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    if (ride.isProxy) {
                        SmallBadgePill(
                            stringResource(R.string.traffic_rides_proxy_tag),
                            c.warningSoft,
                            c.warning,
                        )
                    }
                    if (ride.assistance.isNotBlank()) {
                        val assistLabel = Assistance.labelRes(ride.assistance)?.let { stringResource(it) } ?: ride.assistance
                        Surface(color = c.infoSoft, contentColor = c.info, shape = CircleShape) {
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(4.dp),
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp),
                            ) {
                                Icon(Icons.AutoMirrored.Outlined.Accessible, contentDescription = null, modifier = Modifier.size(14.dp))
                                Text(assistLabel, style = MaterialTheme.typography.labelMedium)
                            }
                        }
                    }
                    SmallBadgePill(
                        ride.driver?.name ?: stringResource(R.string.ride_status_pending_driver),
                        if (ride.driver != null) MaterialTheme.colorScheme.primaryContainer else c.warningSoft,
                        if (ride.driver != null) MaterialTheme.colorScheme.onPrimaryContainer else c.warning,
                    )
                    if (ride.needsAttention) {
                        SmallBadgePill(
                            ride.attentionReason.ifBlank { stringResource(R.string.traffic_rides_attention_only) },
                            c.warningSoft,
                            c.warning,
                        )
                    }
                }
                if (approvable) {
                    Row(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.End,
                    ) {
                        FilledTonalButton(
                            onClick = onQuickApprove,
                            enabled = actionsEnabled,
                            contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp),
                        ) {
                            if (quickApproving) {
                                InlineButtonSpinner()
                                Spacer(Modifier.width(Spacing.xxs))
                            }
                            Text(stringResource(R.string.traffic_rides_quick_approve))
                        }
                    }
                }
            }
        }
    }
}

private fun rideStatusFilterLabelRes(status: String): Int = when (status) {
    "pending_approval" -> R.string.traffic_rides_status_pending_approval
    "pending_driver" -> R.string.traffic_rides_status_pending_driver
    "driver_assigned" -> R.string.traffic_rides_status_driver_assigned
    "confirmed" -> R.string.traffic_rides_status_confirmed
    "in_progress" -> R.string.traffic_rides_status_in_progress
    "completed" -> R.string.traffic_rides_status_completed
    "cancelled" -> R.string.traffic_rides_status_cancelled
    else -> R.string.traffic_rides_filter_all_status
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
private fun RidesFilterBottomSheet(
    selectedDate: String,
    selectedStatus: String,
    attentionOnly: Boolean,
    onApply: (date: String, status: String, attention: Boolean) -> Unit,
    onReset: () -> Unit,
    onDismiss: () -> Unit,
) {
    var draftDate by remember(selectedDate) { mutableStateOf(selectedDate) }
    var draftStatus by remember(selectedStatus) { mutableStateOf(selectedStatus) }
    var draftAttention by remember(attentionOnly) { mutableStateOf(attentionOnly) }
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
        Column(
            Modifier
                .fillMaxWidth()
                .navigationBarsPadding()
                .padding(horizontal = Spacing.md)
                .padding(bottom = Spacing.md),
            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
        ) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(stringResource(R.string.traffic_rides_filters), style = MaterialTheme.typography.titleLarge)
                TextButton(onClick = onReset) {
                    Text(stringResource(R.string.traffic_rides_filters_reset))
                }
            }
            Column(
                Modifier
                    .weight(1f, fill = false)
                    .verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
            ) {
                val dateChips = listOf(
                    "" to R.string.traffic_rides_date_all,
                    "today" to R.string.traffic_rides_date_today,
                    "sunday" to R.string.traffic_rides_date_sunday,
                )
                FlowRow(
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                ) {
                    dateChips.forEach { (key, res) ->
                        FilterChip(
                            selected = draftDate == key,
                            onClick = { draftDate = key },
                            label = { Text(stringResource(res)) },
                        )
                    }
                }
                Row(
                    Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Text(stringResource(R.string.traffic_rides_attention_only), style = MaterialTheme.typography.bodyMedium)
                    Switch(checked = draftAttention, onCheckedChange = { draftAttention = it })
                }
                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                val statuses = listOf(
                    "" to R.string.traffic_rides_filter_all_status,
                    "pending_approval" to R.string.traffic_rides_status_pending_approval,
                    "pending_driver" to R.string.traffic_rides_status_pending_driver,
                    "driver_assigned" to R.string.traffic_rides_status_driver_assigned,
                    "confirmed" to R.string.traffic_rides_status_confirmed,
                    "in_progress" to R.string.traffic_rides_status_in_progress,
                    "completed" to R.string.traffic_rides_status_completed,
                    "cancelled" to R.string.traffic_rides_status_cancelled,
                )
                FlowRow(
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                ) {
                    statuses.forEach { (key, res) ->
                        FilterChip(
                            selected = draftStatus == key,
                            onClick = { draftStatus = key },
                            label = { Text(stringResource(res)) },
                        )
                    }
                }
            }
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                OutlinedButton(onClick = onDismiss, modifier = Modifier.weight(1f)) {
                    Text(stringResource(R.string.common_cancel))
                }
                Button(
                    onClick = { onApply(draftDate, draftStatus, draftAttention) },
                    colors = graceRidePrimaryButtonColors(),
                    modifier = Modifier.weight(1f),
                ) {
                    Text(stringResource(R.string.common_confirm))
                }
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun ReturnNotesDialog(
    notes: String,
    onNotesChange: (String) -> Unit,
    quickReasons: List<String>,
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
    requireNotes: Boolean = true,
) {
    val canConfirm = !requireNotes || notes.isNotBlank()
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.plan_action_return)) },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                if (quickReasons.isNotEmpty()) {
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        quickReasons.forEach { reason ->
                            FilterChip(
                                selected = notes.contains(reason),
                                onClick = {
                                    val next = if (notes.isBlank()) reason else if (notes.contains(reason)) notes else "$notes; $reason"
                                    onNotesChange(next)
                                },
                                label = { Text(reason, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                            )
                        }
                    }
                }
                OutlinedTextField(
                    value = notes,
                    onValueChange = onNotesChange,
                    label = { Text(stringResource(R.string.plan_return_notes_hint)) },
                    supportingText = if (requireNotes && notes.isBlank()) {
                        { Text(stringResource(R.string.plan_return_notes_required)) }
                    } else {
                        null
                    },
                    minLines = 2,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        },
        confirmButton = {
            Button(onClick = onConfirm, enabled = canConfirm) { Text(stringResource(R.string.plan_return_confirm)) }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text(stringResource(R.string.common_cancel)) }
        },
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun TrafficLiveTab(
    state: TrafficHubUiState,
    vm: TrafficHubViewModel,
    bottomClearance: Dp,
) {
    if (state.liveLoading && state.liveData == null) {
        LoadingView()
        return
    }
    if (state.liveError != null && state.liveData == null) {
        ErrorView(state.liveError, onRetry = { vm.loadLive(silent = false) })
        return
    }

    val lifecycleOwner = LocalLifecycleOwner.current
    var nowMs by remember { mutableLongStateOf(System.currentTimeMillis()) }
    LaunchedEffect(lifecycleOwner) {
        lifecycleOwner.repeatOnLifecycle(Lifecycle.State.STARTED) {
            while (true) {
                delay(5_000L)
                nowMs = System.currentTimeMillis()
            }
        }
    }

    val live = state.liveData
    val drivers = live?.drivers.orEmpty()
    val nowKl = remember(nowMs) { KlTime.now() }
    val staleById = remember(drivers, nowKl) {
        drivers.associate { d ->
            val dt = KlTime.parse(d.updatedAt)
            val stale = dt != null && Duration.between(dt, nowKl).toMinutes() > 2
            d.driverId to stale
        }
    }
    val mapPoints = remember(drivers, staleById) {
        drivers.map { d ->
            LiveMapPoint(
                id = "driver-${d.driverId}",
                lat = d.lat,
                lng = d.lng,
                kind = MapMarkerKind.CAR,
                label = d.name,
                badge = d.rides.takeIf { it > 0L }?.toString(),
                stale = staleById[d.driverId] == true,
            )
        }
    }
    val updatedLabel = state.liveUpdatedAtMs?.let { updatedMs ->
        val ageSec = ((nowMs - updatedMs) / 1000L).coerceAtLeast(0L)
        when {
            ageSec < 10L -> stringResource(R.string.traffic_live_updated_just_now)
            ageSec < 60L -> stringResource(R.string.traffic_live_updated_secs, ageSec)
            else -> stringResource(R.string.traffic_live_updated_mins, ageSec / 60L)
        }
    }

    PullToRefreshBox(
        isRefreshing = state.refreshing,
        onRefresh = { vm.loadLive(silent = false) },
        modifier = Modifier.fillMaxSize(),
    ) {
        LazyColumn(
            Modifier.fillMaxSize(),
            contentPadding = PaddingValues(
                start = Spacing.sm,
                end = Spacing.sm,
                top = Spacing.xs,
                bottom = Spacing.lg + bottomClearance,
            ),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            if (live != null) {
                item {
                    val bannerText = if (updatedLabel != null) {
                        "${stringResource(R.string.traffic_live_cap_banner, drivers.size, live.max)} · $updatedLabel"
                    } else {
                        stringResource(R.string.traffic_live_cap_banner, drivers.size, live.max)
                    }
                    CalloutBanner(
                        icon = Icons.Outlined.LocationOn,
                        text = bannerText,
                        containerColor = MaterialTheme.colorScheme.secondaryContainer,
                        contentColor = MaterialTheme.colorScheme.onSecondaryContainer,
                    )
                }
            }
            item {
                GraceRideLiveMap(
                    points = mapPoints,
                    focusPoint = state.liveFocusPoint,
                    showZoomControls = true,
                    onFitAll = { vm.focusLiveDriver(null) },
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(280.dp),
                )
            }
            if (drivers.isEmpty()) {
                item { EmptyView(text = stringResource(R.string.traffic_live_empty), icon = Icons.Outlined.DirectionsCar) }
            } else {
                items(drivers, key = { "live-drv-${it.driverId}" }) { driver ->
                    LiveDriverCard(
                        driver = driver,
                        isStale = staleById[driver.driverId] == true,
                        modifier = Modifier.animateItem(),
                        onClick = { vm.focusLiveDriver(driver.lat to driver.lng) },
                    )
                }
            }
        }
    }
}

@Composable
private fun LiveDriverCard(
    driver: LiveDriverDto,
    isStale: Boolean,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
) {
    OutlinedCard(
        onClick = onClick,
        modifier = modifier.fillMaxWidth(),
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
    ) {
        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(driver.name, style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
                if (isStale) {
                    StatusPill(stringResource(R.string.traffic_live_stale), RideStatusKind.PENDING)
                } else {
                    StatusPill(
                        pluralStringResource(R.plurals.traffic_live_trips_plural, driver.rides.toInt(), driver.rides),
                        RideStatusKind.ACTIVE,
                    )
                }
            }
            driver.firstPickup?.let { first ->
                Text(
                    stringResource(R.string.traffic_live_first_pickup, KlTime.displayTime(first)),
                    style = MaterialTheme.typography.bodySmall.tabular(),
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            if (driver.updatedAt.isNotBlank()) {
                Text(
                    KlTime.displayTime(driver.updatedAt),
                    style = MaterialTheme.typography.labelSmall.tabular(),
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
internal fun rideDraftActionLabel(action: String): String = when (action) {
    "assign" -> stringResource(R.string.ride_draft_action_assign)
    "reassign" -> stringResource(R.string.ride_draft_action_reassign)
    "return" -> stringResource(R.string.ride_draft_action_return)
    "alt_transport" -> stringResource(R.string.ride_draft_action_alt_transport)
    "cancel" -> stringResource(R.string.ride_draft_action_cancel)
    "approve" -> stringResource(R.string.ride_draft_action_approve)
    "reimbursement_excluded" -> stringResource(R.string.ride_draft_action_reimb_excluded)
    else -> action
}
