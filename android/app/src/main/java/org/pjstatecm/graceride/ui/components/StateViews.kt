package org.pjstatecm.graceride.ui.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.CloudOff
import androidx.compose.material.icons.outlined.ErrorOutline
import androidx.compose.material.icons.outlined.Inbox
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.ui.theme.Spacing

/** Spring easing matching webpage `--ease-spring: cubic-bezier(0.22, 1, 0.36, 1)`. */
val GraceRideSpringEasing = CubicBezierEasing(0.22f, 1f, 0.36f, 1f)

/**
 * Branded full-area loading view featuring the native animated Grace Ride logo mark.
 * Waits 150ms when animators are enabled so instant cache hits never flash.
 */
@Composable
fun GraceRideLoadingView(
    modifier: Modifier = Modifier,
    markSize: Dp = 72.dp,
    showLabel: Boolean = true,
) {
    val shouldAnimate = remember { animatorsEnabled() }
    val revealAlpha = remember(shouldAnimate) { Animatable(if (shouldAnimate) 0f else 1f) }
    LaunchedEffect(shouldAnimate) {
        if (!shouldAnimate) return@LaunchedEffect
        delay(150L)
        revealAlpha.animateTo(1f, tween(durationMillis = 220, easing = GraceRideSpringEasing))
    }
    Box(
        modifier = modifier
            .fillMaxSize()
            .graphicsLayer { alpha = revealAlpha.value },
        contentAlignment = Alignment.Center,
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            GraceRideBrandMark(
                modifier = Modifier.size(markSize),
                mode = BrandMarkMode.LOADING,
            )
            if (showLabel) {
                Text(
                    text = stringResource(R.string.common_loading),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
fun LoadingView(modifier: Modifier = Modifier) {
    GraceRideLoadingView(modifier = modifier, markSize = 72.dp, showLabel = true)
}

@Composable
fun InlineButtonSpinner(
    modifier: Modifier = Modifier,
    color: Color = LocalContentColor.current,
) {
    CircularProgressIndicator(
        modifier = modifier.size(18.dp),
        strokeWidth = 2.dp,
        color = color,
    )
}

@Composable
fun ErrorView(message: String, modifier: Modifier = Modifier, onRetry: (() -> Unit)? = null) {
    Column(
        modifier.fillMaxSize().padding(Spacing.md),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Surface(
            color = MaterialTheme.colorScheme.errorContainer,
            shape = CircleShape,
            modifier = Modifier.size(64.dp),
        ) {
            Box(contentAlignment = Alignment.Center) {
                Icon(
                    Icons.Outlined.ErrorOutline,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onErrorContainer,
                    modifier = Modifier.size(32.dp),
                )
            }
        }
        Text(
            stringResource(R.string.common_error_title),
            style = MaterialTheme.typography.titleMedium,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.padding(top = Spacing.sm),
        )
        Text(
            message,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(top = Spacing.xs),
        )
        if (onRetry != null) {
            Button(onClick = onRetry, modifier = Modifier.padding(top = Spacing.sm)) {
                Icon(Icons.Outlined.Refresh, contentDescription = null, modifier = Modifier.size(18.dp))
                Spacer(Modifier.width(Spacing.xs))
                Text(stringResource(R.string.common_retry))
            }
        }
    }
}

@Composable
fun EmptyView(
    text: String,
    modifier: Modifier = Modifier,
    icon: ImageVector = Icons.Outlined.Inbox,
    subtitle: String? = null,
    actionLabel: String? = null,
    onAction: (() -> Unit)? = null,
) {
    Column(
        modifier.fillMaxWidth().padding(Spacing.lg),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Surface(
            color = MaterialTheme.colorScheme.surfaceVariant,
            shape = CircleShape,
            modifier = Modifier.size(64.dp),
        ) {
            Box(contentAlignment = Alignment.Center) {
                Icon(
                    icon,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.size(30.dp),
                )
            }
        }
        Text(
            text,
            style = MaterialTheme.typography.titleMedium,
            color = MaterialTheme.colorScheme.onSurface,
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(top = Spacing.sm),
        )
        if (!subtitle.isNullOrBlank()) {
            Text(
                subtitle,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
                modifier = Modifier.padding(top = Spacing.xxs),
            )
        }
        if (actionLabel != null && onAction != null) {
            Button(onClick = onAction, modifier = Modifier.padding(top = Spacing.sm)) {
                Text(actionLabel)
            }
        }
    }
}

@Composable
fun OfflineBanner(modifier: Modifier = Modifier) {
    Surface(
        color = MaterialTheme.colorScheme.secondaryContainer,
        shape = MaterialTheme.shapes.small,
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
        modifier = modifier.fillMaxWidth(),
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.padding(horizontal = Spacing.sm, vertical = Spacing.xs),
        ) {
            Icon(
                Icons.Outlined.CloudOff,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSecondaryContainer,
                modifier = Modifier.size(18.dp),
            )
            Text(
                stringResource(R.string.common_offline),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSecondaryContainer,
                modifier = Modifier.padding(start = Spacing.xs),
            )
        }
    }
}

@Composable
fun AnimatedOfflineBanner(visible: Boolean, modifier: Modifier = Modifier) {
    AnimatedVisibility(
        visible = visible,
        enter = expandVertically(tween(200)) + fadeIn(tween(180)),
        exit = shrinkVertically(tween(160)) + fadeOut(tween(140)),
        modifier = modifier,
    ) {
        OfflineBanner()
    }
}

@Composable
fun CalloutBanner(
    icon: ImageVector,
    text: String,
    containerColor: Color,
    contentColor: Color,
    modifier: Modifier = Modifier,
    actionLabel: String? = null,
    onAction: (() -> Unit)? = null,
) {
    val body: @Composable () -> Unit = {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
            modifier = Modifier.padding(horizontal = Spacing.sm, vertical = Spacing.xs),
        ) {
            Icon(
                imageVector = icon,
                contentDescription = null,
                tint = contentColor,
                modifier = Modifier.size(20.dp),
            )
            Text(
                text = text,
                style = MaterialTheme.typography.bodyMedium,
                color = contentColor,
                modifier = Modifier.weight(1f),
            )
            if (actionLabel != null && onAction != null) {
                Surface(
                    color = contentColor.copy(alpha = 0.14f),
                    contentColor = contentColor,
                    shape = CircleShape,
                ) {
                    Text(
                        text = actionLabel,
                        style = MaterialTheme.typography.labelMedium,
                        modifier = Modifier.padding(horizontal = 10.dp, vertical = 4.dp),
                    )
                }
            }
        }
    }
    if (onAction != null) {
        Surface(
            onClick = onAction,
            color = containerColor,
            contentColor = contentColor,
            shape = MaterialTheme.shapes.medium,
            border = BorderStroke(1.dp, contentColor.copy(alpha = 0.25f)),
            modifier = modifier.fillMaxWidth(),
            content = body,
        )
    } else {
        Surface(
            color = containerColor,
            contentColor = contentColor,
            shape = MaterialTheme.shapes.medium,
            border = BorderStroke(1.dp, contentColor.copy(alpha = 0.25f)),
            modifier = modifier.fillMaxWidth(),
            content = body,
        )
    }
}

@Composable
fun SkeletonLoadingList(modifier: Modifier = Modifier, @Suppress("UNUSED_PARAMETER") count: Int = 3) {
    GraceRideLoadingView(modifier = modifier, markSize = 68.dp, showLabel = true)
}

@Composable
fun HomeSkeleton(modifier: Modifier = Modifier) {
    GraceRideLoadingView(modifier = modifier, markSize = 76.dp, showLabel = true)
}

@Composable
fun DetailSkeleton(modifier: Modifier = Modifier) {
    GraceRideLoadingView(modifier = modifier, markSize = 68.dp, showLabel = true)
}
