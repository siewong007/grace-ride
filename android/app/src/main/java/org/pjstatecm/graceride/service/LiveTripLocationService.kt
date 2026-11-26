package org.pjstatecm.graceride.service

import android.Manifest
import android.annotation.SuppressLint
import android.app.Notification
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.content.pm.ServiceInfo
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.os.Build
import android.os.Bundle
import android.os.IBinder
import android.os.Looper
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.ProcessLifecycleOwner
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import org.pjstatecm.graceride.GraceRideApp
import org.pjstatecm.graceride.MainActivity
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.TripRouteDto
import org.pjstatecm.graceride.data.api.runSuspendCatching
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.notifications.NotificationHelper
import org.pjstatecm.graceride.notifications.RealtimeEvent
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong
import java.util.concurrent.atomic.AtomicReference
import kotlin.math.asin
import kotlin.math.cos
import kotlin.math.pow
import kotlin.math.sin
import kotlin.math.sqrt

/**
 * Native Android foreground location service that streams driver GPS updates to
 * `POST /api/driver/location` strictly while an `in_progress` journey is active.
 */
class LiveTripLocationService : Service() {

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val stopping = AtomicBoolean(false)
    private val pushMutex = Mutex()

    private var locationManager: LocationManager? = null
    private var activeRideId: Long = -1L
    private var activeTripId: Long = -1L
    private var lastSentMs: Long = 0L
    private var lastConfirmedLiveMs: Long = 0L
    private var consecutiveNetworkErrors: Int = 0
    @Volatile
    private var latestRoute: TripRouteDto? = null
    @Volatile
    private var lastNotifBody: String? = null
    private val autoArrivalHitCounts = ConcurrentHashMap<Long, Int>()
    private val autoArrivedRideIds = ConcurrentHashMap.newKeySet<Long>()
    private val sessionGen = AtomicLong(0L)

    private var verifyAndWatchJob: Job? = null
    private var authObserverJob: Job? = null
    private var sseObserverJob: Job? = null
    private var pushJob: Job? = null

    private val listener = object : LocationListener {
        override fun onLocationChanged(loc: Location) {
            if (stopping.get()) return
            val now = System.currentTimeMillis()
            if (now - lastSentMs < MIN_PUSH_INTERVAL_MS) return
            lastSentMs = now
            pushLocation(loc)
        }

        override fun onProviderDisabled(provider: String) {
            if (!stopping.get()) {
                scope.launch(Dispatchers.Main) {
                    if (!stopping.get()) startLocationUpdates()
                }
            }
        }

        override fun onProviderEnabled(provider: String) {}

        @Suppress("OVERRIDE_DEPRECATION")
        @Deprecated("Deprecated in Java")
        override fun onStatusChanged(provider: String?, status: Int, extras: Bundle?) {}
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_STOP -> {
                if (fgsState.get() == ServiceFgsState.STARTING_FG) {
                    startForegroundWithNotif()
                }
                stopSharingAndSelf(notifyServer = true)
                return START_NOT_STICKY
            }
            ACTION_START -> {
                val rideId = intent.getLongExtra(EXTRA_RIDE_ID, -1L)
                val tripId = intent.getLongExtra(EXTRA_TRIP_ID, -1L)
                activeRideId = rideId
                activeTripId = tripId
                val myGen = sessionGen.incrementAndGet()
                verifyAndWatchJob?.cancel()
                authObserverJob?.cancel()
                sseObserverJob?.cancel()
                pushJob?.cancel()

                if (!hasLocationPermission(this) || !hasActiveJourneyTarget(rideId, tripId)) {
                    if (fgsState.get() == ServiceFgsState.STARTING_FG) {
                        startForegroundWithNotif()
                    }
                    stopping.set(true)
                    _isSharing.value = false
                    fgsState.set(ServiceFgsState.IDLE)
                    pendingStop.set(false)
                    runCatching { stopForeground(STOP_FOREGROUND_REMOVE) }
                    stopSelf()
                    return START_NOT_STICKY
                }
                stopping.set(false)
                if (!startForegroundWithNotif()) {
                    stopSharingAndSelf(notifyServer = false)
                    return START_NOT_STICKY
                }
                fgsState.set(ServiceFgsState.RUNNING)
                if (pendingStop.getAndSet(false)) {
                    stopSharingAndSelf(notifyServer = false)
                    return START_NOT_STICKY
                }
                verifyJourneyThenStream(myGen)
                return START_NOT_STICKY
            }
            else -> {
                if (fgsState.get() == ServiceFgsState.STARTING_FG) {
                    startForegroundWithNotif()
                }
                fgsState.set(ServiceFgsState.IDLE)
                pendingStop.set(false)
                runCatching { stopForeground(STOP_FOREGROUND_REMOVE) }
                stopSelf()
                return START_NOT_STICKY
            }
        }
    }

    private fun buildForegroundNotification(route: TripRouteDto? = null): Pair<Notification, String> {
        val app = applicationContext as? GraceRideApp
        val lang = app?.container?.lang
        val hideLockDetails = app?.container?.secureStore?.getString("notif_hide_lock_details") == "1"
        val ctx = NotificationHelper.localizedContext(this, lang)
        NotificationHelper.ensureChannels(this, lang)
        val body = when {
            !hideLockDetails && route != null && route.live && route.stops.isNotEmpty() ->
                ctx.getString(R.string.live_location_notif_progress, route.stops.size, route.onboard)
            !hideLockDetails && route != null && route.live && route.onboard > 0 ->
                ctx.getString(R.string.live_location_notif_en_route_dest, route.onboard)
            else -> ctx.getString(R.string.live_location_notif_body)
        }
        val tapIntent = Intent(this, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
            putExtra(
                NotificationHelper.EXTRA_NOTIF_LINK,
                if (activeRideId > 0L) "/rides/$activeRideId" else "/drive",
            )
            putExtra(
                NotificationHelper.EXTRA_NOTIF_TOKEN,
                NotificationHelper.internalToken(this@LiveTripLocationService),
            )
        }
        val tapPending = PendingIntent.getActivity(
            this,
            101,
            tapIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val stopPending = PendingIntent.getService(
            this,
            102,
            Intent(this, LiveTripLocationService::class.java).apply { action = ACTION_STOP },
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val notif = NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_launcher_monochrome)
            .setContentTitle(ctx.getString(R.string.live_location_notif_title))
            .setContentText(body)
            .setContentIntent(tapPending)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setCategory(NotificationCompat.CATEGORY_NAVIGATION)
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
            .setForegroundServiceBehavior(NotificationCompat.FOREGROUND_SERVICE_IMMEDIATE)
            .addAction(0, ctx.getString(R.string.live_location_stop), stopPending)
            .build()
        return notif to body
    }

    private fun updateForegroundNotification(route: TripRouteDto?) {
        if (stopping.get() || fgsState.get() != ServiceFgsState.RUNNING) return
        if (!NotificationHelper.hasPostPermission(this)) return
        val (notif, body) = buildForegroundNotification(route)
        if (body == lastNotifBody) return
        lastNotifBody = body
        runCatching {
            getSystemService(NotificationManager::class.java)?.notify(NOTIF_ID, notif)
        }
    }

    private fun startForegroundWithNotif(): Boolean {
        val (notif, body) = buildForegroundNotification(latestRoute)
        lastNotifBody = body
        return try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                startForeground(NOTIF_ID, notif, ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION)
            } else {
                startForeground(NOTIF_ID, notif)
            }
            true
        } catch (_: Exception) {
            false
        }
    }

    private fun verifyJourneyThenStream(myGen: Long) {
        val app = applicationContext as? GraceRideApp ?: run {
            stopSharingAndSelf(notifyServer = false)
            return
        }
        verifyAndWatchJob = scope.launch {
            val route = runSuspendCatching { app.container.rides.driverRoute().route }.getOrNull()
            if (sessionGen.get() != myGen || stopping.get()) return@launch
            latestRoute = route
            if (!canShareDriverLocation(app.container.auth.state.value, route)) {
                stopSharingAndSelf(notifyServer = false)
                return@launch
            }
            lastConfirmedLiveMs = System.currentTimeMillis()
            consecutiveNetworkErrors = 0

            val started = withContext(Dispatchers.Main) {
                if (sessionGen.get() != myGen || stopping.get()) false else startLocationUpdates()
            }
            if (!started || sessionGen.get() != myGen) return@launch
            _isSharing.value = true
            updateForegroundNotification(route)

            authObserverJob = scope.launch {
                app.container.auth.state.collect { st ->
                    if (sessionGen.get() == myGen && (st !is AuthState.SignedIn || !st.me.termsAccepted)) {
                        stopSharingAndSelf(notifyServer = false)
                    }
                }
            }

            sseObserverJob = scope.launch {
                app.container.realtimeStream.events.collect { ev ->
                    if (sessionGen.get() == myGen && ev != RealtimeEvent.Resume) {
                        reverifyActiveJourney(app, myGen)
                    }
                }
            }

            while (isActive && sessionGen.get() == myGen && !stopping.get()) {
                delay(WATCHDOG_INTERVAL_MS)
                if (sessionGen.get() != myGen || stopping.get()) break
                if (!hasLocationPermission(this@LiveTripLocationService)) {
                    stopSharingAndSelf(notifyServer = true)
                    break
                }
                if (System.currentTimeMillis() - lastConfirmedLiveMs >= WATCHDOG_INTERVAL_MS) {
                    reverifyActiveJourney(app, myGen)
                }
            }
        }
    }

    private suspend fun reverifyActiveJourney(app: GraceRideApp, myGen: Long) {
        if (stopping.get() || sessionGen.get() != myGen) return
        var routeLive: Boolean? = null
        var authRejected = false
        try {
            val route = app.container.rides.driverRoute().route
            if (sessionGen.get() != myGen) return
            latestRoute = route
            routeLive = route?.live == true
            if (routeLive) {
                lastConfirmedLiveMs = System.currentTimeMillis()
                consecutiveNetworkErrors = 0
                updateForegroundNotification(route)
            } else {
                _nearDestinationTripId.value = null
            }
        } catch (ce: CancellationException) {
            throw ce
        } catch (e: ApiException) {
            if (e.status in setOf(401, 403, 423)) {
                authRejected = true
            } else {
                consecutiveNetworkErrors++
            }
        } catch (_: Exception) {
            consecutiveNetworkErrors++
        }

        if (sessionGen.get() != myGen) return
        when (
            evaluateJourneyCheck(
                authState = app.container.auth.state.value,
                routeLive = routeLive,
                authRejected = authRejected,
                consecutiveNetworkErrors = consecutiveNetworkErrors,
            )
        ) {
            WatchdogAction.KEEP_RUNNING -> Unit
            WatchdogAction.STOP_AUTH_INVALID,
            WatchdogAction.STOP_JOURNEY_ENDED -> stopSharingAndSelf(notifyServer = false)
        }
    }

    @SuppressLint("MissingPermission")
    private fun startLocationUpdates(): Boolean {
        val lm = getSystemService(LocationManager::class.java) ?: run {
            stopSharingAndSelf(notifyServer = false)
            return false
        }
        locationManager = lm
        runCatching { lm.removeUpdates(listener) }
        val provider = when {
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.S &&
                lm.hasProvider(LocationManager.FUSED_PROVIDER) &&
                lm.isProviderEnabled(LocationManager.FUSED_PROVIDER) -> LocationManager.FUSED_PROVIDER
            lm.isProviderEnabled(LocationManager.GPS_PROVIDER) -> LocationManager.GPS_PROVIDER
            lm.isProviderEnabled(LocationManager.NETWORK_PROVIDER) -> LocationManager.NETWORK_PROVIDER
            else -> null
        }
        if (provider == null) {
            stopSharingAndSelf(notifyServer = true)
            return false
        }
        return try {
            lm.getLastKnownLocation(provider)?.let { loc ->
                val ageMs = System.currentTimeMillis() - loc.time
                if (ageMs in 0..MAX_LAST_KNOWN_AGE_MS) {
                    pushLocation(loc)
                }
            }
            lm.requestLocationUpdates(provider, 8_000L, 10f, listener, Looper.getMainLooper())
            true
        } catch (_: SecurityException) {
            stopSharingAndSelf(notifyServer = false)
            false
        }
    }

    private fun pushLocation(loc: Location) {
        if (stopping.get()) return
        val app = applicationContext as? GraceRideApp ?: return
        val myGen = sessionGen.get()
        pushJob?.cancel()
        pushJob = scope.launch {
            pushMutex.withLock {
                if (stopping.get() || sessionGen.get() != myGen) return@withLock
                try {
                    val heading = if (loc.hasBearing()) loc.bearing.toDouble() else null
                    val res = app.container.rides.postDriverLocation(loc.latitude, loc.longitude, heading)
                    if (sessionGen.get() != myGen) return@withLock
                    if (!res.sharing) {
                        stopSharingAndSelf(notifyServer = false)
                    } else {
                        lastConfirmedLiveMs = System.currentTimeMillis()
                        consecutiveNetworkErrors = 0
                        val route = latestRoute ?: runSuspendCatching { app.container.rides.driverRoute().route }.getOrNull()
                            ?.also {
                                latestRoute = it
                                updateForegroundNotification(it)
                            }
                        _nearDestinationTripId.value = if (
                            isNearDestination(
                                lat = loc.latitude,
                                lng = loc.longitude,
                                accuracyM = if (loc.hasAccuracy()) loc.accuracy else null,
                                hasAccuracy = loc.hasAccuracy(),
                                route = route,
                            )
                        ) {
                            route?.tripId
                        } else {
                            null
                        }
                        val dueIds = findEligibleAutoArrivalStopIds(
                            lat = loc.latitude,
                            lng = loc.longitude,
                            accuracyM = if (loc.hasAccuracy()) loc.accuracy else null,
                            hasAccuracy = loc.hasAccuracy(),
                            autoEnabled = app.container.autoArriveGate.value,
                            route = route,
                            alreadyTriggered = autoArrivedRideIds,
                            hitCounts = autoArrivalHitCounts,
                        )
                        for (stopRideId in dueIds) {
                            autoArrivedRideIds.add(stopRideId)
                            val detail = runSuspendCatching { app.container.rides.detail(stopRideId) }.getOrNull()
                            if (detail != null &&
                                detail.status == "in_progress" &&
                                detail.pickedUpAt == null &&
                                detail.arrivedAtGateAt.isNullOrBlank()
                            ) {
                                runSuspendCatching { app.container.rides.arrivedGate(stopRideId) }
                            }
                        }
                        if (dueIds.isNotEmpty()) {
                            latestRoute = runSuspendCatching { app.container.rides.driverRoute().route }.getOrNull() ?: latestRoute
                            updateForegroundNotification(latestRoute)
                        }
                    }
                } catch (ce: CancellationException) {
                    throw ce
                } catch (e: ApiException) {
                    if (sessionGen.get() == myGen && e.status in setOf(401, 403, 423)) {
                        stopSharingAndSelf(notifyServer = false)
                    }
                } catch (_: Exception) {
                    // Keep trying on subsequent location updates.
                }
            }
        }
    }

    private fun stopSharingAndSelf(notifyServer: Boolean = true) {
        val first = stopping.compareAndSet(false, true)
        sessionGen.incrementAndGet()
        verifyAndWatchJob?.cancel()
        authObserverJob?.cancel()
        sseObserverJob?.cancel()
        pushJob?.cancel()
        locationManager?.let { lm -> runCatching { lm.removeUpdates(listener) } }
        val app = applicationContext as? GraceRideApp
        if (first && notifyServer && app != null) {
            app.container.appScope.launch {
                pushMutex.withLock {
                    runSuspendCatching { app.container.rides.stopDriverLocation() }
                }
            }
        }
        latestRoute = null
        lastNotifBody = null
        autoArrivalHitCounts.clear()
        autoArrivedRideIds.clear()
        _nearDestinationTripId.value = null
        _isSharing.value = false
        fgsState.set(ServiceFgsState.IDLE)
        pendingStop.set(false)
        runCatching { stopForeground(STOP_FOREGROUND_REMOVE) }
        stopSelf()
    }

    override fun onDestroy() {
        stopping.set(true)
        sessionGen.incrementAndGet()
        verifyAndWatchJob?.cancel()
        authObserverJob?.cancel()
        sseObserverJob?.cancel()
        pushJob?.cancel()
        locationManager?.let { lm -> runCatching { lm.removeUpdates(listener) } }
        latestRoute = null
        lastNotifBody = null
        lastNotifBody = null
        autoArrivalHitCounts.clear()
        autoArrivedRideIds.clear()
        _nearDestinationTripId.value = null
        _isSharing.value = false
        fgsState.set(ServiceFgsState.IDLE)
        pendingStop.set(false)
        scope.cancel()
        super.onDestroy()
    }

    companion object {
        private const val CHANNEL_ID = NotificationHelper.CHANNEL_LIVE_LOCATION
        private const val NOTIF_ID = 4910
        private const val MIN_PUSH_INTERVAL_MS = 6_000L
        private const val WATCHDOG_INTERVAL_MS = 25_000L
        private const val MAX_LAST_KNOWN_AGE_MS = 120_000L
        internal const val MAX_CONSECUTIVE_ERRORS = 10
        internal const val AUTO_ARRIVAL_RADIUS_M = 80.0
        internal const val DEST_NEARBY_RADIUS_M = 100.0
        internal const val AUTO_ARRIVAL_MAX_ACCURACY_M = 50f

        const val ACTION_START = "org.pjstatecm.graceride.action.START_LIVE_LOCATION"
        const val ACTION_STOP = "org.pjstatecm.graceride.action.STOP_LIVE_LOCATION"
        const val EXTRA_RIDE_ID = "ride_id"
        const val EXTRA_TRIP_ID = "trip_id"

        private val _isSharing = MutableStateFlow(false)
        val isSharing: StateFlow<Boolean> = _isSharing.asStateFlow()

        private val _nearDestinationTripId = MutableStateFlow<Long?>(null)
        val nearDestinationTripId: StateFlow<Long?> = _nearDestinationTripId.asStateFlow()

        private enum class ServiceFgsState { IDLE, STARTING_FG, RUNNING }
        private val fgsState = AtomicReference(ServiceFgsState.IDLE)
        private val pendingStop = AtomicBoolean(false)

        internal enum class WatchdogAction {
            KEEP_RUNNING,
            STOP_JOURNEY_ENDED,
            STOP_AUTH_INVALID,
        }

        internal fun haversineMeters(lat1: Double, lng1: Double, lat2: Double, lng2: Double): Double {
            val r = 6_371_000.0
            val dLat = Math.toRadians(lat2 - lat1)
            val dLng = Math.toRadians(lng2 - lng1)
            val a = sin(dLat / 2.0).pow(2) +
                cos(Math.toRadians(lat1)) * cos(Math.toRadians(lat2)) * sin(dLng / 2.0).pow(2)
            return 2.0 * r * asin(sqrt(a.coerceIn(0.0, 1.0)))
        }

        internal fun findEligibleAutoArrivalStopIds(
            lat: Double,
            lng: Double,
            accuracyM: Float?,
            hasAccuracy: Boolean,
            autoEnabled: Boolean,
            route: TripRouteDto?,
            alreadyTriggered: Set<Long>,
            hitCounts: MutableMap<Long, Int>,
        ): List<Long> {
            if (!autoEnabled || route == null || !route.live) return emptyList()
            if (hasAccuracy && accuracyM != null && accuracyM > AUTO_ARRIVAL_MAX_ACCURACY_M) return emptyList()

            val ready = mutableListOf<Long>()
            for (stop in route.stops) {
                if (!stop.started || stop.rideId in alreadyTriggered) continue
                val sLat = stop.lat
                val sLng = stop.lng
                val exactCoords = stop.location == "live" || stop.location == "address"
                if (sLat == null || sLng == null || !exactCoords) {
                    hitCounts.remove(stop.rideId)
                    continue
                }
                if (haversineMeters(lat, lng, sLat, sLng) <= AUTO_ARRIVAL_RADIUS_M) {
                    val count = (hitCounts[stop.rideId] ?: 0) + 1
                    hitCounts[stop.rideId] = count
                    if (count >= 2) ready.add(stop.rideId)
                } else {
                    hitCounts.remove(stop.rideId)
                }
            }
            return ready
        }

        internal fun isNearDestination(
            lat: Double,
            lng: Double,
            accuracyM: Float?,
            hasAccuracy: Boolean,
            route: TripRouteDto?,
        ): Boolean {
            if (route == null || !route.live || route.stops.isNotEmpty() || route.onboard <= 0) return false
            if (hasAccuracy && accuracyM != null && accuracyM > AUTO_ARRIVAL_MAX_ACCURACY_M) return false
            val dest = route.destination
            val dLat = dest.lat ?: return false
            val dLng = dest.lng ?: return false
            if (dest.approx) return false
            return haversineMeters(lat, lng, dLat, dLng) <= DEST_NEARBY_RADIUS_M
        }

        internal fun hasActiveJourneyTarget(rideId: Long, tripId: Long): Boolean =
            rideId > 0L || tripId > 0L

        internal fun isJourneyLive(route: TripRouteDto?): Boolean = route?.live == true

        internal fun canShareDriverLocation(authState: AuthState, route: TripRouteDto?): Boolean =
            authState is AuthState.SignedIn && authState.me.termsAccepted && route?.live == true

        internal fun canShareRideLocation(ride: Ride?): Boolean =
            ride != null && ride.status == "in_progress"

        internal fun canSharePassengerWaitingSpot(ride: Ride?): Boolean =
            ride != null &&
                ride.status == "in_progress" &&
                ride.viewer.isPassenger &&
                ride.pickedUpAt == null

        internal fun evaluateJourneyCheck(
            authState: AuthState,
            routeLive: Boolean?,
            authRejected: Boolean,
            consecutiveNetworkErrors: Int,
            maxConsecutiveNetworkErrors: Int = MAX_CONSECUTIVE_ERRORS,
        ): WatchdogAction {
            if (authState !is AuthState.SignedIn || !authState.me.termsAccepted || authRejected) {
                return WatchdogAction.STOP_AUTH_INVALID
            }
            if (routeLive == false || (routeLive == null && consecutiveNetworkErrors >= maxConsecutiveNetworkErrors)) {
                return WatchdogAction.STOP_JOURNEY_ENDED
            }
            return WatchdogAction.KEEP_RUNNING
        }

        fun hasLocationPermission(context: Context): Boolean =
            ContextCompat.checkSelfPermission(context, Manifest.permission.ACCESS_FINE_LOCATION) ==
                PackageManager.PERMISSION_GRANTED ||
                ContextCompat.checkSelfPermission(context, Manifest.permission.ACCESS_COARSE_LOCATION) ==
                PackageManager.PERMISSION_GRANTED

        fun start(context: Context, rideId: Long = -1L, tripId: Long = -1L): Boolean {
            if (!hasLocationPermission(context) || !hasActiveJourneyTarget(rideId, tripId)) {
                return false
            }
            val eligibleFg = fgsState.get() == ServiceFgsState.RUNNING ||
                ProcessLifecycleOwner.get().lifecycle.currentState.isAtLeast(Lifecycle.State.STARTED)
            if (!eligibleFg) {
                return false
            }
            val intent = Intent(context, LiveTripLocationService::class.java).apply {
                action = ACTION_START
                putExtra(EXTRA_RIDE_ID, rideId)
                putExtra(EXTRA_TRIP_ID, tripId)
            }
            pendingStop.set(false)
            fgsState.compareAndSet(ServiceFgsState.IDLE, ServiceFgsState.STARTING_FG)
            return try {
                ContextCompat.startForegroundService(context, intent)
                true
            } catch (_: Exception) {
                fgsState.compareAndSet(ServiceFgsState.STARTING_FG, ServiceFgsState.IDLE)
                false
            }
        }

        fun stop(context: Context, notifyServer: Boolean = true) {
            _isSharing.value = false
            val app = context.applicationContext as? GraceRideApp
            if (notifyServer && app != null) {
                app.container.appScope.launch { runSuspendCatching { app.container.rides.stopDriverLocation() } }
            }
            when (fgsState.get()) {
                ServiceFgsState.IDLE -> Unit
                ServiceFgsState.STARTING_FG -> pendingStop.set(true)
                ServiceFgsState.RUNNING -> {
                    runCatching { context.stopService(Intent(context, LiveTripLocationService::class.java)) }
                }
            }
        }
    }
}
