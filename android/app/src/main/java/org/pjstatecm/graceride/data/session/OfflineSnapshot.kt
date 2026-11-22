package org.pjstatecm.graceride.data.session

import kotlinx.serialization.Serializable
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.RideStatus

@Serializable
data class TodayRidesSnapshot(
    val userId: Long,
    val savedAtMillis: Long,
    val rides: List<Ride>,
)

object OfflineSnapshotStore {
    private const val KEY = "today_rides_snapshot_v1"

    fun save(
        store: SecureStore,
        userId: Long,
        rides: List<Ride>,
        nowMillis: Long = System.currentTimeMillis(),
    ) {
        val today = KlTime.today()
        val relevant = rides
            .filter { it.status in RideStatus.active || it.pickupAt.take(10) >= today }
            .distinctBy { it.id }
        if (relevant.isEmpty()) {
            store.remove(KEY)
            return
        }
        val snap = TodayRidesSnapshot(userId = userId, savedAtMillis = nowMillis, rides = relevant)
        store.putString(KEY, apiJson.encodeToString(TodayRidesSnapshot.serializer(), snap))
    }

    fun load(store: SecureStore, userId: Long): TodayRidesSnapshot? {
        val raw = store.getString(KEY) ?: return null
        return try {
            val snap = apiJson.decodeFromString(TodayRidesSnapshot.serializer(), raw)
            snap.takeIf { it.userId == userId && it.rides.isNotEmpty() }
        } catch (_: Exception) {
            null
        }
    }
}
