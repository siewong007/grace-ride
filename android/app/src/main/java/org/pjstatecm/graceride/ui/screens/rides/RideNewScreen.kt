package org.pjstatecm.graceride.ui.screens.rides

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.Accessible
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.Air
import androidx.compose.material.icons.outlined.CalendarToday
import androidx.compose.material.icons.outlined.ChildFriendly
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Event
import androidx.compose.material.icons.outlined.LocationOn
import androidx.compose.material.icons.outlined.Place
import androidx.compose.material.icons.outlined.Remove
import androidx.compose.material.icons.outlined.Schedule
import androidx.compose.material.icons.outlined.WarningAmber
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
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
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedIconButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Surface
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
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.DatePickerUtc
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.ProxyRelationships
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.DetailSkeleton
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.EventIcon
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.navigateTab
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.tabular

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun RideNewScreen(
    container: AppContainer,
    nav: NavHostController,
    eventId: Long?,
    initialProxy: Boolean = false,
) {
    // Rides exist only for events: without one, point the member at the events
    // list, exactly like the web's /rides/new without ?event=.
    if (eventId == null) {
        val authState by container.auth.state.collectAsStateWithLifecycle()
        val me = (authState as? org.pjstatecm.graceride.data.repo.AuthState.SignedIn)?.me ?: container.auth.me
        val canViewEvents = me == null || me.can("events", "view")
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
            Column(
                Modifier.fillMaxSize().padding(padding).padding(Spacing.md),
                verticalArrangement = Arrangement.Center,
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                EmptyView(
                    text = stringResource(R.string.ride_new_event_first_title),
                    icon = Icons.Outlined.Event,
                    subtitle = stringResource(R.string.ride_new_event_first_desc),
                    actionLabel = if (canViewEvents) stringResource(R.string.ride_new_browse_events) else null,
                    onAction = if (canViewEvents) { { nav.navigateTab(AppRoute.Events) } } else null,
                )
            }
        }
        return
    }

    val vm: RideNewViewModel = viewModel(
        key = "ride-new-$eventId-$initialProxy",
        factory = RideNewViewModel.factory(container, eventId, initialProxy),
    )
    val s by vm.state.collectAsStateWithLifecycle()
    val view = LocalView.current
    val lifecycleOwner = LocalLifecycleOwner.current
    var resumedOnce by remember { mutableStateOf(false) }

    DisposableEffect(lifecycleOwner) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) {
                if (resumedOnce) vm.refreshPlacesAndProxies() else resumedOnce = true
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }

    var showDate by remember { mutableStateOf(false) }
    var showTime by remember { mutableStateOf(false) }
    var showReturnDate by remember { mutableStateOf(false) }
    var showReturnTime by remember { mutableStateOf(false) }
    var areaExpanded by remember { mutableStateOf(false) }
    var relationshipExpanded by remember { mutableStateOf(false) }

    LaunchedEffect(s.createdId) {
        s.createdId?.let {
            nav.navigate(AppRoute.RideDetail(it, justBooked = true)) {
                popUpTo<AppRoute.RideNew> { inclusive = true }
            }
        }
    }

    val ev = s.event
    val showForm = !s.loading && !(s.loadError != null && ev == null) && !(ev != null && ev.mySignup?.status != "going")
    val errMsg = s.error ?: s.errorRes?.let { stringResource(it) }

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
        bottomBar = {
            if (showForm) {
                Surface(color = MaterialTheme.colorScheme.surface) {
                    Column(
                        Modifier
                            .fillMaxWidth()
                            .navigationBarsPadding()
                            .imePadding(),
                    ) {
                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                        Column(
                            Modifier.fillMaxWidth().padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            errMsg?.let {
                                CalloutBanner(
                                    icon = Icons.Outlined.WarningAmber,
                                    text = it,
                                    containerColor = MaterialTheme.colorScheme.errorContainer,
                                    contentColor = MaterialTheme.colorScheme.onErrorContainer,
                                )
                            }
                            Button(
                                onClick = {
                                    UiHaptics.confirm(view)
                                    vm.submit()
                                },
                                enabled = !s.submitting,
                                modifier = Modifier.fillMaxWidth().height(52.dp),
                            ) {
                                if (s.submitting) {
                                    InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                } else {
                                    Icon(Icons.Outlined.DirectionsCar, contentDescription = null, modifier = Modifier.size(18.dp))
                                    Spacer(Modifier.width(Spacing.xs))
                                    Text(stringResource(R.string.ride_new_submit))
                                }
                            }
                        }
                    }
                }
            }
        },
    ) { padding ->
        when {
            s.loading -> DetailSkeleton(Modifier.padding(padding))
            s.loadError != null && ev == null -> ErrorView(s.loadError ?: "", modifier = Modifier.padding(padding), onRetry = vm::retryLoad)
            ev != null && ev.mySignup?.status != "going" -> {
                Column(
                    Modifier.fillMaxSize().padding(padding).padding(Spacing.md),
                    verticalArrangement = Arrangement.Center,
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    EmptyView(
                        text = stringResource(R.string.ride_new_signup_required_title),
                        icon = Icons.Outlined.Event,
                        subtitle = stringResource(R.string.ride_new_signup_required_desc, ev.title),
                        actionLabel = stringResource(R.string.ride_new_go_to_event),
                        onAction = { nav.navigate(AppRoute.EventDetail(ev.id)) },
                    )
                }
            }
            else -> Column(
                Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .verticalScroll(rememberScrollState())
                    .padding(Spacing.sm),
                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
            ) {
                // 1. Linked Event Card
                s.event?.let { event ->
                    OutlinedCard(Modifier.fillMaxWidth()) {
                        Row(
                            Modifier.padding(Spacing.sm),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(Spacing.sm),
                        ) {
                            EventIcon(icon = event.icon, kind = event.kind, size = 30.dp)
                            Column(Modifier.weight(1f)) {
                                Text(
                                    stringResource(R.string.ride_new_event_label),
                                    style = MaterialTheme.typography.labelMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                                Text(event.title, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.onSurface)
                                Text(
                                    KlTime.display(event.startsAt),
                                    style = MaterialTheme.typography.bodyMedium.tabular(),
                                    color = MaterialTheme.colorScheme.primary,
                                )
                            }
                        }
                    }
                }

                // 2. Trip direction & schedule
                OutlinedCard(Modifier.fillMaxWidth()) {
                    Column(
                        Modifier.padding(Spacing.sm),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        Text(stringResource(R.string.ride_new_section_schedule), style = MaterialTheme.typography.titleSmall)
                        val tripOptions = listOf(
                            TripType.OUTBOUND to stringResource(R.string.ride_new_trip_type_outbound),
                            TripType.ROUND to stringResource(R.string.ride_new_trip_type_round),
                            TripType.RETURN to stringResource(R.string.ride_new_trip_type_return),
                        )
                        SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
                            tripOptions.forEachIndexed { idx, (type, label) ->
                                SegmentedButton(
                                    selected = s.tripType == type,
                                    onClick = {
                                        UiHaptics.tick(view)
                                        vm.setTripType(type)
                                    },
                                    shape = SegmentedButtonDefaults.itemShape(index = idx, count = tripOptions.size),
                                    label = { Text(label, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                                )
                            }
                        }

                        Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            OutlinedButton(
                                onClick = { showDate = true },
                                modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                            ) {
                                Icon(Icons.Outlined.CalendarToday, contentDescription = null, modifier = Modifier.size(16.dp))
                                Spacer(Modifier.width(6.dp))
                                Text(KlTime.displayDate(s.date.toString()), maxLines = 1, overflow = TextOverflow.Ellipsis)
                            }
                            OutlinedButton(
                                onClick = { showTime = true },
                                modifier = Modifier.weight(0.75f).defaultMinSize(minHeight = 48.dp),
                            ) {
                                Icon(Icons.Outlined.Schedule, contentDescription = null, modifier = Modifier.size(16.dp))
                                Spacer(Modifier.width(6.dp))
                                Text(java.util.Locale.US.let { "%02d:%02d".format(it, s.hour, s.minute) }, style = MaterialTheme.typography.labelLarge.tabular())
                            }
                        }

                        AnimatedVisibility(
                            visible = s.tripType == TripType.RETURN,
                            enter = expandVertically(tween(180)) + fadeIn(tween(180)),
                            exit = shrinkVertically(tween(150)) + fadeOut(tween(140)),
                        ) {
                            Text(
                                stringResource(R.string.ride_new_time_hint_return),
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }

                        AnimatedVisibility(
                            visible = s.tripType == TripType.ROUND,
                            enter = expandVertically(tween(180)) + fadeIn(tween(180)),
                            exit = shrinkVertically(tween(150)) + fadeOut(tween(140)),
                        ) {
                            Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                Text(stringResource(R.string.ride_new_return_time), style = MaterialTheme.typography.titleSmall)
                                Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                    OutlinedButton(
                                        onClick = { showReturnDate = true },
                                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                                    ) {
                                        Icon(Icons.Outlined.CalendarToday, contentDescription = null, modifier = Modifier.size(16.dp))
                                        Spacer(Modifier.width(6.dp))
                                        Text(KlTime.displayDate(s.returnDate.toString()), maxLines = 1, overflow = TextOverflow.Ellipsis)
                                    }
                                    OutlinedButton(
                                        onClick = { showReturnTime = true },
                                        modifier = Modifier.weight(0.75f).defaultMinSize(minHeight = 48.dp),
                                    ) {
                                        Icon(Icons.Outlined.Schedule, contentDescription = null, modifier = Modifier.size(16.dp))
                                        Spacer(Modifier.width(6.dp))
                                        Text(java.util.Locale.US.let { "%02d:%02d".format(it, s.returnHour, s.returnMinute) }, style = MaterialTheme.typography.labelLarge.tabular())
                                    }
                                }
                            }
                        }
                    }
                }

                // 3. Family / Proxy booking section
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
                                onCheckedChange = { checked ->
                                    UiHaptics.tick(view)
                                    vm.setProxyMode(checked)
                                },
                            )
                        }
                        AnimatedVisibility(
                            visible = s.isProxy,
                            enter = expandVertically(tween(200)) + fadeIn(tween(180)),
                            exit = shrinkVertically(tween(160)) + fadeOut(tween(140)),
                        ) {
                            Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
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
                                    TextButton(onClick = { nav.navigate(AppRoute.ProxyRiders) }) {
                                        Text(stringResource(R.string.common_manage))
                                    }
                                }
                                if (s.proxyRiders.isNotEmpty()) {
                                    FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                        s.proxyRiders.forEach { fr ->
                                            FilterChip(
                                                selected = s.riderName == fr.name,
                                                onClick = {
                                                    UiHaptics.tick(view)
                                                    vm.selectProxyRider(fr)
                                                },
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
                                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Next),
                                    modifier = Modifier.fillMaxWidth(),
                                )
                                OutlinedTextField(
                                    value = s.riderPhone,
                                    onValueChange = { v -> vm.update { it.copy(riderPhone = v) } },
                                    label = { Text(stringResource(R.string.ride_new_rider_phone)) },
                                    singleLine = true,
                                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Phone, imeAction = ImeAction.Next),
                                    modifier = Modifier.fillMaxWidth(),
                                )
                                val relationships = ProxyRelationships.keys.map { key ->
                                    key to (ProxyRelationships.labelRes(key)?.let { stringResource(it) } ?: key)
                                }
                                ExposedDropdownMenuBox(
                                    expanded = relationshipExpanded,
                                    onExpandedChange = { relationshipExpanded = it },
                                ) {
                                    OutlinedTextField(
                                        value = ProxyRelationships.labelRes(s.riderRelationship)?.let { stringResource(it) }
                                            ?: s.riderRelationship,
                                        onValueChange = {},
                                        readOnly = true,
                                        label = { Text(stringResource(R.string.ride_new_rider_relationship)) },
                                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = relationshipExpanded) },
                                        modifier = Modifier.menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
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
                                        .defaultMinSize(minHeight = 48.dp)
                                        .toggleable(
                                            value = s.saveProxyRider,
                                            role = Role.Checkbox,
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
                }

                // 4. Pickup & destination card
                OutlinedCard(Modifier.fillMaxWidth()) {
                    Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        Text(stringResource(R.string.ride_new_section_location), style = MaterialTheme.typography.titleSmall)
                        if (!s.isProxy) {
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.SpaceBetween,
                                modifier = Modifier.fillMaxWidth(),
                            ) {
                                Text(
                                    stringResource(R.string.ride_new_saved_addresses),
                                    style = MaterialTheme.typography.labelMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                                TextButton(onClick = { nav.navigate(AppRoute.SavedAddresses) }) {
                                    Text(stringResource(R.string.common_manage))
                                }
                            }
                            if (s.savedPlaces.isNotEmpty()) {
                                val activeAddr = if (s.tripType == TripType.RETURN) s.destination else s.address
                                FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                    s.savedPlaces.forEach { sp ->
                                        FilterChip(
                                            selected = activeAddr == sp.address,
                                            onClick = {
                                                UiHaptics.tick(view)
                                                vm.applySavedPlace(sp)
                                            },
                                            leadingIcon = {
                                                Icon(Icons.Outlined.Place, contentDescription = null, modifier = Modifier.size(16.dp))
                                            },
                                            label = { Text(sp.label) },
                                        )
                                    }
                                }
                            }
                        }

                        ExposedDropdownMenuBox(expanded = areaExpanded, onExpandedChange = { areaExpanded = it }) {
                            OutlinedTextField(
                                value = s.areas.firstOrNull { it.id == s.areaId }?.name ?: "",
                                onValueChange = {},
                                readOnly = true,
                                label = { Text(stringResource(R.string.ride_new_area)) },
                                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = areaExpanded) },
                                modifier = Modifier.menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
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

                        OutlinedTextField(
                            value = s.address,
                            onValueChange = vm::onAddressChange,
                            label = { Text(stringResource(R.string.ride_new_address)) },
                            trailingIcon = if (s.address.isNotEmpty()) {
                                {
                                    IconButton(onClick = { vm.onAddressChange("") }) {
                                        Icon(Icons.Outlined.Close, contentDescription = stringResource(R.string.a11y_clear))
                                    }
                                }
                            } else {
                                null
                            },
                            modifier = Modifier.fillMaxWidth(),
                        )
                        if (s.suggestions.isNotEmpty() && s.tripType != TripType.RETURN) {
                            OutlinedCard(Modifier.fillMaxWidth()) {
                                Column {
                                    val unique = s.suggestions.distinctBy { it.label }.take(5)
                                    unique.forEachIndexed { idx, place ->
                                        if (idx > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                        Surface(
                                            onClick = { vm.pickSuggestion(place) },
                                            color = MaterialTheme.colorScheme.surface,
                                        ) {
                                            Row(
                                                Modifier
                                                    .fillMaxWidth()
                                                    .defaultMinSize(minHeight = 48.dp)
                                                    .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                                                verticalAlignment = Alignment.CenterVertically,
                                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                            ) {
                                                Icon(
                                                    Icons.Outlined.LocationOn,
                                                    contentDescription = null,
                                                    tint = MaterialTheme.colorScheme.primary,
                                                    modifier = Modifier.size(18.dp),
                                                )
                                                Text(
                                                    place.label,
                                                    style = MaterialTheme.typography.bodyMedium,
                                                    maxLines = 2,
                                                    overflow = TextOverflow.Ellipsis,
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        OutlinedTextField(
                            value = s.destination,
                            onValueChange = vm::onDestinationChange,
                            label = { Text(stringResource(R.string.ride_new_destination)) },
                            trailingIcon = if (s.tripType == TripType.RETURN && s.destination.isNotEmpty()) {
                                {
                                    IconButton(onClick = { vm.onDestinationChange("") }) {
                                        Icon(Icons.Outlined.Close, contentDescription = stringResource(R.string.a11y_clear))
                                    }
                                }
                            } else {
                                null
                            },
                            modifier = Modifier.fillMaxWidth(),
                        )
                        if (s.suggestions.isNotEmpty() && s.tripType == TripType.RETURN) {
                            OutlinedCard(Modifier.fillMaxWidth()) {
                                Column {
                                    val unique = s.suggestions.distinctBy { it.label }.take(5)
                                    unique.forEachIndexed { idx, place ->
                                        if (idx > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                        Surface(
                                            onClick = { vm.pickSuggestion(place) },
                                            color = MaterialTheme.colorScheme.surface,
                                        ) {
                                            Row(
                                                Modifier
                                                    .fillMaxWidth()
                                                    .defaultMinSize(minHeight = 48.dp)
                                                    .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                                                verticalAlignment = Alignment.CenterVertically,
                                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                            ) {
                                                Icon(
                                                    Icons.Outlined.LocationOn,
                                                    contentDescription = null,
                                                    tint = MaterialTheme.colorScheme.primary,
                                                    modifier = Modifier.size(18.dp),
                                                )
                                                Text(
                                                    place.label,
                                                    style = MaterialTheme.typography.bodyMedium,
                                                    maxLines = 2,
                                                    overflow = TextOverflow.Ellipsis,
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }

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
                                    .defaultMinSize(minHeight = 48.dp)
                                    .toggleable(
                                        value = s.saveAddress,
                                        role = Role.Checkbox,
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
                            AnimatedVisibility(
                                visible = s.saveAddress,
                                enter = expandVertically(tween(180)) + fadeIn(tween(160)),
                                exit = shrinkVertically(tween(140)) + fadeOut(tween(120)),
                            ) {
                                Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
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
                        }
                    }
                }

                // 5. Seats, assistance & notes
                OutlinedCard(Modifier.fillMaxWidth()) {
                    Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        Text(stringResource(R.string.ride_new_section_details), style = MaterialTheme.typography.titleSmall)
                        Row(
                            Modifier.fillMaxWidth(),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween,
                        ) {
                            Text(stringResource(R.string.ride_new_seats), style = MaterialTheme.typography.bodyLarge)
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                OutlinedIconButton(
                                    enabled = s.seats > 1,
                                    onClick = {
                                        UiHaptics.tick(view)
                                        vm.update { it.copy(seats = (it.seats - 1).coerceAtLeast(1)) }
                                    },
                                    modifier = Modifier.size(48.dp),
                                ) {
                                    Icon(Icons.Outlined.Remove, contentDescription = stringResource(R.string.a11y_decrease))
                                }
                                Text(
                                    s.seats.toString(),
                                    style = MaterialTheme.typography.titleLarge.tabular(),
                                    textAlign = TextAlign.Center,
                                    modifier = Modifier.width(28.dp),
                                )
                                OutlinedIconButton(
                                    enabled = s.seats < 6,
                                    onClick = {
                                        UiHaptics.tick(view)
                                        vm.update { it.copy(seats = (it.seats + 1).coerceAtMost(6)) }
                                    },
                                    modifier = Modifier.size(48.dp),
                                ) {
                                    Icon(Icons.Outlined.Add, contentDescription = stringResource(R.string.a11y_increase))
                                }
                            }
                        }

                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)

                        Text(stringResource(R.string.ride_new_assistance), style = MaterialTheme.typography.titleSmall)
                        FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            listOf(
                                Triple("", R.string.ride_new_assist_none, null),
                                Triple("wheelchair", R.string.ride_new_assist_wheelchair, Icons.AutoMirrored.Outlined.Accessible),
                                Triple("stroller", R.string.ride_new_assist_stroller, Icons.Outlined.ChildFriendly),
                                Triple("oxygen", R.string.ride_new_assist_oxygen, Icons.Outlined.Air),
                                Triple("other", R.string.ride_new_assist_other, null),
                            ).forEach { (value, labelRes, icon) ->
                                FilterChip(
                                    selected = s.assistance == value,
                                    onClick = {
                                        UiHaptics.tick(view)
                                        vm.update { it.copy(assistance = value) }
                                    },
                                    leadingIcon = icon?.let { ic ->
                                        { Icon(ic, contentDescription = null, modifier = Modifier.size(16.dp)) }
                                    },
                                    label = { Text(stringResource(labelRes)) },
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
                    }
                }
            }
        }
    }

    if (showDate) {
        val todayMillis = remember { DatePickerUtc.toPickerMillis(KlTime.todayDate()) }
        val pickerState = rememberDatePickerState(
            initialSelectedDateMillis = DatePickerUtc.toPickerMillis(s.date),
            selectableDates = object : androidx.compose.material3.SelectableDates {
                override fun isSelectableDate(utcTimeMillis: Long): Boolean = utcTimeMillis >= todayMillis
            },
        )
        DatePickerDialog(
            onDismissRequest = { showDate = false },
            confirmButton = {
                TextButton(onClick = {
                    pickerState.selectedDateMillis?.let { millis ->
                        val picked = DatePickerUtc.fromPickerMillis(millis)
                        vm.update {
                            it.copy(
                                date = picked,
                                returnDate = if (it.returnDate.isBefore(picked)) picked else it.returnDate,
                            )
                        }
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
        val minReturnMillis = DatePickerUtc.toPickerMillis(s.date)
        val pickerState = rememberDatePickerState(
            initialSelectedDateMillis = DatePickerUtc.toPickerMillis(s.returnDate),
            selectableDates = object : androidx.compose.material3.SelectableDates {
                override fun isSelectableDate(utcTimeMillis: Long): Boolean = utcTimeMillis >= minReturnMillis
            },
        )
        DatePickerDialog(
            onDismissRequest = { showReturnDate = false },
            confirmButton = {
                TextButton(onClick = {
                    pickerState.selectedDateMillis?.let { millis ->
                        vm.update { it.copy(returnDate = DatePickerUtc.fromPickerMillis(millis)) }
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
