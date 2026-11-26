package org.pjstatecm.graceride.ui.components


import android.graphics.Bitmap
import android.graphics.Canvas as AndroidCanvas
import android.graphics.Matrix as AndroidMatrix
import android.graphics.Paint as AndroidPaint
import android.graphics.Path as AndroidPath
import android.graphics.RadialGradient as AndroidRadialGradient
import android.graphics.RectF
import android.graphics.Shader
import androidx.compose.foundation.BorderStroke
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

@Composable
fun grokShapeLabelRes(shape: String): Int = when (shape) {
    "pebble" -> R.string.studio_shape_pebble
    "squircle" -> R.string.studio_shape_squircle
    "capsule" -> R.string.studio_shape_capsule
    "triangle" -> R.string.studio_shape_triangle
    "hexagon" -> R.string.studio_shape_hexagon
    "cloud" -> R.string.studio_shape_cloud
    "droplet" -> R.string.studio_shape_droplet
    else -> R.string.studio_shape_circle
}

@Composable
fun grokFaceLabelRes(face: String): Int = when (face) {
    "attentive" -> R.string.studio_face_attentive
    "surprised" -> R.string.studio_face_surprised
    "excited" -> R.string.studio_face_excited
    "happy" -> R.string.studio_face_happy
    "laughing" -> R.string.studio_face_laughing
    "angry" -> R.string.studio_face_angry
    "sad" -> R.string.studio_face_sad
    "scared" -> R.string.studio_face_scared
    "suspicious" -> R.string.studio_face_suspicious
    "confused" -> R.string.studio_face_confused
    "curious" -> R.string.studio_face_curious
    "proud" -> R.string.studio_face_proud
    "shy" -> R.string.studio_face_shy
    "unimpressed" -> R.string.studio_face_unimpressed
    "sleepy" -> R.string.studio_face_sleepy
    else -> R.string.studio_face_neutral
}

@Composable
fun grokColorLabelRes(color: String): Int = when (color) {
    "ink" -> R.string.studio_color_ink
    "brown" -> R.string.studio_color_brown
    "red" -> R.string.studio_color_red
    "orange" -> R.string.studio_color_orange
    "amber" -> R.string.studio_color_amber
    "green" -> R.string.studio_color_green
    "turquoise" -> R.string.studio_color_turquoise
    "purple" -> R.string.studio_color_purple
    "pink" -> R.string.studio_color_pink
    "grey" -> R.string.studio_color_grey
    "cream" -> R.string.studio_color_cream
    else -> R.string.studio_color_blue
}

@Composable
fun grokMoveLabelRes(move: String): Int = when (move) {
    "thinking" -> R.string.studio_move_thinking
    "wink" -> R.string.studio_move_wink
    "wide_eyes" -> R.string.studio_move_wide_eyes
    "alert" -> R.string.studio_move_alert
    "notification" -> R.string.studio_move_notification
    "exclamation" -> R.string.studio_move_exclamation
    "sleep" -> R.string.studio_move_sleep
    "egg" -> R.string.studio_move_egg
    "hexagon" -> R.string.studio_move_hexagon
    "play" -> R.string.studio_move_play
    "orbit" -> R.string.studio_move_orbit
    "swirl" -> R.string.studio_move_swirl
    "burst" -> R.string.studio_move_burst
    "comet" -> R.string.studio_move_comet
    else -> R.string.studio_move_idle
}

@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
@Composable
fun GrokBotControlsPanel(
    spec: GrokBotSpec,
    onSpecChange: (GrokBotSpec) -> Unit,
    activeMove: String,
    onMoveChange: (String) -> Unit,
    tilePreviewSize: Dp = 44.dp,
    cols: Int = 4,
) {
    var tab by remember { androidx.compose.runtime.mutableIntStateOf(0) }
    val shapeRows = remember(cols) { GrokBotSpec.SHAPES.chunked(cols) }
    val faceRows = remember(cols) { GrokBotSpec.FACES.chunked(cols) }
    val colorRows = remember { GrokBotSpec.COLORS.chunked(6) }
    val moveRows = remember(cols) { GrokBotSpec.MOVES.chunked(cols) }
    val colorSwatchBrushes = remember {
        GrokBotSpec.COLORS.associateWith { co ->
            val pal = GrokBotGeometry.COLOR_PALETTES.getValue(co)
            Brush.radialGradient(
                0f to Color(pal.stopColors[0]),
                0.35f to Color(pal.stopColors[1]),
                0.75f to Color(pal.stopColors[2]),
                1f to Color(pal.stopColors[3]),
                center = Offset(24f, 20f),
                radius = 64f,
            )
        }
    }

    Column(verticalArrangement = Arrangement.spacedBy(Spacing.sm)) {
        SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
            SegmentedButton(
                selected = tab == 0,
                onClick = { tab = 0 },
                shape = SegmentedButtonDefaults.itemShape(index = 0, count = 2),
            ) {
                Text(stringResource(R.string.studio_look_tab))
            }
            SegmentedButton(
                selected = tab == 1,
                onClick = { tab = 1 },
                shape = SegmentedButtonDefaults.itemShape(index = 1, count = 2),
            ) {
                Text(stringResource(R.string.studio_move_tab))
            }
        }

        if (tab == 0) {
            // Shapes (8)
            SectionTitleRow(
                title = stringResource(R.string.studio_shape_label),
                current = stringResource(grokShapeLabelRes(spec.shape)),
            )
            shapeRows.forEach { rowItems ->
                Row(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    rowItems.forEach { sh ->
                        val selected = spec.shape == sh
                        GrokOptionTile(
                            selected = selected,
                            label = stringResource(grokShapeLabelRes(sh)),
                            onClick = { onSpecChange(spec.copy(shape = sh)) },
                            modifier = Modifier.weight(1f),
                        ) {
                            GrokBotCanvas(
                                spec = spec.copy(shape = sh),
                                variant = "tile",
                                modifier = Modifier.size(tilePreviewSize),
                            )
                        }
                    }
                    repeat(cols - rowItems.size) { Spacer(Modifier.weight(1f)) }
                }
            }

            // Faces (16)
            SectionTitleRow(
                title = stringResource(R.string.studio_face_label),
                current = stringResource(grokFaceLabelRes(spec.face)),
            )
            faceRows.forEach { rowItems ->
                Row(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    rowItems.forEach { fa ->
                        val selected = spec.face == fa
                        GrokOptionTile(
                            selected = selected,
                            label = stringResource(grokFaceLabelRes(fa)),
                            onClick = { onSpecChange(spec.copy(face = fa)) },
                            modifier = Modifier.weight(1f),
                        ) {
                            GrokBotCanvas(
                                spec = spec.copy(face = fa),
                                variant = "tile",
                                modifier = Modifier.size(tilePreviewSize),
                            )
                        }
                    }
                    repeat(cols - rowItems.size) { Spacer(Modifier.weight(1f)) }
                }
            }

            // Colours (12)
            SectionTitleRow(
                title = stringResource(R.string.studio_color_label),
                current = stringResource(grokColorLabelRes(spec.color)),
            )
            colorRows.forEach { rowColors ->
                Row(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    rowColors.forEach { co ->
                        val selected = spec.color == co
                        val lightInk = co == "cream" || co == "amber" || co == "grey"
                        val swatchBrush = colorSwatchBrushes.getValue(co)
                        Box(
                            modifier = Modifier
                                .size(38.dp)
                                .clip(CircleShape)
                                .background(swatchBrush)
                                .clickable(role = Role.RadioButton) { onSpecChange(spec.copy(color = co)) },
                            contentAlignment = Alignment.Center,
                        ) {
                            if (selected) {
                                Surface(
                                    color = Color.Transparent,
                                    shape = CircleShape,
                                    border = BorderStroke(2.5.dp, MaterialTheme.colorScheme.primary),
                                    modifier = Modifier.fillMaxSize(),
                                ) {}
                                Icon(
                                    Icons.Outlined.Check,
                                    contentDescription = stringResource(grokColorLabelRes(co)),
                                    tint = if (lightInk) Color(0xFF1A1A1A) else Color.White,
                                    modifier = Modifier.size(16.dp),
                                )
                            }
                        }
                    }
                }
            }
        } else {
            // Moves (15)
            SectionTitleRow(
                title = stringResource(R.string.studio_animations_label),
                current = stringResource(grokMoveLabelRes(activeMove)),
            )
            moveRows.forEach { rowMoves ->
                Row(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    rowMoves.forEach { mv ->
                        val selected = activeMove == mv
                        GrokOptionTile(
                            selected = selected,
                            label = stringResource(grokMoveLabelRes(mv)),
                            onClick = { onMoveChange(mv) },
                            modifier = Modifier.weight(1f),
                        ) {
                            GrokBotCanvas(
                                spec = spec,
                                move = mv,
                                variant = "tile",
                                staticMovePreview = true,
                                modifier = Modifier.size(tilePreviewSize),
                            )
                        }
                    }
                    repeat(cols - rowMoves.size) { Spacer(Modifier.weight(1f)) }
                }
            }
        }
    }
}

@Composable
private fun SectionTitleRow(title: String, current: String) {
    Row(
        Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(title, style = MaterialTheme.typography.labelLarge, fontWeight = FontWeight.SemiBold)
        Text(current, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
private fun GrokOptionTile(
    selected: Boolean,
    label: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    preview: @Composable () -> Unit,
) {
    OutlinedCard(
        onClick = onClick,
        shape = RoundedCornerShape(12.dp),
        border = if (selected) {
            BorderStroke(2.dp, MaterialTheme.colorScheme.primary)
        } else {
            BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)
        },
        modifier = modifier,
    ) {
        Column(
            Modifier.fillMaxWidth().padding(vertical = 6.dp, horizontal = 4.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            preview()
            Text(
                label,
                style = MaterialTheme.typography.labelSmall,
                textAlign = TextAlign.Center,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}
