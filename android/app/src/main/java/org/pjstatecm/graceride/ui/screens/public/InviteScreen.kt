package org.pjstatecm.graceride.ui.screens.public

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
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
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.Key
import androidx.compose.material.icons.outlined.WarningAmber
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.GraceRideWordmark
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.TurnstileDialog
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun InviteScreen(
    container: AppContainer,
    token: String,
    onBackToLogin: () -> Unit,
    onFinished: () -> Unit,
) {
    val vm: InviteViewModel = viewModel(
        key = "invite_$token",
        factory = InviteViewModel.factory(container, token),
    )
    val state by vm.state.collectAsStateWithLifecycle()
    val context = LocalContext.current

    LaunchedEffect(state.done) {
        if (state.done) {
            onFinished()
        }
    }

    val preview = state.preview
    val isRecovery = preview?.kind == "recovery"
    val isFamily = preview?.kind == "family"
    val totalSteps = if (isRecovery) 2 else 3
    val visibleStep = if (isRecovery) (state.step - 1).coerceIn(1, 2) else state.step.coerceIn(1, 3)

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        if (isRecovery) {
                            stringResource(R.string.invite_recover_title)
                        } else {
                            stringResource(R.string.invite_title)
                        },
                    )
                },
                navigationIcon = {
                    if (state.step <= 2) {
                        IconButton(
                            onClick = {
                                if (!isRecovery && state.step == 2) vm.backToDetails() else onBackToLogin()
                            },
                        ) {
                            Icon(
                                Icons.AutoMirrored.Filled.ArrowBack,
                                contentDescription = stringResource(R.string.common_back),
                            )
                        }
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
                message = state.loadError ?: stringResource(R.string.invite_invalid_link),
                modifier = Modifier.padding(padding),
                onRetry = vm::load,
            )
            else -> {
                val errText = state.error ?: state.errorRes?.let { resId ->
                    if (resId == R.string.invite_name_too_long) {
                        stringResource(resId, 80)
                    } else {
                        stringResource(resId)
                    }
                }
                LazyColumn(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding)
                        .padding(horizontal = Spacing.sm),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    contentPadding = PaddingValues(vertical = Spacing.sm),
                    verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                ) {
                    item {
                        Column(
                            Modifier.widthIn(max = 560.dp).fillMaxWidth(),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            GraceRideWordmark()
                            Text(
                                stringResource(R.string.invite_step_of, visibleStep, totalSteps),
                                style = MaterialTheme.typography.labelLarge,
                                color = MaterialTheme.colorScheme.primary,
                            )
                            LinearProgressIndicator(
                                progress = { visibleStep.toFloat() / totalSteps.toFloat() },
                                modifier = Modifier.fillMaxWidth(),
                            )
                        }
                    }

                    if (isFamily && !preview?.familyName.isNullOrBlank()) {
                        item {
                            CalloutBanner(
                                icon = Icons.Outlined.Key,
                                text = stringResource(R.string.invite_family_prefix, preview!!.familyName!!),
                                containerColor = MaterialTheme.colorScheme.primaryContainer,
                                contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                                modifier = Modifier.widthIn(max = 560.dp).fillMaxWidth(),
                            )
                        }
                    }

                    when (state.step) {
                        1 -> item {
                            OutlinedCard(
                                modifier = Modifier.widthIn(max = 560.dp).fillMaxWidth(),
                                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                            ) {
                                Column(
                                    Modifier.padding(Spacing.sm),
                                    verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                                ) {
                                    Text(
                                        stringResource(R.string.invite_step_details),
                                        style = MaterialTheme.typography.titleMedium,
                                    )
                                    OutlinedTextField(
                                        value = state.name,
                                        onValueChange = vm::setName,
                                        label = { Text(stringResource(R.string.invite_name_label)) },
                                        singleLine = true,
                                        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Next),
                                        modifier = Modifier.fillMaxWidth(),
                                    )
                                    OutlinedTextField(
                                        value = state.email,
                                        onValueChange = vm::setEmail,
                                        label = { Text(stringResource(R.string.invite_email_label)) },
                                        singleLine = true,
                                        keyboardOptions = KeyboardOptions(
                                            keyboardType = KeyboardType.Email,
                                            imeAction = ImeAction.Next,
                                        ),
                                        modifier = Modifier.fillMaxWidth(),
                                    )
                                    OutlinedTextField(
                                        value = state.phone,
                                        onValueChange = vm::setPhone,
                                        enabled = !isFamily,
                                        label = { Text(stringResource(R.string.invite_phone_label)) },
                                        supportingText = { Text(stringResource(R.string.invite_phone_hint)) },
                                        singleLine = true,
                                        keyboardOptions = KeyboardOptions(
                                            keyboardType = KeyboardType.Phone,
                                            imeAction = ImeAction.Done,
                                        ),
                                        modifier = Modifier.fillMaxWidth(),
                                    )
                                    errText?.let {
                                        CalloutBanner(
                                            icon = Icons.Outlined.WarningAmber,
                                            text = it,
                                            containerColor = MaterialTheme.colorScheme.errorContainer,
                                            contentColor = MaterialTheme.colorScheme.onErrorContainer,
                                        )
                                    }
                                    Button(
                                        onClick = vm::continueFromDetails,
                                        colors = graceRidePrimaryButtonColors(),
                                        modifier = Modifier.fillMaxWidth().height(50.dp),
                                    ) {
                                        Text(stringResource(R.string.invite_continue))
                                    }
                                }
                            }
                        }

                        2 -> {
                            item {
                                OutlinedCard(
                                    modifier = Modifier.widthIn(max = 560.dp).fillMaxWidth(),
                                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                                ) {
                                    Column(
                                        Modifier.padding(Spacing.sm),
                                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    ) {
                                        Text(
                                            stringResource(R.string.invite_step_terms),
                                            style = MaterialTheme.typography.titleMedium,
                                        )
                                        if (isRecovery) {
                                            Text(
                                                stringResource(R.string.invite_recover_desc),
                                                style = MaterialTheme.typography.bodyMedium,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                            )
                                        }
                                        Text(
                                            stringResource(R.string.terms_title),
                                            style = MaterialTheme.typography.titleSmall,
                                        )
                                        Text(
                                            stringResource(R.string.terms_intro),
                                            style = MaterialTheme.typography.bodyMedium,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                        listOf(
                                            R.string.terms_info_title to R.string.terms_info_body,
                                            R.string.terms_purpose_title to R.string.terms_purpose_body,
                                            R.string.terms_location_title to R.string.terms_location_body,
                                            R.string.terms_conduct_title to R.string.terms_conduct_body,
                                            R.string.terms_accept_title to R.string.terms_accept_body,
                                        ).forEach { (titleRes, bodyRes) ->
                                            Text(
                                                stringResource(titleRes),
                                                style = MaterialTheme.typography.titleSmall,
                                            )
                                            Text(
                                                stringResource(bodyRes),
                                                style = MaterialTheme.typography.bodySmall,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                            )
                                        }
                                    }
                                }
                            }
                            item {
                                OutlinedCard(
                                    modifier = Modifier.widthIn(max = 560.dp).fillMaxWidth(),
                                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                                ) {
                                    Row(
                                        Modifier
                                            .fillMaxWidth()
                                            .defaultMinSize(minHeight = 52.dp)
                                            .toggleable(
                                                value = state.termsAccepted,
                                                role = Role.Checkbox,
                                                onValueChange = vm::setTermsAccepted,
                                            )
                                            .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        Checkbox(checked = state.termsAccepted, onCheckedChange = null)
                                        Text(
                                            stringResource(R.string.terms_opt_in),
                                            style = MaterialTheme.typography.bodyMedium,
                                            modifier = Modifier.padding(start = Spacing.xs),
                                        )
                                    }
                                }
                            }
                            item {
                                Column(Modifier.widthIn(max = 560.dp).fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                    errText?.let {
                                        CalloutBanner(
                                            icon = Icons.Outlined.WarningAmber,
                                            text = it,
                                            containerColor = MaterialTheme.colorScheme.errorContainer,
                                            contentColor = MaterialTheme.colorScheme.onErrorContainer,
                                        )
                                    }
                                    if (state.emailTaken) {
                                        Row(
                                            Modifier.fillMaxWidth(),
                                            horizontalArrangement = Arrangement.SpaceBetween,
                                            verticalAlignment = Alignment.CenterVertically,
                                        ) {
                                            Text(
                                                stringResource(R.string.invite_email_taken_hint),
                                                style = MaterialTheme.typography.bodyMedium,
                                            )
                                            TextButton(onClick = onBackToLogin) {
                                                Text(stringResource(R.string.invite_sign_in_instead))
                                            }
                                        }
                                    }
                                    Button(
                                        onClick = { vm.acceptTermsAndJoin() },
                                        enabled = !state.submitting,
                                        colors = graceRidePrimaryButtonColors(),
                                        modifier = Modifier.fillMaxWidth().height(50.dp),
                                    ) {
                                        if (state.submitting) {
                                            InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                        } else {
                                            Text(stringResource(R.string.invite_accept_join))
                                        }
                                    }
                                }
                            }
                        }

                        else -> item {
                            OutlinedCard(
                                modifier = Modifier.widthIn(max = 560.dp).fillMaxWidth(),
                                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                            ) {
                                Column(
                                    Modifier.padding(Spacing.sm),
                                    verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                                ) {
                                    Text(
                                        stringResource(R.string.invite_passkey_title),
                                        style = MaterialTheme.typography.titleMedium,
                                    )
                                    Text(
                                        stringResource(R.string.invite_passkey_desc),
                                        style = MaterialTheme.typography.bodyMedium,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                    errText?.let {
                                        CalloutBanner(
                                            icon = Icons.Outlined.WarningAmber,
                                            text = it,
                                            containerColor = MaterialTheme.colorScheme.errorContainer,
                                            contentColor = MaterialTheme.colorScheme.onErrorContainer,
                                        )
                                    }
                                    Button(
                                        onClick = { vm.registerPasskey(context) },
                                        enabled = !state.registeringPasskey,
                                        colors = graceRidePrimaryButtonColors(),
                                        modifier = Modifier.fillMaxWidth().height(50.dp),
                                    ) {
                                        if (state.registeringPasskey) {
                                            InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                        } else {
                                            Icon(Icons.Outlined.Key, contentDescription = null, modifier = Modifier.size(18.dp))
                                            Spacer(Modifier.width(Spacing.xs))
                                            Text(stringResource(R.string.invite_add_passkey))
                                        }
                                    }
                                    OutlinedButton(
                                        onClick = vm::promptSkipPasskey,
                                        enabled = !state.registeringPasskey,
                                        modifier = Modifier.fillMaxWidth().height(48.dp),
                                    ) {
                                        Text(stringResource(R.string.invite_skip_passkey))
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if (state.confirmSkipOpen) {
        AlertDialog(
            onDismissRequest = vm::dismissSkipPasskey,
            title = { Text(stringResource(R.string.invite_skip_title)) },
            text = { Text(stringResource(R.string.invite_skip_body)) },
            confirmButton = {
                TextButton(onClick = vm::confirmSkipPasskey) {
                    Text(stringResource(R.string.invite_skip_confirm))
                }
            },
            dismissButton = {
                TextButton(onClick = vm::dismissSkipPasskey) {
                    Text(stringResource(R.string.invite_skip_cancel))
                }
            },
        )
    }

    if (state.turnstilePrompted && !state.turnstileSiteKey.isNullOrBlank()) {
        TurnstileDialog(
            siteKey = state.turnstileSiteKey!!,
            baseUrl = container.baseUrl,
            action = "invite_accept",
            onToken = { tok -> vm.acceptTermsAndJoin(tok) },
            onDismiss = vm::dismissTurnstile,
        )
    }
}
