package org.pjstatecm.graceride.ui.screens.history

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.OfflineBanner
import org.pjstatecm.graceride.ui.components.RideCard
import org.pjstatecm.graceride.ui.theme.Spacing

data class HistoryUiState(
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val offline: Boolean = false,
    val error: String? = null,
    val rides: List<Ride> = emptyList(),
)

class HistoryViewModel(private val container: AppContainer) : ViewModel() {
    private val _state = MutableStateFlow(HistoryUiState())
    val state: StateFlow<HistoryUiState> = _state.asStateFlow()

    init { load(first = true) }

    fun refresh() = load(first = false)

    private fun load(first: Boolean) {
        viewModelScope.launch {
            _state.value = _state.value.copy(loading = first, refreshing = !first)
            try {
                _state.value = _state.value.copy(loading = false, refreshing = false, rides = container.rides.past(), error = null, offline = false)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(
                    loading = false, refreshing = false,
                    error = if (e.status == 0) null else e.message,
                    offline = e.status == 0,
                )
            }
        }
    }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory { initializer { HistoryViewModel(container) } }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HistoryScreen(container: AppContainer, nav: NavHostController) {
    val vm: HistoryViewModel = viewModel(factory = HistoryViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.history_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
            )
        },
    ) { padding ->
        when {
            state.loading -> LoadingView(Modifier.padding(padding))
            state.error != null && state.rides.isEmpty() -> ErrorView(state.error!!, vm::refresh, Modifier.padding(padding))
            else -> PullToRefreshBox(
                isRefreshing = state.refreshing,
                onRefresh = vm::refresh,
                modifier = Modifier.padding(padding),
            ) {
                LazyColumn(
                    Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    contentPadding = PaddingValues(vertical = Spacing.xs),
                ) {
                    if (state.offline) item { OfflineBanner() }
                    if (state.rides.isEmpty()) item { EmptyView(stringResource(R.string.history_empty)) }
                    items(state.rides, key = { it.id }) { ride ->
                        RideCard(ride, onClick = { nav.navigate(Routes.rideDetail(ride.id)) })
                    }
                }
            }
        }
    }
}
