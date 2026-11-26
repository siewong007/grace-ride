package org.pjstatecm.graceride.ui.screens.rides

import android.Manifest
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.snap
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
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.verticalScroll
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import coil.compose.AsyncImage
import coil.request.ImageRequest
import androidx.compose.material.icons.outlined.CloudOff
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.Share
import androidx.compose.material.icons.outlined.Shield
import androidx.compose.material.icons.outlined.Sync
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.rememberModalBottomSheetState
import org.pjstatecm.graceride.data.api.AreaPhotoDto
import org.pjstatecm.graceride.ui.copyToClipboard
import org.pjstatecm.graceride.ui.shareText
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.Accessible
import androidx.compose.material.icons.automirrored.outlined.HelpOutline
import androidx.compose.material.icons.outlined.AirlineSeatReclineNormal
import androidx.compose.material.icons.outlined.Call
import androidx.compose.material.icons.outlined.Cancel
import androidx.compose.material.icons.outlined.ChatBubbleOutline
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material.icons.outlined.CheckCircle
import androidx.compose.material.icons.outlined.ChevronRight
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.DoneAll
import androidx.compose.material.icons.outlined.Event
import androidx.compose.material.icons.outlined.Flag
import androidx.compose.material.icons.outlined.Map
import androidx.compose.material.icons.outlined.Person
import androidx.compose.material.icons.outlined.Place
import androidx.compose.material.icons.outlined.PlayArrow
import androidx.compose.material.icons.outlined.Schedule
import androidx.compose.material.icons.outlined.WarningAmber
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.repeatOnLifecycle
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.Assistance
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.ProxyRelationships
import org.pjstatecm.graceride.domain.Readiness
import org.pjstatecm.graceride.domain.RideStatus
import org.pjstatecm.graceride.service.LiveTripLocationService
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.DetailSkeleton
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.MemberAvatar
import org.pjstatecm.graceride.ui.components.RideStatusPill
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.components.animatorsEnabled
import org.pjstatecm.graceride.ui.dialPhone
import org.pjstatecm.graceride.ui.openInMaps
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun RideDetailScreen(
    container: AppContainer,
    nav: NavHostController,
    id: Long,
    justBooked: Boolean = false,
) {
    val vm: RideDetailViewModel = viewModel(
        key = "ride-$id",
        factory = RideDetailViewModel.factory(container, id),
    )
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val me = (authState as? org.pjstatecm.graceride.data.repo.AuthState.SignedIn)?.me ?: container.auth.me
    val context = LocalContext.current
    val view = LocalView.current
    val snackbar = remember { SnackbarHostState() }
    val scope = rememberCoroutineScope()
    var confirmCancel by remember { mutableStateOf(false) }
    var cancelReason by remember { mutableStateOf("") }
    var confirmWithdraw by remember { mutableStateOf(false) }
    var withdrawReason by remember { mutableStateOf("") }
    var confirmDecline by remember { mutableStateOf(false) }
    var confirmNoShow by remember { mutableStateOf(false) }
    var unableNotes by remember { mutableStateOf("") }
    val staleMsg = stringResource(R.string.error_ride_changed)
    val bookedMsg = stringResource(R.string.ride_new_success)
    val queuedMsg = stringResource(R.string.offline_queue_saved)
    val lifecycleOwner = LocalLifecycleOwner.current
    var resumedOnce by remember { mutableStateOf(false) }

    DisposableEffect(lifecycleOwner) {
        val obs = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) {
                if (resumedOnce) vm.load() else resumedOnce = true
            }
        }
        lifecycleOwner.lifecycle.addObserver(obs)
        onDispose { lifecycleOwner.lifecycle.removeObserver(obs) }
    }

    LaunchedEffect(justBooked) {
        if (justBooked) {
            snackbar.showSnackbar(bookedMsg, duration = SnackbarDuration.Long)
        }
    }

    LaunchedEffect(state.message, state.staleReload) {
        if (state.staleReload) {
            snackbar.showSnackbar(staleMsg)
            vm.clearMessage()
        } else if (state.message != null) {
            snackbar.showSnackbar(state.message!!)
            vm.clearMessage()
        }
    }

    LaunchedEffect(state.queuedOfflineNotice) {
        if (state.queuedOfflineNotice) {
            snackbar.showSnackbar(queuedMsg)
            vm.clearQueuedNotice()
        }
    }

    LaunchedEffect(state.ride?.area?.id) {
        state.ride?.area?.id?.let { vm.loadAreaPhotos(it) }
    }

    var showChat by remember { mutableStateOf(false) }
    var showCareSheet by remember { mutableStateOf(false) }
    var zoomedAreaPhoto by remember { mutableStateOf<AreaPhotoDto?>(null) }
    var confirmUnable by remember { mutableStateOf(false) }
    var showStartTripLocDialog by remember { mutableStateOf(false) }
    val startTripPermLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions(),
    ) { granted ->
        val hasLoc = granted[Manifest.permission.ACCESS_FINE_LOCATION] == true ||
            granted[Manifest.permission.ACCESS_COARSE_LOCATION] == true
        vm.startRide { startedRide ->
            if (hasLoc) {
                LiveTripLocationService.start(
                    context,
                    rideId = startedRide.id,
                    tripId = startedRide.tripId ?: -1L,
                )
            }
        }
    }
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.ride_detail_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        var lastRide by remember { mutableStateOf(state.ride) }
        if (state.ride != null) lastRide = state.ride
        val viewMode = when {
            state.loading -> "loading"
            state.ride == null -> "error"
            else -> "content"
        }
        Crossfade(
            targetState = viewMode,
            animationSpec = if (animatorsEnabled()) tween(200) else snap(),
            label = "ride_detail_state",
            modifier = Modifier.padding(padding),
        ) { mode ->
            when (mode) {
                "loading" -> DetailSkeleton()
                "error" -> ErrorView(state.error ?: "", onRetry = vm::load)
                else -> {
                    val ride = state.ride ?: lastRide ?: return@Crossfade
                    val c = LocalStatusColors.current
                    PullToRefreshBox(
                        isRefreshing = state.refreshing,
                        onRefresh = vm::refresh,
                        modifier = Modifier.fillMaxSize(),
                    ) {
                        Column(
                            Modifier
                                .fillMaxSize()
                                .verticalScroll(rememberScrollState())
                                .padding(Spacing.sm),
                            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                        ) {
                            if (state.queuedOffline.isNotEmpty()) {
                                Surface(
                                    color = c.warningSoft,
                                    shape = RoundedCornerShape(12.dp),
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Row(
                                        Modifier
                                            .fillMaxWidth()
                                            .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                                        verticalAlignment = Alignment.CenterVertically,
                                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    ) {
                                        Icon(Icons.Outlined.CloudOff, contentDescription = null, tint = c.warning)
                                        Text(
                                            stringResource(R.string.offline_queue_banner, state.queuedOffline.size),
                                            style = MaterialTheme.typography.bodySmall,
                                            color = c.warning,
                                            modifier = Modifier.weight(1f),
                                        )
                                        TextButton(
                                            onClick = vm::syncOfflineQueue,
                                            enabled = !state.offlineSyncing,
                                        ) {
                                            Icon(Icons.Outlined.Sync, contentDescription = null, modifier = Modifier.size(16.dp))
                                            Spacer(Modifier.width(Spacing.xxs))
                                            Text(stringResource(R.string.offline_queue_sync_now))
                                        }
                                    }
                                }
                            }
                            // Hero header
                            Row(
                                Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Text(
                                    KlTime.display(ride.pickupAt),
                                    style = MaterialTheme.typography.titleLarge.tabular(),
                                    color = MaterialTheme.colorScheme.onSurface,
                                    modifier = Modifier.weight(1f),
                                )
                                RideStatusPill(ride.status)
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
                                        modifier = Modifier.padding(horizontal = Spacing.xs, vertical = Spacing.xxs),
                                    )
                                }
                            }

                            if (ride.status !in setOf("cancelled", "no_show", "alt_transport")) {
                                RideProgressStepper(status = ride.status)
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

                            val canViewLiveTop = ride.viewer.isPassenger || ride.viewer.isDriver || ride.viewer.isStaff
                            if (ride.status == "in_progress" && canViewLiveTop) {
                                LiveTrackCard(
                                    container = container,
                                    ride = ride,
                                    location = state.liveLocation,
                                    sharingPassengerPin = state.sharingPassengerPin,
                                    riderLocationAutoStopped = state.riderLocationAutoStopped,
                                    onRefreshLocation = vm::refreshLiveLocation,
                                    onSharePassengerSpot = vm::sharePassengerSpot,
                                    onStopPassengerSpot = vm::stopPassengerSpot,
                                )
                            }

                            // 1. Route card
                            SectionHeader(stringResource(R.string.ride_section_route))
                            OutlinedCard(Modifier.fillMaxWidth()) {
                                Column(
                                    Modifier.padding(Spacing.sm),
                                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                ) {
                                    val pickupLabel = ride.area.name + (ride.pickupAddress?.let { " — $it" } ?: "")
                                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                        Column(
                                            horizontalAlignment = Alignment.CenterHorizontally,
                                            modifier = Modifier.padding(top = 8.dp).width(18.dp),
                                        ) {
                                            Box(
                                                Modifier
                                                    .size(10.dp)
                                                    .clip(CircleShape)
                                                    .background(MaterialTheme.colorScheme.primary),
                                            )
                                            Box(
                                                Modifier
                                                    .padding(vertical = 4.dp)
                                                    .width(2.dp)
                                                    .height(26.dp)
                                                    .background(MaterialTheme.colorScheme.outlineVariant),
                                            )
                                            Icon(
                                                Icons.Outlined.Place,
                                                contentDescription = null,
                                                tint = MaterialTheme.colorScheme.primary,
                                                modifier = Modifier.size(18.dp),
                                            )
                                        }
                                        Column(
                                            Modifier.weight(1f),
                                            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                                        ) {
                                            InfoRow(stringResource(R.string.ride_pickup), pickupLabel)
                                            InfoRow(stringResource(R.string.ride_destination), ride.destination)
                                        }
                                    }
                                    val mapQuery = ride.pickupAddress?.takeIf {
                                        !ride.viewer.isPassenger &&
                                            ride.pickedUpAt == null &&
                                            !org.pjstatecm.graceride.domain.RideStatus.isTerminal(ride.status) &&
                                            it.isNotBlank()
                                    }
                                        ?: ride.destination.takeIf { it.isNotBlank() }
                                        ?: ride.pickupAddress?.takeIf { it.isNotBlank() }
                                    if (mapQuery != null) {
                                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                        OutlinedButton(
                                            onClick = { openInMaps(context, mapQuery) },
                                            modifier = Modifier.fillMaxWidth(),
                                        ) {
                                            Icon(Icons.Outlined.Map, contentDescription = null, modifier = Modifier.size(18.dp))
                                            Spacer(Modifier.width(Spacing.xs))
                                            Text(stringResource(R.string.common_open_maps))
                                        }
                                    }
                                }
                            }

                            // 2. People & vehicle card
                            if (ride.driver != null || !ride.viewer.isPassenger || (ride.isProxy && ride.proxy != null)) {
                                SectionHeader(stringResource(R.string.ride_section_people))
                                OutlinedCard(Modifier.fillMaxWidth()) {
                                    Column(
                                        Modifier.padding(Spacing.sm),
                                        verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                                    ) {
                                        ride.driver?.let { d ->
                                            Row(
                                                Modifier.fillMaxWidth(),
                                                verticalAlignment = Alignment.CenterVertically,
                                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                            ) {
                                                MemberAvatar(
                                                    name = d.name ?: "",
                                                    photoUrl = d.photoFileId?.let(container::fileUrl),
                                                    avatarColor = d.avatarColor,
                                                    avatarIcon = d.avatarIcon,
                                                    size = 44.dp,
                                                )
                                                Column(Modifier.weight(1f)) {
                                                    Text(
                                                        stringResource(R.string.ride_driver),
                                                        style = MaterialTheme.typography.labelMedium,
                                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                    )
                                                    Text(
                                                        d.name?.takeIf { it.isNotBlank() } ?: "—",
                                                        style = MaterialTheme.typography.titleMedium,
                                                        color = MaterialTheme.colorScheme.onSurface,
                                                    )
                                                    d.vehicle?.let { v ->
                                                        val vSummary = listOfNotNull(
                                                            v.color?.takeIf { it.isNotBlank() },
                                                            v.model.takeIf { it.isNotBlank() },
                                                            v.plate?.takeIf { it.isNotBlank() },
                                                        ).joinToString(" · ")
                                                        if (vSummary.isNotBlank()) {
                                                            Text(
                                                                vSummary,
                                                                style = MaterialTheme.typography.bodyMedium,
                                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                            )
                                                        }
                                                    }
                                                }
                                                d.phone?.takeIf { it.isNotBlank() }?.let { phone ->
                                                    FilledTonalIconButton(onClick = { dialPhone(context, phone) }) {
                                                        Icon(
                                                            Icons.Outlined.Call,
                                                            contentDescription = stringResource(R.string.common_call),
                                                        )
                                                    }
                                                }
                                            }
                                        }

                                        if (!ride.viewer.isPassenger) {
                                            if (ride.driver != null) HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                            PersonPhoneRow(
                                                label = stringResource(R.string.ride_passenger),
                                                name = ride.passenger.name,
                                                phone = ride.passenger.phone,
                                                onCall = { phone -> dialPhone(context, phone) },
                                            )
                                        }

                                        if (ride.isProxy && ride.proxy != null) {
                                            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                            val relLabel = ride.proxy.riderRelationship.takeIf { it.isNotBlank() }?.let { rel ->
                                                ProxyRelationships.labelRes(rel)?.let { res -> stringResource(res) } ?: rel
                                            }
                                            val proxySummary = listOfNotNull(
                                                ride.proxy.riderName.takeIf { it.isNotBlank() },
                                                relLabel,
                                            ).joinToString(" · ")
                                            if (proxySummary.isNotBlank()) {
                                                PersonPhoneRow(
                                                    label = stringResource(R.string.ride_new_rider_name),
                                                    name = proxySummary,
                                                    phone = ride.proxy.riderPhone,
                                                    onCall = { phone -> dialPhone(context, phone) },
                                                )
                                            }
                                            if (!ride.viewer.isPassenger && ride.proxy.bookerName.isNotBlank()) {
                                                PersonPhoneRow(
                                                    label = stringResource(R.string.ride_proxy_booker, ride.proxy.bookerName),
                                                    name = ride.proxy.bookerName,
                                                    phone = ride.proxy.bookerPhone,
                                                    onCall = { phone -> dialPhone(context, phone) },
                                                )
                                            }
                                        }
                                    }
                                }
                            }

                            // 3. Trip details card
                            SectionHeader(stringResource(R.string.ride_section_details))
                            OutlinedCard(Modifier.fillMaxWidth()) {
                                Column(
                                    Modifier.padding(Spacing.sm),
                                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                ) {
                                    ride.event?.let { ev ->
                                        val evTitle = ev.title?.takeIf { it.isNotBlank() }
                                        if (evTitle != null) {
                                            val evId = ev.id
                                            Surface(
                                                onClick = { evId?.let { nav.navigate(AppRoute.EventDetail(it)) } },
                                                enabled = evId != null,
                                                color = MaterialTheme.colorScheme.surfaceContainerLow,
                                                shape = MaterialTheme.shapes.small,
                                                modifier = Modifier.fillMaxWidth(),
                                            ) {
                                                Row(
                                                    Modifier.padding(Spacing.xs),
                                                    verticalAlignment = Alignment.CenterVertically,
                                                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                                ) {
                                                    Icon(
                                                        Icons.Outlined.Event,
                                                        contentDescription = null,
                                                        tint = MaterialTheme.colorScheme.primary,
                                                        modifier = Modifier.size(20.dp),
                                                    )
                                                    Column(Modifier.weight(1f)) {
                                                        Text(
                                                            stringResource(R.string.ride_event),
                                                            style = MaterialTheme.typography.labelMedium,
                                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                        )
                                                        Text(
                                                            evTitle,
                                                            style = MaterialTheme.typography.bodyLarge,
                                                            color = MaterialTheme.colorScheme.onSurface,
                                                        )
                                                    }
                                                    if (evId != null) {
                                                        Icon(
                                                            Icons.Outlined.ChevronRight,
                                                            contentDescription = null,
                                                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                                                        )
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    FlowRow(
                                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    ) {
                                        Surface(
                                            color = MaterialTheme.colorScheme.surfaceVariant,
                                            shape = CircleShape,
                                        ) {
                                            Row(
                                                verticalAlignment = Alignment.CenterVertically,
                                                horizontalArrangement = Arrangement.spacedBy(6.dp),
                                                modifier = Modifier.padding(horizontal = Spacing.xs + 2.dp, vertical = Spacing.xxs + 2.dp),
                                            ) {
                                                Icon(
                                                    Icons.Outlined.AirlineSeatReclineNormal,
                                                    contentDescription = null,
                                                    modifier = Modifier.size(16.dp),
                                                )
                                                Text(
                                                    "${stringResource(R.string.ride_seats)}: ${ride.seats}",
                                                    style = MaterialTheme.typography.labelMedium.tabular(),
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
                                                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                                                    modifier = Modifier.padding(horizontal = Spacing.xs + 2.dp, vertical = Spacing.xxs + 2.dp),
                                                ) {
                                                    Icon(Icons.AutoMirrored.Outlined.Accessible, contentDescription = null, modifier = Modifier.size(16.dp))
                                                    Text(assistText, style = MaterialTheme.typography.labelMedium)
                                                }
                                            }
                                        }
                                    }
                                    if (ride.readinessStatus in Readiness.values) {
                                        InfoRow(
                                            stringResource(R.string.ride_readiness_status),
                                            stringResource(Readiness.labelRes(ride.readinessStatus)),
                                        )
                                    }
                                    if (ride.notes.isNotBlank()) {
                                        InfoRow(stringResource(R.string.ride_notes), ride.notes)
                                    }
                                }
                            }

                            if (state.areaPhotos.isNotEmpty()) {
                                SectionHeader(stringResource(R.string.ride_area_photos_title, ride.area.name))
                                LazyRow(
                                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    items(state.areaPhotos, key = { it.id }) { photo ->
                                        OutlinedCard(
                                            onClick = { zoomedAreaPhoto = photo },
                                            modifier = Modifier.width(148.dp),
                                        ) {
                                            Column {
                                                AsyncImage(
                                                    model = ImageRequest.Builder(context)
                                                        .data(container.fileUrl(photo.fileId))
                                                        .crossfade(true)
                                                        .build(),
                                                    contentDescription = photo.caption.ifBlank { ride.area.name },
                                                    contentScale = ContentScale.Crop,
                                                    modifier = Modifier
                                                        .fillMaxWidth()
                                                        .height(92.dp),
                                                )
                                                if (photo.caption.isNotBlank()) {
                                                    Text(
                                                        photo.caption,
                                                        style = MaterialTheme.typography.labelSmall,
                                                        maxLines = 2,
                                                        overflow = TextOverflow.Ellipsis,
                                                        modifier = Modifier.padding(Spacing.xs),
                                                    )
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if (ride.viewer.isPassenger || ride.viewer.isDriver || ride.viewer.isStaff) {
                                OutlinedButton(
                                    onClick = { showCareSheet = true },
                                    modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
                                ) {
                                    Icon(Icons.Outlined.Shield, contentDescription = null, modifier = Modifier.size(18.dp))
                                    Spacer(Modifier.width(Spacing.xs))
                                    Text(stringResource(R.string.care_link_btn))
                                }
                            }
                            if (me?.can("help") == true) {
                                OutlinedButton(
                                    onClick = { nav.navigate(AppRoute.Help(rideId = ride.id, tab = "tickets")) },
                                    modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
                                ) {
                                    Icon(Icons.AutoMirrored.Outlined.HelpOutline, contentDescription = null, modifier = Modifier.size(18.dp))
                                    Spacer(Modifier.width(Spacing.xs))
                                    Text(stringResource(R.string.ride_need_help))
                                }
                            }

                            ride.pairedRideId?.let { pairedId ->
                                OutlinedButton(
                                    onClick = { nav.navigate(AppRoute.RideDetail(pairedId)) },
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Text(stringResource(R.string.ride_view_paired))
                                }
                            }

                            // Passenger actions
                            val canRidesUpdate = me?.can("rides", "update") == true
                            val canRidesDelete = me?.can("rides", "delete") == true
                            val canDriveUpdate = me?.can("drive", "update") == true
                            val canDriveDelete = me?.can("drive", "delete") == true

                            if (ride.viewer.isPassenger && ride.status == "pending_passenger" && canRidesUpdate) {
                                Row(
                                    Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                ) {
                                    Button(
                                        onClick = {
                                            UiHaptics.confirm(view)
                                            vm.respond(true)
                                        },
                                        enabled = !state.busy,
                                        colors = graceRidePrimaryButtonColors(),
                                        modifier = Modifier.weight(1f).height(50.dp),
                                    ) {
                                        if (state.busy) {
                                            InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                        } else {
                                            Text(stringResource(R.string.ride_respond_accept))
                                        }
                                    }
                                    OutlinedButton(
                                        onClick = {
                                            UiHaptics.tick(view)
                                            confirmDecline = true
                                        },
                                        enabled = !state.busy,
                                        modifier = Modifier.weight(1f).height(50.dp),
                                    ) {
                                        Text(stringResource(R.string.ride_respond_decline))
                                    }
                                }
                            }
                            if (ride.viewer.isPassenger &&
                                canRidesUpdate &&
                                (ride.status == "confirmed" || ride.status == "in_progress") &&
                                ride.pickedUpAt == null
                            ) {
                                OutlinedCard(Modifier.fillMaxWidth()) {
                                    Column(
                                        Modifier.padding(Spacing.sm),
                                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    ) {
                                        Text(stringResource(R.string.ride_readiness), style = MaterialTheme.typography.titleSmall)
                                        Readiness.values.chunked(2).forEach { pair ->
                                            Row(
                                                Modifier.fillMaxWidth(),
                                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                            ) {
                                                pair.forEach { value ->
                                                    val selected = ride.readinessStatus == value
                                                    val icon = when (value) {
                                                        Readiness.READY -> Icons.Outlined.CheckCircle
                                                        Readiness.UNABLE -> Icons.Outlined.Cancel
                                                        else -> Icons.Outlined.Schedule
                                                    }
                                                    val onPick: () -> Unit = {
                                                        UiHaptics.tick(view)
                                                        if (value == Readiness.UNABLE) {
                                                            confirmUnable = true
                                                        } else {
                                                            vm.readiness(value)
                                                        }
                                                    }
                                                    if (selected) {
                                                        Button(
                                                            onClick = onPick,
                                                            enabled = !state.busy,
                                                            colors = graceRidePrimaryButtonColors(),
                                                            modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                                                        ) {
                                                            Icon(icon, contentDescription = null, modifier = Modifier.size(16.dp))
                                                            Spacer(Modifier.width(6.dp))
                                                            Text(
                                                                stringResource(Readiness.labelRes(value)),
                                                                maxLines = 1,
                                                                overflow = TextOverflow.Ellipsis,
                                                            )
                                                        }
                                                    } else {
                                                        OutlinedButton(
                                                            onClick = onPick,
                                                            enabled = !state.busy,
                                                            modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                                                        ) {
                                                            Icon(icon, contentDescription = null, modifier = Modifier.size(16.dp))
                                                            Spacer(Modifier.width(6.dp))
                                                            Text(
                                                                stringResource(Readiness.labelRes(value)),
                                                                maxLines = 1,
                                                                overflow = TextOverflow.Ellipsis,
                                                            )
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if (ride.viewer.isPassenger &&
                                canRidesUpdate &&
                                ride.leg == "return" &&
                                !RideStatus.isTerminal(ride.status)
                            ) {
                                OutlinedCard(Modifier.fillMaxWidth()) {
                                    Column(
                                        Modifier.padding(Spacing.sm),
                                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    ) {
                                        Text(stringResource(R.string.ride_supper_title), style = MaterialTheme.typography.titleSmall)
                                        Row(
                                            Modifier.fillMaxWidth(),
                                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                        ) {
                                            listOf(
                                                "supper" to R.string.ride_supper_join,
                                                "direct_home" to R.string.ride_supper_direct,
                                            ).forEach { (choice, labelRes) ->
                                                val selected = ride.supperChoice == choice
                                                val nextChoice = if (selected) "unspecified" else choice
                                                if (selected) {
                                                    Button(
                                                        onClick = { UiHaptics.tick(view); vm.supperChoice(nextChoice) },
                                                        enabled = !state.busy,
                                                        colors = graceRidePrimaryButtonColors(),
                                                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                                                    ) {
                                                        Text(stringResource(labelRes), maxLines = 1, overflow = TextOverflow.Ellipsis)
                                                    }
                                                } else {
                                                    OutlinedButton(
                                                        onClick = { UiHaptics.tick(view); vm.supperChoice(nextChoice) },
                                                        enabled = !state.busy,
                                                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                                                    ) {
                                                        Text(stringResource(labelRes), maxLines = 1, overflow = TextOverflow.Ellipsis)
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if (ride.viewer.isDriver &&
                                ride.leg == "return" &&
                                ride.supperChoice in setOf("supper", "direct_home")
                            ) {
                                CalloutBanner(
                                    icon = Icons.Outlined.Schedule,
                                    text = stringResource(
                                        if (ride.supperChoice == "supper") R.string.ride_supper_driver_join
                                        else R.string.ride_supper_driver_direct,
                                    ),
                                    containerColor = MaterialTheme.colorScheme.secondaryContainer,
                                    contentColor = MaterialTheme.colorScheme.onSecondaryContainer,
                                )
                            }
                            if (ride.viewer.isPassenger && canRidesDelete && RideStatus.cancellable(ride.status)) {
                                OutlinedButton(
                                    onClick = { confirmCancel = true },
                                    enabled = !state.busy,
                                    colors = ButtonDefaults.outlinedButtonColors(contentColor = MaterialTheme.colorScheme.error),
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Text(stringResource(R.string.ride_cancel))
                                }
                            }
                            if (ride.status == "pending_driver" &&
                                !ride.viewer.isPassenger &&
                                !ride.viewer.isDriver &&
                                me?.can("drive", "create") == true
                            ) {
                                Button(
                                    onClick = {
                                        UiHaptics.confirm(view)
                                        vm.claim()
                                    },
                                    enabled = !state.busy,
                                    colors = graceRidePrimaryButtonColors(),
                                    modifier = Modifier.fillMaxWidth().height(50.dp),
                                ) {
                                    if (state.busy) {
                                        InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                    } else {
                                        Text(stringResource(R.string.drive_claim))
                                    }
                                }
                            }

                            // Driver execution controls
                            if (ride.viewer.isDriver &&
                                (canDriveUpdate || canDriveDelete) &&
                                ride.status in setOf(
                                    "pending_driver_confirm",
                                    "confirmed",
                                    "in_progress",
                                    "pending_passenger",
                                    "pending_approval",
                                )
                            ) {
                                OutlinedCard(Modifier.fillMaxWidth()) {
                                    Column(
                                        Modifier.padding(Spacing.sm),
                                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    ) {
                                        Text(stringResource(R.string.driver_actions_title), style = MaterialTheme.typography.titleSmall)
                                        if (canDriveUpdate) {
                                            when (ride.status) {
                                            "pending_driver_confirm" -> {
                                                Button(
                                                    onClick = {
                                                        UiHaptics.confirm(view)
                                                        vm.confirmAssignment()
                                                    },
                                                    enabled = !state.busy,
                                                    colors = graceRidePrimaryButtonColors(),
                                                    modifier = Modifier.fillMaxWidth().height(52.dp),
                                                ) {
                                                    if (state.busy) {
                                                        InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                                    } else {
                                                        Icon(Icons.Outlined.Check, contentDescription = null, modifier = Modifier.size(18.dp))
                                                        Spacer(Modifier.width(Spacing.xs))
                                                        Text(stringResource(R.string.driver_confirm_assignment))
                                                    }
                                                }
                                            }
                                            "confirmed" -> {
                                                Button(
                                                    onClick = {
                                                        UiHaptics.confirm(view)
                                                        if (LiveTripLocationService.hasLocationPermission(context)) {
                                                            vm.startRide { startedRide ->
                                                                LiveTripLocationService.start(
                                                                    context,
                                                                    rideId = startedRide.id,
                                                                    tripId = startedRide.tripId ?: -1L,
                                                                )
                                                            }
                                                        } else {
                                                            showStartTripLocDialog = true
                                                        }
                                                    },
                                                    enabled = !state.busy,
                                                    colors = graceRidePrimaryButtonColors(),
                                                    modifier = Modifier.fillMaxWidth().height(52.dp),
                                                ) {
                                                    if (state.busy) {
                                                        InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                                    } else {
                                                        Icon(Icons.Outlined.PlayArrow, contentDescription = null, modifier = Modifier.size(20.dp))
                                                        Spacer(Modifier.width(Spacing.xs))
                                                        Text(stringResource(R.string.driver_start_trip))
                                                    }
                                                }
                                                FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                                    OutlinedButton(
                                                        onClick = { UiHaptics.tick(view); vm.driverDelay(5) },
                                                        enabled = !state.busy,
                                                    ) {
                                                        Icon(Icons.Outlined.Schedule, contentDescription = null, modifier = Modifier.size(16.dp))
                                                        Spacer(Modifier.width(4.dp))
                                                        Text(stringResource(R.string.driver_delay_5m))
                                                    }
                                                    OutlinedButton(
                                                        onClick = { UiHaptics.tick(view); vm.driverDelay(10) },
                                                        enabled = !state.busy,
                                                    ) {
                                                        Icon(Icons.Outlined.Schedule, contentDescription = null, modifier = Modifier.size(16.dp))
                                                        Spacer(Modifier.width(4.dp))
                                                        Text(stringResource(R.string.driver_delay_10m))
                                                    }
                                                }
                                            }
                                            "in_progress" -> {
                                                if (ride.pickedUpAt == null) {
                                                    Button(
                                                        onClick = {
                                                            UiHaptics.confirm(view)
                                                            vm.pickup()
                                                        },
                                                        enabled = !state.busy,
                                                        colors = graceRidePrimaryButtonColors(),
                                                        modifier = Modifier.fillMaxWidth().height(52.dp),
                                                    ) {
                                                        if (state.busy) {
                                                            InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                                        } else {
                                                            Icon(Icons.Outlined.Person, contentDescription = null, modifier = Modifier.size(18.dp))
                                                            Spacer(Modifier.width(Spacing.xs))
                                                            Text(stringResource(R.string.driver_picked_up))
                                                        }
                                                    }
                                                    FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                                        OutlinedButton(
                                                            onClick = { UiHaptics.confirm(view); vm.arrivedGate() },
                                                            enabled = !state.busy,
                                                        ) {
                                                            Icon(Icons.Outlined.Flag, contentDescription = null, modifier = Modifier.size(16.dp))
                                                            Spacer(Modifier.width(4.dp))
                                                            Text(stringResource(R.string.driver_arrived_gate))
                                                        }
                                                        OutlinedButton(
                                                            onClick = { UiHaptics.tick(view); vm.driverDelay(5) },
                                                            enabled = !state.busy,
                                                        ) {
                                                            Icon(Icons.Outlined.Schedule, contentDescription = null, modifier = Modifier.size(16.dp))
                                                            Spacer(Modifier.width(4.dp))
                                                            Text(stringResource(R.string.driver_delay_5m))
                                                        }
                                                        OutlinedButton(
                                                            onClick = { UiHaptics.tick(view); vm.driverDelay(10) },
                                                            enabled = !state.busy,
                                                        ) {
                                                            Icon(Icons.Outlined.Schedule, contentDescription = null, modifier = Modifier.size(16.dp))
                                                            Spacer(Modifier.width(4.dp))
                                                            Text(stringResource(R.string.driver_delay_10m))
                                                        }
                                                        val canMarkNoShow = ride.viewer.isStaff ||
                                                            KlTime.parse(ride.pickupAt)?.let { !it.isAfter(KlTime.now()) } == true
                                                        OutlinedButton(
                                                            onClick = { UiHaptics.tick(view); confirmNoShow = true },
                                                            enabled = !state.busy && canMarkNoShow,
                                                            colors = ButtonDefaults.outlinedButtonColors(contentColor = MaterialTheme.colorScheme.error),
                                                        ) {
                                                            Icon(Icons.Outlined.Cancel, contentDescription = null, modifier = Modifier.size(16.dp))
                                                            Spacer(Modifier.width(4.dp))
                                                            Text(stringResource(R.string.ride_status_no_show))
                                                        }
                                                        OutlinedButton(
                                                            onClick = { UiHaptics.confirm(view); vm.complete() },
                                                            enabled = !state.busy,
                                                        ) {
                                                            Icon(Icons.Outlined.DoneAll, contentDescription = null, modifier = Modifier.size(16.dp))
                                                            Spacer(Modifier.width(4.dp))
                                                            Text(stringResource(R.string.driver_complete_trip))
                                                        }
                                                    }
                                                } else {
                                                    Button(
                                                        onClick = {
                                                            UiHaptics.confirm(view)
                                                            vm.complete()
                                                        },
                                                        enabled = !state.busy,
                                                        colors = graceRidePrimaryButtonColors(),
                                                        modifier = Modifier.fillMaxWidth().height(52.dp),
                                                    ) {
                                                        if (state.busy) {
                                                            InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                                        } else {
                                                            Icon(Icons.Outlined.DoneAll, contentDescription = null, modifier = Modifier.size(18.dp))
                                                            Spacer(Modifier.width(Spacing.xs))
                                                            Text(stringResource(R.string.driver_complete_trip))
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        }
                                        if (canDriveDelete && ride.status in setOf("pending_passenger", "pending_approval", "pending_driver_confirm", "confirmed")) {
                                            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                            OutlinedButton(
                                                onClick = { confirmWithdraw = true },
                                                enabled = !state.busy,
                                                colors = ButtonDefaults.outlinedButtonColors(contentColor = MaterialTheme.colorScheme.error),
                                                modifier = Modifier.fillMaxWidth(),
                                            ) {
                                                Text(stringResource(R.string.driver_withdraw))
                                            }
                                        }
                                    }
                                }
                            } else if (ride.viewer.isStaff &&
                                canDriveUpdate &&
                                ride.status in setOf("confirmed", "in_progress") &&
                                ride.pickedUpAt == null
                            ) {
                                OutlinedButton(
                                    onClick = { UiHaptics.tick(view); confirmNoShow = true },
                                    enabled = !state.busy,
                                    colors = ButtonDefaults.outlinedButtonColors(contentColor = MaterialTheme.colorScheme.error),
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Icon(Icons.Outlined.Cancel, contentDescription = null, modifier = Modifier.size(16.dp))
                                    Spacer(Modifier.width(4.dp))
                                    Text(stringResource(R.string.ride_status_no_show))
                                }
                            }

                            val hasMyLockedDraft = state.drafts.any {
                                it.lockIsMine && it.status in setOf("draft", "returned")
                            }
                            LaunchedEffect(lifecycleOwner, hasMyLockedDraft) {
                                if (!hasMyLockedDraft) return@LaunchedEffect
                                lifecycleOwner.repeatOnLifecycle(Lifecycle.State.STARTED) {
                                    while (true) {
                                        delay(180_000L)
                                        vm.renewActiveRideDraftLocks()
                                    }
                                }
                            }
                            RideDetailStaffAndDraftsSection(
                                ride = ride,
                                me = me,
                                state = state,
                                vm = vm,
                                onOpenRide = { nextId -> nav.navigate(AppRoute.RideDetail(nextId)) },
                            )

                            val canViewLive = ride.viewer.isPassenger || ride.viewer.isDriver || ride.viewer.isStaff
                            val hasPrimaryCtaAbove = (ride.viewer.isPassenger && canRidesUpdate && (
                                ride.status == "pending_passenger" ||
                                    ((ride.status == "confirmed" || ride.status == "in_progress") && ride.pickedUpAt == null)
                                )) || (ride.viewer.isDriver && canDriveUpdate && ride.status in setOf("pending_driver_confirm", "confirmed", "in_progress"))
                            val canViewChat = ride.tripId != null && (
                                ride.viewer.isStaff ||
                                    ((ride.viewer.isPassenger || ride.viewer.isDriver) && ride.status !in setOf("cancelled", "no_show", "alt_transport"))
                                )
                            if (canViewChat) {
                                if (hasPrimaryCtaAbove) {
                                    OutlinedButton(
                                        onClick = {
                                            UiHaptics.tick(view)
                                            showChat = true
                                        },
                                        modifier = Modifier.fillMaxWidth().height(50.dp),
                                    ) {
                                        Icon(
                                            Icons.Outlined.ChatBubbleOutline,
                                            contentDescription = null,
                                            modifier = Modifier.padding(end = Spacing.xs),
                                        )
                                        Text(stringResource(R.string.chat_open_button))
                                    }
                                } else {
                                    Button(
                                        onClick = {
                                            UiHaptics.tick(view)
                                            showChat = true
                                        },
                                        colors = graceRidePrimaryButtonColors(),
                                        modifier = Modifier.fillMaxWidth().height(50.dp),
                                    ) {
                                        Icon(
                                            Icons.Outlined.ChatBubbleOutline,
                                            contentDescription = null,
                                            modifier = Modifier.padding(end = Spacing.xs),
                                        )
                                        Text(stringResource(R.string.chat_open_button))
                                    }
                                }
                            } else if (ride.viewer.isStaff && ride.status in setOf("confirmed", "in_progress") && canViewLive) {
                                OutlinedButton(
                                    onClick = { nav.navigate(AppRoute.TrafficHub(tab = "live")) },
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Icon(Icons.Outlined.Map, contentDescription = null, modifier = Modifier.padding(end = Spacing.xs))
                                    Text(stringResource(R.string.traffic_tab_live))
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if (showChat) {
        TripChatSheet(
            container = container,
            rideId = id,
            onDismiss = { showChat = false },
        )
    }

    if (zoomedAreaPhoto != null) {
        val photo = zoomedAreaPhoto!!
        AlertDialog(
            onDismissRequest = { zoomedAreaPhoto = null },
            title = { Text(photo.caption.ifBlank { state.ride?.area?.name ?: "" }) },
            text = {
                AsyncImage(
                    model = container.fileUrl(photo.fileId),
                    contentDescription = photo.caption,
                    contentScale = ContentScale.Fit,
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(300.dp)
                        .clip(RoundedCornerShape(12.dp)),
                )
            },
            confirmButton = {
                TextButton(onClick = { zoomedAreaPhoto = null }) {
                    Text(stringResource(R.string.common_close))
                }
            },
        )
    }

    if (showCareSheet) {
        val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
        var careLabel by remember { mutableStateOf("") }
        ModalBottomSheet(
            onDismissRequest = {
                showCareSheet = false
                vm.clearCareLinkResult()
            },
            sheetState = sheetState,
        ) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .padding(horizontal = Spacing.sm)
                    .padding(bottom = Spacing.md),
                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                Text(stringResource(R.string.care_link_title), style = MaterialTheme.typography.titleMedium)
                Text(
                    stringResource(R.string.care_link_desc),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                OutlinedTextField(
                    value = careLabel,
                    onValueChange = { careLabel = it.take(60) },
                    label = { Text(stringResource(R.string.care_link_label_field)) },
                    placeholder = { Text(stringResource(R.string.care_link_label_placeholder)) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                Button(
                    onClick = { vm.createCareLink(careLabel) },
                    enabled = !state.careBusy,
                    modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
                ) {
                    if (state.careBusy) {
                        InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                    } else {
                        Text(stringResource(R.string.care_link_generate))
                    }
                }
                state.careLink?.let { link ->
                    val fullUrl = container.webUrl(link.url.ifBlank { "/care/${link.token}" })
                    val copiedToast = stringResource(R.string.care_link_copied)
                    OutlinedCard(Modifier.fillMaxWidth()) {
                        Column(
                            Modifier.padding(Spacing.sm),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            Text(
                                fullUrl,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.primary,
                            )
                            Row(
                                Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                OutlinedButton(
                                    onClick = {
                                        copyToClipboard(context, "care-link", fullUrl)
                                        scope.launch { snackbar.showSnackbar(copiedToast) }
                                    },
                                    modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                                ) {
                                    Icon(Icons.Outlined.ContentCopy, contentDescription = null, modifier = Modifier.size(16.dp))
                                    Spacer(Modifier.width(Spacing.xxs))
                                    Text(stringResource(R.string.care_link_copy))
                                }
                                Button(
                                    onClick = { shareText(context, context.getString(R.string.care_link_share_text), fullUrl) },
                                    modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                                ) {
                                    Icon(Icons.Outlined.Share, contentDescription = null, modifier = Modifier.size(16.dp))
                                    Spacer(Modifier.width(Spacing.xxs))
                                    Text(stringResource(R.string.care_link_share))
                                }
                            }
                        }
                    }
                }
                TextButton(
                    onClick = {
                        vm.revokeCareLink()
                        showCareSheet = false
                    },
                    enabled = !state.careBusy,
                    modifier = Modifier.align(Alignment.End),
                ) {
                    Text(stringResource(R.string.care_link_revoke_old))
                }
            }
        }
    }

    if (confirmUnable) {
        AlertDialog(
            onDismissRequest = { confirmUnable = false; unableNotes = "" },
            title = { Text(stringResource(R.string.ride_unable_confirm_title)) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    Text(stringResource(R.string.ride_unable_confirm_body))
                    OutlinedTextField(
                        value = unableNotes,
                        onValueChange = { unableNotes = it },
                        label = { Text(stringResource(R.string.ride_unable_reason_hint)) },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        vm.readiness(Readiness.UNABLE, unableNotes.trim().ifBlank { null })
                        unableNotes = ""
                        confirmUnable = false
                    },
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                ) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = {
                TextButton(onClick = { confirmUnable = false; unableNotes = "" }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }

    if (confirmCancel) {
        AlertDialog(
            onDismissRequest = { confirmCancel = false; cancelReason = "" },
            title = { Text(stringResource(R.string.ride_cancel)) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    Text(stringResource(R.string.ride_cancel_confirm))
                    OutlinedTextField(
                        value = cancelReason,
                        onValueChange = { cancelReason = it },
                        label = { Text(stringResource(R.string.ride_cancel_reason_hint)) },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        vm.cancel(cancelReason.trim().ifBlank { null })
                        cancelReason = ""
                        confirmCancel = false
                    },
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                ) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = {
                TextButton(onClick = { confirmCancel = false; cancelReason = "" }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }

    if (confirmDecline) {
        AlertDialog(
            onDismissRequest = { confirmDecline = false },
            title = { Text(stringResource(R.string.ride_decline_confirm_title)) },
            text = { Text(stringResource(R.string.ride_decline_confirm_body)) },
            confirmButton = {
                TextButton(
                    onClick = { vm.respond(false); confirmDecline = false },
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                ) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = {
                TextButton(onClick = { confirmDecline = false }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }

    if (confirmNoShow) {
        AlertDialog(
            onDismissRequest = { confirmNoShow = false },
            title = { Text(stringResource(R.string.ride_noshow_confirm_title)) },
            text = { Text(stringResource(R.string.ride_noshow_confirm_body)) },
            confirmButton = {
                TextButton(
                    onClick = { vm.noShow(); confirmNoShow = false },
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                ) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = {
                TextButton(onClick = { confirmNoShow = false }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }

    if (confirmWithdraw) {
        AlertDialog(
            onDismissRequest = { confirmWithdraw = false },
            title = { Text(stringResource(R.string.driver_withdraw)) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    Text(stringResource(R.string.driver_withdraw_confirm))
                    OutlinedTextField(
                        value = withdrawReason,
                        onValueChange = { withdrawReason = it },
                        label = { Text(stringResource(R.string.driver_withdraw_reason)) },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        vm.withdraw(withdrawReason.trim().ifBlank { null })
                        withdrawReason = ""
                        confirmWithdraw = false
                    },
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                ) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = {
                TextButton(onClick = { confirmWithdraw = false }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }

    if (showStartTripLocDialog) {
        AlertDialog(
            onDismissRequest = { showStartTripLocDialog = false },
            title = { Text(stringResource(R.string.start_trip_loc_title)) },
            text = { Text(stringResource(R.string.start_trip_loc_body)) },
            confirmButton = {
                Button(
                    onClick = {
                        showStartTripLocDialog = false
                        startTripPermLauncher.launch(
                            arrayOf(
                                Manifest.permission.ACCESS_FINE_LOCATION,
                                Manifest.permission.ACCESS_COARSE_LOCATION,
                            ),
                        )
                    },
                    colors = graceRidePrimaryButtonColors(),
                ) {
                    Text(stringResource(R.string.start_trip_allow_and_start))
                }
            },
            dismissButton = {
                TextButton(
                    onClick = {
                        showStartTripLocDialog = false
                        vm.startRide()
                    },
                ) {
                    Text(stringResource(R.string.start_trip_without_loc))
                }
            },
        )
    }
}
