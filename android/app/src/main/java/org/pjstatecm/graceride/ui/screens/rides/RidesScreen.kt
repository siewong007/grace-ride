package org.pjstatecm.graceride.ui.screens.rides

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Tab
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
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.OfflineBanner
import org.pjstatecm.graceride.ui.components.RideCard
import org.pjstatecm.graceride.ui.theme.Spacing

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun RidesScreen(container: AppContainer, nav: NavHostController) {
    val vm: RidesViewModel = viewModel(factory = RidesViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()

    Scaffold(
        floatingActionButton = {
            if (state.tab == 0) {
                ExtendedFloatingActionButton(
                    onClick = { nav.navigate(Routes.rideNew()) },
                    icon = { Icon(Icons.Filled.Add, contentDescription = null) },
                    text = { Text(stringResource(R.string.rides_book)) },
                )
            }
        },
    ) { padding ->
        Column(Modifier.fillMaxSize().padding(padding)) {
            PrimaryTabRow(selectedTabIndex = state.tab) {
                Tab(selected = state.tab == 0, onClick = { vm.setTab(0) }, text = { Text(stringResource(R.string.rides_tab_upcoming)) })
                Tab(selected = state.tab == 1, onClick = { vm.setTab(1) }, text = { Text(stringResource(R.string.rides_tab_past)) })
            }
            when {
                state.loading -> LoadingView()
                state.error != null && state.rides.isEmpty() -> ErrorView(state.error!!, vm::refresh)
                else -> PullToRefreshBox(isRefreshing = state.refreshing, onRefresh = vm::refresh) {
                    LazyColumn(
                        Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                        contentPadding = PaddingValues(vertical = Spacing.xs),
                    ) {
                        if (state.offline) item { OfflineBanner() }
                        if (state.rides.isEmpty()) {
                            item { EmptyView(stringResource(R.string.rides_empty)) }
                        }
                        items(state.rides, key = { it.id }) { ride ->
                            RideCard(ride, onClick = { nav.navigate(Routes.rideDetail(ride.id)) })
                        }
                    }
                }
            }
        }
    }
}
