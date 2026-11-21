package org.pjstatecm.graceride.ui.screens.events

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MenuAnchorType
import androidx.compose.material3.OutlinedButton
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
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.theme.Spacing

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EventDetailScreen(container: AppContainer, nav: NavHostController, id: Long) {
    val vm: EventDetailViewModel = viewModel(factory = EventDetailViewModel.factory(container, id))
    val state by vm.state.collectAsStateWithLifecycle()
    val snackbar = remember { SnackbarHostState() }
    var showSignup by remember { mutableStateOf(false) }
    var confirmWithdraw by remember { mutableStateOf(false) }

    LaunchedEffect(state.message) {
        state.message?.let {
            snackbar.showSnackbar(it)
            vm.clearMessage()
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.events_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        when {
            state.loading -> LoadingView(Modifier.padding(padding))
            state.event == null -> ErrorView(state.error ?: "", vm::load, Modifier.padding(padding))
            else -> {
                val event = state.event!!
                // A withdrawn signup stays in the payload with status "cancelled";
                // treat it as no signup so the member can sign up again.
                val signup = event.mySignup?.takeIf { it.status == "going" || it.status == "waitlist" }
                Column(
                    Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState()).padding(Spacing.sm),
                    verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                ) {
                    Text(event.title, style = MaterialTheme.typography.headlineSmall, color = MaterialTheme.colorScheme.onSurface)
                    Text(KlTime.display(event.startsAt), style = MaterialTheme.typography.titleMedium, color = MaterialTheme.colorScheme.primary)

                    OutlinedCard(Modifier.fillMaxWidth()) {
                        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            if (event.location.isNotBlank()) InfoRow(stringResource(R.string.event_location), event.location)
                            if (event.capacity > 0) {
                                val spots = event.spotsLeft
                                InfoRow(
                                    stringResource(R.string.event_going),
                                    event.going.toString() + " / " + event.capacity +
                                        (spots?.let { " · " + stringResource(R.string.event_spots_left, it) } ?: ""),
                                )
                            }
                            if (event.costNote.isNotBlank()) InfoRow(stringResource(R.string.event_cost), event.costNote)
                            event.signupDeadline?.takeIf { it.isNotBlank() }?.let {
                                InfoRow(stringResource(R.string.event_deadline), KlTime.display(it))
                            }
                            signup?.let { mine ->
                                InfoRow(
                                    stringResource(R.string.event_my_status),
                                    stringResource(if (mine.status == "going") R.string.event_going else R.string.event_waitlist),
                                )
                                mine.passCode?.let { InfoRow(stringResource(R.string.event_checkin_code), it) }
                            }
                        }
                    }

                    if (event.description.isNotBlank()) {
                        Text(event.description, style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurface)
                    }

                    if (signup == null && event.status == "published") {
                        Button(onClick = { showSignup = true }, enabled = !state.busy, modifier = Modifier.fillMaxWidth()) {
                            Text(stringResource(R.string.event_signup))
                        }
                    } else if (signup != null) {
                        if (signup.status == "going" && event.status == "published") {
                            Button(
                                onClick = { nav.navigate(Routes.rideNew(event.id)) },
                                enabled = !state.busy,
                                modifier = Modifier.fillMaxWidth(),
                            ) { Text(stringResource(R.string.ride_request)) }
                        }
                        OutlinedButton(onClick = { confirmWithdraw = true }, enabled = !state.busy, modifier = Modifier.fillMaxWidth()) {
                            Text(stringResource(R.string.event_withdraw), color = MaterialTheme.colorScheme.error)
                        }
                    }
                }
            }
        }
    }

    val event = state.event
    if (showSignup && event != null) {
        var needRide by remember { mutableStateOf(false) }
        var offerDrive by remember { mutableStateOf(false) }
        var groupId by remember { mutableStateOf(event.joinAs.firstOrNull()?.id) }
        var groupExpanded by remember { mutableStateOf(false) }

        AlertDialog(
            onDismissRequest = { showSignup = false },
            title = { Text(stringResource(R.string.event_signup)) },
            text = {
                Column {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier
                            .fillMaxWidth()
                            .toggleable(
                                value = needRide,
                                onValueChange = { needRide = it },
                            ),
                    ) {
                        Checkbox(checked = needRide, onCheckedChange = null)
                        Text(stringResource(R.string.event_need_ride), modifier = Modifier.padding(start = Spacing.xs))
                    }
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier
                            .fillMaxWidth()
                            .toggleable(
                                value = offerDrive,
                                onValueChange = { offerDrive = it },
                            ),
                    ) {
                        Checkbox(checked = offerDrive, onCheckedChange = null)
                        Text(stringResource(R.string.event_offer_drive), modifier = Modifier.padding(start = Spacing.xs))
                    }
                    if (event.joinAs.size > 1) {
                        ExposedDropdownMenuBox(expanded = groupExpanded, onExpandedChange = { groupExpanded = it }) {
                            OutlinedTextField(
                                value = event.joinAs.firstOrNull { it.id == groupId }?.name ?: "",
                                onValueChange = {},
                                readOnly = true,
                                label = { Text(stringResource(R.string.event_join_as_group)) },
                                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = groupExpanded) },
                                modifier = Modifier.menuAnchor(MenuAnchorType.PrimaryNotEditable).fillMaxWidth().padding(top = Spacing.xs),
                            )
                            ExposedDropdownMenu(expanded = groupExpanded, onDismissRequest = { groupExpanded = false }) {
                                event.joinAs.forEach { g ->
                                    DropdownMenuItem(text = { Text(g.name) }, onClick = { groupId = g.id; groupExpanded = false })
                                }
                            }
                        }
                    }
                }
            },
            confirmButton = {
                TextButton(onClick = {
                    vm.signup(needRide, offerDrive, if (event.joinAs.size == 1) event.joinAs.first().id else groupId)
                    showSignup = false
                }) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = { TextButton(onClick = { showSignup = false }) { Text(stringResource(R.string.common_cancel)) } },
        )
    }

    if (confirmWithdraw) {
        AlertDialog(
            onDismissRequest = { confirmWithdraw = false },
            title = { Text(stringResource(R.string.event_withdraw)) },
            text = { Text(stringResource(R.string.event_withdraw_confirm)) },
            confirmButton = {
                TextButton(onClick = { vm.withdraw(); confirmWithdraw = false }) { Text(stringResource(R.string.common_confirm)) }
            },
            dismissButton = { TextButton(onClick = { confirmWithdraw = false }) { Text(stringResource(R.string.common_cancel)) } },
        )
    }
}

@Composable
private fun InfoRow(label: String, value: String) {
    Column {
        Text(label, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(value, style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurface)
    }
}
