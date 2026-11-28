package org.pjstatecm.graceride.ui.screens.reports

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.Download
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FilterChip
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
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import java.util.Locale
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.Badge
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.platform.CsvBuilder
import org.pjstatecm.graceride.platform.ExportShare
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.AdaptiveContentFrame
import org.pjstatecm.graceride.ui.components.BadgeIcon
import org.pjstatecm.graceride.ui.components.BadgeIcons
import org.pjstatecm.graceride.ui.components.DetailSkeleton
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.tabular

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun AdminReportsScreen(
    container: AppContainer,
    nav: NavHostController,
    initialTab: String? = null,
) {
    val vm: AdminReportsViewModel = viewModel(
        key = "admin-reports-${initialTab ?: "member"}",
        factory = AdminReportsViewModel.factory(container, initialTab),
    )
    val state by vm.state.collectAsStateWithLifecycle()
    val snackbar = remember { SnackbarHostState() }
    val context = LocalContext.current
    val view = LocalView.current
    val scope = rememberCoroutineScope()
    val isZh = Locale.getDefault().language == "zh"

    var memberExpanded by remember { mutableStateOf(false) }
    var editingBadge by remember { mutableStateOf<Badge?>(null) }

    LaunchedEffect(state.message) {
        state.message?.let {
            snackbar.showSnackbar(it)
            vm.clearMessage()
        }
    }

    val tabs = buildList {
        add("member" to R.string.admin_reports_tab_member)
        if (state.caps.overview) add("overview" to R.string.admin_reports_tab_overview)
        if (state.caps.events) add("events" to R.string.admin_reports_tab_events)
        if (state.caps.demand) add("demand" to R.string.admin_reports_tab_demand)
        if (state.caps.finance) add("finance" to R.string.admin_reports_tab_finance)
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.admin_reports_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
                actions = {
                    IconButton(
                        onClick = {
                            UiHaptics.tick(view)
                            scope.launch {
                                val bytes: ByteArray? = when (state.tab) {
                                    "member" -> state.memberReport?.let { buildMonthReportCsv(it, isZh) }
                                    "events" -> state.eventsReport?.let { er ->
                                        CsvBuilder.buildBytes(
                                            listOf("Event", "Kind", "Starts", "Going", "Attended"),
                                            er.events.map { listOf(it.title, it.kind, it.startsAt, it.going.toString(), it.attended.toString()) },
                                        )
                                    }
                                    "demand" -> state.demandReport?.let { dr ->
                                        CsvBuilder.buildBytes(
                                            listOf("Area", "Requests", "Fulfilled", "Unfilled", "Rate %"),
                                            dr.byArea.map {
                                                listOf(
                                                    if (isZh) it.areaNameZh else it.areaNameEn,
                                                    it.total.toString(),
                                                    it.fulfilled.toString(),
                                                    it.unfilled.toString(),
                                                    "%.1f".format(it.fulfillmentRatePct),
                                                )
                                            },
                                        )
                                    }
                                    "finance" -> state.financeReport?.let { fr ->
                                        val rows = buildList {
                                            fr.reimbursementReview?.let { rr ->
                                                add(listOf("Review pending", rr.pendingCount.toString(), Money.rm(rr.pendingCents)))
                                                add(listOf("Approved", rr.approvedCount.toString(), Money.rm(rr.approvedCents)))
                                            }
                                            fr.reimbursementPayouts?.let { rp ->
                                                add(listOf("Paid", rp.paidCount.toString(), Money.rm(rp.paidCents)))
                                            }
                                            fr.bills?.totals?.let { bt ->
                                                add(listOf("Bills collected", bt.totalSettlements.toString(), Money.rm(bt.collectedCents)))
                                            }
                                        }
                                        CsvBuilder.buildBytes(listOf("Category", "Count", "Amount"), rows)
                                    }
                                    else -> state.overview?.let { ov ->
                                        val rows = buildList {
                                            ov.transport?.let { add(listOf("Rides", it.totalRides.toString(), "${it.completedRides} completed")) }
                                            ov.events?.let { add(listOf("Events", it.eventsCount.toString(), "${it.totalAttended}/${it.totalGoing} attended")) }
                                            ov.membership?.let { add(listOf("Members", it.activeMembers.toString(), "${it.pendingMembers} pending")) }
                                        }
                                        CsvBuilder.buildBytes(listOf("Area", "Count", "Notes"), rows)
                                    }
                                }
                                if (bytes != null) {
                                    ExportShare.writeAndShareFile(
                                        context,
                                        "grace-ride-${state.tab}-${state.month}.csv",
                                        "text/csv",
                                        bytes,
                                    )
                                }
                            }
                        },
                    ) {
                        Icon(Icons.Outlined.Download, contentDescription = stringResource(R.string.common_export_csv))
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
            LazyRow(
                contentPadding = PaddingValues(horizontal = Spacing.sm, vertical = Spacing.xxs),
                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                items(tabs, key = { it.first }) { (key, res) ->
                    FilterChip(
                        selected = state.tab == key,
                        onClick = {
                            UiHaptics.tick(view)
                            vm.selectTab(key)
                        },
                        label = { Text(stringResource(res)) },
                    )
                }
            }

            PeriodSelectorBar(
                month = state.month,
                range = state.range,
                onShiftMonth = vm::shiftMonth,
                onSelectRange = vm::setRange,
            )

            when {
                state.loading -> DetailSkeleton(Modifier.weight(1f))
                state.error != null -> ErrorView(state.error ?: "", onRetry = vm::load, modifier = Modifier.weight(1f))
                else -> PullToRefreshBox(
                    isRefreshing = state.refreshing,
                    onRefresh = vm::refresh,
                    modifier = Modifier.weight(1f),
                ) {
                    val statusColors = LocalStatusColors.current
                    AdaptiveContentFrame(
                        maxWidth = 920.dp,
                        compactHorizontalPadding = 0.dp,
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
                        when (state.tab) {
                            "overview" -> state.overview?.let { ov ->
                                ov.transport?.let { t ->
                                    FulfillmentDonutCard(
                                        title = stringResource(R.string.admin_reports_transport),
                                        completed = t.completedRides,
                                        unfilled = t.unfilledRides,
                                        total = t.totalRides,
                                        pct = t.fulfillmentRatePct,
                                        extraLabel = stringResource(R.string.report_distance_km),
                                        extraValue = "${t.totalDistanceKm} km",
                                    )
                                }
                                ov.events?.let { e ->
                                    OutlinedCard(Modifier.fillMaxWidth()) {
                                        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                            SectionHeader(stringResource(R.string.report_events_section))
                                            KpiLine(stringResource(R.string.admin_reports_events_count), e.eventsCount.toString())
                                            KpiLine(stringResource(R.string.admin_reports_going_attended), "${e.totalAttended} / ${e.totalGoing}")
                                            KpiLine(stringResource(R.string.admin_reports_attendance_rate), "%.1f%%".format(e.attendanceRatePct))
                                        }
                                    }
                                }
                                ov.membership?.let { m ->
                                    OutlinedCard(Modifier.fillMaxWidth()) {
                                        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                            SectionHeader(stringResource(R.string.admin_reports_membership))
                                            KpiLine(stringResource(R.string.admin_reports_active_members), m.activeMembers.toString())
                                            KpiLine(stringResource(R.string.admin_reports_pending_members), m.pendingMembers.toString())
                                            KpiLine(stringResource(R.string.admin_reports_active_drivers), m.activeDriversTotal.toString())
                                        }
                                    }
                                }
                            }
                            "events" -> state.eventsReport?.let { er ->
                                OutlinedCard(Modifier.fillMaxWidth()) {
                                    Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                        KpiLine(stringResource(R.string.admin_reports_events_count), er.totals.eventsCount.toString())
                                        KpiLine(stringResource(R.string.admin_reports_going_attended), "${er.totals.totalAttended} / ${er.totals.totalGoing}")
                                        KpiLine(stringResource(R.string.admin_reports_attendance_rate), "%.1f%%".format(er.totals.attendanceRatePct))
                                        KpiLine(stringResource(R.string.admin_reports_need_ride), er.totals.needRideCount.toString())
                                    }
                                }
                                er.events.forEach { ev ->
                                    OutlinedCard(Modifier.fillMaxWidth()) {
                                        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                            Text(ev.title, style = MaterialTheme.typography.titleSmall)
                                            Text(
                                                stringResource(R.string.admin_reports_going_attended_row, ev.going, ev.attended),
                                                style = MaterialTheme.typography.bodySmall.tabular(),
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                            )
                                        }
                                    }
                                }
                            }
                            "demand" -> state.demandReport?.let { dr ->
                                FulfillmentDonutCard(
                                    title = stringResource(R.string.admin_reports_tab_demand),
                                    completed = dr.totals.completed,
                                    unfilled = dr.totals.unfilled,
                                    total = dr.totals.totalRequests,
                                    pct = dr.totals.fulfillmentRatePct,
                                )
                                SectionHeader(stringResource(R.string.admin_reports_by_area))
                                dr.byArea.forEach { row ->
                                    val name = if (isZh) row.areaNameZh.ifBlank { row.areaNameEn } else row.areaNameEn.ifBlank { row.areaNameZh }
                                    val frac = (row.fulfillmentRatePct / 100.0).toFloat().coerceIn(0f, 1f)
                                    val barColor = when {
                                        row.fulfillmentRatePct >= 85.0 -> statusColors.success
                                        row.fulfillmentRatePct >= 65.0 -> statusColors.warning
                                        else -> statusColors.danger
                                    }
                                    val pillKind = when {
                                        row.fulfillmentRatePct >= 85.0 -> RideStatusKind.DONE
                                        row.fulfillmentRatePct >= 65.0 -> RideStatusKind.PENDING
                                        else -> RideStatusKind.CANCELLED
                                    }
                                    val healthRes = when {
                                        row.fulfillmentRatePct >= 85.0 -> R.string.admin_reports_health_good
                                        row.fulfillmentRatePct >= 65.0 -> R.string.admin_reports_health_watch
                                        else -> R.string.admin_reports_health_low
                                    }
                                    OutlinedCard(Modifier.fillMaxWidth()) {
                                        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                            Row(
                                                Modifier.fillMaxWidth(),
                                                horizontalArrangement = Arrangement.SpaceBetween,
                                                verticalAlignment = Alignment.CenterVertically,
                                            ) {
                                                Text(
                                                    name,
                                                    style = MaterialTheme.typography.titleSmall,
                                                    maxLines = 1,
                                                    overflow = TextOverflow.Ellipsis,
                                                    modifier = Modifier.weight(1f),
                                                )
                                                StatusPill(text = stringResource(healthRes), kind = pillKind)
                                            }
                                            KpiLine(
                                                "${row.fulfilled}/${row.total}",
                                                "${"%.0f".format(row.fulfillmentRatePct)}%",
                                            )
                                            LinearProgressIndicator(
                                                progress = { frac },
                                                color = barColor,
                                                trackColor = MaterialTheme.colorScheme.surfaceContainerHighest,
                                                modifier = Modifier
                                                    .fillMaxWidth()
                                                    .height(8.dp)
                                                    .clip(RoundedCornerShape(4.dp)),
                                            )
                                        }
                                    }
                                }
                            }
                            "finance" -> state.financeReport?.let { fr ->
                                fr.reimbursementReview?.let { rr ->
                                    OutlinedCard(Modifier.fillMaxWidth()) {
                                        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                            SectionHeader(stringResource(R.string.reimb_admin_tab_review))
                                            KpiLine(stringResource(R.string.reimb_status_pending), "${rr.pendingCount} (${Money.rm(rr.pendingCents)})")
                                            KpiLine(stringResource(R.string.reimb_status_approved), "${rr.approvedCount} (${Money.rm(rr.approvedCents)})")
                                        }
                                    }
                                }
                                fr.reimbursementPayouts?.let { rp ->
                                    OutlinedCard(Modifier.fillMaxWidth()) {
                                        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                            SectionHeader(stringResource(R.string.reimb_admin_tab_payout))
                                            KpiLine(stringResource(R.string.admin_reports_unpaid_claims), "${rp.approvedUnpaidCount} (${Money.rm(rp.approvedUnpaidCents)})")
                                            KpiLine(stringResource(R.string.reimb_status_paid), "${rp.paidCount} (${Money.rm(rp.paidCents)})")
                                        }
                                    }
                                }
                                fr.bills?.totals?.let { bt ->
                                    OutlinedCard(Modifier.fillMaxWidth()) {
                                        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                            SectionHeader(stringResource(R.string.settlements_title))
                                            KpiLine(stringResource(R.string.settlement_gross), Money.rm(bt.grossCents))
                                            KpiLine(stringResource(R.string.settlement_subsidy), Money.rm(bt.subsidyCents))
                                            KpiLine(stringResource(R.string.admin_reports_collected), Money.rm(bt.collectedCents))
                                            KpiLine(stringResource(R.string.admin_reports_collection_rate), "%.1f%%".format(bt.collectionRatePct))
                                        }
                                    }
                                }
                            }
                            else -> {
                                if (state.caps.memberReportsGlobal && state.members.isNotEmpty()) {
                                    ExposedDropdownMenuBox(expanded = memberExpanded, onExpandedChange = { memberExpanded = it }) {
                                        val chosenName = state.members.firstOrNull { it.id == state.selectedUserId }?.name
                                            ?: state.memberReport?.user?.name ?: ""
                                        OutlinedTextField(
                                            value = chosenName,
                                            onValueChange = {},
                                            readOnly = true,
                                            label = { Text(stringResource(R.string.admin_reports_select_member)) },
                                            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = memberExpanded) },
                                            modifier = Modifier.menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
                                        )
                                        ExposedDropdownMenu(expanded = memberExpanded, onDismissRequest = { memberExpanded = false }) {
                                            state.members.forEach { m ->
                                                DropdownMenuItem(
                                                    text = { Text(m.name) },
                                                    onClick = {
                                                        memberExpanded = false
                                                        vm.selectMember(m.id)
                                                    },
                                                )
                                            }
                                        }
                                    }
                                }
                                state.memberReport?.let { mr ->
                                    MonthReportBody(
                                        report = mr,
                                        isZh = isZh,
                                        onBadgeClick = if (state.caps.memberBadgeIconEdit) { b -> editingBadge = b } else null,
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

    editingBadge?.let { badge ->
        val targetUserId = state.memberReport?.user?.id ?: return@let
        val options = remember { BadgeIcons.allAssetKeys.toList().sorted() }
        val badgeSheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
        ModalBottomSheet(
            onDismissRequest = { editingBadge = null },
            sheetState = badgeSheetState,
        ) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .navigationBarsPadding()
                    .padding(horizontal = Spacing.sm)
                    .padding(bottom = Spacing.md),
                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                Text(badge.name, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
                Text(
                    stringResource(R.string.admin_reports_pick_badge_icon),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                LazyVerticalGrid(
                    columns = GridCells.Adaptive(minSize = 60.dp),
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    modifier = Modifier
                        .fillMaxWidth()
                        .weight(1f, fill = false)
                        .heightIn(max = 320.dp),
                ) {
                    items(options, key = { it }) { key ->
                        val selected = badge.icon == key
                        Surface(
                            onClick = {
                                UiHaptics.confirm(view)
                                editingBadge = null
                                vm.updateBadgeIcon(targetUserId, badge.key, key)
                            },
                            color = if (selected) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surfaceContainerLow,
                            shape = MaterialTheme.shapes.medium,
                            modifier = Modifier
                                .size(60.dp)
                                .then(
                                    if (selected) Modifier.border(2.dp, MaterialTheme.colorScheme.primary, MaterialTheme.shapes.medium)
                                    else Modifier,
                                ),
                        ) {
                            Box(contentAlignment = Alignment.Center) {
                                BadgeIcon(icon = key, size = 28.dp)
                            }
                        }
                    }
                }
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    OutlinedButton(
                        onClick = {
                            UiHaptics.tick(view)
                            editingBadge = null
                            vm.updateBadgeIcon(targetUserId, badge.key, "")
                        },
                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                    ) {
                        Text(stringResource(R.string.pin_picker_reset))
                    }
                    OutlinedButton(
                        onClick = { editingBadge = null },
                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                    ) {
                        Text(stringResource(R.string.common_cancel))
                    }
                }
            }
        }
    }
}

@Composable
private fun FulfillmentDonutCard(
    title: String,
    completed: Long,
    unfilled: Long,
    total: Long,
    pct: Double,
    extraLabel: String? = null,
    extraValue: String? = null,
) {
    val statusColors = LocalStatusColors.current
    val activeColor = when {
        pct >= 85.0 -> statusColors.success
        pct >= 65.0 -> statusColors.warning
        else -> statusColors.danger
    }
    val pillKind = when {
        pct >= 85.0 -> RideStatusKind.DONE
        pct >= 65.0 -> RideStatusKind.PENDING
        else -> RideStatusKind.CANCELLED
    }
    val healthRes = when {
        pct >= 85.0 -> R.string.admin_reports_health_good
        pct >= 65.0 -> R.string.admin_reports_health_watch
        else -> R.string.admin_reports_health_low
    }
    val trackColor = MaterialTheme.colorScheme.surfaceContainerHighest
    val sweepAngle = ((pct / 100.0).toFloat().coerceIn(0f, 1f)) * 360f

    OutlinedCard(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                SectionHeader(title)
                StatusPill(text = stringResource(healthRes), kind = pillKind)
            }
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(Spacing.md),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Box(Modifier.size(92.dp), contentAlignment = Alignment.Center) {
                    Canvas(Modifier.fillMaxSize().padding(6.dp)) {
                        val stroke = Stroke(width = 11.dp.toPx(), cap = StrokeCap.Round)
                        drawArc(
                            color = trackColor,
                            startAngle = 0f,
                            sweepAngle = 360f,
                            useCenter = false,
                            style = stroke,
                        )
                        if (sweepAngle > 0f) {
                            drawArc(
                                color = activeColor,
                                startAngle = -90f,
                                sweepAngle = sweepAngle,
                                useCenter = false,
                                style = stroke,
                            )
                        }
                    }
                    Text(
                        "${"%.0f".format(pct)}%",
                        style = MaterialTheme.typography.titleMedium.tabular(),
                        fontWeight = FontWeight.Bold,
                        color = activeColor,
                    )
                }
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
                    KpiLine(stringResource(R.string.admin_reports_total_rides), total.toString())
                    KpiLine(stringResource(R.string.admin_reports_completed), completed.toString())
                    KpiLine(stringResource(R.string.admin_reports_unfilled), unfilled.toString())
                    if (extraLabel != null && extraValue != null) {
                        KpiLine(extraLabel, extraValue)
                    }
                }
            }
        }
    }
}
