package org.pjstatecm.graceride.notifications

import android.content.Context
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.ProcessLifecycleOwner
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.sse.EventSource
import okhttp3.sse.EventSourceListener
import okhttp3.sse.EventSources
import org.pjstatecm.graceride.data.api.AuthEvent
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.NotificationDto
import org.pjstatecm.graceride.data.repo.AuthRepository
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.data.repo.CommunityRepository
import org.pjstatecm.graceride.data.session.SecureStore
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.random.Random

class RealtimeStreamClient(
    private val okHttp: OkHttpClient,
    private val baseUrlProvider: () -> String,
    private val auth: AuthRepository,
    private val community: CommunityRepository,
    private val appContext: Context?,
    private val authEvents: AuthEventBus? = null,
    private val langProvider: (() -> String)? = null,
    private val hideLockDetailsProvider: (() -> Boolean)? = null,
    private val store: SecureStore? = null,
) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val started = AtomicBoolean(false)
    private val isForeground = MutableStateFlow(true)
    private val syncMutex = Mutex()
    private val _refreshEvents = MutableSharedFlow<String>(extraBufferCapacity = 16)
    val refreshEvents: SharedFlow<String> = _refreshEvents.asSharedFlow()

    @Volatile
    private var activeSource: EventSource? = null
    @Volatile
    private var supervisorJob: Job? = null
    @Volatile
    private var lastSeenNotifId: Long = 0L
    @Volatile
    private var baselineInitialized: Boolean = false
    @Volatile
    private var lastUserId: Long? = null

    private val sseClient: OkHttpClient by lazy {
        okHttp.newBuilder()
            .readTimeout(0, TimeUnit.SECONDS)
            .build()
    }

    private val lifecycleObserver = object : DefaultLifecycleObserver {
        override fun onStart(owner: LifecycleOwner) {
            isForeground.value = true
            _refreshEvents.tryEmit("resume")
        }

        override fun onStop(owner: LifecycleOwner) {
            isForeground.value = false
        }
    }

    fun start() {
        if (appContext == null) return
        if (!started.compareAndSet(false, true)) return

        scope.launch(Dispatchers.Main) {
            ProcessLifecycleOwner.get().lifecycle.addObserver(lifecycleObserver)
        }

        supervisorJob = scope.launch {
            combine(auth.state, isForeground) { st, fg ->
                val uid = (st as? AuthState.SignedIn)?.me?.id
                if (uid != lastUserId) {
                    lastUserId = uid
                    val saved = uid?.let { readWatermark(store, it) }
                    if (saved != null) {
                        lastSeenNotifId = saved
                        baselineInitialized = true
                    } else {
                        lastSeenNotifId = 0L
                        baselineInitialized = false
                    }
                    if (uid != null && appContext != null) {
                        runCatching { BackgroundNotificationWorker.schedule(appContext) }
                    }
                }
                st is AuthState.SignedIn && fg
            }.distinctUntilChanged().collectLatest { shouldConnect ->
                activeSource?.cancel()
                activeSource = null
                if (shouldConnect) {
                    runSseLoop()
                }
            }
        }
    }

    fun stop() {
        activeSource?.cancel()
        activeSource = null
        supervisorJob?.cancel()
        supervisorJob = null
        if (started.compareAndSet(true, false) && appContext != null) {
            scope.launch(Dispatchers.Main.immediate) {
                ProcessLifecycleOwner.get().lifecycle.removeObserver(lifecycleObserver)
            }
        }
    }

    private suspend fun runSseLoop() {
        var backoffMs = 2_000L
        val factory = EventSources.createFactory(sseClient)
        while (currentCoroutineContext().isActive) {
            val url = baseUrlProvider().trimEnd('/') + "/api/stream"
            val req = Request.Builder().url(url).get().build()
            val closedSignal = kotlinx.coroutines.CompletableDeferred<Unit>()
            var terminalAuthStatus = false
            val source = factory.newEventSource(
                req,
                object : EventSourceListener() {
                    override fun onOpen(eventSource: EventSource, response: Response) {
                        backoffMs = 2_000L
                        scope.launch { syncLatestAndNotify(surfaceNew = false) }
                    }

                    override fun onEvent(
                        eventSource: EventSource,
                        id: String?,
                        type: String?,
                        data: String,
                    ) {
                        val trimmed = data.trim()
                        if (type == "unread") {
                            trimmed.toIntOrNull()?.let { auth.updateUnread(it) }
                            return
                        }
                        if (trimmed.isNotEmpty()) {
                            _refreshEvents.tryEmit(trimmed)
                            if (trimmed == "notify") {
                                scope.launch { syncLatestAndNotify(surfaceNew = true) }
                            }
                        }
                    }

                    override fun onClosed(eventSource: EventSource) {
                        closedSignal.complete(Unit)
                    }

                    override fun onFailure(
                        eventSource: EventSource,
                        t: Throwable?,
                        response: Response?,
                    ) {
                        when (response?.code) {
                            401 -> {
                                terminalAuthStatus = true
                                auth.onUnauthorized()
                                authEvents?.emit(AuthEvent.Unauthorized)
                            }
                            423 -> {
                                terminalAuthStatus = true
                                auth.onLocked()
                                authEvents?.emit(AuthEvent.Locked)
                            }
                        }
                        closedSignal.complete(Unit)
                    }
                },
            )
            activeSource = source
            try {
                closedSignal.await()
            } finally {
                source.cancel()
                if (activeSource === source) activeSource = null
            }
            if (terminalAuthStatus || !currentCoroutineContext().isActive) break
            val jitterMs = Random.nextLong(0L, (backoffMs / 4).coerceAtLeast(1L))
            delay(backoffMs + jitterMs)
            backoffMs = (backoffMs * 2).coerceAtMost(30_000L)
        }
    }

    private suspend fun syncLatestAndNotify(surfaceNew: Boolean) {
        val uid = (auth.state.value as? AuthState.SignedIn)?.me?.id ?: return
        syncMutex.withLock {
            try {
                val prevId = if (store != null) readWatermark(store, uid) else if (baselineInitialized) lastSeenNotifId else null
                val result = syncNotifications(
                    community = community,
                    auth = auth,
                    store = store,
                    userId = uid,
                    inMemoryLastSeenId = prevId,
                    appContext = appContext,
                    surfaceNew = surfaceNew,
                    lang = langProvider?.invoke(),
                    hideLockDetails = hideLockDetailsProvider?.invoke() ?: false,
                )
                lastSeenNotifId = result.newWatermark
                baselineInitialized = true
            } catch (_: Exception) {
                // Ignore transient errors during SSE background sync.
            }
        }
    }

    data class SyncResult(
        val newWatermark: Long,
        val surfaced: List<NotificationDto>,
    )

    companion object {
        fun watermarkKey(userId: Long): String = "notif_last_seen_id_$userId"

        fun readWatermark(store: SecureStore?, userId: Long): Long? =
            store?.getString(watermarkKey(userId))?.toLongOrNull()

        fun writeWatermark(store: SecureStore?, userId: Long, id: Long) {
            store?.putString(watermarkKey(userId), id.toString())
        }

        /**
         * Fetch the latest notifications page, update the unread badge count, maintain the
         * per-user watermark in [store], and emit OS notifications for unread items newer than
         * the previous watermark when [surfaceNew] is true.
         */
        suspend fun syncNotifications(
            community: CommunityRepository,
            auth: AuthRepository,
            store: SecureStore?,
            userId: Long,
            inMemoryLastSeenId: Long? = null,
            appContext: Context? = null,
            surfaceNew: Boolean = true,
            lang: String? = null,
            hideLockDetails: Boolean = false,
        ): SyncResult {
            val page = community.notificationsPage(limit = 5)
            auth.updateUnread(page.unread)
            val maxId = page.items.maxOfOrNull { it.id } ?: 0L
            val existing = readWatermark(store, userId) ?: inMemoryLastSeenId
            if (existing == null) {
                writeWatermark(store, userId, maxId)
                return SyncResult(newWatermark = maxId, surfaced = emptyList())
            }
            val newlySurfaced = if (surfaceNew && maxId > existing) {
                page.items
                    .filter { it.id > existing && !it.read }
                    .sortedBy { it.id }
            } else {
                emptyList()
            }
            if (appContext != null) {
                newlySurfaced.forEach {
                    NotificationHelper.showNotification(
                        appContext,
                        it,
                        lang = lang,
                        hideLockDetails = hideLockDetails,
                    )
                }
            }
            val nextWatermark = maxOf(existing, maxId)
            if (nextWatermark > existing) {
                writeWatermark(store, userId, nextWatermark)
            }
            return SyncResult(newWatermark = nextWatermark, surfaced = newlySurfaced)
        }
    }
}
