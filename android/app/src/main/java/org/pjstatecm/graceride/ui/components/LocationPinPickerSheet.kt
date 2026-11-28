package org.pjstatecm.graceride.ui.components

import android.Manifest
import android.annotation.SuppressLint
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationManager
import android.os.Build
import android.os.CancellationSignal
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material.icons.outlined.MyLocation
import androidx.compose.material.icons.outlined.Place
import androidx.compose.material.icons.outlined.Remove
import androidx.compose.material.icons.outlined.RestartAlt
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableDoubleStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.ColorMatrix
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import androidx.core.location.LocationManagerCompat
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular
import java.util.Locale
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.math.floor
import kotlin.math.roundToInt

private val PICKER_DARK_FILTER = ColorFilter.colorMatrix(
    ColorMatrix(
        floatArrayOf(
            0.78f, 0f, 0f, 0f, -18f,
            0f, 0.80f, 0f, 0f, -16f,
            0f, 0f, 0.86f, 0f, -8f,
            0f, 0f, 0f, 1f, 0f,
        ),
    ),
)

private fun roundCoord6(value: Double): Double = (value * 1_000_000.0).roundToInt() / 1_000_000.0

/**
 * Interactive OpenStreetMap pin picker matching Grab/Uber fixed center teardrop UX.
 * Supports drag-to-pan, tap-to-center, zoom +/- controls, My GPS centering, reset, and confirm.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LocationPinPickerSheet(
    initialLat: Double?,
    initialLng: Double?,
    onDismiss: () -> Unit,
    onConfirm: (lat: Double, lng: Double) -> Unit,
) {
    val context = LocalContext.current
    val view = LocalView.current
    val scope = rememberCoroutineScope()
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val startLat = initialLat ?: DEFAULT_LAT
    val startLng = initialLng ?: DEFAULT_LNG
    var centerLat by remember(initialLat, initialLng) { mutableDoubleStateOf(startLat) }
    var centerLng by remember(initialLat, initialLng) { mutableDoubleStateOf(startLng) }
    var zoom by remember(initialLat, initialLng) {
        mutableIntStateOf(if (initialLat != null && initialLng != null) 16 else 14)
    }
    var hasAdjustedPin by remember(initialLat, initialLng) { mutableStateOf(initialLat != null && initialLng != null) }
    var isDragging by remember { mutableStateOf(false) }
    var canvasSize by remember { mutableStateOf(IntSize.Zero) }
    var tileRevision by remember { mutableIntStateOf(0) }
    var locatingGps by remember { mutableStateOf(false) }
    var gpsError by remember { mutableStateOf(false) }
    var activeSignal by remember { mutableStateOf<CancellationSignal?>(null) }
    var gpsTimeoutJob by remember { mutableStateOf<Job?>(null) }

    DisposableEffect(Unit) {
        onDispose {
            gpsTimeoutJob?.cancel()
            activeSignal?.cancel()
        }
    }

    @SuppressLint("MissingPermission")
    fun centerOnGps() {
        val lm = context.getSystemService(LocationManager::class.java)
        if (lm == null) {
            locatingGps = false
            gpsError = true
            return
        }
        gpsTimeoutJob?.cancel()
        activeSignal?.cancel()
        locatingGps = true
        gpsError = false
        val settled = AtomicBoolean(false)
        try {
            val provider = when {
                Build.VERSION.SDK_INT >= Build.VERSION_CODES.S &&
                    lm.hasProvider(LocationManager.FUSED_PROVIDER) &&
                    lm.isProviderEnabled(LocationManager.FUSED_PROVIDER) -> LocationManager.FUSED_PROVIDER
                lm.isProviderEnabled(LocationManager.GPS_PROVIDER) -> LocationManager.GPS_PROVIDER
                lm.isProviderEnabled(LocationManager.NETWORK_PROVIDER) -> LocationManager.NETWORK_PROVIDER
                else -> null
            }
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
                    gpsTimeoutJob?.cancel()
                    activeSignal = null
                    locatingGps = false
                    if (loc != null) {
                        centerLat = loc.latitude.coerceIn(-85.0, 85.0)
                        centerLng = loc.longitude.coerceIn(-180.0, 180.0)
                        if (zoom < 16) zoom = 16
                        hasAdjustedPin = true
                        gpsError = false
                    } else {
                        gpsError = true
                    }
                }
            }
            if (provider != null) {
                val signal = CancellationSignal()
                activeSignal = signal
                gpsTimeoutJob = scope.launch {
                    delay(12_000)
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
            locatingGps = false
            gpsError = true
        }
    }

    val permLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions(),
    ) { granted ->
        if (granted[Manifest.permission.ACCESS_FINE_LOCATION] == true ||
            granted[Manifest.permission.ACCESS_COARSE_LOCATION] == true
        ) {
            centerOnGps()
        } else {
            locatingGps = false
            gpsError = true
        }
    }

    fun requestGpsCenter() {
        UiHaptics.tick(view)
        val fine = ContextCompat.checkSelfPermission(context, Manifest.permission.ACCESS_FINE_LOCATION) == PackageManager.PERMISSION_GRANTED
        val coarse = ContextCompat.checkSelfPermission(context, Manifest.permission.ACCESS_COARSE_LOCATION) == PackageManager.PERMISSION_GRANTED
        if (fine || coarse) {
            centerOnGps()
        } else {
            permLauncher.launch(
                arrayOf(
                    Manifest.permission.ACCESS_FINE_LOCATION,
                    Manifest.permission.ACCESS_COARSE_LOCATION,
                ),
            )
        }
    }

    val darkTiles = MaterialTheme.colorScheme.surface.luminance() < 0.5f
    val primary = MaterialTheme.colorScheme.primary
    val bgColor = if (darkTiles) Color(0xFF141820) else Color(0xFFF2EFE9)

    LaunchedEffect(centerLat, centerLng, zoom, canvasSize) {
        if (canvasSize.width <= 0 || canvasSize.height <= 0) return@LaunchedEffect
        val maxTile = 1 shl zoom
        val halfW = canvasSize.width / 2.0
        val halfH = canvasSize.height / 2.0
        val centerWorldX = lngToWorldX(centerLng, zoom)
        val centerWorldY = latToWorldY(centerLat, zoom)
        val xStart = floor((centerWorldX - halfW) / TILE_PX).toInt()
        val xEnd = floor((centerWorldX + halfW) / TILE_PX).toInt()
        val yStart = floor((centerWorldY - halfH) / TILE_PX).toInt()
        val yEnd = floor((centerWorldY + halfH) / TILE_PX).toInt()
        for (ty in yStart..yEnd) {
            if (ty < 0 || ty >= maxTile) continue
            for (tx in xStart..xEnd) {
                val wrappedX = ((tx % maxTile) + maxTile) % maxTile
                if (OsmTileLoader.getCached(zoom, wrappedX, ty) == null) {
                    launch {
                        if (OsmTileLoader.load(context, zoom, wrappedX, ty) != null) {
                            tileRevision++
                        }
                    }
                }
            }
        }
    }

    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
        BoxWithConstraints(Modifier.fillMaxWidth()) {
            val mapHeight = if (maxHeight.value.isFinite()) {
                (maxHeight * 0.44f).coerceIn(190.dp, 330.dp)
            } else {
                280.dp
            }
            Column(
                Modifier
                    .fillMaxWidth()
                    .navigationBarsPadding()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = Spacing.sm)
                    .padding(bottom = Spacing.md),
                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
            Text(
                stringResource(R.string.pin_picker_title),
                style = MaterialTheme.typography.titleMedium,
            )
            Text(
                stringResource(R.string.pin_picker_hint),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            Box(
                Modifier
                    .fillMaxWidth()
                    .height(mapHeight)
                    .clip(RoundedCornerShape(16.dp))
                    .background(bgColor)
                    .border(BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant), RoundedCornerShape(16.dp))
                    .onSizeChanged { canvasSize = it }
                    .pointerInput(zoom) {
                        detectDragGestures(
                            onDragStart = { isDragging = true },
                            onDragEnd = { isDragging = false },
                            onDragCancel = { isDragging = false },
                        ) { change, dragAmount ->
                            change.consume()
                            val wx = lngToWorldX(centerLng, zoom) - dragAmount.x
                            val wy = latToWorldY(centerLat, zoom) - dragAmount.y
                            centerLng = worldXToLng(wx, zoom).coerceIn(-180.0, 180.0)
                            centerLat = worldYToLat(wy, zoom).coerceIn(-85.0, 85.0)
                            hasAdjustedPin = true
                            gpsError = false
                        }
                    }
                    .pointerInput(zoom, canvasSize) {
                        detectTapGestures { offset ->
                            if (canvasSize.width <= 0 || canvasSize.height <= 0) return@detectTapGestures
                            val halfW = canvasSize.width / 2.0
                            val halfH = canvasSize.height / 2.0
                            val wx = lngToWorldX(centerLng, zoom) + (offset.x - halfW)
                            val wy = latToWorldY(centerLat, zoom) + (offset.y - halfH)
                            centerLat = worldYToLat(wy, zoom).coerceIn(-85.0, 85.0)
                            centerLng = worldXToLng(wx, zoom).coerceIn(-180.0, 180.0)
                            hasAdjustedPin = true
                            gpsError = false
                            UiHaptics.tick(view)
                        }
                    },
            ) {
                Canvas(Modifier.fillMaxSize()) {
                    @Suppress("UNUSED_VARIABLE")
                    val rev = tileRevision
                    val maxTile = 1 shl zoom
                    val halfW = size.width / 2.0
                    val halfH = size.height / 2.0
                    val centerWorldX = lngToWorldX(centerLng, zoom)
                    val centerWorldY = latToWorldY(centerLat, zoom)
                    val xStart = floor((centerWorldX - halfW) / TILE_PX).toInt()
                    val xEnd = floor((centerWorldX + halfW) / TILE_PX).toInt()
                    val yStart = floor((centerWorldY - halfH) / TILE_PX).toInt()
                    val yEnd = floor((centerWorldY + halfH) / TILE_PX).toInt()
                    drawIntoCanvas {
                        for (ty in yStart..yEnd) {
                            if (ty < 0 || ty >= maxTile) continue
                            for (tx in xStart..xEnd) {
                                val wrappedX = ((tx % maxTile) + maxTile) % maxTile
                                val bmp = OsmTileLoader.getCached(zoom, wrappedX, ty) ?: continue
                                val dx = (tx * TILE_PX - centerWorldX + halfW).roundToInt()
                                val dy = (ty * TILE_PX - centerWorldY + halfH).roundToInt()
                                drawImage(
                                    image = bmp,
                                    dstOffset = IntOffset(dx, dy),
                                    dstSize = IntSize(TILE_PX.toInt() + 1, TILE_PX.toInt() + 1),
                                    colorFilter = if (darkTiles) PICKER_DARK_FILTER else null,
                                )
                            }
                        }
                    }

                    // Single unambiguous fixed center teardrop pin matching Uber/Grab UX
                    val cx = size.width / 2f
                    val cy = size.height / 2f
                    val lift = if (isDragging) 14f else 0f
                    drawCircle(
                        color = Color.Black.copy(alpha = if (isDragging) 0.18f else 0.28f),
                        radius = if (isDragging) 11f else 8f,
                        center = Offset(cx, cy),
                    )
                    val tipY = cy - lift
                    val headCenterY = tipY - 30f
                    val r = 16f
                    val pinPath = Path().apply {
                        moveTo(cx, tipY)
                        lineTo(cx - r * 0.72f, headCenterY + r * 0.45f)
                        lineTo(cx + r * 0.72f, headCenterY + r * 0.45f)
                        close()
                    }
                    drawPath(pinPath, color = primary)
                    drawCircle(color = primary, radius = r, center = Offset(cx, headCenterY))
                    drawCircle(color = Color.White, radius = 6.5f, center = Offset(cx, headCenterY))
                }

                // Zoom + My GPS controls
                Column(
                    modifier = Modifier
                        .align(Alignment.TopEnd)
                        .padding(Spacing.xs),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                ) {
                    FilledTonalIconButton(
                        onClick = {
                            UiHaptics.tick(view)
                            zoom = (zoom + 1).coerceAtMost(18)
                        },
                        modifier = Modifier.size(48.dp),
                    ) {
                        Icon(Icons.Outlined.Add, contentDescription = stringResource(R.string.pin_picker_zoom_in))
                    }
                    FilledTonalIconButton(
                        onClick = {
                            UiHaptics.tick(view)
                            zoom = (zoom - 1).coerceAtLeast(11)
                        },
                        modifier = Modifier.size(48.dp),
                    ) {
                        Icon(Icons.Outlined.Remove, contentDescription = stringResource(R.string.pin_picker_zoom_out))
                    }
                    FilledTonalIconButton(
                        onClick = ::requestGpsCenter,
                        enabled = !locatingGps,
                        modifier = Modifier.size(48.dp),
                    ) {
                        if (locatingGps) {
                            CircularProgressIndicator(strokeWidth = 2.dp, modifier = Modifier.size(18.dp))
                        } else {
                            Icon(Icons.Outlined.MyLocation, contentDescription = stringResource(R.string.pin_picker_my_gps))
                        }
                    }
                }

                // OSM attribution pill
                Surface(
                    color = MaterialTheme.colorScheme.surface.copy(alpha = 0.85f),
                    shape = CircleShape,
                    modifier = Modifier
                        .align(Alignment.BottomStart)
                        .padding(Spacing.xs),
                ) {
                    Text(
                        stringResource(R.string.pin_picker_osm_credit),
                        style = MaterialTheme.typography.labelSmall,
                        modifier = Modifier.padding(horizontal = Spacing.xs, vertical = 2.dp),
                    )
                }
            }

            val chosenLat = roundCoord6(centerLat)
            val chosenLng = roundCoord6(centerLng)
            Surface(
                color = MaterialTheme.colorScheme.surfaceContainerLow,
                shape = RoundedCornerShape(12.dp),
                modifier = Modifier.fillMaxWidth(),
            ) {
                Row(
                    Modifier
                        .fillMaxWidth()
                        .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    Icon(
                        Icons.Outlined.Place,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.size(18.dp),
                    )
                    Text(
                        text = if (hasAdjustedPin) {
                            String.format(Locale.US, "%.6f, %.6f", chosenLat, chosenLng)
                        } else {
                            stringResource(
                                R.string.pin_picker_center_preview,
                                String.format(Locale.US, "%.5f, %.5f", chosenLat, chosenLng),
                            )
                        },
                        style = MaterialTheme.typography.bodySmall.tabular().copy(fontWeight = FontWeight.Medium),
                        modifier = Modifier.weight(1f),
                    )
                }
            }
            if (gpsError) {
                Text(
                    stringResource(R.string.pin_picker_gps_unavailable),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                )
            }

            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                OutlinedButton(
                    onClick = {
                        UiHaptics.tick(view)
                        centerLat = startLat
                        centerLng = startLng
                        hasAdjustedPin = initialLat != null && initialLng != null
                        gpsError = false
                    },
                    modifier = Modifier
                        .weight(1f)
                        .defaultMinSize(minHeight = 48.dp),
                ) {
                    Icon(Icons.Outlined.RestartAlt, contentDescription = null, modifier = Modifier.size(18.dp))
                    Spacer(Modifier.size(Spacing.xxs))
                    Text(stringResource(R.string.pin_picker_reset))
                }
                Button(
                    onClick = {
                        UiHaptics.confirm(view)
                        onConfirm(chosenLat, chosenLng)
                        onDismiss()
                    },
                    colors = graceRidePrimaryButtonColors(),
                    modifier = Modifier
                        .weight(1f)
                        .defaultMinSize(minHeight = 48.dp),
                ) {
                    Icon(Icons.Outlined.Check, contentDescription = null, modifier = Modifier.size(18.dp))
                    Spacer(Modifier.size(Spacing.xxs))
                    Text(stringResource(R.string.pin_picker_confirm))
                }
            }
        }
        }
    }
}
