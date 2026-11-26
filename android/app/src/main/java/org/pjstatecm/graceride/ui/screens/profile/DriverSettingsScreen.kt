package org.pjstatecm.graceride.ui.screens.profile

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Remove
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedIconButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.DriverVehicle
import org.pjstatecm.graceride.data.api.DriverVehicleReq
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.NavProvider
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.components.SkeletonLoadingList
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun DriverSettingsScreen(container: AppContainer, nav: NavHostController) {
    val vm: DriverSettingsViewModel = viewModel(factory = DriverSettingsViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val view = LocalView.current
    val snackbar = remember { SnackbarHostState() }
    val navAppPref by container.navAppPref.collectAsStateWithLifecycle()
    val autoArriveGate by container.autoArriveGate.collectAsStateWithLifecycle()

    val savedMsg = stringResource(R.string.driver_settings_saved)
    val vehicleSavedMsg = stringResource(R.string.driver_vehicle_saved)
    val vehicleSelectedMsg = stringResource(R.string.driver_vehicle_selected)
    val vehicleDeletedMsg = stringResource(R.string.driver_vehicle_deleted)

    var showVehicleSheet by remember { mutableStateOf(false) }
    var editingVehicle by remember { mutableStateOf<DriverVehicle?>(null) }
    var gateWaitExpanded by remember { mutableStateOf(false) }

    LaunchedEffect(state.message) {
        if (state.message != null) {
            snackbar.currentSnackbarData?.dismiss()
        }
        when (state.message) {
            "saved" -> snackbar.showSnackbar(savedMsg)
            "vehicle_saved" -> snackbar.showSnackbar(vehicleSavedMsg)
            "vehicle_selected" -> snackbar.showSnackbar(vehicleSelectedMsg)
            "vehicle_deleted" -> snackbar.showSnackbar(vehicleDeletedMsg)
        }
        if (state.message != null) vm.clearMessage()
    }

    val openAddVehicle = {
        vm.clearError()
        editingVehicle = null
        showVehicleSheet = true
    }

    val openEditVehicle: (DriverVehicle) -> Unit = { v ->
        vm.clearError()
        editingVehicle = v
        showVehicleSheet = true
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.driver_settings_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(
                            Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = stringResource(R.string.a11y_back),
                        )
                    }
                },
                actions = {
                    if (container.auth.me?.can("drive", "update") == true && !state.accessDenied) {
                        IconButton(onClick = openAddVehicle) {
                            Icon(
                                Icons.Outlined.Add,
                                contentDescription = stringResource(R.string.driver_add_vehicle),
                            )
                        }
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        val canDriveUpdate = container.auth.me?.can("drive", "update") == true
        when {
            state.accessDenied -> EmptyView(
                text = stringResource(R.string.role_driver_access_required),
                icon = Icons.Outlined.DirectionsCar,
                modifier = Modifier.padding(padding),
            )
            state.loading -> SkeletonLoadingList(count = 3, modifier = Modifier.padding(padding))
            state.error != null && state.areas.isEmpty() && state.vehicles.isEmpty() -> ErrorView(
                message = state.error!!,
                onRetry = vm::load,
                modifier = Modifier.padding(padding),
            )
            else -> PullToRefreshBox(
                isRefreshing = state.refreshing,
                onRefresh = vm::refresh,
                modifier = Modifier.fillMaxSize().padding(padding),
            ) {
                LazyColumn(
                    Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
                    contentPadding = PaddingValues(vertical = Spacing.sm),
                    verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                ) {
                    item { SectionHeader(stringResource(R.string.driver_vehicles_title)) }
                    item {
                        OutlinedCard(Modifier.fillMaxWidth()) {
                            if (state.vehicles.isEmpty()) {
                                EmptyView(
                                    text = stringResource(R.string.driver_no_vehicles),
                                    icon = Icons.Outlined.DirectionsCar,
                                    actionLabel = if (canDriveUpdate) stringResource(R.string.driver_add_vehicle) else null,
                                    onAction = if (canDriveUpdate) openAddVehicle else null,
                                )
                            } else {
                                Column {
                                    state.vehicles.forEachIndexed { idx, v ->
                                        if (idx > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                        val isActive = state.activeVehicleId != null && v.id == state.activeVehicleId ||
                                            (state.activeVehicleId == null && v.isActive)
                                        Row(
                                            Modifier
                                                .fillMaxWidth()
                                                .clickable(enabled = !state.busy && canDriveUpdate) { openEditVehicle(v) }
                                                .padding(horizontal = Spacing.sm, vertical = Spacing.sm),
                                            verticalAlignment = Alignment.CenterVertically,
                                        ) {
                                            Icon(
                                                Icons.Outlined.DirectionsCar,
                                                contentDescription = null,
                                                tint = if (isActive) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                                            )
                                            Column(
                                                Modifier.weight(1f).padding(horizontal = Spacing.xs),
                                                verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                                            ) {
                                                Row(
                                                    verticalAlignment = Alignment.CenterVertically,
                                                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                                ) {
                                                    Text(v.plate, style = MaterialTheme.typography.titleMedium)
                                                    if (isActive) {
                                                        StatusPill(
                                                            text = stringResource(R.string.driver_vehicle_active),
                                                            kind = RideStatusKind.ACTIVE,
                                                        )
                                                    }
                                                }
                                                Text(
                                                    stringResource(R.string.driver_vehicle_summary, v.color, v.model, v.seats),
                                                    style = MaterialTheme.typography.bodyMedium,
                                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                )
                                                val caps = buildList {
                                                    if (v.wheelchairOk) add(stringResource(R.string.driver_cap_wheelchair))
                                                    if (v.lowStep) add(stringResource(R.string.driver_cap_low_step))
                                                    if (v.childSeat) add(stringResource(R.string.driver_cap_child_seat))
                                                }
                                                if (caps.isNotEmpty()) {
                                                    FlowRow(
                                                        horizontalArrangement = Arrangement.spacedBy(Spacing.xxs),
                                                        verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                                                    ) {
                                                        caps.forEach { cap ->
                                                            Surface(
                                                                color = MaterialTheme.colorScheme.surfaceVariant,
                                                                contentColor = MaterialTheme.colorScheme.onSurfaceVariant,
                                                                shape = CircleShape,
                                                            ) {
                                                                Text(
                                                                    cap,
                                                                    style = MaterialTheme.typography.labelMedium,
                                                                    modifier = Modifier.padding(horizontal = Spacing.xs, vertical = 2.dp),
                                                                )
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            if (!isActive && v.id != null && canDriveUpdate) {
                                                TextButton(
                                                    enabled = !state.busy,
                                                    onClick = {
                                                        UiHaptics.tick(view)
                                                        vm.selectVehicle(v.id)
                                                    },
                                                ) {
                                                    Text(stringResource(R.string.driver_select_vehicle))
                                                }
                                            }
                                        }
                                    }
                                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                    Text(
                                        stringResource(R.string.driver_vehicles_instant),
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        modifier = Modifier.padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                                    )
                                }
                            }
                        }
                    }

                    item { SectionHeader(stringResource(R.string.driver_areas_title)) }
                    item {
                        OutlinedCard(Modifier.fillMaxWidth()) {
                            Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                Text(
                                    stringResource(R.string.driver_areas_hint),
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                                FlowRow(
                                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                ) {
                                    state.areas.forEach { area ->
                                        FilterChip(
                                            selected = area.id in state.selectedAreaIds,
                                            onClick = {
                                                UiHaptics.tick(view)
                                                vm.toggleArea(area.id)
                                            },
                                            label = { Text(area.name) },
                                        )
                                    }
                                }
                            }
                        }
                    }

                    item { SectionHeader(stringResource(R.string.driver_carpool_title)) }
                    item {
                        OutlinedCard(Modifier.fillMaxWidth()) {
                            Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.sm)) {
                                Row(
                                    Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Text(
                                        stringResource(R.string.driver_carpool_label),
                                        style = MaterialTheme.typography.bodyMedium,
                                        modifier = Modifier.weight(1f).padding(end = Spacing.xs),
                                    )
                                    Switch(
                                        checked = state.carpoolEnabled,
                                        onCheckedChange = {
                                            UiHaptics.tick(view)
                                            vm.setCarpool(it)
                                        },
                                    )
                                }

                                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)

                                Column(verticalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
                                    Row(
                                        Modifier.fillMaxWidth(),
                                        verticalAlignment = Alignment.CenterVertically,
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                    ) {
                                        Text(stringResource(R.string.driver_spare_seats), style = MaterialTheme.typography.titleSmall)
                                        Row(
                                            verticalAlignment = Alignment.CenterVertically,
                                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                        ) {
                                            OutlinedIconButton(
                                                enabled = state.spareSeats > 1,
                                                onClick = {
                                                    UiHaptics.tick(view)
                                                    vm.stepSpareSeats(-1)
                                                },
                                                modifier = Modifier.size(48.dp),
                                            ) {
                                                Icon(Icons.Outlined.Remove, contentDescription = stringResource(R.string.a11y_decrease))
                                            }
                                            Text(
                                                state.spareSeats.toString(),
                                                style = MaterialTheme.typography.titleLarge.tabular(),
                                                textAlign = TextAlign.Center,
                                                modifier = Modifier.width(28.dp),
                                            )
                                            OutlinedIconButton(
                                                enabled = state.spareSeats < state.maxSpareSeats,
                                                onClick = {
                                                    UiHaptics.tick(view)
                                                    vm.stepSpareSeats(1)
                                                },
                                                modifier = Modifier.size(48.dp),
                                            ) {
                                                Icon(Icons.Outlined.Add, contentDescription = stringResource(R.string.a11y_increase))
                                            }
                                        }
                                    }
                                    state.activeVehicle?.let { v ->
                                        Text(
                                            stringResource(R.string.driver_spare_max_hint, state.maxSpareSeats, v.seats),
                                            style = MaterialTheme.typography.bodySmall,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                    }
                                }

                                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)

                                val waitOptions = listOf(
                                    3 to stringResource(R.string.driver_wait_3m),
                                    5 to stringResource(R.string.driver_wait_5m),
                                    8 to stringResource(R.string.driver_wait_8m),
                                    10 to stringResource(R.string.driver_wait_10m),
                                )
                                ExposedDropdownMenuBox(
                                    expanded = gateWaitExpanded,
                                    onExpandedChange = { gateWaitExpanded = it },
                                ) {
                                    OutlinedTextField(
                                        value = waitOptions.firstOrNull { it.first == state.gateWaitMins }?.second
                                            ?: stringResource(R.string.driver_wait_mins_fallback, state.gateWaitMins),
                                        onValueChange = {},
                                        readOnly = true,
                                        label = { Text(stringResource(R.string.driver_gate_wait)) },
                                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = gateWaitExpanded) },
                                        modifier = Modifier.menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
                                    )
                                    ExposedDropdownMenu(
                                        expanded = gateWaitExpanded,
                                        onDismissRequest = { gateWaitExpanded = false },
                                    ) {
                                        waitOptions.forEach { (mins, label) ->
                                            DropdownMenuItem(
                                                text = { Text(label) },
                                                onClick = { vm.setGateWaitMins(mins); gateWaitExpanded = false },
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }

                    item { SectionHeader(stringResource(R.string.driver_nav_arrival_section)) }
                    item {
                        OutlinedCard(
                            modifier = Modifier.fillMaxWidth(),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                        ) {
                            Column(
                                Modifier.padding(Spacing.sm),
                                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                            ) {
                                Column(verticalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
                                    Text(stringResource(R.string.nav_app_title), style = MaterialTheme.typography.titleSmall)
                                    val navOpts = listOf(
                                        NavProvider.ASK.key to stringResource(R.string.nav_app_ask),
                                        NavProvider.WAZE.key to stringResource(R.string.nav_app_waze),
                                        NavProvider.GOOGLE_MAPS.key to stringResource(R.string.nav_app_google),
                                    )
                                    SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
                                        navOpts.forEachIndexed { idx, (key, label) ->
                                            SegmentedButton(
                                                selected = navAppPref == key,
                                                onClick = {
                                                    UiHaptics.tick(view)
                                                    container.setNavAppPref(key)
                                                },
                                                shape = SegmentedButtonDefaults.itemShape(index = idx, count = navOpts.size),
                                                label = { Text(label, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                                            )
                                        }
                                    }
                                }
                                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                Row(
                                    Modifier
                                        .fillMaxWidth()
                                        .toggleable(
                                            value = autoArriveGate,
                                            role = Role.Switch,
                                            onValueChange = {
                                                UiHaptics.tick(view)
                                                container.setAutoArriveGate(it)
                                            },
                                        ),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Column(Modifier.weight(1f).padding(end = Spacing.sm)) {
                                        Text(stringResource(R.string.driver_auto_arrive_title), style = MaterialTheme.typography.titleSmall)
                                        Text(
                                            stringResource(R.string.driver_auto_arrive_desc),
                                            style = MaterialTheme.typography.bodySmall,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                    }
                                    Switch(
                                        checked = autoArriveGate,
                                        onCheckedChange = null,
                                    )
                                }
                            }
                        }
                    }

                    state.error?.let { err ->
                        item {
                            Text(err, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium)
                        }
                    }

                    if (canDriveUpdate) {
                        item {
                            Button(
                                onClick = {
                                    UiHaptics.confirm(view)
                                    vm.saveProfile()
                                },
                                enabled = !state.busy && state.dirty,
                                colors = graceRidePrimaryButtonColors(),
                                modifier = Modifier.fillMaxWidth().padding(bottom = Spacing.md).height(52.dp),
                            ) {
                                if (state.busy) {
                                    InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                } else {
                                    Text(stringResource(R.string.driver_save_settings))
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if (showVehicleSheet) {
        VehicleSheet(
            editing = editingVehicle,
            busy = state.busy,
            error = state.error,
            onDismiss = {
                showVehicleSheet = false
                editingVehicle = null
            },
            onSave = { req ->
                vm.saveVehicle(editingVehicle?.id, req) {
                    showVehicleSheet = false
                    editingVehicle = null
                }
            },
            onDelete = { id ->
                vm.deleteVehicle(id) {
                    showVehicleSheet = false
                    editingVehicle = null
                }
            },
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun VehicleSheet(
    editing: DriverVehicle?,
    busy: Boolean,
    error: String?,
    onDismiss: () -> Unit,
    onSave: (DriverVehicleReq) -> Unit,
    onDelete: (Long) -> Unit,
) {
    var model by remember(editing) { mutableStateOf(editing?.model ?: "") }
    var color by remember(editing) { mutableStateOf(editing?.color ?: "") }
    var plate by remember(editing) { mutableStateOf(editing?.plate ?: "") }
    var seats by remember(editing) { mutableIntStateOf(editing?.seats ?: 5) }
    var vehicleType by remember(editing) { mutableStateOf(editing?.vehicleType ?: "standard") }
    var wheelchairOk by remember(editing) { mutableStateOf(editing?.wheelchairOk ?: false) }
    var lowStep by remember(editing) { mutableStateOf(editing?.lowStep ?: false) }
    var childSeat by remember(editing) { mutableStateOf(editing?.childSeat ?: false) }
    var isActive by remember(editing) { mutableStateOf(editing?.isActive ?: true) }
    var typeExpanded by remember { mutableStateOf(false) }
    var confirmDelete by remember { mutableStateOf(false) }
    val view = LocalView.current

    val canSave = !busy && model.isNotBlank() && color.isNotBlank() && plate.isNotBlank()
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
        Column(
            Modifier
                .fillMaxWidth()
                .navigationBarsPadding()
                .imePadding()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = Spacing.md)
                .padding(bottom = Spacing.md),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Text(
                stringResource(if (editing == null) R.string.driver_add_vehicle else R.string.driver_edit_vehicle),
                style = MaterialTheme.typography.titleLarge,
            )
            OutlinedCard(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(Spacing.sm)) {
                    Text(plate.trim().uppercase().ifBlank { "VXX 1234" }, style = MaterialTheme.typography.titleLarge)
                    Text(
                        stringResource(
                            R.string.driver_vehicle_summary,
                            color.trim().ifBlank { stringResource(R.string.driver_placeholder_color) },
                            model.trim().ifBlank { stringResource(R.string.driver_placeholder_model) },
                            seats,
                        ),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            OutlinedTextField(
                value = model,
                onValueChange = { model = it },
                label = { Text(stringResource(R.string.driver_vehicle_model)) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = color,
                onValueChange = { color = it },
                label = { Text(stringResource(R.string.driver_vehicle_color)) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = plate,
                onValueChange = { plate = it },
                label = { Text(stringResource(R.string.driver_vehicle_plate)) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                Text(stringResource(R.string.driver_vehicle_seats), style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                OutlinedIconButton(
                    enabled = seats > 2,
                    onClick = { UiHaptics.tick(view); seats = (seats - 1).coerceAtLeast(2) },
                    modifier = Modifier.size(48.dp),
                ) {
                    Icon(Icons.Outlined.Remove, contentDescription = stringResource(R.string.a11y_decrease))
                }
                Text(seats.toString(), style = MaterialTheme.typography.titleMedium.tabular())
                OutlinedIconButton(
                    enabled = seats < 9,
                    onClick = { UiHaptics.tick(view); seats = (seats + 1).coerceAtMost(9) },
                    modifier = Modifier.size(48.dp),
                ) {
                    Icon(Icons.Outlined.Add, contentDescription = stringResource(R.string.a11y_increase))
                }
            }
            ExposedDropdownMenuBox(expanded = typeExpanded, onExpandedChange = { typeExpanded = it }) {
                OutlinedTextField(
                    value = stringResource(if (vehicleType == "other") R.string.driver_vehicle_type_other else R.string.driver_vehicle_type_standard),
                    onValueChange = {},
                    readOnly = true,
                    label = { Text(stringResource(R.string.driver_vehicle_type)) },
                    trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = typeExpanded) },
                    modifier = Modifier.menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
                )
                ExposedDropdownMenu(expanded = typeExpanded, onDismissRequest = { typeExpanded = false }) {
                    DropdownMenuItem(
                        text = { Text(stringResource(R.string.driver_vehicle_type_standard)) },
                        onClick = { vehicleType = "standard"; typeExpanded = false },
                    )
                    DropdownMenuItem(
                        text = { Text(stringResource(R.string.driver_vehicle_type_other)) },
                        onClick = { vehicleType = "other"; typeExpanded = false },
                    )
                }
            }
            VehicleCheckRow(wheelchairOk, stringResource(R.string.driver_wheelchair_ok)) { wheelchairOk = it }
            VehicleCheckRow(lowStep, stringResource(R.string.driver_low_step)) { lowStep = it }
            VehicleCheckRow(childSeat, stringResource(R.string.driver_child_seat)) { childSeat = it }
            VehicleCheckRow(isActive, stringResource(R.string.driver_set_active)) { isActive = it }
            error?.let {
                Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium)
            }
            Spacer(Modifier.height(Spacing.xxs))
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                OutlinedButton(
                    onClick = onDismiss,
                    modifier = Modifier.weight(1f).height(48.dp),
                ) {
                    Text(stringResource(R.string.common_cancel))
                }
                Button(
                    enabled = canSave,
                    onClick = {
                        UiHaptics.confirm(view)
                        onSave(
                            DriverVehicleReq(
                                model = model,
                                color = color,
                                plate = plate,
                                seats = seats,
                                vehicle_type = vehicleType,
                                wheelchair_ok = wheelchairOk,
                                low_step = lowStep,
                                child_seat = childSeat,
                                is_active = isActive,
                            ),
                        )
                    },
                    modifier = Modifier.weight(1f).height(48.dp),
                ) {
                    if (busy) {
                        InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                    } else {
                        Text(stringResource(R.string.common_save))
                    }
                }
            }
            if (editing?.id != null) {
                TextButton(
                    onClick = { confirmDelete = true },
                    enabled = !busy,
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text(stringResource(R.string.driver_delete_vehicle))
                }
            }
        }
    }

    if (confirmDelete && editing?.id != null) {
        AlertDialog(
            onDismissRequest = { confirmDelete = false },
            title = { Text(stringResource(R.string.driver_delete_vehicle)) },
            text = { Text(stringResource(R.string.driver_delete_vehicle_confirm, editing.plate, editing.model)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmDelete = false
                        onDelete(editing.id)
                    },
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                ) {
                    Text(stringResource(R.string.driver_delete_vehicle))
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmDelete = false }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }
}

@Composable
private fun VehicleCheckRow(checked: Boolean, label: String, onCheckedChange: (Boolean) -> Unit) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = Modifier
            .fillMaxWidth()
            .toggleable(value = checked, role = Role.Checkbox, onValueChange = onCheckedChange)
            .padding(vertical = Spacing.xxs),
    ) {
        Checkbox(checked = checked, onCheckedChange = null)
        Text(
            label,
            style = MaterialTheme.typography.bodyMedium,
            modifier = Modifier.padding(start = Spacing.xs),
        )
    }
}
