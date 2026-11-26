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
import kotlin.math.pow
import kotlin.math.sign
import kotlin.math.sin
import kotlin.math.tan
import kotlin.random.Random

fun buildStandaloneGrokBotSvg(spec: GrokBotSpec, size: Int = 512): String {
    val palette = GrokBotGeometry.COLOR_PALETTES[spec.color] ?: GrokBotGeometry.COLOR_PALETTES.getValue("blue")
    val bodyPath = GrokBotGeometry.SHAPE_PATHS[spec.shape] ?: GrokBotGeometry.SHAPE_PATHS.getValue("circle")
    val faceDef = GrokBotGeometry.FACE_DEFS[spec.face] ?: GrokBotGeometry.FACE_DEFS.getValue("neutral")
    val (leftM, rightM) = GrokBotGeometry.computeFaceEyeTransforms(spec.face, 0f, 0f, 1f, spec.shape)
    return listOf(
        """<svg xmlns="http://www.w3.org/2000/svg" width="$size" height="$size" viewBox="-125 -125 250 250">""",
        "  <defs>",
        """    <mask id="grokbot-mask" maskUnits="userSpaceOnUse" x="-125" y="-125" width="250" height="250">""",
        """      <path d="$bodyPath" fill="#fff"/>""",
        """      <path d="${faceDef.left.svgPath}" fill="#000" transform="${leftM.toSvgMatrix()}"/>""",
        """      <path d="${faceDef.right.svgPath}" fill="#000" transform="${rightM.toSvgMatrix()}"/>""",
        "    </mask>",
        """    <radialGradient id="grokbot-fill" gradientUnits="userSpaceOnUse" cx="-40" cy="-50" r="193.75">""",
        """      <stop offset="0%" stop-color="${palette.stopHexes[0]}"/>""",
        """      <stop offset="32%" stop-color="${palette.stopHexes[1]}"/>""",
        """      <stop offset="72%" stop-color="${palette.stopHexes[2]}"/>""",
        """      <stop offset="100%" stop-color="${palette.stopHexes[3]}"/>""",
        "    </radialGradient>",
        "  </defs>",
        """  <g mask="url(#grokbot-mask)">""",
        """    <rect x="-125" y="-125" width="250" height="250" fill="url(#grokbot-fill)"/>""",
        "  </g>",
        "</svg>",
    ).joinToString("\n")
}

internal fun fillBezierPath(coords: FloatArray, outPath: AndroidPath, scale: Float, cx: Float, cy: Float) {
    outPath.reset()
    if (coords.size < 8) return
    outPath.moveTo(cx + coords[0] * scale, cy + coords[1] * scale)
    var i = 2
    while (i + 5 < coords.size) {
        outPath.cubicTo(
            cx + coords[i] * scale,
            cy + coords[i + 1] * scale,
            cx + coords[i + 2] * scale,
            cy + coords[i + 3] * scale,
            cx + coords[i + 4] * scale,
            cy + coords[i + 5] * scale,
        )
        i += 6
    }
    outPath.close()
}

internal fun fillEyeAndroidPath(
    pill: GrokEyePill,
    tr: GrokEyeTransform,
    scale: Float,
    cx: Float,
    cy: Float,
    outPath: AndroidPath,
    scratchRect: RectF,
    scratchValues: FloatArray,
    scratchMatrix: AndroidMatrix,
) {
    outPath.rewind()
    scratchRect.set(-pill.halfW, -pill.halfH, pill.halfW, pill.halfH)
    outPath.addRoundRect(scratchRect, pill.rx, pill.ry, AndroidPath.Direction.CW)
    scratchValues[0] = tr.a * scale
    scratchValues[1] = tr.c * scale
    scratchValues[2] = cx + tr.tx * scale
    scratchValues[3] = tr.b * scale
    scratchValues[4] = tr.d * scale
    scratchValues[5] = cy + tr.ty * scale
    scratchValues[6] = 0f
    scratchValues[7] = 0f
    scratchValues[8] = 1f
    scratchMatrix.setValues(scratchValues)
    outPath.transform(scratchMatrix)
}

fun renderGrokBotPngBitmap(spec: GrokBotSpec, size: Int = 512): Bitmap {
    val bmp = androidx.core.graphics.createBitmap(size, size)
    val canvas = AndroidCanvas(bmp)
    val scale = size / 250f
    val cx = size * 0.5f
    val cy = size * 0.5f

    val coords = GrokBotGeometry.SHAPE_COORDS[spec.shape] ?: GrokBotGeometry.SHAPE_COORDS.getValue("circle")
    val faceDef = GrokBotGeometry.FACE_DEFS[spec.face] ?: GrokBotGeometry.FACE_DEFS.getValue("neutral")
    val (leftTr, rightTr) = GrokBotGeometry.computeFaceEyeTransforms(spec.face, 0f, 0f, 1f, spec.shape)
    val palette = GrokBotGeometry.COLOR_PALETTES[spec.color] ?: GrokBotGeometry.COLOR_PALETTES.getValue("blue")

    val bodyPath = AndroidPath()
    fillBezierPath(coords, bodyPath, scale, cx, cy)
    val mat = AndroidMatrix()
    val rect = RectF()
    val values = FloatArray(9)
    val leftEye = AndroidPath().also { fillEyeAndroidPath(faceDef.left, leftTr, scale, cx, cy, it, rect, values, mat) }
    val rightEye = AndroidPath().also { fillEyeAndroidPath(faceDef.right, rightTr, scale, cx, cy, it, rect, values, mat) }
    bodyPath.op(leftEye, AndroidPath.Op.DIFFERENCE)
    bodyPath.op(rightEye, AndroidPath.Op.DIFFERENCE)

    val paint = AndroidPaint(AndroidPaint.ANTI_ALIAS_FLAG).apply {
        style = AndroidPaint.Style.FILL
        shader = AndroidRadialGradient(
            cx - 40f * scale,
            cy - 50f * scale,
            193.75f * scale,
            intArrayOf(
                Color(palette.stopColors[0]).toArgb(),
                Color(palette.stopColors[1]).toArgb(),
                Color(palette.stopColors[2]).toArgb(),
                Color(palette.stopColors[3]).toArgb(),
            ),
            floatArrayOf(0f, 0.32f, 0.72f, 1f),
            Shader.TileMode.CLAMP,
        )
    }
    canvas.drawPath(bodyPath, paint)
    return bmp
}
