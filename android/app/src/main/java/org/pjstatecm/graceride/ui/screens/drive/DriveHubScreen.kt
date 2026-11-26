package org.pjstatecm.graceride.ui.screens.drive

import android.Manifest
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Flag
import androidx.compose.material.icons.outlined.MyLocation
import androidx.compose.material.icons.outlined.Navigation
import androidx.compose.material.icons.outlined.Route
import androidx.compose.material.icons.outlined.Schedule
import androidx.compose.material.icons.outlined.Settings
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.TripRouteDto
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.Assistance
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.service.LiveTripLocationService
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.AnimatedOfflineBanner
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.GraceRideLiveMap
import org.pjstatecm.graceride.ui.components.GraceRideSpringEasing
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.LiveMapPoint
import org.pjstatecm.graceride.ui.components.MapMarkerKind
import org.pjstatecm.graceride.ui.components.RideCard
import org.pjstatecm.graceride.ui.components.SkeletonLoadingList
import org.pjstatecm.graceride.ui.components.animatorsEnabled
import org.pjstatecm.graceride.ui.rememberSmartNavigator
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DriveHubScreen(container: AppContainer, nav: NavHostController) {
    val vm: DriveHubViewModel = viewModel(factory = DriveHubViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val me = (authState as? org.pjstatecm.graceride.data.repo.AuthState.SignedIn)?.me ?: container.auth.me
    val sharingLocation by LiveTripLocationService.isSharing.collectAsStateWithLifecycle()
    val nearDestinationTripId by LiveTripLocationService.nearDestinationTripId.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val view = LocalView.current
    val snackbar = remember { SnackbarHostState() }
    val smartNavigate = rememberSmartNavigator(container)
    val claimedMsg = stringResource(R.string.drive_claimed_msg)
    val permMsg = stringResource(R.string.live_location_perm_needed)

    val locPermDeniedTick = remember { mutableIntStateOf(0) }
    val locPermLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions(),
    ) { granted ->
        if (granted[Manifest.permission.ACCESS_FINE_LOCATION] == true ||
            granted[Manifest.permission.ACCESS_COARSE_LOCATION] == true
        ) {
            val liveRoute = state.route?.takeIf { it.live }
            if (liveRoute != null) {
                LiveTripLocationService.start(
                    context,
                    rideId = liveRoute.stops.firstOrNull()?.rideId ?: -1L,
                    tripId = liveRoute.tripId,
                )
            }
        } else {
            locPermDeniedTick.intValue++
        }
    }
    val lifecycleOwner = androidx.lifecycle.compose.LocalLifecycleOwner.current
    val resumedOnce = remember { mutableStateOf(false) }

    androidx.compose.runtime.DisposableEffect(lifecycleOwner) {
        val obs = androidx.lifecycle.LifecycleEventObserver { _, event ->
            if (event == androidx.lifecycle.Lifecycle.Event.ON_RESUME) {
                if (resumedOnce.value) vm.refresh() else resumedOnce.value = true
            }
        }
        lifecycleOwner.lifecycle.addObserver(obs)
        onDispose { lifecycleOwner.lifecycle.removeObserver(obs) }
    }

    LaunchedEffect(locPermDeniedTick.intValue) {
        if (locPermDeniedTick.intValue > 0) {
            snackbar.showSnackbar(permMsg)
        }
    }
    LaunchedEffect(state.claimedNotice) {
        if (state.claimedNotice) {
            snackbar.showSnackbar(claimedMsg)
            vm.clearClaimedNotice()
        }
    }
    LaunchedEffect(state.actionError) {
        state.actionError?.let {
            snackbar.showSnackbar(it)
            vm.clearActionError()
        }
    }
    LaunchedEffect(state.error, state.requests.isEmpty(), state.schedule.isEmpty()) {
        val err = state.error
        if (err != null && (state.requests.isNotEmpty() || state.schedule.isNotEmpty())) {
            snackbar.showSnackbar(err)
            vm.clearError()
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.drive_hub_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
                actions = {
                    if (!state.accessDenied) {
                        IconButton(onClick = { nav.navigate(AppRoute.DriverSettings) }) {
                            Icon(Icons.Outlined.Settings, contentDescription = stringResource(R.string.driver_settings_title))
                        }
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        if (state.accessDenied) {
            EmptyView(
                text = stringResource(R.string.role_driver_access_required),
                icon = Icons.Outlined.DirectionsCar,
                modifier = Modifier.padding(padding),
            )
        } else Column(Modifier.fillMaxSize().padding(padding)) {
            PrimaryTabRow(selectedTabIndex = state.tab) {
                Tab(
                    selected = state.tab == 0,
                    onClick = { UiHaptics.tick(view); vm.setTab(0) },
                    text = { Text("${stringResource(R.string.drive_tab_requests)} (${state.requests.size})") },
                )
                Tab(
                    selected = state.tab == 1,
                    onClick = { UiHaptics.tick(view); vm.setTab(1) },
                    text = { Text(stringResource(R.string.drive_tab_schedule)) },
                )
                Tab(
                    selected = state.tab == 2,
                    onClick = { UiHaptics.tick(view); vm.setTab(2) },
                    text = {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                        ) {
                            Text(stringResource(R.string.drive_tab_route))
                            if (state.route?.live == true) {
                                Box(
                                    Modifier
                                        .size(6.dp)
                                        .background(LocalStatusColors.current.success, CircleShape),
                                )
                            }
                        }
                    },
                )
            }
            AnimatedOfflineBanner(
                visible = state.offline,
                modifier = Modifier.padding(horizontal = Spacing.sm, vertical = Spacing.xxs),
            )
            when {
                state.loading -> SkeletonLoadingList(count = 4)
                state.error != null && state.requests.isEmpty() && state.schedule.isEmpty() ->
                    ErrorView(state.error ?: "", onRetry = vm::refresh)
                else -> PullToRefreshBox(
                    isRefreshing = state.refreshing,
                    onRefresh = vm::refresh,
                    modifier = Modifier.weight(1f),
                ) {
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
                        label = "drive_tab_content",
                    ) { tab ->
                        when (tab) {
                            0 -> RequestsTab(
                                requests = state.requests,
                                mineAreasOnly = state.mineAreasOnly,
                                busyRideId = state.busyRideId,
                                canClaim = me?.can("drive", "create") == true,
                                onToggleFilter = { UiHaptics.tick(view); vm.toggleMineAreas() },
                                onClaim = { rideId -> UiHaptics.confirm(view); vm.claim(rideId) },
                                onOpen = { rideId -> nav.navigate(AppRoute.RideDetail(rideId)) },
                            )
                            1 -> ScheduleTab(
                                schedule = state.schedule,
                                photoUrlFor = container::fileUrl,
                                onOpen = { rideId -> nav.navigate(AppRoute.RideDetail(rideId)) },
                            )
                            else -> RouteTab(
                                route = state.route,
                                sharingLocation = sharingLocation,
                                nearDestination = state.route != null &&
                                    state.route!!.live &&
                                    nearDestinationTripId == state.route!!.tripId,
                                onToggleLiveSharing = {
                                    if (sharingLocation) {
                                        LiveTripLocationService.stop(context)
                                    } else {
                                        val liveRoute = state.route?.takeIf { it.live }
                                        if (liveRoute != null) {
                                            if (LiveTripLocationService.hasLocationPermission(context)) {
                                                LiveTripLocationService.start(
                                                    context,
                                                    rideId = liveRoute.stops.firstOrNull()?.rideId ?: -1L,
                                                    tripId = liveRoute.tripId,
                                                )
                                            } else {
                                                locPermLauncher.launch(
                                                    arrayOf(
                                                        Manifest.permission.ACCESS_FINE_LOCATION,
                                                        Manifest.permission.ACCESS_COARSE_LOCATION,
                                                    ),
                                                )
                                            }
                                        }
                                    }
                                },
                                onOpenStop = { rideId -> nav.navigate(AppRoute.RideDetail(rideId)) },
                                onNavigate = smartNavigate,
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun RequestsTab(
    requests: List<Ride>,
    mineAreasOnly: Boolean,
    busyRideId: Long?,
    canClaim: Boolean,
    onToggleFilter: () -> Unit,
    onClaim: (Long) -> Unit,
    onOpen: (Long) -> Unit,
) {
    LazyColumn(
        Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        contentPadding = PaddingValues(vertical = Spacing.xs),
    ) {
        item {
            FilterChip(
                selected = mineAreasOnly,
                onClick = onToggleFilter,
                label = { Text(stringResource(R.string.drive_filter_my_areas)) },
            )
        }
        if (requests.isEmpty()) {
            item {
                EmptyView(
                    text = stringResource(R.string.drive_requests_empty),
                    icon = Icons.Outlined.DirectionsCar,
                )
            }
        }
        items(requests, key = { it.id }) { ride ->
            OutlinedCard(
                onClick = { onOpen(ride.id) },
                modifier = Modifier.fillMaxWidth().animateItem(),
            ) {
                Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                        Text(
                            KlTime.display(ride.pickupAt),
                            style = MaterialTheme.typography.titleSmall.tabular(),
                            color = MaterialTheme.colorScheme.primary,
                        )
                        Text(
                            "${stringResource(R.string.ride_seats)}: ${ride.seats}",
                            style = MaterialTheme.typography.labelMedium.tabular(),
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    Text("${ride.area.name} → ${ride.destination}", style = MaterialTheme.typography.titleMedium)
                    Text(ride.passenger.name, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    if (ride.assistance.isNotBlank()) {
                        val assist = Assistance.labelRes(ride.assistance)?.let { stringResource(it) } ?: ride.assistance
                        Text(assist, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.primary)
                    }
                    if (canClaim) {
                        Button(
                            onClick = { onClaim(ride.id) },
                            enabled = busyRideId == null,
                            modifier = Modifier.fillMaxWidth().height(48.dp),
                        ) {
                            if (busyRideId == ride.id) {
                                InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                            } else {
                                Text(stringResource(R.string.drive_claim))
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun ScheduleTab(
    schedule: List<Ride>,
    photoUrlFor: (Long) -> String,
    onOpen: (Long) -> Unit,
) {
    LazyColumn(
        Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        contentPadding = PaddingValues(vertical = Spacing.xs),
    ) {
        if (schedule.isEmpty()) {
            item {
                EmptyView(
                    text = stringResource(R.string.drive_schedule_empty),
                    icon = Icons.Outlined.Schedule,
                )
            }
        }
        items(schedule, key = { it.id }) { ride ->
            RideCard(
                ride = ride,
                onClick = { onOpen(ride.id) },
                photoUrlFor = photoUrlFor,
                modifier = Modifier.animateItem(),
            )
        }
    }
}

@Composable
private fun RouteTab(
    route: TripRouteDto?,
    sharingLocation: Boolean,
    nearDestination: Boolean,
    onToggleLiveSharing: () -> Unit,
    onOpenStop: (Long) -> Unit,
    onNavigate: (Double?, Double?, String) -> Unit,
) {
    val c = LocalStatusColors.current
    val driverLabel = stringResource(R.string.map_driver_label)
    LazyColumn(
        Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        contentPadding = PaddingValues(vertical = Spacing.xs),
    ) {
        if (route == null) {
            item {
                EmptyView(
                    text = stringResource(R.string.drive_route_empty),
                    icon = Icons.Outlined.Route,
                )
            }
            return@LazyColumn
        }

        val mapPoints = buildList {
            route.origin?.let { origin ->
                add(
                    LiveMapPoint(
                        id = "origin",
                        lat = origin.lat,
                        lng = origin.lng,
                        label = driverLabel,
                        kind = MapMarkerKind.CAR,
                        stale = !sharingLocation && !route.live,
                    ),
                )
            }
            route.stops.forEachIndexed { idx, stop ->
                if (stop.lat != null && stop.lng != null) {
                    add(
                        LiveMapPoint(
                            id = "stop-${stop.rideId}",
                            lat = stop.lat,
                            lng = stop.lng,
                            label = stop.passenger,
                            badge = "${idx + 1}",
                            kind = if (stop.location == "live") MapMarkerKind.PERSON else MapMarkerKind.STOP,
                        ),
                    )
                }
            }
            val destLat = route.destination.lat
            val destLng = route.destination.lng
            if (destLat != null && destLng != null) {
                add(
                    LiveMapPoint(
                        id = "dest",
                        lat = destLat,
                        lng = destLng,
                        label = route.destination.name,
                        kind = MapMarkerKind.DEST,
                    ),
                )
            }
        }
        if (mapPoints.isNotEmpty()) {
            item {
                GraceRideLiveMap(
                    points = mapPoints,
                    route = route.geometry,
                    modifier = Modifier.fillMaxWidth().height(210.dp),
                )
            }
        }

        if (nearDestination) {
            item {
                CalloutBanner(
                    icon = Icons.Outlined.Flag,
                    text = stringResource(R.string.drive_dest_nearby_banner),
                    containerColor = c.successSoft,
                    contentColor = c.success,
                )
            }
        }

        if (route.live) {
            item {
                CalloutBanner(
                    icon = Icons.Outlined.MyLocation,
                    text = stringResource(
                        if (sharingLocation) R.string.live_location_driver_active else R.string.live_location_driver_idle,
                    ),
                    containerColor = if (sharingLocation) c.successSoft else c.infoSoft,
                    contentColor = if (sharingLocation) c.success else c.info,
                )
            }
            item {
                Button(
                    onClick = onToggleLiveSharing,
                    colors = if (sharingLocation) {
                        ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error)
                    } else {
                        graceRidePrimaryButtonColors()
                    },
                    modifier = Modifier.fillMaxWidth().height(48.dp),
                ) {
                    Icon(Icons.Outlined.MyLocation, contentDescription = null, modifier = Modifier.size(18.dp))
                    Spacer(Modifier.width(Spacing.xs))
                    Text(
                        stringResource(
                            if (sharingLocation) R.string.live_location_stop else R.string.live_location_start,
                        ),
                    )
                }
            }
        }

        item {
            OutlinedCard(
                modifier = Modifier.fillMaxWidth(),
                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
            ) {
                Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text(
                        stringResource(R.string.drive_route_destination, route.destination.name),
                        style = MaterialTheme.typography.titleSmall,
                    )
                    Row(horizontalArrangement = Arrangement.spacedBy(Spacing.sm)) {
                        Text(
                            stringResource(R.string.drive_route_onboard, route.onboard),
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        route.destination.eta?.let { eta ->
                            Text(
                                "ETA ${KlTime.time(eta)}" + if (route.approximate) " ${stringResource(R.string.drive_route_approx)}" else "",
                                style = MaterialTheme.typography.bodyMedium.tabular(),
                                color = MaterialTheme.colorScheme.primary,
                            )
                        }
                    }
                    OutlinedButton(
                        onClick = { onNavigate(route.destination.lat, route.destination.lng, route.destination.name) },
                        modifier = Modifier.fillMaxWidth().padding(top = Spacing.xxs),
                    ) {
                        Icon(Icons.Outlined.Navigation, contentDescription = null, modifier = Modifier.size(16.dp))
                        Spacer(Modifier.width(Spacing.xs))
                        Text(stringResource(R.string.drive_route_navigate))
                    }
                }
            }
        }

        itemsIndexed(route.stops, key = { _, it -> it.rideId }) { index, stop ->
            OutlinedCard(
                onClick = { onOpenStop(stop.rideId) },
                modifier = Modifier.fillMaxWidth(),
                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
            ) {
                Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Row(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                            modifier = Modifier.weight(1f),
                        ) {
                            Surface(
                                color = MaterialTheme.colorScheme.primaryContainer,
                                contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                                shape = CircleShape,
                                modifier = Modifier.size(22.dp),
                            ) {
                                Box(contentAlignment = Alignment.Center) {
                                    Text(
                                        "${index + 1}",
                                        style = MaterialTheme.typography.labelSmall.copy(fontWeight = FontWeight.Bold).tabular(),
                                    )
                                }
                            }
                            Text(stop.passenger, style = MaterialTheme.typography.titleMedium)
                        }
                        Text(
                            stop.eta?.let { KlTime.time(it) } ?: KlTime.time(stop.pickupAt),
                            style = MaterialTheme.typography.labelLarge.tabular(),
                            color = MaterialTheme.colorScheme.primary,
                        )
                    }
                    Text(stop.address, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    if (stop.location == "live") {
                        Surface(
                            color = c.successSoft,
                            contentColor = c.success,
                            shape = CircleShape,
                        ) {
                            Text(
                                stringResource(R.string.live_location_spot_shared),
                                style = MaterialTheme.typography.labelSmall,
                                modifier = Modifier.padding(horizontal = Spacing.xs, vertical = 2.dp),
                            )
                        }
                    }
                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                        val seatsLabel = "${stringResource(R.string.ride_seats)}: ${stop.seats}"
                        val distLabel = stop.distanceM?.let { m ->
                            if (m >= 1000) String.format(java.util.Locale.US, "%.1f km", m / 1000.0) else "${m.toInt()} m"
                        } ?: seatsLabel
                        Text(distLabel, style = MaterialTheme.typography.bodySmall.tabular(), color = MaterialTheme.colorScheme.onSurfaceVariant)
                        OutlinedButton(onClick = { onNavigate(stop.lat, stop.lng, stop.address) }) {
                            Icon(Icons.Outlined.Navigation, contentDescription = null, modifier = Modifier.size(16.dp))
                            Spacer(Modifier.width(4.dp))
                            Text(stringResource(R.string.drive_route_navigate))
                        }
                    }
                }
            }
        }
    }
}
