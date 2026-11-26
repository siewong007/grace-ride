package org.pjstatecm.graceride.ui.screens.settlements

import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
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
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.CheckCircle
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.DeleteOutline
import androidx.compose.material.icons.outlined.Edit
import androidx.compose.material.icons.outlined.History
import androidx.compose.material.icons.outlined.Lock
import androidx.compose.material.icons.outlined.Publish
import androidx.compose.material.icons.outlined.Receipt
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
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
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import coil.compose.AsyncImage
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.SettlementShareDto
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.SecureScreenEffect
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.DetailSkeleton
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.components.ZoomableImageDialog
import org.pjstatecm.graceride.ui.copyToClipboard
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun SettlementDetailScreen(
    container: AppContainer,
    nav: NavHostController,
    id: Long,
) {
    val vm: SettlementDetailViewModel = viewModel(
        key = "settlement-$id",
        factory = SettlementDetailViewModel.factory(container, id),
    )
    SecureScreenEffect()
    val state by vm.state.collectAsStateWithLifecycle()
    val snackbar = remember { SnackbarHostState() }
    val lifecycleOwner = LocalLifecycleOwner.current
    val context = LocalContext.current
    val view = LocalView.current
    val scope = rememberCoroutineScope()
    val copiedMsg = stringResource(R.string.common_copied)
    val defaultReceiptTitle = stringResource(R.string.settlement_receipt_preview)
    val qrZoomTitle = stringResource(R.string.settlement_qr_preview)
    val proofZoomTitle = stringResource(R.string.settlement_view_proof)

    var verifyingShare by remember { mutableStateOf<SettlementShareDto?>(null) }
    var zoomedImageFileId by remember { mutableStateOf<Long?>(null) }
    var zoomedImageTitle by remember { mutableStateOf(defaultReceiptTitle) }
    var shareFilter by remember { mutableStateOf("all") }
    var showRevisions by remember { mutableStateOf(false) }
    var confirmDelete by remember { mutableStateOf(false) }
    var confirmClose by remember { mutableStateOf(false) }

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

    LaunchedEffect(state.deleted) {
        if (state.deleted) nav.popBackStack()
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
                title = { Text(state.detail?.title ?: stringResource(R.string.settlements_detail_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
                actions = {
                    state.detail?.let { d ->
                        if (d.version > 1) {
                            IconButton(
                                onClick = {
                                    UiHaptics.tick(view)
                                    vm.loadRevisions()
                                    showRevisions = true
                                },
                            ) {
                                Icon(Icons.Outlined.History, contentDescription = stringResource(R.string.settlement_revisions))
                            }
                        }
                        if (d.canManage && d.status != "closed") {
                            IconButton(
                                onClick = {
                                    UiHaptics.tick(view)
                                    nav.navigate(AppRoute.SettlementForm(settlementId = d.id))
                                },
                            ) {
                                Icon(Icons.Outlined.Edit, contentDescription = stringResource(R.string.settlement_edit))
                            }
                        }
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        val mode = when {
            state.loading -> "loading"
            state.detail == null -> "error"
            else -> "content"
        }
        Crossfade(
            targetState = mode,
            animationSpec = tween(200),
            label = "settlement_detail_state",
            modifier = Modifier.padding(padding),
        ) { current ->
            when (current) {
                "loading" -> DetailSkeleton()
                "error" -> ErrorView(state.error ?: "", onRetry = vm::load)
                else -> {
                    val d = state.detail ?: return@Crossfade
                    val statusColors = LocalStatusColors.current
                    val confirmedCount = d.shares.count { it.status == "confirmed" }
                    val needsReviewCount = d.shares.count { it.status == "pending" || it.status == "needs_info" }
                    val unpaidCount = (d.shares.size - confirmedCount - needsReviewCount).coerceAtLeast(0)
                    val visibleShares = remember(d.shares, shareFilter) {
                        when (shareFilter) {
                            "needs_review" -> d.shares.filter { it.status == "pending" || it.status == "needs_info" }
                            "unpaid" -> d.shares.filter { it.status != "confirmed" && it.status != "pending" && it.status != "needs_info" }
                            "confirmed" -> d.shares.filter { it.status == "confirmed" }
                            else -> d.shares
                        }
                    }

                    PullToRefreshBox(
                        isRefreshing = state.refreshing,
                        onRefresh = vm::refresh,
                        modifier = Modifier.fillMaxSize(),
                    ) {
                        Column(
                            Modifier
                                .fillMaxSize()
                                .verticalScroll(rememberScrollState())
                                .navigationBarsPadding()
                                .padding(Spacing.sm),
                            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                        ) {
                            OutlinedCard(Modifier.fillMaxWidth()) {
                                Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                    Row(
                                        Modifier.fillMaxWidth(),
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        Text(
                                            d.event.title,
                                            style = MaterialTheme.typography.titleMedium,
                                            modifier = Modifier.weight(1f),
                                        )
                                        StatusPill(
                                            text = settlementStatusLabel(d.status),
                                            kind = settlementStatusPillKind(d.status),
                                        )
                                    }
                                    if (d.event.startsAt.isNotBlank()) {
                                        Text(
                                            KlTime.display(d.event.startsAt),
                                            style = MaterialTheme.typography.bodySmall,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                    }
                                    d.dueDate?.takeIf { it.isNotBlank() }?.let { due ->
                                        Text(
                                            stringResource(R.string.settlement_due_date, KlTime.displayDate(due)),
                                            style = MaterialTheme.typography.bodySmall,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                    }
                                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                    MoneyStatRow(stringResource(R.string.settlement_gross), d.grossCents)
                                    if (d.subsidyCents > 0) {
                                        MoneyStatRow(stringResource(R.string.settlement_subsidy), d.subsidyCents)
                                    }
                                    MoneyStatRow(stringResource(R.string.settlement_net_total), d.totalCents, bold = true)

                                    Box(
                                        Modifier
                                            .fillMaxWidth()
                                            .height(10.dp)
                                            .clip(RoundedCornerShape(5.dp))
                                            .background(MaterialTheme.colorScheme.surfaceContainerHighest),
                                    ) {
                                        if (d.shares.isNotEmpty()) {
                                            Row(Modifier.fillMaxSize()) {
                                                if (confirmedCount > 0) {
                                                    Box(
                                                        Modifier
                                                            .weight(confirmedCount.toFloat())
                                                            .fillMaxSize()
                                                            .background(statusColors.success),
                                                    )
                                                }
                                                if (needsReviewCount > 0) {
                                                    Box(
                                                        Modifier
                                                            .weight(needsReviewCount.toFloat())
                                                            .fillMaxSize()
                                                            .background(statusColors.warning),
                                                    )
                                                }
                                                if (unpaidCount > 0) {
                                                    Box(
                                                        Modifier
                                                            .weight(unpaidCount.toFloat())
                                                            .fillMaxSize()
                                                            .background(MaterialTheme.colorScheme.outlineVariant),
                                                    )
                                                }
                                            }
                                        }
                                    }
                                    FlowRow(
                                        horizontalArrangement = Arrangement.spacedBy(Spacing.sm),
                                        verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                                    ) {
                                        Text(
                                            stringResource(R.string.settlements_confirmed_ratio, confirmedCount.toLong(), d.shares.size.toLong()),
                                            style = MaterialTheme.typography.labelSmall.tabular(),
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                        LegendDot(
                                            color = statusColors.success,
                                            label = "${stringResource(R.string.settlement_filter_confirmed)}: $confirmedCount",
                                        )
                                        if (needsReviewCount > 0) {
                                            LegendDot(
                                                color = statusColors.warning,
                                                label = "${stringResource(R.string.settlement_filter_needs_review)}: $needsReviewCount",
                                            )
                                        }
                                        if (unpaidCount > 0) {
                                            LegendDot(
                                                color = MaterialTheme.colorScheme.outline,
                                                label = "${stringResource(R.string.settlement_filter_unpaid)}: $unpaidCount",
                                            )
                                        }
                                    }
                                }
                            }

                            OutlinedCard(Modifier.fillMaxWidth()) {
                                Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                    Row(
                                        Modifier.fillMaxWidth(),
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                            SectionHeader(stringResource(R.string.settlement_payee_section))
                                            Text("${d.payeeName} · ${d.payeeAccount}", style = MaterialTheme.typography.bodyMedium)
                                        }
                                        if (d.payeeAccount.isNotBlank()) {
                                            OutlinedButton(
                                                onClick = {
                                                    copyToClipboard(context, d.payeeName.ifBlank { "Payee" }, d.payeeAccount, sensitive = true)
                                                    UiHaptics.confirm(view)
                                                    scope.launch { snackbar.showSnackbar(copiedMsg) }
                                                },
                                                modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                                            ) {
                                                Icon(Icons.Outlined.ContentCopy, contentDescription = null, modifier = Modifier.size(16.dp))
                                                Spacer(Modifier.width(Spacing.xxs))
                                                Text(stringResource(R.string.settlement_copy_account))
                                            }
                                        }
                                    }
                                    d.payeeQrFileId?.let { qrId ->
                                        OutlinedCard(
                                            onClick = {
                                                zoomedImageTitle = qrZoomTitle
                                                zoomedImageFileId = qrId
                                            },
                                            modifier = Modifier.fillMaxWidth(),
                                        ) {
                                            Column(Modifier.padding(Spacing.xs), horizontalAlignment = Alignment.CenterHorizontally) {
                                                AsyncImage(
                                                    model = container.fileUrl(qrId),
                                                    contentDescription = qrZoomTitle,
                                                    contentScale = ContentScale.Fit,
                                                    modifier = Modifier
                                                        .fillMaxWidth()
                                                        .height(160.dp),
                                                )
                                                Text(
                                                    stringResource(R.string.settlement_tap_zoom_proof),
                                                    style = MaterialTheme.typography.labelSmall,
                                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                )
                                            }
                                        }
                                    }
                                }
                            }

                            SectionHeader(stringResource(R.string.settlement_items_section))
                            OutlinedCard(Modifier.fillMaxWidth()) {
                                Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                    d.items.forEachIndexed { idx, item ->
                                        if (idx > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                        Row(
                                            Modifier.fillMaxWidth(),
                                            horizontalArrangement = Arrangement.SpaceBetween,
                                            verticalAlignment = Alignment.CenterVertically,
                                        ) {
                                            Column(Modifier.weight(1f)) {
                                                Text(item.label, style = MaterialTheme.typography.bodyMedium)
                                                item.receiptFileId?.let { rfid ->
                                                    TextButton(
                                                        onClick = {
                                                            zoomedImageTitle = item.label.ifBlank { defaultReceiptTitle }
                                                            zoomedImageFileId = rfid
                                                        },
                                                    ) {
                                                        Icon(Icons.Outlined.Receipt, contentDescription = null, modifier = Modifier.size(16.dp))
                                                        Spacer(Modifier.width(Spacing.xxs))
                                                        Text(stringResource(R.string.settlement_view_receipt))
                                                    }
                                                }
                                            }
                                            Text(Money.rm(item.amountCents), style = MaterialTheme.typography.bodyMedium.tabular())
                                        }
                                    }
                                }
                            }

                            SectionHeader(stringResource(R.string.settlement_shares_section))
                            if (d.shares.isNotEmpty()) {
                                val filterChips = listOf(
                                    "all" to "${stringResource(R.string.settlement_filter_all)} (${d.shares.size})",
                                    "needs_review" to "${stringResource(R.string.settlement_filter_needs_review)} ($needsReviewCount)",
                                    "unpaid" to "${stringResource(R.string.settlement_filter_unpaid)} ($unpaidCount)",
                                    "confirmed" to "${stringResource(R.string.settlement_filter_confirmed)} ($confirmedCount)",
                                )
                                LazyRow(
                                    contentPadding = PaddingValues(vertical = Spacing.xxs),
                                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                ) {
                                    items(filterChips, key = { it.first }) { (key, label) ->
                                        FilterChip(
                                            selected = shareFilter == key,
                                            onClick = {
                                                UiHaptics.tick(view)
                                                shareFilter = key
                                            },
                                            label = { Text(label, style = MaterialTheme.typography.labelMedium.tabular()) },
                                        )
                                    }
                                }
                            }

                            visibleShares.forEach { sh ->
                                OutlinedCard(Modifier.fillMaxWidth()) {
                                    Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                        Row(
                                            Modifier.fillMaxWidth(),
                                            horizontalArrangement = Arrangement.SpaceBetween,
                                            verticalAlignment = Alignment.CenterVertically,
                                        ) {
                                            Column(Modifier.weight(1f)) {
                                                Text(sh.name, style = MaterialTheme.typography.titleSmall)
                                                Text(
                                                    Money.rm(sh.amountCents),
                                                    style = MaterialTheme.typography.bodyMedium.tabular(),
                                                    fontWeight = FontWeight.SemiBold,
                                                )
                                            }
                                            StatusPill(
                                                text = settlementStatusLabel(sh.status),
                                                kind = settlementStatusPillKind(sh.status),
                                            )
                                        }
                                        if (sh.txnRef.isNotBlank()) {
                                            Text(
                                                "${stringResource(R.string.costs_txn_ref)}: ${sh.txnRef}",
                                                style = MaterialTheme.typography.bodySmall,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                            )
                                        }
                                        if (sh.note.isNotBlank()) {
                                            Text(
                                                sh.note,
                                                style = MaterialTheme.typography.bodySmall,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                            )
                                        }
                                        Row(
                                            Modifier.fillMaxWidth(),
                                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                        ) {
                                            sh.proofFileId?.let { pfid ->
                                                OutlinedButton(
                                                    onClick = {
                                                        zoomedImageTitle = "${sh.name} · $proofZoomTitle"
                                                        zoomedImageFileId = pfid
                                                    },
                                                    modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                                                ) {
                                                    Text(proofZoomTitle)
                                                }
                                            }
                                            if (d.canManage && (sh.status == "pending" || sh.proofFileId != null || sh.status == "confirmed" || sh.status == "needs_info")) {
                                                Button(
                                                    onClick = {
                                                        UiHaptics.tick(view)
                                                        verifyingShare = sh
                                                    },
                                                    colors = graceRidePrimaryButtonColors(),
                                                    modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                                                ) {
                                                    Icon(Icons.Outlined.CheckCircle, contentDescription = null, modifier = Modifier.size(16.dp))
                                                    Spacer(Modifier.width(Spacing.xxs))
                                                    Text(stringResource(R.string.settlement_verify_share))
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if (d.canManage) {
                                Spacer(Modifier.height(Spacing.xs))
                                when (d.status) {
                                    "draft" -> {
                                        Button(
                                            onClick = {
                                                UiHaptics.confirm(view)
                                                vm.publish()
                                            },
                                            enabled = !state.busy,
                                            colors = graceRidePrimaryButtonColors(),
                                            modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
                                        ) {
                                            if (state.busy) {
                                                InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                            } else {
                                                Icon(Icons.Outlined.Publish, contentDescription = null, modifier = Modifier.size(18.dp))
                                                Spacer(Modifier.width(Spacing.xs))
                                                Text(stringResource(R.string.settlement_publish))
                                            }
                                        }
                                        OutlinedButton(
                                            onClick = { confirmDelete = true },
                                            enabled = !state.busy,
                                            colors = ButtonDefaults.outlinedButtonColors(contentColor = MaterialTheme.colorScheme.error),
                                            modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
                                        ) {
                                            Icon(Icons.Outlined.DeleteOutline, contentDescription = null, modifier = Modifier.size(18.dp))
                                            Spacer(Modifier.width(Spacing.xs))
                                            Text(stringResource(R.string.settlement_delete_draft))
                                        }
                                    }
                                    "published" -> {
                                        OutlinedButton(
                                            onClick = { confirmClose = true },
                                            enabled = !state.busy,
                                            modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
                                        ) {
                                            Icon(Icons.Outlined.Lock, contentDescription = null, modifier = Modifier.size(18.dp))
                                            Spacer(Modifier.width(Spacing.xs))
                                            Text(stringResource(R.string.settlement_close))
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

    verifyingShare?.let { sh ->
        var note by remember(sh.id) { mutableStateOf(sh.note) }
        val verifySheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
        ModalBottomSheet(
            onDismissRequest = { verifyingShare = null },
            sheetState = verifySheetState,
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
                        "${sh.name} · ${Money.rm(sh.amountCents)}",
                        style = MaterialTheme.typography.titleMedium.tabular(),
                        fontWeight = FontWeight.SemiBold,
                        modifier = Modifier.weight(1f),
                    )
                    StatusPill(
                        text = settlementStatusLabel(sh.status),
                        kind = settlementStatusPillKind(sh.status),
                    )
                }
                if (sh.txnRef.isNotBlank()) {
                    Text(
                        "${stringResource(R.string.costs_txn_ref)}: ${sh.txnRef}",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                sh.proofFileId?.let { pfid ->
                    OutlinedCard(
                        onClick = {
                            zoomedImageTitle = "${sh.name} · $proofZoomTitle"
                            zoomedImageFileId = pfid
                        },
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Column(Modifier.padding(Spacing.xs), horizontalAlignment = Alignment.CenterHorizontally) {
                            AsyncImage(
                                model = container.fileUrl(pfid),
                                contentDescription = proofZoomTitle,
                                contentScale = ContentScale.Fit,
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .height(180.dp)
                                    .clip(RoundedCornerShape(8.dp)),
                            )
                            Text(
                                stringResource(R.string.settlement_tap_zoom_proof),
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }
                OutlinedTextField(
                    value = note,
                    onValueChange = { note = it },
                    label = { Text(stringResource(R.string.settlement_verify_note)) },
                    minLines = 2,
                    modifier = Modifier.fillMaxWidth(),
                )
                Button(
                    onClick = {
                        UiHaptics.confirm(view)
                        vm.verifyShare(sh.id, "confirm", note)
                        verifyingShare = null
                    },
                    colors = graceRidePrimaryButtonColors(),
                    modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
                ) {
                    Icon(Icons.Outlined.CheckCircle, contentDescription = null, modifier = Modifier.size(18.dp))
                    Spacer(Modifier.width(Spacing.xs))
                    Text(stringResource(R.string.settlement_verify_confirm))
                }
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    OutlinedButton(
                        onClick = {
                            UiHaptics.tick(view)
                            vm.verifyShare(sh.id, "needs_info", note)
                            verifyingShare = null
                        },
                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                    ) {
                        Text(stringResource(R.string.settlement_verify_needs_info))
                    }
                    OutlinedButton(
                        onClick = {
                            UiHaptics.tick(view)
                            vm.verifyShare(sh.id, "reset", note)
                            verifyingShare = null
                        },
                        colors = ButtonDefaults.outlinedButtonColors(contentColor = MaterialTheme.colorScheme.error),
                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                    ) {
                        Text(stringResource(R.string.settlement_verify_reset))
                    }
                }
            }
        }
    }

    zoomedImageFileId?.let { fid ->
        ZoomableImageDialog(
            model = container.fileUrl(fid),
            title = zoomedImageTitle,
            onDismiss = { zoomedImageFileId = null },
        )
    }

    if (showRevisions) {
        val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
        ModalBottomSheet(onDismissRequest = { showRevisions = false }, sheetState = sheetState) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .navigationBarsPadding()
                    .padding(horizontal = Spacing.sm)
                    .padding(bottom = Spacing.md),
                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                Text(stringResource(R.string.settlement_revisions), style = MaterialTheme.typography.titleMedium)
                LazyColumn(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    items(state.revisions, key = { it.version }) { rev ->
                        OutlinedCard(Modifier.fillMaxWidth()) {
                            Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                                    Text("v${rev.version} · ${rev.by}", style = MaterialTheme.typography.titleSmall)
                                    Text(KlTime.display(rev.createdAt), style = MaterialTheme.typography.labelSmall)
                                }
                                if (rev.reason.isNotBlank()) {
                                    Text(rev.reason, style = MaterialTheme.typography.bodySmall)
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if (confirmDelete) {
        AlertDialog(
            onDismissRequest = { confirmDelete = false },
            title = { Text(stringResource(R.string.settlement_delete_draft)) },
            text = { Text(stringResource(R.string.settlement_delete_confirm)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmDelete = false
                        vm.deleteBill()
                    },
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                ) {
                    Text(stringResource(R.string.common_confirm))
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmDelete = false }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }

    if (confirmClose) {
        AlertDialog(
            onDismissRequest = { confirmClose = false },
            title = { Text(stringResource(R.string.settlement_close)) },
            text = { Text(stringResource(R.string.settlement_close_confirm)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmClose = false
                        vm.closeBill()
                    },
                ) {
                    Text(stringResource(R.string.common_confirm))
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmClose = false }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }
}

@Composable
private fun LegendDot(color: Color, label: String) {
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        Box(Modifier.size(8.dp).clip(CircleShape).background(color))
        Text(label, style = MaterialTheme.typography.labelSmall.tabular(), color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
private fun MoneyStatRow(label: String, cents: Long, bold: Boolean = false) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        Text(
            label,
            style = if (bold) MaterialTheme.typography.titleSmall else MaterialTheme.typography.bodySmall,
            color = if (bold) MaterialTheme.colorScheme.onSurface else MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            Money.rm(cents),
            style = (if (bold) MaterialTheme.typography.titleSmall else MaterialTheme.typography.bodySmall).tabular(),
            fontWeight = if (bold) FontWeight.Bold else FontWeight.Medium,
        )
    }
}
