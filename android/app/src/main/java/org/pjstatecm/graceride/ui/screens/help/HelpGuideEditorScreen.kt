package org.pjstatecm.graceride.ui.screens.help

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
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
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.ArrowDownward
import androidx.compose.material.icons.outlined.ArrowUpward
import androidx.compose.material.icons.outlined.DeleteOutline
import androidx.compose.material.icons.outlined.Image
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
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
import androidx.compose.material3.Switch
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import java.util.Locale
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HelpGuideEditorScreen(
    container: AppContainer,
    nav: NavHostController,
    key: String? = null,
) {
    val vm: HelpGuideEditorViewModel = viewModel(
        key = "guide-edit-${key ?: "new"}",
        factory = HelpGuideEditorViewModel.factory(container, key),
    )
    val state by vm.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val view = LocalView.current
    var editorLang by remember { mutableStateOf(if (Locale.getDefault().language == "zh") "zh" else "en") }
    var pickingStepKey by remember { mutableStateOf<Long?>(null) }
    var confirmDelete by remember { mutableStateOf(false) }

    val imgPicker = rememberLauncherForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri ->
        val stepKey = pickingStepKey
        pickingStepKey = null
        if (uri != null && stepKey != null) vm.uploadStepImage(context, stepKey, uri)
    }

    LaunchedEffect(state.deleted) {
        if (state.deleted) nav.popBackStack()
    }
    LaunchedEffect(state.savedKey) {
        val sk = state.savedKey ?: return@LaunchedEffect
        nav.navigate(AppRoute.HelpGuideDetail(sk)) {
            popUpTo(nav.currentBackStackEntry?.destination?.id ?: return@navigate) { inclusive = true }
        }
    }

    val missingMsg = stringResource(R.string.help_guide_required)

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        stringResource(
                            if (state.guideId == null) R.string.help_guide_new else R.string.help_guide_edit,
                        ),
                    )
                },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
                actions = {
                    if (state.guideId != null) {
                        IconButton(onClick = { confirmDelete = true }) {
                            Icon(Icons.Outlined.DeleteOutline, contentDescription = stringResource(R.string.common_delete))
                        }
                    }
                },
            )
        },
        bottomBar = {
            if (!state.loading) {
                Surface(tonalElevation = 3.dp, shadowElevation = 4.dp) {
                    Column(
                        Modifier
                            .fillMaxWidth()
                            .navigationBarsPadding()
                            .imePadding()
                            .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        state.error?.let { err ->
                            Text(err, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                        }
                        Button(
                            onClick = {
                                UiHaptics.confirm(view)
                                vm.save(missingMsg)
                            },
                            enabled = !state.saving,
                            colors = graceRidePrimaryButtonColors(),
                            modifier = Modifier.fillMaxWidth().height(50.dp),
                        ) {
                            if (state.saving) InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                            else Text(stringResource(R.string.common_save))
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
        val c = LocalStatusColors.current
        Column(
            Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
        ) {
            Surface(
                color = if (state.completenessGaps == 0) c.successSoft else c.warningSoft,
                shape = MaterialTheme.shapes.small,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(
                    text = if (state.completenessGaps == 0) {
                        stringResource(R.string.help_guide_complete)
                    } else {
                        stringResource(R.string.help_guide_gaps, state.completenessGaps)
                    },
                    style = MaterialTheme.typography.labelMedium,
                    color = if (state.completenessGaps == 0) c.success else c.warning,
                    modifier = Modifier.padding(Spacing.xs),
                )
            }

            OutlinedTextField(
                value = state.slug,
                onValueChange = { v -> vm.update { it.copy(slug = v) } },
                label = { Text(stringResource(R.string.help_guide_slug)) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )

            SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
                val opts = listOf("member" to R.string.help_audience_members, "staff" to R.string.help_audience_staff)
                opts.forEachIndexed { idx, (k, res) ->
                    SegmentedButton(
                        selected = state.audience == k,
                        onClick = {
                            UiHaptics.tick(view)
                            vm.update { it.copy(audience = k) }
                        },
                        shape = SegmentedButtonDefaults.itemShape(idx, opts.size),
                    ) {
                        Text(stringResource(res))
                    }
                }
            }

            Row(
                Modifier
                    .fillMaxWidth()
                    .defaultMinSize(minHeight = 48.dp)
                    .toggleable(
                        value = state.published,
                        role = Role.Switch,
                        onValueChange = { p ->
                            UiHaptics.tick(view)
                            vm.update { it.copy(published = p) }
                        },
                    ),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(stringResource(R.string.help_guide_published), style = MaterialTheme.typography.bodyMedium)
                Switch(checked = state.published, onCheckedChange = null)
            }

            val enStatus = if (state.enGaps == 0) {
                stringResource(R.string.help_guide_lang_ready)
            } else {
                stringResource(R.string.help_guide_lang_gaps, state.enGaps)
            }
            val zhStatus = if (state.zhGaps == 0) {
                stringResource(R.string.help_guide_lang_ready)
            } else {
                stringResource(R.string.help_guide_lang_gaps, state.zhGaps)
            }
            SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
                val langTabs = listOf(
                    "en" to "${stringResource(R.string.help_guide_lang_en)} · $enStatus",
                    "zh" to "${stringResource(R.string.help_guide_lang_zh)} · $zhStatus",
                )
                langTabs.forEachIndexed { idx, (langKey, label) ->
                    SegmentedButton(
                        selected = editorLang == langKey,
                        onClick = {
                            UiHaptics.tick(view)
                            editorLang = langKey
                        },
                        shape = SegmentedButtonDefaults.itemShape(idx, langTabs.size),
                        icon = {},
                    ) {
                        Text(label, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    }
                }
            }

            if (editorLang == "en") {
                OutlinedTextField(
                    value = state.titleEn,
                    onValueChange = { v -> vm.update { it.copy(titleEn = v) } },
                    label = { Text(stringResource(R.string.help_guide_title_en)) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                OutlinedTextField(
                    value = state.summaryEn,
                    onValueChange = { v -> vm.update { it.copy(summaryEn = v) } },
                    label = { Text(stringResource(R.string.help_guide_summary_en)) },
                    modifier = Modifier.fillMaxWidth(),
                )
            } else {
                OutlinedTextField(
                    value = state.titleZh,
                    onValueChange = { v -> vm.update { it.copy(titleZh = v) } },
                    label = { Text(stringResource(R.string.help_guide_title_zh)) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                OutlinedTextField(
                    value = state.summaryZh,
                    onValueChange = { v -> vm.update { it.copy(summaryZh = v) } },
                    label = { Text(stringResource(R.string.help_guide_summary_zh)) },
                    modifier = Modifier.fillMaxWidth(),
                )
            }

            SectionHeader(stringResource(R.string.help_guide_steps_header))
            state.steps.forEachIndexed { idx, step ->
                OutlinedCard(Modifier.fillMaxWidth()) {
                    Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                            Text("#${idx + 1}", style = MaterialTheme.typography.titleSmall)
                            Row {
                                IconButton(
                                    onClick = {
                                        UiHaptics.tick(view)
                                        vm.moveStep(step.localKey, -1)
                                    },
                                    enabled = idx > 0,
                                ) {
                                    Icon(Icons.Outlined.ArrowUpward, contentDescription = stringResource(R.string.help_move_up))
                                }
                                IconButton(
                                    onClick = {
                                        UiHaptics.tick(view)
                                        vm.moveStep(step.localKey, 1)
                                    },
                                    enabled = idx < state.steps.lastIndex,
                                ) {
                                    Icon(Icons.Outlined.ArrowDownward, contentDescription = stringResource(R.string.help_move_down))
                                }
                                if (state.steps.size > 1) {
                                    IconButton(
                                        onClick = {
                                            UiHaptics.tick(view)
                                            vm.removeStep(step.localKey)
                                        },
                                    ) {
                                        Icon(Icons.Outlined.DeleteOutline, contentDescription = stringResource(R.string.common_delete))
                                    }
                                }
                            }
                        }
                        if (editorLang == "en") {
                            OutlinedTextField(
                                value = step.titleEn,
                                onValueChange = { v -> vm.updateStep(step.localKey) { it.copy(titleEn = v) } },
                                label = { Text(stringResource(R.string.help_guide_step_title_en)) },
                                singleLine = true,
                                modifier = Modifier.fillMaxWidth(),
                            )
                            OutlinedTextField(
                                value = step.bodyEn,
                                onValueChange = { v -> vm.updateStep(step.localKey) { it.copy(bodyEn = v) } },
                                label = { Text(stringResource(R.string.help_guide_step_body_en)) },
                                minLines = 2,
                                modifier = Modifier.fillMaxWidth(),
                            )
                        } else {
                            OutlinedTextField(
                                value = step.titleZh,
                                onValueChange = { v -> vm.updateStep(step.localKey) { it.copy(titleZh = v) } },
                                label = { Text(stringResource(R.string.help_guide_step_title_zh)) },
                                singleLine = true,
                                modifier = Modifier.fillMaxWidth(),
                            )
                            OutlinedTextField(
                                value = step.bodyZh,
                                onValueChange = { v -> vm.updateStep(step.localKey) { it.copy(bodyZh = v) } },
                                label = { Text(stringResource(R.string.help_guide_step_body_zh)) },
                                minLines = 2,
                                modifier = Modifier.fillMaxWidth(),
                            )
                        }
                        OutlinedTextField(
                            value = step.link,
                            onValueChange = { v -> vm.updateStep(step.localKey) { it.copy(link = v) } },
                            label = { Text(stringResource(R.string.help_guide_step_link)) },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth(),
                        )
                        OutlinedButton(
                            onClick = {
                                pickingStepKey = step.localKey
                                imgPicker.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly))
                            },
                            modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
                        ) {
                            Icon(Icons.Outlined.Image, contentDescription = null, modifier = Modifier.size(18.dp))
                            Spacer(Modifier.width(Spacing.xs))
                            Text(stringResource(if (step.imageFileId == null) R.string.help_attach_photo else R.string.help_photo_attached))
                        }
                    }
                }
            }
            OutlinedButton(
                onClick = {
                    UiHaptics.tick(view)
                    vm.addStep()
                },
                modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
            ) {
                Icon(Icons.Outlined.Add, contentDescription = null, modifier = Modifier.size(18.dp))
                Spacer(Modifier.width(Spacing.xs))
                Text(stringResource(R.string.help_guide_add_step))
            }
        }
    }

    if (confirmDelete) {
        AlertDialog(
            onDismissRequest = { confirmDelete = false },
            title = { Text(stringResource(R.string.common_delete)) },
            text = { Text(stringResource(R.string.help_guide_delete_confirm)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmDelete = false
                        vm.deleteGuide()
                    },
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                ) {
                    Text(stringResource(R.string.common_confirm))
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmDelete = false }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }
}
