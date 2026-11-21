package org.pjstatecm.graceride.ui.screens.notifications

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
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
import org.pjstatecm.graceride.data.api.NotificationItem
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.theme.Spacing

data class NotifUiState(
    val loading: Boolean = true,
    val items: List<NotificationItem> = emptyList(),
    val error: String? = null,
)

class NotificationsViewModel(private val container: AppContainer) : ViewModel() {
    private val _state = MutableStateFlow(NotifUiState())
    val state: StateFlow<NotifUiState> = _state.asStateFlow()

    init { load() }

    fun load() {
        viewModelScope.launch {
            _state.value = _state.value.copy(loading = true)
            try {
                _state.value = NotifUiState(loading = false, items = container.community.notifications())
            } catch (e: ApiException) {
                _state.value = NotifUiState(loading = false, error = e.message)
            }
        }
    }

    fun markAllRead() {
        viewModelScope.launch {
            try {
                container.community.readAll()
                load()
                container.auth.refresh()
            } catch (_: ApiException) { }
        }
    }

    fun open(item: NotificationItem, nav: NavHostController) {
        viewModelScope.launch {
            try { container.community.readOne(item.id) } catch (_: ApiException) { }
        }
        val link = item.link ?: return
        val rideMatch = Regex("^/rides/(\\d+)").find(link)
        val eventMatch = Regex("^/events/(\\d+)").find(link)
        when {
            rideMatch != null -> nav.navigate(Routes.rideDetail(rideMatch.groupValues[1].toLong()))
            eventMatch != null -> nav.navigate(Routes.eventDetail(eventMatch.groupValues[1].toLong()))
            link == "/costs" -> nav.navigate(Routes.COSTS)
        }
    }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory { initializer { NotificationsViewModel(container) } }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NotificationsScreen(container: AppContainer, nav: NavHostController) {
    val vm: NotificationsViewModel = viewModel(factory = NotificationsViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.notif_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
                actions = {
                    TextButton(onClick = { vm.markAllRead() }) { Text(stringResource(R.string.notif_mark_all_read)) }
                },
            )
        },
    ) { padding ->
        when {
            state.loading -> LoadingView(Modifier.padding(padding))
            state.error != null -> ErrorView(state.error!!, vm::load, Modifier.padding(padding))
            state.items.isEmpty() -> EmptyView(stringResource(R.string.notif_empty), Modifier.padding(padding))
            else -> LazyColumn(
                Modifier.fillMaxSize().padding(padding).padding(horizontal = Spacing.sm),
                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                contentPadding = PaddingValues(vertical = Spacing.xs),
            ) {
                items(state.items, key = { it.id }) { item ->
                    OutlinedCard(onClick = { vm.open(item, nav) }, modifier = Modifier.fillMaxWidth()) {
                        Column(Modifier.padding(Spacing.sm)) {
                            Text(
                                item.title,
                                style = MaterialTheme.typography.titleSmall,
                                color = if (item.read) MaterialTheme.colorScheme.onSurfaceVariant else MaterialTheme.colorScheme.onSurface,
                            )
                            if (item.body.isNotBlank()) {
                                Text(item.body, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(top = Spacing.xxs))
                            }
                            Text(KlTime.display(item.createdAt), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(top = Spacing.xxs))
                        }
                    }
                }
            }
        }
    }
}
