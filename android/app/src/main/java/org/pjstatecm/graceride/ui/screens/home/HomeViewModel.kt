package org.pjstatecm.graceride.ui.screens.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.serialization.builtins.ListSerializer
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
import org.pjstatecm.graceride.notifications.RealtimeEvent

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
    val unpaidShareCount: Int = 0,
    val announcements: List<Announcement> = emptyList(),
)

class HomeViewModel(private val container: AppContainer) : ViewModel() {

    private val _state = MutableStateFlow(HomeUiState())
    val state: StateFlow<HomeUiState> = _state.asStateFlow()
    private var loadJob: Job? = null

    init {
        load(first = true)
        viewModelScope.launch {
            container.realtimeStream.events.collect { ev ->
                if (ev != RealtimeEvent.Resume) refresh()
            }
        }
        viewModelScope.launch {
            container.networkMonitor.isOnline.drop(1).filter { it }.collect { refresh() }
        }
    }

    fun refresh() = load(first = false)

    private fun load(first: Boolean) {
        loadJob?.cancel()
        loadJob = viewModelScope.launch {
            _state.value = _state.value.copy(
                loading = first && _state.value.me == null,
                refreshing = !first,
            )
            if (!first) {
                container.auth.refresh()
            }
            val me = container.auth.me

            data class SectionResult<T>(
                val data: T,
                val ok: Boolean = false,
                val offline: Boolean = false,
                val error: String? = null,
            )

            suspend fun <T> section(allowed: Boolean, block: suspend () -> T, fallback: T): SectionResult<T> {
                if (!allowed) return SectionResult(fallback)
                return try {
                    SectionResult(block(), ok = true)
                } catch (e: ApiException) {
                    SectionResult(fallback, ok = false, offline = e.status == 0, error = if (e.status == 0) null else e.message)
                }
            }

            val today = KlTime.today()
            val nextWeek = KlTime.todayPlusDays(7)
            val (ridesRes, eventsRes, sharesRes, annRes, drivingRes) = coroutineScope {
                val rDef = async { section(me?.can("rides") == true, { container.rides.upcoming() }, emptyList<Ride>()) }
                val eDef = async { section(me?.can("events") == true, { container.events.upcoming() }, emptyList<EventItem>()) }
                val sDef = async { section(me?.can("costs") == true, { container.shares.mine() }, emptyList<Share>()) }
                val aDef = async { section(true, { container.community.announcements() }, emptyList<Announcement>()) }
                val dDef = async { section(me?.can("drive") == true, { container.rides.drivingToday(today, nextWeek) }, emptyList<Ride>()) }
                listOf(rDef.await(), eDef.await(), sDef.await(), aDef.await(), dDef.await()).let {
                    @Suppress("UNCHECKED_CAST")
                    Tuple5(
                        it[0] as SectionResult<List<Ride>>,
                        it[1] as SectionResult<List<EventItem>>,
                        it[2] as SectionResult<List<Share>>,
                        it[3] as SectionResult<List<Announcement>>,
                        it[4] as SectionResult<List<Ride>>,
                    )
                }
            }

            val rides = ridesRes.data
            val events = eventsRes.data
            val shares = sharesRes.data
            val ann = annRes.data
            val drivingAll = drivingRes.data
            val offline = ridesRes.offline || eventsRes.offline || sharesRes.offline || annRes.offline || drivingRes.offline
            val error = listOfNotNull(ridesRes.error, eventsRes.error, sharesRes.error, annRes.error, drivingRes.error).firstOrNull()

            var effectiveRides = rides
            var effectiveDrivingAll = drivingAll
            var effectiveEvents = events
            var effectiveAnn = ann
            var offlineSnapshotAt: Long? = null
            if (me != null) {
                if (ridesRes.ok || drivingRes.ok) {
                    val combined = (rides + drivingAll).distinctBy { it.id }
                    withContext(Dispatchers.IO) {
                        OfflineSnapshotStore.save(container.secureStore, me.id, combined)
                    }
                } else if (offline && rides.isEmpty() && drivingAll.isEmpty()) {
                    val snap = withContext(Dispatchers.IO) {
                        OfflineSnapshotStore.load(container.secureStore, me.id)
                    }
                    if (snap != null) {
                        effectiveRides = snap.rides.filter { it.viewer.isPassenger }
                        effectiveDrivingAll = snap.rides.filter { it.viewer.isDriver }
                        offlineSnapshotAt = snap.savedAtMillis
                    }
                }
                if (eventsRes.ok) {
                    withContext(Dispatchers.IO) {
                        container.offlineCache.write(me.id, "home_events", events, ListSerializer(EventItem.serializer()))
                    }
                } else if (offline && effectiveEvents.isEmpty()) {
                    withContext(Dispatchers.IO) {
                        container.offlineCache.read(me.id, "home_events", ListSerializer(EventItem.serializer()))
                    }?.let { effectiveEvents = it.data }
                }
                if (annRes.ok) {
                    withContext(Dispatchers.IO) {
                        container.offlineCache.write(me.id, "home_announcements", ann, ListSerializer(Announcement.serializer()))
                    }
                } else if (offline && effectiveAnn.isEmpty()) {
                    withContext(Dispatchers.IO) {
                        container.offlineCache.read(me.id, "home_announcements", ListSerializer(Announcement.serializer()))
                    }?.let { effectiveAnn = it.data }
                }
            }

            val drivingToday = effectiveDrivingAll.filter { it.pickupAt.take(10) == today }
            val upcomingDrives = effectiveDrivingAll
                .filter { it.pickupAt.take(10) > today && it.status in RideStatus.active }
                .sortedBy { it.pickupAt }
            val nextUpcomingDrive = upcomingDrives.firstOrNull()

            val next = effectiveRides
                .filter { it.status in RideStatus.active && it.viewer.isPassenger }
                .minByOrNull { it.pickupAt }
            val allUnpaid = shares.filter { it.status == "unpaid" || it.status == "needs_info" }
            val unpaidShares = allUnpaid.take(3)

            _state.value = HomeUiState(
                loading = false,
                refreshing = false,
                offline = offline,
                offlineSnapshotAt = offlineSnapshotAt,
                error = if (effectiveRides.isEmpty() && effectiveEvents.isEmpty() && unpaidShares.isEmpty() && effectiveAnn.isEmpty() && effectiveDrivingAll.isEmpty()) error else null,
                me = me,
                nextRide = next,
                drivingToday = drivingToday,
                nextUpcomingDrive = nextUpcomingDrive,
                upcomingDrives = upcomingDrives,
                events = effectiveEvents.take(3),
                unpaidShares = unpaidShares,
                unpaidShareCount = allUnpaid.size,
                announcements = effectiveAnn.sortedByDescending { it.pinned }.take(3),
            )
            container.appContext?.let { ctx ->
                withContext(Dispatchers.IO) {
                    val hideLockDetails = container.secureStore.getString("notif_hide_lock_details") == "1"
                    org.pjstatecm.graceride.widget.WidgetSnapshotStore.saveFromHome(
                        ctx,
                        signedIn = me != null,
                        nextRide = next,
                        nextEvent = effectiveEvents.firstOrNull(),
                        hideLockDetails = hideLockDetails,
                    )
                    org.pjstatecm.graceride.widget.refreshGraceRideWidget(ctx)
                }
            }
        }
    }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory { initializer { HomeViewModel(container) } }
    }
}

private data class Tuple5<A, B, C, D, E>(val a: A, val b: B, val c: C, val d: D, val e: E)
