package org.pjstatecm.graceride.ui.screens.admin

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxHeight
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
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.AltRoute
import androidx.compose.material.icons.automirrored.outlined.HelpOutline
import androidx.compose.material.icons.automirrored.outlined.ReceiptLong
import androidx.compose.material.icons.outlined.AdminPanelSettings
import androidx.compose.material.icons.outlined.BarChart
import androidx.compose.material.icons.outlined.Campaign
import androidx.compose.material.icons.outlined.CheckCircle
import androidx.compose.material.icons.outlined.ChevronRight
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Edit
import androidx.compose.material.icons.outlined.History
import androidx.compose.material.icons.outlined.LocationOn
import androidx.compose.material.icons.outlined.PeopleOutline
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material.icons.outlined.RequestQuote
import androidx.compose.material.icons.outlined.Settings
import androidx.compose.material.icons.outlined.Shield
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.AdaptiveContentFrame
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.RideStatusPill
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.tabular

private class ConsoleQueueItem(
    val key: String,
    val count: Long,
    val titleRes: Int,
    val kind: RideStatusKind,
    val route: AppRoute,
    val subtitle: String?,
)

private class ConsoleShortcut(
    val titleRes: Int,
    val icon: ImageVector,
    val route: AppRoute,
)

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun AdminConsoleScreen(
    container: AppContainer,
    nav: NavHostController,
) {
    val vm: AdminConsoleViewModel = viewModel(factory = AdminConsoleViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val me = (authState as? AuthState.SignedIn)?.me ?: container.auth.me
    val view = LocalView.current
    val statusColors = LocalStatusColors.current

    val canDashboard = me?.can("admin_dashboard") == true
    val canAccessAnyStaff = me != null && (canDashboard || Routes.canAdminConsole(me))

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.admin_console_title), maxLines = 1, overflow = TextOverflow.Ellipsis) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
                actions = {
                    IconButton(onClick = { UiHaptics.tick(view); vm.refresh(silent = true) }) {
                        Icon(Icons.Outlined.Refresh, contentDescription = stringResource(R.string.common_refresh))
                    }
                },
            )
        },
    ) { padding ->
        if (!canAccessAnyStaff) {
            Box(Modifier.fillMaxSize().padding(padding)) {
                EmptyView(text = stringResource(R.string.members_no_permission), icon = Icons.Outlined.AdminPanelSettings)
            }
            return@Scaffold
        }

        when {
            state.loading && canDashboard && state.dashboard == null -> {
                Box(Modifier.fillMaxSize().padding(padding)) { LoadingView() }
            }
            state.error != null && state.dashboard == null && canDashboard -> {
                Box(Modifier.fillMaxSize().padding(padding)) {
                    ErrorView(state.error ?: "", onRetry = { vm.refresh(silent = false) })
                }
            }
            else -> {
                val d = state.dashboard
                val queue = remember(d, me) {
                    if (d == null) emptyList()
                    else buildList {
                        if (me.can("admin_rides") && d.noDriver > 0) {
                            add(
                                ConsoleQueueItem(
                                    "noDriver", d.noDriver, R.string.admin_q_no_driver, RideStatusKind.CANCELLED,
                                    AppRoute.TrafficHub(tab = "rides", status = "pending_driver"),
                                    d.nextNoDriverAt?.let { KlTime.display(it) },
                                ),
                            )
                        }
                        if (me.can("admin_rides") && d.pendingApproval > 0) {
                            add(
                                ConsoleQueueItem(
                                    "approve", d.pendingApproval, R.string.admin_q_approve, RideStatusKind.PENDING,
                                    AppRoute.TrafficHub(tab = "rides", status = "pending_approval"),
                                    d.oldestApprovalAt?.let { KlTime.display(it) },
                                ),
                            )
                        }
                        if (me.can("admin_rides") && d.needsAttention > 0) {
                            add(
                                ConsoleQueueItem(
                                    "attention", d.needsAttention, R.string.admin_q_attention, RideStatusKind.CANCELLED,
                                    AppRoute.TrafficHub(tab = "rides", attention = true),
                                    null,
                                ),
                            )
                        }
                        if (me.can("admin_help") && d.openHelp > 0) {
                            add(
                                ConsoleQueueItem(
                                    "help", d.openHelp, R.string.admin_q_help, RideStatusKind.PENDING,
                                    AppRoute.Help(tab = "tickets"),
                                    d.oldestHelpAt?.let { KlTime.display(it) },
                                ),
                            )
                        }
                        if (me.can("admin_reimb_review") && d.pendingReimbursements > 0) {
                            add(
                                ConsoleQueueItem(
                                    "reimb", d.pendingReimbursements, R.string.admin_q_reimb, RideStatusKind.PENDING,
                                    AppRoute.AdminReimbursements(tab = "review"),
                                    null,
                                ),
                            )
                        }
                        if (me.can("admin_reimb_pay") && d.awaitingPayment > 0) {
                            add(
                                ConsoleQueueItem(
                                    "payout", d.awaitingPayment, R.string.admin_q_payout, RideStatusKind.PENDING,
                                    AppRoute.AdminReimbursements(tab = "payout"),
                                    null,
                                ),
                            )
                        }
                        if (me.can("admin_members") && d.pendingMembers > 0) {
                            add(
                                ConsoleQueueItem(
                                    "members", d.pendingMembers, R.string.admin_q_members, RideStatusKind.PENDING,
                                    AppRoute.AdminMembers(tab = "members", status = "pending"),
                                    null,
                                ),
                            )
                        }
                        if (me.can("admin_recovery") && d.pendingRecovery > 0) {
                            add(
                                ConsoleQueueItem(
                                    "recovery", d.pendingRecovery, R.string.admin_q_recovery, RideStatusKind.PENDING,
                                    AppRoute.AdminMembers(tab = "recovery"),
                                    null,
                                ),
                            )
                        }
                    }
                }

                val filteredToday = remember(state.todayRides, state.todayFilter) {
                    when (state.todayFilter) {
                        TodayRideFilter.ALL -> state.todayRides
                        TodayRideFilter.UNASSIGNED -> state.todayRides.filter { it.driver == null || it.status == "pending_driver" }
                        TodayRideFilter.CONFIRMED -> state.todayRides.filter { it.status == "confirmed" }
                        TodayRideFilter.IN_PROGRESS -> state.todayRides.filter { it.status == "in_progress" }
                    }
                }

                val shortcuts = remember(me) { buildConsoleShortcuts(me) }
                val shortcutPairs = remember(shortcuts) { shortcuts.chunked(2) }

                PullToRefreshBox(
                    isRefreshing = state.refreshing,
                    onRefresh = { vm.refresh(silent = true) },
                    modifier = Modifier.fillMaxSize().padding(padding),
                ) {
                    AdaptiveContentFrame(
                        maxWidth = 920.dp,
                        compactHorizontalPadding = 0.dp,
                        modifier = Modifier.fillMaxSize(),
                    ) {
                        LazyColumn(
                            modifier = Modifier.fillMaxSize().navigationBarsPadding(),
                            contentPadding = PaddingValues(Spacing.sm),
                            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                        ) {
                        if (d != null) {
                            item {
                                val todayFrac = if (d.todayTotal > 0) (d.todayConfirmed.toFloat() / d.todayTotal.toFloat()).coerceIn(0f, 1f) else 0f
                                val liveFrac = if (d.maxLiveDrivers > 0) (d.liveDrivers.toFloat() / d.maxLiveDrivers.toFloat()).coerceIn(0f, 1f) else 0f
                                Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                        ConsoleKpiCard(
                                            label = stringResource(R.string.admin_kpi_today),
                                            value = "${d.todayConfirmed}/${d.todayTotal}",
                                            icon = Icons.Outlined.DirectionsCar,
                                            accentColor = MaterialTheme.colorScheme.primary,
                                            progressFraction = todayFrac,
                                            modifier = Modifier.weight(1f),
                                            onClick = if (me.can("admin_rides")) {
                                                { nav.navigate(AppRoute.TrafficHub(tab = "rides")) }
                                            } else null,
                                        )
                                        ConsoleKpiCard(
                                            label = stringResource(R.string.admin_kpi_live),
                                            value = "${d.liveDrivers}/${d.maxLiveDrivers}",
                                            icon = Icons.Outlined.LocationOn,
                                            accentColor = statusColors.success,
                                            progressFraction = liveFrac,
                                            modifier = Modifier.weight(1f),
                                            onClick = if (me.can("admin_live")) {
                                                { nav.navigate(AppRoute.TrafficHub(tab = "live")) }
                                            } else null,
                                        )
                                    }
                                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                        ConsoleKpiCard(
                                            label = stringResource(R.string.admin_kpi_changes),
                                            value = d.todayChanges.toString(),
                                            icon = Icons.Outlined.Edit,
                                            accentColor = statusColors.warning,
                                            progressFraction = null,
                                            modifier = Modifier.weight(1f),
                                            onClick = if (me.can("admin_rides")) {
                                                { nav.navigate(AppRoute.TrafficHub(tab = "rides", attention = true)) }
                                            } else null,
                                        )
                                        ConsoleKpiCard(
                                            label = stringResource(R.string.members_title),
                                            value = d.members.toString(),
                                            icon = Icons.Outlined.PeopleOutline,
                                            accentColor = statusColors.info,
                                            progressFraction = null,
                                            modifier = Modifier.weight(1f),
                                            onClick = if (me.can("admin_members")) {
                                                { nav.navigate(AppRoute.AdminMembers()) }
                                            } else null,
                                        )
                                    }
                                }
                            }
                        }

                        if (queue.isNotEmpty()) {
                            item { SectionHeader(stringResource(R.string.admin_queue_section), count = queue.size) }
                            items(queue, key = { it.key }) { item ->
                                val stripeColor = if (item.kind == RideStatusKind.CANCELLED) statusColors.danger else statusColors.warning
                                OutlinedCard(
                                    onClick = { UiHaptics.tick(view); nav.navigate(item.route) },
                                    modifier = Modifier.fillMaxWidth(),
                                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                                ) {
                                    Row(Modifier.fillMaxWidth().height(IntrinsicSize.Min)) {
                                        Box(Modifier.width(4.dp).fillMaxHeight().background(stripeColor))
                                        Row(
                                            Modifier.weight(1f).padding(Spacing.sm),
                                            verticalAlignment = Alignment.CenterVertically,
                                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                        ) {
                                            StatusPill(text = item.count.toString(), kind = item.kind)
                                            Column(Modifier.weight(1f)) {
                                                Text(stringResource(item.titleRes), style = MaterialTheme.typography.titleSmall)
                                                item.subtitle?.let {
                                                    Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                                }
                                            }
                                            Icon(Icons.Outlined.ChevronRight, contentDescription = null)
                                        }
                                    }
                                }
                            }
                        } else if (d != null) {
                            item {
                                CalloutBanner(
                                    icon = Icons.Outlined.CheckCircle,
                                    text = stringResource(R.string.admin_all_caught_up),
                                    containerColor = statusColors.successSoft,
                                    contentColor = statusColors.success,
                                )
                            }
                        }

                        if (shortcuts.isNotEmpty()) {
                            item { SectionHeader(stringResource(R.string.admin_modules_section)) }
                            items(shortcutPairs, key = { pair -> pair.joinToString("-") { it.titleRes.toString() } }) { pair ->
                                Row(
                                    Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                ) {
                                    pair.forEach { sc ->
                                        OutlinedCard(
                                            onClick = { UiHaptics.tick(view); nav.navigate(sc.route) },
                                            modifier = Modifier.weight(1f).defaultMinSize(minHeight = 56.dp),
                                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                                        ) {
                                            Row(
                                                Modifier.fillMaxWidth().padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                                                verticalAlignment = Alignment.CenterVertically,
                                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                            ) {
                                                Surface(
                                                    color = MaterialTheme.colorScheme.primaryContainer,
                                                    shape = CircleShape,
                                                    modifier = Modifier.size(32.dp),
                                                ) {
                                                    Box(contentAlignment = Alignment.Center) {
                                                        Icon(
                                                            sc.icon,
                                                            contentDescription = null,
                                                            tint = MaterialTheme.colorScheme.primary,
                                                            modifier = Modifier.size(18.dp),
                                                        )
                                                    }
                                                }
                                                Text(
                                                    stringResource(sc.titleRes),
                                                    style = MaterialTheme.typography.titleSmall,
                                                    maxLines = 2,
                                                    overflow = TextOverflow.Ellipsis,
                                                    modifier = Modifier.weight(1f),
                                                )
                                            }
                                        }
                                    }
                                    if (pair.size == 1) {
                                        Spacer(Modifier.weight(1f))
                                    }
                                }
                            }
                        }

                        if (me.can("admin_rides") && state.todayRides.isNotEmpty()) {
                            item { SectionHeader(stringResource(R.string.admin_today_rides_section), count = state.todayRides.size) }
                            item {
                                LazyRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                    items(TodayRideFilter.entries) { filter ->
                                        val labelRes = when (filter) {
                                            TodayRideFilter.ALL -> R.string.admin_filter_all
                                            TodayRideFilter.UNASSIGNED -> R.string.admin_filter_unassigned
                                            TodayRideFilter.CONFIRMED -> R.string.admin_filter_confirmed
                                            TodayRideFilter.IN_PROGRESS -> R.string.admin_filter_in_progress
                                        }
                                        FilterChip(
                                            selected = state.todayFilter == filter,
                                            onClick = { UiHaptics.tick(view); vm.setTodayFilter(filter) },
                                            label = { Text(stringResource(labelRes)) },
                                        )
                                    }
                                }
                            }
                            items(filteredToday.take(5), key = { "today-${it.id}" }) { ride ->
                                OutlinedCard(
                                    onClick = { nav.navigate(AppRoute.RideDetail(ride.id)) },
                                    modifier = Modifier.fillMaxWidth(),
                                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                                ) {
                                    Row(
                                        Modifier.fillMaxWidth().padding(Spacing.sm),
                                        verticalAlignment = Alignment.CenterVertically,
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                    ) {
                                        Column(Modifier.weight(1f)) {
                                            Text(
                                                "${KlTime.displayTime(ride.pickupAt)} · ${ride.passenger.name}",
                                                style = MaterialTheme.typography.titleSmall.tabular(),
                                            )
                                            Text(
                                                "${ride.area.name} · ${ride.driver?.name ?: stringResource(R.string.traffic_unassigned)}",
                                                style = MaterialTheme.typography.bodySmall,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                            )
                                        }
                                        RideStatusPill(ride.status)
                                    }
                                }
                            }
                        }

                        if (me.can("admin_audit") && state.recentAudit.isNotEmpty()) {
                            item { SectionHeader(stringResource(R.string.audit_recent_section)) }
                            items(state.recentAudit, key = { "audit-${it.id}" }) { entry ->
                                val dotColor = when (entry.action) {
                                    in AdminAuditUiState.SECURITY_ACTIONS -> statusColors.danger
                                    in AdminAuditUiState.WARNING_ACTIONS -> statusColors.warning
                                    else -> MaterialTheme.colorScheme.primary
                                }
                                OutlinedCard(
                                    onClick = { nav.navigate(AppRoute.AdminAudit) },
                                    modifier = Modifier.fillMaxWidth(),
                                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                                ) {
                                    Row(
                                        Modifier.fillMaxWidth().padding(Spacing.sm),
                                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        Box(
                                            Modifier
                                                .size(8.dp)
                                                .clip(CircleShape)
                                                .background(dotColor),
                                        )
                                        Column(Modifier.weight(1f)) {
                                            Text(
                                                "${entry.actor ?: stringResource(R.string.audit_system_actor)} · ${entry.action}",
                                                style = MaterialTheme.typography.bodyMedium,
                                                fontWeight = FontWeight.Medium,
                                            )
                                            if (entry.detail.isNotBlank()) {
                                                Text(
                                                    entry.detail,
                                                    style = MaterialTheme.typography.bodySmall,
                                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                    maxLines = 1,
                                                    overflow = TextOverflow.Ellipsis,
                                                )
                                            }
                                        }
                                        Text(
                                            KlTime.displayTime(entry.createdAt),
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
            }
        }
    }
}

@Composable
private fun ConsoleKpiCard(
    label: String,
    value: String,
    icon: ImageVector,
    accentColor: Color,
    progressFraction: Float?,
    modifier: Modifier = Modifier,
    onClick: (() -> Unit)? = null,
) {
    val content: @Composable () -> Unit = {
        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    label,
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f),
                )
                Icon(icon, contentDescription = null, tint = accentColor, modifier = Modifier.size(18.dp))
            }
            Text(value, style = MaterialTheme.typography.titleLarge.tabular(), fontWeight = FontWeight.Bold)
            if (progressFraction != null) {
                LinearProgressIndicator(
                    progress = { progressFraction },
                    color = accentColor,
                    trackColor = MaterialTheme.colorScheme.surfaceVariant,
                    modifier = Modifier.fillMaxWidth().padding(top = 2.dp),
                )
            } else if (onClick != null) {
                Text(
                    stringResource(R.string.admin_tap_to_inspect),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
    if (onClick != null) {
        OutlinedCard(
            onClick = onClick,
            modifier = modifier,
            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
        ) { content() }
    } else {
        OutlinedCard(
            modifier = modifier,
            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
        ) { content() }
    }
}

private fun buildConsoleShortcuts(me: Me?): List<ConsoleShortcut> {
    if (me == null) return emptyList()
    return buildList {
        if (Routes.canTrafficHub(me)) add(ConsoleShortcut(R.string.traffic_title, Icons.AutoMirrored.Outlined.AltRoute, AppRoute.TrafficHub()))
        if (Routes.canAdminMembers(me)) add(ConsoleShortcut(R.string.members_title, Icons.Outlined.PeopleOutline, AppRoute.AdminMembers()))
        if (Routes.canManageSettlements(me)) add(ConsoleShortcut(R.string.settlements_title, Icons.AutoMirrored.Outlined.ReceiptLong, AppRoute.Settlements))
        if (me.can("admin_reimb_review") || me.can("admin_reimb_pay")) add(ConsoleShortcut(R.string.reimb_admin_title, Icons.Outlined.RequestQuote, AppRoute.AdminReimbursements()))
        if (Routes.canManageAnnouncements(me)) add(ConsoleShortcut(R.string.home_admin_announcements_btn, Icons.Outlined.Campaign, AppRoute.Announcements(manage = true)))
        if (me.can("admin_reports")) add(ConsoleShortcut(R.string.admin_reports_title, Icons.Outlined.BarChart, AppRoute.AdminReports()))
        if (me.can("admin_roles")) add(ConsoleShortcut(R.string.roles_title, Icons.Outlined.Shield, AppRoute.AdminRoles))
        if (me.can("admin_audit")) add(ConsoleShortcut(R.string.audit_title, Icons.Outlined.History, AppRoute.AdminAudit))
        if (me.can("admin_settings") || me.can("admin_groups")) add(ConsoleShortcut(R.string.settings_title, Icons.Outlined.Settings, AppRoute.AdminSettings()))
        if (me.can("help") || me.can("admin_help")) add(ConsoleShortcut(R.string.help_title, Icons.AutoMirrored.Outlined.HelpOutline, AppRoute.Help()))
    }
}
