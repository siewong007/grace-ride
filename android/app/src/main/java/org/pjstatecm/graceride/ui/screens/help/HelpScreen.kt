package org.pjstatecm.graceride.ui.screens.help

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.HelpOutline
import androidx.compose.material.icons.automirrored.outlined.MenuBook
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.ArrowDownward
import androidx.compose.material.icons.outlined.ArrowUpward
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material.icons.outlined.UploadFile
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
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
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import java.util.Locale
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.GuideSummaryDto
import org.pjstatecm.graceride.data.api.HelpMemberDto
import org.pjstatecm.graceride.data.api.TicketSummaryDto
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.AdaptiveContentFrame
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.SkeletonLoadingList
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors

internal fun ticketStatusKind(status: String): RideStatusKind = when (status) {
    "answered" -> RideStatusKind.ACTIVE
    "closed" -> RideStatusKind.DONE
    else -> RideStatusKind.PENDING
}

@Composable
internal fun ticketStatusLabel(status: String): String = when (status) {
    "answered" -> stringResource(R.string.help_status_answered)
    "closed" -> stringResource(R.string.help_status_closed)
    else -> stringResource(R.string.help_status_open)
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HelpScreen(
    container: AppContainer,
    nav: NavHostController,
    rideId: Long? = null,
    initialTab: String? = null,
) {
    val vm: HelpViewModel = viewModel(
        key = "help-${rideId ?: 0}-${initialTab ?: "default"}",
        factory = HelpViewModel.factory(container, rideId, initialTab),
    )
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val me = (authState as? AuthState.SignedIn)?.me ?: container.auth.me
    val isStaff = me?.can("admin_help") == true
    val canEditGuides = me?.can("admin_help", "update") == true
    val canCreateTicket = me?.can("help", "create") == true
    val snackbar = remember { SnackbarHostState() }
    val lifecycleOwner = LocalLifecycleOwner.current
    val context = LocalContext.current
    val view = LocalView.current
    val isZh = Locale.getDefault().language == "zh"

    var showAskSheet by remember(rideId) { mutableStateOf(rideId != null && canCreateTicket) }

    DisposableEffect(lifecycleOwner) {
        var first = true
        val obs = LifecycleEventObserver { _, ev ->
            if (ev == Lifecycle.Event.ON_RESUME) {
                if (first) first = false else vm.refresh()
            }
        }
        lifecycleOwner.lifecycle.addObserver(obs)
        onDispose { lifecycleOwner.lifecycle.removeObserver(obs) }
    }

    LaunchedEffect(state.createdTicketId) {
        val id = state.createdTicketId ?: return@LaunchedEffect
        showAskSheet = false
        vm.consumeCreatedTicket()
        nav.navigate(AppRoute.HelpDetail(id))
    }

    LaunchedEffect(state.message) {
        state.message?.let {
            snackbar.showSnackbar(it)
            vm.clearMessage()
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.help_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
            )
        },
        floatingActionButton = {
            if (state.tab == "guides" && canEditGuides) {
                ExtendedFloatingActionButton(
                    onClick = {
                        UiHaptics.tick(view)
                        nav.navigate(AppRoute.HelpGuideEditor())
                    },
                    icon = { Icon(Icons.Outlined.Add, contentDescription = null) },
                    text = { Text(stringResource(R.string.help_guide_new)) },
                    containerColor = MaterialTheme.colorScheme.primary,
                    contentColor = MaterialTheme.colorScheme.onPrimary,
                )
            } else if (canCreateTicket) {
                ExtendedFloatingActionButton(
                    onClick = {
                        UiHaptics.tick(view)
                        showAskSheet = true
                    },
                    icon = { Icon(Icons.Outlined.Add, contentDescription = null) },
                    text = { Text(stringResource(R.string.help_ask)) },
                    containerColor = MaterialTheme.colorScheme.primary,
                    contentColor = MaterialTheme.colorScheme.onPrimary,
                )
            }
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        AdaptiveContentFrame(
            maxWidth = 920.dp,
            compactHorizontalPadding = 0.dp,
            modifier = Modifier.fillMaxSize().padding(padding),
        ) {
            Column(Modifier.fillMaxSize()) {
            SingleChoiceSegmentedButtonRow(
                Modifier
                    .fillMaxWidth()
                    .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
            ) {
                val tabs = listOf("guides" to R.string.help_tab_guides, "tickets" to R.string.help_tab_tickets)
                tabs.forEachIndexed { idx, (key, res) ->
                    SegmentedButton(
                        selected = state.tab == key,
                        onClick = {
                            UiHaptics.tick(view)
                            vm.selectTab(key)
                        },
                        shape = SegmentedButtonDefaults.itemShape(idx, tabs.size),
                    ) {
                        Text(stringResource(res))
                    }
                }
            }

            OutlinedTextField(
                value = state.query,
                onValueChange = vm::setQuery,
                placeholder = { Text(stringResource(R.string.help_search_placeholder)) },
                leadingIcon = { Icon(Icons.Outlined.Search, contentDescription = null) },
                trailingIcon = if (state.query.isNotEmpty()) {
                    {
                        IconButton(onClick = { vm.setQuery("") }) {
                            Icon(Icons.Outlined.Close, contentDescription = stringResource(R.string.a11y_clear))
                        }
                    }
                } else {
                    null
                },
                singleLine = true,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = Spacing.sm, vertical = Spacing.xxs),
            )

            val viewMode = when {
                state.loading -> "loading"
                state.error != null && state.guides.isEmpty() && state.tickets.isEmpty() -> "error"
                else -> "content"
            }
            Crossfade(
                targetState = viewMode,
                animationSpec = tween(200),
                label = "help_state",
                modifier = Modifier.weight(1f),
            ) { mode ->
                when (mode) {
                    "loading" -> SkeletonLoadingList(count = 4)
                    "error" -> ErrorView(state.error ?: "", onRetry = vm::refresh)
                    else -> PullToRefreshBox(
                        isRefreshing = state.refreshing,
                        onRefresh = vm::refresh,
                        modifier = Modifier.fillMaxSize(),
                    ) {
                        if (state.tab == "guides") {
                            val needle = state.query.trim().lowercase()
                            val visibleGuides = state.guides.filter { g ->
                                val matchAudience = state.guideAudience == "all" || g.audience == state.guideAudience
                                val matchQ = needle.isEmpty() ||
                                    g.titleEn.lowercase().contains(needle) ||
                                    g.titleZh.lowercase().contains(needle) ||
                                    g.summaryEn.lowercase().contains(needle) ||
                                    g.summaryZh.lowercase().contains(needle) ||
                                    g.steps.any {
                                        it.titleEn.lowercase().contains(needle) ||
                                            it.titleZh.lowercase().contains(needle) ||
                                            it.bodyEn.lowercase().contains(needle) ||
                                            it.bodyZh.lowercase().contains(needle)
                                    }
                                matchAudience && matchQ
                            }
                            LazyColumn(
                                Modifier.fillMaxSize(),
                                contentPadding = PaddingValues(start = Spacing.sm, end = Spacing.sm, bottom = 88.dp),
                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                item(key = "guide_filters") {
                                    val audOpts = buildList {
                                        add("all" to R.string.help_audience_all)
                                        add("member" to R.string.help_audience_members)
                                        if (isStaff) add("staff" to R.string.help_audience_staff)
                                    }
                                    LazyRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                        items(audOpts, key = { it.first }) { (key, res) ->
                                            FilterChip(
                                                selected = state.guideAudience == key,
                                                onClick = {
                                                    UiHaptics.tick(view)
                                                    vm.setGuideAudience(key)
                                                },
                                                label = { Text(stringResource(res)) },
                                            )
                                        }
                                    }
                                }
                                if (visibleGuides.isEmpty()) {
                                    item {
                                        EmptyView(text = stringResource(R.string.help_guides_empty), icon = Icons.AutoMirrored.Outlined.MenuBook)
                                    }
                                } else {
                                    itemsIndexed(visibleGuides, key = { _, g -> g.id }) { idx, guide ->
                                        GuideCard(
                                            guide = guide,
                                            isZh = isZh,
                                            canReorder = canEditGuides && needle.isEmpty() && state.guideAudience == "all",
                                            canMoveUp = idx > 0,
                                            canMoveDown = idx < visibleGuides.lastIndex,
                                            onMove = { delta ->
                                                UiHaptics.tick(view)
                                                vm.moveGuide(guide.id, delta)
                                            },
                                            onClick = {
                                                UiHaptics.tick(view)
                                                nav.navigate(AppRoute.HelpGuideDetail(guide.slug.ifBlank { guide.id.toString() }))
                                            },
                                        )
                                    }
                                }
                            }
                        } else {
                            val needle = state.query.trim().lowercase()
                            val visibleTickets = state.tickets.filter { tk ->
                                needle.isEmpty() ||
                                    tk.subject.lowercase().contains(needle) ||
                                    tk.userName.lowercase().contains(needle) ||
                                    (tk.lastBody?.lowercase()?.contains(needle) == true)
                            }
                            LazyColumn(
                                Modifier.fillMaxSize(),
                                contentPadding = PaddingValues(start = Spacing.sm, end = Spacing.sm, bottom = 88.dp),
                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                item(key = "ticket_filters") {
                                    LazyRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                        if (isStaff) {
                                            item("scope_all") {
                                                FilterChip(
                                                    selected = state.ticketScopeAll,
                                                    onClick = {
                                                        UiHaptics.tick(view)
                                                        vm.setTicketFilters(scopeAll = !state.ticketScopeAll)
                                                    },
                                                    label = {
                                                        Text(
                                                            "${stringResource(R.string.help_all_requests)} (${state.counts.open + state.counts.answered})",
                                                        )
                                                    },
                                                )
                                            }
                                            if (state.ticketScopeAll) {
                                                item("assigned_me") {
                                                    FilterChip(
                                                        selected = state.assignedToMe,
                                                        onClick = {
                                                            UiHaptics.tick(view)
                                                            vm.setTicketFilters(assignedToMe = !state.assignedToMe)
                                                        },
                                                        label = { Text(stringResource(R.string.help_assigned_me)) },
                                                    )
                                                }
                                            }
                                        }
                                        val statuses = listOf(
                                            "all" to R.string.help_audience_all,
                                            "open" to R.string.help_status_open,
                                            "answered" to R.string.help_status_answered,
                                            "closed" to R.string.help_status_closed,
                                        )
                                        items(statuses, key = { it.first }) { (key, res) ->
                                            FilterChip(
                                                selected = state.ticketStatus == key,
                                                onClick = {
                                                    UiHaptics.tick(view)
                                                    vm.setTicketFilters(status = key)
                                                },
                                                label = { Text(stringResource(res)) },
                                            )
                                        }
                                    }
                                }
                                if (visibleTickets.isEmpty()) {
                                    item {
                                        EmptyView(
                                            text = stringResource(R.string.help_tickets_empty),
                                            icon = Icons.AutoMirrored.Outlined.HelpOutline,
                                        )
                                    }
                                } else {
                                    items(visibleTickets, key = { it.id }) { tk ->
                                        TicketCard(
                                            ticket = tk,
                                            showUser = state.ticketScopeAll,
                                            onClick = {
                                                UiHaptics.tick(view)
                                                nav.navigate(AppRoute.HelpDetail(tk.id))
                                            },
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
    }

    if (showAskSheet) {
        AskHelpSheet(
            initialRideId = rideId,
            isStaff = isStaff,
            memberResults = state.memberResults,
            uploading = state.uploadingAttachment,
            uploadedFileId = state.uploadedFileId,
            submitting = state.submitting,
            onSearchMembers = vm::searchMembers,
            onUploadFile = { uri -> vm.uploadAttachment(context, uri) },
            onDismiss = {
                showAskSheet = false
                vm.clearAttachment()
            },
            onSubmit = { subj, body, rid, uid -> vm.createTicket(subj, body, rid, uid) },
        )
    }
}

@Composable
private fun GuideCard(
    guide: GuideSummaryDto,
    isZh: Boolean,
    canReorder: Boolean,
    canMoveUp: Boolean,
    canMoveDown: Boolean,
    onMove: (Int) -> Unit,
    onClick: () -> Unit,
) {
    val title = if (isZh) guide.titleZh.ifBlank { guide.titleEn } else guide.titleEn.ifBlank { guide.titleZh }
    val summary = if (isZh) guide.summaryZh.ifBlank { guide.summaryEn } else guide.summaryEn.ifBlank { guide.summaryZh }
    OutlinedCard(onClick = onClick, modifier = Modifier.fillMaxWidth()) {
        Row(
            Modifier.padding(Spacing.sm),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
                Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs), verticalAlignment = Alignment.CenterVertically) {
                    Text(title, style = MaterialTheme.typography.titleMedium, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f, fill = false))
                    if (!guide.published) {
                        StatusPill(text = stringResource(R.string.help_guide_draft), kind = RideStatusKind.PENDING)
                    }
                }
                if (summary.isNotBlank()) {
                    Text(summary, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 2, overflow = TextOverflow.Ellipsis)
                }
                val stepCnt = guide.stepCount.coerceAtLeast(guide.steps.size)
                Text(
                    pluralStringResource(R.plurals.help_guide_steps_count_plural, stepCnt, stepCnt),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.primary,
                )
            }
            if (canReorder) {
                Column {
                    IconButton(onClick = { onMove(-1) }, enabled = canMoveUp) {
                        Icon(Icons.Outlined.ArrowUpward, contentDescription = stringResource(R.string.help_move_up))
                    }
                    IconButton(onClick = { onMove(1) }, enabled = canMoveDown) {
                        Icon(Icons.Outlined.ArrowDownward, contentDescription = stringResource(R.string.help_move_down))
                    }
                }
            }
        }
    }
}

@Composable
private fun TicketCard(
    ticket: TicketSummaryDto,
    showUser: Boolean,
    onClick: () -> Unit,
) {
    OutlinedCard(onClick = onClick, modifier = Modifier.fillMaxWidth()) {
        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Text(ticket.subject, style = MaterialTheme.typography.titleMedium, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
                StatusPill(text = ticketStatusLabel(ticket.status), kind = ticketStatusKind(ticket.status))
            }
            if (!ticket.lastBody.isNullOrBlank()) {
                Text(ticket.lastBody, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 2, overflow = TextOverflow.Ellipsis)
            }
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                val meta = buildString {
                    if (showUser && ticket.userName.isNotBlank()) append("${ticket.userName} · ")
                    ticket.rideId?.let { append("#$it · ") }
                    ticket.assigneeName?.takeIf { it.isNotBlank() }?.let { append(it) }
                }
                Text(meta, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                Text(KlTime.display(ticket.updatedAt.ifBlank { ticket.createdAt }), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun AskHelpSheet(
    initialRideId: Long?,
    isStaff: Boolean,
    memberResults: List<HelpMemberDto>,
    uploading: Boolean,
    uploadedFileId: Long?,
    submitting: Boolean,
    onSearchMembers: (String) -> Unit,
    onUploadFile: (android.net.Uri) -> Unit,
    onDismiss: () -> Unit,
    onSubmit: (String, String, Long?, Long?) -> Unit,
) {
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val view = LocalView.current
    var subject by remember { mutableStateOf("") }
    var body by remember { mutableStateOf("") }
    var rideIdText by remember(initialRideId) { mutableStateOf(initialRideId?.toString() ?: "") }
    var memberQuery by remember { mutableStateOf("") }
    var selectedMember by remember { mutableStateOf<HelpMemberDto?>(null) }

    val picker = rememberLauncherForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri ->
        if (uri != null) onUploadFile(uri)
    }

    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
        Column(
            Modifier
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .navigationBarsPadding()
                .imePadding()
                .padding(horizontal = Spacing.sm)
                .padding(bottom = Spacing.md),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Text(stringResource(R.string.help_ask), style = MaterialTheme.typography.titleMedium)
            OutlinedTextField(
                value = subject,
                onValueChange = { subject = it },
                label = { Text(stringResource(R.string.help_subject)) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = body,
                onValueChange = { body = it },
                label = { Text(stringResource(R.string.help_body)) },
                minLines = 3,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = rideIdText,
                onValueChange = { rideIdText = it.filter(Char::isDigit) },
                label = { Text(stringResource(R.string.help_linked_ride)) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            if (isStaff) {
                OutlinedTextField(
                    value = selectedMember?.name ?: memberQuery,
                    onValueChange = {
                        selectedMember = null
                        memberQuery = it
                        onSearchMembers(it)
                    },
                    label = { Text(stringResource(R.string.help_raise_for_member)) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                if (selectedMember == null && memberResults.isNotEmpty()) {
                    OutlinedCard(Modifier.fillMaxWidth()) {
                        Column(Modifier.padding(Spacing.xs)) {
                            memberResults.take(5).forEach { m ->
                                TextButton(
                                    onClick = {
                                        selectedMember = m
                                        memberQuery = m.name
                                    },
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Text(m.name, fontWeight = FontWeight.Medium)
                                }
                            }
                        }
                    }
                }
            }
            OutlinedButton(
                onClick = { picker.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly)) },
                enabled = !uploading,
                modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
            ) {
                if (uploading) {
                    InlineButtonSpinner()
                } else {
                    Icon(Icons.Outlined.UploadFile, contentDescription = null, modifier = Modifier.size(18.dp))
                    Spacer(Modifier.width(Spacing.xs))
                    Text(stringResource(if (uploadedFileId == null) R.string.help_attach_photo else R.string.help_photo_attached))
                }
            }
            Button(
                onClick = {
                    UiHaptics.confirm(view)
                    onSubmit(subject, body, rideIdText.toLongOrNull(), selectedMember?.id)
                },
                enabled = !submitting && !uploading && subject.isNotBlank() && body.isNotBlank(),
                colors = graceRidePrimaryButtonColors(),
                modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
            ) {
                if (submitting) InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                else Text(stringResource(R.string.help_submit))
            }
        }
    }
}
