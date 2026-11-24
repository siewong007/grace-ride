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
import androidx.compose.material.icons.outlined.Shuffle
import androidx.compose.material.icons.outlined.AutoAwesome
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Badge
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
import androidx.compose.material3.Switch
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
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.UpdateNotificationPrefsReq
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.components.AnimatedMemberAvatar
import org.pjstatecm.graceride.ui.components.AvatarStyle
import org.pjstatecm.graceride.ui.components.GrokBotCanvas
import org.pjstatecm.graceride.ui.components.GrokBotControlsPanel
import org.pjstatecm.graceride.ui.components.GrokBotSpec
import org.pjstatecm.graceride.ui.components.MemberAvatar
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.openInWebApp
import org.pjstatecm.graceride.ui.theme.Spacing

private enum class AvatarMode { PHOTO, ANIMATED, STUDIO, INITIALS, ICON }

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun ProfileScreen(container: AppContainer, nav: NavHostController) {
    val vm: ProfileViewModel = viewModel(factory = ProfileViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val activity = context as? android.app.Activity
    val me = state.me
    val unreadCount = (authState as? AuthState.SignedIn)?.me?.unread ?: me?.unread ?: 0
    val snackbar = remember { SnackbarHostState() }
    val savedMsg = stringResource(R.string.profile_saved)
    val prefsSavedMsg = stringResource(R.string.notif_pref_saved)
    var showAvatarEditor by remember { mutableStateOf(false) }

    var name by remember { mutableStateOf(me?.name ?: "") }
    var phone by remember { mutableStateOf(me?.phone ?: "") }
    var telegramChatId by remember { mutableStateOf(me?.telegramChatId ?: "") }
    var pushOptIn by remember { mutableStateOf(me?.pushOptIn ?: true) }
    var whatsappOptIn by remember { mutableStateOf(me?.whatsappOptIn ?: false) }
    var driverRemindMins by remember { mutableStateOf(60) }
    var quietStart by remember { mutableStateOf("") }
    var quietEnd by remember { mutableStateOf("") }
    var hideLockDetails by remember { mutableStateOf(false) }
    var mutedCats by remember { mutableStateOf<List<String>>(emptyList()) }

    LaunchedEffect(me?.status) {
        if (me != null && me.status != "pending") {
            vm.loadNotificationPrefs()
        }
    }

    LaunchedEffect(me) {
        if (me != null) {
            name = me.name
            phone = me.phone
            telegramChatId = me.telegramChatId ?: ""
            pushOptIn = me.pushOptIn
            whatsappOptIn = me.whatsappOptIn
        }
    }

    LaunchedEffect(state.notifPrefs) {
        state.notifPrefs?.let { p ->
            pushOptIn = p.pushOptIn
            whatsappOptIn = p.whatsappOptIn
            driverRemindMins = p.driverRemindMins
            quietStart = p.quietHoursStart
            quietEnd = p.quietHoursEnd
            hideLockDetails = p.hideLockDetails
            mutedCats = p.mutedCategories
            if (p.telegramChatId.isNotBlank()) telegramChatId = p.telegramChatId
        }
    }

    LaunchedEffect(state.message) {
        when (state.message) {
            "prefs_saved" -> {
                snackbar.showSnackbar(prefsSavedMsg)
                vm.clearMessage()
            }
            null -> {}
            else -> {
                snackbar.showSnackbar(savedMsg)
                vm.clearMessage()
            }
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
                            placeholder = { Text("123456789") },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth(),
                        )
                        if (state.notifPrefs?.telegramBlocked == true) {
                            Text(
                                stringResource(R.string.notif_tg_blocked),
                                color = MaterialTheme.colorScheme.error,
                                style = MaterialTheme.typography.bodySmall,
                            )
                        }
                        if (me != null && me.status != "pending") {
                            FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                OutlinedButton(onClick = { vm.connectTelegram { url -> openInWebApp(context, url) } }) {
                                    Text(stringResource(R.string.notif_tg_connect_1tap))
                                }
                                if (telegramChatId.isNotBlank()) {
                                    OutlinedButton(onClick = { vm.testTelegram() }) {
                                        Text(stringResource(R.string.notif_tg_test))
                                    }
                                }
                            }
                            state.telegramStatus?.let { st ->
                                Text(
                                    text = when (st) {
                                        "ok" -> stringResource(R.string.notif_tg_test_ok)
                                        "failed" -> stringResource(R.string.notif_tg_test_fail)
                                        else -> st
                                    },
                                    style = MaterialTheme.typography.bodySmall,
                                    color = if (st == "ok") MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error,
                                )
                            }
                        }
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
                        if (me?.can("rides") == true) {
                            HorizontalDivider()
                            ProfileRow(stringResource(R.string.proxy_riders_title), Icons.Outlined.Groups) { nav.navigate(Routes.PROXY_RIDERS) }
                        }
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
                        ProfileRow(stringResource(R.string.profile_open_grok_studio), Icons.Outlined.AutoAwesome) {
                            val gb = GrokBotSpec.parse(me?.avatarIcon)
                            nav.navigate(Routes.studio(gb?.shape, gb?.color, gb?.face))
                        }
                        HorizontalDivider()
                        ProfileRow(
                            label = stringResource(R.string.notif_title),
                            icon = Icons.Outlined.Notifications,
                            badgeCount = unreadCount,
                        ) { nav.navigate(Routes.NOTIFICATIONS) }
                    }
                }
            }

            if (me != null && me.status != "pending") {
                item { SectionHeader(stringResource(R.string.notif_prefs_title)) }
                item {
                    OutlinedCard(Modifier.fillMaxWidth()) {
                        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            Row(
                                Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Text(stringResource(R.string.notif_pref_push), Modifier.weight(1f))
                                Switch(checked = pushOptIn, onCheckedChange = { pushOptIn = it })
                            }
                            Row(
                                Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Text(stringResource(R.string.notif_pref_whatsapp), Modifier.weight(1f))
                                Switch(checked = whatsappOptIn, onCheckedChange = { whatsappOptIn = it })
                            }
                            Row(
                                Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Text(stringResource(R.string.notif_pref_hide_lock), Modifier.weight(1f))
                                Switch(checked = hideLockDetails, onCheckedChange = { hideLockDetails = it })
                            }
                            Text(
                                stringResource(R.string.notif_pref_driver_remind),
                                style = MaterialTheme.typography.titleSmall,
                                modifier = Modifier.padding(top = Spacing.xxs),
                            )
                            FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                listOf(30, 60, 90, 120).forEach { mins ->
                                    FilterChip(
                                        selected = driverRemindMins == mins,
                                        onClick = { driverRemindMins = mins },
                                        label = { Text(stringResource(R.string.notif_pref_driver_mins, mins)) },
                                    )
                                }
                            }
                            Text(
                                stringResource(R.string.notif_pref_quiet_hours),
                                style = MaterialTheme.typography.titleSmall,
                                modifier = Modifier.padding(top = Spacing.xxs),
                            )
                            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                OutlinedTextField(
                                    value = quietStart,
                                    onValueChange = { quietStart = it },
                                    label = { Text(stringResource(R.string.notif_pref_quiet_start)) },
                                    singleLine = true,
                                    modifier = Modifier.weight(1f),
                                )
                                OutlinedTextField(
                                    value = quietEnd,
                                    onValueChange = { quietEnd = it },
                                    label = { Text(stringResource(R.string.notif_pref_quiet_end)) },
                                    singleLine = true,
                                    modifier = Modifier.weight(1f),
                                )
                            }
                            Text(
                                stringResource(R.string.notif_pref_muted_cats),
                                style = MaterialTheme.typography.titleSmall,
                                modifier = Modifier.padding(top = Spacing.xxs),
                            )
                            FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                listOf(
                                    "ride" to R.string.notif_cat_ride,
                                    "event" to R.string.notif_cat_event,
                                    "cost" to R.string.notif_cat_cost,
                                    "help" to R.string.notif_cat_help,
                                    "family" to R.string.notif_cat_family,
                                    "admin" to R.string.notif_cat_admin,
                                    "announcement" to R.string.notif_cat_announcement,
                                ).forEach { (cat, labelRes) ->
                                    val muted = cat in mutedCats
                                    FilterChip(
                                        selected = muted,
                                        onClick = {
                                            mutedCats = if (muted) mutedCats - cat else mutedCats + cat
                                        },
                                        label = { Text(stringResource(labelRes)) },
                                    )
                                }
                            }
                            Button(
                                onClick = {
                                    vm.saveNotificationPrefs(
                                        UpdateNotificationPrefsReq(
                                            push_opt_in = pushOptIn,
                                            whatsapp_opt_in = whatsappOptIn,
                                            driver_remind_mins = driverRemindMins,
                                            muted_categories = mutedCats,
                                            quiet_hours_start = quietStart.trim(),
                                            quiet_hours_end = quietEnd.trim(),
                                            hide_lock_details = hideLockDetails,
                                        )
                                    )
                                },
                                enabled = !state.busy,
                                modifier = Modifier.padding(top = Spacing.xxs),
                            ) {
                                Text(stringResource(R.string.notif_pref_save))
                            }
                        }
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
                item {
                    val displayErr = if (err == "invalid_telegram_chat_id") {
                        stringResource(R.string.notif_tg_invalid_id)
                    } else err
                    Text(displayErr, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium)
                }
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
            onOpenFullStudio = { spec ->
                showAvatarEditor = false
                nav.navigate(Routes.studio(spec.shape, spec.color, spec.face))
            },
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
    onOpenFullStudio: (GrokBotSpec) -> Unit,
    onSavePhoto: (Uri?, String) -> Unit,
    onSaveStyle: (String, String) -> Unit,
) {
    val initParsedColor = AvatarStyle.parseAvatarColor(me.avatarColor)
    val initGrokBot = GrokBotSpec.parse(me.avatarIcon)
    val initParsedIcon = AvatarStyle.parseAvatarIcon(if (initGrokBot != null) null else me.avatarIcon)
    val hasCustom = me.photoFileId != null || !me.avatarIcon.isNullOrBlank() || !me.avatarColor.isNullOrBlank()
    val initialIconBase = when {
        initParsedIcon.icon == "initials" || initParsedIcon.icon.isEmpty() ->
            if (!hasCustom) AvatarStyle.DEFAULT_ANIMATED_ICON else "car"
        else -> initParsedIcon.icon
    }
    val initialMode = when {
        me.photoFileId != null -> AvatarMode.PHOTO
        GrokBotSpec.isGrokBotIcon(me.avatarIcon) -> AvatarMode.STUDIO
        AvatarStyle.isAnimatedAvatar(initParsedIcon.icon) -> AvatarMode.ANIMATED
        AvatarStyle.staticIconVector(initParsedIcon.icon) != null -> AvatarMode.ICON
        !me.avatarColor.isNullOrBlank() || initParsedIcon.fgColor != null -> AvatarMode.INITIALS
        else -> AvatarMode.ANIMATED
    }
    var mode by remember { mutableStateOf(initialMode) }
    var studioSpec by remember { mutableStateOf(initGrokBot ?: GrokBotSpec.DEFAULT) }
    var studioMove by remember { mutableStateOf("idle") }
    var icon by remember { mutableStateOf(initialIconBase) }
    var avatarShape by remember {
        mutableStateOf(initParsedIcon.shape ?: AvatarStyle.defaultAvatarShape(initialIconBase))
    }
    var fgColor by remember { mutableStateOf(initParsedIcon.fgColor) }
    var bgColor by remember {
        mutableStateOf(
            initParsedColor.bgColor.ifBlank {
                if (!hasCustom) AvatarStyle.defaultColorFor(AvatarStyle.DEFAULT_ANIMATED_ICON) else "purple"
            },
        )
    }
    var bgShape by remember { mutableStateOf(initParsedColor.bgShape) }
    var customColorPicked by remember { mutableStateOf(initParsedColor.bgColor.isNotEmpty()) }
    var pickedUri by remember { mutableStateOf<Uri?>(null) }

    val encodedColor = AvatarStyle.encodeAvatarColor(bgColor, bgShape)
    val encodedIcon = when (mode) {
        AvatarMode.STUDIO -> studioSpec.encode()
        AvatarMode.ANIMATED, AvatarMode.ICON -> AvatarStyle.encodeAvatarIcon(icon, avatarShape, fgColor)
        AvatarMode.INITIALS -> AvatarStyle.encodeAvatarIcon("initials", null, fgColor)
        AvatarMode.PHOTO -> ""
    }

    val pickImage = rememberLauncherForActivityResult(ActivityResultContracts.GetContent()) { uri ->
        if (uri != null) {
            pickedUri = uri
            mode = AvatarMode.PHOTO
        }
    }

    val modes = listOf(
        AvatarMode.ANIMATED to stringResource(R.string.profile_avatar_animated),
        AvatarMode.STUDIO to stringResource(R.string.profile_avatar_studio),
        AvatarMode.PHOTO to stringResource(R.string.profile_avatar_photo),
        AvatarMode.INITIALS to stringResource(R.string.profile_avatar_initials),
        AvatarMode.ICON to stringResource(R.string.profile_avatar_icon),
    )

    val canSave = when (mode) {
        AvatarMode.PHOTO -> pickedUri != null || me.photoFileId != null
        AvatarMode.STUDIO -> true
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
                    if (mode == AvatarMode.STUDIO) {
                        Column(
                            horizontalAlignment = Alignment.CenterHorizontally,
                            verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                        ) {
                            GrokBotCanvas(
                                spec = studioSpec,
                                move = studioMove,
                                variant = "stage",
                                modifier = Modifier.size(116.dp),
                            )
                            TextButton(onClick = { studioSpec = GrokBotSpec.random(studioSpec) }) {
                                Icon(Icons.Outlined.Shuffle, contentDescription = null, modifier = Modifier.size(15.dp))
                                Text(
                                    stringResource(R.string.studio_surprise_me),
                                    modifier = Modifier.padding(start = 4.dp),
                                )
                            }
                        }
                    } else {
                        val previewUrl = when {
                            mode == AvatarMode.PHOTO && pickedUri != null -> pickedUri.toString()
                            mode == AvatarMode.PHOTO -> me.photoFileId?.let(photoUrlFor)
                            else -> null
                        }
                        val previewIcon = when (mode) {
                            AvatarMode.ANIMATED, AvatarMode.ICON, AvatarMode.INITIALS -> encodedIcon
                            else -> null
                        }
                        MemberAvatar(
                            name = me.name,
                            photoUrl = previewUrl,
                            avatarColor = encodedColor,
                            avatarIcon = previewIcon,
                            size = 76.dp,
                        )
                    }
                }

                PrimaryScrollableTabRow(selectedTabIndex = modes.indexOfFirst { it.first == mode }.coerceAtLeast(0)) {
                    modes.forEach { (m, label) ->
                        Tab(
                            selected = mode == m,
                            onClick = {
                                mode = m
                                if (m == AvatarMode.ANIMATED && !AvatarStyle.isAnimatedAvatar(icon)) {
                                    icon = AvatarStyle.DEFAULT_ANIMATED_ICON
                                    avatarShape = AvatarStyle.defaultAvatarShape(icon)
                                    if (!customColorPicked) {
                                        bgColor = AvatarStyle.defaultColorFor(AvatarStyle.DEFAULT_ANIMATED_ICON)
                                    }
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
                        val animatedOptions = listOf(
                            "grace_car" to R.string.profile_avatar_grace_car,
                            "peace_dove" to R.string.profile_avatar_peace_dove,
                            "sunny_cloud" to R.string.profile_avatar_sunny_cloud,
                            "happy_bear" to R.string.profile_avatar_happy_bear,
                            "fellowship_cat" to R.string.profile_avatar_fellowship_cat,
                            "joy_puppy" to R.string.profile_avatar_joy_puppy,
                            "star_lantern" to R.string.profile_avatar_star_lantern,
                            "blossom_sprout" to R.string.profile_avatar_blossom_sprout,
                            "grok" to R.string.profile_avatar_grok,
                            "muse" to R.string.profile_avatar_muse,
                        )
                        Column(
                            Modifier.fillMaxWidth(),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            animatedOptions.chunked(2).forEach { pair ->
                                Row(
                                    Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                ) {
                                    pair.forEach { (key, labelRes) ->
                                        val selected = icon == key
                                        val activeBg = if (selected) bgColor else AvatarStyle.defaultColorFor(key)
                                        val activeFg = if (selected) fgColor else null
                                        val cardShape = if (selected && AvatarStyle.isShapeableAvatar(key)) {
                                            avatarShape
                                        } else {
                                            AvatarStyle.defaultAvatarShape(key)
                                        }
                                        val (cardBg, cardFg) = AvatarStyle.colorPair(activeBg)
                                        OutlinedCard(
                                            onClick = {
                                                icon = key
                                                avatarShape = AvatarStyle.defaultAvatarShape(key)
                                                if (!customColorPicked) {
                                                    bgColor = AvatarStyle.defaultColorFor(key)
                                                }
                                            },
                                            border = if (selected) {
                                                BorderStroke(2.dp, MaterialTheme.colorScheme.primary)
                                            } else {
                                                BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)
                                            },
                                            modifier = Modifier.weight(1f),
                                        ) {
                                            Column(
                                                Modifier.fillMaxWidth().padding(Spacing.xs),
                                                horizontalAlignment = Alignment.CenterHorizontally,
                                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                            ) {
                                                Surface(
                                                    color = cardBg,
                                                    shape = CircleShape,
                                                    border = if (activeBg == "white") {
                                                        BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)
                                                    } else {
                                                        null
                                                    },
                                                    modifier = Modifier.size(48.dp),
                                                ) {
                                                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                                                        AnimatedMemberAvatar(
                                                            icon = key,
                                                            tint = cardFg,
                                                            modifier = Modifier.size(40.dp),
                                                            avatarColor = activeFg ?: activeBg,
                                                            avatarShape = cardShape,
                                                        )
                                                    }
                                                }
                                                Text(
                                                    stringResource(labelRes),
                                                    style = MaterialTheme.typography.labelMedium,
                                                    textAlign = TextAlign.Center,
                                                    maxLines = 1,
                                                )
                                            }
                                        }
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
                                FilterChip(
                                    selected = icon == key,
                                    onClick = { icon = key },
                                    label = {
                                        AnimatedMemberAvatar(
                                            icon = key,
                                            tint = MaterialTheme.colorScheme.onSurface,
                                            modifier = Modifier.size(26.dp),
                                            avatarColor = fgColor ?: bgColor,
                                        )
                                    },
                                )
                            }
                        }
                    }
                    AvatarMode.STUDIO -> {
                        Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            GrokBotControlsPanel(
                                spec = studioSpec,
                                onSpecChange = { studioSpec = it },
                                activeMove = studioMove,
                                onMoveChange = { studioMove = it },
                                tilePreviewSize = 36.dp,
                                cols = 4,
                            )
                            OutlinedButton(
                                onClick = { onOpenFullStudio(studioSpec) },
                                modifier = Modifier.fillMaxWidth(),
                            ) {
                                Text(stringResource(R.string.studio_open_full))
                            }
                        }
                    }
                    AvatarMode.INITIALS -> Unit
                }

                if (mode == AvatarMode.ANIMATED && AvatarStyle.isShapeableAvatar(icon)) {
                    Text(stringResource(R.string.profile_avatar_shape), style = MaterialTheme.typography.labelMedium)
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        AvatarStyle.AVATAR_SHAPES.forEach { sKey ->
                            FilterChip(
                                selected = avatarShape == sKey,
                                onClick = { avatarShape = sKey },
                                label = { Text(stringResource(avatarShapeLabelRes(sKey)), maxLines = 1) },
                            )
                        }
                    }
                }

                if (mode != AvatarMode.PHOTO && mode != AvatarMode.STUDIO) {
                    Text(stringResource(R.string.profile_avatar_fg_color), style = MaterialTheme.typography.labelMedium)
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        FilterChip(
                            selected = fgColor == null,
                            onClick = { fgColor = null },
                            label = { Text(stringResource(R.string.profile_avatar_color_auto), maxLines = 1) },
                        )
                        AvatarStyle.COLORS.forEach { cKey ->
                            val swatchBg = AvatarStyle.grokSkinColor(cKey)
                            val swatchFg = AvatarStyle.grokEyeColor(cKey)
                            Surface(
                                color = swatchBg,
                                shape = CircleShape,
                                border = when {
                                    fgColor == cKey -> BorderStroke(2.dp, MaterialTheme.colorScheme.primary)
                                    cKey == "white" -> BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)
                                    else -> null
                                },
                                modifier = Modifier.size(32.dp).clickable { fgColor = cKey },
                            ) {
                                if (fgColor == cKey) {
                                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                                        Icon(Icons.Outlined.Check, contentDescription = cKey, tint = swatchFg, modifier = Modifier.size(16.dp))
                                    }
                                }
                            }
                        }
                    }
                }

                if (mode != AvatarMode.STUDIO) {
                    Text(stringResource(R.string.profile_avatar_bg_shape), style = MaterialTheme.typography.labelMedium)
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        AvatarStyle.AVATAR_BG_SHAPES.forEach { bsKey ->
                            val selected = bgShape == bsKey
                            FilterChip(
                                selected = selected,
                                onClick = { bgShape = bsKey },
                                leadingIcon = {
                                    Surface(
                                        color = androidx.compose.ui.graphics.Color.Transparent,
                                        shape = AvatarStyle.bgComposeShape(bsKey),
                                        border = BorderStroke(
                                            1.5.dp,
                                            if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                                        ),
                                        modifier = Modifier.size(14.dp),
                                    ) {}
                                },
                                label = { Text(stringResource(avatarBgShapeLabelRes(bsKey)), maxLines = 1) },
                            )
                        }
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
                                border = when {
                                    bgColor == cKey -> BorderStroke(2.dp, fg)
                                    cKey == "white" -> BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)
                                    else -> null
                                },
                                modifier = Modifier.size(32.dp).clickable {
                                    bgColor = cKey
                                    customColorPicked = true
                                },
                            ) {
                                if (bgColor == cKey) {
                                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                                        Icon(Icons.Outlined.Check, contentDescription = cKey, tint = fg, modifier = Modifier.size(16.dp))
                                    }
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
                        AvatarMode.PHOTO -> onSavePhoto(pickedUri, encodedColor)
                        AvatarMode.STUDIO -> onSaveStyle(studioSpec.encode(), "")
                        AvatarMode.ANIMATED, AvatarMode.ICON, AvatarMode.INITIALS -> onSaveStyle(encodedIcon, encodedColor)
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

private fun avatarShapeLabelRes(key: String): Int = when (key) {
    "circle" -> R.string.profile_avatar_shape_circle
    "pebble" -> R.string.profile_avatar_shape_pebble
    "squircle" -> R.string.profile_avatar_shape_squircle
    "capsule" -> R.string.profile_avatar_shape_capsule
    "triangle" -> R.string.profile_avatar_shape_triangle
    "cloud" -> R.string.profile_avatar_shape_cloud
    "droplet" -> R.string.profile_avatar_shape_droplet
    else -> R.string.profile_avatar_shape_hexagon
}

private fun avatarBgShapeLabelRes(key: String): Int = when (key) {
    "squircle" -> R.string.profile_avatar_bg_squircle
    "rounded" -> R.string.profile_avatar_bg_rounded
    "hexagon" -> R.string.profile_avatar_bg_hexagon
    "cloud" -> R.string.profile_avatar_bg_cloud
    "pebble" -> R.string.profile_avatar_bg_pebble
    else -> R.string.profile_avatar_bg_circle
}

@Composable
private fun ProfileRow(
    label: String,
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    badgeCount: Int = 0,
    onClick: () -> Unit,
) {
    Surface(onClick = onClick, color = androidx.compose.ui.graphics.Color.Transparent) {
        Row(
            Modifier.fillMaxWidth().padding(horizontal = Spacing.sm, vertical = Spacing.sm),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(label, style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurface)
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                if (badgeCount > 0) {
                    Badge {
                        Text(if (badgeCount > 99) "99+" else badgeCount.toString())
                    }
                }
                Icon(icon, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
    }
}
