package org.pjstatecm.graceride.ui.screens.history

import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.History
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
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.components.AnimatedOfflineBanner
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.RideCard
import org.pjstatecm.graceride.ui.components.SkeletonLoadingList
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
    private var loadJob: kotlinx.coroutines.Job? = null

    init { load(first = true) }

    fun refresh() = load(first = false)

    private fun load(first: Boolean) {
        loadJob?.cancel()
        if (container.auth.me?.can("rides", "view") != true) {
            _state.value = _state.value.copy(loading = false, refreshing = false)
            return
        }
        loadJob = viewModelScope.launch {
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
        val viewMode = when {
            state.loading -> "loading"
            state.error != null && state.rides.isEmpty() -> "error"
            else -> "content"
        }
        Crossfade(
            targetState = viewMode,
            animationSpec = tween(200),
            label = "history_state",
            modifier = Modifier.padding(padding),
        ) { mode ->
            when (mode) {
                "loading" -> SkeletonLoadingList(count = 4)
                "error" -> ErrorView(state.error ?: "", vm::refresh)
                else -> Column(Modifier.fillMaxSize()) {
                    AnimatedOfflineBanner(
                        visible = state.offline,
                        modifier = Modifier.padding(horizontal = Spacing.sm, vertical = Spacing.xxs),
                    )
                    PullToRefreshBox(
                        isRefreshing = state.refreshing,
                        onRefresh = vm::refresh,
                        modifier = Modifier.weight(1f),
                    ) {
                        LazyColumn(
                            Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            contentPadding = PaddingValues(vertical = Spacing.xs),
                        ) {
                            if (state.rides.isEmpty()) {
                                item {
                                    EmptyView(
                                        text = stringResource(R.string.history_empty),
                                        icon = Icons.Outlined.History,
                                    )
                                }
                            }
                            items(state.rides, key = { it.id }) { ride ->
                                RideCard(
                                    ride,
                                    photoUrlFor = container::fileUrl,
                                    onClick = { nav.navigate(AppRoute.RideDetail(ride.id)) },
                                    modifier = Modifier.animateItem(),
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}
