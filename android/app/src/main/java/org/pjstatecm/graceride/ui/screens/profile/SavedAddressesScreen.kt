package org.pjstatecm.graceride.ui.screens.profile

import android.Manifest
import android.annotation.SuppressLint
import android.content.Context
import android.content.pm.PackageManager
import android.location.LocationManager
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
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
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.LocationOn
import androidx.compose.material.icons.outlined.MyLocation
import androidx.compose.material.icons.outlined.StarOutline
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
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import androidx.core.location.LocationManagerCompat
import android.os.CancellationSignal
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.Area
import androidx.compose.material.icons.outlined.Place
import org.pjstatecm.graceride.data.api.Place
import org.pjstatecm.graceride.data.api.SavedPlace
import org.pjstatecm.graceride.ui.components.LocationPinPickerSheet
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.inferAreaId
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.SkeletonLoadingList
import org.pjstatecm.graceride.ui.theme.Spacing

private const val MAX_SAVED_PLACES = 50

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SavedAddressesScreen(container: AppContainer, nav: NavHostController) {
    org.pjstatecm.graceride.ui.SecureScreenEffect()
    val vm: SavedAddressesViewModel = viewModel(factory = SavedAddressesViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val snackbar = remember { SnackbarHostState() }
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val me = (authState as? org.pjstatecm.graceride.data.repo.AuthState.SignedIn)?.me ?: container.auth.me
    val canCreate = me?.can("rides", "create") == true
    val canUpdate = me?.can("rides", "update") == true
    val canDelete = me?.can("rides", "delete") == true

    val savedMsg = stringResource(R.string.saved_addresses_saved)
    val defaultMsg = stringResource(R.string.saved_addresses_default_updated)
    val removedMsg = stringResource(R.string.saved_addresses_removed)

    var showSheet by remember { mutableStateOf(false) }
    var editing by remember { mutableStateOf<SavedPlace?>(null) }

    LaunchedEffect(state.message) {
        when (state.message) {
            "saved" -> snackbar.showSnackbar(savedMsg)
            "default_updated" -> snackbar.showSnackbar(defaultMsg)
            "removed" -> snackbar.showSnackbar(removedMsg)
        }
        if (state.message != null) vm.clearMessage()
    }

    val openCreate = {
        vm.clearError()
        vm.clearSuggestions()
        editing = null
        showSheet = true
    }

    val openEdit: (SavedPlace) -> Unit = { place ->
        vm.clearError()
        vm.clearSuggestions()
        editing = place
        showSheet = true
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.saved_addresses_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(
                            Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = stringResource(R.string.a11y_back),
                        )
                    }
                },
                actions = {
                    if (canCreate && state.places.size < MAX_SAVED_PLACES) {
                        IconButton(onClick = openCreate) {
                            Icon(
                                Icons.Outlined.Add,
                                contentDescription = stringResource(R.string.saved_addresses_add),
                            )
                        }
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        when {
            state.loading && state.places.isEmpty() -> SkeletonLoadingList(count = 3, modifier = Modifier.padding(padding))
            state.error != null && state.places.isEmpty() -> ErrorView(
                message = state.error!!,
                onRetry = vm::load,
                modifier = Modifier.padding(padding),
            )
            state.places.isEmpty() -> Column(
                Modifier.fillMaxSize().padding(padding).padding(Spacing.md),
                verticalArrangement = Arrangement.Center,
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                EmptyView(
                    text = stringResource(R.string.saved_addresses_empty),
                    icon = Icons.Outlined.LocationOn,
                    actionLabel = if (canCreate) stringResource(R.string.saved_addresses_add) else null,
                    onAction = if (canCreate) openCreate else null,
                )
            }
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
                    item {
                        OutlinedCard(Modifier.fillMaxWidth()) {
                            Column {
                                state.places.forEachIndexed { idx, place ->
                                    if (idx > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                    SavedAddressRow(
                                        place = place,
                                        busy = state.busy,
                                        canUpdate = canUpdate,
                                        onEdit = { if (canUpdate) openEdit(place) },
                                        onSetDefault = { if (canUpdate) vm.setDefault(place) },
                                    )
                                }
                            }
                        }
                    }
                    if (state.places.size >= MAX_SAVED_PLACES) {
                        item {
                            Text(
                                stringResource(R.string.saved_addresses_limit),
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                    state.error?.let { err ->
                        item {
                            Text(err, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium)
                        }
                    }
                }
            }
        }
    }

    if (showSheet) {
        SavedAddressSheet(
            editing = editing,
            areas = state.areas,
            suggestions = state.suggestions,
            defaultInitial = editing?.isDefault == 1 || (editing == null && state.places.isEmpty()),
            busy = state.busy,
            error = state.error,
            canDelete = canDelete,
            onAddressChange = vm::searchAddress,
            onPickSuggestion = { vm.clearSuggestions() },
            onDismiss = {
                vm.clearError()
                vm.clearSuggestions()
                showSheet = false
                editing = null
            },
            onSave = { label, areaId, address, gateNotes, isDefault, lat, lng, clearCoords ->
                vm.save(
                    editingId = editing?.id,
                    label = label,
                    areaId = areaId,
                    address = address,
                    gateNotes = gateNotes,
                    isDefault = isDefault,
                    lat = lat,
                    lng = lng,
                    clearCoords = clearCoords,
                ) {
                    showSheet = false
                    editing = null
                }
            },
            onDelete = { id ->
                vm.delete(id) {
                    showSheet = false
                    editing = null
                }
            },
        )
    }
}

@Composable
private fun SavedAddressRow(
    place: SavedPlace,
    busy: Boolean,
    canUpdate: Boolean,
    onEdit: () -> Unit,
    onSetDefault: () -> Unit,
) {
    val isDefault = place.isDefault == 1
    Row(
        Modifier
            .fillMaxWidth()
            .clickable(enabled = canUpdate && !busy, onClick = onEdit)
            .padding(horizontal = Spacing.sm, vertical = Spacing.sm),
        verticalAlignment = Alignment.Top,
    ) {
        Icon(
            Icons.Outlined.LocationOn,
            contentDescription = null,
            tint = if (isDefault) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(top = Spacing.xxs),
        )
        Column(
            Modifier.weight(1f).padding(horizontal = Spacing.xs),
            verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                Text(
                    place.label,
                    style = MaterialTheme.typography.titleMedium,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                if (isDefault) {
                    Surface(
                        color = MaterialTheme.colorScheme.primaryContainer,
                        contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                        shape = CircleShape,
                    ) {
                        Text(
                            stringResource(R.string.saved_addresses_default),
                            style = MaterialTheme.typography.labelMedium,
                            modifier = Modifier.padding(horizontal = Spacing.xs, vertical = 2.dp),
                        )
                    }
                }
                if (!place.areaName.isNullOrBlank()) {
                    Text(
                        "· ${place.areaName}",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
            Text(
                place.address,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            if (place.gateNotes.isNotBlank()) {
                Text(
                    place.gateNotes,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                if (place.areaId == null) {
                    Text(
                        stringResource(R.string.saved_addresses_area_missing),
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
                val hasPin = place.lat != null && place.lng != null
                Text(
                    stringResource(if (hasPin) R.string.saved_addresses_pinned else R.string.saved_addresses_no_pin),
                    style = MaterialTheme.typography.labelSmall,
                    color = if (hasPin) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        IconButton(
            onClick = onSetDefault,
            enabled = canUpdate && !busy && !isDefault,
        ) {
            Icon(
                imageVector = if (isDefault) Icons.Filled.Star else Icons.Outlined.StarOutline,
                contentDescription = if (isDefault) {
                    stringResource(R.string.saved_addresses_default)
                } else {
                    stringResource(R.string.saved_addresses_set_default_desc, place.label)
                },
                tint = if (isDefault) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class)
@Composable
private fun SavedAddressSheet(
    editing: SavedPlace?,
    areas: List<Area>,
    suggestions: List<Place>,
    defaultInitial: Boolean,
    busy: Boolean,
    error: String?,
    canDelete: Boolean,
    onAddressChange: (String) -> Unit,
    onPickSuggestion: () -> Unit,
    onDismiss: () -> Unit,
    onSave: (label: String, areaId: Long?, address: String, gateNotes: String, isDefault: Boolean, lat: Double?, lng: Double?, clearCoords: Boolean) -> Unit,
    onDelete: (Long) -> Unit,
) {
    var label by remember(editing) { mutableStateOf(editing?.label ?: "") }
    var areaId by remember(editing) { mutableStateOf(editing?.areaId) }
    var address by remember(editing) { mutableStateOf(editing?.address ?: "") }
    var gateNotes by remember(editing) { mutableStateOf(editing?.gateNotes ?: "") }
    var isDefault by remember(editing) { mutableStateOf(defaultInitial) }
    var lat by remember(editing) { mutableStateOf(editing?.lat) }
    var lng by remember(editing) { mutableStateOf(editing?.lng) }
    var gpsStatus by remember { mutableStateOf("idle") }
    var gpsPinnedManually by remember(editing) { mutableStateOf(false) }
    var coordsCleared by remember(editing) { mutableStateOf(false) }
    var areaExpanded by remember { mutableStateOf(false) }
    var showPinPicker by remember(editing) { mutableStateOf(false) }
    var confirmDelete by remember { mutableStateOf(false) }
    val context = LocalContext.current
    val view = LocalView.current
    val scope = rememberCoroutineScope()
    var activeSignal by remember { mutableStateOf<CancellationSignal?>(null) }
    var gpsTimeoutJob by remember { mutableStateOf<Job?>(null) }

    DisposableEffect(Unit) {
        onDispose {
            gpsTimeoutJob?.cancel()
            activeSignal?.cancel()
        }
    }

    @SuppressLint("MissingPermission")
    fun captureGpsLocation() {
        val lm = context.getSystemService(LocationManager::class.java)
        if (lm == null) {
            gpsStatus = "denied"
            return
        }
        gpsTimeoutJob?.cancel()
        activeSignal?.cancel()
        gpsStatus = "locating"
        val settled = AtomicBoolean(false)
        try {
            val provider = when {
                Build.VERSION.SDK_INT >= Build.VERSION_CODES.S &&
                    lm.hasProvider(LocationManager.FUSED_PROVIDER) &&
                    lm.isProviderEnabled(LocationManager.FUSED_PROVIDER) -> LocationManager.FUSED_PROVIDER
                lm.isProviderEnabled(LocationManager.GPS_PROVIDER) -> LocationManager.GPS_PROVIDER
                lm.isProviderEnabled(LocationManager.NETWORK_PROVIDER) -> LocationManager.NETWORK_PROVIDER
                else -> null
            }
            val readLastKnown: () -> android.location.Location? = {
                try {
                    val fused = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S &&
                        lm.hasProvider(LocationManager.FUSED_PROVIDER)
                    ) {
                        lm.getLastKnownLocation(LocationManager.FUSED_PROVIDER)
                    } else {
                        null
                    }
                    fused
                        ?: lm.getLastKnownLocation(LocationManager.GPS_PROVIDER)
                        ?: lm.getLastKnownLocation(LocationManager.NETWORK_PROVIDER)
                } catch (_: SecurityException) {
                    null
                }
            }
            val applyLoc: (android.location.Location?) -> Unit = { loc ->
                if (settled.compareAndSet(false, true)) {
                    gpsTimeoutJob?.cancel()
                    activeSignal = null
                    if (loc != null) {
                        lat = loc.latitude
                        lng = loc.longitude
                        gpsPinnedManually = true
                        coordsCleared = false
                        gpsStatus = "captured"
                    } else {
                        gpsStatus = "denied"
                    }
                }
            }
            if (provider != null) {
                val signal = CancellationSignal()
                activeSignal = signal
                gpsTimeoutJob = scope.launch {
                    delay(15_000)
                    if (!settled.get()) {
                        signal.cancel()
                        applyLoc(readLastKnown())
                    }
                }
                LocationManagerCompat.getCurrentLocation(
                    lm,
                    provider,
                    signal,
                    ContextCompat.getMainExecutor(context),
                ) { loc ->
                    applyLoc(loc ?: readLastKnown())
                }
            } else {
                applyLoc(readLastKnown())
            }
        } catch (_: SecurityException) {
            settled.set(true)
            gpsTimeoutJob?.cancel()
            activeSignal = null
            gpsStatus = "denied"
        }
    }

    val permLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions(),
    ) { granted ->
        if (granted[Manifest.permission.ACCESS_FINE_LOCATION] == true ||
            granted[Manifest.permission.ACCESS_COARSE_LOCATION] == true
        ) {
            captureGpsLocation()
        } else {
            gpsStatus = "denied"
        }
    }

    fun requestOrCaptureGps() {
        val hasFine = ContextCompat.checkSelfPermission(context, Manifest.permission.ACCESS_FINE_LOCATION) == PackageManager.PERMISSION_GRANTED
        val hasCoarse = ContextCompat.checkSelfPermission(context, Manifest.permission.ACCESS_COARSE_LOCATION) == PackageManager.PERMISSION_GRANTED
        if (hasFine || hasCoarse) {
            captureGpsLocation()
        } else {
            permLauncher.launch(arrayOf(Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_COARSE_LOCATION))
        }
    }

    val presets = listOf(
        stringResource(R.string.saved_addresses_preset_home),
        stringResource(R.string.saved_addresses_preset_work),
        stringResource(R.string.saved_addresses_preset_parents),
    )

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
                stringResource(if (editing == null) R.string.saved_addresses_add else R.string.saved_addresses_edit),
                style = MaterialTheme.typography.titleLarge,
            )
            FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                presets.forEach { preset ->
                    FilterChip(
                        selected = label.trim() == preset,
                        onClick = { label = preset },
                        label = { Text(preset) },
                    )
                }
            }
            OutlinedTextField(
                value = label,
                onValueChange = { if (it.length <= 40) label = it },
                label = { Text(stringResource(R.string.saved_addresses_label)) },
                placeholder = { Text(stringResource(R.string.saved_addresses_label_hint)) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = address,
                onValueChange = {
                    if (it.length <= 200) {
                        address = it
                        if (it.isBlank()) {
                            lat = null
                            lng = null
                            gpsPinnedManually = false
                            coordsCleared = editing?.lat != null || editing?.lng != null
                            gpsStatus = "idle"
                        } else if (!gpsPinnedManually) {
                            val unchanged = editing != null && it.trim() == editing.address.trim()
                            if (unchanged) {
                                lat = editing?.lat
                                lng = editing?.lng
                                coordsCleared = false
                            } else {
                                lat = null
                                lng = null
                                coordsCleared = editing?.lat != null || editing?.lng != null
                            }
                        }
                        if (areaId == null && it.trim().length >= 3) {
                            areaId = inferAreaId(it, areas)
                        }
                        onAddressChange(it)
                    }
                },
                label = { Text(stringResource(R.string.saved_addresses_street)) },
                modifier = Modifier.fillMaxWidth(),
            )
            if (suggestions.isNotEmpty()) {
                OutlinedCard(Modifier.fillMaxWidth()) {
                    Column(Modifier.padding(Spacing.xs)) {
                        suggestions.take(4).forEach { s ->
                            TextButton(
                                onClick = {
                                    address = s.label.take(200)
                                    lat = s.lat
                                    lng = s.lng
                                    gpsPinnedManually = false
                                    coordsCleared = false
                                    gpsStatus = "idle"
                                    if (areaId == null) {
                                        areaId = inferAreaId(s.label, areas)
                                    }
                                    onPickSuggestion()
                                },
                                modifier = Modifier.fillMaxWidth(),
                            ) {
                                Text(s.label, style = MaterialTheme.typography.bodyMedium, maxLines = 2)
                            }
                        }
                    }
                }
            }
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                OutlinedButton(
                    onClick = { requestOrCaptureGps() },
                    enabled = !busy && gpsStatus != "locating",
                    modifier = Modifier.weight(1f).height(48.dp),
                ) {
                    Icon(Icons.Outlined.MyLocation, contentDescription = null)
                    Text(
                        stringResource(
                            if (gpsStatus == "locating") R.string.saved_addresses_locating else R.string.saved_addresses_use_gps,
                        ),
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.padding(start = Spacing.xs),
                    )
                }
                OutlinedButton(
                    onClick = { showPinPicker = true },
                    enabled = !busy,
                    modifier = Modifier.weight(1f).height(48.dp),
                ) {
                    Icon(Icons.Outlined.Place, contentDescription = null)
                    Text(
                        stringResource(R.string.pin_picker_btn),
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.padding(start = Spacing.xs),
                    )
                }
            }
            if (showPinPicker) {
                LocationPinPickerSheet(
                    initialLat = lat,
                    initialLng = lng,
                    onDismiss = { showPinPicker = false },
                    onConfirm = { pickedLat, pickedLng ->
                        lat = pickedLat
                        lng = pickedLng
                        gpsPinnedManually = true
                        coordsCleared = false
                        gpsStatus = "idle"
                    },
                )
            }
            if (gpsStatus == "captured" || (gpsStatus != "denied" && lat != null && lng != null)) {
                Row(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        stringResource(
                            if (gpsStatus == "captured") R.string.saved_addresses_gps_captured
                            else R.string.saved_addresses_pinned,
                        ),
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.primary,
                    )
                    TextButton(
                        onClick = {
                            lat = null
                            lng = null
                            gpsPinnedManually = false
                            coordsCleared = true
                            gpsStatus = "idle"
                        },
                    ) {
                        Text(stringResource(R.string.saved_addresses_gps_clear))
                    }
                }
            } else if (gpsStatus == "denied") {
                Text(
                    stringResource(R.string.saved_addresses_gps_denied),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            ExposedDropdownMenuBox(expanded = areaExpanded, onExpandedChange = { areaExpanded = it }) {
                OutlinedTextField(
                    value = areas.firstOrNull { it.id == areaId }?.name ?: "—",
                    onValueChange = {},
                    readOnly = true,
                    label = { Text(stringResource(R.string.saved_addresses_area)) },
                    supportingText = { Text(stringResource(R.string.saved_addresses_area_hint)) },
                    trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = areaExpanded) },
                    modifier = Modifier.menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
                )
                ExposedDropdownMenu(expanded = areaExpanded, onDismissRequest = { areaExpanded = false }) {
                    DropdownMenuItem(
                        text = { Text("—") },
                        onClick = { areaId = null; areaExpanded = false },
                    )
                    areas.forEach { area ->
                        DropdownMenuItem(
                            text = { Text(area.name) },
                            onClick = { areaId = area.id; areaExpanded = false },
                        )
                    }
                }
            }
            OutlinedTextField(
                value = gateNotes,
                onValueChange = { if (it.length <= 300) gateNotes = it },
                label = { Text(stringResource(R.string.saved_addresses_gate_notes)) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier
                    .fillMaxWidth()
                    .toggleable(value = isDefault, role = Role.Checkbox, onValueChange = { isDefault = it })
                    .padding(vertical = Spacing.xxs),
            ) {
                Checkbox(checked = isDefault, onCheckedChange = null)
                Text(
                    stringResource(R.string.saved_addresses_use_default),
                    style = MaterialTheme.typography.bodyMedium,
                    modifier = Modifier.padding(start = Spacing.xs),
                )
            }
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
                    enabled = !busy && label.isNotBlank() && address.isNotBlank(),
                    onClick = {
                        UiHaptics.confirm(view)
                        onSave(label, areaId, address, gateNotes, isDefault, lat, lng, coordsCleared)
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
            if (editing != null && canDelete) {
                TextButton(
                    onClick = { confirmDelete = true },
                    enabled = !busy,
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text(stringResource(R.string.saved_addresses_remove))
                }
            }
        }
    }

    if (confirmDelete && editing != null) {
        AlertDialog(
            onDismissRequest = { confirmDelete = false },
            title = { Text(stringResource(R.string.saved_addresses_remove)) },
            text = { Text(stringResource(R.string.saved_addresses_remove_confirm)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmDelete = false
                        onDelete(editing.id)
                    },
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                ) {
                    Text(stringResource(R.string.saved_addresses_remove))
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
