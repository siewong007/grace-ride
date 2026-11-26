package org.pjstatecm.graceride.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ButtonColors
import androidx.compose.material3.ButtonDefaults
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
    surfaceBright = GrBg,
    surfaceDim = GrSurfaceHigh,
    surfaceContainerLowest = GrSurfaceLowest,
    surfaceContainerLow = GrSurfaceLow,
    surfaceContainer = GrSurfaceContainer,
    surfaceContainerHigh = GrSurfaceHigh,
    surfaceContainerHighest = GrSurfaceHighest,
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
    primaryContainer = GrPrimarySoftDark,
    onPrimaryContainer = GrPrimaryHoverDark,
    secondary = GrInfoDark,
    onSecondary = GrPrimaryInkDark,
    secondaryContainer = GrPrimarySoftDark,
    onSecondaryContainer = GrInfoDark,
    background = GrBgDark,
    onBackground = GrTextDark,
    surface = GrBgDark,
    onSurface = GrTextDark,
    surfaceVariant = GrSurface2Dark,
    onSurfaceVariant = GrMutedDark,
    surfaceBright = GrSurfaceHighDark,
    surfaceDim = GrSurfaceLowestDark,
    surfaceContainerLowest = GrSurfaceLowestDark,
    surfaceContainerLow = GrSurfaceLowDark,
    surfaceContainer = GrSurfaceContainerDark,
    surfaceContainerHigh = GrSurfaceHighDark,
    surfaceContainerHighest = GrSurfaceHighestDark,
    outline = GrMutedDark,
    outlineVariant = GrBorderDark,
    error = GrDangerDark,
    onError = GrPrimaryInkDark,
    errorContainer = GrDangerSoftDark,
    onErrorContainer = GrDangerDark,
)

/** Consistent Ride Blue CTA button fill in both Light and Dark themes (matching `--btn-bg`). */
@Composable
fun graceRidePrimaryButtonColors(): ButtonColors = ButtonDefaults.buttonColors(
    containerColor = GrPrimary,
    contentColor = Color.White,
)

enum class ThemeMode(val key: String) {
    SYSTEM("system"),
    LIGHT("light"),
    DARK("dark"),
    ;

    companion object {
        fun fromKey(raw: String?): ThemeMode = entries.firstOrNull { it.key == raw } ?: SYSTEM
    }
}

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

// Native Material 3 shape progression while keeping pills explicitly CircleShape.
private val GrShapes = Shapes(
    extraSmall = RoundedCornerShape(8.dp),
    small = RoundedCornerShape(10.dp),
    medium = RoundedCornerShape(12.dp),
    large = RoundedCornerShape(16.dp),
    extraLarge = RoundedCornerShape(24.dp),
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
