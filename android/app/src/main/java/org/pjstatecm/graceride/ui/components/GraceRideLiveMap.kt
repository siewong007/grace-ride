package org.pjstatecm.graceride.ui.components

import android.content.Context
import android.graphics.BitmapFactory
import android.util.LruCache
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Flag
import androidx.compose.material.icons.outlined.MyLocation
import androidx.compose.material.icons.outlined.Remove
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.ColorMatrix
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalInspectionMode
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import okhttp3.Cache
import okhttp3.Call
import okhttp3.Callback
import okhttp3.CookieJar
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.ui.theme.GrPrimary
import org.pjstatecm.graceride.ui.theme.GrSuccess
import java.io.File
import java.io.IOException
import java.util.concurrent.TimeUnit
import kotlin.coroutines.resume
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.floor
import kotlin.math.ln
import kotlin.math.max
import kotlin.math.min
import kotlin.math.pow
import kotlin.math.roundToInt
import kotlin.math.sin
import kotlin.math.tan

enum class MapMarkerKind { CAR, PERSON, STOP, DEST }

data class LiveMapPoint(
    val id: String,
    val lat: Double,
    val lng: Double,
    val label: String = "",
    val badge: String? = null,
    val kind: MapMarkerKind = MapMarkerKind.CAR,
    val stale: Boolean = false,
)

private object OsmTileLoader {
    private const val TILE_CACHE_KB = 16 * 1024
    private val memory = object : LruCache<String, ImageBitmap>(TILE_CACHE_KB) {
        override fun sizeOf(key: String, value: ImageBitmap): Int =
            ((value.width * value.height * 4) / 1024).coerceAtLeast(1)
    }

    @Volatile
    private var client: OkHttpClient? = null

    private fun getClient(context: Context): OkHttpClient? {
        client?.let { return it }
        val cacheDir = runCatching { context.cacheDir }.getOrNull() ?: return null
        return synchronized(this) {
            client ?: OkHttpClient.Builder()
                .cookieJar(CookieJar.NO_COOKIES)
                .cache(Cache(File(cacheDir, "osm_tiles"), 24L * 1024L * 1024L))
                .connectTimeout(8, TimeUnit.SECONDS)
                .readTimeout(12, TimeUnit.SECONDS)
                .build()
                .also { client = it }
        }
    }

    fun getCached(z: Int, x: Int, y: Int): ImageBitmap? = memory.get("$z/$x/$y")

    suspend fun load(context: Context, z: Int, x: Int, y: Int): ImageBitmap? {
        val key = "$z/$x/$y"
        memory.get(key)?.let { return it }
        val http = getClient(context) ?: return null
        val req = Request.Builder()
            .url("https://tile.openstreetmap.org/$z/$x/$y.png")
            .header("User-Agent", "GraceRideAndroid/0.1 (+https://graceride.saliminn.my)")
            .build()
        val bytes = suspendCancellableCoroutine<ByteArray?> { cont ->
            val call = http.newCall(req)
            cont.invokeOnCancellation { call.cancel() }
            call.enqueue(object : Callback {
                override fun onFailure(call: Call, e: IOException) {
                    if (cont.isActive) cont.resume(null)
                }

                override fun onResponse(call: Call, response: Response) {
                    val data = response.use { r ->
                        if (r.isSuccessful) runCatching { r.body?.bytes() }.getOrNull() else null
                    }
                    if (cont.isActive) cont.resume(data)
                }
            })
        } ?: return null
        return withContext(Dispatchers.Default) {
            memory.get(key)?.let { return@withContext it }
            runCatching {
                BitmapFactory.decodeByteArray(bytes, 0, bytes.size)?.asImageBitmap()?.also { bmp ->
                    memory.put(key, bmp)
                }
            }.getOrNull()
        }
    }
}

private const val DEFAULT_LAT = 3.1073
private const val DEFAULT_LNG = 101.6067
private const val TILE_PX = 256.0
private val AMBER_PIN = Color(0xFFB47616)

private val DARK_MAP_FILTER = ColorFilter.colorMatrix(
    ColorMatrix(
        floatArrayOf(
            0.78f, 0f, 0f, 0f, -18f,
            0f, 0.80f, 0f, 0f, -16f,
            0f, 0f, 0.86f, 0f, -8f,
            0f, 0f, 0f, 1f, 0f,
        ),
    ),
)

private fun lngToWorldX(lng: Double, zoom: Int): Double =
    ((lng + 180.0) / 360.0) * TILE_PX * 2.0.pow(zoom)

private fun latToWorldY(lat: Double, zoom: Int): Double {
    val rad = Math.toRadians(lat.coerceIn(-85.0511, 85.0511))
    val n = ln(tan(rad) + 1.0 / cos(rad))
    return ((1.0 - n / PI) / 2.0) * TILE_PX * 2.0.pow(zoom)
}

private fun fitViewport(
    points: List<LiveMapPoint>,
    route: List<List<Double>>,
    widthPx: Int,
    heightPx: Int,
): Triple<Double, Double, Int> {
    val coords = buildList {
        points.forEach { add(it.lat to it.lng) }
        route.forEach { if (it.size >= 2) add(it[0] to it[1]) }
    }
    if (coords.isEmpty()) return Triple(DEFAULT_LAT, DEFAULT_LNG, 14)
    if (coords.size == 1) return Triple(coords[0].first, coords[0].second, 15)

    val minLat = coords.minOf { it.first }
    val maxLat = coords.maxOf { it.first }
    val minLng = coords.minOf { it.second }
    val maxLng = coords.maxOf { it.second }
    val cLat = (minLat + maxLat) / 2.0
    val cLng = (minLng + maxLng) / 2.0

    val availW = max(120.0, widthPx * 0.72)
    val availH = max(80.0, heightPx * 0.68)
    for (z in 16 downTo 11) {
        val dx = abs(lngToWorldX(maxLng, z) - lngToWorldX(minLng, z))
        val dy = abs(latToWorldY(minLat, z) - latToWorldY(maxLat, z))
        if (dx <= availW && dy <= availH) return Triple(cLat, cLng, z)
    }
    return Triple(cLat, cLng, 11)
}

/**
 * Native Compose OpenStreetMap viewport matching web `LiveMap.tsx`:
 * zero external SDK/API key requirements, cookie-free tile fetches on Dispatchers.IO,
 * and Grace Ride marker colors (Ride Blue car, Green rider/stops, Amber destination).
 */
@Composable
fun GraceRideLiveMap(
    points: List<LiveMapPoint>,
    modifier: Modifier = Modifier,
    route: List<List<Double>> = emptyList(),
    focusPoint: Pair<Double, Double>? = null,
    showZoomControls: Boolean = false,
    onFitAll: (() -> Unit)? = null,
) {
    val context = LocalContext.current
    val inspectionMode = LocalInspectionMode.current
    val darkTheme = MaterialTheme.colorScheme.background.luminance() < 0.5f
    val borderColor = MaterialTheme.colorScheme.outlineVariant
    val primaryLine = MaterialTheme.colorScheme.primary
    val fallbackLand = if (darkTheme) Color(0xFF1B1E24) else Color(0xFFF3F5F8)
    val fallbackGrid = if (darkTheme) Color(0xFF262A33) else Color(0xFFE4E8EE)

    var viewportSize by remember { mutableStateOf(IntSize.Zero) }
    var panOffsetPx by remember { mutableStateOf(Offset.Zero) }
    var tileRevision by remember { mutableIntStateOf(0) }
    val routePath = remember { Path() }

    val fitKey = remember(points, route, focusPoint) {
        val ids = points.map { it.id }.sorted().joinToString(",")
        val start = route.firstOrNull()?.take(2)?.joinToString(",") ?: ""
        val end = route.lastOrNull()?.take(2)?.joinToString(",") ?: ""
        val focus = focusPoint?.let { "${it.first},${it.second}" } ?: ""
        "$ids|${route.size}|$start|$end|$focus"
    }
    var zoomDelta by remember(fitKey) { mutableIntStateOf(0) }
    LaunchedEffect(fitKey) {
        panOffsetPx = Offset.Zero
    }

    val (centerLat, centerLng, baseZoom) = remember(fitKey, viewportSize) {
        if (focusPoint != null) {
            Triple(focusPoint.first, focusPoint.second, 15)
        } else {
            fitViewport(points, route, viewportSize.width, viewportSize.height)
        }
    }
    val zoom = (baseZoom + zoomDelta).coerceIn(10, 18)

    val centerWorldX = lngToWorldX(centerLng, zoom) - panOffsetPx.x
    val centerWorldY = latToWorldY(centerLat, zoom) - panOffsetPx.y
    val maxTile = (1 shl zoom)

    val visibleTiles = remember(centerWorldX, centerWorldY, zoom, viewportSize) {
        if (viewportSize.width <= 0 || viewportSize.height <= 0) {
            emptyList()
        } else {
            val leftWorld = centerWorldX - viewportSize.width / 2.0
            val topWorld = centerWorldY - viewportSize.height / 2.0
            val rightWorld = centerWorldX + viewportSize.width / 2.0
            val bottomWorld = centerWorldY + viewportSize.height / 2.0
            val xStart = floor(leftWorld / TILE_PX).toInt()
            val xEnd = floor(rightWorld / TILE_PX).toInt()
            val yStart = floor(topWorld / TILE_PX).toInt()
            val yEnd = floor(bottomWorld / TILE_PX).toInt()
            buildList {
                for (tx in xStart..xEnd) {
                    val wrappedX = ((tx % maxTile) + maxTile) % maxTile
                    for (ty in yStart..yEnd) {
                        if (ty in 0 until maxTile) {
                            add(Triple(tx, wrappedX, ty))
                        }
                    }
                }
            }
        }
    }

    LaunchedEffect(visibleTiles, zoom, inspectionMode) {
        if (inspectionMode || visibleTiles.isEmpty()) return@LaunchedEffect
        visibleTiles.forEach { (_, wrappedX, ty) ->
            if (OsmTileLoader.getCached(zoom, wrappedX, ty) == null) {
                launch {
                    if (OsmTileLoader.load(context, zoom, wrappedX, ty) != null) {
                        tileRevision++
                    }
                }
            }
        }
    }

    val routeScreenPoints = remember(route, centerWorldX, centerWorldY, zoom, viewportSize) {
        if (viewportSize.width <= 0 || viewportSize.height <= 0 || route.size < 2) {
            emptyList()
        } else {
            val halfW = viewportSize.width / 2.0
            val halfH = viewportSize.height / 2.0
            route.mapNotNull { coord ->
                if (coord.size < 2) null
                else {
                    val sx = (lngToWorldX(coord[1], zoom) - centerWorldX + halfW).toFloat()
                    val sy = (latToWorldY(coord[0], zoom) - centerWorldY + halfH).toFloat()
                    Offset(sx, sy)
                }
            }
        }
    }

    Box(
        modifier = modifier
            .clip(RoundedCornerShape(10.dp))
            .background(fallbackLand)
            .border(1.dp, borderColor, RoundedCornerShape(10.dp))
            .onSizeChanged { viewportSize = it }
            .pointerInput(Unit) {
                detectDragGestures { change, dragAmount ->
                    change.consume()
                    panOffsetPx += dragAmount
                }
            },
    ) {
        Canvas(Modifier.fillMaxSize()) {
            @Suppress("UNUSED_EXPRESSION")
            tileRevision
            val halfW = size.width / 2.0
            val halfH = size.height / 2.0

            // Subtle fallback street grid while tiles stream in
            val step = 44.dp.toPx()
            var gx = (-panOffsetPx.x % step + step) % step
            while (gx < size.width) {
                drawLine(fallbackGrid, Offset(gx, 0f), Offset(gx, size.height), strokeWidth = 1.dp.toPx())
                gx += step
            }
            var gy = (-panOffsetPx.y % step + step) % step
            while (gy < size.height) {
                drawLine(fallbackGrid, Offset(0f, gy), Offset(size.width, gy), strokeWidth = 1.dp.toPx())
                gy += step
            }

            visibleTiles.forEach { (tx, wrappedX, ty) ->
                val bmp = OsmTileLoader.getCached(zoom, wrappedX, ty) ?: return@forEach
                val dx = (tx * TILE_PX - centerWorldX + halfW).roundToInt()
                val dy = (ty * TILE_PX - centerWorldY + halfH).roundToInt()
                drawImage(
                    image = bmp,
                    dstOffset = IntOffset(dx, dy),
                    dstSize = IntSize(TILE_PX.toInt() + 1, TILE_PX.toInt() + 1),
                    colorFilter = if (darkTheme) DARK_MAP_FILTER else null,
                )
            }

            if (routeScreenPoints.size >= 2) {
                routePath.reset()
                routePath.moveTo(routeScreenPoints[0].x, routeScreenPoints[0].y)
                for (i in 1 until routeScreenPoints.size) {
                    routePath.lineTo(routeScreenPoints[i].x, routeScreenPoints[i].y)
                }
                drawPath(
                    path = routePath,
                    color = Color.White.copy(alpha = 0.85f),
                    style = Stroke(width = 6.dp.toPx(), cap = StrokeCap.Round, join = StrokeJoin.Round),
                )
                drawPath(
                    path = routePath,
                    color = primaryLine,
                    style = Stroke(width = 3.8.dp.toPx(), cap = StrokeCap.Round, join = StrokeJoin.Round),
                )
            }
        }

        // Compose Marker Pins positioned over the map canvas
        if (viewportSize.width > 0 && viewportSize.height > 0) {
            val density = LocalDensity.current
            val pinHalfPx = with(density) { 14.dp.roundToPx() }
            val halfW = viewportSize.width / 2.0
            val halfH = viewportSize.height / 2.0
            points.forEach { p ->
                val sx = (lngToWorldX(p.lng, zoom) - centerWorldX + halfW).roundToInt()
                val sy = (latToWorldY(p.lat, zoom) - centerWorldY + halfH).roundToInt()
                if (sx in -48..(viewportSize.width + 48) && sy in -48..(viewportSize.height + 48)) {
                    val pinBg = when {
                        p.kind == MapMarkerKind.CAR && p.stale -> AMBER_PIN
                        p.kind == MapMarkerKind.CAR -> GrPrimary
                        p.kind == MapMarkerKind.DEST -> AMBER_PIN
                        else -> GrSuccess
                    }
                    Row(
                        modifier = Modifier
                            .align(Alignment.TopStart)
                            .offset { IntOffset(sx - pinHalfPx, sy - pinHalfPx) },
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        Box(
                            modifier = Modifier
                                .size(28.dp)
                                .clip(CircleShape)
                                .background(pinBg)
                                .border(2.dp, Color.White, CircleShape),
                            contentAlignment = Alignment.Center,
                        ) {
                            when {
                                !p.badge.isNullOrBlank() -> Text(
                                    text = p.badge,
                                    color = Color.White,
                                    fontSize = 11.sp,
                                    fontWeight = FontWeight.Bold,
                                )
                                p.kind == MapMarkerKind.CAR -> Icon(
                                    Icons.Outlined.DirectionsCar,
                                    contentDescription = p.label,
                                    tint = Color.White,
                                    modifier = Modifier.size(16.dp),
                                )
                                p.kind == MapMarkerKind.DEST -> Icon(
                                    Icons.Outlined.Flag,
                                    contentDescription = p.label,
                                    tint = Color.White,
                                    modifier = Modifier.size(15.dp),
                                )
                                else -> Box(
                                    Modifier
                                        .size(8.dp)
                                        .clip(CircleShape)
                                        .background(Color.White),
                                )
                            }
                        }
                        if (p.label.isNotBlank()) {
                            Surface(
                                color = MaterialTheme.colorScheme.surface.copy(alpha = 0.94f),
                                contentColor = MaterialTheme.colorScheme.onSurface,
                                shape = RoundedCornerShape(6.dp),
                                border = androidx.compose.foundation.BorderStroke(1.dp, borderColor),
                            ) {
                                Text(
                                    text = p.label,
                                    style = MaterialTheme.typography.labelSmall,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                    modifier = Modifier
                                        .widthIn(max = 128.dp)
                                        .padding(horizontal = 6.dp, vertical = 2.dp),
                                )
                            }
                        }
                    }
                }
            }
        }

        if (showZoomControls) {
            Column(
                modifier = Modifier
                    .align(Alignment.TopEnd)
                    .padding(8.dp),
                horizontalAlignment = Alignment.End,
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                Surface(
                    color = MaterialTheme.colorScheme.surface.copy(alpha = 0.94f),
                    shape = RoundedCornerShape(10.dp),
                    border = androidx.compose.foundation.BorderStroke(1.dp, borderColor),
                ) {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        IconButton(
                            onClick = { if (zoom < 18) zoomDelta++ },
                            enabled = zoom < 18,
                            modifier = Modifier.size(36.dp),
                        ) {
                            Icon(
                                Icons.Outlined.Add,
                                contentDescription = stringResource(R.string.traffic_live_zoom_in),
                                modifier = Modifier.size(18.dp),
                            )
                        }
                        HorizontalDivider(Modifier.widthIn(max = 36.dp), color = borderColor)
                        IconButton(
                            onClick = { if (zoom > 10) zoomDelta-- },
                            enabled = zoom > 10,
                            modifier = Modifier.size(36.dp),
                        ) {
                            Icon(
                                Icons.Outlined.Remove,
                                contentDescription = stringResource(R.string.traffic_live_zoom_out),
                                modifier = Modifier.size(18.dp),
                            )
                        }
                    }
                }
                if (focusPoint != null || panOffsetPx.getDistance() > 24f || zoomDelta != 0) {
                    Surface(
                        onClick = {
                            panOffsetPx = Offset.Zero
                            zoomDelta = 0
                            onFitAll?.invoke()
                        },
                        color = MaterialTheme.colorScheme.surface.copy(alpha = 0.94f),
                        contentColor = MaterialTheme.colorScheme.primary,
                        shape = CircleShape,
                        border = androidx.compose.foundation.BorderStroke(1.dp, borderColor),
                    ) {
                        Row(
                            Modifier.padding(horizontal = 10.dp, vertical = 5.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(4.dp),
                        ) {
                            Icon(Icons.Outlined.MyLocation, contentDescription = null, modifier = Modifier.size(14.dp))
                            Text(stringResource(R.string.traffic_live_fit_all), style = MaterialTheme.typography.labelSmall)
                        }
                    }
                }
            }
        } else if (panOffsetPx.getDistance() > 24f) {
            Surface(
                onClick = { panOffsetPx = Offset.Zero },
                color = MaterialTheme.colorScheme.surface.copy(alpha = 0.94f),
                contentColor = MaterialTheme.colorScheme.primary,
                shape = CircleShape,
                border = androidx.compose.foundation.BorderStroke(1.dp, borderColor),
                modifier = Modifier
                    .align(Alignment.TopEnd)
                    .padding(8.dp),
            ) {
                Row(
                    Modifier.padding(horizontal = 10.dp, vertical = 5.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    Icon(Icons.Outlined.MyLocation, contentDescription = null, modifier = Modifier.size(14.dp))
                    Text(stringResource(R.string.map_recenter), style = MaterialTheme.typography.labelSmall)
                }
            }
        }

        Text(
            text = "© OpenStreetMap",
            fontSize = 9.sp,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier
                .align(Alignment.BottomEnd)
                .background(MaterialTheme.colorScheme.surface.copy(alpha = 0.72f), RoundedCornerShape(topStart = 4.dp))
                .padding(horizontal = 5.dp, vertical = 1.dp),
        )
    }
}
