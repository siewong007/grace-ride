package org.pjstatecm.graceride.ui.screens.profile

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.Groups
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
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
import org.pjstatecm.graceride.data.api.ProxyRider
import org.pjstatecm.graceride.data.api.ProxyRiderReq
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.ProxyRelationships
import org.pjstatecm.graceride.domain.inferAreaId
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.theme.Spacing

private const val MAX_PROXY_RIDERS = 50

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProxyRidersScreen(container: AppContainer, nav: NavHostController) {
    val vm: ProxyRidersViewModel = viewModel(factory = ProxyRidersViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val snackbar = remember { SnackbarHostState() }

    val savedMsg = stringResource(R.string.proxy_riders_saved)
    val removedMsg = stringResource(R.string.proxy_riders_removed)

    var showDialog by remember { mutableStateOf(false) }
    var editing by remember { mutableStateOf<ProxyRider?>(null) }

    LaunchedEffect(state.message) {
        when (state.message) {
            "saved" -> snackbar.showSnackbar(savedMsg)
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

    val openEdit: (ProxyRider) -> Unit = { rider ->
        vm.clearError()
        vm.clearSuggestions()
        editing = rider
        showDialog = true
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.proxy_riders_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(
                            Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = stringResource(R.string.a11y_back),
                        )
                    }
                },
                actions = {
                    if (state.riders.size < MAX_PROXY_RIDERS) {
                        IconButton(onClick = openCreate) {
                            Icon(
                                Icons.Outlined.Add,
                                contentDescription = stringResource(R.string.proxy_riders_add),
                            )
                        }
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        when {
            state.loading && state.riders.isEmpty() -> LoadingView(Modifier.padding(padding))
            state.error != null && state.riders.isEmpty() -> ErrorView(
                message = state.error!!,
                onRetry = vm::load,
                modifier = Modifier.padding(padding),
            )
            state.riders.isEmpty() -> Column(
                Modifier.fillMaxSize().padding(padding).padding(Spacing.md),
                verticalArrangement = Arrangement.Center,
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                EmptyView(stringResource(R.string.proxy_riders_empty))
                Button(onClick = openCreate, modifier = Modifier.padding(top = Spacing.xs)) {
                    Text(stringResource(R.string.proxy_riders_add))
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
                            val areaNames = state.areas.associate { it.id to it.name }
                            state.riders.forEachIndexed { idx, rider ->
                                if (idx > 0) HorizontalDivider()
                                Row(
                                    Modifier
                                        .fillMaxWidth()
                                        .clickable(enabled = !state.busy) { openEdit(rider) }
                                        .padding(horizontal = Spacing.sm, vertical = Spacing.sm),
                                    verticalAlignment = Alignment.Top,
                                ) {
                                    Icon(
                                        Icons.Outlined.Groups,
                                        contentDescription = null,
                                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
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
                                            Text(rider.name, style = MaterialTheme.typography.titleMedium)
                                            if (rider.relationship.isNotBlank()) {
                                                val relLabel = ProxyRelationships.labelRes(rider.relationship)
                                                    ?.let { stringResource(it) }
                                                    ?: rider.relationship
                                                Text(
                                                    relLabel,
                                                    style = MaterialTheme.typography.labelMedium,
                                                    color = MaterialTheme.colorScheme.primary,
                                                )
                                            }
                                            val areaName = rider.areaId?.let(areaNames::get)
                                            if (!areaName.isNullOrBlank()) {
                                                Text(
                                                    "· $areaName",
                                                    style = MaterialTheme.typography.bodySmall,
                                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                )
                                            }
                                        }
                                        if (rider.phone.isNotBlank()) {
                                            Text(
                                                rider.phone,
                                                style = MaterialTheme.typography.bodySmall,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                            )
                                        }
                                        if (rider.pickupAddress.isNotBlank()) {
                                            Text(
                                                rider.pickupAddress,
                                                style = MaterialTheme.typography.bodyMedium,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                maxLines = 2,
                                                overflow = TextOverflow.Ellipsis,
                                            )
                                        }
                                        val extra = listOf(rider.gateNotes, rider.notes)
                                            .filter { it.isNotBlank() }
                                            .joinToString(" · ")
                                        if (extra.isNotBlank()) {
                                            Text(
                                                extra,
                                                style = MaterialTheme.typography.bodySmall,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
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
    }

    if (showDialog) {
        ProxyRiderDialog(
            editing = editing,
            areas = state.areas,
            suggestions = state.suggestions,
            busy = state.busy,
            error = state.error,
            onAddressChange = vm::searchAddress,
            onPickSuggestion = { vm.clearSuggestions() },
            onDismiss = {
                vm.clearSuggestions()
                showDialog = false
                editing = null
            },
            onSave = { req ->
                vm.save(editing?.id, req) {
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

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ProxyRiderDialog(
    editing: ProxyRider?,
    areas: List<Area>,
    suggestions: List<Place>,
    busy: Boolean,
    error: String?,
    onAddressChange: (String) -> Unit,
    onPickSuggestion: () -> Unit,
    onDismiss: () -> Unit,
    onSave: (ProxyRiderReq) -> Unit,
    onDelete: (Long) -> Unit,
) {
    var name by remember(editing) { mutableStateOf(editing?.name ?: "") }
    var relationship by remember(editing) { mutableStateOf(editing?.relationship?.ifBlank { "parent" } ?: "parent") }
    var phone by remember(editing) { mutableStateOf(editing?.phone ?: "") }
    var pickupAddress by remember(editing) { mutableStateOf(editing?.pickupAddress ?: "") }
    var areaId by remember(editing) { mutableStateOf(editing?.areaId) }
    var gateNotes by remember(editing) { mutableStateOf(editing?.gateNotes ?: "") }
    var notes by remember(editing) { mutableStateOf(editing?.notes ?: "") }
    var relExpanded by remember { mutableStateOf(false) }
    var areaExpanded by remember { mutableStateOf(false) }
    var confirmDelete by remember { mutableStateOf(false) }

    val relOptions = ProxyRelationships.keys.map { key ->
        key to (ProxyRelationships.labelRes(key)?.let { stringResource(it) } ?: key)
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = {
            Text(
                stringResource(
                    if (editing == null) R.string.proxy_riders_add else R.string.proxy_riders_edit,
                ),
            )
        },
        text = {
            Column(
                Modifier.verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                OutlinedTextField(
                    value = name,
                    onValueChange = { if (it.length <= 80) name = it },
                    label = { Text(stringResource(R.string.ride_new_rider_name)) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                ExposedDropdownMenuBox(expanded = relExpanded, onExpandedChange = { relExpanded = it }) {
                    OutlinedTextField(
                        value = relOptions.firstOrNull { it.first == relationship }?.second ?: relationship,
                        onValueChange = {},
                        readOnly = true,
                        label = { Text(stringResource(R.string.ride_new_rider_relationship)) },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = relExpanded) },
                        modifier = Modifier.menuAnchor(MenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
                    )
                    ExposedDropdownMenu(expanded = relExpanded, onDismissRequest = { relExpanded = false }) {
                        relOptions.forEach { (key, label) ->
                            DropdownMenuItem(
                                text = { Text(label) },
                                onClick = { relationship = key; relExpanded = false },
                            )
                        }
                    }
                }
                OutlinedTextField(
                    value = phone,
                    onValueChange = { phone = it },
                    label = { Text(stringResource(R.string.ride_new_rider_phone)) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                OutlinedTextField(
                    value = pickupAddress,
                    onValueChange = {
                        if (it.length <= 200) {
                            pickupAddress = it
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
                                        pickupAddress = s.label.take(200)
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
                OutlinedTextField(
                    value = notes,
                    onValueChange = { if (it.length <= 500) notes = it },
                    label = { Text(stringResource(R.string.proxy_riders_notes)) },
                    modifier = Modifier.fillMaxWidth(),
                )
                error?.let {
                    Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium)
                }
                if (editing != null) {
                    TextButton(
                        onClick = { confirmDelete = true },
                        enabled = !busy,
                        colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                    ) {
                        Text(stringResource(R.string.proxy_riders_remove))
                    }
                }
            }
        },
        confirmButton = {
            Button(
                enabled = !busy && name.isNotBlank(),
                onClick = {
                    onSave(
                        ProxyRiderReq(
                            name = name,
                            phone = phone,
                            relationship = relationship,
                            notes = notes,
                            area_id = areaId,
                            pickup_address = pickupAddress,
                            gate_notes = gateNotes,
                        ),
                    )
                },
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
            title = { Text(stringResource(R.string.proxy_riders_remove)) },
            text = { Text(stringResource(R.string.proxy_riders_remove_confirm)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmDelete = false
                        onDelete(editing.id)
                    },
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                ) {
                    Text(stringResource(R.string.proxy_riders_remove))
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
