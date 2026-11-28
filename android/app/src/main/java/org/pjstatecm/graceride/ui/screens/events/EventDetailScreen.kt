package org.pjstatecm.graceride.ui.screens.events


import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.snap
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
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
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.AltRoute
import androidx.compose.material.icons.automirrored.outlined.OpenInNew
import androidx.compose.material.icons.outlined.CalendarToday
import androidx.compose.material.icons.outlined.CheckCircle
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Edit
import androidx.compose.material.icons.outlined.EventAvailable
import androidx.compose.material.icons.outlined.Map
import androidx.compose.material.icons.outlined.People
import androidx.compose.material.icons.outlined.PersonAdd
import androidx.compose.material.icons.outlined.Place
import androidx.compose.material.icons.outlined.QrCode2
import androidx.compose.material.icons.outlined.QrCodeScanner
import androidx.compose.material.icons.outlined.Share
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.rememberModalBottomSheetState
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
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.AdaptiveContentFrame
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.DetailSkeleton
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.EventIcon
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.ScheduleTimeline
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.components.animatorsEnabled
import org.pjstatecm.graceride.ui.copyToClipboard
import org.pjstatecm.graceride.ui.openInMaps
import org.pjstatecm.graceride.ui.shareText
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun EventDetailScreen(
    container: AppContainer,
    nav: NavHostController,
    id: Long,
    checkinCode: String? = null,
    openCheckin: Boolean = false,
) {
    org.pjstatecm.graceride.ui.SecureScreenEffect()
    val vm: EventDetailViewModel = viewModel(
        key = "event-$id",
        factory = EventDetailViewModel.factory(container, id),
    )
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val me = (authState as? AuthState.SignedIn)?.me ?: container.auth.me
    val context = LocalContext.current
    val view = LocalView.current
    val scope = rememberCoroutineScope()
    val snackbar = remember { SnackbarHostState() }
    val publishedNotice = stringResource(R.string.event_published_notice)
    val unpublishedNotice = stringResource(R.string.event_unpublished_notice)
    val copiedNotice = stringResource(R.string.common_copied)
    val calendarErrMsg = stringResource(R.string.event_calendar_unavailable)
    var showSignup by remember { mutableStateOf(false) }
    var showPassQr by remember { mutableStateOf(false) }
    var checkinOrganiserMode by remember { mutableStateOf<Boolean?>(null) }
    var prefillCheckinCode by remember { mutableStateOf("") }
    var deepCheckinHandled by remember(id, checkinCode, openCheckin) { mutableStateOf(false) }
    var confirmWithdraw by remember { mutableStateOf(false) }
    var confirmPublish by remember { mutableStateOf(false) }
    var confirmUnpublish by remember { mutableStateOf(false) }
    var confirmFinish by remember { mutableStateOf(false) }
    var confirmCancel by remember { mutableStateOf(false) }
    var showVenueQr by remember { mutableStateOf(false) }
    var walkInOpen by remember { mutableStateOf(false) }
    var walkInQuery by remember { mutableStateOf("") }
    var showDriveRequests by remember { mutableStateOf(false) }
    val finishDoneMsg = stringResource(R.string.event_finished_done)
    val cancelDoneMsg = stringResource(R.string.event_cancelled_done)
    val qrResetDoneMsg = stringResource(R.string.event_venue_qr_reset_done)
    val driveClaimedMsg = stringResource(R.string.drive_claimed_msg)
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
    LaunchedEffect(state.checkinResult, state.scanMemberResult) {
        state.checkinResult?.let { res ->
            val eventTitle = res.title.ifBlank { state.event?.title.orEmpty() }
            val msg = context.getString(
                if (res.alreadyAttended) R.string.event_checkin_already else R.string.event_checkin_success,
                eventTitle,
            )
            snackbar.showSnackbar(msg)
            checkinOrganiserMode = null
            prefillCheckinCode = ""
            vm.clearCheckinResults()
        }
        state.scanMemberResult?.let { res ->
            val name = res.memberName.ifBlank { "#${res.userId}" }
            val msg = context.getString(
                if (res.alreadyAttended) R.string.event_scan_member_already else R.string.event_scan_member_success,
                name,
            )
            snackbar.showSnackbar(msg)
            checkinOrganiserMode = null
            prefillCheckinCode = ""
            vm.clearCheckinResults()
        }
    }
    LaunchedEffect(state.event?.id, checkinCode, openCheckin) {
        val ev = state.event ?: return@LaunchedEffect
        val startDt = KlTime.parse(ev.startsAt)
        val endedByTime = startDt != null && KlTime.now().isAfter(startDt.plusHours(12))
        if (!deepCheckinHandled && (checkinCode != null || openCheckin)) {
            deepCheckinHandled = true
            if (ev.attendanceEnabled && ev.status == "published" && ev.mySignup?.attended != true && !endedByTime) {
                prefillCheckinCode = checkinCode?.trim().orEmpty()
                checkinOrganiserMode = false
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.events_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
                actions = {
                    state.event?.let { ev ->
                        IconButton(
                            onClick = {
                                UiHaptics.tick(view)
                                shareText(
                                    context = context,
                                    text = context.getString(
                                        R.string.event_share_text,
                                        ev.title,
                                        KlTime.display(ev.startsAt),
                                        ev.location,
                                    ),
                                    subject = ev.title,
                                )
                            },
                        ) {
                            Icon(
                                Icons.Outlined.Share,
                                contentDescription = stringResource(R.string.event_share_btn),
                            )
                        }
                        IconButton(
                            onClick = {
                                UiHaptics.tick(view)
                                val ok = org.pjstatecm.graceride.platform.addEventToCalendar(
                                    context = context,
                                    title = ev.title,
                                    startsAt = ev.startsAt,
                                    location = ev.location,
                                    description = ev.description,
                                )
                                if (!ok) {
                                    scope.launch { snackbar.showSnackbar(calendarErrMsg) }
                                }
                            },
                        ) {
                            Icon(
                                Icons.Outlined.EventAvailable,
                                contentDescription = stringResource(R.string.event_add_to_calendar),
                            )
                        }
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        var lastEvent by remember { mutableStateOf(state.event) }
        if (state.event != null) lastEvent = state.event
        val viewMode = when {
            state.loading -> "loading"
            state.event == null -> "error"
            else -> "content"
        }
        Crossfade(
            targetState = viewMode,
            animationSpec = if (animatorsEnabled()) tween(200) else snap(),
            label = "event_detail_state",
            modifier = Modifier.padding(padding),
        ) { mode ->
            when (mode) {
                "loading" -> DetailSkeleton()
                "error" -> ErrorView(state.error ?: "", onRetry = vm::load)
                else -> {
                    val event = state.event ?: lastEvent ?: return@Crossfade
                    // A withdrawn signup stays in the payload with status "cancelled";
                    // treat it as no signup so the member can sign up again.
                    val signup = event.mySignup?.takeIf { it.status == "going" || it.status == "waitlist" }
                    val nowKl = KlTime.nowStamp()
                    val startDt = KlTime.parse(event.startsAt)
                    val isPast = event.startsAt.take(16).replace('T', ' ') < nowKl
                    val isEndedByTime = startDt != null && KlTime.now().isAfter(startDt.plusHours(12))
                    val deadlinePassed = !event.signupDeadline.isNullOrBlank() &&
                        event.signupDeadline.take(16).replace('T', ' ') < nowKl
                    val isPublished = event.status == "published"
                    val open = isPublished && !isEndedByTime
                    val canSignUp = isPublished && !isPast && !deadlinePassed
                    val myRide = event.myRide
                    val isGoing = signup?.status == "going"
                    val hasTransport = myRide != null ||
                        event.myReturnRide != null ||
                        state.drivingForEvent.isNotEmpty() ||
                        signup?.selfTransport == true
                    val canCreateRide = me?.can("rides", "create") == true
                    val timelineNodes = buildEventWorkflowNodes(
                        event = event,
                        signup = signup,
                        outboundRide = state.outboundRide,
                        returnRide = state.returnRide,
                        drivingForEvent = state.drivingForEvent,
                        onOpenRide = { rideId -> nav.navigate(AppRoute.RideDetail(rideId)) },
                    )

                    PullToRefreshBox(
                        isRefreshing = state.refreshing,
                        onRefresh = vm::refresh,
                        modifier = Modifier.fillMaxSize(),
                    ) {
                        AdaptiveContentFrame(
                            modifier = Modifier.fillMaxSize(),
                            maxWidth = 840.dp,
                        ) {
                            Column(
                                Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(Spacing.sm),
                                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                            ) {
                            event.photoFileId?.let { fid ->
                                AsyncImage(
                                    model = container.fileUrl(fid),
                                    contentDescription = null,
                                    contentScale = ContentScale.Crop,
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .height(180.dp)
                                        .clip(MaterialTheme.shapes.medium),
                                )
                            }

                            // Hero Header
                            Row(
                                verticalAlignment = Alignment.Top,
                                horizontalArrangement = Arrangement.spacedBy(Spacing.sm),
                                modifier = Modifier.fillMaxWidth(),
                            ) {
                                Surface(
                                    color = MaterialTheme.colorScheme.surfaceVariant,
                                    shape = MaterialTheme.shapes.medium,
                                    modifier = Modifier.size(52.dp),
                                ) {
                                    Box(contentAlignment = Alignment.Center) {
                                        EventIcon(icon = event.icon, kind = event.kind, size = 34.dp)
                                    }
                                }
                                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
                                    Text(
                                        event.title,
                                        style = MaterialTheme.typography.headlineSmall,
                                        color = MaterialTheme.colorScheme.onSurface,
                                    )
                                    Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                        when (event.status) {
                                            "draft" -> StatusPill(stringResource(R.string.event_status_draft), RideStatusKind.PENDING)
                                            "cancelled" -> StatusPill(stringResource(R.string.event_status_cancelled), RideStatusKind.CANCELLED)
                                            "finished" -> StatusPill(stringResource(R.string.event_status_finished), RideStatusKind.DONE)
                                        }
                                        if (event.status != "cancelled") {
                                            signup?.let {
                                                StatusPill(
                                                    text = stringResource(if (it.status == "going") R.string.event_going else R.string.event_waitlist),
                                                    kind = if (it.status == "going") RideStatusKind.ACTIVE else RideStatusKind.PENDING,
                                                )
                                            }
                                        }
                                    }
                                }
                            }

                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                Icon(
                                    Icons.Outlined.CalendarToday,
                                    contentDescription = null,
                                    tint = MaterialTheme.colorScheme.primary,
                                    modifier = Modifier.size(18.dp),
                                )
                                Text(
                                    KlTime.display(event.startsAt),
                                    style = MaterialTheme.typography.titleMedium.tabular(),
                                    color = MaterialTheme.colorScheme.primary,
                                )
                            }

                            if (event.status == "draft") {
                                val c = LocalStatusColors.current
                                Surface(
                                    color = c.warningSoft,
                                    contentColor = c.warning,
                                    border = BorderStroke(1.dp, c.warning.copy(alpha = 0.4f)),
                                    shape = MaterialTheme.shapes.medium,
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Text(
                                        stringResource(R.string.event_draft_banner),
                                        style = MaterialTheme.typography.bodyMedium,
                                        modifier = Modifier.padding(Spacing.sm),
                                    )
                                }
                            }

                            // Check-in pass spotlight card
                            if (event.attendanceEnabled && isPublished) {
                                signup?.passCode?.takeIf { it.isNotBlank() }?.let { code ->
                                    OutlinedCard(
                                        border = BorderStroke(1.dp, MaterialTheme.colorScheme.primary.copy(alpha = 0.55f)),
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .clickable {
                                                UiHaptics.tick(view)
                                                showPassQr = true
                                            },
                                    ) {
                                        Row(
                                            Modifier.fillMaxWidth().padding(Spacing.sm),
                                            verticalAlignment = Alignment.CenterVertically,
                                            horizontalArrangement = Arrangement.spacedBy(Spacing.sm),
                                        ) {
                                            Icon(
                                                Icons.Outlined.QrCode2,
                                                contentDescription = null,
                                                tint = MaterialTheme.colorScheme.primary,
                                                modifier = Modifier.size(28.dp),
                                            )
                                            Column(Modifier.weight(1f)) {
                                                Text(
                                                    stringResource(R.string.event_pass_label),
                                                    style = MaterialTheme.typography.labelMedium,
                                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                )
                                                Text(
                                                    code,
                                                    style = MaterialTheme.typography.headlineMedium.tabular(),
                                                    fontFamily = FontFamily.Monospace,
                                                    fontWeight = FontWeight.Bold,
                                                    color = MaterialTheme.colorScheme.onSurface,
                                                )
                                            }
                                            FilledTonalButton(
                                                onClick = {
                                                    UiHaptics.tick(view)
                                                    if (copyToClipboard(context, "Check-in pass", code, sensitive = true)) {
                                                        scope.launch { snackbar.showSnackbar(copiedNotice) }
                                                    }
                                                },
                                            ) {
                                                Icon(Icons.Outlined.ContentCopy, contentDescription = null, modifier = Modifier.size(16.dp))
                                                Spacer(Modifier.width(4.dp))
                                                Text(stringResource(R.string.common_copy))
                                            }
                                        }
                                    }
                                }
                            }

                            OutlinedCard(Modifier.fillMaxWidth()) {
                                Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                    Text(
                                        stringResource(
                                            if (isGoing && state.drivingForEvent.isNotEmpty() && myRide == null) {
                                                R.string.event_you_are_driving
                                            } else {
                                                R.string.event_schedule_title
                                            },
                                        ),
                                        style = MaterialTheme.typography.labelLarge,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                    ScheduleTimeline(items = timelineNodes)
                                }
                            }

                            if (canSignUp && signup == null) {
                                Button(
                                    onClick = { showSignup = true },
                                    enabled = !state.busy,
                                    colors = graceRidePrimaryButtonColors(),
                                    modifier = Modifier.fillMaxWidth().height(52.dp),
                                ) {
                                    if (state.busy) {
                                        InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                    } else {
                                        Text(stringResource(R.string.event_signup))
                                    }
                                }
                            } else if (isPublished && (isPast || deadlinePassed) && signup == null && !isEndedByTime) {
                                Text(
                                    stringResource(R.string.event_signup_closed),
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            } else if (open && signup != null) {
                                OutlinedButton(
                                    onClick = { confirmWithdraw = true },
                                    enabled = !state.busy,
                                    colors = ButtonDefaults.outlinedButtonColors(contentColor = MaterialTheme.colorScheme.error),
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Text(stringResource(R.string.event_withdraw))
                                }
                            }

                            if (open && isGoing && signup?.selfTransport == true && myRide == null && event.myReturnRide == null && state.drivingForEvent.isEmpty()) {
                                CalloutBanner(
                                    icon = Icons.Outlined.CheckCircle,
                                    text = stringResource(R.string.event_going_own_way_status),
                                    containerColor = MaterialTheme.colorScheme.secondaryContainer,
                                    contentColor = MaterialTheme.colorScheme.onSecondaryContainer,
                                )
                                if (canCreateRide) {
                                    OutlinedButton(
                                        onClick = { UiHaptics.tick(view); vm.setTransport(false) },
                                        enabled = !state.busy,
                                        modifier = Modifier.fillMaxWidth(),
                                    ) {
                                        Text(stringResource(R.string.event_change_transport))
                                    }
                                }
                            }
                            if (open && isGoing && !hasTransport) {
                                OutlinedButton(
                                    onClick = { UiHaptics.tick(view); vm.setTransport(true) },
                                    enabled = !state.busy,
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Text(stringResource(R.string.event_going_own_way))
                                }
                            }
                            if (open && isGoing && canCreateRide) {
                                if (!hasTransport) {
                                    Button(
                                        onClick = { nav.navigate(AppRoute.RideNew(event.id)) },
                                        enabled = !state.busy,
                                        colors = graceRidePrimaryButtonColors(),
                                        modifier = Modifier.fillMaxWidth().height(52.dp),
                                    ) {
                                        Icon(Icons.Outlined.DirectionsCar, contentDescription = null, modifier = Modifier.size(18.dp))
                                        Spacer(Modifier.width(Spacing.xs))
                                        Text(stringResource(R.string.ride_request))
                                    }
                                } else {
                                    OutlinedButton(
                                        onClick = { nav.navigate(AppRoute.RideNew(event.id, proxy = true)) },
                                        enabled = !state.busy,
                                        modifier = Modifier.fillMaxWidth(),
                                    ) {
                                        Icon(Icons.Outlined.People, contentDescription = null, modifier = Modifier.padding(end = Spacing.xs))
                                        Text(stringResource(R.string.event_book_for_family))
                                    }
                                }
                            }

                            OutlinedCard(Modifier.fillMaxWidth()) {
                                Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                    if (event.location.isNotBlank()) {
                                        Row(
                                            Modifier.fillMaxWidth(),
                                            verticalAlignment = Alignment.CenterVertically,
                                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                        ) {
                                            Column(Modifier.weight(1f)) {
                                                InfoRow(stringResource(R.string.event_location), event.location)
                                            }
                                            IconButton(onClick = { openInMaps(context, event.location) }) {
                                                Icon(
                                                    Icons.Outlined.Map,
                                                    contentDescription = stringResource(R.string.common_open_maps),
                                                    tint = MaterialTheme.colorScheme.primary,
                                                )
                                            }
                                        }
                                    }
                                    if (event.capacity > 0) {
                                        val spots = event.spotsLeft
                                        val ratio = (event.going.toFloat() / event.capacity.toFloat()).coerceIn(0f, 1f)
                                        val c = LocalStatusColors.current
                                        Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                                            InfoRow(
                                                stringResource(R.string.event_going),
                                                event.going.toString() + " / " + event.capacity +
                                                    (spots?.let { " · " + stringResource(R.string.event_spots_left, it) } ?: ""),
                                            )
                                            LinearProgressIndicator(
                                                progress = { ratio },
                                                color = if (ratio >= 0.85f) c.warning else MaterialTheme.colorScheme.primary,
                                                trackColor = MaterialTheme.colorScheme.surfaceVariant,
                                                modifier = Modifier
                                                    .fillMaxWidth()
                                                    .height(6.dp)
                                                    .clip(CircleShape),
                                            )
                                        }
                                    }
                                    if (event.costNote.isNotBlank()) InfoRow(stringResource(R.string.event_cost), event.costNote)
                                    event.signupDeadline?.takeIf { it.isNotBlank() }?.let {
                                        InfoRow(stringResource(R.string.event_deadline), KlTime.display(it))
                                    }
                                    signup?.let { mine ->
                                        InfoRow(
                                            stringResource(R.string.event_my_status),
                                            stringResource(if (mine.status == "going") R.string.event_going else R.string.event_waitlist),
                                        )
                                    }
                                }
                            }

                            if (event.description.isNotBlank()) {
                                Text(event.description, style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurface)
                            }

                            if (event.status == "draft" && event.canPublish) {
                                Button(
                                    onClick = { confirmPublish = true },
                                    enabled = !state.busy,
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Text(stringResource(R.string.event_publish))
                                }
                            } else if (isPublished && event.canManage) {
                                OutlinedButton(
                                    onClick = { confirmUnpublish = true },
                                    enabled = event.canUnpublish && !state.busy,
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Text(stringResource(R.string.event_unpublish))
                                }
                                if (!event.canUnpublish) {
                                    Text(
                                        stringResource(R.string.event_unpublish_blocked),
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                            }

                            if (event.attendanceEnabled) {
                                if (event.mySignup?.attended == true) {
                                    Row(
                                        Modifier.fillMaxWidth(),
                                        verticalAlignment = Alignment.CenterVertically,
                                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    ) {
                                        Icon(
                                            Icons.Outlined.CheckCircle,
                                            contentDescription = null,
                                            tint = MaterialTheme.colorScheme.primary,
                                        )
                                        Text(
                                            stringResource(R.string.event_checkin_done),
                                            style = MaterialTheme.typography.titleSmall,
                                            color = MaterialTheme.colorScheme.primary,
                                        )
                                    }
                                } else if (isPublished && me?.status == "active" && !isEndedByTime) {
                                    Button(
                                        onClick = {
                                            UiHaptics.tick(view)
                                            vm.clearCheckinError()
                                            checkinOrganiserMode = false
                                        },
                                        enabled = !state.busy,
                                        modifier = Modifier.fillMaxWidth().height(50.dp),
                                    ) {
                                        Icon(Icons.Outlined.QrCodeScanner, contentDescription = null, modifier = Modifier.padding(end = Spacing.xs))
                                        Text(stringResource(R.string.event_checkin_btn))
                                    }
                                }
                                if (isPublished && event.canManage) {
                                    OutlinedButton(
                                        onClick = {
                                            UiHaptics.tick(view)
                                            vm.clearCheckinError()
                                            checkinOrganiserMode = true
                                        },
                                        enabled = !state.busy,
                                        modifier = Modifier.fillMaxWidth(),
                                    ) {
                                        Icon(Icons.Outlined.QrCodeScanner, contentDescription = null, modifier = Modifier.padding(end = Spacing.xs))
                                        Text(stringResource(R.string.event_scan_member_btn))
                                    }
                                }
                            }

                            if (me?.can("admin_plan") == true) {
                                FilledTonalButton(
                                    onClick = {
                                        UiHaptics.tick(view)
                                        nav.navigate(AppRoute.PlanTransport(event.id))
                                    },
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Icon(Icons.AutoMirrored.Outlined.AltRoute, contentDescription = null, modifier = Modifier.padding(end = Spacing.xs))
                                    Text(stringResource(R.string.event_plan_transport))
                                }
                            }

                            // Inline Drive Requests for Volunteer Drivers
                            if (open && isGoing && me?.can("drive", "create") == true) {
                                OutlinedCard(
                                    modifier = Modifier.fillMaxWidth(),
                                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                                ) {
                                    Column(
                                        Modifier.padding(Spacing.sm),
                                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    ) {
                                        Row(
                                            Modifier.fillMaxWidth(),
                                            horizontalArrangement = Arrangement.SpaceBetween,
                                            verticalAlignment = Alignment.CenterVertically,
                                        ) {
                                            Text(
                                                stringResource(R.string.event_drive_requests_title),
                                                style = MaterialTheme.typography.titleSmall,
                                            )
                                            TextButton(
                                                onClick = {
                                                    showDriveRequests = !showDriveRequests
                                                    if (showDriveRequests) vm.loadDriveRequests()
                                                },
                                            ) {
                                                Text(
                                                    stringResource(
                                                        if (showDriveRequests) {
                                                            R.string.event_offer_drive_hide
                                                        } else {
                                                            R.string.event_offer_drive_show
                                                        },
                                                    ),
                                                )
                                            }
                                        }
                                        if (showDriveRequests) {
                                            if (state.driveRequestsLoading) {
                                                InlineButtonSpinner()
                                            } else if (state.driveRequests.isEmpty()) {
                                                Text(
                                                    stringResource(R.string.drive_requests_empty),
                                                    style = MaterialTheme.typography.bodySmall,
                                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                )
                                            } else {
                                                state.driveRequests.forEach { reqRide ->
                                                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                                    Row(
                                                        Modifier.fillMaxWidth(),
                                                        horizontalArrangement = Arrangement.SpaceBetween,
                                                        verticalAlignment = Alignment.CenterVertically,
                                                    ) {
                                                        Column(Modifier.weight(1f)) {
                                                            Text(
                                                                reqRide.proxy?.riderName?.takeIf { reqRide.isProxy && it.isNotBlank() }
                                                                    ?: reqRide.passenger.name,
                                                                style = MaterialTheme.typography.titleSmall,
                                                            )
                                                            Text(
                                                                "${KlTime.displayTime(reqRide.pickupAt)} · ${reqRide.area.name} → ${reqRide.destination}",
                                                                style = MaterialTheme.typography.bodySmall,
                                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                            )
                                                        }
                                                        FilledTonalButton(
                                                            onClick = { vm.claimDriveRequest(reqRide.id, driveClaimedMsg) },
                                                            enabled = !state.busy,
                                                        ) {
                                                            Text(stringResource(R.string.drive_claim))
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Visitor Passes Section
                            if (open && (isGoing || event.canManage) && canCreateRide) {
                                LaunchedEffect(event.id, event.status) {
                                    vm.loadVisitorsAndAreas()
                                }
                                VisitorInvitesSection(
                                    eventTitle = event.title,
                                    eventStartsAt = event.startsAt,
                                    visitors = state.visitors,
                                    areas = state.areas,
                                    busy = state.busy,
                                    lastSharePair = state.visitorSharePair,
                                    onCreateVisitor = vm::createVisitor,
                                    onReissueVisitor = vm::reissueVisitor,
                                    onRevokeVisitor = vm::revokeVisitor,
                                    onOpenRide = { rid -> nav.navigate(AppRoute.RideDetail(rid)) },
                                    onDismissSharePair = vm::dismissVisitorShare,
                                    onCopiedToast = { scope.launch { snackbar.showSnackbar(copiedNotice) } },
                                )
                            }

                            // Organiser Management Panel
                            if (event.canManage) {
                                val activeSignups = remember(state.signups) {
                                    state.signups.filter { it.status != "cancelled" }
                                }
                                val attendedTotal = remember(activeSignups) {
                                    activeSignups.count { it.attended == 1L }
                                }
                                OutlinedCard(
                                    modifier = Modifier.fillMaxWidth(),
                                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                                ) {
                                    Column(
                                        Modifier.padding(Spacing.sm),
                                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    ) {
                                        Row(
                                            Modifier.fillMaxWidth(),
                                            horizontalArrangement = Arrangement.SpaceBetween,
                                            verticalAlignment = Alignment.CenterVertically,
                                        ) {
                                            Text(
                                                stringResource(R.string.event_manage_title),
                                                style = MaterialTheme.typography.titleMedium,
                                            )
                                            if (event.status != "draft" && event.attendanceEnabled) {
                                                Text(
                                                    stringResource(
                                                        R.string.event_manage_attendance_count,
                                                        attendedTotal,
                                                        activeSignups.size,
                                                    ),
                                                    style = MaterialTheme.typography.labelLarge,
                                                    color = MaterialTheme.colorScheme.primary,
                                                )
                                            }
                                        }

                                        FlowRow(
                                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                        ) {
                                            if (isPublished || event.status == "draft") {
                                                OutlinedButton(
                                                    onClick = { nav.navigate(AppRoute.EventEdit(event.id)) },
                                                ) {
                                                    Icon(Icons.Outlined.Edit, contentDescription = null, modifier = Modifier.size(16.dp))
                                                    Spacer(Modifier.width(Spacing.xxs))
                                                    Text(stringResource(R.string.event_edit_btn))
                                                }
                                            }
                                            if (me?.can("events", "create") == true) {
                                                OutlinedButton(
                                                    onClick = { nav.navigate(AppRoute.EventNew(duplicateId = event.id)) },
                                                ) {
                                                    Icon(Icons.Outlined.ContentCopy, contentDescription = null, modifier = Modifier.size(16.dp))
                                                    Spacer(Modifier.width(Spacing.xxs))
                                                    Text(stringResource(R.string.event_duplicate_btn))
                                                }
                                            }
                                            if (event.status != "draft" && event.attendanceEnabled) {
                                                OutlinedButton(onClick = { showVenueQr = true }) {
                                                    Icon(Icons.Outlined.QrCode2, contentDescription = null, modifier = Modifier.size(16.dp))
                                                    Spacer(Modifier.width(Spacing.xxs))
                                                    Text(stringResource(R.string.event_venue_qr_btn))
                                                }
                                                OutlinedButton(
                                                    onClick = {
                                                        walkInQuery = ""
                                                        walkInOpen = true
                                                        vm.searchWalkInMembers("")
                                                    },
                                                ) {
                                                    Icon(Icons.Outlined.PersonAdd, contentDescription = null, modifier = Modifier.size(16.dp))
                                                    Spacer(Modifier.width(Spacing.xxs))
                                                    Text(stringResource(R.string.event_walk_in_btn))
                                                }
                                            }
                                        }

                                        if (event.status != "draft") {
                                            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                            SectionHeader(
                                                title = stringResource(R.string.event_signups_title),
                                                count = activeSignups.size,
                                            )
                                            if (activeSignups.isEmpty()) {
                                                Text(
                                                    stringResource(R.string.event_signups_empty),
                                                    style = MaterialTheme.typography.bodySmall,
                                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                )
                                            } else {
                                                activeSignups.forEachIndexed { idx, row ->
                                                    if (idx > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                                    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                                        Row(
                                                            Modifier.fillMaxWidth(),
                                                            horizontalArrangement = Arrangement.SpaceBetween,
                                                            verticalAlignment = Alignment.CenterVertically,
                                                        ) {
                                                            Text(
                                                                text = row.name + (row.asGroup?.let { " · $it" }.orEmpty()),
                                                                style = MaterialTheme.typography.bodyMedium,
                                                                modifier = Modifier.weight(1f),
                                                            )
                                                            if (row.status == "waitlist") {
                                                                StatusPill(
                                                                    stringResource(R.string.event_waitlist),
                                                                    RideStatusKind.PENDING,
                                                                )
                                                            }
                                                        }
                                                        val roleSummary = listOfNotNull(
                                                            if (row.needRide) stringResource(R.string.event_need_ride) else null,
                                                            if (row.offerDrive) stringResource(R.string.event_signup_offers_seats, row.offerSeats.toInt()) else null,
                                                            if (!row.needRide && !row.offerDrive && row.selfTransport) stringResource(R.string.event_going_own_way) else null,
                                                        ).joinToString(" · ")
                                                        if (roleSummary.isNotBlank()) {
                                                            Text(
                                                                roleSummary,
                                                                style = MaterialTheme.typography.bodySmall,
                                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                            )
                                                        }
                                                        if (event.attendanceEnabled) {
                                                            Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                                                FilterChip(
                                                                    selected = row.attended == 1L,
                                                                    onClick = {
                                                                        vm.markAttendance(row.userId, if (row.attended == 1L) null else true)
                                                                    },
                                                                    enabled = !state.busy,
                                                                    label = { Text(stringResource(R.string.event_attending_chip)) },
                                                                )
                                                                FilterChip(
                                                                    selected = row.attended == 0L,
                                                                    onClick = {
                                                                        vm.markAttendance(row.userId, if (row.attended == 0L) null else false)
                                                                    },
                                                                    enabled = !state.busy,
                                                                    label = { Text(stringResource(R.string.event_absent_chip)) },
                                                                )
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        if (event.status == "published" || event.status == "finished") {
                                            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                            Text(
                                                stringResource(R.string.event_after_title),
                                                style = MaterialTheme.typography.titleSmall,
                                            )
                                            if (event.mileageClaimable &&
                                                (event.canManage ||
                                                    me?.can("admin_events", "update") == true ||
                                                    me?.can("admin_plan", "update") == true ||
                                                    me?.can("admin_rides", "update") == true)
                                            ) {
                                                FilledTonalButton(
                                                    onClick = vm::openPostEventReimb,
                                                    enabled = !state.busy,
                                                    modifier = Modifier.fillMaxWidth(),
                                                ) {
                                                    Icon(Icons.Outlined.DirectionsCar, contentDescription = null, modifier = Modifier.size(18.dp))
                                                    Spacer(Modifier.width(Spacing.xs))
                                                    Text(stringResource(R.string.event_post_reimb_btn))
                                                }
                                            }
                                            val settleRef = event.settlement
                                            if (settleRef != null) {
                                                OutlinedButton(
                                                    onClick = { nav.navigate(AppRoute.SettlementDetail(settleRef.id)) },
                                                    modifier = Modifier.fillMaxWidth(),
                                                ) {
                                                    Text(stringResource(R.string.event_view_settlement))
                                                }
                                            } else if (me?.can("settlements", "create") == true) {
                                                OutlinedButton(
                                                    onClick = { nav.navigate(AppRoute.SettlementForm(eventId = event.id)) },
                                                    modifier = Modifier.fillMaxWidth(),
                                                ) {
                                                    Text(stringResource(R.string.event_create_settlement))
                                                }
                                            }
                                        }

                                        if (isPublished || event.status == "draft") {
                                            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                            FlowRow(
                                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                            ) {
                                                if (isPublished) {
                                                    OutlinedButton(
                                                        onClick = { confirmFinish = true },
                                                        enabled = !state.busy,
                                                    ) {
                                                        Text(stringResource(R.string.event_finish_btn))
                                                    }
                                                }
                                                if (event.canCancel) {
                                                    TextButton(
                                                        onClick = { confirmCancel = true },
                                                        enabled = !state.busy,
                                                        colors = ButtonDefaults.textButtonColors(
                                                            contentColor = MaterialTheme.colorScheme.error,
                                                        ),
                                                    ) {
                                                        Text(stringResource(R.string.event_cancel_btn))
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
        }
    }

    val event = state.event
    val passCode = event?.mySignup?.passCode?.takeIf { it.isNotBlank() && event.attendanceEnabled && event.status == "published" }
    if (showPassQr && event != null && passCode != null) {
        EventPassQrSheet(
            eventId = event.id,
            passCode = passCode,
            eventTitle = event.title,
            onDismiss = { showPassQr = false },
        )
    }
    val organiserScanMode = checkinOrganiserMode
    if (organiserScanMode != null && event != null) {
        EventCheckInSheet(
            eventId = event.id,
            busy = state.busy,
            error = state.checkinError,
            scannerModeForOrganiser = organiserScanMode,
            initialCode = if (organiserScanMode) "" else prefillCheckinCode,
            onSelfCheckin = vm::selfCheckin,
            onScanMemberPass = vm::scanMemberPass,
            onClearError = vm::clearCheckinError,
            onDismiss = {
                vm.clearCheckinError()
                checkinOrganiserMode = null
                prefillCheckinCode = ""
            },
        )
    }
    if (showSignup && event != null) {
        val canOfferDrive = me?.can("drive", "create") == true
        var needRide by remember { mutableStateOf(false) }
        var offerDrive by remember { mutableStateOf(false) }
        var selfTransport by remember { mutableStateOf(false) }
        var offerSeats by remember { androidx.compose.runtime.mutableIntStateOf(3) }
        var groupId by remember { mutableStateOf(event.joinAs.firstOrNull()?.id) }
        var groupExpanded by remember { mutableStateOf(false) }
        val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

        ModalBottomSheet(
            onDismissRequest = { showSignup = false },
            sheetState = sheetState,
        ) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .navigationBarsPadding()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = Spacing.md)
                    .padding(bottom = Spacing.md),
                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
            ) {
                Text(event.title, style = MaterialTheme.typography.titleLarge, color = MaterialTheme.colorScheme.onSurface)
                OutlinedCard(Modifier.fillMaxWidth()) {
                    Column {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier
                                .fillMaxWidth()
                                .defaultMinSize(minHeight = 52.dp)
                                .toggleable(
                                    value = needRide,
                                    role = Role.Checkbox,
                                    onValueChange = { checked ->
                                        UiHaptics.tick(view)
                                        needRide = checked
                                        if (checked) {
                                            offerDrive = false
                                            selfTransport = false
                                        }
                                    },
                                )
                                .padding(horizontal = Spacing.sm),
                        ) {
                            Checkbox(checked = needRide, onCheckedChange = null)
                            Icon(
                                Icons.Outlined.Place,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.primary,
                                modifier = Modifier.padding(start = Spacing.xs).size(20.dp),
                            )
                            Text(
                                stringResource(R.string.event_need_ride),
                                style = MaterialTheme.typography.bodyLarge,
                                modifier = Modifier.padding(start = Spacing.xs),
                            )
                        }
                        if (canOfferDrive) {
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .defaultMinSize(minHeight = 52.dp)
                                    .toggleable(
                                        value = offerDrive,
                                        role = Role.Checkbox,
                                        onValueChange = { checked ->
                                            UiHaptics.tick(view)
                                            offerDrive = checked
                                            if (checked) {
                                                needRide = false
                                                selfTransport = false
                                            }
                                        },
                                    )
                                    .padding(horizontal = Spacing.sm),
                            ) {
                                Checkbox(checked = offerDrive, onCheckedChange = null)
                                Icon(
                                    Icons.Outlined.DirectionsCar,
                                    contentDescription = null,
                                    tint = MaterialTheme.colorScheme.primary,
                                    modifier = Modifier.padding(start = Spacing.xs).size(20.dp),
                                )
                                Text(
                                    stringResource(R.string.event_offer_drive),
                                    style = MaterialTheme.typography.bodyLarge,
                                    modifier = Modifier.padding(start = Spacing.xs),
                                )
                            }
                            if (offerDrive) {
                                Row(
                                    Modifier.fillMaxWidth().padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                ) {
                                    Text(
                                        stringResource(R.string.event_offer_seats),
                                        style = MaterialTheme.typography.bodyMedium,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                    Row(verticalAlignment = Alignment.CenterVertically) {
                                        TextButton(
                                            onClick = { if (offerSeats > 1) offerSeats-- },
                                            enabled = offerSeats > 1,
                                        ) { Text("-") }
                                        Text(
                                            "$offerSeats",
                                            style = MaterialTheme.typography.titleMedium.tabular(),
                                            modifier = Modifier.padding(horizontal = Spacing.xs),
                                        )
                                        TextButton(
                                            onClick = { if (offerSeats < 8) offerSeats++ },
                                            enabled = offerSeats < 8,
                                        ) { Text("+") }
                                    }
                                }
                            }
                        }
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier
                                .fillMaxWidth()
                                .defaultMinSize(minHeight = 52.dp)
                                .toggleable(
                                    value = selfTransport,
                                    role = Role.Checkbox,
                                    onValueChange = { checked ->
                                        UiHaptics.tick(view)
                                        selfTransport = checked
                                        if (checked) {
                                            needRide = false
                                            offerDrive = false
                                        }
                                    },
                                )
                                .padding(horizontal = Spacing.sm),
                        ) {
                            Checkbox(checked = selfTransport, onCheckedChange = null)
                            Icon(
                                Icons.Outlined.CheckCircle,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.primary,
                                modifier = Modifier.padding(start = Spacing.xs).size(20.dp),
                            )
                            Text(
                                stringResource(R.string.event_going_own_way),
                                style = MaterialTheme.typography.bodyLarge,
                                modifier = Modifier.padding(start = Spacing.xs),
                            )
                        }
                    }
                }
                if (event.joinAs.size > 1) {
                    ExposedDropdownMenuBox(expanded = groupExpanded, onExpandedChange = { groupExpanded = it }) {
                        OutlinedTextField(
                            value = event.joinAs.firstOrNull { it.id == groupId }?.name ?: "",
                            onValueChange = {},
                            readOnly = true,
                            label = { Text(stringResource(R.string.event_join_as_group)) },
                            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = groupExpanded) },
                            modifier = Modifier.menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
                        )
                        ExposedDropdownMenu(expanded = groupExpanded, onDismissRequest = { groupExpanded = false }) {
                            event.joinAs.forEach { g ->
                                DropdownMenuItem(text = { Text(g.name) }, onClick = { groupId = g.id; groupExpanded = false })
                            }
                        }
                    }
                }
                Button(
                    onClick = {
                        UiHaptics.confirm(view)
                        vm.signup(
                            needRide = needRide,
                            offerDrive = offerDrive,
                            asGroupId = if (event.joinAs.size == 1) event.joinAs.first().id else groupId,
                            offerSeats = if (offerDrive) offerSeats else null,
                            selfTransport = if (selfTransport) true else null,
                        )
                        showSignup = false
                    },
                    modifier = Modifier.fillMaxWidth().height(52.dp),
                ) {
                    Text(stringResource(R.string.common_confirm))
                }
            }
        }
    }

    if (confirmWithdraw) {
        AlertDialog(
            onDismissRequest = { confirmWithdraw = false },
            title = { Text(stringResource(R.string.event_withdraw)) },
            text = { Text(stringResource(R.string.event_withdraw_confirm)) },
            confirmButton = {
                TextButton(
                    onClick = { vm.withdraw(); confirmWithdraw = false },
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                ) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = { TextButton(onClick = { confirmWithdraw = false }) { Text(stringResource(R.string.common_cancel)) } },
        )
    }

    if (confirmPublish) {
        AlertDialog(
            onDismissRequest = { confirmPublish = false },
            title = { Text(stringResource(R.string.event_publish)) },
            text = { Text(stringResource(R.string.event_publish_confirm)) },
            confirmButton = {
                TextButton(onClick = { vm.publish(); confirmPublish = false }) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = { TextButton(onClick = { confirmPublish = false }) { Text(stringResource(R.string.common_cancel)) } },
        )
    }

    if (confirmUnpublish) {
        AlertDialog(
            onDismissRequest = { confirmUnpublish = false },
            title = { Text(stringResource(R.string.event_unpublish)) },
            text = { Text(stringResource(R.string.event_unpublish_confirm)) },
            confirmButton = {
                TextButton(onClick = { vm.unpublish(); confirmUnpublish = false }) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = { TextButton(onClick = { confirmUnpublish = false }) { Text(stringResource(R.string.common_cancel)) } },
        )
    }

    if (confirmFinish) {
        AlertDialog(
            onDismissRequest = { confirmFinish = false },
            title = { Text(stringResource(R.string.event_finish_btn)) },
            text = { Text(stringResource(R.string.event_finish_confirm)) },
            confirmButton = {
                TextButton(onClick = { vm.finishEvent(finishDoneMsg); confirmFinish = false }) {
                    Text(stringResource(R.string.common_confirm))
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmFinish = false }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }

    if (confirmCancel) {
        AlertDialog(
            onDismissRequest = { confirmCancel = false },
            title = { Text(stringResource(R.string.event_cancel_btn)) },
            text = { Text(stringResource(R.string.event_cancel_confirm)) },
            confirmButton = {
                TextButton(
                    onClick = { vm.cancelEvent(cancelDoneMsg); confirmCancel = false },
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                ) {
                    Text(stringResource(R.string.common_confirm))
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmCancel = false }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }

    if (showVenueQr && event != null) {
        val qrPath = buildString {
            append("/events/${event.id}/checkin")
            event.checkinCode?.takeIf { it.isNotBlank() }?.let { append("?code=$it") }
        }
        VenueCheckinQrDialog(
            event = event,
            checkinUrl = container.webUrl(qrPath),
            busy = state.busy,
            onResetCode = { vm.resetCheckinCode(qrResetDoneMsg) },
            onDismiss = { showVenueQr = false },
            onCopied = { scope.launch { snackbar.showSnackbar(copiedNotice) } },
        )
    }

    if (walkInOpen) {
        WalkInCheckinDialog(
            query = walkInQuery,
            candidates = state.walkInCandidates,
            busy = state.busy,
            onQueryChange = { q ->
                walkInQuery = q
                vm.searchWalkInMembers(q)
            },
            onPickUser = { uid ->
                vm.markAttendance(uid, true)
                walkInOpen = false
            },
            onDismiss = { walkInOpen = false },
        )
    }

    val postEventResp = state.postEventDrivers
    if (state.postEventOpen && postEventResp != null) {
        PostEventReimbSheet(
            data = postEventResp,
            busy = state.busy,
            onSubmit = { req ->
                vm.submitPostEventReimb(req) { count, cents ->
                    context.getString(R.string.event_post_reimb_done, count, Money.rm(cents))
                }
            },
            onDismiss = vm::closePostEventReimb,
        )
    }
}
