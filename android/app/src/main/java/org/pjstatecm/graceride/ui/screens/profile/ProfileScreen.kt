package org.pjstatecm.graceride.ui.screens.profile

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
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
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.HelpOutline
import androidx.compose.material.icons.automirrored.outlined.Logout
import androidx.compose.material.icons.outlined.AdminPanelSettings
import androidx.compose.material.icons.outlined.BarChart
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material.icons.outlined.ChevronRight
import androidx.compose.material.icons.outlined.Delete
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Groups
import androidx.compose.material.icons.outlined.History
import androidx.compose.material.icons.outlined.Key
import androidx.compose.material.icons.outlined.LocationOn
import androidx.compose.material.icons.outlined.Notifications
import androidx.compose.material.icons.outlined.Payments
import androidx.compose.material.icons.outlined.ReceiptLong
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MenuAnchorType
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.PrimaryScrollableTabRow
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.components.AvatarStyle
import org.pjstatecm.graceride.ui.components.GrokBotAvatar
import org.pjstatecm.graceride.ui.components.MemberAvatar
import org.pjstatecm.graceride.ui.components.MuseBotAvatar
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.openInWebApp
import org.pjstatecm.graceride.ui.theme.Spacing

private enum class AvatarMode { PHOTO, ANIMATED, INITIALS, ICON }

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProfileScreen(container: AppContainer, nav: NavHostController) {
    val vm: ProfileViewModel = viewModel(factory = ProfileViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val activity = context as? android.app.Activity
    val me = state.me
    val snackbar = remember { SnackbarHostState() }
    val savedMsg = stringResource(R.string.profile_saved)
    var showAvatarEditor by remember { mutableStateOf(false) }

    var name by remember { mutableStateOf(me?.name ?: "") }
    var phone by remember { mutableStateOf(me?.phone ?: "") }
    var telegramChatId by remember { mutableStateOf(me?.telegramChatId ?: "") }

    LaunchedEffect(me) {
        if (me != null) {
            name = me.name
            phone = me.phone
            telegramChatId = me.telegramChatId ?: ""
        }
    }

    LaunchedEffect(state.message) {
        if (state.message != null) {
            snackbar.showSnackbar(savedMsg)
            vm.clearMessage()
        }
    }

    Scaffold(
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        LazyColumn(
            Modifier.fillMaxSize().padding(padding).padding(horizontal = Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
            contentPadding = PaddingValues(vertical = Spacing.sm),
        ) {
            item {
                Row(
                    Modifier.fillMaxWidth().padding(vertical = Spacing.xs),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(Spacing.sm),
                ) {
                    MemberAvatar(
                        name = me?.name ?: "",
                        photoUrl = me?.photoFileId?.let(container::fileUrl),
                        avatarColor = me?.avatarColor,
                        avatarIcon = me?.avatarIcon,
                        size = 64.dp,
                    )
                    Column(Modifier.weight(1f)) {
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
                    OutlinedButton(onClick = { showAvatarEditor = true }) {
                        Text(stringResource(R.string.profile_edit_avatar))
                    }
                }
            }

            item { SectionHeader(stringResource(R.string.profile_personal)) }
            item {
                OutlinedCard(Modifier.fillMaxWidth()) {
                    Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        OutlinedTextField(
                            value = name,
                            onValueChange = { name = it },
                            label = { Text(stringResource(R.string.profile_name)) },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth(),
                        )
                        OutlinedTextField(
                            value = phone,
                            onValueChange = { phone = it },
                            label = { Text(stringResource(R.string.profile_phone)) },
                            placeholder = { Text(stringResource(R.string.profile_phone_placeholder)) },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth(),
                        )
                        OutlinedTextField(
                            value = telegramChatId,
                            onValueChange = { telegramChatId = it },
                            label = { Text(stringResource(R.string.profile_telegram_id)) },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth(),
                        )
                        Button(
                            onClick = { vm.saveProfile(name, phone, telegramChatId) },
                            enabled = !state.busy && name.isNotBlank(),
                            modifier = Modifier.padding(top = Spacing.xxs),
                        ) {
                            Text(stringResource(R.string.profile_save))
                        }
                    }
                }
            }

            item { SectionHeader(stringResource(R.string.profile_section_rides)) }
            item {
                OutlinedCard(Modifier.fillMaxWidth()) {
                    Column {
                        ProfileRow(stringResource(R.string.saved_addresses_title), Icons.Outlined.LocationOn) { nav.navigate(Routes.SAVED_ADDRESSES) }
                        if (me?.can("drive") == true) {
                            HorizontalDivider()
                            ProfileRow(stringResource(R.string.driver_settings_title), Icons.Outlined.DirectionsCar) { nav.navigate(Routes.DRIVER_SETTINGS) }
                        }
                        HorizontalDivider()
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

            if (me != null && me.status != "pending") {
                item { SectionHeader(stringResource(R.string.profile_section_web)) }
                item {
                    OutlinedCard(Modifier.fillMaxWidth()) {
                        Column {
                            var hasAbove = false
                            if (me.can("drive")) {
                                ProfileRow(stringResource(R.string.profile_web_drive), Icons.Outlined.DirectionsCar) {
                                    openInWebApp(context, container.webUrl("drive"))
                                }
                                hasAbove = true
                            }
                            if (me.can("reimbursements")) {
                                if (hasAbove) HorizontalDivider()
                                ProfileRow(stringResource(R.string.profile_reimbursements), Icons.Outlined.ReceiptLong) {
                                    openInWebApp(context, container.webUrl("reimbursements"))
                                }
                                hasAbove = true
                            }
                            if (hasAbove) HorizontalDivider()
                            ProfileRow(stringResource(R.string.profile_family), Icons.Outlined.Groups) {
                                openInWebApp(context, container.webUrl("me/family"))
                            }
                            if (me.can("report")) {
                                HorizontalDivider()
                                ProfileRow(stringResource(R.string.profile_report), Icons.Outlined.BarChart) {
                                    openInWebApp(context, container.webUrl("report"))
                                }
                            }
                            if (me.can("help")) {
                                HorizontalDivider()
                                ProfileRow(stringResource(R.string.profile_help), Icons.AutoMirrored.Outlined.HelpOutline) {
                                    openInWebApp(context, container.webUrl("help"))
                                }
                            }
                            if (me.privileged) {
                                HorizontalDivider()
                                val staffPath = if (me.can("admin_plan") || me.can("admin_rides")) "admin/traffic" else "admin/menu"
                                ProfileRow(stringResource(R.string.profile_web_staff), Icons.Outlined.AdminPanelSettings) {
                                    openInWebApp(context, container.webUrl(staffPath))
                                }
                            }
                        }
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

    if (showAvatarEditor && me != null) {
        AvatarEditorDialog(
            me = me,
            photoUrlFor = container::fileUrl,
            busy = state.busy,
            onDismiss = { showAvatarEditor = false },
            onSavePhoto = { uri, color ->
                if (uri != null) {
                    vm.saveAvatarPhoto(context, uri, color)
                } else if (me.photoFileId != null) {
                    vm.saveAvatarPhotoColor(me.photoFileId, color)
                }
                showAvatarEditor = false
            },
            onSaveStyle = { icon, color ->
                vm.saveAvatarStyle(icon, color)
                showAvatarEditor = false
            },
        )
    }
}

@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class)
@Composable
private fun AvatarEditorDialog(
    me: Me,
    photoUrlFor: (Long) -> String,
    busy: Boolean,
    onDismiss: () -> Unit,
    onSavePhoto: (Uri?, String) -> Unit,
    onSaveStyle: (String, String) -> Unit,
) {
    val initialMode = when {
        me.photoFileId != null -> AvatarMode.PHOTO
        AvatarStyle.isAnimatedAvatar(me.avatarIcon) -> AvatarMode.ANIMATED
        !me.avatarIcon.isNullOrBlank() -> AvatarMode.ICON
        else -> AvatarMode.INITIALS
    }
    var mode by remember { mutableStateOf(initialMode) }
    var color by remember { mutableStateOf(me.avatarColor ?: "purple") }
    var icon by remember { mutableStateOf(me.avatarIcon ?: "car") }
    var pickedUri by remember { mutableStateOf<Uri?>(null) }

    val pickImage = rememberLauncherForActivityResult(ActivityResultContracts.GetContent()) { uri ->
        if (uri != null) {
            pickedUri = uri
            mode = AvatarMode.PHOTO
        }
    }

    val modes = listOf(
        AvatarMode.PHOTO to stringResource(R.string.profile_avatar_photo),
        AvatarMode.ANIMATED to stringResource(R.string.profile_avatar_animated),
        AvatarMode.INITIALS to stringResource(R.string.profile_avatar_initials),
        AvatarMode.ICON to stringResource(R.string.profile_avatar_icon),
    )

    val canSave = when (mode) {
        AvatarMode.PHOTO -> pickedUri != null || me.photoFileId != null
        AvatarMode.ANIMATED -> AvatarStyle.isAnimatedAvatar(icon)
        AvatarMode.ICON -> AvatarStyle.staticIconVector(icon) != null
        AvatarMode.INITIALS -> true
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.profile_edit_avatar)) },
        text = {
            Column(
                Modifier.verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
            ) {
                Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
                    val previewUrl = when {
                        mode == AvatarMode.PHOTO && pickedUri != null -> pickedUri.toString()
                        mode == AvatarMode.PHOTO -> me.photoFileId?.let(photoUrlFor)
                        else -> null
                    }
                    val previewIcon = when (mode) {
                        AvatarMode.ANIMATED, AvatarMode.ICON -> icon
                        else -> null
                    }
                    MemberAvatar(
                        name = me.name,
                        photoUrl = previewUrl,
                        avatarColor = color,
                        avatarIcon = previewIcon,
                        size = 76.dp,
                    )
                }

                PrimaryScrollableTabRow(selectedTabIndex = modes.indexOfFirst { it.first == mode }.coerceAtLeast(0)) {
                    modes.forEach { (m, label) ->
                        Tab(
                            selected = mode == m,
                            onClick = {
                                mode = m
                                if (m == AvatarMode.ANIMATED && !AvatarStyle.isAnimatedAvatar(icon)) {
                                    icon = "grok"
                                } else if (m == AvatarMode.ICON && AvatarStyle.staticIconVector(icon) == null) {
                                    icon = "car"
                                }
                            },
                            text = { Text(label, maxLines = 1) },
                        )
                    }
                }

                when (mode) {
                    AvatarMode.PHOTO -> {
                        OutlinedButton(
                            onClick = { pickImage.launch("image/*") },
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Text(stringResource(R.string.profile_avatar_choose_photo))
                        }
                    }
                    AvatarMode.ANIMATED -> {
                        val (_, fg) = AvatarStyle.colorPair(color)
                        Row(
                            Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            listOf(
                                "grok" to stringResource(R.string.profile_avatar_grok),
                                "muse" to stringResource(R.string.profile_avatar_muse),
                            ).forEach { (key, label) ->
                                val selected = icon == key
                                OutlinedCard(
                                    onClick = { icon = key },
                                    border = if (selected) {
                                        BorderStroke(2.dp, MaterialTheme.colorScheme.primary)
                                    } else {
                                        BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)
                                    },
                                    modifier = Modifier.weight(1f),
                                ) {
                                    Column(
                                        Modifier.fillMaxWidth().padding(Spacing.sm),
                                        horizontalAlignment = Alignment.CenterHorizontally,
                                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                    ) {
                                        if (key == "grok") {
                                            GrokBotAvatar(fg, Modifier.size(44.dp))
                                        } else {
                                            MuseBotAvatar(fg, Modifier.size(44.dp))
                                        }
                                        Text(label, style = MaterialTheme.typography.labelLarge)
                                    }
                                }
                            }
                        }
                    }
                    AvatarMode.ICON -> {
                        FlowRow(
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            AvatarStyle.STATIC_ICONS.forEach { key ->
                                val vec = AvatarStyle.staticIconVector(key) ?: return@forEach
                                FilterChip(
                                    selected = icon == key,
                                    onClick = { icon = key },
                                    label = { Icon(vec, contentDescription = key, modifier = Modifier.size(20.dp)) },
                                )
                            }
                        }
                    }
                    AvatarMode.INITIALS -> Unit
                }

                Text(stringResource(R.string.profile_avatar_color), style = MaterialTheme.typography.labelMedium)
                FlowRow(
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    AvatarStyle.COLORS.forEach { cKey ->
                        val (bg, fg) = AvatarStyle.colorPair(cKey)
                        Surface(
                            color = bg,
                            shape = CircleShape,
                            border = if (color == cKey) BorderStroke(2.dp, fg) else null,
                            modifier = Modifier.size(32.dp).clickable { color = cKey },
                        ) {
                            if (color == cKey) {
                                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                                    Icon(Icons.Outlined.Check, contentDescription = cKey, tint = fg, modifier = Modifier.size(16.dp))
                                }
                            }
                        }
                    }
                }
            }
        },
        confirmButton = {
            Button(
                enabled = !busy && canSave,
                onClick = {
                    when (mode) {
                        AvatarMode.PHOTO -> onSavePhoto(pickedUri, color)
                        AvatarMode.ANIMATED, AvatarMode.ICON -> onSaveStyle(icon, color)
                        AvatarMode.INITIALS -> onSaveStyle("", color)
                    }
                },
            ) {
                Text(stringResource(R.string.profile_avatar_save))
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text(stringResource(R.string.common_cancel))
            }
        },
    )
}

@Composable
private fun ProfileRow(label: String, icon: androidx.compose.ui.graphics.vector.ImageVector, onClick: () -> Unit) {
    Surface(onClick = onClick, color = androidx.compose.ui.graphics.Color.Transparent) {
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
