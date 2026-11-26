package org.pjstatecm.graceride.ui.screens.profile

import android.Manifest
import android.content.Intent
import android.os.Build
import androidx.core.net.toUri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
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
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.Delete
import androidx.compose.material.icons.outlined.Fingerprint
import androidx.compose.material.icons.outlined.Key
import androidx.compose.material.icons.outlined.Lock
import androidx.compose.material.icons.outlined.NotificationsActive
import androidx.compose.material.icons.outlined.NotificationsOff
import androidx.compose.material.icons.outlined.QrCode2
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FilledTonalButton
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
import androidx.compose.runtime.DisposableEffect
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
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.width
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.applyAppLocale
import org.pjstatecm.graceride.data.api.BiometricInfo
import org.pjstatecm.graceride.data.api.PasskeyInfo
import org.pjstatecm.graceride.data.api.UpdateNotificationPrefsReq
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.notifications.NotificationHelper
import org.pjstatecm.graceride.platform.BiometricAvailability
import org.pjstatecm.graceride.platform.QrCodeCanvas
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.copyToClipboard
import org.pjstatecm.graceride.ui.openInWebApp
import org.pjstatecm.graceride.ui.SecureScreenEffect
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
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
    SecureScreenEffect()
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
    val textScaleMode by container.textScale.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val me = (authState as? AuthState.SignedIn)?.me ?: state.me
    val context = LocalContext.current
    val view = LocalView.current
    val snackbar = remember { SnackbarHostState() }
    val scope = androidx.compose.runtime.rememberCoroutineScope()

    val savedMsg = stringResource(R.string.profile_saved)
    val calReadyMsg = stringResource(R.string.profile_calendar_feed_ready)
    val calRevokedMsg = stringResource(R.string.profile_calendar_feed_revoked)
    val calCopiedMsg = stringResource(R.string.profile_calendar_feed_copied)

    LaunchedEffect(me?.id) {
        if (me != null && me.status != "pending") {
            vm.loadCalendarFeed()
        }
    }

    LaunchedEffect(state.message) {
        when (state.message) {
            "saved" -> {
                snackbar.showSnackbar(savedMsg)
                vm.clearMessage()
            }
            "calendar_ready" -> {
                snackbar.showSnackbar(calReadyMsg)
                vm.clearMessage()
            }
            "calendar_revoked" -> {
                snackbar.showSnackbar(calRevokedMsg)
                vm.clearMessage()
            }
        }
    }
    LaunchedEffect(state.error) {
        state.error?.let {
            snackbar.showSnackbar(it)
            vm.clearError()
        }
    }

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
                        Text(stringResource(R.string.profile_text_size), style = MaterialTheme.typography.titleSmall)
                        val textSizeOptions = listOf(
                            "normal" to stringResource(R.string.profile_text_size_normal),
                            "large" to stringResource(R.string.profile_text_size_large),
                            "xl" to stringResource(R.string.profile_text_size_xl),
                        )
                        SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
                            textSizeOptions.forEachIndexed { idx, (modeKey, label) ->
                                SegmentedButton(
                                    selected = textScaleMode == modeKey,
                                    onClick = {
                                        UiHaptics.tick(view)
                                        container.setTextScale(modeKey)
                                    },
                                    shape = SegmentedButtonDefaults.itemShape(index = idx, count = textSizeOptions.size),
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

            if (me != null && me.status != "pending") {
                item {
                    SettingsSurfaceCard {
                        Row(
                            Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Column(Modifier.weight(1f).padding(end = Spacing.xs)) {
                                Text(stringResource(R.string.profile_badges_opt_out), style = MaterialTheme.typography.titleSmall)
                                Text(
                                    stringResource(R.string.profile_badges_opt_out_hint),
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                            Switch(
                                checked = me.badgesOptOut,
                                enabled = !state.busy,
                                onCheckedChange = { wantOptOut ->
                                    UiHaptics.tick(view)
                                    vm.setBadgesOptOut(wantOptOut)
                                },
                            )
                        }
                    }
                }

                item {
                    SettingsSurfaceCard {
                        Text(stringResource(R.string.profile_calendar_feed_title), style = MaterialTheme.typography.titleSmall)
                        Text(
                            stringResource(R.string.profile_calendar_feed_desc),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        val feedUrl = state.calendarFeedUrl
                        if (!feedUrl.isNullOrBlank()) {
                            Text(
                                feedUrl,
                                style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                                color = MaterialTheme.colorScheme.primary,
                            )
                            Row(
                                Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                OutlinedButton(
                                    onClick = {
                                        if (copyToClipboard(context, "Calendar Feed", feedUrl, sensitive = true)) {
                                            scope.launch { snackbar.showSnackbar(calCopiedMsg) }
                                        }
                                    },
                                    modifier = Modifier.weight(1f),
                                ) {
                                    Text(stringResource(R.string.profile_calendar_feed_copy))
                                }
                                Button(
                                    onClick = {
                                        val webcalUrl = feedUrl
                                            .replaceFirst("https://", "webcal://")
                                            .replaceFirst("http://", "webcal://")
                                        val opened = runCatching {
                                            context.startActivity(
                                                Intent(Intent.ACTION_VIEW, webcalUrl.toUri()).apply {
                                                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                                                },
                                            )
                                            true
                                        }.getOrDefault(false)
                                        if (!opened) openInWebApp(context, feedUrl)
                                    },
                                    colors = graceRidePrimaryButtonColors(),
                                    modifier = Modifier.weight(1f),
                                ) {
                                    Text(stringResource(R.string.profile_calendar_feed_subscribe))
                                }
                            }
                        }
                        Row(
                            Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            OutlinedButton(
                                onClick = vm::rotateCalendarFeed,
                                enabled = !state.busy,
                                modifier = Modifier.weight(1f),
                            ) {
                                Text(
                                    if (state.calendarFeedEnabled == true) {
                                        stringResource(R.string.profile_calendar_feed_rotate)
                                    } else {
                                        stringResource(R.string.profile_calendar_feed_enable)
                                    },
                                )
                            }
                            if (state.calendarFeedEnabled == true) {
                                TextButton(
                                    onClick = vm::revokeCalendarFeed,
                                    enabled = !state.busy,
                                    colors = ButtonDefaults.textButtonColors(
                                        contentColor = MaterialTheme.colorScheme.error,
                                    ),
                                ) {
                                    Text(stringResource(R.string.profile_calendar_feed_revoke))
                                }
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
    SecureScreenEffect()
    val appLang by container.langFlow.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val view = LocalView.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val snackbar = remember { SnackbarHostState() }
    val statusColors = LocalStatusColors.current
    val prefsSavedMsg = stringResource(R.string.notif_pref_saved)
    val langSyncedMsg = stringResource(R.string.notif_pref_lang_synced)
    val me = when (val a = authState) {
        is AuthState.SignedIn -> a.me
        is AuthState.Pending -> a.me
        is AuthState.Locked -> a.me
        else -> null
    } ?: state.me

    var osNotifsEnabled by remember { mutableStateOf(NotificationHelper.areNotificationsEnabled(context)) }
    var permDeniedOnce by remember { mutableStateOf(false) }
    val permLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted ->
        osNotifsEnabled = NotificationHelper.areNotificationsEnabled(context)
        if (!granted) permDeniedOnce = true
    }
    DisposableEffect(lifecycleOwner, context) {
        val obs = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) {
                osNotifsEnabled = NotificationHelper.areNotificationsEnabled(context)
            }
        }
        lifecycleOwner.lifecycle.addObserver(obs)
        onDispose { lifecycleOwner.lifecycle.removeObserver(obs) }
    }

    val invalidQuietMsg = stringResource(R.string.notif_quiet_hours_invalid)
    var localErrorTick by remember { androidx.compose.runtime.mutableIntStateOf(0) }
    var dirty by remember(me?.id) { mutableStateOf(false) }

    var pushOptIn by remember { mutableStateOf(me?.pushOptIn ?: true) }
    var whatsappOptIn by remember { mutableStateOf(me?.whatsappOptIn ?: false) }
    var readinessPromptMins by remember {
        androidx.compose.runtime.mutableIntStateOf(me?.readinessPromptMins ?: 45)
    }
    var driverRemindMins by remember { androidx.compose.runtime.mutableIntStateOf(60) }
    var quietStart by remember { mutableStateOf("") }
    var quietEnd by remember { mutableStateOf("") }
    var hideLockDetails by remember { mutableStateOf(false) }
    var mutedCats by remember { mutableStateOf<List<String>>(emptyList()) }

    LaunchedEffect(me?.status) {
        if (me != null && me.status != "pending") {
            vm.loadNotificationPrefs()
            vm.loadPushAndTelegram()
        }
    }

    LaunchedEffect(state.notifPrefs) {
        state.notifPrefs?.let { p ->
            if (!dirty) {
                pushOptIn = p.pushOptIn
                whatsappOptIn = p.whatsappOptIn
                readinessPromptMins = p.readinessPromptMins
                driverRemindMins = p.driverRemindMins
                quietStart = p.quietHoursStart
                quietEnd = p.quietHoursEnd
                hideLockDetails = p.hideLockDetails
                mutedCats = p.mutedCategories
            }
        }
    }

    val tgTestSentMsg = stringResource(R.string.notif_pref_telegram_test_sent)
    val tgDiscMsg = stringResource(R.string.notif_pref_telegram_disconnected)
    val pushRemovedMsg = stringResource(R.string.notif_pref_push_removed)
    val pushZeroMsg = stringResource(R.string.notif_pref_push_test_zero)
    LaunchedEffect(state.message) {
        val msg = state.message ?: return@LaunchedEffect
        when {
            msg == "prefs_saved" -> {
                dirty = false
                snackbar.showSnackbar(prefsSavedMsg)
                vm.clearMessage()
            }
            msg == "lang_synced" -> {
                snackbar.showSnackbar(langSyncedMsg)
                vm.clearMessage()
            }
            msg == "telegram_test_sent" -> {
                snackbar.showSnackbar(tgTestSentMsg)
                vm.clearMessage()
            }
            msg == "telegram_disconnected" -> {
                snackbar.showSnackbar(tgDiscMsg)
                vm.clearMessage()
            }
            msg == "push_removed" -> {
                snackbar.showSnackbar(pushRemovedMsg)
                vm.clearMessage()
            }
            msg.startsWith("push_test:") -> {
                val parts = msg.removePrefix("push_test:").split(':')
                val delivered = parts.getOrNull(0)?.toIntOrNull() ?: 0
                val total = parts.getOrNull(1)?.toIntOrNull() ?: 0
                snackbar.showSnackbar(
                    if (delivered <= 0) {
                        pushZeroMsg
                    } else {
                        context.getString(R.string.notif_pref_push_test_sent, delivered, total)
                    },
                )
                vm.clearMessage()
            }
        }
    }
    LaunchedEffect(localErrorTick) {
        if (localErrorTick > 0) {
            snackbar.showSnackbar(invalidQuietMsg)
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
            // 1. Device OS notification permission & channel management
            item {
                val canRequestPrompt = Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
                    !NotificationHelper.hasPostPermission(context) &&
                    !permDeniedOnce
                SettingsSurfaceCard {
                    if (osNotifsEnabled) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            Icon(
                                Icons.Outlined.NotificationsActive,
                                contentDescription = null,
                                tint = statusColors.success,
                                modifier = Modifier.size(20.dp),
                            )
                            Text(
                                stringResource(R.string.notif_os_enabled_status),
                                style = MaterialTheme.typography.titleSmall,
                                color = statusColors.success,
                            )
                        }
                    } else {
                        Row(
                            verticalAlignment = Alignment.Top,
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            Icon(
                                Icons.Outlined.NotificationsOff,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.error,
                                modifier = Modifier.padding(top = 2.dp).size(20.dp),
                            )
                            Column(
                                modifier = Modifier.weight(1f),
                                verticalArrangement = Arrangement.spacedBy(2.dp),
                            ) {
                                Text(
                                    stringResource(R.string.notif_os_disabled_title),
                                    style = MaterialTheme.typography.titleSmall,
                                    color = MaterialTheme.colorScheme.error,
                                )
                                Text(
                                    stringResource(R.string.notif_os_disabled_body),
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                        FilledTonalButton(
                            onClick = {
                                UiHaptics.tick(view)
                                if (canRequestPrompt) {
                                    permLauncher.launch(Manifest.permission.POST_NOTIFICATIONS)
                                } else {
                                    runCatching {
                                        context.startActivity(NotificationHelper.openNotificationSettingsIntent(context))
                                    }
                                }
                            },
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Text(
                                stringResource(
                                    if (canRequestPrompt) R.string.notif_os_enable_btn
                                    else R.string.notif_os_open_settings,
                                ),
                            )
                        }
                    }
                    OutlinedButton(
                        onClick = {
                            UiHaptics.tick(view)
                            runCatching {
                                context.startActivity(NotificationHelper.openNotificationSettingsIntent(context))
                            }
                        },
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Text(stringResource(R.string.notif_os_channels_btn))
                    }
                }
            }

            // 2. Notification language & server sync check
            item {
                val serverLang = me?.lang ?: appLang
                val serverLangLabel = stringResource(
                    if (serverLang == "zh") R.string.profile_lang_zh else R.string.profile_lang_en,
                )
                val appLangLabel = stringResource(
                    if (appLang == "zh") R.string.profile_lang_zh else R.string.profile_lang_en,
                )
                SettingsSurfaceCard {
                    Text(
                        stringResource(R.string.notif_pref_lang_title),
                        style = MaterialTheme.typography.titleSmall,
                    )
                    Text(
                        stringResource(R.string.notif_pref_lang_desc, serverLangLabel),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    if (me != null && me.lang != appLang) {
                        Text(
                            stringResource(R.string.notif_pref_lang_mismatch, serverLangLabel, appLangLabel),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                        )
                        FilledTonalButton(
                            onClick = {
                                UiHaptics.tick(view)
                                vm.syncServerLangToApp()
                            },
                            enabled = !state.busy,
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Text(stringResource(R.string.notif_pref_lang_sync_btn))
                        }
                    }
                }
            }

            // 3. Alert channels & lock-screen privacy
            item {
                SettingsSurfaceCard {
                    Row(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(Modifier.weight(1f).padding(end = Spacing.xs)) {
                            Text(stringResource(R.string.notif_pref_push))
                            Text(
                                stringResource(R.string.notif_pref_push_hint),
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        Switch(checked = pushOptIn, onCheckedChange = { dirty = true; pushOptIn = it })
                    }
                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                    Row(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(Modifier.weight(1f).padding(end = Spacing.xs)) {
                            Text(stringResource(R.string.notif_pref_whatsapp))
                            Text(
                                stringResource(R.string.notif_pref_whatsapp_hint),
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            if (state.pushConfig?.whatsappConfigured == false) {
                                Text(
                                    stringResource(R.string.notif_pref_whatsapp_not_configured),
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                        Switch(checked = whatsappOptIn, onCheckedChange = { dirty = true; whatsappOptIn = it })
                    }
                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                    Row(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(Modifier.weight(1f).padding(end = Spacing.xs)) {
                            Text(stringResource(R.string.notif_pref_hide_lock))
                            Text(
                                stringResource(R.string.notif_pref_hide_lock_hint),
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        Switch(checked = hideLockDetails, onCheckedChange = { dirty = true; hideLockDetails = it })
                    }
                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                    val tgId = state.notifPrefs?.telegramChatId?.trim().orEmpty()
                    val tgBlocked = state.notifPrefs?.telegramBlocked == true
                    val pushCfg = state.pushConfig
                    Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        Text(
                            stringResource(R.string.notif_pref_telegram_title),
                            style = MaterialTheme.typography.titleSmall,
                        )
                        Text(
                            when {
                                pushCfg != null && !pushCfg.telegramConfigured -> stringResource(R.string.notif_pref_telegram_not_configured)
                                tgId.isNotEmpty() && tgBlocked -> stringResource(R.string.notif_pref_telegram_blocked)
                                tgId.isNotEmpty() -> stringResource(R.string.notif_pref_telegram_linked, tgId)
                                else -> stringResource(R.string.notif_pref_telegram_not_linked)
                            },
                            style = MaterialTheme.typography.bodySmall,
                            color = if (tgId.isNotEmpty() && tgBlocked) {
                                MaterialTheme.colorScheme.error
                            } else {
                                MaterialTheme.colorScheme.onSurfaceVariant
                            },
                        )
                        if (pushCfg == null || pushCfg.telegramConfigured) {
                            FlowRow(
                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                if (tgId.isEmpty()) {
                                    OutlinedButton(
                                        onClick = {
                                            UiHaptics.tick(view)
                                            vm.connectTelegram { url -> openInWebApp(context, url) }
                                        },
                                        enabled = !state.busy,
                                    ) {
                                        Text(stringResource(R.string.notif_pref_telegram_connect))
                                    }
                                    val linkUrl = state.telegramLinkUrl
                                    if (!linkUrl.isNullOrBlank() && !pushCfg?.telegramBotUsername.isNullOrBlank()) {
                                        TextButton(onClick = { openInWebApp(context, linkUrl) }) {
                                            Text(stringResource(R.string.notif_pref_telegram_open_bot, pushCfg!!.telegramBotUsername))
                                        }
                                    }
                                } else {
                                    OutlinedButton(
                                        onClick = {
                                            UiHaptics.tick(view)
                                            vm.testTelegram()
                                        },
                                        enabled = !state.busy,
                                    ) {
                                        Text(stringResource(R.string.notif_pref_telegram_test))
                                    }
                                    TextButton(
                                        onClick = {
                                            UiHaptics.tick(view)
                                            vm.disconnectTelegram()
                                        },
                                        enabled = !state.busy,
                                        colors = ButtonDefaults.textButtonColors(
                                            contentColor = MaterialTheme.colorScheme.error,
                                        ),
                                    ) {
                                        Text(stringResource(R.string.notif_pref_telegram_disconnect))
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 4. Passenger pre-trip readiness reminder
            if (me?.can("rides") != false) {
                item { SectionHeader(stringResource(R.string.notif_pref_readiness_title)) }
                item {
                    SettingsSurfaceCard {
                        Text(
                            stringResource(R.string.notif_pref_readiness_hint),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            listOf(30, 45, 60, 0).forEach { mins ->
                                FilterChip(
                                    selected = readinessPromptMins == mins,
                                    onClick = { dirty = true; readinessPromptMins = mins },
                                    label = {
                                        Text(
                                            if (mins == 0) stringResource(R.string.notif_pref_readiness_off)
                                            else stringResource(R.string.notif_pref_driver_mins, mins),
                                        )
                                    },
                                )
                            }
                        }
                    }
                }
            }

            // 5. Driver pickup reminder
            if (me?.can("drive") == true) {
                item { SectionHeader(stringResource(R.string.notif_pref_driver_remind)) }
                item {
                    SettingsSurfaceCard {
                        Text(
                            stringResource(R.string.notif_pref_driver_remind_hint),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            listOf(30, 60, 90, 120).forEach { mins ->
                                FilterChip(
                                    selected = driverRemindMins == mins,
                                    onClick = { dirty = true; driverRemindMins = mins },
                                    label = { Text(stringResource(R.string.notif_pref_driver_mins, mins)) },
                                )
                            }
                        }
                    }
                }
            }

            // 6. Quiet hours & muted topics
            item { SectionHeader(stringResource(R.string.notif_pref_quiet_hours)) }
            item {
                SettingsSurfaceCard {
                    Text(
                        stringResource(R.string.notif_pref_quiet_hours_hint),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        OutlinedTextField(
                            value = quietStart,
                            onValueChange = { dirty = true; quietStart = it },
                            label = { Text(stringResource(R.string.notif_pref_quiet_start)) },
                            singleLine = true,
                            modifier = Modifier.weight(1f),
                        )
                        OutlinedTextField(
                            value = quietEnd,
                            onValueChange = { dirty = true; quietEnd = it },
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
                    Text(
                        stringResource(R.string.notif_pref_muted_cats_hint),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        buildList {
                            add("ride" to R.string.notif_cat_ride)
                            add("chat" to R.string.notif_cat_chat)
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
                                    dirty = true
                                    mutedCats = if (muted) mutedCats - cat else mutedCats + cat
                                },
                                label = { Text(stringResource(labelRes)) },
                            )
                        }
                    }
                }
            }

            val pushSubs = state.pushSubs
            if (pushSubs != null && (pushSubs.web.isNotEmpty() || pushSubs.mobile.isNotEmpty())) {
                item { SectionHeader(stringResource(R.string.notif_pref_push_devices)) }
                item {
                    SettingsSurfaceCard {
                        pushSubs.mobile.forEach { dev ->
                            Row(
                                Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Column(Modifier.weight(1f)) {
                                    Text(
                                        "${stringResource(R.string.notif_pref_push_device_mobile)} · ${dev.deviceLabel.ifBlank { dev.platform }}",
                                        style = MaterialTheme.typography.bodyMedium,
                                    )
                                    dev.lastDeliveredAt?.let { ts ->
                                        Text(
                                            stringResource(R.string.notif_pref_push_last_delivered, KlTime.display(ts)),
                                            style = MaterialTheme.typography.bodySmall,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                    }
                                }
                            }
                        }
                        pushSubs.web.forEach { sub ->
                            Row(
                                Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Column(Modifier.weight(1f)) {
                                    Text(
                                        "${stringResource(R.string.notif_pref_push_device_web)} · ${sub.deviceLabel.ifBlank { sub.endpointPreview }}",
                                        style = MaterialTheme.typography.bodyMedium,
                                    )
                                    sub.lastDeliveredAt?.let { ts ->
                                        Text(
                                            stringResource(R.string.notif_pref_push_last_delivered, KlTime.display(ts)),
                                            style = MaterialTheme.typography.bodySmall,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                    }
                                }
                                IconButton(
                                    onClick = { vm.deletePushSubscription(sub.id) },
                                    enabled = !state.busy,
                                ) {
                                    Icon(
                                        Icons.Outlined.Delete,
                                        contentDescription = stringResource(R.string.notif_pref_push_remove),
                                        tint = MaterialTheme.colorScheme.error,
                                    )
                                }
                            }
                        }
                        if (pushSubs.web.isNotEmpty()) {
                            OutlinedButton(
                                onClick = vm::sendTestPush,
                                enabled = !state.busy,
                                modifier = Modifier.fillMaxWidth(),
                            ) {
                                Text(stringResource(R.string.notif_pref_push_test))
                            }
                        }
                    }
                }
            }

            item {
                val quietRe = remember { Regex("^([01]\\d|2[0-3]):[0-5]\\d$") }
                Button(
                    onClick = {
                        UiHaptics.confirm(view)
                        val qStart = quietStart.trim()
                        val qEnd = quietEnd.trim()
                        val quietValid = (qStart.isEmpty() && qEnd.isEmpty()) ||
                            (quietRe.matches(qStart) && quietRe.matches(qEnd))
                        if (!quietValid) {
                            localErrorTick++
                            return@Button
                        }
                        vm.saveNotificationPrefs(
                            UpdateNotificationPrefsReq(
                                push_opt_in = pushOptIn,
                                whatsapp_opt_in = whatsappOptIn,
                                readiness_prompt_mins = readinessPromptMins,
                                driver_remind_mins = driverRemindMins,
                                muted_categories = mutedCats,
                                quiet_hours_start = qStart,
                                quiet_hours_end = qEnd,
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
    val totpEnabledMsg = stringResource(R.string.profile_totp_enabled_msg)
    val regenCodesMsg = stringResource(R.string.profile_regen_backup_codes_done)
    val staff2faMsg = stringResource(R.string.profile_verify_2fa_done)
    val secretCopiedMsg = stringResource(R.string.profile_totp_secret_copied)
    val backupCopiedMsg = stringResource(R.string.profile_backup_codes_copied)
    var pendingDeletePasskey by remember { mutableStateOf<PasskeyInfo?>(null) }
    var pendingDeleteBiometric by remember { mutableStateOf<BiometricInfo?>(null) }
    var confirmLogoutAll by remember { mutableStateOf(false) }
    var confirmRegenBackupCodes by remember { mutableStateOf(false) }
    var totpCodeInput by remember { mutableStateOf("") }
    var staffVerifyCodeInput by remember { mutableStateOf("") }
    val scope = androidx.compose.runtime.rememberCoroutineScope()

    if (state.totpSetup != null || state.backupCodes.isNotEmpty()) {
        SecureScreenEffect()
    }

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
            "totp_enabled" -> {
                totpCodeInput = ""
                snackbar.showSnackbar(totpEnabledMsg)
                vm.clearMessage()
            }
            "backup_codes_regen" -> {
                snackbar.showSnackbar(regenCodesMsg)
                vm.clearMessage()
            }
            "staff_2fa_verified" -> {
                staffVerifyCodeInput = ""
                snackbar.showSnackbar(staff2faMsg)
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

            item { SectionHeader(stringResource(R.string.profile_totp_title)) }
            item {
                SettingsSurfaceCard {
                    val totpSetup = state.totpSetup
                    if (me?.totpEnabled == true) {
                        Row(
                            Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                Icon(Icons.Outlined.QrCode2, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                                Text(stringResource(R.string.profile_totp_enabled), style = MaterialTheme.typography.titleSmall)
                            }
                        }
                        if (me.privileged && !me.mfaOk) {
                            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                            Text(
                                stringResource(R.string.profile_verify_2fa),
                                style = MaterialTheme.typography.titleSmall,
                            )
                            Text(
                                stringResource(R.string.profile_verify_2fa_desc),
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            OutlinedTextField(
                                value = staffVerifyCodeInput,
                                onValueChange = { staffVerifyCodeInput = it },
                                label = { Text(stringResource(R.string.profile_verify_2fa_code)) },
                                singleLine = true,
                                modifier = Modifier.fillMaxWidth(),
                            )
                            Button(
                                onClick = { vm.verifyTotpForStaff(staffVerifyCodeInput) },
                                enabled = staffVerifyCodeInput.isNotBlank() && !state.busy,
                                colors = graceRidePrimaryButtonColors(),
                                modifier = Modifier.fillMaxWidth(),
                            ) {
                                Text(stringResource(R.string.profile_verify_2fa))
                            }
                        }
                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                        OutlinedButton(
                            onClick = { confirmRegenBackupCodes = true },
                            enabled = !state.busy,
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Icon(Icons.Outlined.Refresh, contentDescription = null, modifier = Modifier.size(18.dp))
                            Spacer(Modifier.width(Spacing.xs))
                            Text(stringResource(R.string.profile_regen_backup_codes))
                        }
                    } else if (totpSetup == null) {
                        Text(
                            stringResource(R.string.profile_totp_disabled),
                            style = MaterialTheme.typography.titleSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Text(
                            stringResource(R.string.profile_totp_note),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        OutlinedButton(
                            onClick = { vm.startTotpSetup(context) },
                            enabled = !state.busy,
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Icon(Icons.Outlined.QrCode2, contentDescription = null, modifier = Modifier.size(18.dp))
                            Spacer(Modifier.width(Spacing.xs))
                            Text(stringResource(R.string.profile_totp_setup_btn))
                        }
                    } else {
                        Text(
                            stringResource(R.string.profile_totp_setup_step1),
                            style = MaterialTheme.typography.bodySmall,
                        )
                        if (totpSetup.url.isNotBlank()) {
                            QrCodeCanvas(
                                content = totpSetup.url,
                                modifier = Modifier
                                    .size(200.dp)
                                    .align(Alignment.CenterHorizontally),
                            )
                        }
                        if (totpSetup.secret.isNotBlank()) {
                            Text(
                                totpSetup.secret,
                                style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                                color = MaterialTheme.colorScheme.primary,
                            )
                        }
                        Row(
                            Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            OutlinedButton(
                                onClick = {
                                    if (copyToClipboard(context, "TOTP secret", totpSetup.secret, sensitive = true)) {
                                        scope.launch { snackbar.showSnackbar(secretCopiedMsg) }
                                    }
                                },
                                modifier = Modifier.weight(1f),
                            ) {
                                Icon(Icons.Outlined.ContentCopy, contentDescription = null, modifier = Modifier.size(16.dp))
                                Spacer(Modifier.width(Spacing.xxs))
                                Text(stringResource(R.string.profile_totp_copy_secret))
                            }
                            if (totpSetup.url.isNotBlank()) {
                                OutlinedButton(
                                    onClick = {
                                        runCatching {
                                            context.startActivity(
                                                Intent(Intent.ACTION_VIEW, totpSetup.url.toUri()).apply {
                                                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                                                },
                                            )
                                        }
                                    },
                                    modifier = Modifier.weight(1f),
                                ) {
                                    Text(stringResource(R.string.profile_totp_open_app))
                                }
                            }
                        }
                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                        Text(
                            stringResource(R.string.profile_totp_setup_step2),
                            style = MaterialTheme.typography.bodySmall,
                        )
                        OutlinedTextField(
                            value = totpCodeInput,
                            onValueChange = { totpCodeInput = it.filter { ch -> ch.isDigit() }.take(8) },
                            label = { Text(stringResource(R.string.profile_totp_code_label)) },
                            singleLine = true,
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                            modifier = Modifier.fillMaxWidth(),
                        )
                        Row(
                            Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            Button(
                                onClick = { vm.enableTotp(totpCodeInput) },
                                enabled = totpCodeInput.length >= 6 && !state.busy,
                                colors = graceRidePrimaryButtonColors(),
                                modifier = Modifier.weight(1f),
                            ) {
                                Text(stringResource(R.string.profile_totp_enable_btn))
                            }
                            TextButton(
                                onClick = vm::cancelTotpSetup,
                                enabled = !state.busy,
                            ) {
                                Text(stringResource(R.string.common_cancel))
                            }
                        }
                    }
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
                OutlinedButton(
                    onClick = { vm.lockSessionNow(context) },
                    enabled = !state.busy,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Icon(Icons.Outlined.Lock, contentDescription = null, modifier = Modifier.size(18.dp))
                    Spacer(Modifier.width(Spacing.xs))
                    Text(stringResource(R.string.profile_lock_now))
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

    if (confirmRegenBackupCodes) {
        AlertDialog(
            onDismissRequest = { confirmRegenBackupCodes = false },
            title = { Text(stringResource(R.string.profile_regen_backup_codes)) },
            text = { Text(stringResource(R.string.profile_regen_backup_codes_confirm)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmRegenBackupCodes = false
                        vm.regenerateBackupCodes(context)
                    },
                ) {
                    Text(stringResource(R.string.common_confirm))
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmRegenBackupCodes = false }) {
                    Text(stringResource(R.string.common_cancel))
                }
            },
        )
    }

    if (state.backupCodes.isNotEmpty()) {
        val joined = state.backupCodes.joinToString("\n")
        AlertDialog(
            onDismissRequest = vm::dismissBackupCodes,
            title = { Text(stringResource(R.string.profile_backup_codes_title)) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    Text(
                        stringResource(R.string.profile_backup_codes_desc),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Text(
                        joined,
                        style = MaterialTheme.typography.bodyMedium.copy(fontFamily = FontFamily.Monospace),
                    )
                }
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        if (copyToClipboard(context, "Backup codes", joined, sensitive = true)) {
                            scope.launch { snackbar.showSnackbar(backupCopiedMsg) }
                        }
                    },
                ) {
                    Text(stringResource(R.string.profile_backup_codes_copy))
                }
            },
            dismissButton = {
                TextButton(onClick = vm::dismissBackupCodes) {
                    Text(stringResource(R.string.common_close))
                }
            },
        )
    }
}
