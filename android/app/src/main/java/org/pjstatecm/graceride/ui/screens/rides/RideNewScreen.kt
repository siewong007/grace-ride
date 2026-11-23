package org.pjstatecm.graceride.ui.screens.rides

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MenuAnchorType
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TimePicker
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.material3.rememberTimePickerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.theme.Spacing
import java.time.Instant
import java.time.ZoneId

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun RideNewScreen(
    container: AppContainer,
    nav: NavHostController,
    eventId: Long?,
    initialProxy: Boolean = false,
) {
    val vm: RideNewViewModel = viewModel(factory = RideNewViewModel.factory(container, eventId, initialProxy))
    val s by vm.state.collectAsStateWithLifecycle()
    val lifecycleOwner = LocalLifecycleOwner.current

    DisposableEffect(lifecycleOwner) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) {
                vm.refreshPlacesAndProxies()
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }

    // Rides exist only for events: without one, point the member at the events
    // list, exactly like the web's /rides/new without ?event=.
    if (eventId == null) {
        Column(
            Modifier.fillMaxSize().padding(Spacing.md),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(stringResource(R.string.ride_new_event_first_title), style = MaterialTheme.typography.titleLarge, color = MaterialTheme.colorScheme.onSurface)
            Text(
                stringResource(R.string.ride_new_event_first_desc),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = Spacing.xs),
            )
            Button(onClick = { nav.navigate(Routes.EVENTS) }, modifier = Modifier.padding(top = Spacing.sm)) {
                Text(stringResource(R.string.ride_new_browse_events))
            }
        }
        return
    }

    var showDate by remember { mutableStateOf(false) }
    var showTime by remember { mutableStateOf(false) }
    var showReturnDate by remember { mutableStateOf(false) }
    var showReturnTime by remember { mutableStateOf(false) }
    var areaExpanded by remember { mutableStateOf(false) }
    var relationshipExpanded by remember { mutableStateOf(false) }

    LaunchedEffect(s.createdId) {
        s.createdId?.let {
            nav.navigate(Routes.rideDetail(it)) {
                popUpTo(Routes.RIDE_NEW) { inclusive = true }
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.ride_new_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
            )
        },
    ) { padding ->
        val ev = s.event
        when {
            s.loading -> LoadingView(Modifier.padding(padding))
            ev != null && ev.mySignup?.status != "going" -> {
                Column(
                    Modifier.fillMaxSize().padding(padding).padding(Spacing.md),
                    verticalArrangement = Arrangement.Center,
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    Text(
                        stringResource(R.string.ride_new_signup_required_title),
                        style = MaterialTheme.typography.titleLarge,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    Text(
                        stringResource(R.string.ride_new_signup_required_desc, ev.title),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(top = Spacing.xs),
                    )
                    Button(
                        onClick = { nav.navigate(Routes.eventDetail(ev.id)) },
                        modifier = Modifier.padding(top = Spacing.sm),
                    ) {
                        Text(stringResource(R.string.ride_new_go_to_event))
                    }
                }
            }
            else -> Column(
                Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState()).padding(Spacing.sm),
                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
            ) {
                // The event this ride belongs to (fixed; chosen on the event page).
                s.event?.let { event ->
                    OutlinedCard(Modifier.fillMaxWidth()) {
                        Column(Modifier.padding(Spacing.sm)) {
                            Text(stringResource(R.string.ride_new_event_label), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Text(event.title, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.onSurface)
                            Text(KlTime.display(event.startsAt), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.primary)
                        }
                    }
                }

                // Trip direction selector
                Text(stringResource(R.string.ride_new_trip_type_label), style = MaterialTheme.typography.titleSmall)
                val tripOptions = listOf(
                    TripType.OUTBOUND to stringResource(R.string.ride_new_trip_type_outbound),
                    TripType.ROUND to stringResource(R.string.ride_new_trip_type_round),
                    TripType.RETURN to stringResource(R.string.ride_new_trip_type_return),
                )
                SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
                    tripOptions.forEachIndexed { idx, (type, label) ->
                        SegmentedButton(
                            selected = s.tripType == type,
                            onClick = { vm.setTripType(type) },
                            shape = SegmentedButtonDefaults.itemShape(index = idx, count = tripOptions.size),
                            label = { Text(label, maxLines = 1) },
                        )
                    }
                }

                // Pickup date & time
                Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    OutlinedButton(onClick = { showDate = true }, modifier = Modifier.weight(1f)) {
                        Text(s.date.toString())
                    }
                    OutlinedButton(onClick = { showTime = true }, modifier = Modifier.weight(1f)) {
                        Text("%02d:%02d".format(s.hour, s.minute))
                    }
                }
                if (s.tripType == TripType.RETURN) {
                    Text(
                        stringResource(R.string.ride_new_time_hint_return),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                if (s.tripType == TripType.ROUND) {
                    Text(stringResource(R.string.ride_new_return_time), style = MaterialTheme.typography.titleSmall)
                    Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        OutlinedButton(onClick = { showReturnDate = true }, modifier = Modifier.weight(1f)) { Text(s.returnDate.toString()) }
                        OutlinedButton(onClick = { showReturnTime = true }, modifier = Modifier.weight(1f)) {
                            Text("%02d:%02d".format(s.returnHour, s.returnMinute))
                        }
                    }
                }

                // Family / Proxy booking section
                OutlinedCard(Modifier.fillMaxWidth()) {
                    Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween,
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Text(stringResource(R.string.ride_new_is_proxy), style = MaterialTheme.typography.titleSmall)
                            Switch(
                                checked = s.isProxy,
                                onCheckedChange = { checked -> vm.update { it.copy(isProxy = checked) } },
                            )
                        }
                        if (s.isProxy) {
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.SpaceBetween,
                                modifier = Modifier.fillMaxWidth(),
                            ) {
                                Text(
                                    stringResource(R.string.ride_new_frequent_riders),
                                    style = MaterialTheme.typography.labelMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                                TextButton(onClick = { nav.navigate(Routes.PROXY_RIDERS) }) {
                                    Text(stringResource(R.string.common_manage))
                                }
                            }
                            if (s.proxyRiders.isNotEmpty()) {
                                FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                    s.proxyRiders.forEach { fr ->
                                        FilterChip(
                                            selected = s.riderName == fr.name,
                                            onClick = { vm.selectProxyRider(fr) },
                                            label = { Text(fr.name) },
                                        )
                                    }
                                }
                            }
                            OutlinedTextField(
                                value = s.riderName,
                                onValueChange = { v -> vm.update { it.copy(riderName = v) } },
                                label = { Text(stringResource(R.string.ride_new_rider_name)) },
                                singleLine = true,
                                modifier = Modifier.fillMaxWidth(),
                            )
                            OutlinedTextField(
                                value = s.riderPhone,
                                onValueChange = { v -> vm.update { it.copy(riderPhone = v) } },
                                label = { Text(stringResource(R.string.ride_new_rider_phone)) },
                                singleLine = true,
                                modifier = Modifier.fillMaxWidth(),
                            )
                            val relationships = listOf(
                                "parent" to stringResource(R.string.ride_new_relationship_parent),
                                "child" to stringResource(R.string.ride_new_relationship_child),
                                "spouse" to stringResource(R.string.ride_new_relationship_spouse),
                                "friend" to stringResource(R.string.ride_new_relationship_friend),
                                "other" to stringResource(R.string.ride_new_relationship_other),
                            )
                            ExposedDropdownMenuBox(
                                expanded = relationshipExpanded,
                                onExpandedChange = { relationshipExpanded = it },
                            ) {
                                OutlinedTextField(
                                    value = relationships.firstOrNull { it.first == s.riderRelationship }?.second
                                        ?: s.riderRelationship,
                                    onValueChange = {},
                                    readOnly = true,
                                    label = { Text(stringResource(R.string.ride_new_rider_relationship)) },
                                    trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = relationshipExpanded) },
                                    modifier = Modifier.menuAnchor(MenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
                                )
                                ExposedDropdownMenu(
                                    expanded = relationshipExpanded,
                                    onDismissRequest = { relationshipExpanded = false },
                                ) {
                                    relationships.forEach { (key, label) ->
                                        DropdownMenuItem(
                                            text = { Text(label) },
                                            onClick = {
                                                vm.update { it.copy(riderRelationship = key) }
                                                relationshipExpanded = false
                                            },
                                        )
                                    }
                                }
                            }
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .toggleable(
                                        value = s.saveProxyRider,
                                        onValueChange = { checked -> vm.update { it.copy(saveProxyRider = checked) } },
                                    ),
                            ) {
                                Checkbox(checked = s.saveProxyRider, onCheckedChange = null)
                                Text(
                                    stringResource(R.string.ride_new_save_proxy),
                                    style = MaterialTheme.typography.bodyMedium,
                                    modifier = Modifier.padding(start = Spacing.xs),
                                )
                            }
                        }
                    }
                }

                if (!s.isProxy) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Text(
                            stringResource(R.string.ride_new_saved_addresses),
                            style = MaterialTheme.typography.titleSmall,
                        )
                        TextButton(onClick = { nav.navigate(Routes.SAVED_ADDRESSES) }) {
                            Text(stringResource(R.string.common_manage))
                        }
                    }
                    if (s.savedPlaces.isNotEmpty()) {
                        val activeAddr = if (s.tripType == TripType.RETURN) s.destination else s.address
                        FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            s.savedPlaces.forEach { sp ->
                                FilterChip(
                                    selected = activeAddr == sp.address,
                                    onClick = { vm.applySavedPlace(sp) },
                                    label = { Text(sp.label) },
                                )
                            }
                        }
                    }
                }

                // Pickup area
                ExposedDropdownMenuBox(expanded = areaExpanded, onExpandedChange = { areaExpanded = it }) {
                    OutlinedTextField(
                        value = s.areas.firstOrNull { it.id == s.areaId }?.name ?: "",
                        onValueChange = {},
                        readOnly = true,
                        label = { Text(stringResource(R.string.ride_new_area)) },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = areaExpanded) },
                        modifier = Modifier.menuAnchor(MenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
                    )
                    ExposedDropdownMenu(expanded = areaExpanded, onDismissRequest = { areaExpanded = false }) {
                        s.areas.forEach { area ->
                            DropdownMenuItem(
                                text = { Text(area.name) },
                                onClick = { vm.update { it.copy(areaId = area.id) }; areaExpanded = false },
                            )
                        }
                    }
                }

                // Pickup address with geocoder suggestions
                OutlinedTextField(
                    value = s.address,
                    onValueChange = vm::onAddressChange,
                    label = { Text(stringResource(R.string.ride_new_address)) },
                    modifier = Modifier.fillMaxWidth(),
                )
                if (s.suggestions.isNotEmpty()) {
                    OutlinedCard(Modifier.fillMaxWidth()) {
                        Column(Modifier.padding(Spacing.xs)) {
                            s.suggestions
                                .distinctBy { it.label }
                                .take(5)
                                .forEach { place ->
                                    TextButton(onClick = { vm.pickSuggestion(place) }, modifier = Modifier.fillMaxWidth()) {
                                        Text(place.label, style = MaterialTheme.typography.bodyMedium, maxLines = 2)
                                    }
                                }
                        }
                    }
                }

                OutlinedTextField(
                    value = s.destination,
                    onValueChange = { v -> vm.update { it.copy(destination = v) } },
                    label = { Text(stringResource(R.string.ride_new_destination)) },
                    modifier = Modifier.fillMaxWidth(),
                )

                val memberAddr = (if (s.tripType == TripType.RETURN) s.destination else s.address).trim()
                val alreadySaved = s.savedPlaces.any { it.address.trim().equals(memberAddr, ignoreCase = true) }
                val matchesEvent = s.event?.location?.trim()?.equals(memberAddr, ignoreCase = true) == true
                if (!s.isProxy && memberAddr.isNotEmpty() && !alreadySaved && !matchesEvent && s.savedPlaces.size < 50) {
                    val presets = listOf(
                        stringResource(R.string.saved_addresses_preset_home),
                        stringResource(R.string.saved_addresses_preset_work),
                        stringResource(R.string.saved_addresses_preset_parents),
                    )
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier
                            .fillMaxWidth()
                            .toggleable(
                                value = s.saveAddress,
                                onValueChange = { checked ->
                                    vm.update { cur ->
                                        val nextLabel = if (checked && cur.saveAddressLabel.isBlank()) {
                                            val used = cur.savedPlaces.map { it.label.trim().lowercase() }.toSet()
                                            presets.firstOrNull { it.lowercase() !in used } ?: ""
                                        } else {
                                            cur.saveAddressLabel
                                        }
                                        cur.copy(saveAddress = checked, saveAddressLabel = nextLabel)
                                    }
                                },
                            ),
                    ) {
                        Checkbox(checked = s.saveAddress, onCheckedChange = null)
                        Text(
                            stringResource(R.string.ride_new_save_address),
                            style = MaterialTheme.typography.bodyMedium,
                            modifier = Modifier.padding(start = Spacing.xs),
                        )
                    }
                    if (s.saveAddress) {
                        FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            presets.forEach { preset ->
                                FilterChip(
                                    selected = s.saveAddressLabel.trim() == preset,
                                    onClick = { vm.update { it.copy(saveAddressLabel = preset) } },
                                    label = { Text(preset) },
                                )
                            }
                        }
                        OutlinedTextField(
                            value = s.saveAddressLabel,
                            onValueChange = { v -> if (v.length <= 40) vm.update { it.copy(saveAddressLabel = v) } },
                            label = { Text(stringResource(R.string.ride_new_save_address_label)) },
                            placeholder = { Text(stringResource(R.string.saved_addresses_label_hint)) },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth(),
                        )
                    }
                }

                // Seats stepper
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(Spacing.sm)) {
                    Text(stringResource(R.string.ride_new_seats), style = MaterialTheme.typography.bodyLarge)
                    OutlinedButton(onClick = { vm.update { it.copy(seats = (it.seats - 1).coerceAtLeast(1)) } }) { Text("−") }
                    Text(s.seats.toString(), style = MaterialTheme.typography.titleMedium)
                    OutlinedButton(onClick = { vm.update { it.copy(seats = (it.seats + 1).coerceAtMost(6)) } }) { Text("+") }
                }

                // Assistance chips (canonical backend enum)
                Text(stringResource(R.string.ride_new_assistance), style = MaterialTheme.typography.titleSmall)
                FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    listOf("" to "None", "wheelchair" to "Wheelchair", "stroller" to "Stroller", "oxygen" to "Oxygen", "other" to "Other")
                        .forEach { (value, label) ->
                            FilterChip(
                                selected = s.assistance == value,
                                onClick = { vm.update { it.copy(assistance = value) } },
                                label = { Text(label) },
                            )
                        }
                }

                OutlinedTextField(
                    value = s.notes,
                    onValueChange = { v -> vm.update { it.copy(notes = v) } },
                    label = { Text(stringResource(R.string.ride_new_notes)) },
                    modifier = Modifier.fillMaxWidth(),
                    minLines = 2,
                )

                s.error?.let {
                    Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium)
                }

                Button(
                    onClick = vm::submit,
                    enabled = !s.submitting,
                    modifier = Modifier.fillMaxWidth().padding(top = Spacing.xs),
                ) { Text(stringResource(R.string.ride_new_submit)) }
            }
        }
    }

    if (showDate) {
        val pickerState = rememberDatePickerState(initialSelectedDateMillis = s.date.atStartOfDay(ZoneId.systemDefault()).toInstant().toEpochMilli())
        DatePickerDialog(
            onDismissRequest = { showDate = false },
            confirmButton = {
                TextButton(onClick = {
                    pickerState.selectedDateMillis?.let { millis ->
                        vm.update { it.copy(date = Instant.ofEpochMilli(millis).atZone(ZoneId.systemDefault()).toLocalDate()) }
                    }
                    showDate = false
                }) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = { TextButton(onClick = { showDate = false }) { Text(stringResource(R.string.common_cancel)) } },
        ) { DatePicker(state = pickerState) }
    }
    if (showTime) {
        TimePickerDialog(
            title = stringResource(R.string.ride_new_pickup_time),
            initialHour = s.hour, initialMinute = s.minute,
            onConfirm = { h, m -> vm.update { it.copy(hour = h, minute = m) }; showTime = false },
            onDismiss = { showTime = false },
        )
    }
    if (showReturnDate) {
        val pickerState = rememberDatePickerState(initialSelectedDateMillis = s.returnDate.atStartOfDay(ZoneId.systemDefault()).toInstant().toEpochMilli())
        DatePickerDialog(
            onDismissRequest = { showReturnDate = false },
            confirmButton = {
                TextButton(onClick = {
                    pickerState.selectedDateMillis?.let { millis ->
                        vm.update { it.copy(returnDate = Instant.ofEpochMilli(millis).atZone(ZoneId.systemDefault()).toLocalDate()) }
                    }
                    showReturnDate = false
                }) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = { TextButton(onClick = { showReturnDate = false }) { Text(stringResource(R.string.common_cancel)) } },
        ) { DatePicker(state = pickerState) }
    }
    if (showReturnTime) {
        TimePickerDialog(
            title = stringResource(R.string.ride_new_return_time),
            initialHour = s.returnHour, initialMinute = s.returnMinute,
            onConfirm = { h, m -> vm.update { it.copy(returnHour = h, returnMinute = m) }; showReturnTime = false },
            onDismiss = { showReturnTime = false },
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun TimePickerDialog(
    title: String,
    initialHour: Int,
    initialMinute: Int,
    onConfirm: (Int, Int) -> Unit,
    onDismiss: () -> Unit,
) {
    val state = rememberTimePickerState(initialHour = initialHour, initialMinute = initialMinute, is24Hour = true)
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = { TimePicker(state = state) },
        confirmButton = { TextButton(onClick = { onConfirm(state.hour, state.minute) }) { Text(stringResource(R.string.common_confirm)) } },
        dismissButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.common_cancel)) } },
    )
}
