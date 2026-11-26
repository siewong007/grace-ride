package org.pjstatecm.graceride.ui.screens.profile

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
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
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.HelpOutline
import androidx.compose.material.icons.automirrored.outlined.Logout
import androidx.compose.material.icons.automirrored.outlined.ReceiptLong
import androidx.compose.material.icons.outlined.AdminPanelSettings
import androidx.compose.material.icons.outlined.AutoAwesome
import androidx.compose.material.icons.outlined.BarChart
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Edit
import androidx.compose.material.icons.outlined.Groups
import androidx.compose.material.icons.outlined.LocationOn
import androidx.compose.material.icons.outlined.NotificationsActive
import androidx.compose.material.icons.outlined.Palette
import androidx.compose.material.icons.outlined.Person
import androidx.compose.material.icons.outlined.Security
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.LocalBottomBarClearance
import org.pjstatecm.graceride.ui.NavProvider
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.components.GrokBotSpec
import org.pjstatecm.graceride.ui.components.MemberAvatar
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.openInWebApp
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.ThemeMode

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun ProfileScreen(container: AppContainer, nav: NavHostController) {
    val vm: ProfileViewModel = viewModel(factory = ProfileViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val biometricEnrollment by vm.biometricEnrollment.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val themeModeKey by container.themeMode.collectAsStateWithLifecycle()
    val navAppPref by container.navAppPref.collectAsStateWithLifecycle()
    val autoArriveGate by container.autoArriveGate.collectAsStateWithLifecycle()
    val context = LocalContext.current

    val liveMe = when (val a = authState) {
        is AuthState.SignedIn -> a.me
        is AuthState.Pending -> a.me
        is AuthState.Locked -> a.me
        else -> null
    }
    val me = liveMe ?: state.me
    val snackbar = remember { SnackbarHostState() }
    val savedMsg = stringResource(R.string.profile_saved)
    var showAvatarEditor by remember { mutableStateOf(false) }

    LaunchedEffect(me?.id) {
        if (me != null) {
            vm.loadBiometrics()
            vm.loadPasskeys()
        }
    }

    LaunchedEffect(me?.status) {
        if (me != null && me.status != "pending") {
            vm.loadNotificationPrefs()
        }
    }

    LaunchedEffect(state.message) {
        if (state.message == "saved") {
            snackbar.showSnackbar(savedMsg)
            vm.clearMessage()
        }
    }

    LaunchedEffect(state.error, showAvatarEditor) {
        val err = state.error
        if (err != null && !showAvatarEditor) {
            snackbar.showSnackbar(err)
            vm.clearError()
        }
    }

    val themeLabel = when (ThemeMode.fromKey(themeModeKey)) {
        ThemeMode.LIGHT -> stringResource(R.string.profile_theme_light)
        ThemeMode.DARK -> stringResource(R.string.profile_theme_dark)
        ThemeMode.SYSTEM -> stringResource(R.string.profile_theme_system)
    }
    val langLabel = if (state.lang == "zh") {
        stringResource(R.string.profile_lang_zh)
    } else {
        stringResource(R.string.profile_lang_en)
    }
    val onWord = stringResource(R.string.common_on)
    val offWord = stringResource(R.string.common_off)
    val pushOn = state.notifPrefs?.pushOptIn ?: me?.pushOptIn ?: true
    val waOn = state.notifPrefs?.whatsappOptIn ?: me?.whatsappOptIn ?: false
    val notifSummary = stringResource(
        R.string.notif_summary_channels,
        if (pushOn) onWord else offWord,
        if (waOn) onWord else offWord,
    )
    val bioEnabledHere = me != null && biometricEnrollment?.userId == me.id
    val lockText = when (val mins = me?.autoLockMins ?: 0) {
        0 -> stringResource(R.string.profile_auto_lock_default)
        in Int.MIN_VALUE..-1 -> stringResource(R.string.profile_auto_lock_off)
        else -> stringResource(R.string.profile_auto_lock_mins, mins)
    }
    val securitySummary = buildList {
        if (bioEnabledHere) add(stringResource(R.string.profile_biometric_summary_on))
        add(stringResource(R.string.profile_passkey_count, state.passkeys.size))
        add(lockText)
    }.joinToString(" · ")
    val navLabel = when (navAppPref) {
        NavProvider.WAZE.key -> stringResource(R.string.nav_app_waze)
        NavProvider.GOOGLE_MAPS.key -> stringResource(R.string.nav_app_google)
        else -> stringResource(R.string.nav_app_ask)
    }
    val driverPrefSummary = buildList {
        add(navLabel)
        if (autoArriveGate) add(stringResource(R.string.driver_auto_arrive_title))
    }.joinToString(" · ")

    val bottomClearance = LocalBottomBarClearance.current

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        snackbarHost = { SnackbarHost(snackbar, modifier = Modifier.padding(bottom = bottomClearance)) },
    ) { padding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(padding).padding(horizontal = Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
            contentPadding = PaddingValues(top = Spacing.sm, bottom = Spacing.sm + bottomClearance),
        ) {
            // 1. Identity Header Card
            item {
                OutlinedCard(
                    modifier = Modifier.fillMaxWidth(),
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                ) {
                    Row(
                        Modifier.fillMaxWidth().padding(Spacing.sm),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(Spacing.sm),
                    ) {
                        Box(
                            modifier = Modifier.clickable { showAvatarEditor = true },
                            contentAlignment = Alignment.BottomEnd,
                        ) {
                            MemberAvatar(
                                name = me?.name ?: "",
                                photoUrl = me?.photoFileId?.let(container::fileUrl),
                                avatarColor = me?.avatarColor,
                                avatarIcon = me?.avatarIcon,
                                size = 68.dp,
                            )
                            Surface(
                                color = MaterialTheme.colorScheme.primary,
                                contentColor = MaterialTheme.colorScheme.onPrimary,
                                shape = CircleShape,
                                border = BorderStroke(2.dp, MaterialTheme.colorScheme.surface),
                                modifier = Modifier.size(24.dp),
                            ) {
                                Box(contentAlignment = Alignment.Center) {
                                    Icon(
                                        Icons.Outlined.Edit,
                                        contentDescription = stringResource(R.string.profile_edit_avatar),
                                        modifier = Modifier.size(13.dp),
                                    )
                                }
                            }
                        }
                        Column(
                            modifier = Modifier
                                .weight(1f)
                                .clickable { nav.navigate(AppRoute.PersonalDetails) },
                            verticalArrangement = Arrangement.spacedBy(4.dp),
                        ) {
                            Text(
                                me?.name ?: "",
                                style = MaterialTheme.typography.headlineSmall,
                                color = MaterialTheme.colorScheme.onSurface,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                            Text(
                                me?.email ?: "",
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                            val roles = me?.roles.orEmpty()
                            if (roles.isNotEmpty()) {
                                FlowRow(
                                    horizontalArrangement = Arrangement.spacedBy(Spacing.xxs),
                                    verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                                ) {
                                    roles.forEach { roleKey ->
                                        Surface(
                                            color = MaterialTheme.colorScheme.primaryContainer,
                                            contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                                            shape = CircleShape,
                                        ) {
                                            val roleLabel = builtInRoleLabelRes(roleKey)?.let { stringResource(it) }
                                                ?: me?.roleNames?.get(roleKey)
                                                ?: roleKey
                                            Text(
                                                roleLabel,
                                                style = MaterialTheme.typography.labelMedium,
                                                modifier = Modifier.padding(horizontal = Spacing.xs, vertical = 2.dp),
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 2. Account & Saved Setup (non-duplicated ride tools only)
            item { SectionHeader(stringResource(R.string.profile_section_setup)) }
            item {
                OutlinedCard(
                    modifier = Modifier.fillMaxWidth(),
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                ) {
                    Column {
                        ProfileRow(
                            label = stringResource(R.string.profile_personal),
                            subtitle = me?.phone?.takeIf { it.isNotBlank() }
                                ?: stringResource(R.string.profile_phone_not_set),
                            icon = Icons.Outlined.Person,
                        ) { nav.navigate(AppRoute.PersonalDetails) }
                        if (me?.can("rides") == true) {
                            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                            ProfileRow(
                                label = stringResource(R.string.saved_addresses_title),
                                icon = Icons.Outlined.LocationOn,
                            ) { nav.navigate(AppRoute.SavedAddresses) }
                            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                            ProfileRow(
                                label = stringResource(R.string.proxy_riders_title),
                                icon = Icons.Outlined.Groups,
                            ) { nav.navigate(AppRoute.ProxyRiders) }
                        }
                        if (me?.can("drive") == true) {
                            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                            ProfileRow(
                                label = stringResource(R.string.driver_settings_title),
                                subtitle = driverPrefSummary,
                                icon = Icons.Outlined.DirectionsCar,
                            ) { nav.navigate(AppRoute.DriverSettings) }
                        }
                    }
                }
            }

            // 3. Preferences, Alerts, Security & Studio (sliding into native sub-screens)
            item { SectionHeader(stringResource(R.string.profile_preferences)) }
            item {
                OutlinedCard(
                    modifier = Modifier.fillMaxWidth(),
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                ) {
                    Column {
                        ProfileRow(
                            label = stringResource(R.string.profile_appearance_lang),
                            subtitle = "$themeLabel · $langLabel",
                            icon = Icons.Outlined.Palette,
                        ) { nav.navigate(AppRoute.AppearanceSettings) }
                        if (me != null && me.status != "pending") {
                            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                            ProfileRow(
                                label = stringResource(R.string.notif_prefs_title),
                                subtitle = notifSummary,
                                icon = Icons.Outlined.NotificationsActive,
                            ) { nav.navigate(AppRoute.NotificationSettings) }
                        }
                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                        ProfileRow(
                            label = stringResource(R.string.profile_security),
                            subtitle = securitySummary,
                            icon = Icons.Outlined.Security,
                        ) { nav.navigate(AppRoute.SecuritySettings) }
                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                        ProfileRow(
                            label = stringResource(R.string.profile_open_grok_studio),
                            icon = Icons.Outlined.AutoAwesome,
                        ) {
                            val gb = GrokBotSpec.parse(me?.avatarIcon)
                            nav.navigate(AppRoute.Studio(gb?.shape, gb?.color, gb?.face))
                        }
                    }
                }
            }

            // 4. More in Web App (staff, admin, and event organisers only — permission-gated)
            if (me != null && me.status != "pending" && Routes.canUseStaffWeb(me)) {
                val webRows = buildList<Triple<Int, ImageVector, String>> {
                    staffConsolePathFor(me)?.let { add(Triple(R.string.profile_web_staff, Icons.Outlined.AdminPanelSettings, it)) }
                    if (me.can("admin_reports")) add(Triple(R.string.profile_admin_reports, Icons.Outlined.BarChart, "admin/reports"))
                    if (Routes.canManageSettlements(me)) add(Triple(R.string.profile_settlements, Icons.AutoMirrored.Outlined.ReceiptLong, "settlements"))
                    if (me.can("reimbursements")) add(Triple(R.string.profile_reimbursements, Icons.AutoMirrored.Outlined.ReceiptLong, "reimbursements"))
                    if (me.can("report")) add(Triple(R.string.profile_report, Icons.Outlined.BarChart, "report"))
                    if (me.can("help") || me.can("admin_help")) add(Triple(R.string.profile_help, Icons.AutoMirrored.Outlined.HelpOutline, "help"))
                }
                if (webRows.isNotEmpty()) {
                    item { SectionHeader(stringResource(R.string.profile_section_web)) }
                    item {
                        OutlinedCard(
                            modifier = Modifier.fillMaxWidth(),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                        ) {
                            Column {
                                webRows.forEachIndexed { idx, (labelRes, icon, path) ->
                                    if (idx > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                    ProfileRow(
                                        label = stringResource(labelRes),
                                        icon = icon,
                                        isExternal = true,
                                    ) {
                                        openInWebApp(context, container.webUrl(path))
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 5. Clean single Sign Out action (Sign out all devices lives inside Security Settings)
            item {
                OutlinedButton(
                    onClick = { vm.logout() },
                    modifier = Modifier.fillMaxWidth().padding(top = Spacing.xs).height(48.dp),
                ) {
                    Icon(Icons.AutoMirrored.Outlined.Logout, contentDescription = null, modifier = Modifier.size(18.dp))
                    Spacer(Modifier.width(Spacing.xs))
                    Text(stringResource(R.string.profile_logout))
                }
            }
        }
    }

    if (showAvatarEditor && me != null) {
        AvatarEditorSheet(
            me = me,
            photoUrlFor = container::fileUrl,
            busy = state.busy,
            error = state.error,
            onDismiss = {
                vm.clearError()
                showAvatarEditor = false
            },
            onOpenFullStudio = { spec ->
                vm.clearError()
                showAvatarEditor = false
                nav.navigate(AppRoute.Studio(spec.shape, spec.color, spec.face))
            },
            onSavePhoto = { uri, color ->
                if (uri != null) {
                    vm.saveAvatarPhoto(context, uri, color) { showAvatarEditor = false }
                } else if (me.photoFileId != null) {
                    vm.saveAvatarPhotoColor(me.photoFileId, color) { showAvatarEditor = false }
                }
            },
            onSaveStyle = { icon, color ->
                vm.saveAvatarStyle(icon, color) { showAvatarEditor = false }
            },
        )
    }
}

private fun builtInRoleLabelRes(name: String): Int? = when (name) {
    "member" -> R.string.role_member
    "superadmin" -> R.string.role_superadmin
    "admin" -> R.string.role_admin
    "president" -> R.string.role_president
    "vice_president" -> R.string.role_vice_president
    "secretary" -> R.string.role_secretary
    "traffic_head" -> R.string.role_traffic_head
    "traffic_member" -> R.string.role_traffic_member
    "cell_leader" -> R.string.role_cell_leader
    "transport" -> R.string.role_transport
    "finance" -> R.string.role_finance
    "auditor" -> R.string.role_auditor
    "leader" -> R.string.role_leader
    "driver" -> R.string.role_driver
    else -> null
}

private val TRAFFIC_HUB_PERMS = listOf(
    "admin_plan",
    "admin_rides",
    "admin_live",
    "admin_area_photos",
    "admin_areas",
    "admin_fares",
)

private val NON_TRAFFIC_STAFF_PERMS = listOf(
    "admin_members",
    "admin_invites",
    "admin_recovery",
    "admin_roles",
    "admin_reimb_review",
    "admin_reimb_pay",
    "admin_reports",
    "admin_announcements",
    "admin_help",
    "admin_groups",
    "admin_settings",
    "admin_audit",
)

internal fun staffConsolePathFor(me: Me): String? {
    val canTraffic = TRAFFIC_HUB_PERMS.any { me.can(it) }
    val hasNonTraffic = NON_TRAFFIC_STAFF_PERMS.any { me.can(it) }
    return when {
        canTraffic && !hasNonTraffic -> "admin/traffic"
        canTraffic || hasNonTraffic || me.can("admin_dashboard") -> "admin/menu"
        else -> null
    }
}
