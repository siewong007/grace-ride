package org.pjstatecm.graceride.ui.components


import android.graphics.Bitmap
import android.graphics.Canvas as AndroidCanvas
import android.graphics.Matrix as AndroidMatrix
import android.graphics.Paint as AndroidPaint
import android.graphics.Path as AndroidPath
import android.graphics.RadialGradient as AndroidRadialGradient
import android.graphics.RectF
import android.graphics.Shader
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.asComposePath
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.ui.theme.Spacing
import java.util.Locale
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.exp
import kotlin.math.hypot
import kotlin.math.sin
import kotlin.math.tan
import kotlin.random.Random

data class GrokBotSpec(
    val shape: String = "circle",
    val face: String = "neutral",
    val color: String = "blue",
) {
    fun encode(): String = "grokbot:$shape:$face:$color"

    fun filename(ext: String): String = "grok-bot-$shape-$face-$color.$ext"

    companion object {
        val SHAPES = listOf(
            "circle", "pebble", "squircle", "capsule",
            "triangle", "hexagon", "cloud", "droplet",
        )
        val FACES = listOf(
            "neutral", "attentive", "surprised", "excited",
            "happy", "laughing", "angry", "sad",
            "scared", "suspicious", "confused", "curious",
            "proud", "shy", "unimpressed", "sleepy",
        )
        val COLORS = listOf(
            "ink", "brown", "red", "orange", "amber", "green",
            "turquoise", "blue", "purple", "pink", "grey", "cream",
        )
        val MOVES = listOf(
            "idle", "thinking", "wink", "wide_eyes", "alert",
            "notification", "exclamation", "sleep", "egg", "hexagon",
            "play", "orbit", "swirl", "burst", "comet",
        )

        val DEFAULT = GrokBotSpec("circle", "neutral", "blue")

        fun isGrokBotIcon(raw: String?): Boolean {
            val t = raw?.trim() ?: return false
            return t == "grokbot" || t.startsWith("grokbot:")
        }

        fun parse(raw: String?): GrokBotSpec? {
            val t = raw?.trim() ?: return null
            if (t == "grokbot") return DEFAULT
            if (!t.startsWith("grokbot:")) return null
            val parts = t.split(':')
            if (parts.size != 4) return null
            val sh = parts[1]
            val fa = parts[2]
            val co = parts[3]
            if (sh !in SHAPES || fa !in FACES || co !in COLORS) return null
            return GrokBotSpec(sh, fa, co)
        }

        fun random(exclude: GrokBotSpec? = null): GrokBotSpec {
            repeat(12) {
                val candidate = GrokBotSpec(
                    shape = SHAPES[Random.nextInt(SHAPES.size)],
                    face = FACES[Random.nextInt(FACES.size)],
                    color = COLORS[Random.nextInt(COLORS.size)],
                )
                if (candidate != exclude) return candidate
            }
            return GrokBotSpec("squircle", "happy", "purple")
        }
    }
}

data class GrokClayPalette(
    val hex: String,
    val glowColor: Long,
    val stopHexes: List<String>,
    val stopColors: List<Long>,
)

data class GrokEyePill(
    val halfW: Float,
    val halfH: Float,
    val rx: Float,
    val ry: Float,
    val svgPath: String,
)

data class GrokFaceDef(
    val left: GrokEyePill,
    val right: GrokEyePill,
    val lx: Float,
    val ly: Float,
    val rx: Float,
    val ry: Float,
    val lDeg: Float,
    val rDeg: Float,
    val lSy: Float = 1f,
    val rSy: Float = 1f,
    val gazeBiasX: Float = 0f,
    val gazeBiasY: Float = 0f,
)

data class GrokEyeTransform(
    val a: Float,
    val b: Float,
    val c: Float,
    val d: Float,
    val tx: Float,
    val ty: Float,
) {
    fun toSvgMatrix(): String = String.format(
        Locale.US,
        "matrix(%.3f,%.3f,%.3f,%.3f,%.2f,%.2f)",
        a, b, c, d, tx, ty,
    )
}

object GrokBotGeometry {
    private val NUM_REGEX = Regex("""-?\d*\.?\d+""")

    private fun fmt(v: Float): String {
        val s = String.format(Locale.US, "%.2f", v)
        return if (s.contains('.')) s.trimEnd('0').trimEnd('.') else s
    }

    fun pill(rx: Float, straightHalf: Float, ry: Float = rx): GrokEyePill {
        return if (straightHalf == 0f) {
            val d = "M-${fmt(rx)} 0A${fmt(rx)} ${fmt(ry)} 0 0 1 0 -${fmt(ry)}" +
                "A${fmt(rx)} ${fmt(ry)} 0 0 1 ${fmt(rx)} 0" +
                "A${fmt(rx)} ${fmt(ry)} 0 0 1 0 ${fmt(ry)}" +
                "A${fmt(rx)} ${fmt(ry)} 0 0 1 -${fmt(rx)} 0Z"
            GrokEyePill(rx, ry, rx, ry, d)
        } else {
            val top = rx + straightHalf
            val d = "M-${fmt(rx)} -${fmt(straightHalf)}A${fmt(rx)} ${fmt(rx)} 0 0 1 0 -${fmt(top)}" +
                "L0 -${fmt(top)}A${fmt(rx)} ${fmt(rx)} 0 0 1 ${fmt(rx)} -${fmt(straightHalf)}" +
                "L${fmt(rx)} ${fmt(straightHalf)}A${fmt(rx)} ${fmt(rx)} 0 0 1 0 ${fmt(top)}" +
                "L0 ${fmt(top)}A${fmt(rx)} ${fmt(rx)} 0 0 1 -${fmt(rx)} ${fmt(straightHalf)}Z"
            GrokEyePill(rx, top, rx, rx, d)
        }
    }

    fun hpill(halfW: Float, ry: Float): GrokEyePill {
        val sx = (halfW - ry).coerceAtLeast(0f)
        val d = "M-${fmt(halfW)} 0A${fmt(ry)} ${fmt(ry)} 0 0 1 -${fmt(sx)} -${fmt(ry)}" +
            "L${fmt(sx)} -${fmt(ry)}A${fmt(ry)} ${fmt(ry)} 0 0 1 ${fmt(halfW)} 0" +
            "L${fmt(halfW)} 0A${fmt(ry)} ${fmt(ry)} 0 0 1 ${fmt(sx)} ${fmt(ry)}" +
            "L-${fmt(sx)} ${fmt(ry)}A${fmt(ry)} ${fmt(ry)} 0 0 1 -${fmt(halfW)} 0Z"
        return GrokEyePill(halfW, ry, ry, ry, d)
    }

    val COLOR_PALETTES: Map<String, GrokClayPalette> = mapOf(
        "ink" to GrokClayPalette("#0a0a0c", 0x2E0A0A0C, listOf("#a2a2a3", "#3b3b3d", "#0a0a0c", "#0a0a0c"), listOf(0xFFA2A2A3, 0xFF3B3B3D, 0xFF0A0A0C, 0xFF0A0A0C)),
        "brown" to GrokClayPalette("#8b5e3c", 0x2E8B5E3C, listOf("#d3c2b5", "#a27e63", "#8b5e3c", "#573c29"), listOf(0xFFD3C2B5, 0xFFA27E63, 0xFF8B5E3C, 0xFF573C29)),
        "red" to GrokClayPalette("#e8483f", 0x2EE8483F, listOf("#f6b9b6", "#ed6d65", "#e8483f", "#8f2f2b"), listOf(0xFFF6B9B6, 0xFFED6D65, 0xFFE8483F, 0xFF8F2F2B)),
        "orange" to GrokClayPalette("#f08a24", 0x2EF08A24, listOf("#f9d3ac", "#f3a150", "#f08a24", "#94571a"), listOf(0xFFF9D3AC, 0xFFF3A150, 0xFFF08A24, 0xFF94571A)),
        "amber" to GrokClayPalette("#f0b429", 0x2EF0B429, listOf("#f9e3ae", "#f3c354", "#f0b429", "#94701d"), listOf(0xFFF9E3AE, 0xFFF3C354, 0xFFF0B429, 0xFF94701D)),
        "green" to GrokClayPalette("#3ecf8e", 0x2E3ECF8E, listOf("#b6edd4", "#65d9a5", "#3ecf8e", "#29805a"), listOf(0xFFB6EDD4, 0xFF65D9A5, 0xFF3ECF8E, 0xFF29805A)),
        "turquoise" to GrokClayPalette("#2fbfa0", 0x2E2FBFA0, listOf("#b0e7db", "#59ccb3", "#2fbfa0", "#207765"), listOf(0xFFB0E7DB, 0xFF59CCB3, 0xFF2FBFA0, 0xFF207765)),
        "blue" to GrokClayPalette("#3b93f0", 0x2E3B93F0, listOf("#b5d6f9", "#62a9f3", "#3b93f0", "#275c95"), listOf(0xFFB5D6F9, 0xFF62A9F3, 0xFF3B93F0, 0xFF275C95)),
        "purple" to GrokClayPalette("#8b5cf6", 0x2E8B5CF6, listOf("#d3c1fc", "#a27df8", "#8b5cf6", "#573b98"), listOf(0xFFD3C1FC, 0xFFA27DF8, 0xFF8B5CF6, 0xFF573B98)),
        "pink" to GrokClayPalette("#e152b0", 0x2EE152B0, listOf("#f4bde1", "#e775c0", "#e152b0", "#8b356e"), listOf(0xFFF4BDE1, 0xFFE775C0, 0xFFE152B0, 0xFF8B356E)),
        "grey" to GrokClayPalette("#a3a3a3", 0x2EA3A3A3, listOf("#dcdcdc", "#b5b5b5", "#a3a3a3", "#666667"), listOf(0xFFDCDCDC, 0xFFB5B5B5, 0xFFA3A3A3, 0xFF666667)),
        "cream" to GrokClayPalette("#f1efe9", 0x2EF1EFE9, listOf("#faf9f7", "#f4f2ed", "#f1efe9", "#959391"), listOf(0xFFFAF9F7, 0xFFF4F2ED, 0xFFF1EFE9, 0xFF959391)),
    )

    val SHAPE_PATHS: Map<String, String> = mapOf(
        "circle" to "M100.1 0.26C100.1 3.55 99.94 6.85 99.62 10.11C99.3 13.38 98.82 16.65 98.18 19.87C97.54 23.09 96.74 26.29 95.8 29.43C94.85 32.57 93.74 35.68 92.49 38.72C91.24 41.75 89.83 44.74 88.29 47.63C86.75 50.53 85.06 53.36 83.25 56.09C81.43 58.82 79.48 61.47 77.4 64.01C75.33 66.55 73.12 68.99 70.81 71.32C68.5 73.64 66.07 75.85 63.54 77.94C61.02 80.02 58.38 81.99 55.66 83.81C52.94 85.64 50.12 87.33 47.24 88.88C44.36 90.43 41.39 91.84 38.37 93.1C35.35 94.35 32.26 95.47 29.13 96.42C26 97.37 22.82 98.17 19.61 98.81C16.41 99.46 13.16 99.94 9.9 100.26C6.65 100.58 3.37 100.75 0.1 100.75C-3.16 100.75 -6.45 100.58 -9.7 100.26C-12.95 99.94 -16.2 99.46 -19.41 98.81C-22.61 98.17 -25.8 97.37 -28.93 96.42C-32.05 95.47 -35.15 94.35 -38.17 93.1C-41.18 91.84 -44.16 90.43 -47.04 88.88C-49.92 87.33 -52.74 85.64 -55.45 83.81C-58.17 81.99 -60.81 80.02 -63.34 77.94C-65.86 75.85 -68.3 73.64 -70.61 71.32C-72.92 68.99 -75.13 66.55 -77.2 64.01C-79.27 61.47 -81.23 58.82 -83.04 56.09C-84.86 53.36 -86.55 50.53 -88.09 47.63C-89.63 44.74 -91.04 41.75 -92.29 38.72C-93.54 35.68 -94.64 32.57 -95.59 29.43C-96.54 26.29 -97.34 23.09 -97.98 19.87C-98.61 16.65 -99.1 13.38 -99.42 10.11C-99.74 6.85 -99.9 3.55 -99.9 0.26C-99.9 -3.02 -99.74 -6.32 -99.42 -9.58C-99.1 -12.85 -98.61 -16.12 -97.98 -19.34C-97.34 -22.56 -96.54 -25.76 -95.59 -28.9C-94.64 -32.05 -93.54 -35.15 -92.29 -38.19C-91.04 -41.22 -89.63 -44.21 -88.09 -47.1C-86.55 -50 -84.86 -52.83 -83.04 -55.56C-81.23 -58.29 -79.27 -60.94 -77.2 -63.48C-75.13 -66.02 -72.92 -68.46 -70.61 -70.79C-68.3 -73.11 -65.86 -75.33 -63.34 -77.41C-60.81 -79.49 -58.17 -81.46 -55.45 -83.28C-52.74 -85.11 -49.92 -86.8 -47.04 -88.35C-44.16 -89.9 -41.18 -91.31 -38.17 -92.57C-35.15 -93.82 -32.05 -94.94 -28.93 -95.89C-25.8 -96.84 -22.61 -97.65 -19.41 -98.29C-16.2 -98.93 -12.95 -99.41 -9.7 -99.73C-6.45 -100.05 -3.16 -100.22 0.1 -100.22C3.37 -100.22 6.65 -100.05 9.9 -99.73C13.16 -99.41 16.41 -98.93 19.61 -98.29C22.82 -97.65 26 -96.84 29.13 -95.89C32.26 -94.94 35.35 -93.82 38.37 -92.57C41.39 -91.31 44.36 -89.9 47.24 -88.35C50.12 -86.8 52.94 -85.11 55.66 -83.28C58.38 -81.46 61.02 -79.49 63.54 -77.41C66.07 -75.33 68.5 -73.11 70.81 -70.79C73.12 -68.46 75.33 -66.02 77.4 -63.48C79.48 -60.94 81.43 -58.29 83.25 -55.56C85.06 -52.83 86.75 -50 88.29 -47.1C89.83 -44.21 91.24 -41.22 92.49 -38.19C93.74 -35.15 94.85 -32.05 95.8 -28.9C96.74 -25.76 97.54 -22.56 98.18 -19.34C98.82 -16.12 99.3 -12.85 99.62 -9.58C99.94 -6.32 100.1 -3.02 100.1 0.26Z",
        "pebble" to "M97.76 0.26C97.27 3.47 96.58 6.64 95.78 9.73C94.97 12.82 93.99 15.86 92.9 18.81C91.82 21.76 90.59 24.65 89.28 27.44C87.96 30.24 86.52 32.96 85.01 35.6C83.5 38.24 81.89 40.8 80.2 43.29C78.52 45.77 76.75 48.17 74.92 50.5C73.08 52.82 71.17 55.07 69.19 57.23C67.21 59.4 65.15 61.49 63.03 63.49C60.9 65.5 58.71 67.42 56.44 69.25C54.18 71.07 51.84 72.82 49.43 74.45C47.03 76.08 44.55 77.62 42 79.04C39.46 80.45 36.85 81.76 34.18 82.94C31.52 84.12 28.79 85.18 26.02 86.1C23.24 87.01 20.41 87.8 17.56 88.44C14.7 89.08 11.8 89.58 8.89 89.93C5.98 90.28 3.04 90.49 0.1 90.55C-2.83 90.61 -5.79 90.51 -8.72 90.28C-11.65 90.05 -14.59 89.68 -17.5 89.17C-20.4 88.66 -23.3 88 -26.15 87.23C-29.01 86.45 -31.84 85.54 -34.63 84.52C-37.42 83.49 -40.17 82.33 -42.88 81.07C-45.59 79.8 -48.26 78.42 -50.87 76.92C-53.49 75.43 -56.06 73.82 -58.57 72.1C-61.07 70.38 -63.53 68.54 -65.92 66.6C-68.3 64.66 -70.63 62.6 -72.86 60.43C-75.1 58.27 -77.27 55.98 -79.33 53.59C-81.38 51.2 -83.36 48.69 -85.2 46.08C-87.03 43.46 -88.78 40.73 -90.35 37.91C-91.92 35.09 -93.37 32.15 -94.63 29.14C-95.89 26.13 -97 23.01 -97.9 19.85C-98.79 16.69 -99.52 13.44 -100.01 10.17C-100.5 6.91 -100.79 3.57 -100.84 0.26C-100.9 -3.04 -100.73 -6.39 -100.33 -9.67C-99.93 -12.96 -99.3 -16.25 -98.45 -19.43C-97.6 -22.62 -96.51 -25.77 -95.23 -28.79C-93.95 -31.82 -92.44 -34.76 -90.78 -37.56C-89.11 -40.35 -87.23 -43.04 -85.23 -45.56C-83.22 -48.08 -81.04 -50.47 -78.77 -52.69C-76.5 -54.91 -74.08 -56.97 -71.62 -58.88C-69.16 -60.79 -66.58 -62.53 -63.99 -64.14C-61.4 -65.75 -58.74 -67.19 -56.08 -68.52C-53.42 -69.86 -50.72 -71.04 -48.04 -72.14C-45.36 -73.24 -42.67 -74.2 -40 -75.11C-37.32 -76.02 -34.65 -76.83 -31.99 -77.59C-29.33 -78.35 -26.69 -79.04 -24.03 -79.69C-21.38 -80.34 -18.74 -80.94 -16.08 -81.5C-13.42 -82.06 -10.76 -82.58 -8.07 -83.06C-5.37 -83.54 -2.66 -83.98 0.1 -84.36C2.86 -84.74 5.65 -85.08 8.49 -85.32C11.33 -85.57 14.22 -85.76 17.15 -85.83C20.07 -85.89 23.05 -85.88 26.06 -85.71C29.06 -85.54 32.12 -85.27 35.17 -84.81C38.23 -84.35 41.32 -83.75 44.37 -82.95C47.42 -82.15 50.49 -81.19 53.48 -80.01C56.47 -78.83 59.45 -77.46 62.3 -75.88C65.15 -74.31 67.95 -72.52 70.58 -70.55C73.22 -68.58 75.76 -66.4 78.11 -64.06C80.45 -61.72 82.66 -59.17 84.66 -56.5C86.65 -53.84 88.47 -50.98 90.06 -48.05C91.65 -45.11 93.04 -42.03 94.19 -38.9C95.35 -35.77 96.29 -32.52 97 -29.27C97.72 -26.02 98.2 -22.7 98.49 -19.4C98.77 -16.1 98.82 -12.77 98.7 -9.49C98.58 -6.22 98.24 -2.94 97.76 0.26Z",
        "squircle" to "M96.01 0.26C96.01 3.43 96.01 6.56 96.01 9.76C96.01 12.95 96.01 16.15 95.99 19.43C95.96 22.71 95.95 26.03 95.86 29.45C95.77 32.87 95.69 36.37 95.46 39.95C95.22 43.53 94.97 47.23 94.44 50.93C93.9 54.62 93.29 58.47 92.23 62.12C91.18 65.77 89.9 69.52 88.09 72.82C86.29 76.13 84.05 79.33 81.42 81.97C78.79 84.62 75.61 86.87 72.31 88.68C69.02 90.49 65.29 91.78 61.66 92.84C58.03 93.9 54.2 94.51 50.52 95.05C46.85 95.59 43.16 95.84 39.6 96.08C36.04 96.31 32.55 96.39 29.15 96.48C25.75 96.57 22.44 96.58 19.17 96.61C15.91 96.63 12.73 96.63 9.55 96.63C6.37 96.64 3.25 96.63 0.1 96.63C-3.05 96.63 -6.16 96.64 -9.34 96.63C-12.52 96.63 -15.7 96.63 -18.97 96.61C-22.24 96.58 -25.54 96.57 -28.94 96.48C-32.35 96.39 -35.83 96.31 -39.39 96.08C-42.96 95.84 -46.64 95.59 -50.32 95.05C-54 94.51 -57.83 93.9 -61.46 92.84C-65.09 91.78 -68.82 90.49 -72.11 88.68C-75.4 86.87 -78.59 84.62 -81.21 81.97C-83.84 79.33 -86.09 76.13 -87.89 72.82C-89.69 69.52 -90.97 65.77 -92.03 62.12C-93.08 58.47 -93.69 54.62 -94.23 50.93C-94.77 47.23 -95.01 43.53 -95.25 39.95C-95.49 36.37 -95.57 32.87 -95.65 29.45C-95.74 26.03 -95.75 22.71 -95.78 19.43C-95.81 16.15 -95.8 12.95 -95.8 9.76C-95.81 6.56 -95.81 3.43 -95.81 0.26C-95.81 -2.9 -95.81 -6.03 -95.8 -9.23C-95.8 -12.42 -95.81 -15.62 -95.78 -18.9C-95.75 -22.18 -95.74 -25.5 -95.65 -28.92C-95.57 -32.34 -95.49 -35.84 -95.25 -39.42C-95.01 -43 -94.77 -46.71 -94.23 -50.4C-93.69 -54.09 -93.08 -57.94 -92.03 -61.59C-90.97 -65.24 -89.69 -68.99 -87.89 -72.29C-86.09 -75.6 -83.84 -78.8 -81.21 -81.44C-78.59 -84.09 -75.4 -86.34 -72.11 -88.15C-68.82 -89.96 -65.09 -91.25 -61.46 -92.31C-57.83 -93.37 -54 -93.98 -50.32 -94.52C-46.64 -95.06 -42.96 -95.31 -39.39 -95.55C-35.83 -95.79 -32.35 -95.86 -28.94 -95.95C-25.54 -96.04 -22.24 -96.05 -18.97 -96.08C-15.7 -96.1 -12.52 -96.1 -9.34 -96.1C-6.16 -96.11 -3.05 -96.11 0.1 -96.11C3.25 -96.11 6.37 -96.11 9.55 -96.1C12.73 -96.1 15.91 -96.1 19.17 -96.08C22.44 -96.05 25.75 -96.04 29.15 -95.95C32.55 -95.86 36.04 -95.79 39.6 -95.55C43.16 -95.31 46.85 -95.06 50.52 -94.52C54.2 -93.98 58.03 -93.37 61.66 -92.31C65.29 -91.25 69.02 -89.96 72.31 -88.15C75.61 -86.34 78.79 -84.09 81.42 -81.44C84.05 -78.8 86.29 -75.6 88.09 -72.29C89.9 -68.99 91.18 -65.24 92.23 -61.59C93.29 -57.94 93.9 -54.09 94.44 -50.4C94.97 -46.71 95.22 -43 95.46 -39.42C95.69 -35.84 95.77 -32.34 95.86 -28.92C95.95 -25.5 95.96 -22.18 95.99 -18.9C96.01 -15.62 96.01 -12.42 96.01 -9.23C96.01 -6.03 96.01 -2.9 96.01 0.26Z",
        "capsule" to "M104.1 0.26C104.1 3.67 103.79 7.12 103.23 10.47C102.68 13.82 101.86 17.18 100.78 20.39C99.69 23.59 98.29 26.74 96.7 29.71C95.12 32.68 93.3 35.56 91.27 38.21C89.25 40.86 86.95 43.35 84.55 45.62C82.16 47.89 79.57 49.98 76.89 51.82C74.2 53.65 71.34 55.26 68.45 56.63C65.56 57.99 62.55 59.12 59.56 60.01C56.57 60.9 53.51 61.55 50.51 61.98C47.5 62.4 44.41 62.47 41.53 62.56C38.65 62.66 35.87 62.56 33.24 62.56C30.62 62.56 28.17 62.56 25.78 62.56C23.4 62.56 21.13 62.56 18.91 62.56C16.69 62.56 14.55 62.56 12.44 62.56C10.32 62.56 8.26 62.56 6.21 62.56C4.15 62.56 2.14 62.56 0.1 62.56C-1.93 62.56 -3.95 62.56 -6 62.56C-8.06 62.56 -10.11 62.56 -12.23 62.56C-14.35 62.56 -16.48 62.56 -18.71 62.56C-20.93 62.56 -23.19 62.56 -25.58 62.56C-27.97 62.56 -30.41 62.56 -33.04 62.56C-35.66 62.56 -38.45 62.66 -41.32 62.56C-44.2 62.47 -47.3 62.4 -50.3 61.98C-53.31 61.55 -56.37 60.9 -59.36 60.01C-62.35 59.12 -65.36 57.99 -68.25 56.63C-71.13 55.26 -74 53.65 -76.68 51.82C-79.37 49.98 -81.95 47.89 -84.35 45.62C-86.75 43.35 -89.04 40.86 -91.07 38.21C-93.09 35.56 -94.91 32.68 -96.5 29.71C-98.08 26.74 -99.48 23.59 -100.57 20.39C-101.66 17.18 -102.47 13.82 -103.03 10.47C-103.58 7.12 -103.9 3.67 -103.9 0.26C-103.9 -3.14 -103.58 -6.59 -103.03 -9.94C-102.47 -13.3 -101.66 -16.65 -100.57 -19.86C-99.48 -23.06 -98.08 -26.21 -96.5 -29.18C-94.91 -32.15 -93.09 -35.03 -91.07 -37.68C-89.04 -40.33 -86.75 -42.83 -84.35 -45.09C-81.95 -47.36 -79.37 -49.45 -76.68 -51.29C-74 -53.12 -71.13 -54.73 -68.25 -56.1C-65.36 -57.46 -62.35 -58.59 -59.36 -59.48C-56.37 -60.37 -53.31 -61.02 -50.3 -61.45C-47.3 -61.87 -44.2 -61.94 -41.32 -62.03C-38.45 -62.13 -35.66 -62.03 -33.04 -62.03C-30.41 -62.03 -27.97 -62.03 -25.58 -62.03C-23.19 -62.03 -20.93 -62.03 -18.71 -62.03C-16.48 -62.03 -14.35 -62.03 -12.23 -62.03C-10.11 -62.03 -8.06 -62.03 -6 -62.03C-3.95 -62.03 -1.93 -62.03 0.1 -62.03C2.14 -62.03 4.15 -62.03 6.21 -62.03C8.26 -62.03 10.32 -62.03 12.44 -62.03C14.55 -62.03 16.69 -62.03 18.91 -62.03C21.13 -62.03 23.4 -62.03 25.78 -62.03C28.17 -62.03 30.62 -62.03 33.24 -62.03C35.87 -62.03 38.65 -62.13 41.53 -62.03C44.41 -61.94 47.5 -61.87 50.51 -61.45C53.51 -61.02 56.57 -60.37 59.56 -59.48C62.55 -58.59 65.56 -57.46 68.45 -56.1C71.34 -54.73 74.2 -53.12 76.89 -51.29C79.57 -49.45 82.16 -47.36 84.55 -45.09C86.95 -42.83 89.25 -40.33 91.27 -37.68C93.3 -35.03 95.12 -32.15 96.7 -29.18C98.29 -26.21 99.69 -23.06 100.78 -19.86C101.86 -16.65 102.68 -13.3 103.23 -9.94C103.79 -6.59 104.1 -3.14 104.1 0.26Z",
        "triangle" to "M84.4 0.26C86 3.05 87.66 5.94 89.48 9.11C91.3 12.28 93.51 15.68 95.33 19.3C97.15 22.92 99.38 26.98 100.4 30.84C101.42 34.69 101.77 38.73 101.47 42.45C101.17 46.17 100.04 49.83 98.59 53.16C97.14 56.5 95.21 59.78 92.75 62.47C90.29 65.16 87.18 67.53 83.82 69.3C80.46 71.07 76.55 72.38 72.58 73.09C68.62 73.81 63.96 73.53 60.01 73.62C56.06 73.7 52.36 73.62 48.88 73.62C45.4 73.62 42.21 73.62 39.12 73.62C36.03 73.62 33.15 73.62 30.34 73.62C27.53 73.62 24.87 73.62 22.25 73.62C19.63 73.62 17.12 73.62 14.62 73.62C12.13 73.62 9.71 73.62 7.29 73.62C4.87 73.62 2.5 73.62 0.1 73.62C-2.29 73.62 -4.67 73.62 -7.09 73.62C-9.51 73.62 -11.93 73.62 -14.42 73.62C-16.91 73.62 -19.42 73.62 -22.04 73.62C-24.66 73.62 -27.32 73.62 -30.14 73.62C-32.95 73.62 -35.83 73.62 -38.92 73.62C-42.01 73.62 -45.19 73.62 -48.67 73.62C-52.16 73.62 -55.86 73.7 -59.81 73.62C-63.76 73.53 -68.41 73.81 -72.38 73.09C-76.35 72.38 -80.26 71.07 -83.62 69.3C-86.98 67.53 -90.08 65.16 -92.54 62.47C-95 59.78 -96.93 56.5 -98.39 53.16C-99.84 49.83 -100.96 46.17 -101.26 42.45C-101.56 38.73 -101.22 34.69 -100.19 30.84C-99.17 26.98 -96.95 22.92 -95.13 19.3C-93.31 15.68 -91.1 12.28 -89.27 9.11C-87.45 5.94 -85.79 3.05 -84.19 0.26C-82.59 -2.53 -81.1 -5.11 -79.66 -7.63C-78.21 -10.15 -76.85 -12.52 -75.51 -14.85C-74.17 -17.18 -72.89 -19.4 -71.63 -21.6C-70.36 -23.8 -69.14 -25.93 -67.92 -28.05C-66.7 -30.17 -65.51 -32.24 -64.31 -34.33C-63.11 -36.42 -61.93 -38.48 -60.73 -40.57C-59.52 -42.67 -58.32 -44.76 -57.09 -46.9C-55.86 -49.04 -54.62 -51.2 -53.34 -53.43C-52.05 -55.67 -50.75 -57.94 -49.38 -60.32C-48.01 -62.7 -46.61 -65.14 -45.12 -67.74C-43.62 -70.34 -42.09 -73.01 -40.42 -75.91C-38.75 -78.82 -37.03 -81.84 -35.11 -85.15C-33.19 -88.47 -31.3 -92.5 -28.9 -95.8C-26.5 -99.11 -23.76 -102.57 -20.73 -104.98C-17.7 -107.4 -14.21 -109.08 -10.73 -110.3C-7.26 -111.51 -3.51 -112.27 0.1 -112.27C3.71 -112.27 7.47 -111.51 10.94 -110.3C14.41 -109.08 17.91 -107.4 20.94 -104.98C23.96 -102.57 26.71 -99.11 29.11 -95.8C31.5 -92.5 33.39 -88.47 35.31 -85.15C37.23 -81.84 38.96 -78.82 40.63 -75.91C42.29 -73.01 43.83 -70.34 45.32 -67.74C46.82 -65.14 48.21 -62.7 49.58 -60.32C50.95 -57.94 52.26 -55.67 53.54 -53.43C54.83 -51.2 56.06 -49.04 57.3 -46.9C58.53 -44.76 59.73 -42.67 60.93 -40.57C62.13 -38.48 63.32 -36.42 64.52 -34.33C65.72 -32.24 66.91 -30.17 68.13 -28.05C69.35 -25.93 70.57 -23.8 71.83 -21.6C73.1 -19.4 74.37 -17.18 75.71 -14.85C77.05 -12.52 78.41 -10.15 79.86 -7.63C81.31 -5.11 82.79 -2.53 84.4 0.26Z",
        "hexagon" to "M104.1 0.26C104.1 3.63 103.24 7.13 102.06 10.35C100.88 13.58 98.67 16.64 97 19.63C95.32 22.62 93.65 25.46 92.03 28.28C90.41 31.1 88.84 33.83 87.28 36.55C85.71 39.27 84.19 41.92 82.65 44.6C81.11 47.28 79.6 49.92 78.05 52.6C76.51 55.28 74.97 57.96 73.4 60.7C71.82 63.45 70.23 66.21 68.59 69.08C66.94 71.94 65.36 74.99 63.51 77.9C61.66 80.82 59.9 84.17 57.49 86.56C55.08 88.95 52.14 90.97 49.04 92.25C45.93 93.54 42.28 93.93 38.85 94.26C35.43 94.6 31.84 94.26 28.48 94.26C25.12 94.26 21.9 94.26 18.71 94.26C15.52 94.26 12.42 94.26 9.32 94.26C6.21 94.26 3.17 94.26 0.1 94.26C-2.97 94.26 -6.01 94.26 -9.11 94.26C-12.21 94.26 -15.31 94.26 -18.51 94.26C-21.7 94.26 -24.92 94.26 -28.28 94.26C-31.63 94.26 -35.22 94.6 -38.65 94.26C-42.07 93.93 -45.73 93.54 -48.83 92.25C-51.94 90.97 -54.87 88.95 -57.28 86.56C-59.7 84.17 -61.46 80.82 -63.31 77.9C-65.16 74.99 -66.73 71.94 -68.38 69.08C-70.03 66.21 -71.61 63.45 -73.19 60.7C-74.77 57.96 -76.31 55.28 -77.85 52.6C-79.39 49.92 -80.91 47.28 -82.45 44.6C-83.98 41.92 -85.51 39.27 -87.07 36.55C-88.63 33.83 -90.2 31.1 -91.82 28.28C-93.44 25.46 -95.12 22.62 -96.79 19.63C-98.46 16.64 -100.67 13.58 -101.85 10.35C-103.04 7.13 -103.9 3.63 -103.9 0.26C-103.9 -3.1 -103.04 -6.6 -101.85 -9.83C-100.67 -13.05 -98.46 -16.11 -96.79 -19.1C-95.12 -22.09 -93.44 -24.93 -91.82 -27.75C-90.2 -30.57 -88.63 -33.3 -87.07 -36.02C-85.51 -38.74 -83.98 -41.39 -82.45 -44.07C-80.91 -46.75 -79.39 -49.39 -77.85 -52.07C-76.31 -54.76 -74.77 -57.43 -73.19 -60.18C-71.61 -62.92 -70.03 -65.68 -68.38 -68.55C-66.73 -71.41 -65.16 -74.46 -63.31 -77.37C-61.46 -80.29 -59.7 -83.64 -57.28 -86.03C-54.87 -88.42 -51.94 -90.44 -48.83 -91.72C-45.73 -93.01 -42.07 -93.4 -38.65 -93.74C-35.22 -94.07 -31.63 -93.74 -28.28 -93.74C-24.92 -93.74 -21.7 -93.74 -18.51 -93.74C-15.31 -93.74 -12.21 -93.74 -9.11 -93.74C-6.01 -93.74 -2.97 -93.74 0.1 -93.74C3.17 -93.74 6.21 -93.74 9.32 -93.74C12.42 -93.74 15.52 -93.74 18.71 -93.74C21.9 -93.74 25.12 -93.74 28.48 -93.74C31.84 -93.74 35.43 -94.07 38.85 -93.74C42.28 -93.4 45.93 -93.01 49.04 -91.72C52.14 -90.44 55.08 -88.42 57.49 -86.03C59.9 -83.64 61.66 -80.29 63.51 -77.37C65.36 -74.46 66.94 -71.41 68.59 -68.55C70.23 -65.68 71.82 -62.92 73.4 -60.18C74.97 -57.43 76.51 -54.76 78.05 -52.07C79.6 -49.39 81.11 -46.75 82.65 -44.07C84.19 -41.39 85.71 -38.74 87.28 -36.02C88.84 -33.3 90.41 -30.57 92.03 -27.75C93.65 -24.93 95.32 -22.09 97 -19.1C98.67 -16.11 100.88 -13.05 102.06 -9.83C103.24 -6.6 104.1 -3.1 104.1 0.26Z",
        "cloud" to "M91.68 0.26C92.98 3.26 94 6.44 94.69 9.63C95.39 12.81 95.77 16.13 95.83 19.4C95.89 22.66 95.62 25.99 95.05 29.2C94.47 32.42 93.56 35.63 92.37 38.67C91.18 41.7 89.66 44.68 87.91 47.42C86.15 50.17 84.08 52.79 81.82 55.13C79.57 57.48 77.03 59.64 74.36 61.5C71.69 63.36 68.78 64.98 65.8 66.28C62.82 67.58 59.65 68.6 56.48 69.29C53.3 69.97 49.39 69.22 46.74 70.4C44.09 71.58 42.78 74.52 40.58 76.36C38.39 78.21 36.03 79.93 33.57 81.46C31.12 82.98 28.51 84.36 25.84 85.51C23.17 86.67 20.37 87.65 17.55 88.41C14.73 89.17 11.81 89.73 8.9 90.06C6 90.39 3.03 90.51 0.1 90.42C-2.82 90.32 -5.77 90 -8.64 89.47C-11.51 88.95 -14.37 88.2 -17.12 87.27C-19.87 86.33 -22.57 85.18 -25.14 83.86C-27.7 82.55 -30.16 80.98 -32.51 79.37C-34.86 77.77 -36.49 75.09 -39.24 74.23C-42 73.36 -45.79 74.48 -49.04 74.16C-52.28 73.84 -55.57 73.21 -58.73 72.29C-61.88 71.37 -65.01 70.14 -67.96 68.65C-70.91 67.17 -73.77 65.37 -76.41 63.35C-79.04 61.34 -81.53 59.04 -83.75 56.56C-85.97 54.09 -88 51.36 -89.73 48.51C-91.46 45.67 -92.96 42.61 -94.13 39.49C-95.31 36.37 -96.21 33.08 -96.79 29.8C-97.38 26.52 -97.66 23.13 -97.63 19.8C-97.6 16.47 -97.25 13.09 -96.61 9.84C-95.97 6.58 -95.02 3.33 -93.8 0.26C-92.58 -2.81 -91.06 -5.8 -89.31 -8.58C-87.57 -11.36 -85.53 -14.01 -83.33 -16.41C-81.13 -18.81 -78.05 -20.76 -76.11 -22.97C-74.17 -25.17 -72.55 -27.13 -71.7 -29.62C-70.85 -32.11 -71.48 -35.19 -71.01 -37.93C-70.54 -40.67 -69.83 -43.42 -68.88 -46.05C-67.94 -48.68 -66.75 -51.28 -65.36 -53.72C-63.97 -56.15 -62.34 -58.51 -60.54 -60.67C-58.74 -62.83 -56.72 -64.86 -54.57 -66.67C-52.42 -68.48 -50.07 -70.13 -47.64 -71.53C-45.21 -72.93 -42.61 -74.13 -39.98 -75.08C-37.34 -76.03 -34.59 -76.75 -31.84 -77.22C-29.09 -77.69 -26.26 -77.91 -23.49 -77.9C-20.72 -77.88 -17.92 -77.61 -15.21 -77.11C-12.51 -76.62 -9.82 -75.87 -7.27 -74.92C-4.71 -73.98 -2.22 -72.8 0.1 -71.45C2.43 -70.11 4.55 -68.04 6.68 -66.86C8.81 -65.68 10.62 -64.39 12.9 -64.38C15.18 -64.37 17.8 -66.21 20.35 -66.79C22.89 -67.37 25.55 -67.73 28.18 -67.84C30.81 -67.95 33.5 -67.82 36.12 -67.45C38.74 -67.08 41.38 -66.46 43.91 -65.61C46.43 -64.76 48.92 -63.67 51.26 -62.37C53.59 -61.07 55.85 -59.53 57.91 -57.82C59.97 -56.12 61.92 -54.18 63.63 -52.13C65.35 -50.07 66.91 -47.81 68.23 -45.47C69.54 -43.13 70.66 -40.63 71.52 -38.1C72.39 -35.56 73.03 -32.9 73.43 -30.25C73.82 -27.61 72.69 -24.63 73.87 -22.22C75.06 -19.81 78.37 -18.13 80.54 -15.81C82.71 -13.5 85.04 -11 86.9 -8.32C88.75 -5.65 90.38 -2.73 91.68 0.26Z",
        "droplet" to "M61.61 0.26C62.55 2.29 63.45 4.41 64.22 6.61C64.98 8.81 65.64 11.11 66.18 13.47C66.72 15.84 67.17 18.29 67.48 20.8C67.78 23.31 67.99 25.91 68.02 28.53C68.05 31.15 67.94 33.85 67.64 36.54C67.34 39.23 66.87 41.95 66.24 44.67C65.61 47.39 64.85 50.16 63.87 52.85C62.88 55.53 61.67 58.17 60.31 60.77C58.96 63.36 57.46 65.96 55.76 68.41C54.05 70.85 52.12 73.19 50.08 75.42C48.03 77.65 45.83 79.84 43.47 81.8C41.12 83.76 38.57 85.55 35.93 87.19C33.3 88.82 30.53 90.33 27.68 91.61C24.83 92.88 21.84 93.97 18.82 94.83C15.8 95.7 12.68 96.35 9.56 96.8C6.44 97.24 3.26 97.52 0.1 97.52C-3.05 97.52 -6.24 97.24 -9.36 96.8C-12.48 96.35 -15.6 95.7 -18.62 94.83C-21.64 93.97 -24.62 92.88 -27.47 91.61C-30.32 90.33 -33.1 88.82 -35.73 87.19C-38.36 85.55 -40.91 83.76 -43.27 81.8C-45.63 79.84 -47.83 77.65 -49.87 75.42C-51.92 73.19 -53.85 70.85 -55.55 68.41C-57.26 65.96 -58.76 63.36 -60.11 60.77C-61.46 58.17 -62.67 55.53 -63.66 52.85C-64.65 50.16 -65.41 47.39 -66.03 44.67C-66.66 41.95 -67.14 39.23 -67.43 36.54C-67.73 33.85 -67.84 31.15 -67.81 28.53C-67.79 25.91 -67.58 23.31 -67.27 20.8C-66.97 18.29 -66.52 15.84 -65.97 13.47C-65.43 11.11 -64.77 8.81 -64.01 6.61C-63.25 4.41 -62.34 2.29 -61.41 0.26C-60.47 -1.76 -59.41 -3.68 -58.41 -5.53C-57.41 -7.38 -56.39 -9.12 -55.42 -10.83C-54.46 -12.55 -53.54 -14.19 -52.63 -15.81C-51.72 -17.43 -50.83 -19 -49.95 -20.57C-49.07 -22.14 -48.21 -23.67 -47.34 -25.21C-46.47 -26.76 -45.61 -28.29 -44.74 -29.84C-43.86 -31.39 -42.99 -32.94 -42.1 -34.53C-41.2 -36.13 -40.3 -37.73 -39.36 -39.39C-38.43 -41.06 -37.48 -42.75 -36.48 -44.52C-35.48 -46.3 -34.46 -48.12 -33.36 -50.06C-32.27 -52.01 -31.15 -54.01 -29.92 -56.18C-28.7 -58.36 -27.43 -60.61 -26.02 -63.12C-24.62 -65.62 -23.15 -68.23 -21.47 -71.21C-19.8 -74.18 -18.05 -77.3 -15.98 -80.98C-13.91 -84.66 -11.75 -89.4 -9.07 -93.27C-6.39 -97.15 -2.95 -104.24 0.1 -104.24C3.16 -104.24 6.59 -97.15 9.27 -93.27C11.95 -89.4 14.12 -84.66 16.19 -80.98C18.25 -77.3 20.01 -74.18 21.68 -71.21C23.35 -68.23 24.82 -65.62 26.23 -63.12C27.64 -60.61 28.91 -58.36 30.13 -56.18C31.35 -54.01 32.48 -52.01 33.57 -50.06C34.66 -48.12 35.68 -46.3 36.68 -44.52C37.68 -42.75 38.63 -41.06 39.57 -39.39C40.51 -37.73 41.41 -36.13 42.3 -34.53C43.2 -32.94 44.07 -31.39 44.94 -29.84C45.82 -28.29 46.67 -26.76 47.54 -25.21C48.41 -23.67 49.27 -22.14 50.16 -20.57C51.04 -19 51.92 -17.43 52.83 -15.81C53.74 -14.19 54.67 -12.55 55.63 -10.83C56.59 -9.12 57.62 -7.38 58.61 -5.53C59.61 -3.68 60.68 -1.76 61.61 0.26Z",
    )

    fun parseBezier64Coords(d: String): FloatArray {
        val list = NUM_REGEX.findAll(d).map { it.value.toFloat() }.toList()
        return list.toFloatArray()
    }

    val SHAPE_COORDS: Map<String, FloatArray> by lazy { SHAPE_PATHS.mapValues { parseBezier64Coords(it.value) } }

    private fun buildPolar64Coords(pointAt: (Double) -> Pair<Float, Float>): FloatArray {
        val n = 64
        val ptsX = FloatArray(n)
        val ptsY = FloatArray(n)
        for (i in 0 until n) {
            val theta = (i.toDouble() / n.toDouble()) * PI * 2.0
            val (px, py) = pointAt(theta)
            ptsX[i] = px
            ptsY[i] = py
        }
        val k = (4f / 3f) * tan((PI / (2.0 * n)).toFloat()) * n * 0.5f
        val out = FloatArray(2 + n * 6)
        out[0] = ptsX[0]
        out[1] = ptsY[0]
        var idx = 2
        for (i in 0 until n) {
            val pPrev = (i - 1 + n) % n
            val pCurr = i
            val pNext = (i + 1) % n
            val pNext2 = (i + 2) % n
            val tx0 = (ptsX[pNext] - ptsX[pPrev]) * 0.5f
            val ty0 = (ptsY[pNext] - ptsY[pPrev]) * 0.5f
            val tx1 = (ptsX[pNext2] - ptsX[pCurr]) * 0.5f
            val ty1 = (ptsY[pNext2] - ptsY[pCurr]) * 0.5f
            val scale = k / n
            out[idx++] = ptsX[pCurr] + tx0 * scale * 2f
            out[idx++] = ptsY[pCurr] + ty0 * scale * 2f
            out[idx++] = ptsX[pNext] - tx1 * scale * 2f
            out[idx++] = ptsY[pNext] - ty1 * scale * 2f
            out[idx++] = ptsX[pNext]
            out[idx++] = ptsY[pNext]
        }
        return out
    }

    val EGG_COORDS: FloatArray by lazy {
        buildPolar64Coords { theta ->
            val y = sin(theta).toFloat()
            val rx = 82f * (1f + 0.14f * y)
            Pair(rx * cos(theta).toFloat(), 102f * y + 2f)
        }
    }

    val PLAY_COORDS: FloatArray by lazy {
        buildPolar64Coords { theta ->
            val corners = arrayOf(82f to 0f, -56f to 78f, -56f to -78f)
            var rx = 0f
            var ry = 0f
            var wSum = 0f
            val ux = cos(theta).toFloat()
            val uy = sin(theta).toFloat()
            for ((cx, cy) in corners) {
                val len = hypot(cx, cy)
                val dot = (ux * cx + uy * cy) / len
                val w = exp((dot * 3.1f).toDouble()).toFloat()
                rx += cx * w
                ry += cy * w
                wSum += w
            }
            Pair((rx / wSum) * 1.16f + 6f, (ry / wSum) * 1.16f)
        }
    }

    val FACE_DEFS: Map<String, GrokFaceDef> = mapOf(
        "neutral" to GrokFaceDef(pill(9.3f, 11.3f), pill(9.3f, 11.3f), -26f, -8f, 26f, -14f, -11f, -10f),
        "attentive" to GrokFaceDef(pill(10.5f, 11.5f), pill(10.5f, 11.5f), -26f, -10f, 28f, -12f, -5f, -4f),
        "surprised" to GrokFaceDef(pill(22.5f, 1f), pill(22.5f, 1f), -31f, 0f, 33f, 0f, 0f, 0f),
        "excited" to GrokFaceDef(pill(20f, 8f), pill(20f, 8f), -29f, 12f, 33f, 12f, -8f, 8f),
        "happy" to GrokFaceDef(hpill(13.5f, 8.5f), hpill(13.5f, 8.5f), -27f, -16f, 30f, -16f, 11f, -12f),
        "laughing" to GrokFaceDef(hpill(17f, 6.5f), hpill(17f, 6.5f), -29f, -22f, 31f, -22f, 16f, -17f),
        "angry" to GrokFaceDef(hpill(17f, 7.5f), hpill(17f, 7.5f), -28f, -12f, 29f, -12f, 28f, -28f),
        "sad" to GrokFaceDef(pill(11f, 9f), pill(11f, 9f), -27f, 16f, 28f, 16f, -26f, 26f),
        "scared" to GrokFaceDef(pill(20f, 10f), pill(20f, 10f), -32f, 20f, 34f, 20f, 4f, -4f),
        "suspicious" to GrokFaceDef(pill(10.5f, 9.5f), hpill(11f, 7.5f), -16f, -8f, 36f, -13f, -8f, -6f, gazeBiasX = 0.25f),
        "confused" to GrokFaceDef(pill(10f, 12f), hpill(14f, 8.5f), -34f, -10f, 21f, -3f, -11f, 22f),
        "curious" to GrokFaceDef(pill(12f, 11f), pill(10f, 9f), -16f, 12f, 36f, 0f, -20f, -22f),
        "proud" to GrokFaceDef(hpill(15f, 7.5f), hpill(15f, 7.5f), -27f, -26f, 30f, -26f, 13f, -14f),
        "shy" to GrokFaceDef(pill(8.5f, 6.5f), pill(8.5f, 6.5f), -38f, 20f, 8f, 15f, -8f, -10f, gazeBiasX = -0.35f, gazeBiasY = 0.25f),
        "unimpressed" to GrokFaceDef(hpill(15f, 6f), hpill(15f, 6f), -36f, -6f, 16f, -6f, -1f, 0f),
        "sleepy" to GrokFaceDef(pill(10f, 11f), pill(10f, 11f), -24f, 12f, 30f, 9f, -2f, -3f, lSy = 0.45f, rSy = 0.45f),
    )

    val WINK_LEFT_EYE = pill(11.8f, 11.4f)
    val WINK_RIGHT_BAR = hpill(22.35f, 4.45f)
    val WIDE_EYE = pill(17.8f, 25.95f)
    val NOTIF_EYE = pill(24.9f, 0.35f)
    val PLAY_EYE = pill(9f, 8f)
    val EGG_EYE = pill(8.2f, 11.05f)
    val HEX_EYE = pill(8.85f, 11.7f)

    fun buildEyeTransform(
        tx: Float,
        ty: Float,
        deg: Float,
        sy: Float = 1f,
        sxExtra: Float = 1f,
    ): GrokEyeTransform {
        val foreshorten = (1f - abs(tx / 98f) * 0.2f).coerceIn(0.76f, 1f) * sxExtra
        val rad = (deg * PI / 180.0).toFloat()
        val c = cos(rad)
        val s = sin(rad)
        return GrokEyeTransform(
            a = c * foreshorten,
            b = s * foreshorten,
            c = -s * sy,
            d = c * sy,
            tx = tx,
            ty = ty,
        )
    }

    private data class ShapeEyeLayout(val dx: Float, val dy: Float, val s: Float)

    private val SHAPE_EYE_LAYOUT: Map<String, ShapeEyeLayout> = mapOf(
        "circle" to ShapeEyeLayout(0f, 0f, 1f),
        "pebble" to ShapeEyeLayout(-1.5f, 1.7f, 1f),
        "squircle" to ShapeEyeLayout(-0.4f, -0.3f, 0.98f),
        "capsule" to ShapeEyeLayout(-4.9f, 7.4f, 1.02f),
        "triangle" to ShapeEyeLayout(-3.7f, 0.9f, 0.88f),
        "hexagon" to ShapeEyeLayout(-0.4f, -0.4f, 0.98f),
        "cloud" to ShapeEyeLayout(-4.7f, 3.2f, 0.9f),
        "droplet" to ShapeEyeLayout(-3.7f, 3f, 0.62f),
    )

    fun buildEyeTransform(pose: GrokEyePose): GrokEyeTransform =
        buildEyeTransform(pose.x, pose.y, pose.deg, pose.sy)

    /** The face's eye cutouts and where they sit for a gaze offset (-1..1) and silhouette shape. */
    fun computeFaceEyePoses(
        face: String,
        gazeX: Float = 0f,
        gazeY: Float = 0f,
        blinkScaleY: Float = 1f,
        shape: String = "circle",
    ): Pair<GrokEyePose, GrokEyePose> {
        val def = FACE_DEFS[face] ?: FACE_DEFS.getValue("neutral")
        val lay = SHAPE_EYE_LAYOUT[shape] ?: SHAPE_EYE_LAYOUT.getValue("circle")
        val gx = (gazeX + def.gazeBiasX).coerceIn(-1f, 1f)
        val gy = (gazeY + def.gazeBiasY).coerceIn(-1f, 1f)
        val shiftX = gx * 18f
        val shiftY = gy * 14f
        val tilt = gx * 6f
        return Pair(
            GrokEyePose(def.left, lay.dx + def.lx * lay.s + shiftX, lay.dy + def.ly * lay.s + shiftY, def.lDeg + tilt, def.lSy * blinkScaleY),
            GrokEyePose(def.right, lay.dx + def.rx * lay.s + shiftX, lay.dy + def.ry * lay.s + shiftY, def.rDeg + tilt, def.rSy * blinkScaleY),
        )
    }

    fun computeFaceEyeTransforms(
        face: String,
        gazeX: Float = 0f,
        gazeY: Float = 0f,
        blinkScaleY: Float = 1f,
        shape: String = "circle",
    ): Pair<GrokEyeTransform, GrokEyeTransform> {
        val (l, r) = computeFaceEyePoses(face, gazeX, gazeY, blinkScaleY, shape)
        return Pair(buildEyeTransform(l), buildEyeTransform(r))
    }

    fun lerpCoords(
        a: FloatArray,
        b: FloatArray,
        t: Float,
        out: FloatArray,
        scale: Float = 1f,
        rotDeg: Float = 0f,
        tx: Float = 0f,
        ty: Float = 0f,
    ) {
        val rad = (rotDeg * PI / 180.0).toFloat()
        val c = cos(rad) * scale
        val s = sin(rad) * scale
        var i = 0
        while (i + 1 < a.size) {
            val x = a[i] + (b[i] - a[i]) * t
            val y = a[i + 1] + (b[i + 1] - a[i + 1]) * t
            out[i] = x * c - y * s + tx
            out[i + 1] = x * s + y * c + ty
            i += 2
        }
    }
}
