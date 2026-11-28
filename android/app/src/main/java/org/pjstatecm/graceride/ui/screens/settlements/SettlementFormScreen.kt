package org.pjstatecm.graceride.ui.screens.settlements

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.PressInteraction
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
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.CalendarMonth
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.DeleteOutline
import androidx.compose.material.icons.outlined.QrCode2
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import java.time.LocalDate
import kotlin.math.abs
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.DatePickerUtc
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.AdaptiveContentFrame
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.components.ZoomableImageDialog
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun SettlementFormScreen(
    container: AppContainer,
    nav: NavHostController,
    settlementId: Long? = null,
    eventId: Long? = null,
) {
    org.pjstatecm.graceride.ui.SecureScreenEffect()
    val vm: SettlementFormViewModel = viewModel(
        key = "settle-form-${settlementId ?: "new"}-${eventId ?: 0}",
        factory = SettlementFormViewModel.factory(container, settlementId, eventId),
    )
    val state by vm.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val view = LocalView.current
    var eventExpanded by remember { mutableStateOf(false) }
    var showDueDatePicker by remember { mutableStateOf(false) }
    var participantQuery by remember { mutableStateOf("") }
    var showQrZoom by remember { mutableStateOf(false) }

    val qrPicker = rememberLauncherForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri ->
        if (uri != null) vm.uploadQr(context, uri)
    }

    LaunchedEffect(state.savedId) {
        val targetId = state.savedId ?: return@LaunchedEffect
        nav.navigate(AppRoute.SettlementDetail(targetId)) {
            popUpTo(nav.currentBackStackEntry?.destination?.id ?: return@navigate) { inclusive = true }
        }
    }

    val missingMsg = stringResource(R.string.settlement_form_required)
    val unbalancedMsg = stringResource(R.string.settlement_custom_unbalanced_error)

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        stringResource(
                            if (state.editingId == null) R.string.settlements_new else R.string.settlement_edit,
                        ),
                    )
                },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
            )
        },
        bottomBar = {
            if (!state.loading) {
                Surface(color = MaterialTheme.colorScheme.surface) {
                    Column(
                        Modifier
                            .fillMaxWidth()
                            .navigationBarsPadding()
                            .imePadding(),
                        horizontalAlignment = Alignment.CenterHorizontally,
                    ) {
                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                        Column(
                            Modifier
                                .widthIn(max = 720.dp)
                                .fillMaxWidth()
                                .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            Row(
                                Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Text(
                                    pluralStringResource(
                                        R.plurals.settlement_total_preview_plural,
                                        state.selectedParticipants.size,
                                        Money.rm(state.netTotalCents),
                                        state.selectedParticipants.size,
                                    ),
                                    style = MaterialTheme.typography.titleSmall.tabular(),
                                    fontWeight = FontWeight.SemiBold,
                                    modifier = Modifier.weight(1f),
                                )
                            if (state.splitMode == "equal" && state.selectedParticipants.isNotEmpty()) {
                                val perShare = state.equalShareAmounts.firstOrNull() ?: 0L
                                StatusPill(
                                    text = stringResource(R.string.settlement_per_person, Money.rm(perShare)),
                                    kind = RideStatusKind.ACTIVE,
                                )
                            } else if (state.splitMode == "custom") {
                                val diff = state.netTotalCents - state.customTotalCents
                                if (state.isCustomBalanced) {
                                    StatusPill(
                                        text = stringResource(R.string.settlement_custom_balanced),
                                        kind = RideStatusKind.DONE,
                                    )
                                } else {
                                    val formatted = if (diff < 0) "-${Money.rm(abs(diff))}" else Money.rm(diff)
                                    StatusPill(
                                        text = stringResource(R.string.settlement_custom_remaining, formatted),
                                        kind = RideStatusKind.PENDING,
                                    )
                                }
                            }
                        }

                        state.error?.let { err ->
                            Text(err, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                        }

                        if (state.existingStatus == "published") {
                            Button(
                                onClick = {
                                    UiHaptics.confirm(view)
                                    vm.submit(publishAfterSave = false, fallbackErr = missingMsg, unbalancedErr = unbalancedMsg)
                                },
                                enabled = !state.submitting && state.revisionReason.trim().isNotEmpty() && state.isCustomBalanced,
                                colors = graceRidePrimaryButtonColors(),
                                modifier = Modifier.fillMaxWidth().height(50.dp),
                            ) {
                                if (state.submitting) {
                                    InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                } else {
                                    Text(stringResource(R.string.settlement_save_changes))
                                }
                            }
                        } else {
                            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                OutlinedButton(
                                    onClick = {
                                        UiHaptics.tick(view)
                                        vm.submit(publishAfterSave = false, fallbackErr = missingMsg, unbalancedErr = unbalancedMsg)
                                    },
                                    enabled = !state.submitting,
                                    modifier = Modifier.weight(1f).height(50.dp),
                                ) {
                                    Text(stringResource(R.string.settlement_save_draft))
                                }
                                Button(
                                    onClick = {
                                        UiHaptics.confirm(view)
                                        vm.submit(publishAfterSave = true, fallbackErr = missingMsg, unbalancedErr = unbalancedMsg)
                                    },
                                    enabled = !state.submitting && state.isCustomBalanced,
                                    colors = graceRidePrimaryButtonColors(),
                                    modifier = Modifier.weight(1f).height(50.dp),
                                ) {
                                    if (state.submitting) {
                                        InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                    } else {
                                        Text(stringResource(R.string.settlement_publish))
                                    }
                                }
                            }
                        }
                    }
                }
            }
            }
        },
    ) { padding ->
        if (state.loading) {
            LoadingView(Modifier.padding(padding))
            return@Scaffold
        }
        val equalShareMap = remember(state.participants, state.equalShareAmounts, state.splitMode) {
            if (state.splitMode != "equal") {
                emptyMap()
            } else {
                var idx = 0
                val list = state.equalShareAmounts
                buildMap {
                    state.participants.forEach { p ->
                        if (p.selected) put(p.userId, list.getOrElse(idx++) { 0L })
                    }
                }
            }
        }
        val needle = participantQuery.trim().lowercase()
        val visibleParticipants = remember(state.participants, needle) {
            if (needle.isEmpty()) state.participants
            else state.participants.filter { it.name.lowercase().contains(needle) }
        }
        val visibleIds = remember(visibleParticipants) { visibleParticipants.map { it.userId }.toSet() }

        AdaptiveContentFrame(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
            maxWidth = 720.dp,
        ) {
            Column(
                Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(Spacing.sm),
                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
            ) {
                if (state.editingId == null && state.events.isNotEmpty() && eventId == null) {
                ExposedDropdownMenuBox(expanded = eventExpanded, onExpandedChange = { eventExpanded = it }) {
                    OutlinedTextField(
                        value = state.events.firstOrNull { it.id == state.selectedEventId }?.title ?: "",
                        onValueChange = {},
                        readOnly = true,
                        label = { Text(stringResource(R.string.settlement_event_label)) },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = eventExpanded) },
                        modifier = Modifier.menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
                    )
                    ExposedDropdownMenu(expanded = eventExpanded, onDismissRequest = { eventExpanded = false }) {
                        state.events.forEach { ev ->
                            DropdownMenuItem(
                                text = { Text(ev.title) },
                                onClick = {
                                    vm.selectEvent(ev.id)
                                    eventExpanded = false
                                },
                            )
                        }
                    }
                }
            }

            OutlinedTextField(
                value = state.title,
                onValueChange = { v -> vm.update { it.copy(title = v) } },
                label = { Text(stringResource(R.string.settlement_title_label)) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )

            SectionHeader(stringResource(R.string.settlement_items_section))
            state.items.forEachIndexed { idx, line ->
                OutlinedCard(Modifier.fillMaxWidth()) {
                    Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        Row(
                            Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Text("#${idx + 1}", style = MaterialTheme.typography.labelMedium)
                            if (state.items.size > 1) {
                                IconButton(
                                    onClick = {
                                        UiHaptics.tick(view)
                                        vm.removeItem(line.key)
                                    },
                                ) {
                                    Icon(Icons.Outlined.DeleteOutline, contentDescription = stringResource(R.string.common_delete))
                                }
                            }
                        }
                        OutlinedTextField(
                            value = line.label,
                            onValueChange = { vm.updateItem(line.key, label = it) },
                            label = { Text(stringResource(R.string.settlement_item_desc)) },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth(),
                        )
                        OutlinedTextField(
                            value = line.amountText,
                            onValueChange = { vm.updateItem(line.key, amount = it) },
                            label = { Text(stringResource(R.string.settlement_amount_myr)) },
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth(),
                        )
                    }
                }
            }
            OutlinedButton(
                onClick = {
                    UiHaptics.tick(view)
                    vm.addItem()
                },
                modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
            ) {
                Icon(Icons.Outlined.Add, contentDescription = null, modifier = Modifier.size(18.dp))
                Spacer(Modifier.width(Spacing.xs))
                Text(stringResource(R.string.settlement_add_item))
            }

            OutlinedTextField(
                value = state.subsidyText,
                onValueChange = { v -> vm.update { it.copy(subsidyText = v) } },
                label = { Text(stringResource(R.string.settlement_subsidy_myr)) },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            val dueDateInteraction = remember { MutableInteractionSource() }
            LaunchedEffect(dueDateInteraction) {
                dueDateInteraction.interactions.collect { interaction ->
                    if (interaction is PressInteraction.Release) {
                        UiHaptics.tick(view)
                        showDueDatePicker = true
                    }
                }
            }
            OutlinedTextField(
                value = if (state.dueDate.isBlank()) "" else KlTime.displayDate(state.dueDate),
                onValueChange = {},
                readOnly = true,
                interactionSource = dueDateInteraction,
                label = { Text(stringResource(R.string.settlement_due_date_field)) },
                placeholder = { Text("YYYY-MM-DD") },
                trailingIcon = {
                    Row {
                        if (state.dueDate.isNotBlank()) {
                            IconButton(
                                onClick = {
                                    UiHaptics.tick(view)
                                    vm.update { it.copy(dueDate = "") }
                                },
                            ) {
                                Icon(Icons.Outlined.Close, contentDescription = stringResource(R.string.a11y_clear))
                            }
                        }
                        IconButton(
                            onClick = {
                                UiHaptics.tick(view)
                                showDueDatePicker = true
                            },
                        ) {
                            Icon(Icons.Outlined.CalendarMonth, contentDescription = stringResource(R.string.settlement_pick_due_date))
                        }
                    }
                },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )

            SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
                val modes = listOf("equal" to R.string.settlement_split_equal, "custom" to R.string.settlement_split_custom)
                modes.forEachIndexed { idx, (key, res) ->
                    SegmentedButton(
                        selected = state.splitMode == key,
                        onClick = {
                            UiHaptics.tick(view)
                            vm.update { it.copy(splitMode = key) }
                        },
                        shape = SegmentedButtonDefaults.itemShape(idx, modes.size),
                    ) {
                        Text(stringResource(res))
                    }
                }
            }

            OutlinedCard(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    SectionHeader(stringResource(R.string.settlement_payee_section))
                    OutlinedTextField(
                        value = state.payeeName,
                        onValueChange = { v -> vm.update { it.copy(payeeName = v) } },
                        label = { Text(stringResource(R.string.settlement_payee_name)) },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    OutlinedTextField(
                        value = state.payeeAccount,
                        onValueChange = { v -> vm.update { it.copy(payeeAccount = v) } },
                        label = { Text(stringResource(R.string.settlement_payee_account)) },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        OutlinedButton(
                            onClick = { qrPicker.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly)) },
                            modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                        ) {
                            Icon(Icons.Outlined.QrCode2, contentDescription = null, modifier = Modifier.size(18.dp))
                            Spacer(Modifier.width(Spacing.xs))
                            Text(
                                stringResource(
                                    if (state.payeeQrFileId == null) R.string.settlement_upload_qr else R.string.settlement_qr_ready,
                                ),
                            )
                        }
                        if (state.payeeQrFileId != null) {
                            OutlinedButton(
                                onClick = { showQrZoom = true },
                                modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                            ) {
                                Text(stringResource(R.string.settlement_qr_preview))
                            }
                        }
                    }
                }
            }

            SectionHeader(stringResource(R.string.settlement_participants_section))
            OutlinedTextField(
                value = participantQuery,
                onValueChange = { participantQuery = it },
                placeholder = { Text(stringResource(R.string.settlement_search_participants)) },
                leadingIcon = { Icon(Icons.Outlined.Search, contentDescription = null) },
                trailingIcon = if (participantQuery.isNotEmpty()) {
                    {
                        IconButton(onClick = { participantQuery = "" }) {
                            Icon(Icons.Outlined.Close, contentDescription = stringResource(R.string.a11y_clear))
                        }
                    }
                } else {
                    null
                },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                AssistChip(
                    onClick = {
                        UiHaptics.tick(view)
                        vm.selectParticipants(visibleIds, true)
                    },
                    enabled = visibleIds.isNotEmpty(),
                    label = { Text(stringResource(R.string.settlement_select_all)) },
                )
                AssistChip(
                    onClick = {
                        UiHaptics.tick(view)
                        vm.selectParticipants(visibleIds, false)
                    },
                    enabled = visibleIds.isNotEmpty(),
                    label = { Text(stringResource(R.string.settlement_select_none)) },
                )
            }

            OutlinedCard(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    visibleParticipants.forEachIndexed { idx, p ->
                        if (idx > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                        val shareCents = equalShareMap[p.userId]
                        Column {
                            Row(
                                Modifier
                                    .fillMaxWidth()
                                    .defaultMinSize(minHeight = 48.dp)
                                    .toggleable(
                                        value = p.selected,
                                        role = Role.Checkbox,
                                        onValueChange = {
                                            UiHaptics.tick(view)
                                            vm.toggleParticipant(p.userId)
                                        },
                                    ),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Checkbox(checked = p.selected, onCheckedChange = null)
                                Text(p.name, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f).padding(start = Spacing.xs))
                                if (shareCents != null) {
                                    Text(Money.rm(shareCents), style = MaterialTheme.typography.bodySmall.tabular())
                                }
                            }
                            if (p.selected && state.splitMode == "custom") {
                                OutlinedTextField(
                                    value = p.customAmountText,
                                    onValueChange = { vm.setParticipantAmount(p.userId, it) },
                                    label = { Text(stringResource(R.string.settlement_amount_myr)) },
                                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                                    singleLine = true,
                                    modifier = Modifier.fillMaxWidth().padding(bottom = Spacing.xs),
                                )
                            }
                        }
                    }
                }
            }

            if (state.existingStatus == "published") {
                OutlinedTextField(
                    value = state.revisionReason,
                    onValueChange = { v -> vm.update { it.copy(revisionReason = v) } },
                    label = { Text(stringResource(R.string.settlement_revision_reason)) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
            }
        }
    }

    if (showDueDatePicker) {
        val initialDate = remember(state.dueDate) {
            runCatching { LocalDate.parse(state.dueDate.trim()) }
                .getOrElse { LocalDate.parse(KlTime.today()) }
        }
        val datePickerState = rememberDatePickerState(
            initialSelectedDateMillis = DatePickerUtc.toPickerMillis(initialDate),
        )
        DatePickerDialog(
            onDismissRequest = { showDueDatePicker = false },
            confirmButton = {
                TextButton(
                    onClick = {
                        datePickerState.selectedDateMillis?.let { millis ->
                            val picked = DatePickerUtc.fromPickerMillis(millis)
                            vm.update { it.copy(dueDate = picked.toString()) }
                            UiHaptics.confirm(view)
                        }
                        showDueDatePicker = false
                    },
                ) {
                    Text(stringResource(R.string.common_confirm))
                }
            },
            dismissButton = {
                TextButton(onClick = { showDueDatePicker = false }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        ) {
            DatePicker(
                state = datePickerState,
                modifier = Modifier.verticalScroll(rememberScrollState()),
            )
        }
    }

    if (showQrZoom && state.payeeQrFileId != null) {
        ZoomableImageDialog(
            model = container.fileUrl(state.payeeQrFileId!!),
            title = stringResource(R.string.settlement_qr_preview),
            onDismiss = { showQrZoom = false },
        )
    }
}
