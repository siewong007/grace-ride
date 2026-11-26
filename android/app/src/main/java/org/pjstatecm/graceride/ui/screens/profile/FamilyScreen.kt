package org.pjstatecm.graceride.ui.screens.profile

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.ChevronRight
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.Delete
import androidx.compose.material.icons.outlined.Groups
import androidx.compose.material.icons.outlined.PersonAdd
import androidx.compose.material.icons.outlined.Share
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.FamilyMemberDto
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.copyToClipboard
import org.pjstatecm.graceride.ui.shareText
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FamilyScreen(container: AppContainer, nav: NavHostController) {
    val vm: FamilyViewModel = viewModel(factory = FamilyViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val me = when (val a = authState) {
        is AuthState.SignedIn -> a.me
        is AuthState.Pending -> a.me
        else -> null
    }
    val context = LocalContext.current
    val snackbar = remember { SnackbarHostState() }
    val scope = rememberCoroutineScope()

    var confirmDeleteFamily by remember { mutableStateOf(false) }
    var confirmLeaveFamily by remember { mutableStateOf(false) }
    var pendingRemoveMember by remember { mutableStateOf<FamilyMemberDto?>(null) }

    val msgText = state.messageRes?.let { resId ->
        val arg = state.messageArg
        if (arg != null) stringResource(resId, arg) else stringResource(resId)
    }
    LaunchedEffect(msgText) {
        msgText?.let {
            snackbar.showSnackbar(it)
            vm.clearFeedback()
        }
    }
    LaunchedEffect(state.error) {
        state.error?.let {
            snackbar.showSnackbar(it)
            vm.clearFeedback()
        }
    }

    val linkCopiedMsg = stringResource(R.string.common_copy)

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.family_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.common_back))
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background),
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
        containerColor = MaterialTheme.colorScheme.background,
    ) { padding ->
        if (state.loading) {
            LoadingView(Modifier.fillMaxSize().padding(padding))
            return@Scaffold
        }

        val familyData = state.data
        val family = familyData.family
        val isOwner = family != null && me?.id == family.ownerId

        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(horizontal = Spacing.sm),
            contentPadding = PaddingValues(vertical = Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
        ) {
            // 1. Proxy riders shortcut when member can book rides
            if (me?.can("rides") == true) {
                item {
                    OutlinedCard(
                        modifier = Modifier.fillMaxWidth(),
                        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                    ) {
                        Surface(
                            onClick = { nav.navigate(AppRoute.ProxyRiders) },
                            color = MaterialTheme.colorScheme.surface,
                        ) {
                            Row(
                                Modifier
                                    .fillMaxWidth()
                                    .padding(Spacing.sm),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(Spacing.sm),
                            ) {
                                Icon(Icons.Outlined.Groups, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                                Column(Modifier.weight(1f)) {
                                    Text(stringResource(R.string.family_proxy_shortcut), style = MaterialTheme.typography.titleSmall)
                                    Text(
                                        stringResource(R.string.family_proxy_shortcut_desc),
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                                Icon(
                                    Icons.Outlined.ChevronRight,
                                    contentDescription = null,
                                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    }
                }
            }

            // 2. Incoming family invite
            familyData.incoming?.let { incoming ->
                item {
                    OutlinedCard(
                        modifier = Modifier.fillMaxWidth(),
                        border = BorderStroke(1.dp, MaterialTheme.colorScheme.primary),
                    ) {
                        Column(
                            Modifier.padding(Spacing.sm),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            Text(stringResource(R.string.family_incoming_title), style = MaterialTheme.typography.titleMedium)
                            Text(
                                stringResource(R.string.family_incoming_body, incoming.ownerName, incoming.familyName),
                                style = MaterialTheme.typography.bodyMedium,
                            )
                            Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                Button(
                                    onClick = { vm.acceptIncoming(incoming.id) },
                                    enabled = !state.busy,
                                    colors = graceRidePrimaryButtonColors(),
                                ) { Text(stringResource(R.string.family_accept)) }
                                OutlinedButton(
                                    onClick = { vm.declineIncoming(incoming.id) },
                                    enabled = !state.busy,
                                ) { Text(stringResource(R.string.family_decline)) }
                            }
                        }
                    }
                }
            }

            if (family == null) {
                item {
                    OutlinedCard(
                        modifier = Modifier.fillMaxWidth(),
                        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                    ) {
                        Column(
                            Modifier.padding(Spacing.sm),
                            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                        ) {
                            if (me?.status == "pending") {
                                Text(
                                    stringResource(R.string.family_pending_hint),
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            } else {
                                Text(stringResource(R.string.family_create_title), style = MaterialTheme.typography.titleMedium)
                                OutlinedTextField(
                                    value = state.createName,
                                    onValueChange = vm::setCreateName,
                                    label = { Text(stringResource(R.string.family_name_label)) },
                                    singleLine = true,
                                    modifier = Modifier.fillMaxWidth(),
                                )
                                Button(
                                    onClick = vm::createFamily,
                                    enabled = state.createName.isNotBlank() && !state.busy,
                                    colors = graceRidePrimaryButtonColors(),
                                    modifier = Modifier.fillMaxWidth().height(48.dp),
                                ) {
                                    Text(stringResource(R.string.family_create_btn))
                                }
                            }
                        }
                    }
                }
            } else {
                // 3. Family overview & owner controls
                item {
                    OutlinedCard(
                        modifier = Modifier.fillMaxWidth(),
                        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                    ) {
                        Column(
                            Modifier.padding(Spacing.sm),
                            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                        ) {
                            if (isOwner) {
                                OutlinedTextField(
                                    value = state.renameName,
                                    onValueChange = vm::setRenameName,
                                    label = { Text(stringResource(R.string.family_name_label)) },
                                    singleLine = true,
                                    modifier = Modifier.fillMaxWidth(),
                                )
                                Row(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                    Button(
                                        onClick = vm::renameFamily,
                                        enabled = state.renameName.trim().isNotEmpty() && !state.busy,
                                        colors = graceRidePrimaryButtonColors(),
                                    ) {
                                        Text(stringResource(R.string.family_rename_btn))
                                    }
                                    OutlinedButton(
                                        onClick = { confirmDeleteFamily = true },
                                        enabled = !state.busy,
                                        colors = ButtonDefaults.outlinedButtonColors(
                                            contentColor = MaterialTheme.colorScheme.error,
                                        ),
                                    ) {
                                        Text(stringResource(R.string.family_delete_btn))
                                    }
                                }
                            } else {
                                Text(family.name, style = MaterialTheme.typography.titleLarge)
                                OutlinedButton(
                                    onClick = { confirmLeaveFamily = true },
                                    enabled = !state.busy,
                                    colors = ButtonDefaults.outlinedButtonColors(
                                        contentColor = MaterialTheme.colorScheme.error,
                                    ),
                                ) {
                                    Text(stringResource(R.string.family_leave_btn))
                                }
                            }
                        }
                    }
                }

                // 4. Add / invite person by phone (owner)
                if (isOwner) {
                    item {
                        OutlinedCard(
                            modifier = Modifier.fillMaxWidth(),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                        ) {
                            Column(
                                Modifier.padding(Spacing.sm),
                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                Text(
                                    stringResource(R.string.family_add_phone_label),
                                    style = MaterialTheme.typography.titleSmall,
                                )
                                OutlinedTextField(
                                    value = state.addPhone,
                                    onValueChange = vm::setAddPhone,
                                    label = { Text(stringResource(R.string.invite_phone_label)) },
                                    singleLine = true,
                                    keyboardOptions = KeyboardOptions(
                                        keyboardType = KeyboardType.Phone,
                                        imeAction = ImeAction.Done,
                                    ),
                                    modifier = Modifier.fillMaxWidth(),
                                )
                                Button(
                                    onClick = vm::addPerson,
                                    enabled = state.addPhone.isNotBlank() && !state.busy,
                                    colors = graceRidePrimaryButtonColors(),
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Icon(Icons.Outlined.PersonAdd, contentDescription = null, modifier = Modifier.size(18.dp))
                                    Spacer(Modifier.width(Spacing.xs))
                                    Text(stringResource(R.string.family_add_person_btn))
                                }
                            }
                        }
                    }
                }

                // 5. Members list
                item { SectionHeader(stringResource(R.string.family_members_title), count = familyData.members.size) }
                item {
                    OutlinedCard(
                        modifier = Modifier.fillMaxWidth(),
                        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                    ) {
                        Column {
                            familyData.members.forEachIndexed { idx, member ->
                                if (idx > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                Row(
                                    Modifier
                                        .fillMaxWidth()
                                        .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Column(Modifier.weight(1f)) {
                                        Text(member.name, style = MaterialTheme.typography.bodyLarge)
                                        if (member.status != "active") {
                                            Text(
                                                stringResource(R.string.family_waiting_approval),
                                                style = MaterialTheme.typography.labelSmall,
                                                color = MaterialTheme.colorScheme.primary,
                                            )
                                        }
                                    }
                                    if (isOwner && member.userId != me?.id) {
                                        IconButton(onClick = { pendingRemoveMember = member }) {
                                            Icon(
                                                Icons.Outlined.Delete,
                                                contentDescription = stringResource(R.string.family_remove_member),
                                                tint = MaterialTheme.colorScheme.error,
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 6. Pending requests
                if (familyData.requests.isNotEmpty()) {
                    item { SectionHeader(stringResource(R.string.family_requests_title), count = familyData.requests.size) }
                    item {
                        OutlinedCard(
                            modifier = Modifier.fillMaxWidth(),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                        ) {
                            Column {
                                familyData.requests.forEachIndexed { idx, req ->
                                    if (idx > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                    Row(
                                        Modifier
                                            .fillMaxWidth()
                                            .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        Text(req.name, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                                        if (isOwner) {
                                            TextButton(
                                                onClick = { vm.cancelRequest(req.id) },
                                                enabled = !state.busy,
                                            ) {
                                                Text(stringResource(R.string.family_cancel_request))
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 7. Pending invite links
                if (familyData.invites.isNotEmpty()) {
                    item { SectionHeader(stringResource(R.string.family_invites_title), count = familyData.invites.size) }
                    item {
                        OutlinedCard(
                            modifier = Modifier.fillMaxWidth(),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                        ) {
                            Column {
                                familyData.invites.forEachIndexed { idx, inv ->
                                    if (idx > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                    val fullUrl = container.webUrl(inv.url)
                                    Row(
                                        Modifier
                                            .fillMaxWidth()
                                            .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        Column(Modifier.weight(1f)) {
                                            Text(inv.phone, style = MaterialTheme.typography.bodyMedium)
                                            if (inv.expiresAt.isNotBlank()) {
                                                Text(
                                                    KlTime.display(inv.expiresAt),
                                                    style = MaterialTheme.typography.bodySmall,
                                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                )
                                            }
                                        }
                                        Row {
                                            IconButton(
                                                onClick = {
                                                    if (copyToClipboard(context, "Family invite", fullUrl)) {
                                                        scope.launch { snackbar.showSnackbar(linkCopiedMsg) }
                                                    }
                                                },
                                            ) {
                                                Icon(Icons.Outlined.ContentCopy, contentDescription = stringResource(R.string.family_copy_link))
                                            }
                                            IconButton(onClick = { shareText(context, fullUrl) }) {
                                                Icon(Icons.Outlined.Share, contentDescription = stringResource(R.string.family_share_link))
                                            }
                                            if (isOwner) {
                                                IconButton(onClick = { vm.revokeInvite(inv.id) }) {
                                                    Icon(
                                                        Icons.Outlined.Delete,
                                                        contentDescription = stringResource(R.string.family_cancel_invite),
                                                        tint = MaterialTheme.colorScheme.error,
                                                    )
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
    }

    if (confirmDeleteFamily) {
        AlertDialog(
            onDismissRequest = { confirmDeleteFamily = false },
            title = { Text(stringResource(R.string.family_delete_btn)) },
            text = { Text(stringResource(R.string.family_delete_confirm)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmDeleteFamily = false
                        vm.deleteFamily()
                    },
                ) {
                    Text(stringResource(R.string.family_delete_btn), color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmDeleteFamily = false }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }

    if (confirmLeaveFamily) {
        AlertDialog(
            onDismissRequest = { confirmLeaveFamily = false },
            title = { Text(stringResource(R.string.family_leave_btn)) },
            text = { Text(stringResource(R.string.family_leave_confirm)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmLeaveFamily = false
                        vm.leaveFamily()
                    },
                ) {
                    Text(stringResource(R.string.family_leave_btn), color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmLeaveFamily = false }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }

    pendingRemoveMember?.let { member ->
        AlertDialog(
            onDismissRequest = { pendingRemoveMember = null },
            title = { Text(stringResource(R.string.family_remove_member)) },
            text = { Text(stringResource(R.string.family_remove_member_confirm, member.name)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        pendingRemoveMember = null
                        vm.removeMember(member.userId)
                    },
                ) {
                    Text(stringResource(R.string.family_remove_member), color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { pendingRemoveMember = null }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }
}
