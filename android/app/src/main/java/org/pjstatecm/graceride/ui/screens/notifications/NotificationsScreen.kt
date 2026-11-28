package org.pjstatecm.graceride.ui.screens.notifications

import android.Manifest
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.Crossfade
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.snap
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.Chat
import androidx.compose.material.icons.automirrored.outlined.HelpOutline
import androidx.compose.material.icons.automirrored.outlined.OpenInNew
import androidx.compose.material.icons.automirrored.outlined.ReceiptLong
import androidx.compose.material.icons.outlined.AdminPanelSettings
import androidx.compose.material.icons.outlined.Campaign
import androidx.compose.material.icons.outlined.ChevronRight
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.Delete
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.DoneAll
import androidx.compose.material.icons.outlined.Event
import androidx.compose.material.icons.outlined.FavoriteBorder
import androidx.compose.material.icons.outlined.MarkEmailRead
import androidx.compose.material.icons.outlined.MarkEmailUnread
import androidx.compose.material.icons.outlined.MoreVert
import androidx.compose.material.icons.outlined.Notifications
import androidx.compose.material.icons.outlined.NotificationsNone
import androidx.compose.material.icons.outlined.NotificationsOff
import androidx.compose.material.icons.outlined.Settings
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.SnackbarResult
import androidx.compose.material3.Surface
import androidx.compose.material3.SwipeToDismissBox
import androidx.compose.material3.SwipeToDismissBoxValue
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.rememberSwipeToDismissBoxState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.LocalLifecycleOwner
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
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.NotificationItem
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.notifications.NotificationHelper
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.NotifDestination
import org.pjstatecm.graceride.ui.NotifLinkRouter
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.AdaptiveContentFrame
import org.pjstatecm.graceride.ui.components.animatorsEnabled
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.components.SkeletonLoadingList
import org.pjstatecm.graceride.ui.navigateRoute
import org.pjstatecm.graceride.ui.openInWebApp
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.tabular
import java.util.Locale

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
    val restoreTick: Int = 0,
)

class NotificationsViewModel(private val container: AppContainer) : ViewModel() {
    private data class PendingDelete(val item: NotificationItem, val index: Int)

    private val _state = MutableStateFlow(NotifUiState())
    val state: StateFlow<NotifUiState> = _state.asStateFlow()
    private var loadJob: Job? = null
    private val pendingDeletes = LinkedHashMap<Long, PendingDelete>()

    init {
        load(reset = true)
        viewModelScope.launch {
            container.realtimeStream.events.collect { ev ->
                if (ev == org.pjstatecm.graceride.notifications.RealtimeEvent.Notify ||
                    ev == org.pjstatecm.graceride.notifications.RealtimeEvent.Resume
                ) {
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
                val visible = page.items.filter { it.id !in pendingDeletes }
                val pendingUnread = pendingDeletes.values.count { !it.item.read }
                val effectiveUnread = (page.unread - pendingUnread).coerceAtLeast(0)
                container.auth.updateUnread(effectiveUnread)
                _state.value = _state.value.copy(
                    loading = false,
                    refreshing = false,
                    loadMoreError = false,
                    items = visible,
                    nextBefore = page.nextBefore,
                    unreadCount = effectiveUnread,
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
                val merged = latest.items + page.items.filter { it.id !in existingIds && it.id !in pendingDeletes }
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

    fun resetFilters() {
        if (!_state.value.unreadOnly && _state.value.category == null) return
        _state.value = _state.value.copy(
            unreadOnly = false,
            category = null,
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

    /** Optimistically removes [item] while waiting for the Undo snackbar window. */
    fun stageDelete(item: NotificationItem) {
        pendingDeletes.keys.toList().forEach { pendingId ->
            if (pendingId != item.id) commitDelete(pendingId)
        }
        val cur = _state.value
        val idx = cur.items.indexOfFirst { it.id == item.id }.coerceAtLeast(0)
        pendingDeletes[item.id] = PendingDelete(item, idx)
        val dec = if (!item.read) 1 else 0
        _state.value = cur.copy(
            items = cur.items.filter { it.id != item.id },
            unreadCount = (cur.unreadCount - dec).coerceAtLeast(0),
        )
    }

    /** Restores a staged notification when the user taps Undo. */
    fun undoDelete(id: Long) {
        val pending = pendingDeletes.remove(id) ?: return
        val cur = _state.value
        if (cur.items.any { it.id == id }) return
        val restored = cur.items.toMutableList().apply {
            add(pending.index.coerceIn(0, size), pending.item)
        }
        val inc = if (!pending.item.read) 1 else 0
        _state.value = cur.copy(
            items = restored,
            unreadCount = cur.unreadCount + inc,
            restoreTick = cur.restoreTick + 1,
        )
    }

    /** Finalizes deletion on the backend once the Undo window ends. */
    fun commitDelete(id: Long) {
        pendingDeletes.remove(id) ?: return
        container.appScope.launch {
            try {
                val res = container.community.deleteOne(id)
                container.auth.updateUnread(res.unread)
                if (viewModelScope.isActive) {
                    _state.value = _state.value.copy(unreadCount = res.unread)
                }
            } catch (_: ApiException) {
                if (viewModelScope.isActive) {
                    load(reset = false)
                }
            }
        }
    }

    override fun onCleared() {
        pendingDeletes.keys.toList().forEach { commitDelete(it) }
        super.onCleared()
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

    fun openFor(item: NotificationItem, me: Me?): NotifDestination {
        if (!item.read) {
            toggleRead(item)
        }
        return if (me != null) {
            NotifLinkRouter.resolveFor(item.link, me)
        } else {
            NotifLinkRouter.resolve(item.link)
        }
    }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory {
            initializer { NotificationsViewModel(container) }
        }
    }
}

private data class NotifCategoryMeta(
    val key: String?,
    val labelRes: Int,
    val icon: ImageVector?,
)

private val categoryChips = listOf(
    NotifCategoryMeta(null, R.string.notif_cat_all, null),
    NotifCategoryMeta("ride", R.string.notif_cat_ride, Icons.Outlined.DirectionsCar),
    NotifCategoryMeta("event", R.string.notif_cat_event, Icons.Outlined.Event),
    NotifCategoryMeta("cost", R.string.notif_cat_cost, Icons.AutoMirrored.Outlined.ReceiptLong),
    NotifCategoryMeta("help", R.string.notif_cat_help, Icons.AutoMirrored.Outlined.HelpOutline),
    NotifCategoryMeta("family", R.string.notif_cat_family, Icons.Outlined.FavoriteBorder),
    NotifCategoryMeta("admin", R.string.notif_cat_admin, Icons.Outlined.AdminPanelSettings),
    NotifCategoryMeta("announcement", R.string.notif_cat_announcement, Icons.Outlined.Campaign),
)

private fun categoryLabelRes(category: String): Int? = NotificationHelper.categoryLabelRes(category)

private fun notifIconFor(item: NotificationItem): ImageVector = when {
    item.category == "chat" || item.eventType == "trip_chat" -> Icons.AutoMirrored.Outlined.Chat
    item.category == "ride" -> Icons.Outlined.DirectionsCar
    item.category == "event" -> Icons.Outlined.Event
    item.category == "cost" -> Icons.AutoMirrored.Outlined.ReceiptLong
    item.category == "help" -> Icons.AutoMirrored.Outlined.HelpOutline
    item.category == "family" -> Icons.Outlined.FavoriteBorder
    item.category == "admin" -> Icons.Outlined.AdminPanelSettings
    item.category == "announcement" -> Icons.Outlined.Campaign
    item.link?.contains("chat") == true -> Icons.AutoMirrored.Outlined.Chat
    item.link?.contains("ride") == true -> Icons.Outlined.DirectionsCar
    item.link?.contains("event") == true -> Icons.Outlined.Event
    item.link?.contains("cost") == true || item.link?.contains("settlement") == true ->
        Icons.AutoMirrored.Outlined.ReceiptLong
    else -> Icons.Outlined.Notifications
}

private fun notifTimestampText(createdAt: String, sameDayGroup: Boolean, locale: Locale): String {
    if (KlTime.parse(createdAt) == null) return createdAt
    return if (sameDayGroup) {
        KlTime.displayTime(createdAt)
    } else {
        "${KlTime.notificationDate(createdAt, locale)} · ${KlTime.displayTime(createdAt)}"
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NotificationsScreen(container: AppContainer, nav: NavHostController) {
    val vm: NotificationsViewModel = viewModel(factory = NotificationsViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val me = (authState as? AuthState.SignedIn)?.me ?: container.auth.me
    val canSeeAdmin = me?.hasStaffAccess() == true
    val visibleCategories = remember(canSeeAdmin) {
        categoryChips.filter { it.key != "admin" || canSeeAdmin }
    }
    val context = LocalContext.current
    val view = LocalView.current
    val scope = rememberCoroutineScope()
    val snackbar = remember { SnackbarHostState() }
    val composeLocale = LocalConfiguration.current.locales.let {
        if (it.isEmpty) Locale.getDefault() else it[0]
    }
    val lifecycleOwner = LocalLifecycleOwner.current
    var osNotifsEnabled by remember { mutableStateOf(NotificationHelper.areNotificationsEnabled(context)) }
    var dismissedOsBanner by remember { mutableStateOf(false) }
    var permDeniedOnce by remember { mutableStateOf(false) }
    val permLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted ->
        osNotifsEnabled = NotificationHelper.areNotificationsEnabled(context)
        if (!granted) permDeniedOnce = true
    }
    DisposableEffect(lifecycleOwner, context) {
        val obs = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) {
                osNotifsEnabled = NotificationHelper.areNotificationsEnabled(context)
            }
        }
        lifecycleOwner.lifecycle.addObserver(obs)
        onDispose { lifecycleOwner.lifecycle.removeObserver(obs) }
    }
    val hasUnread = state.unreadCount > 0 || state.items.any { !it.read }
    val deletedMsg = stringResource(R.string.notif_deleted)
    val undoLabel = stringResource(R.string.notif_undo)

    fun requestDelete(item: NotificationItem) {
        UiHaptics.tick(view)
        vm.stageDelete(item)
        scope.launch {
            snackbar.currentSnackbarData?.dismiss()
            val res = snackbar.showSnackbar(
                message = deletedMsg,
                actionLabel = undoLabel,
                withDismissAction = true,
                duration = SnackbarDuration.Short,
            )
            if (res == SnackbarResult.ActionPerformed) {
                vm.undoDelete(item.id)
            } else {
                vm.commitDelete(item.id)
            }
        }
    }

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
                        TextButton(onClick = { UiHaptics.tick(view); vm.markAllRead() }) {
                            Icon(
                                Icons.Outlined.DoneAll,
                                contentDescription = null,
                                modifier = Modifier.size(18.dp),
                            )
                            Spacer(Modifier.width(4.dp))
                            Text(stringResource(R.string.notif_mark_all_read))
                        }
                    }
                    IconButton(onClick = { nav.navigate(AppRoute.NotificationSettings) }) {
                        Icon(
                            Icons.Outlined.Settings,
                            contentDescription = stringResource(R.string.notif_settings_a11y),
                        )
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
                val unreadLabel = if (state.unreadCount > 0) {
                    "${stringResource(R.string.notif_filter_unread)} (${state.unreadCount})"
                } else {
                    stringResource(R.string.notif_filter_unread)
                }
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = Spacing.sm, vertical = Spacing.xxs),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    FilterChip(
                        selected = !state.unreadOnly,
                        onClick = { UiHaptics.tick(view); vm.setUnreadOnly(false) },
                        label = { Text(stringResource(R.string.notif_filter_all), maxLines = 1) },
                    )
                    FilterChip(
                        selected = state.unreadOnly,
                        onClick = { UiHaptics.tick(view); vm.setUnreadOnly(true) },
                        leadingIcon = if (state.unreadCount > 0 && !state.unreadOnly) {
                            {
                                Box(
                                    Modifier
                                        .size(7.dp)
                                        .clip(CircleShape)
                                        .background(MaterialTheme.colorScheme.primary),
                                )
                            }
                        } else {
                            null
                        },
                        label = { Text(unreadLabel, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                    )
                    Spacer(Modifier.weight(1f))
                    if (state.unreadOnly || state.category != null) {
                        TextButton(
                            onClick = { UiHaptics.tick(view); vm.resetFilters() },
                            contentPadding = PaddingValues(horizontal = Spacing.xs, vertical = 0.dp),
                        ) {
                            Text(stringResource(R.string.notif_reset_filters))
                        }
                    }
                }
                LazyRow(
                    modifier = Modifier.fillMaxWidth(),
                    contentPadding = PaddingValues(horizontal = Spacing.sm),
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    items(visibleCategories, key = { it.key ?: "all" }) { cat ->
                        val selected = state.category == cat.key
                        FilterChip(
                            selected = selected,
                            onClick = { UiHaptics.tick(view); vm.setCategory(cat.key) },
                            leadingIcon = cat.icon?.let { ic ->
                                {
                                    Icon(
                                        imageVector = ic,
                                        contentDescription = null,
                                        modifier = Modifier.size(16.dp),
                                    )
                                }
                            },
                            label = { Text(stringResource(cat.labelRes)) },
                        )
                    }
                }
                HorizontalDivider(
                    color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.5f),
                    modifier = Modifier.padding(top = Spacing.xxs),
                )

                AnimatedVisibility(
                    visible = !osNotifsEnabled && !dismissedOsBanner,
                    enter = expandVertically(tween(200)) + fadeIn(tween(180)),
                    exit = shrinkVertically(tween(160)) + fadeOut(tween(140)),
                ) {
                    val canRequestPrompt = Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
                        !NotificationHelper.hasPostPermission(context) &&
                        !permDeniedOnce
                    OutlinedCard(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                        border = BorderStroke(1.dp, MaterialTheme.colorScheme.error.copy(alpha = 0.45f)),
                    ) {
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(start = Spacing.sm, top = Spacing.xs, bottom = Spacing.xs, end = Spacing.xxs),
                            verticalAlignment = Alignment.Top,
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            Icon(
                                Icons.Outlined.NotificationsOff,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.error,
                                modifier = Modifier.padding(top = 2.dp).size(20.dp),
                            )
                            Column(
                                modifier = Modifier.weight(1f),
                                verticalArrangement = Arrangement.spacedBy(2.dp),
                            ) {
                                Text(
                                    stringResource(R.string.notif_os_disabled_title),
                                    style = MaterialTheme.typography.titleSmall,
                                    color = MaterialTheme.colorScheme.onSurface,
                                )
                                Text(
                                    stringResource(R.string.notif_os_disabled_body),
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                                TextButton(
                                    onClick = {
                                        UiHaptics.tick(view)
                                        if (canRequestPrompt) {
                                            permLauncher.launch(Manifest.permission.POST_NOTIFICATIONS)
                                        } else {
                                            runCatching {
                                                context.startActivity(NotificationHelper.openNotificationSettingsIntent(context))
                                            }
                                        }
                                    },
                                    contentPadding = PaddingValues(horizontal = 0.dp, vertical = 2.dp),
                                    modifier = Modifier.defaultMinSize(minHeight = 32.dp),
                                ) {
                                    Text(
                                        stringResource(
                                            if (canRequestPrompt) R.string.notif_os_enable_btn
                                            else R.string.notif_os_open_settings,
                                        ),
                                    )
                                }
                            }
                            IconButton(
                                onClick = { dismissedOsBanner = true },
                                modifier = Modifier.size(36.dp),
                            ) {
                                Icon(
                                    Icons.Outlined.Close,
                                    contentDescription = stringResource(R.string.common_close),
                                    modifier = Modifier.size(18.dp),
                                )
                            }
                        }
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
                        "loading" -> SkeletonLoadingList(count = 5)
                        "error" -> ErrorView(state.error ?: "", modifier = Modifier.fillMaxSize(), onRetry = { vm.load(reset = true) })
                        "empty" -> Box(
                            Modifier.fillMaxSize().verticalScroll(rememberScrollState()),
                            contentAlignment = Alignment.Center,
                        ) {
                            when {
                                state.unreadOnly && state.category == null -> Column(
                                    horizontalAlignment = Alignment.CenterHorizontally,
                                    verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                                    modifier = Modifier.padding(Spacing.xl),
                                ) {
                                    EmptyView(
                                        text = stringResource(R.string.notif_empty_unread),
                                        icon = Icons.Outlined.DoneAll,
                                    )
                                    FilledTonalButton(onClick = { UiHaptics.tick(view); vm.setUnreadOnly(false) }) {
                                        Text(stringResource(R.string.notif_show_all))
                                    }
                                }
                                state.category != null -> Column(
                                    horizontalAlignment = Alignment.CenterHorizontally,
                                    verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                                    modifier = Modifier.padding(Spacing.xl),
                                ) {
                                    EmptyView(
                                        text = stringResource(R.string.notif_empty_category),
                                        icon = Icons.Outlined.NotificationsNone,
                                    )
                                    FilledTonalButton(onClick = { UiHaptics.tick(view); vm.setCategory(null) }) {
                                        Text(stringResource(R.string.notif_show_all_topics))
                                    }
                                }
                                else -> Column(
                                    horizontalAlignment = Alignment.CenterHorizontally,
                                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    modifier = Modifier.padding(Spacing.xl),
                                ) {
                                    EmptyView(
                                        text = stringResource(R.string.notif_empty),
                                        icon = Icons.Outlined.NotificationsNone,
                                    )
                                    Text(
                                        stringResource(R.string.notif_empty_hint),
                                        style = MaterialTheme.typography.bodyMedium,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        textAlign = TextAlign.Center,
                                    )
                                }
                            }
                        }
                        else -> AdaptiveContentFrame(
                            maxWidth = 920.dp,
                            compactHorizontalPadding = 0.dp,
                            modifier = Modifier.fillMaxSize(),
                        ) {
                            LazyColumn(
                                Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                contentPadding = PaddingValues(top = Spacing.xs, bottom = Spacing.lg),
                            ) {
                            val groups = listOf(
                                Triple(R.string.notif_today, todayItems, true),
                                Triple(R.string.notif_yesterday, yesterdayItems, true),
                                Triple(R.string.notif_earlier, earlierItems, false),
                            )
                            groups.forEach { (headerRes, rows, sameDayGroup) ->
                                if (rows.isNotEmpty()) {
                                    item(key = "header-$headerRes") {
                                        Box(Modifier.animateItem().padding(top = Spacing.xxs)) {
                                            SectionHeader(stringResource(headerRes), count = rows.size)
                                        }
                                    }
                                    items(rows, key = { it.id }) { item ->
                                        val resolvedDest = remember(item.link, me) {
                                            if (me != null) {
                                                NotifLinkRouter.resolveFor(item.link, me)
                                            } else {
                                                NotifLinkRouter.resolve(item.link)
                                            }
                                        }
                                        NotificationRowCard(
                                            item = item,
                                            sameDayGroup = sameDayGroup,
                                            locale = composeLocale,
                                            showCategoryBadge = state.category == null && (item.category != "admin" || canSeeAdmin),
                                            resolvedDest = resolvedDest,
                                            restoreTick = state.restoreTick,
                                            onOpen = {
                                                when (val dest = vm.openFor(item, me)) {
                                                    is NotifDestination.Native -> {
                                                        val permitted = me?.let {
                                                            Routes.isLegacyRoutePermitted(dest.route, it)
                                                        } != false
                                                        if (permitted && dest.route != Routes.NOTIFICATIONS) {
                                                            nav.navigateRoute(dest.route) { launchSingleTop = true }
                                                        }
                                                    }
                                                    is NotifDestination.Web -> {
                                                        val permitted = me?.let {
                                                            Routes.isWebPathPermitted(dest.path, it)
                                                        } ?: false
                                                        if (permitted) {
                                                            openInWebApp(context, container.webUrl(dest.path))
                                                        }
                                                    }
                                                    NotifDestination.None -> {}
                                                }
                                            },
                                            onToggleRead = { UiHaptics.tick(view); vm.toggleRead(item) },
                                            onDelete = { requestDelete(item) },
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
                                        if (state.loadingMore) {
                                            InlineButtonSpinner()
                                        } else {
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
    }
}

@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class)
@Composable
private fun NotificationRowCard(
    item: NotificationItem,
    sameDayGroup: Boolean,
    locale: Locale,
    showCategoryBadge: Boolean,
    resolvedDest: NotifDestination,
    restoreTick: Int,
    onOpen: () -> Unit,
    onToggleRead: () -> Unit,
    onDelete: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val unread = !item.read
    val isUrgent = item.priority == "urgent"
    val isNavigable = when (resolvedDest) {
        is NotifDestination.Native -> resolvedDest.route != Routes.NOTIFICATIONS
        is NotifDestination.Web -> true
        NotifDestination.None -> false
    }
    var menuExpanded by remember { mutableStateOf(false) }
    var expanded by remember(item.id) { mutableStateOf(false) }
    var hasOverflow by remember(item.id, item.body) { mutableStateOf(false) }
    val readStateDesc = stringResource(if (unread) R.string.notif_a11y_unread else R.string.notif_a11y_read)

    val currentOnToggleRead by rememberUpdatedState(onToggleRead)
    val currentOnDelete by rememberUpdatedState(onDelete)

    @Suppress("DEPRECATION") // confirmValueChange lets StartToEnd snap back immediately after toggling read.
    val dismissState = rememberSwipeToDismissBoxState(
        confirmValueChange = { value ->
            when (value) {
                SwipeToDismissBoxValue.StartToEnd -> {
                    currentOnToggleRead()
                    false
                }
                SwipeToDismissBoxValue.EndToStart -> {
                    currentOnDelete()
                    true
                }
                SwipeToDismissBoxValue.Settled -> true
            }
        },
    )

    LaunchedEffect(restoreTick) {
        if (dismissState.currentValue != SwipeToDismissBoxValue.Settled) {
            dismissState.snapTo(SwipeToDismissBoxValue.Settled)
        }
    }

    val stripeColor = when {
        unread && isUrgent -> MaterialTheme.colorScheme.error
        unread -> MaterialTheme.colorScheme.primary
        else -> Color.Transparent
    }
    val border = when {
        unread && isUrgent -> BorderStroke(1.dp, MaterialTheme.colorScheme.error.copy(alpha = 0.55f))
        unread -> BorderStroke(1.dp, MaterialTheme.colorScheme.primary.copy(alpha = 0.42f))
        else -> BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.7f))
    }
    val containerColor = MaterialTheme.colorScheme.surface
    val (badgeBg, badgeTint) = when {
        isUrgent -> MaterialTheme.colorScheme.errorContainer to MaterialTheme.colorScheme.error
        !unread -> MaterialTheme.colorScheme.surfaceVariant to MaterialTheme.colorScheme.onSurfaceVariant
        item.category == "admin" -> MaterialTheme.colorScheme.errorContainer to MaterialTheme.colorScheme.error
        item.category == "event" || item.category == "family" ->
            MaterialTheme.colorScheme.tertiaryContainer to MaterialTheme.colorScheme.onTertiaryContainer
        item.category == "cost" ->
            MaterialTheme.colorScheme.secondaryContainer to MaterialTheme.colorScheme.onSecondaryContainer
        else -> MaterialTheme.colorScheme.primaryContainer to MaterialTheme.colorScheme.primary
    }

    SwipeToDismissBox(
        state = dismissState,
        modifier = modifier.fillMaxWidth(),
        backgroundContent = {
            val dir = dismissState.dismissDirection
            val targetBgColor = when (dir) {
                SwipeToDismissBoxValue.StartToEnd -> MaterialTheme.colorScheme.primaryContainer
                SwipeToDismissBoxValue.EndToStart -> MaterialTheme.colorScheme.errorContainer
                SwipeToDismissBoxValue.Settled -> Color.Transparent
            }
            val bgColor by animateColorAsState(
                targetValue = targetBgColor,
                animationSpec = if (animatorsEnabled()) tween(150) else snap(),
                label = "notif_swipe_bg",
            )
            val align = if (dir == SwipeToDismissBoxValue.StartToEnd) {
                Alignment.CenterStart
            } else {
                Alignment.CenterEnd
            }
            Surface(
                color = bgColor,
                shape = MaterialTheme.shapes.medium,
                modifier = Modifier.fillMaxSize(),
            ) {
                Box(
                    modifier = Modifier.fillMaxSize().padding(horizontal = Spacing.md),
                    contentAlignment = align,
                ) {
                    when (dir) {
                        SwipeToDismissBoxValue.StartToEnd -> Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            Icon(
                                imageVector = if (item.read) Icons.Outlined.MarkEmailUnread else Icons.Outlined.MarkEmailRead,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.onPrimaryContainer,
                            )
                            Text(
                                stringResource(if (item.read) R.string.notif_mark_unread else R.string.notif_mark_read),
                                style = MaterialTheme.typography.labelLarge,
                                color = MaterialTheme.colorScheme.onPrimaryContainer,
                            )
                        }
                        SwipeToDismissBoxValue.EndToStart -> Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            Text(
                                stringResource(R.string.notif_delete),
                                style = MaterialTheme.typography.labelLarge,
                                color = MaterialTheme.colorScheme.onErrorContainer,
                            )
                            Icon(
                                imageVector = Icons.Outlined.Delete,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.onErrorContainer,
                            )
                        }
                        SwipeToDismissBoxValue.Settled -> {}
                    }
                }
            }
        },
    ) {
        OutlinedCard(
            onClick = {
                if (isNavigable) {
                    onOpen()
                } else if (hasOverflow || expanded) {
                    expanded = !expanded
                    if (unread) onOpen()
                } else if (unread) {
                    onOpen()
                }
            },
            shape = MaterialTheme.shapes.medium,
            border = border,
            colors = CardDefaults.outlinedCardColors(containerColor = containerColor),
            modifier = Modifier
                .fillMaxWidth()
                .semantics { stateDescription = readStateDesc },
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(IntrinsicSize.Min),
            ) {
                Box(
                    Modifier
                        .width(3.5.dp)
                        .fillMaxHeight()
                        .background(stripeColor),
                )
                Row(
                    modifier = Modifier
                        .weight(1f)
                        .padding(start = Spacing.sm, top = Spacing.sm, bottom = Spacing.sm, end = Spacing.xs),
                    verticalAlignment = Alignment.Top,
                    horizontalArrangement = Arrangement.spacedBy(Spacing.sm),
                ) {
                    Surface(
                        color = badgeBg,
                        shape = CircleShape,
                        modifier = Modifier.padding(top = 2.dp).size(40.dp),
                    ) {
                        Box(contentAlignment = Alignment.Center) {
                            Icon(
                                imageVector = notifIconFor(item),
                                contentDescription = null,
                                tint = badgeTint,
                                modifier = Modifier.size(20.dp),
                            )
                        }
                    }

                    Column(
                        modifier = Modifier.weight(1f),
                        verticalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                        ) {
                            if (unread) {
                                Box(
                                    Modifier
                                        .size(7.dp)
                                        .clip(CircleShape)
                                        .background(
                                            if (isUrgent) MaterialTheme.colorScheme.error
                                            else MaterialTheme.colorScheme.primary,
                                        ),
                                )
                            }
                            Text(
                                item.title,
                                style = MaterialTheme.typography.titleSmall,
                                fontWeight = if (unread) FontWeight.SemiBold else FontWeight.Medium,
                                color = if (unread) MaterialTheme.colorScheme.onSurface else MaterialTheme.colorScheme.onSurfaceVariant,
                                maxLines = 2,
                                overflow = TextOverflow.Ellipsis,
                                modifier = Modifier.weight(1f),
                            )
                            Text(
                                notifTimestampText(item.createdAt, sameDayGroup, locale),
                                style = MaterialTheme.typography.labelSmall.tabular(),
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                maxLines = 1,
                            )
                            Box {
                                IconButton(
                                    onClick = { menuExpanded = true },
                                    modifier = Modifier.size(32.dp),
                                ) {
                                    Icon(
                                        Icons.Outlined.MoreVert,
                                        contentDescription = stringResource(R.string.notif_more_actions),
                                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                                        modifier = Modifier.size(18.dp),
                                    )
                                }
                                DropdownMenu(
                                    expanded = menuExpanded,
                                    onDismissRequest = { menuExpanded = false },
                                ) {
                                    DropdownMenuItem(
                                        text = {
                                            Text(
                                                stringResource(
                                                    if (item.read) R.string.notif_mark_unread else R.string.notif_mark_read,
                                                ),
                                            )
                                        },
                                        leadingIcon = {
                                            Icon(
                                                imageVector = if (item.read) Icons.Outlined.MarkEmailUnread else Icons.Outlined.MarkEmailRead,
                                                contentDescription = null,
                                            )
                                        },
                                        onClick = {
                                            menuExpanded = false
                                            onToggleRead()
                                        },
                                    )
                                    DropdownMenuItem(
                                        text = {
                                            Text(
                                                stringResource(R.string.notif_delete),
                                                color = MaterialTheme.colorScheme.error,
                                            )
                                        },
                                        leadingIcon = {
                                            Icon(
                                                Icons.Outlined.Delete,
                                                contentDescription = null,
                                                tint = MaterialTheme.colorScheme.error,
                                            )
                                        },
                                        onClick = {
                                            menuExpanded = false
                                            onDelete()
                                        },
                                    )
                                }
                            }
                        }

                        if (item.body.isNotBlank()) {
                            Text(
                                item.body,
                                style = MaterialTheme.typography.bodyMedium,
                                color = if (unread) {
                                    MaterialTheme.colorScheme.onSurface.copy(alpha = 0.85f)
                                } else {
                                    MaterialTheme.colorScheme.onSurfaceVariant
                                },
                                maxLines = if (expanded) Int.MAX_VALUE else 3,
                                overflow = TextOverflow.Ellipsis,
                                onTextLayout = { res ->
                                    if (!expanded) hasOverflow = res.hasVisualOverflow
                                },
                            )
                            if (hasOverflow || expanded) {
                                TextButton(
                                    onClick = { expanded = !expanded },
                                    contentPadding = PaddingValues(horizontal = 0.dp, vertical = 2.dp),
                                    modifier = Modifier.defaultMinSize(minHeight = 28.dp),
                                ) {
                                    Text(
                                        stringResource(
                                            if (expanded) R.string.common_show_less else R.string.common_read_more,
                                        ),
                                        style = MaterialTheme.typography.labelMedium,
                                    )
                                }
                            }
                        }

                        val catLabel = if (showCategoryBadge) categoryLabelRes(item.category) else null
                        if (isUrgent || catLabel != null || isNavigable) {
                            Row(
                                modifier = Modifier.fillMaxWidth().padding(top = 2.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                FlowRow(
                                    modifier = Modifier.weight(1f),
                                    horizontalArrangement = Arrangement.spacedBy(Spacing.xxs),
                                    verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                                ) {
                                    if (isUrgent) {
                                        Surface(
                                            color = MaterialTheme.colorScheme.errorContainer,
                                            contentColor = MaterialTheme.colorScheme.onErrorContainer,
                                            shape = CircleShape,
                                        ) {
                                            Text(
                                                stringResource(R.string.notif_urgent),
                                                style = MaterialTheme.typography.labelSmall,
                                                modifier = Modifier.padding(horizontal = Spacing.xs, vertical = 2.dp),
                                            )
                                        }
                                    }
                                    if (catLabel != null) {
                                        Surface(
                                            color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.75f),
                                            contentColor = MaterialTheme.colorScheme.onSurfaceVariant,
                                            shape = CircleShape,
                                        ) {
                                            Text(
                                                stringResource(catLabel),
                                                style = MaterialTheme.typography.labelSmall,
                                                modifier = Modifier.padding(horizontal = Spacing.xs, vertical = 2.dp),
                                            )
                                        }
                                    }
                                }
                                if (isNavigable) {
                                    val actionTint = if (unread) {
                                        MaterialTheme.colorScheme.primary
                                    } else {
                                        MaterialTheme.colorScheme.onSurfaceVariant
                                    }
                                    when (resolvedDest) {
                                        is NotifDestination.Native -> Row(
                                            verticalAlignment = Alignment.CenterVertically,
                                            horizontalArrangement = Arrangement.spacedBy(2.dp),
                                        ) {
                                            Text(
                                                stringResource(R.string.notif_action_view),
                                                style = MaterialTheme.typography.labelMedium,
                                                color = actionTint,
                                            )
                                            Icon(
                                                Icons.Outlined.ChevronRight,
                                                contentDescription = null,
                                                tint = actionTint,
                                                modifier = Modifier.size(16.dp),
                                            )
                                        }
                                        is NotifDestination.Web -> Row(
                                            verticalAlignment = Alignment.CenterVertically,
                                            horizontalArrangement = Arrangement.spacedBy(4.dp),
                                        ) {
                                            Text(
                                                stringResource(R.string.notif_action_open_web),
                                                style = MaterialTheme.typography.labelMedium,
                                                color = actionTint,
                                            )
                                            Icon(
                                                Icons.AutoMirrored.Outlined.OpenInNew,
                                                contentDescription = null,
                                                tint = actionTint,
                                                modifier = Modifier.size(14.dp),
                                            )
                                        }
                                        NotifDestination.None -> {}
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
