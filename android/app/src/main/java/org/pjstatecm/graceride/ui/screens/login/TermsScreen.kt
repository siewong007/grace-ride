package org.pjstatecm.graceride.ui.screens.login

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.selection.toggleable
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors

/**
 * Terms gate, mirroring the web's TermsGate: the full terms text, an opt-in
 * checkbox that is never pre-ticked, and an explicit agree action that records
 * the version server-side (POST /auth/terms {accepted:true}).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TermsScreen(
    container: AppContainer,
    readOnly: Boolean = false,
    onBack: () -> Unit = {},
    onOpenLicenses: () -> Unit = {},
) {
    var checked by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    val sections = listOf(
        R.string.terms_service_title to R.string.terms_service_body,
        R.string.terms_who_title to R.string.terms_who_body,
        R.string.terms_controller_title to R.string.terms_controller_body,
        R.string.terms_info_title to R.string.terms_info_body,
        R.string.terms_sensitive_title to R.string.terms_sensitive_body,
        R.string.terms_purpose_title to R.string.terms_purpose_body,
        R.string.terms_location_title to R.string.terms_location_body,
        R.string.terms_see_title to R.string.terms_see_body,
        R.string.terms_third_title to R.string.terms_third_body,
        R.string.terms_external_title to R.string.terms_external_body,
        R.string.terms_retention_title to R.string.terms_retention_body,
        R.string.terms_rights_title to R.string.terms_rights_body,
        R.string.terms_drivers_title to R.string.terms_drivers_body,
        R.string.terms_conduct_title to R.string.terms_conduct_body,
        R.string.terms_ownership_title to R.string.terms_ownership_body,
        R.string.terms_changes_title to R.string.terms_changes_body,
        R.string.terms_accept_title to R.string.terms_accept_body,
    )

    val listContent: @Composable (Modifier) -> Unit = { mod ->
        LazyColumn(
            mod.testTag("termsList"),
            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
            contentPadding = PaddingValues(vertical = Spacing.sm),
        ) {
            item {
                OutlinedCard(Modifier.fillMaxWidth()) {
                    Column(Modifier.padding(Spacing.sm)) {
                        Text(stringResource(R.string.terms_title), style = MaterialTheme.typography.headlineSmall, color = MaterialTheme.colorScheme.onSurface)
                        Text(stringResource(R.string.terms_updated), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.primary, modifier = Modifier.padding(top = Spacing.xxs))
                        Text(stringResource(R.string.terms_intro), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurface, modifier = Modifier.padding(top = Spacing.xs))
                    }
                }
            }
            items(sections.size) { i ->
                val (title, body) = sections[i]
                Column(Modifier.fillMaxWidth().padding(vertical = Spacing.xs)) {
                    Text(stringResource(title), style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.onSurface)
                    Text(stringResource(body), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(top = Spacing.xxs))
                    HorizontalDivider(
                        color = MaterialTheme.colorScheme.outlineVariant,
                        modifier = Modifier.padding(top = Spacing.sm),
                    )
                }
            }
            if (readOnly) {
                item {
                    OutlinedButton(
                        onClick = onOpenLicenses,
                        modifier = Modifier.fillMaxWidth().height(48.dp),
                    ) {
                        Text(stringResource(R.string.terms_open_licenses))
                    }
                }
            } else {
                item {
                    OutlinedCard(Modifier.fillMaxWidth()) {
                        Row(
                            Modifier
                                .fillMaxWidth()
                                .defaultMinSize(minHeight = 52.dp)
                                .toggleable(value = checked, role = Role.Checkbox, onValueChange = { checked = it })
                                .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Checkbox(checked = checked, onCheckedChange = null)
                            Text(
                                stringResource(R.string.terms_opt_in),
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurface,
                                modifier = Modifier.padding(start = Spacing.xs),
                            )
                        }
                    }
                }
                item {
                    Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        error?.let {
                            Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium)
                        }
                        Button(
                            onClick = {
                                if (!busy) {
                                    busy = true
                                    error = null
                                    scope.launch {
                                        try {
                                            container.auth.acceptTerms()
                                        } catch (e: ApiException) {
                                            error = e.message
                                        } finally {
                                            busy = false
                                        }
                                    }
                                }
                            },
                            enabled = checked && !busy,
                            colors = graceRidePrimaryButtonColors(),
                            modifier = Modifier.fillMaxWidth().height(52.dp),
                        ) {
                            if (busy) {
                                InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                            } else {
                                Text(stringResource(R.string.terms_agree))
                            }
                        }
                        TextButton(
                            onClick = { scope.launch { container.auth.logout() } },
                            modifier = Modifier.fillMaxWidth(),
                        ) { Text(stringResource(R.string.terms_logout)) }
                    }
                }
            }
        }
    }

    if (readOnly) {
        Scaffold(
            topBar = {
                TopAppBar(
                    title = { Text(stringResource(R.string.terms_title)) },
                    navigationIcon = {
                        IconButton(onClick = onBack) {
                            Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.terms_back))
                        }
                    },
                    colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background),
                )
            },
            containerColor = MaterialTheme.colorScheme.background,
        ) { padding ->
            listContent(Modifier.fillMaxSize().padding(padding).padding(horizontal = Spacing.sm))
        }
    } else {
        listContent(
            Modifier
                .fillMaxSize()
                .statusBarsPadding()
                .navigationBarsPadding()
                .padding(horizontal = Spacing.sm),
        )
    }
}
