package org.pjstatecm.graceride.ui.screens.profile

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.Logout
import androidx.compose.material.icons.outlined.ChevronRight
import androidx.compose.material.icons.outlined.Delete
import androidx.compose.material.icons.outlined.History
import androidx.compose.material.icons.outlined.Key
import androidx.compose.material.icons.outlined.Notifications
import androidx.compose.material.icons.outlined.Payments
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MenuAnchorType
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.theme.Spacing

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProfileScreen(container: AppContainer, nav: NavHostController) {
    val vm: ProfileViewModel = viewModel(factory = ProfileViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val activity = context as? android.app.Activity
    val me = state.me

    LazyColumn(
        Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
        verticalArrangement = Arrangement.spacedBy(Spacing.sm),
        contentPadding = androidx.compose.foundation.layout.PaddingValues(vertical = Spacing.sm),
    ) {
        item {
            Column(Modifier.padding(vertical = Spacing.xs)) {
                Text(me?.name ?: "", style = MaterialTheme.typography.headlineSmall, color = MaterialTheme.colorScheme.onSurface)
                Text(me?.email ?: "", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                if (!me?.roleNames.isNullOrEmpty() || !me?.roles.isNullOrEmpty()) {
                    Text(
                        (me?.roles ?: emptyList()).joinToString(" · ") { me?.roleNames?.get(it) ?: it },
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.padding(top = Spacing.xxs),
                    )
                }
            }
        }

        item { SectionHeader(stringResource(R.string.profile_section_rides)) }
        item {
            OutlinedCard(Modifier.fillMaxWidth()) {
                Column {
                    ProfileRow(stringResource(R.string.rides_title), Icons.Outlined.ChevronRight) { nav.navigate(Routes.RIDES) }
                    HorizontalDivider()
                    ProfileRow(stringResource(R.string.history_title), Icons.Outlined.History) { nav.navigate(Routes.HISTORY) }
                    HorizontalDivider()
                    ProfileRow(stringResource(R.string.costs_title), Icons.Outlined.Payments) { nav.navigate(Routes.COSTS) }
                }
            }
        }

        item { SectionHeader(stringResource(R.string.profile_security)) }
        item {
            OutlinedCard(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(Spacing.sm)) {
                    Text(stringResource(R.string.profile_passkeys), style = MaterialTheme.typography.titleSmall)
                    state.passkeys.forEach { key ->
                        Row(
                            Modifier.fillMaxWidth().padding(top = Spacing.xxs),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Icon(Icons.Outlined.Key, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
                                Text(
                                    key.label.ifBlank { "Passkey #${key.id}" },
                                    style = MaterialTheme.typography.bodyMedium,
                                    modifier = Modifier.padding(start = Spacing.xs),
                                )
                            }
                            IconButton(onClick = { vm.deletePasskey(key.id) }) {
                                Icon(Icons.Outlined.Delete, contentDescription = stringResource(R.string.common_cancel), tint = MaterialTheme.colorScheme.error)
                            }
                        }
                    }
                    OutlinedButton(
                        onClick = { vm.addPasskey(context) },
                        enabled = !state.busy,
                        modifier = Modifier.padding(top = Spacing.xs),
                    ) { Text(stringResource(R.string.profile_add_passkey)) }

                    HorizontalDivider(Modifier.padding(vertical = Spacing.sm))

                    var lockExpanded by remember { mutableStateOf(false) }
                    val lockOptions = listOf(0, 5, 15, 30, 60)
                    ExposedDropdownMenuBox(expanded = lockExpanded, onExpandedChange = { lockExpanded = it }) {
                        OutlinedTextField(
                            value = if ((me?.autoLockMins ?: 0) == 0) "Off" else "${me?.autoLockMins} min",
                            onValueChange = {},
                            readOnly = true,
                            label = { Text(stringResource(R.string.profile_auto_lock)) },
                            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = lockExpanded) },
                            modifier = Modifier.menuAnchor(MenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
                        )
                        ExposedDropdownMenu(expanded = lockExpanded, onDismissRequest = { lockExpanded = false }) {
                            lockOptions.forEach { mins ->
                                DropdownMenuItem(
                                    text = { Text(if (mins == 0) "Off" else "$mins min") },
                                    onClick = { vm.setAutoLock(mins); lockExpanded = false },
                                )
                            }
                        }
                    }

                    HorizontalDivider(Modifier.padding(vertical = Spacing.sm))

                    var langExpanded by remember { mutableStateOf(false) }
                    ExposedDropdownMenuBox(expanded = langExpanded, onExpandedChange = { langExpanded = it }) {
                        OutlinedTextField(
                            value = stringResource(if (state.lang == "zh") R.string.profile_lang_zh else R.string.profile_lang_en),
                            onValueChange = {},
                            readOnly = true,
                            label = { Text(stringResource(R.string.profile_language)) },
                            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = langExpanded) },
                            modifier = Modifier.menuAnchor(MenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
                        )
                        ExposedDropdownMenu(expanded = langExpanded, onDismissRequest = { langExpanded = false }) {
                            DropdownMenuItem(
                                text = { Text(stringResource(R.string.profile_lang_zh)) },
                                onClick = { vm.setLang("zh") { activity?.recreate() }; langExpanded = false },
                            )
                            DropdownMenuItem(
                                text = { Text(stringResource(R.string.profile_lang_en)) },
                                onClick = { vm.setLang("en") { activity?.recreate() }; langExpanded = false },
                            )
                        }
                    }
                }
            }
        }

        item { SectionHeader(stringResource(R.string.profile_section_more)) }
        item {
            OutlinedCard(Modifier.fillMaxWidth()) {
                Column {
                    ProfileRow(stringResource(R.string.notif_title), Icons.Outlined.Notifications) { nav.navigate(Routes.NOTIFICATIONS) }
                }
            }
        }

        state.error?.let { err ->
            item { Text(err, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium) }
        }

        item {
            Column(Modifier.padding(top = Spacing.xs)) {
                TextButton(onClick = { vm.logout() }, modifier = Modifier.fillMaxWidth()) {
                    Icon(Icons.AutoMirrored.Outlined.Logout, contentDescription = null)
                    Text(stringResource(R.string.profile_logout), modifier = Modifier.padding(start = Spacing.xs))
                }
                TextButton(onClick = { vm.logoutAll() }, modifier = Modifier.fillMaxWidth()) {
                    Text(stringResource(R.string.profile_logout_all))
                }
            }
        }
    }
}

@Composable
private fun ProfileRow(label: String, icon: androidx.compose.ui.graphics.vector.ImageVector, onClick: () -> Unit) {
    androidx.compose.material3.Surface(onClick = onClick, color = androidx.compose.ui.graphics.Color.Transparent) {
        Row(
            Modifier.fillMaxWidth().padding(horizontal = Spacing.sm, vertical = Spacing.sm),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(label, style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurface)
            Icon(icon, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}
