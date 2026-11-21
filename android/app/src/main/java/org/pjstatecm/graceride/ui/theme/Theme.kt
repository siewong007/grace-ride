package org.pjstatecm.graceride.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

private val LightColors = lightColorScheme(
    primary = GrPrimary,
    onPrimary = Color.White,
    primaryContainer = GrPrimarySoft,
    onPrimaryContainer = GrPrimaryHover,
    secondary = GrInfo,
    onSecondary = Color.White,
    secondaryContainer = GrInfoSoft,
    onSecondaryContainer = GrInfo,
    background = GrBg,
    onBackground = GrText,
    surface = GrBg,
    onSurface = GrText,
    surfaceVariant = GrSurface2,
    onSurfaceVariant = GrMuted,
    outline = GrMuted,
    outlineVariant = GrBorder,
    error = GrDanger,
    onError = Color.White,
    errorContainer = GrDangerSoft,
    onErrorContainer = GrDanger,
)

private val DarkColors = darkColorScheme(
    primary = GrPrimaryDark,
    onPrimary = GrPrimaryInkDark,
    primaryContainer = GrInfoSoftDark,
    onPrimaryContainer = GrPrimaryHoverDark,
    secondary = GrInfoDark,
    onSecondary = GrPrimaryInkDark,
    secondaryContainer = GrInfoSoftDark,
    onSecondaryContainer = GrInfoDark,
    background = GrBgDark,
    onBackground = GrTextDark,
    surface = GrBgDark,
    onSurface = GrTextDark,
    surfaceVariant = GrSurface2Dark,
    onSurfaceVariant = GrMutedDark,
    outline = GrMutedDark,
    outlineVariant = GrBorderDark,
    error = GrDangerDark,
    onError = GrPrimaryInkDark,
    errorContainer = GrDangerSoftDark,
    onErrorContainer = GrDangerDark,
)

@Immutable
data class StatusColors(
    val success: Color, val successSoft: Color,
    val warning: Color, val warningSoft: Color,
    val danger: Color, val dangerSoft: Color,
    val info: Color, val infoSoft: Color,
)

val LocalStatusColors = staticCompositionLocalOf {
    StatusColors(GrSuccess, GrSuccessSoft, GrWarning, GrWarningSoft, GrDanger, GrDangerSoft, GrInfo, GrInfoSoft)
}

// One radius (8dp) like the web; pills use CircleShape directly.
private val GrShapes = Shapes(
    extraSmall = RoundedCornerShape(8.dp),
    small = RoundedCornerShape(8.dp),
    medium = RoundedCornerShape(8.dp),
    large = RoundedCornerShape(8.dp),
    extraLarge = RoundedCornerShape(8.dp),
)

object Spacing {
    val xxs = 4.dp
    val xs = 8.dp
    val sm = 16.dp
    val md = 24.dp
    val lg = 32.dp
    val xl = 48.dp
    val xxl = 64.dp
}

@Composable
fun GraceRideTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    val colors = if (darkTheme) DarkColors else LightColors
    val status = if (darkTheme) {
        StatusColors(GrSuccessDark, GrSuccessSoftDark, GrWarningDark, GrWarningSoftDark, GrDangerDark, GrDangerSoftDark, GrInfoDark, GrInfoSoftDark)
    } else {
        StatusColors(GrSuccess, GrSuccessSoft, GrWarning, GrWarningSoft, GrDanger, GrDangerSoft, GrInfo, GrInfoSoft)
    }
    CompositionLocalProvider(LocalStatusColors provides status) {
        MaterialTheme(
            colorScheme = colors,
            typography = GrTypography,
            shapes = GrShapes,
            content = content,
        )
    }
}
