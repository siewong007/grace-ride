package org.pjstatecm.graceride.notifications

import android.content.Context
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.ProcessLifecycleOwner
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
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
import org.pjstatecm.graceride.data.repo.AuthRepository
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.data.repo.CommunityRepository
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean

class RealtimeStreamClient(
    private val okHttp: OkHttpClient,
    private val baseUrlProvider: () -> String,
    private val auth: AuthRepository,
    private val community: CommunityRepository,
    private val appContext: Context?,
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
    }

    private suspend fun runSseLoop() {
        var backoffMs = 2_000L
        val factory = EventSources.createFactory(sseClient)
        while (scope.isActive) {
            val url = baseUrlProvider().trimEnd('/') + "/api/stream"
            val req = Request.Builder().url(url).get().build()
            val closedSignal = kotlinx.coroutines.CompletableDeferred<Unit>()
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
                        closedSignal.complete(Unit)
                    }
                },
            )
            activeSource = source
            closedSignal.await()
            activeSource = null
            if (!scope.isActive) break
            delay(backoffMs)
            backoffMs = (backoffMs * 2).coerceAtMost(30_000L)
        }
    }

    private suspend fun syncLatestAndNotify(surfaceNew: Boolean) {
        if (auth.state.value !is AuthState.SignedIn) return
        syncMutex.withLock {
            try {
                val page = community.notificationsPage(limit = 5)
                auth.updateUnread(page.unread)
                val maxId = page.items.maxOfOrNull { it.id } ?: 0L
                if (!baselineInitialized) {
                    lastSeenNotifId = maxId
                    baselineInitialized = true
                    return@withLock
                }
                if (surfaceNew && appContext != null && maxId > lastSeenNotifId) {
                    page.items
                        .filter { it.id > lastSeenNotifId && !it.read }
                        .sortedBy { it.id }
                        .forEach { NotificationHelper.showNotification(appContext, it) }
                }
                if (maxId > lastSeenNotifId) {
                    lastSeenNotifId = maxId
                }
            } catch (_: Exception) {
                // Ignore transient errors during SSE background sync.
            }
        }
    }
}
