package org.pjstatecm.graceride.ui.screens.profile

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.LocationOn
import androidx.compose.material.icons.outlined.StarOutline
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MenuAnchorType
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.Area
import org.pjstatecm.graceride.data.api.Place
import org.pjstatecm.graceride.data.api.SavedPlace
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.theme.Spacing

private const val MAX_SAVED_PLACES = 50

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SavedAddressesScreen(container: AppContainer, nav: NavHostController) {
    val vm: SavedAddressesViewModel = viewModel(factory = SavedAddressesViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val snackbar = remember { SnackbarHostState() }

    val savedMsg = stringResource(R.string.saved_addresses_saved)
    val defaultMsg = stringResource(R.string.saved_addresses_default_updated)
    val removedMsg = stringResource(R.string.saved_addresses_removed)

    var showDialog by remember { mutableStateOf(false) }
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
        showDialog = true
    }

    val openEdit: (SavedPlace) -> Unit = { place ->
        vm.clearError()
        vm.clearSuggestions()
        editing = place
        showDialog = true
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
                    if (state.places.size < MAX_SAVED_PLACES) {
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
            state.loading && state.places.isEmpty() -> LoadingView(Modifier.padding(padding))
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
                EmptyView(stringResource(R.string.saved_addresses_empty))
                Button(onClick = openCreate, modifier = Modifier.padding(top = Spacing.xs)) {
                    Text(stringResource(R.string.saved_addresses_add))
                }
            }
            else -> LazyColumn(
                Modifier.fillMaxSize().padding(padding).padding(horizontal = Spacing.sm),
                contentPadding = PaddingValues(vertical = Spacing.sm),
                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
            ) {
                item {
                    OutlinedCard(Modifier.fillMaxWidth()) {
                        Column {
                            state.places.forEachIndexed { idx, place ->
                                if (idx > 0) HorizontalDivider()
                                SavedAddressRow(
                                    place = place,
                                    busy = state.busy,
                                    onEdit = { openEdit(place) },
                                    onSetDefault = { vm.setDefault(place) },
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

    if (showDialog) {
        SavedAddressDialog(
            editing = editing,
            areas = state.areas,
            suggestions = state.suggestions,
            defaultInitial = editing?.isDefault == 1 || (editing == null && state.places.isEmpty()),
            busy = state.busy,
            error = state.error,
            onAddressChange = vm::searchAddress,
            onPickSuggestion = { vm.clearSuggestions() },
            onDismiss = {
                vm.clearSuggestions()
                showDialog = false
                editing = null
            },
            onSave = { label, areaId, address, gateNotes, isDefault ->
                vm.save(
                    editingId = editing?.id,
                    label = label,
                    areaId = areaId,
                    address = address,
                    gateNotes = gateNotes,
                    isDefault = isDefault,
                ) {
                    showDialog = false
                    editing = null
                }
            },
            onDelete = { id ->
                vm.delete(id) {
                    showDialog = false
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
    onEdit: () -> Unit,
    onSetDefault: () -> Unit,
) {
    val isDefault = place.isDefault == 1
    Row(
        Modifier
            .fillMaxWidth()
            .clickable(enabled = !busy, onClick = onEdit)
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
                    Text(
                        stringResource(R.string.saved_addresses_default),
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.primary,
                    )
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
        }
        IconButton(
            onClick = onSetDefault,
            enabled = !busy && !isDefault,
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
private fun SavedAddressDialog(
    editing: SavedPlace?,
    areas: List<Area>,
    suggestions: List<Place>,
    defaultInitial: Boolean,
    busy: Boolean,
    error: String?,
    onAddressChange: (String) -> Unit,
    onPickSuggestion: () -> Unit,
    onDismiss: () -> Unit,
    onSave: (label: String, areaId: Long?, address: String, gateNotes: String, isDefault: Boolean) -> Unit,
    onDelete: (Long) -> Unit,
) {
    var label by remember(editing) { mutableStateOf(editing?.label ?: "") }
    var areaId by remember(editing) { mutableStateOf(editing?.areaId) }
    var address by remember(editing) { mutableStateOf(editing?.address ?: "") }
    var gateNotes by remember(editing) { mutableStateOf(editing?.gateNotes ?: "") }
    var isDefault by remember(editing) { mutableStateOf(defaultInitial) }
    var areaExpanded by remember { mutableStateOf(false) }
    var confirmDelete by remember { mutableStateOf(false) }

    val presets = listOf(
        stringResource(R.string.saved_addresses_preset_home),
        stringResource(R.string.saved_addresses_preset_work),
        stringResource(R.string.saved_addresses_preset_parents),
    )

    AlertDialog(
        onDismissRequest = onDismiss,
        title = {
            Text(
                stringResource(
                    if (editing == null) R.string.saved_addresses_add else R.string.saved_addresses_edit,
                ),
            )
        },
        text = {
            Column(
                Modifier.verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
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
                ExposedDropdownMenuBox(expanded = areaExpanded, onExpandedChange = { areaExpanded = it }) {
                    OutlinedTextField(
                        value = areas.firstOrNull { it.id == areaId }?.name ?: "—",
                        onValueChange = {},
                        readOnly = true,
                        label = { Text(stringResource(R.string.saved_addresses_area)) },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = areaExpanded) },
                        modifier = Modifier.menuAnchor(MenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
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
                        .toggleable(value = isDefault, onValueChange = { isDefault = it })
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
                if (editing != null) {
                    TextButton(
                        onClick = { confirmDelete = true },
                        enabled = !busy,
                        colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                    ) {
                        Text(stringResource(R.string.saved_addresses_remove))
                    }
                }
            }
        },
        confirmButton = {
            Button(
                enabled = !busy && label.isNotBlank() && address.isNotBlank(),
                onClick = { onSave(label, areaId, address, gateNotes, isDefault) },
            ) {
                Text(stringResource(R.string.common_save))
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text(stringResource(R.string.common_cancel))
            }
        },
    )

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
