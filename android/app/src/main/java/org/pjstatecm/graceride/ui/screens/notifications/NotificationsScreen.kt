package org.pjstatecm.graceride.ui.screens.notifications

import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.HelpOutline
import androidx.compose.material.icons.automirrored.outlined.ReceiptLong
import androidx.compose.material.icons.outlined.AdminPanelSettings
import androidx.compose.material.icons.outlined.Campaign
import androidx.compose.material.icons.outlined.Delete
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Event
import androidx.compose.material.icons.outlined.FavoriteBorder
import androidx.compose.material.icons.outlined.MarkEmailRead
import androidx.compose.material.icons.outlined.MarkEmailUnread
import androidx.compose.material.icons.outlined.Notifications
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.navigation.NavHostController
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.NotificationItem
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.ui.NotifDestination
import org.pjstatecm.graceride.ui.NotifLinkRouter
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.openInWebApp
import org.pjstatecm.graceride.ui.theme.Spacing

data class NotifUiState(
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val loadingMore: Boolean = false,
    val loadMoreError: Boolean = false,
    val items: List<NotificationItem> = emptyList(),
    val nextBefore: Long? = null,
    val unreadCount: Int = 0,
    val unreadOnly: Boolean = false,
    val category: String? = null,
    val error: String? = null,
)

class NotificationsViewModel(private val container: AppContainer) : ViewModel() {
    private val _state = MutableStateFlow(NotifUiState())
    val state: StateFlow<NotifUiState> = _state.asStateFlow()
    private var loadJob: Job? = null

    init {
        load(reset = true)
        viewModelScope.launch {
            container.realtimeStream.refreshEvents.collect { kind ->
                if (kind == "notify" || kind == "resume") {
                    refresh()
                }
            }
        }
    }

    fun clearError() {
        _state.value = _state.value.copy(error = null)
    }

    fun load(reset: Boolean = true) {
        loadJob?.cancel()
        val cur = _state.value
        val targetUnreadOnly = cur.unreadOnly
        val targetCategory = cur.category
        _state.value = cur.copy(
            loading = reset && cur.items.isEmpty(),
            refreshing = !reset,
            error = null,
        )
        loadJob = viewModelScope.launch {
            try {
                val page = container.community.notificationsPage(
                    before = null,
                    limit = 30,
                    unreadOnly = targetUnreadOnly,
                    category = targetCategory,
                )
                if (_state.value.unreadOnly != targetUnreadOnly || _state.value.category != targetCategory) {
                    return@launch
                }
                container.auth.updateUnread(page.unread)
                _state.value = _state.value.copy(
                    loading = false,
                    refreshing = false,
                    loadMoreError = false,
                    items = page.items,
                    nextBefore = page.nextBefore,
                    unreadCount = page.unread,
                    error = null,
                )
            } catch (e: ApiException) {
                if (_state.value.unreadOnly == targetUnreadOnly && _state.value.category == targetCategory) {
                    _state.value = _state.value.copy(
                        loading = false,
                        refreshing = false,
                        error = e.message,
                    )
                }
            }
        }
    }

    fun refresh() = load(reset = false)

    fun loadMore() {
        val cur = _state.value
        val before = cur.nextBefore ?: return
        if (cur.loadingMore) return
        val targetUnreadOnly = cur.unreadOnly
        val targetCategory = cur.category
        viewModelScope.launch {
            _state.value = _state.value.copy(loadingMore = true, loadMoreError = false)
            try {
                val page = container.community.notificationsPage(
                    before = before,
                    limit = 30,
                    unreadOnly = targetUnreadOnly,
                    category = targetCategory,
                )
                val latest = _state.value
                if (latest.unreadOnly != targetUnreadOnly || latest.category != targetCategory) {
                    _state.value = latest.copy(loadingMore = false)
                    return@launch
                }
                val existingIds = latest.items.map { it.id }.toSet()
                val merged = latest.items + page.items.filter { it.id !in existingIds }
                _state.value = latest.copy(
                    loadingMore = false,
                    items = merged,
                    nextBefore = page.nextBefore,
                    unreadCount = page.unread,
                )
            } catch (_: ApiException) {
                if (_state.value.unreadOnly == targetUnreadOnly && _state.value.category == targetCategory) {
                    _state.value = _state.value.copy(loadingMore = false, loadMoreError = true)
                }
            }
        }
    }

    fun setUnreadOnly(enabled: Boolean) {
        if (_state.value.unreadOnly == enabled) return
        _state.value = _state.value.copy(
            unreadOnly = enabled,
            items = emptyList(),
            nextBefore = null,
            error = null,
            loading = true,
        )
        load(reset = true)
    }

    fun setCategory(category: String?) {
        val normalized = category?.takeIf { it.isNotBlank() }
        if (_state.value.category == normalized) return
        _state.value = _state.value.copy(
            category = normalized,
            items = emptyList(),
            nextBefore = null,
            error = null,
            loading = true,
        )
        load(reset = true)
    }

    fun toggleRead(item: NotificationItem) {
        val nextRead = !item.read
        val cur = _state.value
        val updatedItems = if (cur.unreadOnly && nextRead) {
            cur.items.filter { it.id != item.id }
        } else {
            cur.items.map { if (it.id == item.id) it.copy(read = nextRead) else it }
        }
        val delta = if (nextRead) -1 else 1
        _state.value = cur.copy(
            items = updatedItems,
            unreadCount = (cur.unreadCount + delta).coerceAtLeast(0),
        )
        viewModelScope.launch {
            try {
                val res = if (nextRead) {
                    container.community.readOne(item.id)
                } else {
                    container.community.unreadOne(item.id)
                }
                container.auth.updateUnread(res.unread)
                _state.value = _state.value.copy(unreadCount = res.unread)
            } catch (_: ApiException) {
                load(reset = false)
            }
        }
    }

    fun delete(item: NotificationItem) {
        val cur = _state.value
        val dec = if (!item.read) 1 else 0
        _state.value = cur.copy(
            items = cur.items.filter { it.id != item.id },
            unreadCount = (cur.unreadCount - dec).coerceAtLeast(0),
        )
        viewModelScope.launch {
            try {
                val res = container.community.deleteOne(item.id)
                container.auth.updateUnread(res.unread)
                _state.value = _state.value.copy(unreadCount = res.unread)
            } catch (_: ApiException) {
                load(reset = false)
            }
        }
    }

    fun markAllRead() {
        val cur = _state.value
        val updated = if (cur.unreadOnly) {
            emptyList()
        } else {
            cur.items.map { it.copy(read = true) }
        }
        val clearedCount = cur.items.count { !it.read }
        _state.value = cur.copy(
            items = updated,
            unreadCount = if (cur.category == null) 0 else (cur.unreadCount - clearedCount).coerceAtLeast(0),
        )
        viewModelScope.launch {
            try {
                val res = container.community.readAll(cur.category)
                container.auth.updateUnread(res.unread)
                _state.value = _state.value.copy(unreadCount = res.unread)
            } catch (_: ApiException) {
                load(reset = false)
            }
        }
    }

    fun open(item: NotificationItem): NotifDestination {
        if (!item.read) {
            toggleRead(item)
        }
        return NotifLinkRouter.resolve(item.link)
    }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory {
            initializer { NotificationsViewModel(container) }
        }
    }
}

private val categoryChips = listOf(
    null to R.string.notif_cat_all,
    "ride" to R.string.notif_cat_ride,
    "event" to R.string.notif_cat_event,
    "cost" to R.string.notif_cat_cost,
    "help" to R.string.notif_cat_help,
    "family" to R.string.notif_cat_family,
    "admin" to R.string.notif_cat_admin,
    "announcement" to R.string.notif_cat_announcement,
)

private fun notifIconFor(item: NotificationItem): ImageVector = when (item.category) {
    "ride" -> Icons.Outlined.DirectionsCar
    "event" -> Icons.Outlined.Event
    "cost" -> Icons.AutoMirrored.Outlined.ReceiptLong
    "help" -> Icons.AutoMirrored.Outlined.HelpOutline
    "family" -> Icons.Outlined.FavoriteBorder
    "admin" -> Icons.Outlined.AdminPanelSettings
    "announcement" -> Icons.Outlined.Campaign
    else -> when {
        item.link?.contains("ride") == true -> Icons.Outlined.DirectionsCar
        item.link?.contains("event") == true -> Icons.Outlined.Event
        item.link?.contains("cost") == true || item.link?.contains("settlement") == true -> Icons.AutoMirrored.Outlined.ReceiptLong
        else -> Icons.Outlined.Notifications
    }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun NotificationsScreen(container: AppContainer, nav: NavHostController) {
    val vm: NotificationsViewModel = viewModel(factory = NotificationsViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val snackbar = remember { SnackbarHostState() }
    val hasUnread = state.unreadCount > 0 || state.items.any { !it.read }

    LaunchedEffect(state.error, state.items.size) {
        val err = state.error
        if (err != null && state.items.isNotEmpty()) {
            snackbar.showSnackbar(err)
            vm.clearError()
        }
    }

    val today = KlTime.today()
    val yesterday = KlTime.todayPlusDays(-1)
    val (todayItems, yesterdayItems, earlierItems) = remember(state.items, today, yesterday) {
        val t = mutableListOf<NotificationItem>()
        val y = mutableListOf<NotificationItem>()
        val e = mutableListOf<NotificationItem>()
        state.items.forEach { item ->
            val day = KlTime.parse(item.createdAt)?.let { KlTime.date(item.createdAt) } ?: item.createdAt.take(10)
            when (day) {
                today -> t += item
                yesterday -> y += item
                else -> e += item
            }
        }
        Triple(t, y, e)
    }

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
                    if (hasUnread) {
                        TextButton(onClick = { vm.markAllRead() }) {
                            Text(stringResource(R.string.notif_mark_all_read))
                        }
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        PullToRefreshBox(
            isRefreshing = state.refreshing,
            onRefresh = { vm.refresh() },
            modifier = Modifier.fillMaxSize().padding(padding),
        ) {
            Column(Modifier.fillMaxSize()) {
                FlowRow(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = Spacing.sm, vertical = Spacing.xxs),
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    FilterChip(
                        selected = !state.unreadOnly,
                        onClick = { vm.setUnreadOnly(false) },
                        label = { Text(stringResource(R.string.notif_filter_all)) },
                    )
                    FilterChip(
                        selected = state.unreadOnly,
                        onClick = { vm.setUnreadOnly(true) },
                        label = { Text(stringResource(R.string.notif_filter_unread)) },
                    )
                }
                FlowRow(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = Spacing.sm),
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    categoryChips.forEach { (key, labelRes) ->
                        FilterChip(
                            selected = state.category == key,
                            onClick = { vm.setCategory(key) },
                            label = { Text(stringResource(labelRes)) },
                        )
                    }
                }

                val viewMode = when {
                    state.loading -> "loading"
                    state.error != null && state.items.isEmpty() -> "error"
                    state.items.isEmpty() -> "empty"
                    else -> "content"
                }

                Crossfade(
                    targetState = viewMode,
                    animationSpec = tween(200),
                    label = "notifications_state",
                    modifier = Modifier.weight(1f),
                ) { mode ->
                    when (mode) {
                        "loading" -> LoadingView(Modifier.fillMaxSize())
                        "error" -> ErrorView(state.error ?: "", { vm.load(reset = true) }, Modifier.fillMaxSize())
                        "empty" -> Box(
                            Modifier.fillMaxSize().verticalScroll(rememberScrollState()),
                            contentAlignment = Alignment.Center,
                        ) {
                            EmptyView(stringResource(R.string.notif_empty))
                        }
                        else -> LazyColumn(
                            Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            contentPadding = PaddingValues(vertical = Spacing.xs),
                        ) {
                            val groups = listOf(
                                R.string.notif_today to todayItems,
                                R.string.notif_yesterday to yesterdayItems,
                                R.string.notif_earlier to earlierItems,
                            )
                            groups.forEach { (headerRes, rows) ->
                                if (rows.isNotEmpty()) {
                                    item(key = "header-$headerRes") {
                                        Box(Modifier.animateItem()) {
                                            SectionHeader(stringResource(headerRes))
                                        }
                                    }
                                    items(rows, key = { it.id }) { item ->
                                        NotificationRowCard(
                                            item = item,
                                            onOpen = {
                                                when (val dest = vm.open(item)) {
                                                    is NotifDestination.Native -> {
                                                        if (dest.route != Routes.NOTIFICATIONS) {
                                                            nav.navigate(dest.route) { launchSingleTop = true }
                                                        }
                                                    }
                                                    is NotifDestination.Web -> openInWebApp(context, container.webUrl(dest.path))
                                                    NotifDestination.None -> {}
                                                }
                                            },
                                            onToggleRead = { vm.toggleRead(item) },
                                            onDelete = { vm.delete(item) },
                                            modifier = Modifier.animateItem(),
                                        )
                                    }
                                }
                            }

                            if (state.loadMoreError) {
                                item(key = "load-more-error") {
                                    Text(
                                        stringResource(R.string.error_network),
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.error,
                                        modifier = Modifier.padding(top = Spacing.xs),
                                    )
                                }
                            }

                            if (state.nextBefore != null) {
                                item(key = "load-more") {
                                    OutlinedButton(
                                        onClick = { vm.loadMore() },
                                        enabled = !state.loadingMore,
                                        modifier = Modifier.fillMaxWidth().padding(vertical = Spacing.xs),
                                    ) {
                                        Text(
                                            stringResource(
                                                if (state.loadMoreError) R.string.common_retry else R.string.notif_load_more,
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

@Composable
private fun NotificationRowCard(
    item: NotificationItem,
    onOpen: () -> Unit,
    onToggleRead: () -> Unit,
    onDelete: () -> Unit,
    modifier: Modifier = Modifier,
) {
    OutlinedCard(onClick = onOpen, modifier = modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(start = Spacing.sm, top = Spacing.xs, bottom = Spacing.xs, end = Spacing.xxs),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Icon(
                imageVector = notifIconFor(item),
                contentDescription = null,
                tint = if (item.priority == "urgent") {
                    MaterialTheme.colorScheme.error
                } else if (!item.read) {
                    MaterialTheme.colorScheme.primary
                } else {
                    MaterialTheme.colorScheme.onSurfaceVariant
                },
            )
            Column(Modifier.weight(1f).padding(vertical = Spacing.xxs)) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    if (!item.read) {
                        Box(
                            Modifier
                                .size(8.dp)
                                .clip(CircleShape)
                                .background(MaterialTheme.colorScheme.primary),
                        )
                    }
                    Text(
                        item.title,
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = if (item.read) FontWeight.Normal else FontWeight.Bold,
                        color = if (item.read) MaterialTheme.colorScheme.onSurfaceVariant else MaterialTheme.colorScheme.onSurface,
                    )
                }
                if (item.body.isNotBlank()) {
                    Text(
                        item.body,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(top = Spacing.xxs),
                    )
                }
                Text(
                    KlTime.display(item.createdAt),
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(top = Spacing.xxs),
                )
            }
            IconButton(onClick = onToggleRead) {
                Icon(
                    imageVector = if (item.read) Icons.Outlined.MarkEmailUnread else Icons.Outlined.MarkEmailRead,
                    contentDescription = stringResource(if (item.read) R.string.notif_mark_unread else R.string.notif_mark_read),
                )
            }
            IconButton(onClick = onDelete) {
                Icon(
                    imageVector = Icons.Outlined.Delete,
                    contentDescription = stringResource(R.string.notif_delete),
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}
