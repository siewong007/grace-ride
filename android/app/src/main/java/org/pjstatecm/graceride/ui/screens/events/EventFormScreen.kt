package org.pjstatecm.graceride.ui.screens.events

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.PhotoLibrary
import androidx.compose.material.icons.outlined.WarningAmber
import androidx.compose.material3.Button
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
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import coil.compose.AsyncImage
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.GroupTree
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.EventIcon
import org.pjstatecm.graceride.ui.components.EventIcons
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun EventFormScreen(
    container: AppContainer,
    nav: NavHostController,
    eventId: Long? = null,
    duplicateId: Long? = null,
) {
    val vm: EventFormViewModel = viewModel(
        key = "event_form_${eventId ?: 0}_${duplicateId ?: 0}",
        factory = EventFormViewModel.factory(container, eventId, duplicateId),
    )
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val me = (authState as? AuthState.SignedIn)?.me ?: container.auth.me
    val canMileage = me?.canAuthoriseMileage() == true
    val context = LocalContext.current
    val statusColors = LocalStatusColors.current

    val photoPicker = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.PickVisualMedia(),
    ) { uri ->
        if (uri != null) {
            vm.uploadCoverPhoto(context, uri)
        }
    }

    LaunchedEffect(state.savedEventId) {
        val id = state.savedEventId ?: return@LaunchedEffect
        nav.navigate(AppRoute.EventDetail(id)) {
            if (eventId != null) {
                popUpTo(AppRoute.EventDetail(eventId)) { inclusive = true }
            }
            launchSingleTop = true
        }
    }

    val titleRes = when {
        eventId != null -> R.string.event_form_edit_title
        duplicateId != null -> R.string.event_form_duplicate_title
        else -> R.string.event_form_create_title
    }

    val ordinaryGroups = remember(state.groups) { GroupTree.ordinary(state.groups) }
    val orderedGroups = remember(ordinaryGroups) { GroupTree.treeOrder(ordinaryGroups) }
    val forEveryone = remember(ordinaryGroups, state.groupIds) {
        ordinaryGroups.any { it.isAll && it.id in state.groupIds }
    }
    val reachIds = remember(ordinaryGroups, state.groupIds, forEveryone) {
        if (forEveryone) {
            ordinaryGroups.map { it.id }.toSet()
        } else {
            state.groupIds.flatMap { gid -> GroupTree.withDescendants(ordinaryGroups, gid) }.toSet()
        }
    }
    val joinableGroups = remember(ordinaryGroups, reachIds) {
        GroupTree.treeOrder(ordinaryGroups.filter { it.id in reachIds && !it.isAll }).map { it.group }
    }
    val joinableIdSet = remember(joinableGroups) { joinableGroups.map { it.id }.toSet() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(titleRes)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.common_back))
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background),
            )
        },
        containerColor = MaterialTheme.colorScheme.background,
    ) { padding ->
        when {
            state.loading -> LoadingView(Modifier.fillMaxSize().padding(padding))
            state.loadError != null -> ErrorView(
                message = state.loadError ?: "",
                modifier = Modifier.padding(padding),
                onRetry = vm::load,
            )
            else -> {
                val errText = state.error ?: state.errorRes?.let { stringResource(it) }
                LazyColumn(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding)
                        .padding(horizontal = Spacing.sm),
                    contentPadding = PaddingValues(vertical = Spacing.sm),
                    verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                ) {
                    if (state.isEditingDraft) {
                        item {
                            CalloutBanner(
                                icon = Icons.Outlined.WarningAmber,
                                text = stringResource(R.string.event_draft_banner),
                                containerColor = statusColors.warningSoft,
                                contentColor = statusColors.warning,
                            )
                        }
                    }

                    // 1. Core details
                    item {
                        OutlinedCard(
                            modifier = Modifier.fillMaxWidth(),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                        ) {
                            Column(
                                Modifier.padding(Spacing.sm),
                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                OutlinedTextField(
                                    value = state.title,
                                    onValueChange = { txt -> vm.update { it.copy(title = txt) } },
                                    label = { Text(stringResource(R.string.event_form_name)) },
                                    placeholder = { Text(stringResource(R.string.event_form_name_hint)) },
                                    singleLine = true,
                                    modifier = Modifier.fillMaxWidth(),
                                )
                                Text(stringResource(R.string.event_form_type), style = MaterialTheme.typography.titleSmall)
                                FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                    listOf(
                                        "badminton" to R.string.event_kind_badminton,
                                        "bowling" to R.string.event_kind_bowling,
                                        "kayak" to R.string.event_kind_kayak,
                                        "walk" to R.string.event_kind_walk,
                                        "other" to R.string.event_kind_other,
                                    ).forEach { (kindKey, labelRes) ->
                                        FilterChip(
                                            selected = state.kind == kindKey,
                                            onClick = { vm.update { it.copy(kind = kindKey) } },
                                            label = { Text(stringResource(labelRes)) },
                                        )
                                    }
                                }

                                // Icon picker
                                Row(
                                    Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Row(
                                        verticalAlignment = Alignment.CenterVertically,
                                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    ) {
                                        EventIcon(icon = state.icon.ifBlank { null }, kind = state.kind, size = 28.dp)
                                        Text(stringResource(R.string.event_form_icon), style = MaterialTheme.typography.titleSmall)
                                    }
                                    if (state.icon.isNotBlank()) {
                                        TextButton(onClick = { vm.update { it.copy(icon = "") } }) {
                                            Text(stringResource(R.string.event_form_icon_reset))
                                        }
                                    }
                                }
                                FlowRow(
                                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                ) {
                                    EventIcons.PRESET_KEYS.forEach { key ->
                                        val selected = state.icon.trim() == key
                                        Surface(
                                            onClick = {
                                                vm.update { cur ->
                                                    cur.copy(icon = if (cur.icon.trim() == key) "" else key)
                                                }
                                            },
                                            shape = RoundedCornerShape(10.dp),
                                            color = if (selected) {
                                                MaterialTheme.colorScheme.primaryContainer
                                            } else {
                                                MaterialTheme.colorScheme.surfaceVariant
                                            },
                                            border = if (selected) BorderStroke(1.5.dp, MaterialTheme.colorScheme.primary) else null,
                                            modifier = Modifier.size(44.dp),
                                        ) {
                                            Box(contentAlignment = Alignment.Center) {
                                                EventIcon(icon = key, size = 24.dp)
                                            }
                                        }
                                    }
                                }
                                OutlinedTextField(
                                    value = if (state.icon.trim() in EventIcons.PRESET_KEYS) "" else state.icon,
                                    onValueChange = { txt -> vm.update { it.copy(icon = txt.take(32)) } },
                                    label = { Text(stringResource(R.string.event_form_icon_custom)) },
                                    singleLine = true,
                                    modifier = Modifier.fillMaxWidth(),
                                )
                            }
                        }
                    }

                    // 2. Schedule, Venue & Cost
                    item {
                        var areaExpanded by remember { mutableStateOf(false) }
                        OutlinedCard(
                            modifier = Modifier.fillMaxWidth(),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                        ) {
                            Column(
                                Modifier.padding(Spacing.sm),
                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                    OutlinedTextField(
                                        value = state.date,
                                        onValueChange = { txt -> vm.update { it.copy(date = txt) } },
                                        label = { Text(stringResource(R.string.event_form_date)) },
                                        placeholder = { Text("2026-12-05") },
                                        singleLine = true,
                                        modifier = Modifier.weight(1f),
                                    )
                                    OutlinedTextField(
                                        value = state.time,
                                        onValueChange = { txt -> vm.update { it.copy(time = txt) } },
                                        label = { Text(stringResource(R.string.event_form_time)) },
                                        placeholder = { Text("09:30") },
                                        singleLine = true,
                                        modifier = Modifier.weight(1f),
                                    )
                                }
                                OutlinedTextField(
                                    value = state.location,
                                    onValueChange = { txt -> vm.update { it.copy(location = txt) } },
                                    label = { Text(stringResource(R.string.event_location)) },
                                    singleLine = true,
                                    modifier = Modifier.fillMaxWidth(),
                                )
                                ExposedDropdownMenuBox(
                                    expanded = areaExpanded,
                                    onExpandedChange = { areaExpanded = it },
                                ) {
                                    val selectedArea = state.areas.firstOrNull { it.id == state.areaId }
                                    OutlinedTextField(
                                        value = selectedArea?.name ?: stringResource(R.string.event_form_area_none),
                                        onValueChange = {},
                                        readOnly = true,
                                        label = { Text(stringResource(R.string.event_form_area)) },
                                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = areaExpanded) },
                                        modifier = Modifier
                                            .menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable)
                                            .fillMaxWidth(),
                                    )
                                    ExposedDropdownMenu(
                                        expanded = areaExpanded,
                                        onDismissRequest = { areaExpanded = false },
                                    ) {
                                        DropdownMenuItem(
                                            text = { Text(stringResource(R.string.event_form_area_none)) },
                                            onClick = {
                                                vm.update { it.copy(areaId = null) }
                                                areaExpanded = false
                                            },
                                        )
                                        state.areas.forEach { a ->
                                            DropdownMenuItem(
                                                text = { Text(a.name) },
                                                onClick = {
                                                    vm.update { it.copy(areaId = a.id) }
                                                    areaExpanded = false
                                                },
                                            )
                                        }
                                    }
                                }
                                Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                    OutlinedTextField(
                                        value = state.capacity,
                                        onValueChange = { txt -> vm.update { it.copy(capacity = txt.filter { c -> c.isDigit() }) } },
                                        label = { Text(stringResource(R.string.event_form_capacity)) },
                                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                                        singleLine = true,
                                        modifier = Modifier.weight(1f),
                                    )
                                    OutlinedTextField(
                                        value = state.costNote,
                                        onValueChange = { txt -> vm.update { it.copy(costNote = txt) } },
                                        label = { Text(stringResource(R.string.event_form_cost_note)) },
                                        placeholder = { Text(stringResource(R.string.event_form_cost_hint)) },
                                        singleLine = true,
                                        modifier = Modifier.weight(1f),
                                    )
                                }
                                OutlinedTextField(
                                    value = state.deadline,
                                    onValueChange = { txt -> vm.update { it.copy(deadline = txt) } },
                                    label = { Text(stringResource(R.string.event_form_deadline)) },
                                    singleLine = true,
                                    modifier = Modifier.fillMaxWidth(),
                                )
                                OutlinedTextField(
                                    value = state.description,
                                    onValueChange = { txt -> vm.update { it.copy(description = txt) } },
                                    label = { Text(stringResource(R.string.event_form_desc)) },
                                    minLines = 3,
                                    modifier = Modifier.fillMaxWidth(),
                                )
                            }
                        }
                    }

                    // 3. Cover photo, modules & repeat schedule
                    item {
                        OutlinedCard(
                            modifier = Modifier.fillMaxWidth(),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                        ) {
                            Column(
                                Modifier.padding(Spacing.sm),
                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                Text(stringResource(R.string.event_form_photo), style = MaterialTheme.typography.titleSmall)
                                state.photoFileId?.let { fid ->
                                    AsyncImage(
                                        model = container.fileUrl(fid),
                                        contentDescription = null,
                                        contentScale = ContentScale.Crop,
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .height(160.dp)
                                            .clip(MaterialTheme.shapes.medium),
                                    )
                                }
                                Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                    OutlinedButton(
                                        onClick = {
                                            photoPicker.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly))
                                        },
                                        enabled = !state.saving,
                                    ) {
                                        Icon(Icons.Outlined.PhotoLibrary, contentDescription = null, modifier = Modifier.size(18.dp))
                                        Spacer(Modifier.width(Spacing.xxs))
                                        Text(stringResource(R.string.event_form_photo_pick))
                                    }
                                    if (state.photoFileId != null) {
                                        TextButton(onClick = { vm.update { it.copy(photoFileId = null) } }) {
                                            Text(stringResource(R.string.event_form_photo_clear))
                                        }
                                    }
                                }

                                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                Row(
                                    Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Text(stringResource(R.string.event_form_attendance), modifier = Modifier.weight(1f))
                                    Switch(
                                        checked = state.attendanceEnabled,
                                        onCheckedChange = { v -> vm.update { it.copy(attendanceEnabled = v) } },
                                    )
                                }
                                Row(
                                    Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Column(Modifier.weight(1f).padding(end = Spacing.xs)) {
                                        Text(stringResource(R.string.event_form_mileage))
                                        if (!canMileage) {
                                            Text(
                                                stringResource(R.string.event_form_mileage_restricted),
                                                style = MaterialTheme.typography.bodySmall,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                            )
                                        }
                                    }
                                    Switch(
                                        checked = state.mileageClaimable,
                                        enabled = canMileage,
                                        onCheckedChange = { v -> vm.update { it.copy(mileageClaimable = v) } },
                                    )
                                }

                                if (eventId == null) {
                                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                    Text(stringResource(R.string.event_form_repeat), style = MaterialTheme.typography.titleSmall)
                                    FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                        listOf(
                                            "none" to R.string.event_form_repeat_none,
                                            "weekly" to R.string.event_form_repeat_weekly,
                                            "biweekly" to R.string.event_form_repeat_biweekly,
                                            "monthly" to R.string.event_form_repeat_monthly,
                                        ).forEach { (freqKey, labelRes) ->
                                            FilterChip(
                                                selected = state.repeatFrequency == freqKey,
                                                onClick = { vm.update { it.copy(repeatFrequency = freqKey) } },
                                                label = { Text(stringResource(labelRes)) },
                                            )
                                        }
                                    }
                                    if (state.repeatFrequency != "none") {
                                        OutlinedTextField(
                                            value = state.repeatCount,
                                            onValueChange = { txt -> vm.update { it.copy(repeatCount = txt.filter { c -> c.isDigit() }) } },
                                            label = { Text(stringResource(R.string.event_form_repeat_count)) },
                                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                                            singleLine = true,
                                            modifier = Modifier.fillMaxWidth(),
                                        )
                                    }
                                }
                            }
                        }
                    }

                    // 4. Audience groups & optional Join-as narrowing
                    item {
                        OutlinedCard(
                            modifier = Modifier.fillMaxWidth(),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                        ) {
                            Column(
                                Modifier.padding(Spacing.sm),
                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                Text(stringResource(R.string.event_form_groups), style = MaterialTheme.typography.titleSmall)
                                orderedGroups.forEach { (g, depth) ->
                                    Row(
                                        Modifier
                                            .fillMaxWidth()
                                            .defaultMinSize(minHeight = 44.dp)
                                            .clickable { vm.toggleGroup(g.id) }
                                            .padding(start = (depth * 16).dp),
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        Checkbox(
                                            checked = g.id in state.groupIds,
                                            onCheckedChange = null,
                                        )
                                        Text(
                                            text = buildString {
                                                if (depth > 0) append("↳ ")
                                                append(g.name)
                                                if (!g.isAll) append(" (${g.memberCount})")
                                            },
                                            style = MaterialTheme.typography.bodyMedium,
                                            modifier = Modifier.padding(start = Spacing.xs),
                                        )
                                    }
                                }

                                if (joinableGroups.size > 1) {
                                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                    Row(
                                        Modifier
                                            .fillMaxWidth()
                                            .clickable { vm.update { it.copy(narrowJoinAs = !it.narrowJoinAs) } },
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        Checkbox(checked = state.narrowJoinAs, onCheckedChange = null)
                                        Text(
                                            stringResource(R.string.event_form_join_as_narrow),
                                            style = MaterialTheme.typography.bodyMedium,
                                            modifier = Modifier.padding(start = Spacing.xs),
                                        )
                                    }
                                    if (state.narrowJoinAs) {
                                        FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                            joinableGroups.forEach { g ->
                                                FilterChip(
                                                    selected = g.id in state.asGroupIds,
                                                    onClick = { vm.toggleAsGroup(g.id) },
                                                    label = { Text(g.name) },
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // 5. Errors & Submit Actions
                    item {
                        Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            errText?.let {
                                CalloutBanner(
                                    icon = Icons.Outlined.WarningAmber,
                                    text = it,
                                    containerColor = MaterialTheme.colorScheme.errorContainer,
                                    contentColor = MaterialTheme.colorScheme.onErrorContainer,
                                )
                            }
                            when {
                                eventId == null -> {
                                    Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                        OutlinedButton(
                                            onClick = { vm.submit("draft", false, joinableIdSet) },
                                            enabled = !state.saving,
                                            modifier = Modifier.weight(1f).height(50.dp),
                                        ) {
                                            Text(stringResource(R.string.event_form_save_draft))
                                        }
                                        Button(
                                            onClick = { vm.submit("published", false, joinableIdSet) },
                                            enabled = !state.saving,
                                            colors = graceRidePrimaryButtonColors(),
                                            modifier = Modifier.weight(1f).height(50.dp),
                                        ) {
                                            if (state.saving) {
                                                InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                            } else {
                                                Text(stringResource(R.string.event_publish))
                                            }
                                        }
                                    }
                                }
                                state.isEditingDraft -> {
                                    Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                        OutlinedButton(
                                            onClick = { vm.submit(null, false, joinableIdSet) },
                                            enabled = !state.saving,
                                            modifier = Modifier.weight(1f).height(50.dp),
                                        ) {
                                            Text(stringResource(R.string.event_form_save_draft))
                                        }
                                        Button(
                                            onClick = { vm.submit(null, true, joinableIdSet) },
                                            enabled = !state.saving,
                                            colors = graceRidePrimaryButtonColors(),
                                            modifier = Modifier.weight(1f).height(50.dp),
                                        ) {
                                            if (state.saving) {
                                                InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                            } else {
                                                Text(stringResource(R.string.event_form_save_publish))
                                            }
                                        }
                                    }
                                }
                                else -> {
                                    Button(
                                        onClick = { vm.submit(null, false, joinableIdSet) },
                                        enabled = !state.saving,
                                        colors = graceRidePrimaryButtonColors(),
                                        modifier = Modifier.fillMaxWidth().height(50.dp),
                                    ) {
                                        if (state.saving) {
                                            InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                        } else {
                                            Text(stringResource(R.string.event_form_save))
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
}
