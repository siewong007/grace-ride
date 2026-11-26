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
import org.pjstatecm.graceride.data.api.AdminDashboardStatsDto
import org.pjstatecm.graceride.data.api.Announcement
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.ProfileUpdateReq
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.Share
import org.pjstatecm.graceride.data.api.runSuspendCatching
import org.pjstatecm.graceride.data.session.OfflineSnapshotStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.RideStatus
import org.pjstatecm.graceride.notifications.RealtimeEvent
import org.pjstatecm.graceride.ui.Routes

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
    val trafficStats: AdminDashboardStatsDto? = null,
    val onboardingLoaded: Boolean = false,
    val onboardingHasPlace: Boolean = false,
    val onboardingHasRide: Boolean = false,
    val onboardingHasEvent: Boolean = false,
    val onboardingHasVehicle: Boolean = false,
)

class HomeViewModel(private val container: AppContainer) : ViewModel() {

    private val _state = MutableStateFlow(HomeUiState())
    val state: StateFlow<HomeUiState> = _state.asStateFlow()
    private var loadJob: Job? = null
    private var onboardingJob: Job? = null

    init {
        load(first = true)
        viewModelScope.launch {
            container.realtimeStream.events.collect { ev ->
                if (ev != RealtimeEvent.Resume && ev != RealtimeEvent.Chat) refresh()
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
            var trafficStats: AdminDashboardStatsDto? = null
            val (ridesRes, eventsRes, sharesRes, annRes, drivingRes) = coroutineScope {
                val rDef = async { section(me?.can("rides") == true, { container.rides.upcoming() }, emptyList<Ride>()) }
                val eDef = async { section(me?.can("events") == true, { container.events.upcoming() }, emptyList<EventItem>()) }
                val sDef = async { section(me?.can("costs") == true, { container.shares.mine() }, emptyList<Share>()) }
                val aDef = async { section(me != null, { container.community.announcements() }, emptyList<Announcement>()) }
                val dDef = async { section(me?.can("drive") == true, { container.rides.drivingToday(today, nextWeek) }, emptyList<Ride>()) }
                val tDef = async {
                    if (me != null && (me.can("admin_dashboard") || Routes.canTrafficHub(me) || Routes.canAdminMembers(me))) {
                        container.traffic.dashboardStats(passive = !first).getOrNull()
                    } else {
                        null
                    }
                }
                trafficStats = tDef.await()
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

            val prev = _state.value
            val sameUser = !first && me != null && prev.me?.id == me.id
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
            var usedOfflineRidesSnap = false
            var usedOfflineEventsCache = false
            var usedOfflineAnnCache = false
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
                        usedOfflineRidesSnap = true
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
                    }?.let {
                        effectiveEvents = it.data
                        usedOfflineEventsCache = true
                    }
                }
                if (annRes.ok) {
                    withContext(Dispatchers.IO) {
                        container.offlineCache.write(me.id, "home_announcements", ann, ListSerializer(Announcement.serializer()))
                    }
                } else if (offline && effectiveAnn.isEmpty()) {
                    withContext(Dispatchers.IO) {
                        container.offlineCache.read(me.id, "home_announcements", ListSerializer(Announcement.serializer()))
                    }?.let {
                        effectiveAnn = it.data
                        usedOfflineAnnCache = true
                    }
                }
            }

            val preserveDriving = sameUser && me?.can("drive") == true && !drivingRes.ok && !usedOfflineRidesSnap
            val drivingToday = if (preserveDriving) {
                prev.drivingToday
            } else {
                effectiveDrivingAll.filter { it.pickupAt.take(10) == today }.sortedBy { KlTime.sortKey(it.pickupAt) }
            }
            val upcomingDrives = if (preserveDriving) {
                prev.upcomingDrives
            } else {
                effectiveDrivingAll
                    .filter { it.pickupAt.take(10) > today && it.status in RideStatus.active }
                    .sortedBy { KlTime.sortKey(it.pickupAt) }
            }
            val nextUpcomingDrive = if (preserveDriving) prev.nextUpcomingDrive else upcomingDrives.firstOrNull()

            val next = if (sameUser && me?.can("rides") == true && !ridesRes.ok && !usedOfflineRidesSnap) {
                prev.nextRide
            } else {
                effectiveRides
                    .filter { it.status in RideStatus.active && it.viewer.isPassenger }
                    .minByOrNull { KlTime.sortKey(it.pickupAt) }
            }
            val preserveShares = sameUser && me?.can("costs") == true && !sharesRes.ok
            val allUnpaid = shares.filter { it.status == "unpaid" || it.status == "needs_info" }
            val unpaidShares = if (preserveShares) prev.unpaidShares else allUnpaid.take(3)
            val unpaidShareCount = if (preserveShares) prev.unpaidShareCount else allUnpaid.size
            val finalEvents = if (sameUser && me?.can("events") == true && !eventsRes.ok && !usedOfflineEventsCache) {
                prev.events
            } else {
                effectiveEvents.take(3)
            }
            val finalAnn = if (sameUser && !annRes.ok && !usedOfflineAnnCache) {
                prev.announcements
            } else {
                effectiveAnn.sortedByDescending { it.pinned }.take(3)
            }

            _state.value = HomeUiState(
                loading = false,
                refreshing = false,
                offline = offline,
                offlineSnapshotAt = offlineSnapshotAt,
                error = if (next == null && finalEvents.isEmpty() && unpaidShares.isEmpty() && finalAnn.isEmpty() && drivingToday.isEmpty() && nextUpcomingDrive == null) error else null,
                me = me,
                nextRide = next,
                drivingToday = drivingToday,
                nextUpcomingDrive = nextUpcomingDrive,
                upcomingDrives = upcomingDrives,
                events = finalEvents,
                unpaidShares = unpaidShares,
                unpaidShareCount = unpaidShareCount,
                announcements = finalAnn,
                trafficStats = trafficStats ?: if (sameUser && (Routes.canTrafficHub(me) || Routes.canAdminMembers(me) || me?.can("admin_dashboard") == true)) prev.trafficStats else null,
                onboardingLoaded = if (sameUser) prev.onboardingLoaded else false,
                onboardingHasPlace = if (sameUser) prev.onboardingHasPlace else false,
                onboardingHasRide = if (sameUser) prev.onboardingHasRide else false,
                onboardingHasEvent = if (sameUser) prev.onboardingHasEvent else false,
                onboardingHasVehicle = if (sameUser) prev.onboardingHasVehicle else false,
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

    /** Lazy check triggered from `HomeScreen` when onboarding checklist is visible so unit test HTTP order is unchanged. */
    fun loadOnboardingChecks() {
        val me = _state.value.me ?: container.auth.me ?: return
        if (me.status != "active" || me.onboardingDismissed) return
        if (onboardingJob?.isActive == true) return
        onboardingJob = viewModelScope.launch {
            val (hasPlace, hasEvent, hasRide, hasVehicle) = coroutineScope {
                val placeDef = async {
                    if (me.can("rides", "create")) {
                        runSuspendCatching { container.rides.savedPlaces().isNotEmpty() }.getOrDefault(false)
                    } else {
                        true
                    }
                }
                val eventDef = async {
                    if (me.can("events")) {
                        val cachedAny = _state.value.events.any { it.mySignup != null }
                        if (cachedAny) true else runSuspendCatching {
                            container.events.upcoming().any { it.mySignup != null }
                        }.getOrDefault(false)
                    } else {
                        true
                    }
                }
                val rideDef = async {
                    if (me.can("rides", "create")) {
                        val hasNext = _state.value.nextRide != null
                        if (hasNext) true else runSuspendCatching {
                            container.rides.upcoming().isNotEmpty() ||
                                container.rides.pastPage(before = null, limit = 1).items.isNotEmpty()
                        }.getOrDefault(false)
                    } else {
                        true
                    }
                }
                val vehicleDef = async {
                    if (me.can("drive")) {
                        runSuspendCatching { container.rides.driverProfile().vehicles.isNotEmpty() }.getOrDefault(false)
                    } else {
                        true
                    }
                }
                listOf(placeDef.await(), eventDef.await(), rideDef.await(), vehicleDef.await())
            }
            _state.value = _state.value.copy(
                onboardingLoaded = true,
                onboardingHasPlace = hasPlace,
                onboardingHasEvent = hasEvent,
                onboardingHasRide = hasRide,
                onboardingHasVehicle = hasVehicle,
            )
        }
    }

    fun dismissOnboarding() {
        viewModelScope.launch {
            val current = _state.value.me
            if (current != null) {
                _state.value = _state.value.copy(me = current.copy(onboardingDismissed = true))
            }
            runSuspendCatching {
                container.community.updateProfile(ProfileUpdateReq(onboarding_dismissed = true))
                container.auth.refresh()
            }
        }
    }
}

private data class Tuple5<A, B, C, D, E>(val a: A, val b: B, val c: C, val d: D, val e: E)
