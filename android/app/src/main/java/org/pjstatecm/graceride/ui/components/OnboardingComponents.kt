package org.pjstatecm.graceride.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.CheckCircle
import androidx.compose.material.icons.outlined.ChevronRight
import androidx.compose.material.icons.outlined.HourglassTop
import androidx.compose.material.icons.outlined.RadioButtonUnchecked
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.tabular

private enum class OnboardingStepState { DONE, TODO, WAITING }

private data class OnboardingStepItem(
    val titleRes: Int,
    val hintRes: Int?,
    val state: OnboardingStepState,
    val route: AppRoute? = null,
)

/**
 * Waiting-room checklist displayed on `PendingScreen` while transport leads review a new account.
 * Mirrors web `PendingChecklistCard` (`frontend/src/components/Onboarding.tsx`).
 */
@Composable
fun PendingChecklistCard(me: Me, modifier: Modifier = Modifier) {
    val c = LocalStatusColors.current
    val steps = listOf(
        OnboardingStepItem(
            titleRes = R.string.onboarding_joined,
            hintRes = null,
            state = OnboardingStepState.DONE,
        ),
        OnboardingStepItem(
            titleRes = R.string.onboarding_passkey,
            hintRes = R.string.onboarding_passkey_hint,
            state = if (me.passkeyCount > 0 || me.biometricCount > 0) OnboardingStepState.DONE else OnboardingStepState.TODO,
        ),
        OnboardingStepItem(
            titleRes = R.string.onboarding_photo,
            hintRes = R.string.onboarding_photo_hint,
            state = if (me.photoFileId != null) OnboardingStepState.DONE else OnboardingStepState.TODO,
        ),
        OnboardingStepItem(
            titleRes = R.string.onboarding_review,
            hintRes = R.string.onboarding_review_hint,
            state = OnboardingStepState.WAITING,
        ),
    )

    OutlinedCard(modifier = modifier.fillMaxWidth()) {
        Column(
            Modifier.padding(Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Text(
                stringResource(R.string.onboarding_waiting_title),
                style = MaterialTheme.typography.titleMedium,
            )
            Text(
                stringResource(R.string.onboarding_waiting_body),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            steps.forEachIndexed { idx, step ->
                if (idx > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                Row(
                    Modifier
                        .fillMaxWidth()
                        .padding(vertical = Spacing.xxs),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    val icon = when (step.state) {
                        OnboardingStepState.DONE -> Icons.Outlined.CheckCircle
                        OnboardingStepState.WAITING -> Icons.Outlined.HourglassTop
                        OnboardingStepState.TODO -> Icons.Outlined.RadioButtonUnchecked
                    }
                    val tint = when (step.state) {
                        OnboardingStepState.DONE -> c.success
                        OnboardingStepState.WAITING -> c.warning
                        OnboardingStepState.TODO -> MaterialTheme.colorScheme.onSurfaceVariant
                    }
                    Icon(icon, contentDescription = null, tint = tint, modifier = Modifier.size(20.dp))
                    Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        Text(
                            stringResource(step.titleRes),
                            style = MaterialTheme.typography.bodyMedium,
                            fontWeight = FontWeight.Medium,
                        )
                        step.hintRes?.let { h ->
                            Text(
                                stringResource(h),
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                    val pillRes = when (step.state) {
                        OnboardingStepState.DONE -> R.string.onboarding_state_done
                        OnboardingStepState.WAITING -> R.string.onboarding_state_waiting
                        OnboardingStepState.TODO -> R.string.onboarding_state_todo
                    }
                    val pillColors = when (step.state) {
                        OnboardingStepState.DONE -> c.successSoft to c.success
                        OnboardingStepState.WAITING -> c.warningSoft to c.warning
                        OnboardingStepState.TODO -> MaterialTheme.colorScheme.surfaceContainerHigh to MaterialTheme.colorScheme.onSurfaceVariant
                    }
                    Surface(
                        color = pillColors.first,
                        contentColor = pillColors.second,
                        shape = CircleShape,
                    ) {
                        Text(
                            stringResource(pillRes),
                            style = MaterialTheme.typography.labelSmall,
                            modifier = Modifier.padding(horizontal = Spacing.xs, vertical = 2.dp),
                        )
                    }
                }
            }
            Surface(
                color = MaterialTheme.colorScheme.surfaceContainerLow,
                shape = MaterialTheme.shapes.small,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(
                    stringResource(R.string.onboarding_waiting_tip),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(Spacing.xs),
                )
            }
        }
    }
}

/**
 * Getting-started checklist displayed on `HomeScreen` for active members until dismissed.
 * Shows only role-relevant setup milestones (`saved place`, `event signup`, `first ride`, `driver vehicle`).
 */
@Composable
fun OnboardingChecklistCard(
    me: Me,
    hasSavedPlace: Boolean,
    hasRide: Boolean,
    hasEvent: Boolean,
    hasVehicle: Boolean,
    onNavigate: (AppRoute) -> Unit,
    onHide: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = LocalStatusColors.current
    val view = LocalView.current
    val canBookRide = me.can("rides", "create")
    val canJoinEvents = me.can("events")
    val isDriver = me.can("drive")

    val steps = buildList {
        if (canBookRide) {
            add(
                OnboardingStepItem(
                    titleRes = R.string.onboarding_place,
                    hintRes = R.string.onboarding_place_hint,
                    state = if (hasSavedPlace) OnboardingStepState.DONE else OnboardingStepState.TODO,
                    route = AppRoute.SavedAddresses,
                ),
            )
        }
        if (canJoinEvents) {
            add(
                OnboardingStepItem(
                    titleRes = R.string.onboarding_event,
                    hintRes = R.string.onboarding_event_hint,
                    state = if (hasEvent) OnboardingStepState.DONE else OnboardingStepState.TODO,
                    route = AppRoute.Events,
                ),
            )
        }
        if (canBookRide) {
            add(
                OnboardingStepItem(
                    titleRes = R.string.onboarding_ride,
                    hintRes = R.string.onboarding_ride_hint,
                    state = if (hasRide) OnboardingStepState.DONE else OnboardingStepState.TODO,
                    route = AppRoute.RideNew(),
                ),
            )
        }
        if (isDriver) {
            add(
                OnboardingStepItem(
                    titleRes = R.string.onboarding_car,
                    hintRes = R.string.onboarding_car_hint,
                    state = if (hasVehicle) OnboardingStepState.DONE else OnboardingStepState.TODO,
                    route = AppRoute.DriverSettings,
                ),
            )
        }
    }
    if (steps.isEmpty()) return

    val doneCount = steps.count { it.state == OnboardingStepState.DONE }
    val progress = (doneCount.toFloat() / steps.size.coerceAtLeast(1)).coerceIn(0f, 1f)

    OutlinedCard(modifier = modifier.fillMaxWidth()) {
        Column(
            Modifier.padding(Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(Modifier.weight(1f)) {
                    Text(
                        stringResource(R.string.onboarding_start_title),
                        style = MaterialTheme.typography.titleMedium,
                    )
                    Text(
                        stringResource(R.string.onboarding_progress, doneCount, steps.size),
                        style = MaterialTheme.typography.labelSmall.tabular(),
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                TextButton(
                    onClick = onHide,
                    modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                ) {
                    Text(stringResource(R.string.onboarding_hide))
                }
            }

            LinearProgressIndicator(
                progress = { progress },
                modifier = Modifier.fillMaxWidth(),
            )

            Text(
                stringResource(R.string.onboarding_start_body),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            steps.forEachIndexed { idx, step ->
                if (idx > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                Surface(
                    onClick = {
                        step.route?.let {
                            UiHaptics.tick(view)
                            onNavigate(it)
                        }
                    },
                    color = MaterialTheme.colorScheme.surface,
                ) {
                    Row(
                        Modifier
                            .fillMaxWidth()
                            .defaultMinSize(minHeight = 48.dp)
                            .padding(vertical = Spacing.xs),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        val icon = if (step.state == OnboardingStepState.DONE) {
                            Icons.Outlined.CheckCircle
                        } else {
                            Icons.Outlined.RadioButtonUnchecked
                        }
                        val tint = if (step.state == OnboardingStepState.DONE) {
                            c.success
                        } else {
                            MaterialTheme.colorScheme.onSurfaceVariant
                        }
                        Icon(icon, contentDescription = null, tint = tint, modifier = Modifier.size(20.dp))
                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text(
                                stringResource(step.titleRes),
                                style = MaterialTheme.typography.bodyMedium,
                                fontWeight = FontWeight.Medium,
                            )
                            step.hintRes?.let { h ->
                                Text(
                                    stringResource(h),
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                        Icon(
                            Icons.Outlined.ChevronRight,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.size(18.dp),
                        )
                    }
                }
            }
        }
    }
}
