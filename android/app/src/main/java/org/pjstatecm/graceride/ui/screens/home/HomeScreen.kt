package org.pjstatecm.graceride.ui.screens.home

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.EventCard
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.OfflineBanner
import org.pjstatecm.graceride.ui.components.RideCard
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.components.ShareCard
import org.pjstatecm.graceride.ui.theme.Spacing

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(container: AppContainer, nav: NavHostController) {
    val vm: HomeViewModel = viewModel(factory = HomeViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()

    when {
        state.loading -> LoadingView()
        state.error != null && state.nextRide == null && state.events.isEmpty() && state.announcements.isEmpty() ->
            ErrorView(state.error!!, onRetry = vm::refresh)
        else -> PullToRefreshBox(isRefreshing = state.refreshing, onRefresh = vm::refresh) {
            LazyColumn(
                Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                contentPadding = androidx.compose.foundation.layout.PaddingValues(vertical = Spacing.sm),
            ) {
                item {
                    Text(
                        stringResource(R.string.home_greeting, state.me?.name ?: ""),
                        style = MaterialTheme.typography.headlineSmall,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                }
                if (state.offline) item { OfflineBanner() }

                item { SectionHeader(stringResource(R.string.home_next_ride)) }
                item {
                    val ride = state.nextRide
                    if (ride != null) {
                        RideCard(ride, onClick = { nav.navigate(Routes.rideDetail(ride.id)) })
                    } else {
                        OutlinedCard(Modifier.fillMaxWidth()) {
                            Column(Modifier.padding(Spacing.sm)) {
                                Text(
                                    stringResource(R.string.home_no_ride),
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                                if (state.me?.can("rides", "create") == true) {
                                    Button(
                                        onClick = { nav.navigate(Routes.RIDE_NEW) },
                                        modifier = Modifier.padding(top = Spacing.xs),
                                    ) { Text(stringResource(R.string.home_book_ride)) }
                                }
                            }
                        }
                    }
                }

                if (state.unpaidShares.isNotEmpty()) {
                    item {
                        SectionHeader(
                            stringResource(R.string.home_unpaid_shares),
                            stringResource(R.string.home_view_all),
                        ) { nav.navigate(Routes.COSTS) }
                    }
                    items(state.unpaidShares, key = { "share-${it.id}" }) { share ->
                        ShareCard(share, onPay = { nav.navigate(Routes.COSTS) })
                    }
                }

                if (state.drivingToday.isNotEmpty()) {
                    item { SectionHeader(stringResource(R.string.home_today_driving)) }
                    items(state.drivingToday, key = { "drive-${it.id}" }) { ride ->
                        RideCard(ride, onClick = { nav.navigate(Routes.rideDetail(ride.id)) })
                    }
                }

                if (state.events.isNotEmpty()) {
                    item {
                        SectionHeader(
                            stringResource(R.string.home_upcoming_events),
                            stringResource(R.string.home_view_all),
                        ) { nav.navigate(Routes.EVENTS) }
                    }
                    items(state.events, key = { "event-${it.id}" }) { event ->
                        EventCard(event, onClick = { nav.navigate(Routes.eventDetail(event.id)) })
                    }
                }

                if (state.announcements.isNotEmpty()) {
                    item { SectionHeader(stringResource(R.string.home_announcements)) }
                    items(state.announcements, key = { "ann-${it.id}" }) { ann ->
                        OutlinedCard(Modifier.fillMaxWidth()) {
                            Column(Modifier.padding(Spacing.sm)) {
                                Text(ann.title, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.onSurface)
                                if (ann.body.isNotBlank()) {
                                    Text(
                                        ann.body,
                                        style = MaterialTheme.typography.bodyMedium,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        maxLines = 4,
                                        overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                                        modifier = Modifier.padding(top = Spacing.xxs),
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
