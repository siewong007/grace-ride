package org.pjstatecm.graceride.ui.screens.reimbursements

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.ReceiptLong
import androidx.compose.material.icons.outlined.ChevronLeft
import androidx.compose.material.icons.outlined.ChevronRight
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.Download
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.ExperimentalMaterial3Api
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
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import coil.compose.AsyncImage
import java.time.YearMonth
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.PayoutBatchDto
import org.pjstatecm.graceride.data.api.ReimbursementDto
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.platform.CsvBuilder
import org.pjstatecm.graceride.platform.ExportShare
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.SecureScreenEffect
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.AdaptiveContentFrame
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.SkeletonLoadingList
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.components.ZoomableImageDialog
import org.pjstatecm.graceride.ui.copyToClipboard
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun AdminReimbursementsScreen(
    container: AppContainer,
    nav: NavHostController,
    initialTab: String = "review",
) {
    val authState by container.auth.state.collectAsStateWithLifecycle()
    SecureScreenEffect()
    val me = (authState as? AuthState.SignedIn)?.me ?: container.auth.me
    val canReview = me?.can("admin_reimb_review") == true
    val canPay = me?.can("admin_reimb_pay") == true
    val effectiveInitial = when {
        initialTab in setOf("payout", "batches") && canPay -> initialTab
        canReview -> "review"
        canPay -> "payout"
        else -> initialTab
    }
    val vm: AdminReimbursementsViewModel = viewModel(
        key = "admin-reimb-$effectiveInitial",
        factory = AdminReimbursementsViewModel.factory(container, effectiveInitial),
    )
    val state by vm.state.collectAsStateWithLifecycle()
    val snackbar = remember { SnackbarHostState() }
    val context = LocalContext.current
    val view = LocalView.current
    val scope = rememberCoroutineScope()
    val copiedMsg = stringResource(R.string.common_copied)

    var reviewingClaim by remember { mutableStateOf<ReimbursementDto?>(null) }
    var payingClaim by remember { mutableStateOf<ReimbursementDto?>(null) }
    var payingBatch by remember { mutableStateOf<PayoutBatchDto?>(null) }
    var zoomedReceiptId by remember { mutableStateOf<Long?>(null) }
    var batchMonth by remember {
        mutableStateOf(
            runCatching { YearMonth.parse(KlTime.today().take(7)) }.getOrElse { YearMonth.now(ZoneId.of("Asia/Kuala_Lumpur")) },
        )
    }

    LaunchedEffect(state.message) {
        state.message?.let {
            snackbar.showSnackbar(it)
            vm.clearMessage()
        }
    }

    LaunchedEffect(state.exportedBatchCsv) {
        val (name, bytes) = state.exportedBatchCsv ?: return@LaunchedEffect
        ExportShare.writeAndShareFile(context, name, "text/csv", bytes)
        vm.clearExportedBatchCsv()
    }

    val tabs = buildList {
        if (canReview) add("review" to R.string.reimb_admin_tab_review)
        if (canPay) {
            add("payout" to R.string.reimb_admin_tab_payout)
            add("batches" to R.string.reimb_admin_tab_batches)
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.reimb_admin_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
                actions = {
                    if (state.tab != "batches" && state.claims.isNotEmpty()) {
                        IconButton(
                            onClick = {
                                scope.launch {
                                    val headers = listOf("ID", "Claimant", "Kind", "Amount", "Status", "Payee", "Phone", "Paid Ref", "Created")
                                    val rows = state.claims.map { r ->
                                        listOf(
                                            r.id.toString(),
                                            r.user.name,
                                            r.kind,
                                            Money.rm(r.amountCents),
                                            r.status,
                                            r.payeeName.orEmpty(),
                                            r.payeePhone.orEmpty(),
                                            r.paidRef.orEmpty(),
                                            r.createdAt,
                                        )
                                    }
                                    ExportShare.writeAndShareFile(
                                        context,
                                        "reimbursements-${state.tab}-${KlTime.today()}.csv",
                                        "text/csv",
                                        CsvBuilder.buildBytes(headers, rows),
                                    )
                                }
                            },
                        ) {
                            Icon(Icons.Outlined.Download, contentDescription = stringResource(R.string.common_export_csv))
                        }
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        Column(
            Modifier
                .fillMaxSize()
                .padding(padding),
        ) {
            if (tabs.size > 1) {
                SingleChoiceSegmentedButtonRow(
                    Modifier
                        .fillMaxWidth()
                        .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                ) {
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
            }

            if (state.tab != "batches") {
                OutlinedTextField(
                    value = state.query,
                    onValueChange = vm::setQuery,
                    placeholder = { Text(stringResource(R.string.reimb_search_hint)) },
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
                val statusChips = if (state.tab == "payout") {
                    listOf("approved" to R.string.reimb_status_approved, "paid" to R.string.reimb_status_paid, "all" to R.string.reimb_filter_all)
                } else {
                    listOf(
                        "pending" to R.string.reimb_status_pending,
                        "returned" to R.string.reimb_status_returned,
                        "approved" to R.string.reimb_status_approved,
                        "rejected" to R.string.reimb_status_rejected,
                        "all" to R.string.reimb_filter_all,
                    )
                }
                LazyRow(
                    contentPadding = PaddingValues(horizontal = Spacing.sm),
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    items(statusChips, key = { it.first }) { (key, res) ->
                        FilterChip(
                            selected = state.statusFilter == key,
                            onClick = {
                                UiHaptics.tick(view)
                                vm.setStatusFilter(key)
                            },
                            label = { Text(stringResource(res)) },
                        )
                    }
                }
            }

            when {
                state.loading -> SkeletonLoadingList(count = 4, modifier = Modifier.weight(1f))
                state.error != null && state.claims.isEmpty() && state.batches.isEmpty() ->
                    ErrorView(state.error ?: "", onRetry = vm::refresh, modifier = Modifier.weight(1f))
                else -> PullToRefreshBox(
                    isRefreshing = state.refreshing,
                    onRefresh = vm::refresh,
                    modifier = Modifier.weight(1f),
                ) {
                    AdaptiveContentFrame(
                        maxWidth = 920.dp,
                        compactHorizontalPadding = 0.dp,
                        modifier = Modifier.fillMaxSize(),
                    ) {
                        LazyColumn(
                            Modifier
                                .fillMaxSize()
                                .padding(horizontal = Spacing.sm),
                            contentPadding = PaddingValues(vertical = Spacing.xs),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                        if (state.tab == "batches") {
                            if (me?.can("admin_reimb_pay", "update") == true) {
                                item(key = "batch_creator") {
                                    val monthFormatter = remember { DateTimeFormatter.ofPattern("MMM yyyy", Locale.getDefault()) }
                                    OutlinedCard(Modifier.fillMaxWidth()) {
                                        Column(
                                            Modifier.padding(Spacing.sm),
                                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                        ) {
                                            Text(
                                                stringResource(R.string.reimb_batch_month),
                                                style = MaterialTheme.typography.labelMedium,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                            )
                                            Row(
                                                Modifier.fillMaxWidth(),
                                                verticalAlignment = Alignment.CenterVertically,
                                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                            ) {
                                                IconButton(
                                                    onClick = {
                                                        UiHaptics.tick(view)
                                                        batchMonth = batchMonth.minusMonths(1)
                                                    },
                                                    modifier = Modifier.defaultMinSize(minHeight = 48.dp, minWidth = 48.dp),
                                                ) {
                                                    Icon(Icons.Outlined.ChevronLeft, contentDescription = stringResource(R.string.common_prev))
                                                }
                                                Text(
                                                    batchMonth.format(monthFormatter),
                                                    style = MaterialTheme.typography.titleMedium.tabular(),
                                                    fontWeight = FontWeight.SemiBold,
                                                    textAlign = TextAlign.Center,
                                                    modifier = Modifier.weight(1f),
                                                )
                                                IconButton(
                                                    onClick = {
                                                        UiHaptics.tick(view)
                                                        batchMonth = batchMonth.plusMonths(1)
                                                    },
                                                    modifier = Modifier.defaultMinSize(minHeight = 48.dp, minWidth = 48.dp),
                                                ) {
                                                    Icon(Icons.Outlined.ChevronRight, contentDescription = stringResource(R.string.common_next))
                                                }
                                                Button(
                                                    onClick = {
                                                        UiHaptics.confirm(view)
                                                        vm.createBatch(batchMonth.toString())
                                                    },
                                                    enabled = !state.busy,
                                                    colors = graceRidePrimaryButtonColors(),
                                                    modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                                                ) {
                                                    Text(stringResource(R.string.reimb_batch_create))
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if (state.batches.isEmpty()) {
                                item {
                                    EmptyView(text = stringResource(R.string.reimb_batches_empty), icon = Icons.AutoMirrored.Outlined.ReceiptLong)
                                }
                            } else {
                                items(state.batches, key = { it.id }) { batch ->
                                    PayoutBatchCard(
                                        batch = batch,
                                        canPay = me?.can("admin_reimb_pay", "update") == true,
                                        busy = state.busy,
                                        onCsv = { vm.downloadBatchCsv(batch) },
                                        onCancel = { vm.cancelBatch(batch.id) },
                                        onPay = {
                                            UiHaptics.tick(view)
                                            payingBatch = batch
                                        },
                                    )
                                }
                            }
                        } else {
                            if (state.claims.isEmpty()) {
                                item {
                                    EmptyView(text = stringResource(R.string.reimb_empty), icon = Icons.AutoMirrored.Outlined.ReceiptLong)
                                }
                            } else {
                                items(state.claims, key = { it.id }) { claim ->
                                    ReimbursementCard(
                                        item = claim,
                                        showClaimant = true,
                                        onClick = {
                                            UiHaptics.tick(view)
                                            when {
                                                claim.status != "paid" && state.tab == "review" && me?.can("admin_reimb_review", "update") == true ->
                                                    reviewingClaim = claim
                                                state.tab == "payout" && claim.status == "approved" && me?.can("admin_reimb_pay", "update") == true ->
                                                    payingClaim = claim
                                                else -> nav.navigate(AppRoute.ReimbursementDetail(claim.id))
                                            }
                                        },
                                        modifier = Modifier.animateItem(),
                                    )
                                }
                            }
                            if (state.totalPages > 1) {
                                item(key = "admin_reimb_pager") {
                                    Row(
                                        Modifier.fillMaxWidth().padding(vertical = Spacing.xs),
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        OutlinedButton(
                                            onClick = {
                                                UiHaptics.tick(view)
                                                vm.loadPage(state.page - 1)
                                            },
                                            enabled = state.page > 1 && !state.refreshing,
                                            modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                                        ) {
                                            Text(stringResource(R.string.reimb_page_prev))
                                        }
                                        Text("${state.page} / ${state.totalPages}", style = MaterialTheme.typography.labelMedium.tabular())
                                        OutlinedButton(
                                            onClick = {
                                                UiHaptics.tick(view)
                                                vm.loadPage(state.page + 1)
                                            },
                                            enabled = state.page < state.totalPages && !state.refreshing,
                                            modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                                        ) {
                                            Text(stringResource(R.string.reimb_page_next))
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

    reviewingClaim?.let { r ->
        var note by remember(r.id) { mutableStateOf("") }
        val reviewSheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
        val quickReasons = listOf(
            stringResource(R.string.reimb_quick_blurry),
            stringResource(R.string.reimb_quick_amount),
            stringResource(R.string.reimb_quick_missing_info),
        )
        ModalBottomSheet(
            onDismissRequest = { reviewingClaim = null },
            sheetState = reviewSheetState,
        ) {
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
                Row(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        "${r.user.name} · ${Money.rm(r.amountCents)}",
                        style = MaterialTheme.typography.titleMedium.tabular(),
                        fontWeight = FontWeight.SemiBold,
                        modifier = Modifier.weight(1f),
                    )
                    StatusPill(
                        text = reimbStatusLabel(r.status),
                        kind = reimbStatusKind(r.status),
                    )
                }
                r.receiptFileId?.let { rfid ->
                    OutlinedCard(
                        onClick = { zoomedReceiptId = rfid },
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Column(Modifier.padding(Spacing.xs), horizontalAlignment = Alignment.CenterHorizontally) {
                            AsyncImage(
                                model = container.fileUrl(rfid),
                                contentDescription = stringResource(R.string.reimb_receipt),
                                contentScale = ContentScale.Fit,
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .height(180.dp)
                                    .clip(RoundedCornerShape(10.dp)),
                            )
                            Text(
                                stringResource(R.string.reimb_tap_zoom_receipt),
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }
                if (r.description.isNotBlank()) {
                    Text(r.description, style = MaterialTheme.typography.bodySmall)
                }
                FlowRow(
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                ) {
                    quickReasons.forEach { reason ->
                        AssistChip(
                            onClick = {
                                UiHaptics.tick(view)
                                note = if (note.isBlank()) reason else "${note.trimEnd()}; $reason"
                            },
                            label = { Text(reason) },
                        )
                    }
                }
                OutlinedTextField(
                    value = note,
                    onValueChange = { note = it },
                    label = { Text(stringResource(R.string.reimb_review_note)) },
                    minLines = 2,
                    modifier = Modifier.fillMaxWidth(),
                )
                Button(
                    onClick = {
                        UiHaptics.confirm(view)
                        vm.review(r.id, "approve", note)
                        reviewingClaim = null
                    },
                    colors = graceRidePrimaryButtonColors(),
                    modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
                ) {
                    Text(stringResource(R.string.reimb_approve))
                }
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    OutlinedButton(
                        onClick = {
                            UiHaptics.tick(view)
                            vm.review(r.id, "return", note)
                            reviewingClaim = null
                        },
                        enabled = note.trim().isNotEmpty(),
                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                    ) {
                        Text(stringResource(R.string.reimb_return))
                    }
                    OutlinedButton(
                        onClick = {
                            UiHaptics.tick(view)
                            vm.review(r.id, "reject", note)
                            reviewingClaim = null
                        },
                        enabled = note.trim().isNotEmpty(),
                        colors = ButtonDefaults.outlinedButtonColors(contentColor = MaterialTheme.colorScheme.error),
                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                    ) {
                        Text(stringResource(R.string.reimb_reject))
                    }
                }
            }
        }
    }

    payingClaim?.let { r ->
        PaySheet(
            title = "${r.payeeName ?: r.user.name} · ${Money.rm(r.amountCents)}",
            subtitle = r.payeePhone,
            showCopyPhone = !r.payeePhone.isNullOrBlank(),
            onCopied = { scope.launch { snackbar.showSnackbar(copiedMsg) } },
            onDismiss = { payingClaim = null },
            onConfirm = { verified, paidRef ->
                vm.pay(r.id, verified, paidRef)
                payingClaim = null
            },
        )
    }

    payingBatch?.let { b ->
        PaySheet(
            title = "${b.title} · ${Money.rm(b.totalCents)}",
            subtitle = b.month,
            showCopyPhone = false,
            onCopied = {},
            onDismiss = { payingBatch = null },
            onConfirm = { verified, paidRef ->
                vm.payBatch(b.id, verified, paidRef)
                payingBatch = null
            },
        )
    }

    zoomedReceiptId?.let { fid ->
        ZoomableImageDialog(
            model = container.fileUrl(fid),
            title = stringResource(R.string.reimb_receipt),
            onDismiss = { zoomedReceiptId = null },
        )
    }
}

@Composable
private fun PayoutBatchCard(
    batch: PayoutBatchDto,
    canPay: Boolean,
    busy: Boolean,
    onCsv: () -> Unit,
    onCancel: () -> Unit,
    onPay: () -> Unit,
) {
    val isPaid = batch.status == "paid"
    OutlinedCard(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text(batch.title, style = MaterialTheme.typography.titleSmall)
                    Text(
                        "${batch.month} · ${batch.createdBy} (${batch.claimCount})",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                StatusPill(
                    text = stringResource(if (isPaid) R.string.reimb_batch_paid else R.string.reimb_batch_open),
                    kind = if (isPaid) RideStatusKind.DONE else RideStatusKind.PENDING,
                )
            }
            Text(Money.rm(batch.totalCents), style = MaterialTheme.typography.titleMedium.tabular(), fontWeight = FontWeight.Bold)
            if (isPaid && batch.paidRef.isNotBlank()) {
                Text("${stringResource(R.string.reimb_paid_ref)}: ${batch.paidRef}", style = MaterialTheme.typography.bodySmall)
            }
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                OutlinedButton(onClick = onCsv, enabled = !busy, modifier = Modifier.defaultMinSize(minHeight = 48.dp)) {
                    Icon(Icons.Outlined.Download, contentDescription = null, modifier = Modifier.size(16.dp))
                    Spacer(Modifier.width(Spacing.xxs))
                    Text(stringResource(R.string.common_export_csv))
                }
                if (!isPaid && canPay) {
                    OutlinedButton(onClick = onCancel, enabled = !busy, modifier = Modifier.defaultMinSize(minHeight = 48.dp)) {
                        Text(stringResource(R.string.common_cancel))
                    }
                    Button(
                        onClick = onPay,
                        enabled = !busy,
                        colors = graceRidePrimaryButtonColors(),
                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                    ) {
                        Text(stringResource(R.string.reimb_mark_paid))
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun PaySheet(
    title: String,
    subtitle: String?,
    showCopyPhone: Boolean,
    onCopied: () -> Unit,
    onDismiss: () -> Unit,
    onConfirm: (Boolean, String) -> Unit,
) {
    SecureScreenEffect()
    val context = LocalContext.current
    val view = LocalView.current
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    var verified by remember { mutableStateOf(false) }
    var paidRef by remember { mutableStateOf("") }

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
            Text(title, style = MaterialTheme.typography.titleMedium.tabular(), fontWeight = FontWeight.SemiBold)
            subtitle?.takeIf { it.isNotBlank() }?.let { sub ->
                Row(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        sub,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.weight(1f),
                    )
                    if (showCopyPhone) {
                        IconButton(
                            onClick = {
                                copyToClipboard(context, title, sub)
                                UiHaptics.confirm(view)
                                onCopied()
                            },
                            modifier = Modifier.defaultMinSize(minHeight = 48.dp, minWidth = 48.dp),
                        ) {
                            Icon(Icons.Outlined.ContentCopy, contentDescription = stringResource(R.string.common_copied))
                        }
                    }
                }
            }
            Row(
                Modifier
                    .fillMaxWidth()
                    .defaultMinSize(minHeight = 48.dp)
                    .toggleable(
                        value = verified,
                        role = Role.Checkbox,
                        onValueChange = {
                            UiHaptics.tick(view)
                            verified = it
                        },
                    ),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Checkbox(checked = verified, onCheckedChange = null)
                Text(stringResource(R.string.reimb_payee_verified), modifier = Modifier.padding(start = Spacing.xs))
            }
            OutlinedTextField(
                value = paidRef,
                onValueChange = { paidRef = it },
                label = { Text(stringResource(R.string.reimb_paid_ref)) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            Button(
                onClick = {
                    UiHaptics.confirm(view)
                    onConfirm(verified, paidRef.trim())
                },
                enabled = verified && paidRef.trim().length >= 4,
                colors = graceRidePrimaryButtonColors(),
                modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
            ) {
                Text(stringResource(R.string.reimb_mark_paid))
            }
            OutlinedButton(
                onClick = onDismiss,
                modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
            ) {
                Text(stringResource(R.string.common_cancel))
            }
        }
    }
}
