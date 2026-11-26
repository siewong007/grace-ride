package org.pjstatecm.graceride.ui.screens.rides

import android.Manifest
import android.annotation.SuppressLint
import android.location.Location
import android.location.LocationManager
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.CheckCircle
import androidx.compose.material.icons.outlined.Flag
import androidx.compose.material.icons.outlined.MyLocation
import androidx.compose.material.icons.outlined.Navigation
import androidx.compose.material.icons.outlined.ShareLocation
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import androidx.core.location.LocationManagerCompat
import androidx.core.os.CancellationSignal
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.RideLocationDto
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.service.LiveTripLocationService
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.GraceRideLiveMap
import org.pjstatecm.graceride.ui.components.LiveMapPoint
import org.pjstatecm.graceride.ui.components.MapMarkerKind
import org.pjstatecm.graceride.ui.openInMaps
import org.pjstatecm.graceride.ui.rememberSmartNavigator
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular
import java.util.Locale
import java.util.concurrent.atomic.AtomicBoolean

@Composable
fun LiveTrackCard(
    container: AppContainer,
    ride: Ride,
    location: RideLocationDto?,
    sharingPassengerPin: Boolean,
    riderLocationAutoStopped: Boolean = false,
    onRefreshLocation: () -> Unit,
    onSharePassengerSpot: (Double, Double) -> Unit,
    onStopPassengerSpot: () -> Unit,
) {
    if (!LiveTripLocationService.canShareRideLocation(ride)) return

    val context = LocalContext.current
    val view = LocalView.current
    val driverSharing by LiveTripLocationService.isSharing.collectAsStateWithLifecycle()
    val nearDestTripId by LiveTripLocationService.nearDestinationTripId.collectAsStateWithLifecycle()
    val smartNavigate = rememberSmartNavigator(container)
    val c = LocalStatusColors.current
    val scope = rememberCoroutineScope()

    var activeSignal by remember { mutableStateOf<CancellationSignal?>(null) }
    var spotTimeoutJob by remember { mutableStateOf<Job?>(null) }
    var spotLocating by remember { mutableStateOf(false) }
    var spotDenied by remember { mutableStateOf(false) }
    var driverPermDenied by remember { mutableStateOf(false) }

    val isBoarded = !ride.viewer.isDriver && (ride.pickedUpAt != null || location?.eta?.phase == "dropoff")

    LaunchedEffect(isBoarded) {
        if (isBoarded) {
            spotTimeoutJob?.cancel()
            spotTimeoutJob = null
            activeSignal?.cancel()
            activeSignal = null
            spotLocating = false
        }
    }

    DisposableEffect(Unit) {
        onDispose {
            spotTimeoutJob?.cancel()
            activeSignal?.cancel()
        }
    }

    LaunchedEffect(ride.id, ride.status) {
        while (true) {
            onRefreshLocation()
            delay(12_000L)
        }
    }

    @SuppressLint("MissingPermission")
    fun sendPassengerSpotOnce() {
        if (!LiveTripLocationService.canSharePassengerWaitingSpot(ride)) return
        val lm = context.getSystemService(LocationManager::class.java)
        if (lm == null) {
            spotLocating = false
            spotDenied = true
            return
        }
        spotTimeoutJob?.cancel()
        activeSignal?.cancel()
        spotLocating = true
        spotDenied = false
        val settled = AtomicBoolean(false)

        val readLastKnown: () -> Location? = {
            try {
                val fused = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S &&
                    lm.hasProvider(LocationManager.FUSED_PROVIDER)
                ) {
                    lm.getLastKnownLocation(LocationManager.FUSED_PROVIDER)
                } else {
                    null
                }
                fused
                    ?: lm.getLastKnownLocation(LocationManager.GPS_PROVIDER)
                    ?: lm.getLastKnownLocation(LocationManager.NETWORK_PROVIDER)
            } catch (_: SecurityException) {
                null
            }
        }

        val applyLoc: (Location?) -> Unit = { loc ->
            if (settled.compareAndSet(false, true)) {
                spotTimeoutJob?.cancel()
                activeSignal = null
                spotLocating = false
                if (loc != null && LiveTripLocationService.canSharePassengerWaitingSpot(ride)) {
                    spotDenied = false
                    onSharePassengerSpot(loc.latitude, loc.longitude)
                } else {
                    spotDenied = true
                }
            }
        }

        val provider = when {
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.S &&
                lm.hasProvider(LocationManager.FUSED_PROVIDER) &&
                lm.isProviderEnabled(LocationManager.FUSED_PROVIDER) -> LocationManager.FUSED_PROVIDER
            lm.isProviderEnabled(LocationManager.NETWORK_PROVIDER) -> LocationManager.NETWORK_PROVIDER
            lm.isProviderEnabled(LocationManager.GPS_PROVIDER) -> LocationManager.GPS_PROVIDER
            else -> null
        }

        try {
            if (provider != null) {
                val signal = CancellationSignal()
                activeSignal = signal
                spotTimeoutJob = scope.launch {
                    delay(15_000L)
                    if (!settled.get()) {
                        signal.cancel()
                        applyLoc(readLastKnown())
                    }
                }
                LocationManagerCompat.getCurrentLocation(
                    lm,
                    provider,
                    signal,
                    ContextCompat.getMainExecutor(context),
                ) { loc ->
                    applyLoc(loc ?: readLastKnown())
                }
            } else {
                applyLoc(readLastKnown())
            }
        } catch (_: SecurityException) {
            settled.set(true)
            spotTimeoutJob?.cancel()
            activeSignal = null
            spotLocating = false
            spotDenied = true
        }
    }

    val permLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions(),
    ) { granted ->
        if (granted[Manifest.permission.ACCESS_FINE_LOCATION] == true ||
            granted[Manifest.permission.ACCESS_COARSE_LOCATION] == true
        ) {
            if (ride.viewer.isDriver && LiveTripLocationService.canShareRideLocation(ride)) {
                driverPermDenied = false
                LiveTripLocationService.start(
                    context,
                    rideId = ride.id,
                    tripId = ride.tripId ?: -1L,
                )
            } else if (LiveTripLocationService.canSharePassengerWaitingSpot(ride)) {
                spotDenied = false
                sendPassengerSpotOnce()
            }
        } else {
            if (ride.viewer.isDriver) {
                driverPermDenied = true
            } else {
                spotDenied = true
            }
        }
    }

    val mapDriverLabel = stringResource(R.string.map_driver_label)
    val mapYouLabel = stringResource(R.string.map_you_label)
    val mapPickupLabel = stringResource(R.string.ride_pickup)
    val etaTarget = location?.eta?.target?.takeIf { it.size >= 2 }

    val mapPoints = buildList {
        if (location?.lat != null && location.lng != null) {
            add(
                LiveMapPoint(
                    id = "driver",
                    lat = location.lat,
                    lng = location.lng,
                    label = if (ride.viewer.isDriver) mapDriverLabel else (ride.driver?.name ?: mapDriverLabel),
                    kind = MapMarkerKind.CAR,
                    stale = if (ride.viewer.isDriver) (!driverSharing && !location.sharing) else !location.sharing,
                ),
            )
        }
        if (ride.viewer.isDriver) {
            if (ride.pickedUpAt == null) {
                if (location?.passenger != null) {
                    add(
                        LiveMapPoint(
                            id = "pax",
                            lat = location.passenger.lat,
                            lng = location.passenger.lng,
                            label = ride.passenger.name,
                            kind = MapMarkerKind.PERSON,
                        ),
                    )
                } else if (etaTarget != null) {
                    add(
                        LiveMapPoint(
                            id = "pickup",
                            lat = etaTarget[0],
                            lng = etaTarget[1],
                            label = ride.passenger.name,
                            kind = MapMarkerKind.STOP,
                        ),
                    )
                }
            } else if (etaTarget != null) {
                add(
                    LiveMapPoint(
                        id = "dest",
                        lat = etaTarget[0],
                        lng = etaTarget[1],
                        label = ride.destination,
                        kind = MapMarkerKind.DEST,
                    ),
                )
            }
        } else {
            if (!isBoarded) {
                if (location?.passenger != null) {
                    add(
                        LiveMapPoint(
                            id = "you",
                            lat = location.passenger.lat,
                            lng = location.passenger.lng,
                            label = mapYouLabel,
                            kind = MapMarkerKind.PERSON,
                        ),
                    )
                } else if (etaTarget != null) {
                    add(
                        LiveMapPoint(
                            id = "pickup",
                            lat = etaTarget[0],
                            lng = etaTarget[1],
                            label = mapPickupLabel,
                            kind = MapMarkerKind.STOP,
                        ),
                    )
                }
            } else if (etaTarget != null) {
                add(
                    LiveMapPoint(
                        id = "dest",
                        lat = etaTarget[0],
                        lng = etaTarget[1],
                        label = ride.destination,
                        kind = MapMarkerKind.DEST,
                    ),
                )
            }
        }
    }

    OutlinedCard(
        modifier = Modifier.fillMaxWidth(),
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
    ) {
        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                Icon(
                    Icons.Outlined.MyLocation,
                    contentDescription = null,
                    tint = if ((ride.viewer.isDriver && driverSharing) || location?.sharing == true) c.success else MaterialTheme.colorScheme.primary,
                    modifier = Modifier.size(20.dp),
                )
                Text(stringResource(R.string.live_location_title), style = MaterialTheme.typography.titleSmall)
            }

            if (mapPoints.isNotEmpty()) {
                GraceRideLiveMap(
                    points = mapPoints,
                    route = location?.eta?.path.orEmpty(),
                    modifier = Modifier.fillMaxWidth().height(190.dp),
                )
            }

            if (ride.viewer.isDriver) {
                if (ride.tripId != null && nearDestTripId == ride.tripId && ride.pickedUpAt != null) {
                    CalloutBanner(
                        icon = Icons.Outlined.Flag,
                        text = stringResource(R.string.drive_dest_nearby_banner),
                        containerColor = c.successSoft,
                        contentColor = c.success,
                    )
                }
                Text(
                    stringResource(if (driverSharing) R.string.live_location_driver_active else R.string.live_location_driver_idle),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    Button(
                        onClick = {
                            UiHaptics.confirm(view)
                            if (driverSharing) {
                                LiveTripLocationService.stop(context)
                            } else if (LiveTripLocationService.hasLocationPermission(context)) {
                                driverPermDenied = false
                                LiveTripLocationService.start(
                                    context,
                                    rideId = ride.id,
                                    tripId = ride.tripId ?: -1L,
                                )
                            } else {
                                permLauncher.launch(
                                    arrayOf(
                                        Manifest.permission.ACCESS_FINE_LOCATION,
                                        Manifest.permission.ACCESS_COARSE_LOCATION,
                                    ),
                                )
                            }
                        },
                        colors = if (driverSharing) {
                            ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error)
                        } else {
                            graceRidePrimaryButtonColors()
                        },
                        modifier = Modifier.weight(1f).height(48.dp),
                    ) {
                        Text(stringResource(if (driverSharing) R.string.live_location_stop else R.string.live_location_start))
                    }
                    OutlinedButton(
                        onClick = {
                            val spot = if (ride.pickedUpAt == null) location?.passenger else null
                            val navQuery = if (ride.pickedUpAt == null) ride.pickupAddress.orEmpty() else ride.destination
                            smartNavigate(spot?.lat, spot?.lng, navQuery)
                        },
                        modifier = Modifier.height(48.dp),
                    ) {
                        Icon(Icons.Outlined.Navigation, contentDescription = null, modifier = Modifier.size(16.dp))
                        Spacer(Modifier.width(4.dp))
                        Text(stringResource(R.string.drive_route_navigate))
                    }
                }
                if (driverPermDenied) {
                    Text(
                        stringResource(R.string.live_location_perm_needed),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
            } else {
                if (isBoarded || riderLocationAutoStopped) {
                    Surface(
                        color = c.successSoft,
                        contentColor = c.success,
                        shape = CircleShape,
                    ) {
                        Row(
                            modifier = Modifier.fillMaxWidth().padding(horizontal = Spacing.sm, vertical = Spacing.xxs),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            Icon(Icons.Outlined.CheckCircle, contentDescription = null, modifier = Modifier.size(16.dp))
                            Text(
                                stringResource(R.string.live_onboard_auto_off),
                                style = MaterialTheme.typography.labelMedium,
                            )
                        }
                    }
                }

                if (location?.sharing == true) {
                    val eta = location.eta
                    val mins = eta?.durationS?.let { ((it + 30.0) / 60.0).toInt().coerceAtLeast(1) }
                    val headline = if (isBoarded) {
                        if (mins != null) {
                            stringResource(R.string.live_eta_arriving_dest, ride.destination, mins)
                        } else {
                            ride.destination
                        }
                    } else {
                        if (mins != null) {
                            stringResource(R.string.live_eta_away, mins)
                        } else {
                            stringResource(R.string.live_location_passenger_waiting)
                        }
                    }
                    val detailParts = buildList {
                        eta?.eta?.takeIf { it.isNotBlank() }?.let { add("ETA ${KlTime.time(it)}") }
                        eta?.distanceM?.let { m ->
                            add(if (m >= 1000) String.format(Locale.US, "%.1f km", m / 1000.0) else "${m.toInt()} m")
                        }
                        if (!isBoarded && eta != null) {
                            if (eta.stopsBefore == 0) {
                                add(stringResource(R.string.live_next_stop))
                            } else {
                                add(stringResource(R.string.live_location_stops_before, eta.stopsBefore))
                            }
                        }
                    }.joinToString(" · ")
                    Text(
                        headline,
                        style = MaterialTheme.typography.titleMedium.tabular(),
                        color = c.success,
                    )
                    if (detailParts.isNotBlank()) {
                        Text(
                            detailParts,
                            style = MaterialTheme.typography.bodySmall.tabular(),
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    if (!isBoarded && location.lat != null && location.lng != null) {
                        OutlinedButton(
                            onClick = { openInMaps(context, "${location.lat},${location.lng}") },
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Icon(Icons.Outlined.Navigation, contentDescription = null, modifier = Modifier.size(16.dp))
                            Spacer(Modifier.width(Spacing.xs))
                            Text(stringResource(R.string.common_open_maps))
                        }
                    }
                } else {
                    Text(
                        stringResource(R.string.live_location_passenger_idle),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }

                if (LiveTripLocationService.canSharePassengerWaitingSpot(ride) && !isBoarded) {
                    val activePin = sharingPassengerPin || location?.passenger != null
                    OutlinedButton(
                        onClick = {
                            UiHaptics.tick(view)
                            if (activePin) {
                                spotDenied = false
                                onStopPassengerSpot()
                            } else if (LiveTripLocationService.hasLocationPermission(context)) {
                                sendPassengerSpotOnce()
                            } else {
                                permLauncher.launch(
                                    arrayOf(
                                        Manifest.permission.ACCESS_FINE_LOCATION,
                                        Manifest.permission.ACCESS_COARSE_LOCATION,
                                    ),
                                )
                            }
                        },
                        enabled = !spotLocating,
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Icon(Icons.Outlined.ShareLocation, contentDescription = null, modifier = Modifier.size(18.dp))
                        Spacer(Modifier.width(Spacing.xs))
                        Text(
                            stringResource(
                                when {
                                    spotLocating -> R.string.live_location_spot_locating
                                    activePin -> R.string.live_location_stop_spot
                                    else -> R.string.live_location_share_spot
                                },
                            ),
                        )
                    }
                    if (activePin) {
                        Text(
                            stringResource(R.string.live_location_spot_shared),
                            style = MaterialTheme.typography.bodySmall,
                            color = c.info,
                        )
                    } else if (spotDenied) {
                        Text(
                            stringResource(R.string.live_location_spot_unavailable),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                        )
                    }
                }
            }
        }
    }
}
