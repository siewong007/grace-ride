package org.pjstatecm.graceride.ui.screens.events


import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
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
import androidx.compose.material.icons.automirrored.outlined.OpenInNew
import androidx.compose.material.icons.outlined.CalendarToday
import androidx.compose.material.icons.outlined.CheckCircle
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.EventAvailable
import androidx.compose.material.icons.outlined.Map
import androidx.compose.material.icons.outlined.People
import androidx.compose.material.icons.outlined.Place
import androidx.compose.material.icons.outlined.QrCode2
import androidx.compose.material.icons.outlined.QrCodeScanner
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.MySignup
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.RideStatus
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.DetailSkeleton
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.EventIcon
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.ScheduleTimeline
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.components.TimelineBadge
import org.pjstatecm.graceride.ui.components.TimelineKind
import org.pjstatecm.graceride.ui.components.TimelineNode
import org.pjstatecm.graceride.ui.components.TimelineState
import org.pjstatecm.graceride.ui.copyToClipboard
import org.pjstatecm.graceride.ui.openInMaps
import org.pjstatecm.graceride.ui.openInWebApp
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EventDetailScreen(
    container: AppContainer,
    nav: NavHostController,
    id: Long,
    checkinCode: String? = null,
) {
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
    var deepCheckinHandled by remember(id, checkinCode) { mutableStateOf(false) }
    val checkinSuccessFmt = stringResource(R.string.event_checkin_success, "%s")
    val checkinAlreadyFmt = stringResource(R.string.event_checkin_already, "%s")
    val scanMemberSuccessFmt = stringResource(R.string.event_scan_member_success, "%s")
    val scanMemberAlreadyFmt = stringResource(R.string.event_scan_member_already, "%s")
    var confirmWithdraw by remember { mutableStateOf(false) }
    var confirmPublish by remember { mutableStateOf(false) }
    var confirmUnpublish by remember { mutableStateOf(false) }
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
            val msg = if (res.alreadyAttended) {
                checkinAlreadyFmt.format(eventTitle)
            } else {
                checkinSuccessFmt.format(eventTitle)
            }
            snackbar.showSnackbar(msg)
            checkinOrganiserMode = null
            prefillCheckinCode = ""
            vm.clearCheckinResults()
        }
        state.scanMemberResult?.let { res ->
            val name = res.memberName.ifBlank { "#${res.userId}" }
            val msg = if (res.alreadyAttended) {
                scanMemberAlreadyFmt.format(name)
            } else {
                scanMemberSuccessFmt.format(name)
            }
            snackbar.showSnackbar(msg)
            checkinOrganiserMode = null
            prefillCheckinCode = ""
            vm.clearCheckinResults()
        }
    }
    LaunchedEffect(state.event?.id, checkinCode) {
        val ev = state.event ?: return@LaunchedEffect
        if (!deepCheckinHandled && checkinCode != null) {
            deepCheckinHandled = true
            if (ev.attendanceEnabled && ev.status == "published" && ev.mySignup?.attended != true) {
                prefillCheckinCode = checkinCode.trim()
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
        val viewMode = when {
            state.loading -> "loading"
            state.event == null -> "error"
            else -> "content"
        }
        Crossfade(
            targetState = viewMode,
            animationSpec = tween(200),
            label = "event_detail_state",
            modifier = Modifier.padding(padding),
        ) { mode ->
            when (mode) {
                "loading" -> DetailSkeleton()
                "error" -> ErrorView(state.error ?: "", vm::load)
                else -> {
                    val event = state.event!!
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
                    val canCreateRide = container.auth.me?.can("rides", "create") == true
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
                        Column(
                            Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(Spacing.sm),
                            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                        ) {
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
                                } else if (isPublished && container.auth.me?.status == "active" && !isEndedByTime) {
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

                            val canOpenEventWeb = event.canManage ||
                                me?.can("admin_events") == true ||
                                me?.can("events", "update") == true
                            if (canOpenEventWeb) {
                                OutlinedButton(
                                    onClick = { openInWebApp(context, container.webUrl("events/${event.id}")) },
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Icon(Icons.AutoMirrored.Outlined.OpenInNew, contentDescription = null, modifier = Modifier.padding(end = Spacing.xs))
                                    Text(stringResource(R.string.event_open_web_full))
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
        val canOfferDrive = container.auth.me?.can("drive", "create") == true
        var needRide by remember { mutableStateOf(false) }
        var offerDrive by remember { mutableStateOf(false) }
        var offerSeats by remember { mutableStateOf(3) }
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
                                        if (checked) offerDrive = false
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
                                            if (checked) needRide = false
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
}
