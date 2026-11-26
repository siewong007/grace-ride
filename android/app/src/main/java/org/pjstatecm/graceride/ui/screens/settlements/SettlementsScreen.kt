package org.pjstatecm.graceride.ui.screens.settlements

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
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
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
import org.pjstatecm.graceride.data.api.SettlementSummaryDto
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.components.AnimatedOfflineBanner
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.SkeletonLoadingList
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.tabular

internal fun settlementStatusPillKind(status: String): RideStatusKind = when (status) {
    "published" -> RideStatusKind.ACTIVE
    "closed", "confirmed" -> RideStatusKind.DONE
    "draft", "pending", "needs_info" -> RideStatusKind.PENDING
    else -> RideStatusKind.CANCELLED
}

@Composable
internal fun settlementStatusLabel(status: String): String = when (status) {
    "draft" -> stringResource(R.string.settlement_status_draft)
    "published" -> stringResource(R.string.settlement_status_published)
    "closed" -> stringResource(R.string.settlement_status_closed)
    "confirmed" -> stringResource(R.string.costs_status_confirmed)
    "pending" -> stringResource(R.string.costs_status_pending)
    "needs_info" -> stringResource(R.string.settlement_status_needs_info)
    "unpaid" -> stringResource(R.string.settlement_status_unpaid)
    else -> status
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettlementsScreen(container: AppContainer, nav: NavHostController) {
    val vm: SettlementsViewModel = viewModel(factory = SettlementsViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val me = (authState as? AuthState.SignedIn)?.me ?: container.auth.me
    val canCreate = me?.can("settlements", "create") == true
    val lifecycleOwner = LocalLifecycleOwner.current

    DisposableEffect(lifecycleOwner) {
        var firstResume = true
        val obs = LifecycleEventObserver { _, ev ->
            if (ev == Lifecycle.Event.ON_RESUME) {
                if (firstResume) firstResume = false else vm.refresh()
            }
        }
        lifecycleOwner.lifecycle.addObserver(obs)
        onDispose { lifecycleOwner.lifecycle.removeObserver(obs) }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.settlements_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
            )
        },
        floatingActionButton = {
            if (canCreate) {
                FloatingActionButton(onClick = { nav.navigate(AppRoute.SettlementForm()) }) {
                    Icon(Icons.Outlined.Add, contentDescription = stringResource(R.string.settlements_new))
                }
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
            label = "settlements_state",
            modifier = Modifier.padding(padding),
        ) { current ->
            when (current) {
                "loading" -> SkeletonLoadingList(count = 4)
                "error" -> ErrorView(state.error ?: "", onRetry = vm::refresh)
                else -> Column(Modifier.fillMaxSize()) {
                    AnimatedOfflineBanner(
                        visible = state.offline,
                        modifier = Modifier.padding(horizontal = Spacing.sm, vertical = Spacing.xxs),
                    )
                    OutlinedTextField(
                        value = state.query,
                        onValueChange = vm::setQuery,
                        placeholder = { Text(stringResource(R.string.settlements_search_hint)) },
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
                    val statuses = listOf(
                        "all" to R.string.settlement_filter_all,
                        "draft" to R.string.settlement_status_draft,
                        "published" to R.string.settlement_status_published,
                        "closed" to R.string.settlement_status_closed,
                    )
                    LazyRow(
                        contentPadding = PaddingValues(horizontal = Spacing.sm),
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        items(statuses, key = { it.first }) { (key, res) ->
                            FilterChip(
                                selected = state.statusFilter == key,
                                onClick = { vm.setStatusFilter(key) },
                                label = { Text(stringResource(res)) },
                            )
                        }
                    }

                    PullToRefreshBox(
                        isRefreshing = state.refreshing,
                        onRefresh = vm::refresh,
                        modifier = Modifier.weight(1f),
                    ) {
                        val visible = state.visibleItems
                        LazyColumn(
                            Modifier
                                .fillMaxSize()
                                .padding(horizontal = Spacing.sm),
                            contentPadding = PaddingValues(top = Spacing.xs, bottom = 88.dp),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            if (visible.isEmpty()) {
                                item {
                                    EmptyView(
                                        text = stringResource(R.string.settlements_empty),
                                        icon = Icons.AutoMirrored.Outlined.ReceiptLong,
                                    )
                                }
                            } else {
                                items(visible, key = { it.id }) { item ->
                                    SettlementSummaryCard(
                                        item = item,
                                        onClick = { nav.navigate(AppRoute.SettlementDetail(item.id)) },
                                        modifier = Modifier.animateItem(),
                                    )
                                }
                            }
                            if (state.totalPages > 1) {
                                item(key = "pager") {
                                    Row(
                                        Modifier.fillMaxWidth().padding(vertical = Spacing.xs),
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        OutlinedButton(
                                            onClick = { vm.loadPage(state.page - 1) },
                                            enabled = state.page > 1 && !state.refreshing,
                                            modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                                        ) {
                                            Text(stringResource(R.string.common_prev))
                                        }
                                        Text(
                                            "${state.page} / ${state.totalPages}",
                                            style = MaterialTheme.typography.labelMedium.tabular(),
                                        )
                                        OutlinedButton(
                                            onClick = { vm.loadPage(state.page + 1) },
                                            enabled = state.page < state.totalPages && !state.refreshing,
                                            modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                                        ) {
                                            Text(stringResource(R.string.common_next))
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
private fun SettlementSummaryCard(
    item: SettlementSummaryDto,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val progress = if (item.count > 0) (item.confirmed.toFloat() / item.count.toFloat()).coerceIn(0f, 1f) else 0f
    OutlinedCard(onClick = onClick, modifier = modifier.fillMaxWidth()) {
        Column(
            Modifier.padding(Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(Modifier.weight(1f)) {
                    Text(
                        item.title,
                        style = MaterialTheme.typography.titleMedium,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    if (item.eventTitle.isNotBlank()) {
                        Text(
                            item.eventTitle,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }
                StatusPill(
                    text = settlementStatusLabel(item.status),
                    kind = settlementStatusPillKind(item.status),
                )
            }
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    Money.rm(item.totalCents),
                    style = MaterialTheme.typography.titleSmall.tabular(),
                    fontWeight = FontWeight.SemiBold,
                )
                Text(
                    stringResource(R.string.settlements_confirmed_ratio, item.confirmed, item.count),
                    style = MaterialTheme.typography.labelMedium.tabular(),
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            LinearProgressIndicator(
                progress = { progress },
                modifier = Modifier.fillMaxWidth(),
            )
        }
    }
}
