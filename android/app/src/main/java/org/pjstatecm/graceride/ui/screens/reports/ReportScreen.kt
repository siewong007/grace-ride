package org.pjstatecm.graceride.ui.screens.reports

import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.CalendarMonth
import androidx.compose.material.icons.outlined.ChevronLeft
import androidx.compose.material.icons.outlined.ChevronRight
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Download
import androidx.compose.material.icons.outlined.Group
import androidx.compose.material.icons.outlined.Place
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import java.time.YearMonth
import java.time.format.DateTimeFormatter
import java.util.Locale
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.Badge
import org.pjstatecm.graceride.data.api.MonthReportDto
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.platform.CsvBuilder
import org.pjstatecm.graceride.platform.ExportShare
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.AdaptiveContentFrame
import org.pjstatecm.graceride.ui.components.DetailSkeleton
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.components.TieredBadgeMedallion
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.tabular

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun ReportScreen(container: AppContainer, nav: NavHostController) {
    val vm: ReportViewModel = viewModel(factory = ReportViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val view = LocalView.current
    val scope = rememberCoroutineScope()
    val isZh = Locale.getDefault().language == "zh"

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.report_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
                actions = {
                    state.report?.let { r ->
                        IconButton(
                            onClick = {
                                UiHaptics.tick(view)
                                scope.launch {
                                    val bytes = buildMonthReportCsv(r, isZh)
                                    ExportShare.writeAndShareFile(
                                        context,
                                        "grace-ride-report-${r.range}-${r.month}.csv",
                                        "text/csv",
                                        bytes,
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
    ) { padding ->
        AdaptiveContentFrame(
            maxWidth = 920.dp,
            compactHorizontalPadding = 0.dp,
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
        ) {
            Column(Modifier.fillMaxSize()) {
                PeriodSelectorBar(
                month = state.month,
                range = state.range,
                onShiftMonth = vm::shiftMonth,
                onSelectRange = vm::setRange,
            )

            Row(
                Modifier
                    .fillMaxWidth()
                    .padding(horizontal = Spacing.sm),
                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                listOf(
                    "all" to R.string.report_type_all,
                    "driver" to R.string.report_type_driver,
                    "passenger" to R.string.report_type_passenger,
                ).forEach { (key, res) ->
                    FilterChip(
                        selected = state.type == key,
                        onClick = {
                            UiHaptics.tick(view)
                            vm.setType(key)
                        },
                        label = { Text(stringResource(res)) },
                    )
                }
            }

            val viewMode = when {
                state.loading -> "loading"
                state.report == null -> "error"
                else -> "content"
            }
            Crossfade(
                targetState = viewMode,
                animationSpec = tween(200),
                label = "report_state",
                modifier = Modifier.weight(1f),
            ) { mode ->
                when (mode) {
                    "loading" -> DetailSkeleton()
                    "error" -> ErrorView(state.error ?: "", onRetry = vm::load)
                    else -> {
                        val r = state.report ?: return@Crossfade
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
                                MonthReportBody(report = r, isZh = isZh)
                            }
                        }
                    }
                }
            }
            }
        }
    }
}

@Composable
internal fun PeriodSelectorBar(
    month: String,
    range: String,
    onShiftMonth: (Long) -> Unit,
    onSelectRange: (String) -> Unit,
) {
    val view = LocalView.current
    val locale = Locale.getDefault()
    val formattedPeriod = remember(month, range, locale) {
        val ym = runCatching { YearMonth.parse(month) }.getOrNull()
        when {
            ym == null -> month
            range == "yearly" -> ym.year.toString()
            range == "quarterly" -> {
                val q = ((ym.monthValue - 1) / 3) + 1
                "Q$q ${ym.year}"
            }
            else -> ym.format(DateTimeFormatter.ofPattern("MMM yyyy", locale))
        }
    }

    val stepMonths = remember(range) {
        when (range) {
            "yearly" -> 12L
            "quarterly" -> 3L
            else -> 1L
        }
    }

    Column(
        Modifier
            .fillMaxWidth()
            .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
    ) {
        Row(
            Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            IconButton(
                onClick = {
                    UiHaptics.tick(view)
                    onShiftMonth(-stepMonths)
                },
                modifier = Modifier.defaultMinSize(minHeight = 48.dp, minWidth = 48.dp),
            ) {
                Icon(Icons.Outlined.ChevronLeft, contentDescription = stringResource(R.string.common_prev))
            }
            Text(
                formattedPeriod,
                style = MaterialTheme.typography.titleMedium.tabular(),
                fontWeight = FontWeight.SemiBold,
            )
            IconButton(
                onClick = {
                    UiHaptics.tick(view)
                    onShiftMonth(stepMonths)
                },
                modifier = Modifier.defaultMinSize(minHeight = 48.dp, minWidth = 48.dp),
            ) {
                Icon(Icons.Outlined.ChevronRight, contentDescription = stringResource(R.string.common_next))
            }
        }
        SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
            val ranges = listOf(
                "monthly" to R.string.report_range_monthly,
                "quarterly" to R.string.report_range_quarterly,
                "yearly" to R.string.report_range_yearly,
            )
            ranges.forEachIndexed { idx, (k, res) ->
                SegmentedButton(
                    selected = range == k,
                    onClick = {
                        UiHaptics.tick(view)
                        onSelectRange(k)
                    },
                    shape = SegmentedButtonDefaults.itemShape(idx, ranges.size),
                ) {
                    Text(stringResource(res))
                }
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun MonthReportBody(
    report: MonthReportDto,
    isZh: Boolean,
    onBadgeClick: ((Badge) -> Unit)? = null,
) {
    val vis = report.visibleSections
    val statusColors = LocalStatusColors.current

    Text(
        "${report.user.name} · ${report.periodStart} – ${report.periodEndInclusive ?: report.periodEnd}",
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )

    val heroCountTiles = buildList {
        report.eventsAttended?.let { n ->
            add(HeroMetricSpec(Icons.Outlined.CalendarMonth, stringResource(R.string.report_events_attended), n.toString(), statusColors.infoSoft, statusColors.info))
        }
        report.ridesDriven?.let { n ->
            add(HeroMetricSpec(Icons.Outlined.DirectionsCar, stringResource(R.string.report_rides_driven), n.toString(), statusColors.successSoft, statusColors.success))
        }
        report.distanceKm?.let { km ->
            add(HeroMetricSpec(Icons.Outlined.Place, stringResource(R.string.report_distance_km), "$km km", MaterialTheme.colorScheme.primaryContainer, MaterialTheme.colorScheme.onPrimaryContainer))
        }
        report.ridesReceived?.let { n ->
            add(HeroMetricSpec(Icons.Outlined.Group, stringResource(R.string.report_rides_received), n.toString(), MaterialTheme.colorScheme.secondaryContainer, MaterialTheme.colorScheme.onSecondaryContainer))
        }
    }

    if (heroCountTiles.isNotEmpty()) {
        heroCountTiles.chunked(2).forEach { pair ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                pair.forEach { spec ->
                    HeroMetricCard(spec = spec, modifier = Modifier.weight(1f))
                }
                if (pair.size == 1) {
                    Box(Modifier.weight(1f))
                }
            }
        }
    }

    if (report.reimbursementCents != null || report.costsPaidCents != null) {
        OutlinedCard(Modifier.fillMaxWidth()) {
            Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                report.reimbursementCents?.let { c ->
                    KpiLine(stringResource(R.string.report_reimbursement_total), Money.rm(c))
                }
                report.costsPaidCents?.let { c ->
                    KpiLine(stringResource(R.string.report_costs_paid), Money.rm(c))
                }
            }
        }
    }

    report.reimbursementsSummary?.let { rs ->
        OutlinedCard(Modifier.fillMaxWidth()) {
            Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
                SectionHeader(stringResource(R.string.report_reimb_summary))
                KpiLine(stringResource(R.string.report_reimb_claims), rs.claimCount.toString())
                rs.pendingCount?.let { pc ->
                    KpiLine(stringResource(R.string.reimb_status_pending), "$pc (${Money.rm(rs.pendingCents ?: 0L)})")
                }
                KpiLine(stringResource(R.string.reimb_status_approved), "${rs.approvedCount} (${Money.rm(rs.approvedCents)})")
                KpiLine(stringResource(R.string.reimb_status_paid), "${rs.paidCount} (${Money.rm(rs.paidCents)})")
            }
        }
    }

    if (vis?.events != false && report.events.isNotEmpty()) {
        SectionHeader(stringResource(R.string.report_events_section))
        OutlinedCard(Modifier.fillMaxWidth()) {
            Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                report.events.forEachIndexed { idx, ev ->
                    if (idx > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                        Text(ev.title, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                        Text(KlTime.displayDate(ev.startsAt), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
            }
        }
    }

    if (vis?.badges != false && report.badges.isNotEmpty()) {
        SectionHeader(stringResource(R.string.report_badges_section))
        Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
            report.badges.forEach { badge ->
                val bName = if (isZh) badge.nameZh ?: badge.name else badge.name
                val bRule = if (isZh) badge.ruleZh ?: badge.rule else badge.rule
                val bChar = if (isZh) badge.charZh ?: badge.char else badge.char
                val frac = if (badge.target > 0) (badge.progress.toFloat() / badge.target.toFloat()).coerceIn(0f, 1f) else 0f
                OutlinedCard(
                    onClick = { onBadgeClick?.invoke(badge) },
                    enabled = onBadgeClick != null,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Row(
                        Modifier.padding(Spacing.sm),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(Spacing.sm),
                    ) {
                        TieredBadgeMedallion(badge = badge, fallbackChar = bChar, size = 56.dp)
                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                            Row(
                                Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Text(
                                    bName,
                                    style = MaterialTheme.typography.titleSmall,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                    modifier = Modifier.weight(1f),
                                )
                                if (badge.earned) {
                                    StatusPill(
                                        text = badge.tier.replaceFirstChar { it.uppercase() },
                                        kind = RideStatusKind.DONE,
                                    )
                                }
                            }
                            Text(bRule, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            LinearProgressIndicator(
                                progress = { frac },
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .height(6.dp)
                                    .clip(RoundedCornerShape(3.dp)),
                            )
                            Text(
                                stringResource(R.string.report_badge_progress, badge.progress, badge.target, badge.tier),
                                style = MaterialTheme.typography.labelSmall.tabular(),
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }
            }
        }
    }
}

private class HeroMetricSpec(
    val icon: ImageVector,
    val label: String,
    val value: String,
    val iconBg: Color,
    val iconTint: Color,
)

@Composable
private fun HeroMetricCard(spec: HeroMetricSpec, modifier: Modifier = Modifier) {
    OutlinedCard(modifier = modifier) {
        Row(
            Modifier.padding(Spacing.sm),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Surface(
                color = spec.iconBg,
                contentColor = spec.iconTint,
                shape = CircleShape,
                modifier = Modifier.size(38.dp),
            ) {
                Box(contentAlignment = Alignment.Center) {
                    Icon(spec.icon, contentDescription = null, modifier = Modifier.size(20.dp))
                }
            }
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(
                    spec.value,
                    style = MaterialTheme.typography.titleLarge.tabular(),
                    fontWeight = FontWeight.Bold,
                    maxLines = 1,
                )
                Text(
                    spec.label,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

@Composable
internal fun KpiLine(label: String, value: String) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        Text(label, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(value, style = MaterialTheme.typography.bodyMedium.tabular(), fontWeight = FontWeight.SemiBold)
    }
}

internal fun buildMonthReportCsv(r: MonthReportDto, isZh: Boolean): ByteArray {
    val rows = mutableListOf<List<String>>()
    rows.add(listOf("User", "${r.user.name} (${r.user.id})"))
    rows.add(listOf("Period", "${r.periodStart} to ${r.periodEndInclusive ?: r.periodEnd}"))
    r.eventsAttended?.let { rows.add(listOf("Events attended", it.toString())) }
    r.ridesDriven?.let { rows.add(listOf("Rides driven", it.toString())) }
    r.ridesReceived?.let { rows.add(listOf("Rides received", it.toString())) }
    r.distanceKm?.let { rows.add(listOf("Distance (km)", it.toString())) }
    r.reimbursementCents?.let { rows.add(listOf("Reimbursements", Money.rm(it))) }
    r.costsPaidCents?.let { rows.add(listOf("Costs paid", Money.rm(it))) }
    r.badges.forEach { b ->
        val name = if (isZh) b.nameZh ?: b.name else b.name
        rows.add(listOf("Badge: $name", "${b.progress}/${b.target} (${b.tier})"))
    }
    return CsvBuilder.buildBytes(listOf("Metric", "Value"), rows)
}
