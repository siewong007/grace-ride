package org.pjstatecm.graceride.data.cache

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.json.Json
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.repo.RidesRepository
import org.pjstatecm.graceride.data.session.SecureStore
import java.util.UUID

@Serializable
data class QueuedRideAction(
    val id: String = UUID.randomUUID().toString(),
    val rideId: Long,
    val kind: String,
    val minutes: Int? = null,
    val readinessStatus: String? = null,
    val notes: String? = null,
    val createdAtMs: Long = System.currentTimeMillis(),
)

/**
 * Durable offline queue for high-frequency driver trip status & passenger readiness transitions.
 * Actions triggered offline (`ApiException.status == 0`) are deduplicated per `(rideId, kind)` and
 * flushed in FIFO order when connectivity returns or when the user taps "Sync now".
 */
class OfflineActionQueue(private val store: SecureStore?) {
    private val json = Json { ignoreUnknownKeys = true; encodeDefaults = true }
    private val mutex = Mutex()

    private val _pending = MutableStateFlow(loadInitial())
    val pending: StateFlow<List<QueuedRideAction>> = _pending.asStateFlow()

    private val _syncing = MutableStateFlow(false)
    val syncing: StateFlow<Boolean> = _syncing.asStateFlow()

    private fun loadInitial(): List<QueuedRideAction> {
        val raw = store?.getString(STORAGE_KEY) ?: return emptyList()
        return runCatching {
            json.decodeFromString(ListSerializer(QueuedRideAction.serializer()), raw)
        }.getOrDefault(emptyList())
    }

    private fun persist(list: List<QueuedRideAction>) {
        _pending.value = list
        val localStore = store ?: return
        if (list.isEmpty()) {
            localStore.remove(STORAGE_KEY)
        } else {
            runCatching {
                localStore.putString(STORAGE_KEY, json.encodeToString(ListSerializer(QueuedRideAction.serializer()), list))
            }
        }
    }

    fun enqueue(
        rideId: Long,
        kind: String,
        minutes: Int? = null,
        readinessStatus: String? = null,
        notes: String? = null,
    ): QueuedRideAction {
        val item = QueuedRideAction(
            id = UUID.randomUUID().toString(),
            rideId = rideId,
            kind = kind,
            minutes = minutes,
            readinessStatus = readinessStatus,
            notes = notes,
            createdAtMs = System.currentTimeMillis(),
        )
        val filtered = _pending.value.filterNot { it.rideId == rideId && it.kind == kind }
        persist(filtered + item)
        return item
    }

    fun pendingForRide(rideId: Long): List<QueuedRideAction> =
        _pending.value.filter { it.rideId == rideId }

    fun clear() {
        persist(emptyList())
    }

    /**
     * Replays queued items in FIFO order.
     * - Successful replays and 4xx client/conflict errors (e.g. `409 RIDE_STATUS_CHANGED`) remove the item.
     * - Network errors (`status == 0`) or 5xx errors keep the remaining queue intact and stop flushing.
     */
    suspend fun flush(rides: RidesRepository): Int = mutex.withLock {
        val initial = _pending.value
        if (initial.isEmpty()) return@withLock 0
        _syncing.value = true
        var syncedCount = 0
        try {
            var remaining = initial
            for (action in initial) {
                try {
                    replayAction(rides, action)
                    syncedCount += 1
                    remaining = remaining.filterNot { it.id == action.id }
                    persist(remaining)
                } catch (c: CancellationException) {
                    throw c
                } catch (e: ApiException) {
                    if (e.status in 400..499) {
                        // Stale transition or already superseded on server; discard instead of blocking later actions.
                        remaining = remaining.filterNot { it.id == action.id }
                        persist(remaining)
                    } else {
                        break
                    }
                }
            }
            syncedCount
        } finally {
            _syncing.value = false
        }
    }

    private suspend fun replayAction(rides: RidesRepository, action: QueuedRideAction) {
        when (action.kind) {
            KIND_START -> rides.startRide(action.rideId)
            KIND_ARRIVED_GATE -> rides.arrivedGate(action.rideId)
            KIND_PICKUP -> rides.pickupRide(action.rideId)
            KIND_COMPLETE -> rides.completeRide(action.rideId)
            KIND_DELAY -> rides.driverDelay(action.rideId, action.minutes ?: 10)
            KIND_READINESS -> rides.readiness(action.rideId, action.readinessStatus ?: "ready", action.notes)
        }
    }

    companion object {
        const val STORAGE_KEY = "offline_action_queue_v1"
        const val KIND_START = "start"
        const val KIND_ARRIVED_GATE = "arrived_gate"
        const val KIND_PICKUP = "pickup"
        const val KIND_COMPLETE = "complete"
        const val KIND_DELAY = "delay"
        const val KIND_READINESS = "readiness"
    }
}
