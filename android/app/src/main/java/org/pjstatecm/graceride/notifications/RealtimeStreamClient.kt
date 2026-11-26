package org.pjstatecm.graceride.notifications

import android.content.Context
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.ProcessLifecycleOwner
import kotlinx.coroutines.CancellationException
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
import org.pjstatecm.graceride.data.api.NotificationItem
import org.pjstatecm.graceride.data.repo.AuthRepository
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.data.repo.CommunityRepository
import org.pjstatecm.graceride.data.session.SecureStore
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.random.Random

sealed interface RealtimeEvent {
    data object Chat : RealtimeEvent
    data object Notify : RealtimeEvent
    data object Resume : RealtimeEvent
    data class Other(val kind: String) : RealtimeEvent

    companion object {
        fun parse(kind: String): RealtimeEvent = when (kind) {
            "chat" -> Chat
            "notify" -> Notify
            "resume" -> Resume
            else -> Other(kind)
        }
    }
}

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
    private val biometric: org.pjstatecm.graceride.platform.BiometricAuthManager? = null,
) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val started = AtomicBoolean(false)
    private val isForeground = MutableStateFlow(false)
    private val syncMutex = Mutex()
    private val _refreshEvents = MutableSharedFlow<String>(extraBufferCapacity = 16)
    val refreshEvents: SharedFlow<String> = _refreshEvents.asSharedFlow()
    private val _events = MutableSharedFlow<RealtimeEvent>(extraBufferCapacity = 16)
    val events: SharedFlow<RealtimeEvent> = _events.asSharedFlow()

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
            .cache(null)
            .readTimeout(45, TimeUnit.SECONDS)
            .build()
    }

    private val lifecycleObserver = object : DefaultLifecycleObserver {
        override fun onStart(owner: LifecycleOwner) {
            isForeground.value = true
            _refreshEvents.tryEmit("resume")
            _events.tryEmit(RealtimeEvent.Resume)
        }

        override fun onStop(owner: LifecycleOwner) {
            isForeground.value = false
        }
    }

    fun start() {
        if (appContext == null) return
        if (!started.compareAndSet(false, true)) return

        scope.launch(Dispatchers.Main.immediate) {
            if (!started.get()) return@launch
            val lifecycle = ProcessLifecycleOwner.get().lifecycle
            isForeground.value = lifecycle.currentState.isAtLeast(Lifecycle.State.STARTED)
            lifecycle.addObserver(lifecycleObserver)
        }

        supervisorJob = scope.launch {
            val bioEnrollment = biometric?.enrollment
                ?: MutableStateFlow<org.pjstatecm.graceride.data.api.BiometricEnrollment?>(null)
            val bioUnlocked = biometric?.appUnlocked ?: MutableStateFlow(true)
            combine(auth.state, isForeground, bioEnrollment, bioUnlocked) { st, fg, enrollment, unlocked ->
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
                    if (uid != null) {
                        runCatching { BackgroundNotificationWorker.schedule(appContext) }
                    }
                }
                val gateBlocked = uid != null && enrollment?.userId == uid && !unlocked
                st is AuthState.SignedIn && st.me.termsAccepted && fg && !gateBlocked
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
                if (started.get()) return@launch
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
                            _events.tryEmit(RealtimeEvent.parse(trimmed))
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
                                if (authEvents != null) {
                                    authEvents.emit(AuthEvent.Unauthorized)
                                } else {
                                    auth.onUnauthorized()
                                }
                            }
                            423 -> {
                                terminalAuthStatus = true
                                if (authEvents != null) {
                                    authEvents.emit(AuthEvent.Locked)
                                } else {
                                    auth.onLocked()
                                }
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
            } catch (ce: CancellationException) {
                throw ce
            } catch (_: Exception) {
                // Ignore transient errors during SSE background sync.
            }
        }
    }

    data class SyncResult(
        val newWatermark: Long,
        val surfaced: List<NotificationItem>,
    )

    companion object {
        private val globalSyncMutex = Mutex()

        fun watermarkKey(userId: Long): String = "notif_last_seen_id_$userId"

        fun readWatermark(store: SecureStore?, userId: Long): Long? =
            store?.getString(watermarkKey(userId))?.toLongOrNull()

        fun writeWatermark(store: SecureStore?, userId: Long, id: Long) {
            store?.putString(watermarkKey(userId), id.toString())
        }

        fun readCachedPrefs(store: SecureStore?): org.pjstatecm.graceride.data.api.NotificationPrefsDto? {
            val raw = store?.getString(AuthRepository.NOTIF_PREFS_KEY) ?: return null
            return runCatching {
                org.pjstatecm.graceride.data.api.apiJson.decodeFromString(
                    org.pjstatecm.graceride.data.api.NotificationPrefsDto.serializer(),
                    raw,
                )
            }.getOrNull()
        }

        fun writeCachedPrefs(store: SecureStore?, prefs: org.pjstatecm.graceride.data.api.NotificationPrefsDto) {
            store?.putString(
                AuthRepository.NOTIF_PREFS_KEY,
                org.pjstatecm.graceride.data.api.apiJson.encodeToString(
                    org.pjstatecm.graceride.data.api.NotificationPrefsDto.serializer(),
                    prefs,
                ),
            )
            store?.putString("notif_hide_lock_details", if (prefs.hideLockDetails) "1" else "0")
        }

        private fun parseHmMinutes(hm: String?): Int? {
            val parts = hm?.trim()?.split(':') ?: return null
            if (parts.size != 2) return null
            val h = parts[0].toIntOrNull() ?: return null
            val m = parts[1].toIntOrNull() ?: return null
            if (h !in 0..23 || m !in 0..59) return null
            return h * 60 + m
        }

        fun shouldSurfaceNotification(
            item: NotificationItem,
            prefs: org.pjstatecm.graceride.data.api.NotificationPrefsDto?,
            nowHm: String = java.time.ZonedDateTime.now(org.pjstatecm.graceride.domain.KlTime.zone)
                .let { String.format(java.util.Locale.US, "%02d:%02d", it.hour, it.minute) },
        ): Boolean {
            if (prefs == null) return true
            if (!prefs.pushOptIn) return false
            if (item.priority == "urgent") return true
            if (item.category in prefs.mutedCategories) return false
            val startMin = parseHmMinutes(prefs.quietHoursStart)
            val endMin = parseHmMinutes(prefs.quietHoursEnd)
            val nowMin = parseHmMinutes(nowHm)
            if (startMin != null && endMin != null && nowMin != null && startMin != endMin) {
                val inQuiet = if (startMin < endMin) {
                    nowMin in startMin until endMin
                } else {
                    nowMin >= startMin || nowMin < endMin
                }
                if (inQuiet) return false
            }
            return true
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
            prefs: org.pjstatecm.graceride.data.api.NotificationPrefsDto? = readCachedPrefs(store),
            passive: Boolean = false,
        ): SyncResult = globalSyncMutex.withLock {
            val page = community.notificationsPage(limit = 15, passive = passive)
            auth.updateUnread(page.unread)
            val maxId = page.items.maxOfOrNull { it.id } ?: 0L
            val existing = readWatermark(store, userId) ?: inMemoryLastSeenId
            if (existing == null) {
                writeWatermark(store, userId, maxId)
                return@withLock SyncResult(newWatermark = maxId, surfaced = emptyList())
            }
            val effectiveHideLock = hideLockDetails || (prefs?.hideLockDetails == true)
            val newlySurfaced = if (surfaceNew && maxId > existing) {
                page.items
                    .filter { it.id > existing && !it.read && shouldSurfaceNotification(it, prefs) }
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
                        hideLockDetails = effectiveHideLock,
                    )
                }
                if (newlySurfaced.size >= 2) {
                    NotificationHelper.showGroupSummary(
                        appContext,
                        newlySurfaced,
                        lang = lang,
                        hideLockDetails = effectiveHideLock,
                    )
                }
            }
            val nextWatermark = maxOf(existing, maxId)
            if (nextWatermark > existing) {
                writeWatermark(store, userId, nextWatermark)
            }
            SyncResult(newWatermark = nextWatermark, surfaced = newlySurfaced)
        }
    }
}
