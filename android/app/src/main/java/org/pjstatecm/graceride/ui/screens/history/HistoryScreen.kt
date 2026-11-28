package org.pjstatecm.graceride.ui.screens.history

import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.History
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
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
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.notifications.RealtimeEvent
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.components.AdaptiveContentFrame
import org.pjstatecm.graceride.ui.components.AnimatedOfflineBanner
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.RideCard
import org.pjstatecm.graceride.ui.components.SkeletonLoadingList
import org.pjstatecm.graceride.ui.theme.Spacing

data class HistoryUiState(
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val loadingMore: Boolean = false,
    val offline: Boolean = false,
    val error: String? = null,
    val rides: List<Ride> = emptyList(),
    val nextBefore: String? = null,
)

class HistoryViewModel(private val container: AppContainer) : ViewModel() {
    private val _state = MutableStateFlow(HistoryUiState())
    val state: StateFlow<HistoryUiState> = _state.asStateFlow()
    private var loadJob: kotlinx.coroutines.Job? = null
    private var moreJob: kotlinx.coroutines.Job? = null

    init {
        load(first = true)
        viewModelScope.launch {
            container.realtimeStream.events.collect { ev ->
                if (ev != RealtimeEvent.Resume && ev != RealtimeEvent.Chat) refresh()
            }
        }
        viewModelScope.launch {
            container.networkMonitor.isOnline.drop(1).filter { it }.collect { refresh() }
        }
    }

    fun refresh() = load(first = false)

    private fun load(first: Boolean) {
        loadJob?.cancel()
        moreJob?.cancel()
        if (container.auth.me?.can("rides", "view") != true) {
            _state.value = _state.value.copy(loading = false, refreshing = false, loadingMore = false)
            return
        }
        loadJob = viewModelScope.launch {
            _state.value = _state.value.copy(loading = first, refreshing = !first, loadingMore = false)
            try {
                val page = container.rides.pastPage(before = null, limit = 30)
                _state.value = _state.value.copy(
                    loading = false,
                    refreshing = false,
                    rides = page.items,
                    nextBefore = page.nextBefore,
                    error = null,
                    offline = false,
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(
                    loading = false,
                    refreshing = false,
                    error = if (e.status == 0) null else e.message,
                    offline = e.status == 0,
                )
            }
        }
    }

    fun loadMore() {
        val cursor = _state.value.nextBefore ?: return
        if (_state.value.loading || _state.value.loadingMore) return
        moreJob?.cancel()
        moreJob = viewModelScope.launch {
            _state.value = _state.value.copy(loadingMore = true)
            try {
                val page = container.rides.pastPage(before = cursor, limit = 30)
                val merged = (_state.value.rides + page.items).distinctBy { it.id }
                _state.value = _state.value.copy(
                    loadingMore = false,
                    rides = merged,
                    nextBefore = page.nextBefore,
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(
                    loadingMore = false,
                    offline = e.status == 0 || _state.value.offline,
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
    val lifecycleOwner = androidx.lifecycle.compose.LocalLifecycleOwner.current
    val resumedOnce = androidx.compose.runtime.remember { androidx.compose.runtime.mutableStateOf(false) }

    androidx.compose.runtime.DisposableEffect(lifecycleOwner) {
        val obs = androidx.lifecycle.LifecycleEventObserver { _, event ->
            if (event == androidx.lifecycle.Lifecycle.Event.ON_RESUME) {
                if (resumedOnce.value) vm.refresh() else resumedOnce.value = true
            }
        }
        lifecycleOwner.lifecycle.addObserver(obs)
        onDispose { lifecycleOwner.lifecycle.removeObserver(obs) }
    }

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
                "error" -> ErrorView(state.error ?: "", onRetry = vm::refresh)
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
                        val listState = rememberLazyListState()
                        val shouldLoadMore by remember(state.nextBefore, state.loadingMore, state.rides.size) {
                            derivedStateOf {
                                val total = listState.layoutInfo.totalItemsCount
                                val lastVisible = listState.layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: 0
                                state.nextBefore != null && !state.loadingMore && total > 0 && lastVisible >= total - 3
                            }
                        }
                        LaunchedEffect(shouldLoadMore) {
                            if (shouldLoadMore) vm.loadMore()
                        }
                        AdaptiveContentFrame(
                            maxWidth = 920.dp,
                            compactHorizontalPadding = 0.dp,
                            modifier = Modifier.fillMaxSize(),
                        ) {
                            LazyColumn(
                                state = listState,
                                modifier = Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
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
                            if (state.nextBefore != null) {
                                item(key = "history_load_more") {
                                    OutlinedButton(
                                        onClick = vm::loadMore,
                                        enabled = !state.loadingMore,
                                        modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
                                    ) {
                                        Text(
                                            stringResource(
                                                if (state.loadingMore) R.string.common_loading else R.string.history_load_more,
                                            ),
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
