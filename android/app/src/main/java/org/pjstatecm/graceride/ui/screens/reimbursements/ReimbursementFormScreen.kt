package org.pjstatecm.graceride.ui.screens.reimbursements

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.Receipt
import androidx.compose.material.icons.outlined.UploadFile
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
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
import androidx.compose.material3.TopAppBar
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
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import coil.compose.AsyncImage
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.AdaptiveContentFrame
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.components.ZoomableImageDialog
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun ReimbursementFormScreen(
    container: AppContainer,
    nav: NavHostController,
    editId: Long? = null,
) {
    org.pjstatecm.graceride.ui.SecureScreenEffect()
    val vm: ReimbursementFormViewModel = viewModel(
        key = "reimb-form-${editId ?: "new"}",
        factory = ReimbursementFormViewModel.factory(container, editId),
    )
    val state by vm.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val view = LocalView.current
    var eventExpanded by remember { mutableStateOf(false) }
    var rideExpanded by remember { mutableStateOf(false) }
    var zoomReceipt by remember { mutableStateOf(false) }

    val receiptPicker = rememberLauncherForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri ->
        if (uri != null) vm.uploadReceipt(context, uri)
    }

    LaunchedEffect(state.savedId) {
        val id = state.savedId ?: return@LaunchedEffect
        nav.navigate(AppRoute.ReimbursementDetail(id)) {
            popUpTo(nav.currentBackStackEntry?.destination?.id ?: return@navigate) { inclusive = true }
        }
    }

    val invalidMsg = stringResource(R.string.reimb_form_required)

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        stringResource(
                            if (state.editId == null) R.string.reimb_new else R.string.reimb_resubmit,
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
                            state.error?.let { err ->
                                Text(err, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                            }
                            Button(
                                onClick = {
                                    UiHaptics.confirm(view)
                                    vm.submit(invalidMsg)
                                },
                                enabled = !state.submitting && !state.uploadingReceipt,
                                colors = graceRidePrimaryButtonColors(),
                                modifier = Modifier.fillMaxWidth().height(50.dp),
                            ) {
                                if (state.submitting) {
                                    InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                } else {
                                    Text(stringResource(R.string.reimb_submit))
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
        Column(
            Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(Spacing.sm),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            AdaptiveContentFrame(maxWidth = 720.dp) {
                Column(verticalArrangement = Arrangement.spacedBy(Spacing.sm)) {
            OutlinedCard(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        Icon(
                            Icons.Outlined.Receipt,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.size(18.dp),
                        )
                        SectionHeader(stringResource(R.string.reimb_receipt))
                    }
                    state.receiptFileId?.let { rfid ->
                        OutlinedCard(
                            onClick = { zoomReceipt = true },
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Column(Modifier.padding(Spacing.xs), horizontalAlignment = Alignment.CenterHorizontally) {
                                AsyncImage(
                                    model = container.fileUrl(rfid),
                                    contentDescription = stringResource(R.string.reimb_receipt),
                                    contentScale = ContentScale.Fit,
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .height(160.dp)
                                        .clip(RoundedCornerShape(8.dp)),
                                )
                                Text(
                                    stringResource(R.string.reimb_tap_zoom_receipt),
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    }
                    OutlinedButton(
                        onClick = { receiptPicker.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly)) },
                        enabled = !state.uploadingReceipt,
                        modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
                    ) {
                        if (state.uploadingReceipt) {
                            InlineButtonSpinner()
                        } else {
                            Icon(Icons.Outlined.UploadFile, contentDescription = null, modifier = Modifier.size(18.dp))
                            Spacer(Modifier.width(Spacing.xs))
                            Text(
                                stringResource(
                                    if (state.receiptFileId == null) R.string.reimb_upload_receipt else R.string.reimb_receipt_ready,
                                ),
                            )
                        }
                    }
                    if (state.extractingOcr) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            InlineButtonSpinner()
                            Text(
                                stringResource(R.string.reimb_ocr_scanning),
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                    if (state.ocrCandidates.isNotEmpty()) {
                        FlowRow(
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                        ) {
                            state.ocrCandidates.forEach { candidate ->
                                AssistChip(
                                    onClick = {
                                        UiHaptics.confirm(view)
                                        vm.update { current ->
                                            val curDesc = current.description.trim()
                                            val updated = when {
                                                curDesc.isEmpty() -> candidate
                                                candidate in curDesc -> curDesc
                                                else -> "$curDesc · $candidate"
                                            }
                                            current.copy(description = updated)
                                        }
                                    },
                                    label = { Text(stringResource(R.string.reimb_ocr_use_ref, candidate)) },
                                )
                            }
                        }
                    }
                }
            }

            SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
                val kinds = listOf("grab" to R.string.reimb_kind_grab, "mileage" to R.string.reimb_kind_mileage)
                kinds.forEachIndexed { idx, (key, res) ->
                    SegmentedButton(
                        selected = state.kind == key,
                        onClick = {
                            UiHaptics.tick(view)
                            vm.update { it.copy(kind = key) }
                        },
                        shape = SegmentedButtonDefaults.itemShape(idx, kinds.size),
                    ) {
                        Text(stringResource(res))
                    }
                }
            }

            if (state.events.isNotEmpty()) {
                ExposedDropdownMenuBox(expanded = eventExpanded, onExpandedChange = { eventExpanded = it }) {
                    OutlinedTextField(
                        value = state.events.firstOrNull { it.id == state.selectedEventId }?.title ?: "—",
                        onValueChange = {},
                        readOnly = true,
                        label = { Text(stringResource(R.string.reimb_event)) },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = eventExpanded) },
                        modifier = Modifier.menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
                    )
                    ExposedDropdownMenu(expanded = eventExpanded, onDismissRequest = { eventExpanded = false }) {
                        state.events.forEach { ev ->
                            DropdownMenuItem(
                                text = { Text(ev.title) },
                                onClick = {
                                    vm.update { it.copy(selectedEventId = ev.id) }
                                    eventExpanded = false
                                },
                            )
                        }
                    }
                }
            }

            if (state.rides.isNotEmpty()) {
                ExposedDropdownMenuBox(expanded = rideExpanded, onExpandedChange = { rideExpanded = it }) {
                    val chosen = state.rides.firstOrNull { it.id == state.selectedRideId }
                    OutlinedTextField(
                        value = chosen?.let { "#${it.id} · ${KlTime.display(it.pickupAt)}" } ?: "—",
                        onValueChange = {},
                        readOnly = true,
                        label = { Text(stringResource(R.string.reimb_ride_optional)) },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = rideExpanded) },
                        modifier = Modifier.menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
                    )
                    ExposedDropdownMenu(expanded = rideExpanded, onDismissRequest = { rideExpanded = false }) {
                        DropdownMenuItem(
                            text = { Text("—") },
                            onClick = {
                                vm.update { it.copy(selectedRideId = null) }
                                rideExpanded = false
                            },
                        )
                        state.rides.forEach { ride ->
                            DropdownMenuItem(
                                text = { Text("#${ride.id} · ${ride.area.name} · ${KlTime.display(ride.pickupAt)}") },
                                onClick = {
                                    vm.update {
                                        it.copy(
                                            selectedRideId = ride.id,
                                            selectedEventId = ride.event?.id ?: it.selectedEventId,
                                        )
                                    }
                                    rideExpanded = false
                                },
                            )
                        }
                    }
                }
            }

            OutlinedTextField(
                value = state.amountText,
                onValueChange = { v -> vm.update { it.copy(amountText = v) } },
                label = { Text(stringResource(R.string.reimb_amount_myr)) },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = state.description,
                onValueChange = { v -> vm.update { it.copy(description = v) } },
                label = { Text(stringResource(R.string.reimb_description)) },
                minLines = 2,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = state.payeeName,
                onValueChange = { v -> vm.update { it.copy(payeeName = v) } },
                label = { Text(stringResource(R.string.reimb_payee)) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = state.payeePhone,
                onValueChange = { v -> vm.update { it.copy(payeePhone = v) } },
                label = { Text(stringResource(R.string.reimb_phone)) },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Phone),
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
                }
            }
        }
    }

    if (zoomReceipt && state.receiptFileId != null) {
        ZoomableImageDialog(
            model = container.fileUrl(state.receiptFileId!!),
            title = stringResource(R.string.reimb_receipt),
            onDismiss = { zoomReceipt = false },
        )
    }
}
