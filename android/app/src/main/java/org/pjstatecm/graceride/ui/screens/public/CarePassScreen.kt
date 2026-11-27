package org.pjstatecm.graceride.ui.screens.public

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.Call
import androidx.compose.material.icons.outlined.CheckCircle
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Lock
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material.icons.outlined.Schedule
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.GraceRideLiveMap
import org.pjstatecm.graceride.ui.components.LiveMapPoint
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.MapMarkerKind
import org.pjstatecm.graceride.ui.components.RideStatusPill
import org.pjstatecm.graceride.ui.dialPhone
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import kotlin.math.roundToInt

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CarePassScreen(
    container: AppContainer,
    token: String,
    onBack: () -> Unit,
) {
    org.pjstatecm.graceride.ui.SecureScreenEffect()
    val vm: CarePassViewModel = viewModel(
        key = "care_$token",
        factory = CarePassViewModel.factory(container, token),
    )
    val state by vm.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val view = LocalView.current
    val snackbar = remember { SnackbarHostState() }
    val statusColors = LocalStatusColors.current

    val toastText = state.toastRes?.let { stringResource(it) }
    LaunchedEffect(toastText) {
        toastText?.let {
            snackbar.showSnackbar(it)
            vm.clearToast()
        }
    }
    LaunchedEffect(state.actionError) {
        state.actionError?.let {
            snackbar.showSnackbar(it)
            vm.clearActionError()
        }
    }

    val activeCare = state.status?.takeIf { it.completedAt == null && it.status != "cancelled" && !state.closedError }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.care_badge)) },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.common_back))
                    }
                },
                actions = {
                    IconButton(onClick = { UiHaptics.tick(view); vm.refresh(initial = true) }) {
                        Icon(Icons.Outlined.Refresh, contentDescription = stringResource(R.string.common_retry))
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background),
            )
        },
        bottomBar = {
            if (activeCare != null) {
                Surface(
                    color = MaterialTheme.colorScheme.surface,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Column(Modifier.navigationBarsPadding()) {
                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                        Column(
                            Modifier
                                .fillMaxWidth()
                                .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            Text(
                                stringResource(R.string.care_readiness_label),
                                style = MaterialTheme.typography.labelMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            Button(
                                onClick = {
                                    UiHaptics.confirm(view)
                                    vm.sendReadiness("ready")
                                },
                                enabled = !state.updating,
                                colors = graceRidePrimaryButtonColors(),
                                modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
                            ) {
                                Text(
                                    stringResource(R.string.care_rider_ready, activeCare.riderName.ifBlank { activeCare.label }),
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                )
                            }
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                activeCare.driver?.phone?.takeIf { it.isNotBlank() }?.let { phone ->
                                    FilledTonalButton(
                                        onClick = {
                                            UiHaptics.tick(view)
                                            dialPhone(context, phone)
                                        },
                                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                                    ) {
                                        Icon(Icons.Outlined.Call, contentDescription = null, modifier = Modifier.size(18.dp))
                                        Spacer(Modifier.width(Spacing.xxs))
                                        Text(
                                            stringResource(R.string.visit_call_driver),
                                            maxLines = 1,
                                            overflow = TextOverflow.Ellipsis,
                                        )
                                    }
                                }
                                OutlinedButton(
                                    onClick = {
                                        UiHaptics.tick(view)
                                        vm.sendReadiness("delayed_5m")
                                    },
                                    enabled = !state.updating,
                                    modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                                ) {
                                    Text(
                                        stringResource(R.string.visit_late),
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                    )
                                }
                            }
                        }
                    }
                }
            }
        },
        snackbarHost = { SnackbarHost(snackbar) },
        containerColor = MaterialTheme.colorScheme.background,
    ) { padding ->
        when {
            state.loading -> LoadingView(Modifier.fillMaxSize().padding(padding))
            state.closedError || state.status == null -> ErrorView(
                message = state.loadError ?: stringResource(R.string.care_closed_title),
                modifier = Modifier.padding(padding),
                onRetry = { vm.refresh(initial = true) },
            )
            else -> {
                val care = state.status!!
                val vehicleText = care.driver?.vehicle?.let { v ->
                    listOfNotNull(
                        v.color?.takeIf { it.isNotBlank() },
                        v.model?.takeIf { it.isNotBlank() },
                        v.plate?.takeIf { it.isNotBlank() }?.let { "($it)" },
                    ).joinToString(" ")
                }.orEmpty()
                val etaMins = care.location.eta?.durationS?.let { (it / 60.0).roundToInt().coerceAtLeast(1) }

                LazyColumn(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding)
                        .padding(horizontal = Spacing.sm),
                    contentPadding = PaddingValues(vertical = Spacing.sm),
                    verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                ) {
                    item {
                        OutlinedCard(
                            modifier = Modifier.fillMaxWidth(),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                        ) {
                            Column(
                                Modifier.padding(Spacing.sm),
                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                Row(
                                    Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Text(
                                        stringResource(R.string.care_title, care.riderName.ifBlank { care.label }),
                                        style = MaterialTheme.typography.headlineSmall,
                                        maxLines = 2,
                                        overflow = TextOverflow.Ellipsis,
                                        modifier = Modifier.weight(1f),
                                    )
                                    RideStatusPill(care.status)
                                }
                                if (care.leg == "return") {
                                    AssistChip(
                                        onClick = {},
                                        label = { Text(stringResource(R.string.care_return_leg)) },
                                    )
                                }
                                Surface(
                                    color = MaterialTheme.colorScheme.surfaceVariant,
                                    shape = MaterialTheme.shapes.small,
                                ) {
                                    Row(
                                        Modifier.padding(horizontal = Spacing.xs, vertical = Spacing.xxs),
                                        verticalAlignment = Alignment.CenterVertically,
                                        horizontalArrangement = Arrangement.spacedBy(Spacing.xxs),
                                    ) {
                                        Icon(Icons.Outlined.Lock, contentDescription = null, modifier = Modifier.size(14.dp))
                                        Text(
                                            stringResource(R.string.care_privacy_hint),
                                            style = MaterialTheme.typography.labelSmall,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                    }
                                }
                                when {
                                    care.completedAt != null -> CalloutBanner(
                                        icon = Icons.Outlined.CheckCircle,
                                        text = stringResource(R.string.care_arrived_safely, care.destination),
                                        containerColor = statusColors.successSoft,
                                        contentColor = statusColors.success,
                                    )
                                    care.pickedUpAt != null -> CalloutBanner(
                                        icon = Icons.Outlined.DirectionsCar,
                                        text = stringResource(R.string.care_picked_up_on_way, care.destination),
                                        containerColor = statusColors.infoSoft,
                                        contentColor = statusColors.info,
                                    )
                                    care.arrivedAtGateAt != null -> CalloutBanner(
                                        icon = Icons.Outlined.DirectionsCar,
                                        text = stringResource(R.string.visit_at_gate),
                                        containerColor = statusColors.successSoft,
                                        contentColor = statusColors.success,
                                    )
                                }
                                if (care.delayMinutes > 0) {
                                    CalloutBanner(
                                        icon = Icons.Outlined.Schedule,
                                        text = stringResource(R.string.visit_driver_late, care.delayMinutes),
                                        containerColor = statusColors.warningSoft,
                                        contentColor = statusColors.warning,
                                    )
                                }
                            }
                        }
                    }

                    item {
                        OutlinedCard(
                            modifier = Modifier.fillMaxWidth(),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                        ) {
                            Column(
                                Modifier.padding(Spacing.sm),
                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                Text(
                                    stringResource(R.string.visit_time),
                                    style = MaterialTheme.typography.labelMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                                Text(KlTime.display(care.pickupAt), style = MaterialTheme.typography.titleMedium)
                                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                Text(
                                    stringResource(R.string.care_route),
                                    style = MaterialTheme.typography.labelMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                                Text(
                                    "${care.area.name} → ${care.destination}",
                                    style = MaterialTheme.typography.bodyLarge,
                                )
                                etaMins?.let { mins ->
                                    Text(
                                        stringResource(R.string.care_eta_mins, mins),
                                        style = MaterialTheme.typography.titleSmall,
                                        color = MaterialTheme.colorScheme.primary,
                                    )
                                }
                                care.driver?.let { driver ->
                                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                    Text(
                                        stringResource(R.string.visit_driver),
                                        style = MaterialTheme.typography.labelMedium,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                    Text(driver.name, style = MaterialTheme.typography.bodyLarge)
                                    if (vehicleText.isNotBlank()) {
                                        Text(
                                            vehicleText,
                                            style = MaterialTheme.typography.bodyMedium,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                    }
                                    if (!driver.phone.isNullOrBlank() && activeCare == null) {
                                        FilledTonalButton(
                                            onClick = { dialPhone(context, driver.phone) },
                                            modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
                                        ) {
                                            Icon(Icons.Outlined.Call, contentDescription = null, modifier = Modifier.size(18.dp))
                                            Spacer(Modifier.width(Spacing.xs))
                                            Text(stringResource(R.string.visit_call_driver))
                                        }
                                    }
                                }
                            }
                        }
                    }

                    val loc = care.location
                    if (loc.sharing && loc.lat != null && loc.lng != null) {
                        item {
                            OutlinedCard(
                                modifier = Modifier.fillMaxWidth(),
                                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                            ) {
                                Column(
                                    Modifier.padding(Spacing.sm),
                                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                ) {
                                    Text(
                                        stringResource(R.string.visit_driver_here),
                                        style = MaterialTheme.typography.titleSmall,
                                    )
                                    GraceRideLiveMap(
                                        points = listOf(
                                            LiveMapPoint(
                                                id = "driver",
                                                lat = loc.lat,
                                                lng = loc.lng,
                                                label = care.driver?.name.orEmpty(),
                                                kind = MapMarkerKind.CAR,
                                            ),
                                        ),
                                        showZoomControls = true,
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .height(200.dp),
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
