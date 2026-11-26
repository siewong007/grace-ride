package org.pjstatecm.graceride.ui.screens.reimbursements

import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.ReceiptLong
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalView
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
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.ReimbursementDto
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.SecureScreenEffect
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.AnimatedOfflineBanner
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.SkeletonLoadingList
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.tabular

internal fun reimbStatusKind(status: String): RideStatusKind = when (status) {
    "approved" -> RideStatusKind.ACTIVE
    "paid" -> RideStatusKind.DONE
    "pending", "returned" -> RideStatusKind.PENDING
    else -> RideStatusKind.CANCELLED
}

@Composable
internal fun reimbStatusLabel(status: String): String = when (status) {
    "pending" -> stringResource(R.string.reimb_status_pending)
    "approved" -> stringResource(R.string.reimb_status_approved)
    "returned" -> stringResource(R.string.reimb_status_returned)
    "rejected" -> stringResource(R.string.reimb_status_rejected)
    "paid" -> stringResource(R.string.reimb_status_paid)
    else -> status
}

@Composable
internal fun reimbKindLabel(kind: String): String = when (kind) {
    "mileage" -> stringResource(R.string.reimb_kind_mileage)
    "visitor_special" -> stringResource(R.string.reimb_kind_visitor_special)
    else -> stringResource(R.string.reimb_kind_grab)
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ReimbursementsScreen(container: AppContainer, nav: NavHostController) {
    val vm: ReimbursementsViewModel = viewModel(factory = ReimbursementsViewModel.factory(container))
    SecureScreenEffect()
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val me = (authState as? AuthState.SignedIn)?.me ?: container.auth.me
    val canCreate = me?.can("reimbursements", "create") == true
    val lifecycleOwner = LocalLifecycleOwner.current
    val view = LocalView.current

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

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.reimb_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
            )
        },
        floatingActionButton = {
            if (canCreate) {
                ExtendedFloatingActionButton(
                    onClick = {
                        UiHaptics.tick(view)
                        nav.navigate(AppRoute.ReimbursementForm())
                    },
                    icon = { Icon(Icons.Outlined.Add, contentDescription = null) },
                    text = { Text(stringResource(R.string.reimb_new)) },
                    containerColor = MaterialTheme.colorScheme.primary,
                    contentColor = MaterialTheme.colorScheme.onPrimary,
                )
            }
        },
    ) { padding ->
        val mode = when {
            state.loading -> "loading"
            state.error != null && state.items.isEmpty() -> "error"
            else -> "content"
        }
        Crossfade(
            targetState = mode,
            animationSpec = tween(200),
            label = "reimbursements_state",
            modifier = Modifier.padding(padding),
        ) { current ->
            when (current) {
                "loading" -> SkeletonLoadingList(count = 4)
                "error" -> ErrorView(state.error ?: "", onRetry = vm::refresh)
                else -> {
                    val statusColors = LocalStatusColors.current
                    Column(Modifier.fillMaxSize()) {
                        AnimatedOfflineBanner(
                            visible = state.offline,
                            modifier = Modifier.padding(horizontal = Spacing.sm, vertical = Spacing.xxs),
                        )
                        if (state.summaryLoaded) {
                            Row(
                                Modifier
                                    .fillMaxWidth()
                                    .padding(horizontal = Spacing.sm, vertical = Spacing.xxs),
                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                SummaryTile(
                                    label = stringResource(R.string.reimb_total_pending),
                                    cents = state.summaryPendingCents,
                                    containerColor = statusColors.warningSoft,
                                    contentColor = statusColors.warning,
                                    modifier = Modifier.weight(1f),
                                )
                                SummaryTile(
                                    label = stringResource(R.string.reimb_total_approved),
                                    cents = state.summaryApprovedCents,
                                    containerColor = statusColors.infoSoft,
                                    contentColor = statusColors.info,
                                    modifier = Modifier.weight(1f),
                                )
                                SummaryTile(
                                    label = stringResource(R.string.reimb_total_paid),
                                    cents = state.summaryPaidCents,
                                    containerColor = statusColors.successSoft,
                                    contentColor = statusColors.success,
                                    modifier = Modifier.weight(1f),
                                )
                            }
                        }
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
                        val filters = listOf(
                            "all" to R.string.reimb_filter_all,
                            "pending" to R.string.reimb_status_pending,
                            "returned" to R.string.reimb_status_returned,
                            "approved" to R.string.reimb_status_approved,
                            "paid" to R.string.reimb_status_paid,
                            "rejected" to R.string.reimb_status_rejected,
                        )
                        LazyRow(
                            contentPadding = PaddingValues(horizontal = Spacing.sm),
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            items(filters, key = { it.first }) { (key, res) ->
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

                        PullToRefreshBox(
                            isRefreshing = state.refreshing,
                            onRefresh = vm::refresh,
                            modifier = Modifier.weight(1f),
                        ) {
                            LazyColumn(
                                Modifier
                                    .fillMaxSize()
                                    .padding(horizontal = Spacing.sm),
                                contentPadding = PaddingValues(top = Spacing.xs, bottom = 88.dp),
                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                if (state.items.isEmpty()) {
                                    item {
                                        EmptyView(
                                            text = stringResource(R.string.reimb_empty),
                                            icon = Icons.AutoMirrored.Outlined.ReceiptLong,
                                        )
                                    }
                                } else {
                                    items(state.items, key = { it.id }) { item ->
                                        ReimbursementCard(
                                            item = item,
                                            onClick = {
                                                UiHaptics.tick(view)
                                                nav.navigate(AppRoute.ReimbursementDetail(item.id))
                                            },
                                            modifier = Modifier.animateItem(),
                                        )
                                    }
                                }
                                if (state.totalPages > 1) {
                                    item(key = "reimb_pager") {
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
                                            Text(
                                                "${state.page} / ${state.totalPages}",
                                                style = MaterialTheme.typography.labelMedium.tabular(),
                                            )
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
}

@Composable
private fun SummaryTile(
    label: String,
    cents: Long,
    containerColor: Color,
    contentColor: Color,
    modifier: Modifier = Modifier,
) {
    Surface(
        color = containerColor,
        shape = MaterialTheme.shapes.small,
        modifier = modifier,
    ) {
        Column(
            Modifier.padding(horizontal = Spacing.xs, vertical = Spacing.xxs),
            verticalArrangement = Arrangement.spacedBy(2.dp),
        ) {
            Text(
                label,
                style = MaterialTheme.typography.labelSmall,
                color = contentColor,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                Money.rm(cents),
                style = MaterialTheme.typography.labelLarge.tabular(),
                fontWeight = FontWeight.Bold,
                color = contentColor,
                maxLines = 1,
            )
        }
    }
}

@Composable
internal fun ReimbursementCard(
    item: ReimbursementDto,
    onClick: () -> Unit,
    showClaimant: Boolean = false,
    modifier: Modifier = Modifier,
) {
    OutlinedCard(onClick = onClick, modifier = modifier.fillMaxWidth()) {
        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    Money.rm(item.amountCents),
                    style = MaterialTheme.typography.titleMedium.tabular(),
                    fontWeight = FontWeight.Bold,
                )
                StatusPill(
                    text = reimbStatusLabel(item.status),
                    kind = reimbStatusKind(item.status),
                )
            }
            Text(
                buildString {
                    if (showClaimant) append("${item.user.name} · ")
                    append(reimbKindLabel(item.kind))
                    item.eventTitle?.takeIf { it.isNotBlank() }?.let { append(" · $it") }
                },
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.primary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            if (item.description.isNotBlank()) {
                Text(
                    item.description,
                    style = MaterialTheme.typography.bodyMedium,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            if (item.reviewNote.isNotBlank()) {
                Text(
                    item.reviewNote,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            Text(
                KlTime.display(item.createdAt),
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}
