package org.pjstatecm.graceride.ui.screens.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.Announcement
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.Share
import org.pjstatecm.graceride.data.session.OfflineSnapshotStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.RideStatus

data class HomeUiState(
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val offline: Boolean = false,
    val offlineSnapshotAt: Long? = null,
    val error: String? = null,
    val me: Me? = null,
    val nextRide: Ride? = null,
    val drivingToday: List<Ride> = emptyList(),
    val nextUpcomingDrive: Ride? = null,
    val upcomingDrives: List<Ride> = emptyList(),
    val events: List<EventItem> = emptyList(),
    val unpaidShares: List<Share> = emptyList(),
    val announcements: List<Announcement> = emptyList(),
)

class HomeViewModel(private val container: AppContainer) : ViewModel() {

    private val _state = MutableStateFlow(HomeUiState())
    val state: StateFlow<HomeUiState> = _state.asStateFlow()

    init { load(first = true) }

    fun refresh() = load(first = false)

    private fun load(first: Boolean) {
        viewModelScope.launch {
            _state.value = _state.value.copy(
                loading = first && _state.value.me == null,
                refreshing = !first,
            )
            if (!first) {
                container.auth.refresh()
            }
            val me = container.auth.me
            var offline = false
            var error: String? = null

            suspend fun <T> section(allowed: Boolean, block: suspend () -> T, fallback: T): T {
                if (!allowed) return fallback
                return try {
                    block()
                } catch (e: ApiException) {
                    if (e.status == 0) offline = true else error = e.message
                    fallback
                }
            }

            var ridesOk = false
            var drivingOk = false
            val rides = section(me?.can("rides") == true, { container.rides.upcoming().also { ridesOk = true } }, emptyList<Ride>())
            val events = section(me?.can("events") == true, { container.events.upcoming() }, emptyList<EventItem>())
            val shares = section(me?.can("costs") == true, { container.shares.mine() }, emptyList<Share>())
            val ann = section(true, { container.community.announcements() }, emptyList<Announcement>())
            val today = KlTime.today()
            val nextWeek = KlTime.todayPlusDays(7)
            val drivingAll = section(me?.can("drive") == true, { container.rides.drivingToday(today, nextWeek).also { drivingOk = true } }, emptyList<Ride>())

            var effectiveRides = rides
            var effectiveDrivingAll = drivingAll
            var offlineSnapshotAt: Long? = null
            if (me != null) {
                if (ridesOk || drivingOk) {
                    OfflineSnapshotStore.save(container.secureStore, me.id, (rides + drivingAll).distinctBy { it.id })
                } else if (offline && rides.isEmpty() && drivingAll.isEmpty()) {
                    val snap = OfflineSnapshotStore.load(container.secureStore, me.id)
                    if (snap != null) {
                        effectiveRides = snap.rides.filter { it.viewer.isPassenger }
                        effectiveDrivingAll = snap.rides.filter { it.viewer.isDriver }
                        offlineSnapshotAt = snap.savedAtMillis
                    }
                }
            }

            val drivingToday = effectiveDrivingAll.filter { it.pickupAt.take(10) == today }
            val upcomingDrives = effectiveDrivingAll.filter { it.pickupAt.take(10) > today }.sortedBy { it.pickupAt }
            val nextUpcomingDrive = upcomingDrives.firstOrNull()

            val next = effectiveRides
                .filter { it.status in RideStatus.active && it.viewer.isPassenger }
                .minByOrNull { it.pickupAt }

            _state.value = HomeUiState(
                loading = false,
                refreshing = false,
                offline = offline,
                offlineSnapshotAt = offlineSnapshotAt,
                error = if (effectiveRides.isEmpty() && events.isEmpty() && ann.isEmpty() && effectiveDrivingAll.isEmpty()) error else null,
                me = me,
                nextRide = next,
                drivingToday = drivingToday,
                nextUpcomingDrive = nextUpcomingDrive,
                upcomingDrives = upcomingDrives,
                events = events.take(3),
                unpaidShares = shares.filter { it.status == "unpaid" }.take(3),
                announcements = ann.sortedByDescending { it.pinned }.take(3),
            )
        }
    }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory { initializer { HomeViewModel(container) } }
    }
}
