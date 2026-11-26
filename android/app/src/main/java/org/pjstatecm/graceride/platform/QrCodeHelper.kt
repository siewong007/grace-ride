package org.pjstatecm.graceride.platform

import android.content.Context
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.codescanner.GmsBarcodeScannerOptions
import com.google.mlkit.vision.codescanner.GmsBarcodeScanning
import com.google.zxing.BarcodeFormat
import com.google.zxing.EncodeHintType
import com.google.zxing.qrcode.QRCodeWriter

data class QrMatrix(val size: Int, val bits: BooleanArray) {
    operator fun get(x: Int, y: Int): Boolean = bits[y * size + x]

    override fun equals(other: Any?): Boolean =
        other is QrMatrix && size == other.size && bits.contentEquals(other.bits)

    override fun hashCode(): Int = 31 * size + bits.contentHashCode()
}

/** Pure-Kotlin ZXing QR matrix encoder (works identically on Android and JVM tests). */
fun generateQrMatrix(content: String, targetSize: Int = 0): QrMatrix? {
    val clean = content.trim()
    if (clean.isEmpty()) return null
    return runCatching {
        val hints = mapOf(
            EncodeHintType.MARGIN to 1,
            EncodeHintType.CHARACTER_SET to "UTF-8",
        )
        val matrix = QRCodeWriter().encode(clean, BarcodeFormat.QR_CODE, targetSize, targetSize, hints)
        val side = matrix.width
        val bits = BooleanArray(side * side)
        for (y in 0 until side) {
            for (x in 0 until side) {
                bits[y * side + x] = matrix.get(x, y)
            }
        }
        QrMatrix(side, bits)
    }.getOrNull()
}

/** Vector QR code renderer framed in a high-contrast white quiet zone for dark/light mode reliability. */
@Composable
fun QrCodeCanvas(
    content: String,
    modifier: Modifier = Modifier,
) {
    val matrix = remember(content) { generateQrMatrix(content, targetSize = 0) }
    Surface(
        color = Color.White,
        shape = RoundedCornerShape(12.dp),
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
        modifier = modifier,
    ) {
        Box(Modifier.fillMaxSize().padding(12.dp)) {
            if (matrix != null) {
                Canvas(Modifier.fillMaxSize()) {
                    val cell = size.minDimension / matrix.size.toFloat()
                    val ox = (size.width - cell * matrix.size) * 0.5f
                    val oy = (size.height - cell * matrix.size) * 0.5f
                    val cellSize = Size(cell + 0.5f, cell + 0.5f)
                    for (y in 0 until matrix.size) {
                        for (x in 0 until matrix.size) {
                            if (matrix[x, y]) {
                                drawRect(
                                    color = Color.Black,
                                    topLeft = Offset(ox + x * cell, oy + y * cell),
                                    size = cellSize,
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

sealed interface CheckinParseResult {
    data class Code(val value: String) : CheckinParseResult
    data object WrongEvent : CheckinParseResult
    data object Invalid : CheckinParseResult
}

private val CHECKIN_EVENT_PATH_RE = Regex("/events/(\\d+)/checkin", RegexOption.IGNORE_CASE)
private val GRPASS_RE = Regex("^grpass:(\\d*):(.+)$", RegexOption.IGNORE_CASE)

private fun parseQueryParams(rawQuery: String?): Map<String, String> {
    val q = rawQuery?.trim()?.takeIf { it.isNotEmpty() } ?: return emptyMap()
    val out = LinkedHashMap<String, String>()
    for (pair in q.split('&')) {
        val idx = pair.indexOf('=')
        if (idx <= 0) continue
        val key = runCatching { java.net.URLDecoder.decode(pair.substring(0, idx), "UTF-8") }.getOrDefault(pair.substring(0, idx))
        val value = runCatching { java.net.URLDecoder.decode(pair.substring(idx + 1), "UTF-8") }.getOrDefault(pair.substring(idx + 1))
        out[key] = value
    }
    return out
}

/**
 * Normalizes a scanned or pasted venue check-in code or URL (supporting query `code=...`,
 * URL encoding, `#fragment` stripping, and mismatching `/events/{otherId}` detection).
 */
fun parseCheckinInput(raw: String, currentEventId: Long): CheckinParseResult {
    val trimmed = raw.trim()
    if (trimmed.isEmpty()) return CheckinParseResult.Invalid
    val looksLikeUrl = trimmed.startsWith("http://", ignoreCase = true) ||
        trimmed.startsWith("https://", ignoreCase = true) ||
        trimmed.startsWith("/events/", ignoreCase = true) ||
        trimmed.contains("code=", ignoreCase = true)
    if (!looksLikeUrl) return CheckinParseResult.Code(trimmed)

    val uri = runCatching {
        java.net.URI(if (trimmed.startsWith("code=", ignoreCase = true)) "https://local/?$trimmed" else trimmed)
    }.getOrNull()
    val path = uri?.path ?: trimmed.substringBefore('?').substringBefore('#')
    val pathEventId = CHECKIN_EVENT_PATH_RE.find(path)?.groupValues?.get(1)?.toLongOrNull()
    if (pathEventId != null && pathEventId != currentEventId) {
        return CheckinParseResult.WrongEvent
    }
    val rawQuery = uri?.rawQuery ?: trimmed.substringAfter('?', "").substringBefore('#')
    val code = parseQueryParams(rawQuery)["code"]?.trim()
    return if (!code.isNullOrEmpty()) CheckinParseResult.Code(code) else CheckinParseResult.Invalid
}

/**
 * Normalizes a scanned or manually entered member pass (`GRPASS:{eventId}:{code}`,
 * `GRPASS::{code}`, mixed case, or bare pass code), rejecting wrong-event passes locally.
 */
fun parseMemberPassInput(raw: String, currentEventId: Long): CheckinParseResult {
    val trimmed = raw.trim()
    if (trimmed.isEmpty()) return CheckinParseResult.Invalid
    val m = GRPASS_RE.matchEntire(trimmed) ?: return CheckinParseResult.Code(trimmed)
    val eventPart = m.groupValues[1]
    val pass = m.groupValues[2].trim()
    if (pass.isEmpty()) return CheckinParseResult.Invalid
    if (eventPart.isNotEmpty() && eventPart.toLongOrNull() != currentEventId) {
        return CheckinParseResult.WrongEvent
    }
    return CheckinParseResult.Code(pass)
}

/**
 * Launches Google Play Services native QR Code Scanner (zero `CAMERA` permission required).
 * Calls [onUnavailable] when Play Services scanning isn't present so UI offers manual entry.
 */
fun launchNativeQrScanner(
    context: Context,
    onResult: (String) -> Unit,
    onUnavailable: () -> Unit,
) {
    try {
        val options = GmsBarcodeScannerOptions.Builder()
            .setBarcodeFormats(Barcode.FORMAT_QR_CODE)
            .enableAutoZoom()
            .build()
        GmsBarcodeScanning.getClient(context, options)
            .startScan()
            .addOnSuccessListener { barcode ->
                val raw = barcode.rawValue?.trim()
                if (!raw.isNullOrEmpty()) onResult(raw)
            }
            .addOnFailureListener {
                onUnavailable()
            }
    } catch (_: Exception) {
        onUnavailable()
    }
}
