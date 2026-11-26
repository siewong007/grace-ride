package org.pjstatecm.graceride.ui.screens.profile

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.Delete
import androidx.compose.material.icons.outlined.Fingerprint
import androidx.compose.material.icons.outlined.Key
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FilterChip
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
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
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
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.applyAppLocale
import org.pjstatecm.graceride.data.api.BiometricInfo
import org.pjstatecm.graceride.data.api.PasskeyInfo
import org.pjstatecm.graceride.data.api.UpdateNotificationPrefsReq
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.platform.BiometricAvailability
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.ThemeMode
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun SettingsSubScreenScaffold(
    title: String,
    nav: NavHostController,
    snackbar: SnackbarHostState,
    content: @Composable (PaddingValues) -> Unit,
) {
    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.background,
                    titleContentColor = MaterialTheme.colorScheme.onBackground,
                ),
                title = { Text(title, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(
                            Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = stringResource(R.string.a11y_back),
                        )
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
        content = content,
    )
}

@Composable
private fun SettingsSurfaceCard(content: @Composable ColumnScope.() -> Unit) {
    OutlinedCard(
        modifier = Modifier.fillMaxWidth(),
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
    ) {
        Column(
            modifier = Modifier.padding(Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
            content = content,
        )
    }
}

@Composable
fun PersonalDetailsScreen(container: AppContainer, nav: NavHostController) {
    val vm: ProfileViewModel = viewModel(factory = ProfileViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val view = LocalView.current
    val snackbar = remember { SnackbarHostState() }
    val savedMsg = stringResource(R.string.profile_saved)
    val me = when (val a = authState) {
        is AuthState.SignedIn -> a.me
        is AuthState.Pending -> a.me
        is AuthState.Locked -> a.me
        else -> null
    } ?: state.me

    var name by remember { mutableStateOf(me?.name ?: "") }
    var phone by remember { mutableStateOf(me?.phone ?: "") }

    LaunchedEffect(me?.id, me?.name, me?.phone) {
        if (me != null) {
            name = me.name
            phone = me.phone
        }
    }

    LaunchedEffect(state.message) {
        if (state.message == "saved") {
            snackbar.showSnackbar(savedMsg)
            vm.clearMessage()
        }
    }
    LaunchedEffect(state.error) {
        state.error?.let {
            snackbar.showSnackbar(it)
            vm.clearError()
        }
    }

    SettingsSubScreenScaffold(
        title = stringResource(R.string.profile_personal),
        nav = nav,
        snackbar = snackbar,
    ) { padding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(padding).padding(horizontal = Spacing.sm),
            contentPadding = PaddingValues(vertical = Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
        ) {
            item {
                SettingsSurfaceCard {
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
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Phone),
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            }
            item {
                Button(
                    onClick = {
                        UiHaptics.confirm(view)
                        vm.saveProfile(name, phone)
                    },
                    enabled = !state.busy && name.isNotBlank(),
                    colors = graceRidePrimaryButtonColors(),
                    modifier = Modifier.fillMaxWidth().height(50.dp),
                ) {
                    if (state.busy) {
                        InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                    } else {
                        Text(stringResource(R.string.profile_save))
                    }
                }
            }
        }
    }
}

@Composable
fun AppearanceSettingsScreen(container: AppContainer, nav: NavHostController) {
    val vm: ProfileViewModel = viewModel(factory = ProfileViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val themeModeKey by container.themeMode.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val view = LocalView.current
    val snackbar = remember { SnackbarHostState() }

    SettingsSubScreenScaffold(
        title = stringResource(R.string.profile_appearance_lang),
        nav = nav,
        snackbar = snackbar,
    ) { padding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(padding).padding(horizontal = Spacing.sm),
            contentPadding = PaddingValues(vertical = Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
        ) {
            item {
                SettingsSurfaceCard {
                    Column(verticalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
                        Text(stringResource(R.string.profile_theme), style = MaterialTheme.typography.titleSmall)
                        val activeTheme = ThemeMode.fromKey(themeModeKey)
                        val themeOptions = listOf(
                            ThemeMode.SYSTEM to stringResource(R.string.profile_theme_system),
                            ThemeMode.LIGHT to stringResource(R.string.profile_theme_light),
                            ThemeMode.DARK to stringResource(R.string.profile_theme_dark),
                        )
                        SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
                            themeOptions.forEachIndexed { idx, (modeOpt, label) ->
                                SegmentedButton(
                                    selected = activeTheme == modeOpt,
                                    onClick = {
                                        UiHaptics.tick(view)
                                        container.setThemeMode(modeOpt.key)
                                    },
                                    shape = SegmentedButtonDefaults.itemShape(index = idx, count = themeOptions.size),
                                    label = { Text(label, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                                )
                            }
                        }
                    }

                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)

                    Column(verticalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
                        Text(stringResource(R.string.profile_language), style = MaterialTheme.typography.titleSmall)
                        val langs = listOf(
                            "en" to stringResource(R.string.profile_lang_en),
                            "zh" to stringResource(R.string.profile_lang_zh),
                        )
                        SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
                            langs.forEachIndexed { idx, (code, label) ->
                                SegmentedButton(
                                    selected = state.lang == code,
                                    onClick = {
                                        if (state.lang != code) {
                                            UiHaptics.tick(view)
                                            vm.setLang(code) { applyAppLocale(context, code) }
                                        }
                                    },
                                    shape = SegmentedButtonDefaults.itemShape(index = idx, count = langs.size),
                                    label = { Text(label, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun NotificationSettingsScreen(container: AppContainer, nav: NavHostController) {
    val vm: ProfileViewModel = viewModel(factory = ProfileViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val view = LocalView.current
    val snackbar = remember { SnackbarHostState() }
    val prefsSavedMsg = stringResource(R.string.notif_pref_saved)
    val me = when (val a = authState) {
        is AuthState.SignedIn -> a.me
        is AuthState.Pending -> a.me
        is AuthState.Locked -> a.me
        else -> null
    } ?: state.me

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

    LaunchedEffect(state.notifPrefs) {
        state.notifPrefs?.let { p ->
            pushOptIn = p.pushOptIn
            whatsappOptIn = p.whatsappOptIn
            driverRemindMins = p.driverRemindMins
            quietStart = p.quietHoursStart
            quietEnd = p.quietHoursEnd
            hideLockDetails = p.hideLockDetails
            mutedCats = p.mutedCategories
        }
    }

    LaunchedEffect(state.message) {
        if (state.message == "prefs_saved") {
            snackbar.showSnackbar(prefsSavedMsg)
            vm.clearMessage()
        }
    }
    LaunchedEffect(state.error) {
        state.error?.let {
            snackbar.showSnackbar(it)
            vm.clearError()
        }
    }

    SettingsSubScreenScaffold(
        title = stringResource(R.string.notif_prefs_title),
        nav = nav,
        snackbar = snackbar,
    ) { padding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(padding).padding(horizontal = Spacing.sm),
            contentPadding = PaddingValues(vertical = Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
        ) {
            item {
                SettingsSurfaceCard {
                    Row(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(stringResource(R.string.notif_pref_push), Modifier.weight(1f))
                        Switch(checked = pushOptIn, onCheckedChange = { pushOptIn = it })
                    }
                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                    Row(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(stringResource(R.string.notif_pref_whatsapp), Modifier.weight(1f))
                        Switch(checked = whatsappOptIn, onCheckedChange = { whatsappOptIn = it })
                    }
                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                    Row(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(stringResource(R.string.notif_pref_hide_lock), Modifier.weight(1f))
                        Switch(checked = hideLockDetails, onCheckedChange = { hideLockDetails = it })
                    }
                }
            }

            if (me?.can("drive") == true) {
                item { SectionHeader(stringResource(R.string.notif_pref_driver_remind)) }
                item {
                    SettingsSurfaceCard {
                        FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            listOf(30, 60, 90, 120).forEach { mins ->
                                FilterChip(
                                    selected = driverRemindMins == mins,
                                    onClick = { driverRemindMins = mins },
                                    label = { Text(stringResource(R.string.notif_pref_driver_mins, mins)) },
                                )
                            }
                        }
                    }
                }
            }

            item { SectionHeader(stringResource(R.string.notif_pref_quiet_hours)) }
            item {
                SettingsSurfaceCard {
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
                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                    Text(
                        stringResource(R.string.notif_pref_muted_cats),
                        style = MaterialTheme.typography.titleSmall,
                    )
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        buildList {
                            add("ride" to R.string.notif_cat_ride)
                            add("event" to R.string.notif_cat_event)
                            add("cost" to R.string.notif_cat_cost)
                            add("help" to R.string.notif_cat_help)
                            add("family" to R.string.notif_cat_family)
                            if (me?.hasStaffAccess() == true) {
                                add("admin" to R.string.notif_cat_admin)
                            }
                            add("announcement" to R.string.notif_cat_announcement)
                        }.forEach { (cat, labelRes) ->
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
                }
            }

            item {
                Button(
                    onClick = {
                        UiHaptics.confirm(view)
                        vm.saveNotificationPrefs(
                            UpdateNotificationPrefsReq(
                                push_opt_in = pushOptIn,
                                whatsapp_opt_in = whatsappOptIn,
                                driver_remind_mins = driverRemindMins,
                                muted_categories = mutedCats,
                                quiet_hours_start = quietStart.trim(),
                                quiet_hours_end = quietEnd.trim(),
                                hide_lock_details = hideLockDetails,
                            ),
                        )
                    },
                    enabled = !state.busy,
                    colors = graceRidePrimaryButtonColors(),
                    modifier = Modifier.fillMaxWidth().height(50.dp),
                ) {
                    if (state.busy) {
                        InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                    } else {
                        Text(stringResource(R.string.notif_pref_save))
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SecuritySettingsScreen(container: AppContainer, nav: NavHostController) {
    val vm: ProfileViewModel = viewModel(factory = ProfileViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val biometricEnrollment by vm.biometricEnrollment.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val view = LocalView.current
    val snackbar = remember { SnackbarHostState() }
    val biometricAvailability = remember(context) { container.biometric.availability(context) }

    val bioEnabledMsg = stringResource(R.string.profile_biometric_enabled_msg)
    val bioDisabledMsg = stringResource(R.string.profile_biometric_disabled_msg)
    var pendingDeletePasskey by remember { mutableStateOf<PasskeyInfo?>(null) }
    var pendingDeleteBiometric by remember { mutableStateOf<BiometricInfo?>(null) }
    var confirmLogoutAll by remember { mutableStateOf(false) }

    val me = when (val a = authState) {
        is AuthState.SignedIn -> a.me
        is AuthState.Pending -> a.me
        is AuthState.Locked -> a.me
        else -> null
    } ?: state.me

    LaunchedEffect(me?.id) {
        if (me != null) {
            vm.loadBiometrics()
            vm.loadPasskeys()
        }
    }

    LaunchedEffect(state.message) {
        when (state.message) {
            "biometric_enabled" -> {
                snackbar.showSnackbar(bioEnabledMsg)
                vm.clearMessage()
            }
            "biometric_disabled" -> {
                snackbar.showSnackbar(bioDisabledMsg)
                vm.clearMessage()
            }
            else -> {}
        }
    }
    LaunchedEffect(state.error) {
        state.error?.let {
            snackbar.showSnackbar(it)
            vm.clearError()
        }
    }

    val bioEnabledHere = me != null && biometricEnrollment?.userId == me.id

    SettingsSubScreenScaffold(
        title = stringResource(R.string.profile_security),
        nav = nav,
        snackbar = snackbar,
    ) { padding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(padding).padding(horizontal = Spacing.sm),
            contentPadding = PaddingValues(vertical = Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
        ) {
            item {
                SettingsSurfaceCard {
                    val bioSubtitle = when {
                        bioEnabledHere -> stringResource(R.string.profile_biometric_enabled_desc)
                        biometricAvailability == BiometricAvailability.AVAILABLE -> stringResource(R.string.profile_biometric_available_desc)
                        biometricAvailability == BiometricAvailability.NONE_ENROLLED -> stringResource(R.string.profile_biometric_none_enrolled)
                        else -> stringResource(R.string.profile_biometric_unavailable)
                    }
                    Row(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(Modifier.weight(1f).padding(end = Spacing.sm)) {
                            Text(stringResource(R.string.profile_biometric_title), style = MaterialTheme.typography.titleSmall)
                            Text(
                                bioSubtitle,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        Switch(
                            checked = bioEnabledHere,
                            enabled = !state.busy && (bioEnabledHere || biometricAvailability == BiometricAvailability.AVAILABLE),
                            onCheckedChange = { wantEnable ->
                                UiHaptics.tick(view)
                                if (wantEnable) vm.enableBiometric(context) else vm.disableCurrentBiometric(context)
                            },
                        )
                    }
                    if (state.biometrics.isNotEmpty()) {
                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                        Text(
                            stringResource(R.string.profile_biometric_devices),
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        state.biometrics.forEach { bio ->
                            val baseLabel = bio.label.ifBlank { stringResource(R.string.profile_biometric_default, bio.id) }
                            val label = if (bio.credential_id == biometricEnrollment?.credentialId) {
                                stringResource(R.string.profile_biometric_this_device, baseLabel)
                            } else {
                                baseLabel
                            }
                            Row(
                                Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Row(Modifier.weight(1f), verticalAlignment = Alignment.CenterVertically) {
                                    Icon(Icons.Outlined.Fingerprint, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
                                    Text(label, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.padding(start = Spacing.xs))
                                }
                                IconButton(onClick = { pendingDeleteBiometric = bio }) {
                                    Icon(Icons.Outlined.Delete, contentDescription = stringResource(R.string.common_delete), tint = MaterialTheme.colorScheme.error)
                                }
                            }
                        }
                    }
                }
            }

            item { SectionHeader(stringResource(R.string.profile_passkeys)) }
            item {
                SettingsSurfaceCard {
                    state.passkeys.forEach { key ->
                        Row(
                            Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Icon(Icons.Outlined.Key, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
                                Text(
                                    key.label.ifBlank { stringResource(R.string.profile_passkey_default, key.id) },
                                    style = MaterialTheme.typography.bodyMedium,
                                    modifier = Modifier.padding(start = Spacing.xs),
                                )
                            }
                            IconButton(onClick = { pendingDeletePasskey = key }) {
                                Icon(Icons.Outlined.Delete, contentDescription = stringResource(R.string.common_delete), tint = MaterialTheme.colorScheme.error)
                            }
                        }
                    }
                    OutlinedButton(
                        onClick = { vm.addPasskey(context) },
                        enabled = !state.busy,
                        modifier = Modifier.fillMaxWidth(),
                    ) { Text(stringResource(R.string.profile_add_passkey)) }
                }
            }

            item {
                SettingsSurfaceCard {
                    var lockExpanded by remember { mutableStateOf(false) }
                    val lockOptions = listOf(0, 5, 15, 30, 60, -1)
                    val autoLockLabel: @Composable (Int) -> String = { mins ->
                        when {
                            mins == 0 -> stringResource(R.string.profile_auto_lock_default)
                            mins < 0 -> stringResource(R.string.profile_auto_lock_off)
                            else -> stringResource(R.string.profile_auto_lock_mins, mins)
                        }
                    }
                    ExposedDropdownMenuBox(expanded = lockExpanded, onExpandedChange = { lockExpanded = it }) {
                        OutlinedTextField(
                            value = autoLockLabel(me?.autoLockMins ?: 0),
                            onValueChange = {},
                            readOnly = true,
                            label = { Text(stringResource(R.string.profile_auto_lock)) },
                            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = lockExpanded) },
                            modifier = Modifier.menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
                        )
                        ExposedDropdownMenu(expanded = lockExpanded, onDismissRequest = { lockExpanded = false }) {
                            lockOptions.forEach { mins ->
                                DropdownMenuItem(
                                    text = { Text(autoLockLabel(mins)) },
                                    onClick = { vm.setAutoLock(mins); lockExpanded = false },
                                )
                            }
                        }
                    }
                }
            }

            item {
                TextButton(
                    onClick = { confirmLogoutAll = true },
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text(stringResource(R.string.profile_logout_all))
                }
            }
        }
    }

    pendingDeleteBiometric?.let { bio ->
        val bioLabel = bio.label.ifBlank { stringResource(R.string.profile_biometric_default, bio.id) }
        AlertDialog(
            onDismissRequest = { pendingDeleteBiometric = null },
            title = { Text(stringResource(R.string.profile_biometric_delete_title)) },
            text = { Text(stringResource(R.string.profile_biometric_delete_confirm, bioLabel)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        vm.deleteBiometric(bio.id, context)
                        pendingDeleteBiometric = null
                    },
                ) {
                    Text(stringResource(R.string.common_delete), color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { pendingDeleteBiometric = null }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }

    pendingDeletePasskey?.let { pk ->
        val pkLabel = pk.label.ifBlank { stringResource(R.string.profile_passkey_default, pk.id) }
        AlertDialog(
            onDismissRequest = { pendingDeletePasskey = null },
            title = { Text(stringResource(R.string.profile_passkey_delete_title)) },
            text = { Text(stringResource(R.string.profile_passkey_delete_confirm, pkLabel)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        vm.deletePasskey(pk.id, context)
                        pendingDeletePasskey = null
                    },
                ) {
                    Text(stringResource(R.string.common_delete), color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { pendingDeletePasskey = null }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }

    if (confirmLogoutAll) {
        AlertDialog(
            onDismissRequest = { confirmLogoutAll = false },
            title = { Text(stringResource(R.string.profile_logout_all)) },
            text = { Text(stringResource(R.string.profile_logout_all_confirm)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmLogoutAll = false
                        vm.logoutAll()
                    },
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                ) {
                    Text(stringResource(R.string.common_confirm))
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmLogoutAll = false }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }
}
