package org.pjstatecm.graceride.ui

import android.Manifest
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.snap
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.displayCutout
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.systemBars
import androidx.compose.foundation.layout.union
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.AltRoute
import androidx.compose.material.icons.automirrored.outlined.AltRoute
import androidx.compose.material.icons.filled.Event
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.outlined.CloudOff
import androidx.compose.material.icons.outlined.Event
import androidx.compose.material.icons.outlined.Home
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.NavigationDrawerItemDefaults
import androidx.compose.material3.NavigationRailItemDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.VerticalDivider
import androidx.compose.material3.adaptive.currentWindowAdaptiveInfo
import androidx.compose.material3.adaptive.navigationsuite.NavigationSuiteDefaults
import androidx.compose.material3.adaptive.navigationsuite.NavigationSuiteScaffold
import androidx.compose.material3.adaptive.navigationsuite.NavigationSuiteType
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import kotlin.math.abs
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import androidx.core.net.toUri
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.runSuspendCatching
import org.pjstatecm.graceride.platform.BiometricSignResult
import org.pjstatecm.graceride.ui.components.GraceRideLandingSplash
import org.pjstatecm.graceride.ui.components.GraceRideSpringEasing
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.MemberAvatar
import org.pjstatecm.graceride.ui.components.animatorsEnabled
import org.pjstatecm.graceride.ui.screens.drive.DriveHubScreen
import org.pjstatecm.graceride.ui.screens.login.BiometricGateScreen
import org.pjstatecm.graceride.ui.screens.profile.AppearanceSettingsScreen
import org.pjstatecm.graceride.ui.screens.profile.NotificationSettingsScreen
import org.pjstatecm.graceride.ui.screens.profile.PersonalDetailsScreen
import org.pjstatecm.graceride.ui.screens.profile.SecuritySettingsScreen
import org.pjstatecm.graceride.ui.theme.Spacing
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.navigation.toRoute
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import org.pjstatecm.graceride.notifications.NotificationHelper
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.AuthEvent
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.screens.admin.AdminAuditScreen
import org.pjstatecm.graceride.ui.screens.admin.AdminConsoleScreen
import org.pjstatecm.graceride.ui.screens.admin.AdminMembersScreen
import org.pjstatecm.graceride.ui.screens.admin.AdminRolesScreen
import org.pjstatecm.graceride.ui.screens.admin.AdminSettingsScreen
import org.pjstatecm.graceride.ui.screens.announcements.AnnouncementsScreen
import org.pjstatecm.graceride.ui.screens.costs.CostsScreen
import org.pjstatecm.graceride.ui.screens.events.EventDetailScreen
import org.pjstatecm.graceride.ui.screens.events.EventFormScreen
import org.pjstatecm.graceride.ui.screens.events.EventsScreen
import org.pjstatecm.graceride.ui.screens.history.HistoryScreen
import org.pjstatecm.graceride.ui.screens.home.HomeScreen
import org.pjstatecm.graceride.ui.screens.login.LoginScreen
import org.pjstatecm.graceride.ui.screens.login.PendingScreen
import org.pjstatecm.graceride.ui.screens.login.TermsScreen
import org.pjstatecm.graceride.ui.screens.login.UnlockScreen
import org.pjstatecm.graceride.ui.screens.notifications.NotificationsScreen
import org.pjstatecm.graceride.ui.screens.profile.DriverSettingsScreen
import org.pjstatecm.graceride.ui.screens.profile.FamilyScreen
import org.pjstatecm.graceride.ui.screens.profile.GrokBotStudioScreen
import org.pjstatecm.graceride.ui.screens.profile.ProfileScreen
import org.pjstatecm.graceride.ui.screens.profile.ProxyRidersScreen
import org.pjstatecm.graceride.ui.screens.profile.SavedAddressesScreen
import org.pjstatecm.graceride.ui.screens.public.CarePassScreen
import org.pjstatecm.graceride.ui.screens.public.InviteScreen
import org.pjstatecm.graceride.ui.screens.public.LicensesScreen
import org.pjstatecm.graceride.ui.screens.public.VisitScreen
import org.pjstatecm.graceride.ui.screens.rides.RideDetailScreen
import org.pjstatecm.graceride.ui.screens.rides.RideNewScreen
import org.pjstatecm.graceride.ui.screens.rides.RidesScreen
import org.pjstatecm.graceride.ui.screens.reimbursements.AdminReimbursementsScreen
import org.pjstatecm.graceride.ui.screens.reimbursements.ReimbursementDetailScreen
import org.pjstatecm.graceride.ui.screens.reimbursements.ReimbursementFormScreen
import org.pjstatecm.graceride.ui.screens.reimbursements.ReimbursementsScreen
import org.pjstatecm.graceride.ui.screens.settlements.SettlementDetailScreen
import org.pjstatecm.graceride.ui.screens.settlements.SettlementFormScreen
import org.pjstatecm.graceride.ui.screens.settlements.SettlementsScreen
import org.pjstatecm.graceride.ui.screens.help.HelpDetailScreen
import org.pjstatecm.graceride.ui.screens.help.HelpGuideDetailScreen
import org.pjstatecm.graceride.ui.screens.help.HelpGuideEditorScreen
import org.pjstatecm.graceride.ui.screens.help.HelpScreen
import org.pjstatecm.graceride.ui.screens.reports.AdminReportsScreen
import org.pjstatecm.graceride.ui.screens.reports.ReportScreen
import org.pjstatecm.graceride.ui.screens.traffic.PlanTransportScreen
import org.pjstatecm.graceride.ui.screens.traffic.TrafficHubScreen

val LocalBottomBarClearance = compositionLocalOf { 0.dp }

@Composable
fun GraceRideAppRoot(
    container: AppContainer,
    pendingDeepLink: MutableStateFlow<String?>? = null,
) {
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val biometricEnrollment by container.biometric.enrollment.collectAsStateWithLifecycle()
    val biometricAppUnlocked by container.biometric.appUnlocked.collectAsStateWithLifecycle()
    val pendingBiometricOptIn by container.biometric.pendingOptInUserId.collectAsStateWithLifecycle()
    val isOnline by container.networkMonitor.isOnline.collectAsStateWithLifecycle()
    val currentLang by container.langFlow.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val view = LocalView.current
    val scope = rememberCoroutineScope()
    var optInBusy by remember { mutableStateOf(false) }
    var optInError by remember { mutableStateOf<String?>(null) }
    val notifPermLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { /* Permission result handled gracefully by NotificationHelper. */ }

    LaunchedEffect(Unit) { container.auth.refresh() }

    val syncKey = (authState as? AuthState.SignedIn)?.let {
        Triple(it.me.id, it.me.termsAccepted, it.me.can("drive"))
    }
    LaunchedEffect(syncKey, biometricEnrollment?.userId, biometricAppUnlocked) {
        val signedIn = authState as? AuthState.SignedIn ?: run {
            kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
                runCatching {
                    androidx.core.content.pm.ShortcutManagerCompat.removeDynamicShortcuts(context, listOf("drive_hub"))
                }
            }
            return@LaunchedEffect
        }
        if (!signedIn.me.termsAccepted) return@LaunchedEffect
        if (biometricEnrollment?.userId == signedIn.me.id && !biometricAppUnlocked) {
            return@LaunchedEffect
        }
        kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
            if (signedIn.me.can("drive")) {
                val intent = android.content.Intent(
                    android.content.Intent.ACTION_VIEW,
                    "graceride://open/drive".toUri(),
                    context,
                    org.pjstatecm.graceride.MainActivity::class.java,
                )
                val shortcut = androidx.core.content.pm.ShortcutInfoCompat.Builder(context, "drive_hub")
                    .setShortLabel(context.getString(R.string.shortcut_drive_short))
                    .setLongLabel(context.getString(R.string.shortcut_drive_long))
                    .setIcon(androidx.core.graphics.drawable.IconCompat.createWithResource(context, R.drawable.ic_launcher_monochrome))
                    .setIntent(intent)
                    .build()
                runCatching { androidx.core.content.pm.ShortcutManagerCompat.pushDynamicShortcut(context, shortcut) }
            } else {
                runCatching {
                    androidx.core.content.pm.ShortcutManagerCompat.removeDynamicShortcuts(context, listOf("drive_hub"))
                }
            }
            runSuspendCatching {
                val prefs = container.community.notificationPrefs()
                org.pjstatecm.graceride.notifications.RealtimeStreamClient.writeCachedPrefs(container.secureStore, prefs)
            }
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU && !NotificationHelper.hasPostPermission(context)) {
            val asked = kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
                val already = container.secureStore.getString("notif_perm_asked") == "1"
                if (!already) container.secureStore.putString("notif_perm_asked", "1")
                already
            }
            if (!asked) {
                notifPermLauncher.launch(Manifest.permission.POST_NOTIFICATIONS)
            }
        }
    }

    LaunchedEffect(syncKey, isOnline, currentLang, biometricEnrollment?.userId, biometricAppUnlocked) {
        val signedIn = authState as? AuthState.SignedIn ?: return@LaunchedEffect
        if (!signedIn.me.termsAccepted ||
            !isOnline ||
            !container.isLangPendingSync() ||
            container.biometric.requiresAppGate(signedIn.me.id)
        ) {
            return@LaunchedEffect
        }
        kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
            if (signedIn.me.lang != currentLang) {
                runSuspendCatching {
                    container.community.updateProfile(
                        org.pjstatecm.graceride.data.api.ProfileUpdateReq(lang = currentLang),
                    )
                    container.markLangPendingSync(false)
                    container.auth.refresh()
                }
            } else {
                container.markLangPendingSync(false)
            }
        }
    }

    val animatorsOn = animatorsEnabled()
    var splashMinElapsed by remember { mutableStateOf(container.launchSplashShown || !animatorsOn) }
    var splashMounted by remember {
        mutableStateOf((!container.launchSplashShown && animatorsOn) || authState is AuthState.Loading)
    }

    LaunchedEffect(Unit) {
        if (!splashMounted && !container.launchSplashShown) {
            container.launchSplashShown = true
        }
        if (!splashMinElapsed) {
            delay(1350L)
            splashMinElapsed = true
        }
    }

    Box(Modifier.fillMaxSize()) {
        if (authState !is AuthState.Loading) {
            val signedInMe = (authState as? AuthState.SignedIn)?.me
            if (signedInMe != null &&
                biometricEnrollment?.userId == signedInMe.id &&
                !biometricAppUnlocked
            ) {
                BiometricGateScreen(container, signedInMe)
            } else {
            val activeUserId = container.auth.userIdOf(authState)
            androidx.compose.runtime.key(activeUserId) {
                val navController = rememberNavController()
                val backStack by navController.currentBackStackEntryAsState()
                val currentRoute = backStack?.destination?.route
                val deepLink by (
                    pendingDeepLink?.collectAsStateWithLifecycle()
                        ?: remember { mutableStateOf<String?>(null) }
                    )

                LaunchedEffect(authState, deepLink, currentRoute) {
                    val rawLink = deepLink ?: return@LaunchedEffect
                    val publicRoute = NotifLinkRouter.resolvePublic(rawLink)
                    if (publicRoute != null && currentRoute != null) {
                        navController.navigate(publicRoute) {
                            launchSingleTop = true
                        }
                        pendingDeepLink?.value = null
                        return@LaunchedEffect
                    }
                    val signedIn = authState as? AuthState.SignedIn ?: return@LaunchedEffect
                    if (!signedIn.me.termsAccepted) return@LaunchedEffect
                    val onAuthRoute = currentRoute == null ||
                        Routes.authClasses.any { currentRoute == it || currentRoute.startsWith("$it/") }
                    if (onAuthRoute) {
                        return@LaunchedEffect
                    }
                    when (val dest = NotifLinkRouter.resolveFor(rawLink, signedIn.me)) {
                        is NotifDestination.Native -> {
                            if (dest.route in Routes.tabs) {
                                navController.navigateRoute(dest.route) {
                                    popUpTo<AppRoute.Home> { saveState = false }
                                    launchSingleTop = true
                                    restoreState = false
                                }
                            } else {
                                navController.navigateRoute(dest.route) {
                                    launchSingleTop = true
                                }
                            }
                        }
                        is NotifDestination.Web -> openInWebApp(context, container.webUrl(dest.path))
                        NotifDestination.None -> {}
                    }
                    pendingDeepLink?.value = null
                }

            // Session state drives the global navigation, mirroring RequireAuth on the web.
            LaunchedEffect(authState) {
                // Wait until the NavHost has installed its graph: on activity recreation
                // this effect can fire before the first navigation destination exists.
                val route = navController.currentBackStackEntryFlow
                    .map { it.destination.route }
                    .filterNotNull()
                    .first()
                val onPublicRoute = Routes.publicClasses.any { matchesRouteClass(route, it) }
                val target: AppRoute? = when (authState) {
                    is AuthState.SignedOut -> if (onPublicRoute) null else AppRoute.Login
                    is AuthState.Locked -> if (onPublicRoute) null else AppRoute.Unlock
                    is AuthState.Pending -> if (onPublicRoute) null else AppRoute.Pending
                    is AuthState.SignedIn -> {
                        val signedIn = authState as AuthState.SignedIn
                        if (!signedIn.me.termsAccepted) {
                            AppRoute.Terms
                        } else if (Routes.authClasses.any { route == it || route.startsWith("$it/") }) {
                            AppRoute.Home
                        } else {
                            null
                        }
                    }
                    else -> null
                }
                val targetClass = target?.let { it::class.qualifiedName }
                if (target != null && targetClass != null && route != targetClass && !route.startsWith("$targetClass/")) {
                    navController.navigate(target) {
                        popUpTo(0) { inclusive = true }
                        launchSingleTop = true
                    }
                }
            }

            val start: AppRoute = remember {
                when (authState) {
                    is AuthState.SignedIn ->
                        if ((authState as AuthState.SignedIn).me.termsAccepted) AppRoute.Home else AppRoute.Terms
                    is AuthState.Locked -> AppRoute.Unlock
                    is AuthState.Pending -> AppRoute.Pending
                    else -> AppRoute.Login
                }
            }

            BoxWithConstraints(Modifier.fillMaxSize()) {
                val isTabRoute = currentRoute != null && Routes.tabClasses.any {
                    matchesRouteClass(currentRoute, it)
                }
                val adaptiveInfo = currentWindowAdaptiveInfo()
                val calculatedSuiteType = when {
                    adaptiveInfo.windowPosture.isTabletop -> NavigationSuiteType.NavigationBar
                    maxWidth >= 600.dp || maxHeight < 480.dp -> NavigationSuiteType.NavigationRail
                    else -> NavigationSuiteType.NavigationBar
                }
                val useFloatingBottomPill = isTabRoute && calculatedSuiteType == NavigationSuiteType.NavigationBar
            val layoutType = if (!isTabRoute || useFloatingBottomPill) {
                NavigationSuiteType.None
            } else {
                calculatedSuiteType
            }
            val isHomeSelected = matchesRouteClass(currentRoute, AppRoute.Home::class.qualifiedName)
            val isEventsSelected = matchesRouteClass(currentRoute, AppRoute.Events::class.qualifiedName)
            val isTrafficSelected = matchesRouteClass(currentRoute, AppRoute.TrafficHub::class.qualifiedName)
            val isProfileSelected = matchesRouteClass(currentRoute, AppRoute.Profile::class.qualifiedName)
            val canEventsTab = (authState as? AuthState.SignedIn)?.me?.can("events") != false
            val canTrafficTab = (authState as? AuthState.SignedIn)?.me?.let(Routes::canTrafficHub) == true
            val swipeTabs = remember(canEventsTab, canTrafficTab) {
                buildList<AppRoute> {
                    add(AppRoute.Home)
                    if (canEventsTab) add(AppRoute.Events)
                    if (canTrafficTab) add(AppRoute.TrafficHub())
                    add(AppRoute.Profile)
                }
            }
            val navItemColors = NavigationSuiteDefaults.itemColors(
                navigationBarItemColors = NavigationBarItemDefaults.colors(
                    selectedIconColor = MaterialTheme.colorScheme.primary,
                    selectedTextColor = MaterialTheme.colorScheme.primary,
                    indicatorColor = MaterialTheme.colorScheme.primaryContainer,
                    unselectedIconColor = MaterialTheme.colorScheme.onSurfaceVariant,
                    unselectedTextColor = MaterialTheme.colorScheme.onSurfaceVariant,
                ),
                navigationRailItemColors = NavigationRailItemDefaults.colors(
                    selectedIconColor = MaterialTheme.colorScheme.primary,
                    selectedTextColor = MaterialTheme.colorScheme.primary,
                    indicatorColor = MaterialTheme.colorScheme.primaryContainer,
                    unselectedIconColor = MaterialTheme.colorScheme.onSurfaceVariant,
                    unselectedTextColor = MaterialTheme.colorScheme.onSurfaceVariant,
                ),
                navigationDrawerItemColors = NavigationDrawerItemDefaults.colors(
                    selectedIconColor = MaterialTheme.colorScheme.primary,
                    selectedTextColor = MaterialTheme.colorScheme.primary,
                    selectedContainerColor = MaterialTheme.colorScheme.primaryContainer,
                    unselectedIconColor = MaterialTheme.colorScheme.onSurfaceVariant,
                    unselectedTextColor = MaterialTheme.colorScheme.onSurfaceVariant,
                ),
            )
            val suiteColors = NavigationSuiteDefaults.colors(
                navigationBarContainerColor = MaterialTheme.colorScheme.background,
                navigationBarContentColor = MaterialTheme.colorScheme.onSurfaceVariant,
                navigationRailContainerColor = MaterialTheme.colorScheme.background,
                navigationRailContentColor = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            val navHomeLabel = stringResource(R.string.nav_home)
            val navEventsLabel = stringResource(R.string.nav_events)
            val navTrafficLabel = stringResource(R.string.nav_traffic)
            val navProfileLabel = stringResource(R.string.nav_profile)

            NavigationSuiteScaffold(
                layoutType = layoutType,
                navigationSuiteColors = suiteColors,
                navigationSuiteItems = {
                    item(
                        selected = isHomeSelected,
                        onClick = {
                            UiHaptics.tick(view)
                            navController.navigateTab(AppRoute.Home)
                        },
                        modifier = Modifier.semantics { contentDescription = navHomeLabel },
                        icon = {
                            Icon(
                                if (isHomeSelected) Icons.Filled.Home else Icons.Outlined.Home,
                                contentDescription = null,
                            )
                        },
                        label = { Text(navHomeLabel) },
                        colors = navItemColors,
                    )
                    if (canEventsTab) {
                        item(
                            selected = isEventsSelected,
                            onClick = {
                                UiHaptics.tick(view)
                                navController.navigateTab(AppRoute.Events)
                            },
                            modifier = Modifier.semantics { contentDescription = navEventsLabel },
                            icon = {
                                Icon(
                                    if (isEventsSelected) Icons.Filled.Event else Icons.Outlined.Event,
                                    contentDescription = null,
                                )
                            },
                            label = { Text(navEventsLabel) },
                            colors = navItemColors,
                        )
                    }
                    if (canTrafficTab) {
                        item(
                            selected = isTrafficSelected,
                            onClick = {
                                UiHaptics.tick(view)
                                navController.navigateTab(AppRoute.TrafficHub())
                            },
                            modifier = Modifier.semantics { contentDescription = navTrafficLabel },
                            icon = {
                                Icon(
                                    if (isTrafficSelected) Icons.AutoMirrored.Filled.AltRoute else Icons.AutoMirrored.Outlined.AltRoute,
                                    contentDescription = null,
                                )
                            },
                            label = { Text(navTrafficLabel) },
                            colors = navItemColors,
                        )
                    }
                    item(
                        selected = isProfileSelected,
                        onClick = {
                            UiHaptics.tick(view)
                            navController.navigateTab(AppRoute.Profile)
                        },
                        modifier = Modifier.semantics { contentDescription = navProfileLabel },
                        icon = {
                            Box(
                                modifier = if (isProfileSelected && layoutType != NavigationSuiteType.None) {
                                    Modifier
                                        .border(2.dp, MaterialTheme.colorScheme.primary, CircleShape)
                                        .padding(2.dp)
                                } else {
                                    Modifier
                                },
                                contentAlignment = Alignment.Center,
                            ) {
                                MemberAvatar(
                                    name = signedInMe?.name ?: "",
                                    photoUrl = signedInMe?.photoFileId?.let(container::fileUrl),
                                    avatarColor = signedInMe?.avatarColor,
                                    avatarIcon = signedInMe?.avatarIcon,
                                    size = 24.dp,
                                )
                            }
                        },
                        label = { Text(navProfileLabel) },
                        colors = navItemColors,
                    )
                },
            ) {
                CompositionLocalProvider(
                    LocalBottomBarClearance provides if (useFloatingBottomPill) 80.dp else 0.dp,
                ) {
                    Scaffold(
                        contentWindowInsets = WindowInsets.systemBars.union(WindowInsets.displayCutout),
                    ) { padding ->
                        Box(
                            Modifier
                                .fillMaxSize()
                                .padding(padding)
                                .consumeWindowInsets(padding),
                        ) {
                            Row(Modifier.fillMaxSize()) {
                                if (layoutType == NavigationSuiteType.NavigationRail) {
                                    VerticalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                }
                                Column(Modifier.weight(1f).fillMaxHeight()) {
                        AnimatedVisibility(
                            visible = !isOnline && authState is AuthState.SignedIn,
                            enter = slideInVertically(tween(180)) { -it } + fadeIn(tween(160)),
                            exit = slideOutVertically(tween(160)) { -it } + fadeOut(tween(140)),
                        ) {
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .background(MaterialTheme.colorScheme.errorContainer)
                                    .padding(horizontal = Spacing.md, vertical = Spacing.xs),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                Icon(
                                    Icons.Outlined.CloudOff,
                                    contentDescription = null,
                                    tint = MaterialTheme.colorScheme.onErrorContainer,
                                    modifier = Modifier.size(16.dp),
                                )
                                Text(
                                    stringResource(R.string.network_offline_live),
                                    style = MaterialTheme.typography.labelMedium,
                                    color = MaterialTheme.colorScheme.onErrorContainer,
                                )
                            }
                        }
                        val swipeThresholdPx = with(LocalDensity.current) { 72.dp.toPx() }
                        val isTrafficHub = matchesRouteClass(currentRoute, AppRoute.TrafficHub::class.qualifiedName)
                        Box(
                            Modifier
                                .weight(1f)
                                .pointerInput(currentRoute, swipeTabs, isTabRoute, isTrafficHub) {
                                    if (!isTabRoute || isTrafficHub) return@pointerInput
                                    var totalDrag = 0f
                                    var switched = false
                                    detectHorizontalDragGestures(
                                        onDragStart = {
                                            totalDrag = 0f
                                            switched = false
                                        },
                                        onHorizontalDrag = { _, dragAmount ->
                                            if (switched) return@detectHorizontalDragGestures
                                            totalDrag += dragAmount
                                            val idx = swipeTabs.indexOfFirst {
                                                matchesRouteClass(currentRoute, it::class.qualifiedName)
                                            }
                                            if (idx != -1 && abs(totalDrag) >= swipeThresholdPx) {
                                                val targetIdx = if (totalDrag < 0f) idx + 1 else idx - 1
                                                if (targetIdx in swipeTabs.indices) {
                                                    switched = true
                                                    UiHaptics.tick(view)
                                                    navController.navigateTab(swipeTabs[targetIdx])
                                                }
                                            }
                                        },
                                    )
                                },
                        ) {
                            NavHost(
                                navController = navController,
                                startDestination = start,
                                modifier = Modifier.fillMaxSize(),
                                enterTransition = {
                                    val fromRoute = initialState.destination.route
                                    val toRoute = targetState.destination.route
                                    val fromIdx = tabOrderIndex(fromRoute)
                                    val toIdx = tabOrderIndex(toRoute)
                                    val anim = animatorsEnabled()
                                    when {
                                        isAuthRoute(fromRoute) || isAuthRoute(toRoute) ->
                                            fadeIn(tween(if (anim) 180 else 80))
                                        fromIdx != -1 && toIdx != -1 && fromIdx != toIdx && anim -> {
                                            val dir = if (toIdx > fromIdx) 1 else -1
                                            slideInHorizontally(tween(260, easing = GraceRideSpringEasing)) { it * dir } +
                                                fadeIn(tween(180))
                                        }
                                        fromIdx != -1 && toIdx != -1 -> fadeIn(tween(160))
                                        anim -> slideInHorizontally(tween(250, easing = GraceRideSpringEasing)) { it / 4 } +
                                            fadeIn(tween(180))
                                        else -> fadeIn(tween(120))
                                    }
                                },
                                exitTransition = {
                                    val fromRoute = initialState.destination.route
                                    val toRoute = targetState.destination.route
                                    val fromIdx = tabOrderIndex(fromRoute)
                                    val toIdx = tabOrderIndex(toRoute)
                                    val anim = animatorsEnabled()
                                    when {
                                        isAuthRoute(fromRoute) || isAuthRoute(toRoute) ->
                                            fadeOut(tween(if (anim) 150 else 80))
                                        fromIdx != -1 && toIdx != -1 && fromIdx != toIdx && anim -> {
                                            val dir = if (toIdx > fromIdx) -1 else 1
                                            slideOutHorizontally(tween(240, easing = GraceRideSpringEasing)) { (it / 3) * dir } +
                                                fadeOut(tween(150))
                                        }
                                        fromIdx != -1 && toIdx != -1 -> fadeOut(tween(140))
                                        anim -> slideOutHorizontally(tween(220, easing = GraceRideSpringEasing)) { -it / 6 } +
                                            fadeOut(tween(160))
                                        else -> fadeOut(tween(100))
                                    }
                                },
                                popEnterTransition = {
                                    val fromRoute = initialState.destination.route
                                    val toRoute = targetState.destination.route
                                    val fromIdx = tabOrderIndex(fromRoute)
                                    val toIdx = tabOrderIndex(toRoute)
                                    val anim = animatorsEnabled()
                                    when {
                                        isAuthRoute(fromRoute) || isAuthRoute(toRoute) ->
                                            fadeIn(tween(if (anim) 180 else 80))
                                        fromIdx != -1 && toIdx != -1 && fromIdx != toIdx && anim -> {
                                            val dir = if (toIdx > fromIdx) 1 else -1
                                            slideInHorizontally(tween(260, easing = GraceRideSpringEasing)) { it * dir } +
                                                fadeIn(tween(180))
                                        }
                                        fromIdx != -1 && toIdx != -1 -> fadeIn(tween(160))
                                        anim -> slideInHorizontally(tween(240, easing = GraceRideSpringEasing)) { -it / 5 } +
                                            fadeIn(tween(180))
                                        else -> fadeIn(tween(120))
                                    }
                                },
                                popExitTransition = {
                                    val fromRoute = initialState.destination.route
                                    val toRoute = targetState.destination.route
                                    val fromIdx = tabOrderIndex(fromRoute)
                                    val toIdx = tabOrderIndex(toRoute)
                                    val anim = animatorsEnabled()
                                    when {
                                        isAuthRoute(fromRoute) || isAuthRoute(toRoute) ->
                                            fadeOut(tween(if (anim) 150 else 80))
                                        fromIdx != -1 && toIdx != -1 && fromIdx != toIdx && anim -> {
                                            val dir = if (toIdx > fromIdx) -1 else 1
                                            slideOutHorizontally(tween(240, easing = GraceRideSpringEasing)) { (it / 3) * dir } +
                                                fadeOut(tween(150))
                                        }
                                        fromIdx != -1 && toIdx != -1 -> fadeOut(tween(140))
                                        anim -> slideOutHorizontally(tween(240, easing = GraceRideSpringEasing)) { it / 4 } +
                                            fadeOut(tween(160))
                                        else -> fadeOut(tween(100))
                                    }
                                },
                            ) {
                                composable<AppRoute.Login> {
                                    LoginScreen(
                                        container = container,
                                        onOpenTerms = { navController.navigate(AppRoute.TermsPublic) },
                                        onOpenLicenses = { navController.navigate(AppRoute.Licenses) },
                                    )
                                }
                                composable<AppRoute.Unlock> { UnlockScreen(container) }
                                composable<AppRoute.Pending> { PendingScreen(container) }
                                composable<AppRoute.Terms> { TermsScreen(container) }
                                composable<AppRoute.TermsPublic> {
                                    TermsScreen(
                                        container = container,
                                        readOnly = true,
                                        onBack = { navController.popBackStack() },
                                        onOpenLicenses = { navController.navigate(AppRoute.Licenses) },
                                    )
                                }
                                composable<AppRoute.Licenses> {
                                    LicensesScreen(
                                        container = container,
                                        onBack = { navController.popBackStack() },
                                    )
                                }
                                composable<AppRoute.Invite> { entry ->
                                    val args = entry.toRoute<AppRoute.Invite>()
                                    InviteScreen(
                                        container = container,
                                        token = args.token,
                                        onBackToLogin = {
                                            navController.navigate(AppRoute.Login) {
                                                popUpTo(0) { inclusive = true }
                                                launchSingleTop = true
                                            }
                                        },
                                        onFinished = {
                                            if (authState !is AuthState.SignedIn) {
                                                navController.navigate(AppRoute.Login) {
                                                    popUpTo(0) { inclusive = true }
                                                    launchSingleTop = true
                                                }
                                            }
                                        },
                                    )
                                }
                                composable<AppRoute.Visit> { entry ->
                                    val args = entry.toRoute<AppRoute.Visit>()
                                    VisitScreen(
                                        container = container,
                                        token = args.token,
                                        onBack = {
                                            if (!navController.popBackStack()) {
                                                val fallback = if (authState is AuthState.SignedIn) AppRoute.Home else AppRoute.Login
                                                navController.navigate(fallback) { launchSingleTop = true }
                                            }
                                        },
                                    )
                                }
                                composable<AppRoute.CarePass> { entry ->
                                    val args = entry.toRoute<AppRoute.CarePass>()
                                    CarePassScreen(
                                        container = container,
                                        token = args.token,
                                        onBack = {
                                            if (!navController.popBackStack()) {
                                                val fallback = if (authState is AuthState.SignedIn) AppRoute.Home else AppRoute.Login
                                                navController.navigate(fallback) { launchSingleTop = true }
                                            }
                                        },
                                    )
                                }
                                composable<AppRoute.Home> { HomeScreen(container, navController) }
                                composable<AppRoute.Rides> { RidesScreen(container, navController) }
                                composable<AppRoute.RideDetail> { entry ->
                                    val args = entry.toRoute<AppRoute.RideDetail>()
                                    RideDetailScreen(
                                        container = container,
                                        nav = navController,
                                        id = args.id,
                                        justBooked = args.justBooked,
                                    )
                                }
                                composable<AppRoute.RideNew> { entry ->
                                    val args = entry.toRoute<AppRoute.RideNew>()
                                    RideNewScreen(container, navController, args.eventId, args.proxy)
                                }
                                composable<AppRoute.Events> { EventsScreen(container, navController) }
                                composable<AppRoute.EventNew> { entry ->
                                    val args = entry.toRoute<AppRoute.EventNew>()
                                    EventFormScreen(
                                        container = container,
                                        nav = navController,
                                        eventId = null,
                                        duplicateId = args.duplicateId,
                                    )
                                }
                                composable<AppRoute.EventEdit> { entry ->
                                    val args = entry.toRoute<AppRoute.EventEdit>()
                                    EventFormScreen(
                                        container = container,
                                        nav = navController,
                                        eventId = args.id,
                                        duplicateId = null,
                                    )
                                }
                                composable<AppRoute.EventDetail> { entry ->
                                    val args = entry.toRoute<AppRoute.EventDetail>()
                                    EventDetailScreen(container, navController, args.id, args.checkinCode, args.openCheckin)
                                }
                                composable<AppRoute.Costs> { CostsScreen(container, navController) }
                                composable<AppRoute.History> { HistoryScreen(container, navController) }
                                composable<AppRoute.Profile> { ProfileScreen(container, navController) }
                                composable<AppRoute.PersonalDetails> { PersonalDetailsScreen(container, navController) }
                                composable<AppRoute.AppearanceSettings> { AppearanceSettingsScreen(container, navController) }
                                composable<AppRoute.NotificationSettings> { NotificationSettingsScreen(container, navController) }
                                composable<AppRoute.SecuritySettings> { SecuritySettingsScreen(container, navController) }
                                composable<AppRoute.DriverSettings> { DriverSettingsScreen(container, navController) }
                                composable<AppRoute.ProxyRiders> { ProxyRidersScreen(container, navController) }
                                composable<AppRoute.Family> { FamilyScreen(container, navController) }
                                composable<AppRoute.SavedAddresses> { SavedAddressesScreen(container, navController) }
                                composable<AppRoute.Notifications> { NotificationsScreen(container, navController) }
                                composable<AppRoute.DriveHub> { DriveHubScreen(container, navController) }
                                composable<AppRoute.TrafficHub> { entry ->
                                    val args = entry.toRoute<AppRoute.TrafficHub>()
                                    TrafficHubScreen(
                                        container = container,
                                        nav = navController,
                                        initialTab = args.tab,
                                        initialRidesView = args.view,
                                        initialAreaTab = args.areaTab,
                                        initialRidesStatus = args.status,
                                        initialRidesAttention = args.attention,
                                        initialPlanFilter = args.planFilter,
                                    )
                                }
                                composable<AppRoute.PlanTransport> { entry ->
                                    val args = entry.toRoute<AppRoute.PlanTransport>()
                                    PlanTransportScreen(
                                        container = container,
                                        nav = navController,
                                        eventId = args.eventId,
                                        initialDraftId = args.draftId,
                                    )
                                }
                                composable<AppRoute.Studio> { entry ->
                                    val args = entry.toRoute<AppRoute.Studio>()
                                    GrokBotStudioScreen(
                                        container = container,
                                        nav = navController,
                                        initialShape = args.shape,
                                        initialColor = args.color,
                                        initialFace = args.face,
                                    )
                                }
                                composable<AppRoute.AdminMembers> { entry ->
                                    val args = entry.toRoute<AppRoute.AdminMembers>()
                                    AdminMembersScreen(
                                        container = container,
                                        nav = navController,
                                        initialTab = args.tab,
                                        initialStatus = args.status,
                                    )
                                }
                                composable<AppRoute.Announcements> { entry ->
                                    val args = entry.toRoute<AppRoute.Announcements>()
                                    AnnouncementsScreen(
                                        container = container,
                                        nav = navController,
                                        manage = args.manage,
                                    )
                                }
                                composable<AppRoute.Settlements> {
                                    SettlementsScreen(container = container, nav = navController)
                                }
                                composable<AppRoute.SettlementDetail> { entry ->
                                    val args = entry.toRoute<AppRoute.SettlementDetail>()
                                    SettlementDetailScreen(container = container, nav = navController, id = args.id)
                                }
                                composable<AppRoute.SettlementForm> { entry ->
                                    val args = entry.toRoute<AppRoute.SettlementForm>()
                                    SettlementFormScreen(
                                        container = container,
                                        nav = navController,
                                        settlementId = args.settlementId,
                                        eventId = args.eventId,
                                    )
                                }
                                composable<AppRoute.Reimbursements> {
                                    ReimbursementsScreen(container = container, nav = navController)
                                }
                                composable<AppRoute.ReimbursementDetail> { entry ->
                                    val args = entry.toRoute<AppRoute.ReimbursementDetail>()
                                    ReimbursementDetailScreen(container = container, nav = navController, id = args.id)
                                }
                                composable<AppRoute.ReimbursementForm> { entry ->
                                    val args = entry.toRoute<AppRoute.ReimbursementForm>()
                                    ReimbursementFormScreen(container = container, nav = navController, editId = args.editId)
                                }
                                composable<AppRoute.AdminReimbursements> { entry ->
                                    val args = entry.toRoute<AppRoute.AdminReimbursements>()
                                    AdminReimbursementsScreen(
                                        container = container,
                                        nav = navController,
                                        initialTab = args.tab,
                                    )
                                }
                                composable<AppRoute.Help> { entry ->
                                    val args = entry.toRoute<AppRoute.Help>()
                                    HelpScreen(
                                        container = container,
                                        nav = navController,
                                        rideId = args.rideId,
                                        initialTab = args.tab,
                                    )
                                }
                                composable<AppRoute.HelpDetail> { entry ->
                                    val args = entry.toRoute<AppRoute.HelpDetail>()
                                    HelpDetailScreen(container = container, nav = navController, id = args.id)
                                }
                                composable<AppRoute.HelpGuideDetail> { entry ->
                                    val args = entry.toRoute<AppRoute.HelpGuideDetail>()
                                    HelpGuideDetailScreen(container = container, nav = navController, key = args.key)
                                }
                                composable<AppRoute.HelpGuideEditor> { entry ->
                                    val args = entry.toRoute<AppRoute.HelpGuideEditor>()
                                    HelpGuideEditorScreen(container = container, nav = navController, key = args.key)
                                }
                                composable<AppRoute.Report> {
                                    ReportScreen(container = container, nav = navController)
                                }
                                composable<AppRoute.AdminReports> { entry ->
                                    val args = entry.toRoute<AppRoute.AdminReports>()
                                    AdminReportsScreen(container = container, nav = navController, initialTab = args.tab)
                                }
                                composable<AppRoute.AdminConsole> {
                                    AdminConsoleScreen(container = container, nav = navController)
                                }
                                composable<AppRoute.AdminRoles> {
                                    AdminRolesScreen(container = container, nav = navController)
                                }
                                composable<AppRoute.AdminAudit> {
                                    AdminAuditScreen(container = container, nav = navController)
                                }
                                composable<AppRoute.AdminSettings> { entry ->
                                    val args = entry.toRoute<AppRoute.AdminSettings>()
                                    AdminSettingsScreen(container = container, nav = navController, initialTab = args.tab)
                                }
                            }
                        }
                                }
                            }
                            AnimatedVisibility(
                                visible = useFloatingBottomPill,
                                modifier = Modifier.align(Alignment.BottomCenter),
                                enter = if (animatorsEnabled()) {
                                    slideInVertically(spring(dampingRatio = 0.82f, stiffness = 420f)) { it } + fadeIn(tween(180))
                                } else {
                                    fadeIn(tween(100))
                                },
                                exit = if (animatorsEnabled()) {
                                    slideOutVertically(tween(160)) { it } + fadeOut(tween(140))
                                } else {
                                    fadeOut(tween(80))
                                },
                            ) {
                                Box(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .background(
                                            Brush.verticalGradient(
                                                listOf(
                                                    Color.Transparent,
                                                    MaterialTheme.colorScheme.background.copy(alpha = 0.72f),
                                                    MaterialTheme.colorScheme.background.copy(alpha = 0.94f),
                                                ),
                                            ),
                                        ),
                                    contentAlignment = Alignment.BottomCenter,
                                ) {
                                    FloatingGlassBottomBar(
                                        isHomeSelected = isHomeSelected,
                                        isEventsSelected = isEventsSelected,
                                        isTrafficSelected = isTrafficSelected,
                                        isProfileSelected = isProfileSelected,
                                        canEvents = canEventsTab,
                                        canTraffic = canTrafficTab,
                                        signedInMe = signedInMe,
                                        fileUrlProvider = container::fileUrl,
                                        onSelectTab = { route ->
                                            UiHaptics.tick(view)
                                            navController.navigateTab(route)
                                        },
                                    )
                                }
                            }
                        }
                    }
                }
            }
            }
            }
            if (signedInMe != null &&
                signedInMe.termsAccepted &&
                pendingBiometricOptIn == signedInMe.id &&
                container.biometric.shouldOfferOptIn(signedInMe.id, context)
            ) {
                AlertDialog(
                    onDismissRequest = {
                        if (!optInBusy) {
                            optInError = null
                            container.biometric.dismissOptIn(signedInMe.id)
                        }
                    },
                    title = { Text(stringResource(R.string.biometric_opt_in_title)) },
                    text = {
                        Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            Text(stringResource(R.string.biometric_opt_in_body))
                            optInError?.let { err ->
                                Text(
                                    err,
                                    color = MaterialTheme.colorScheme.error,
                                    style = MaterialTheme.typography.bodySmall,
                                )
                            }
                        }
                    },
                    confirmButton = {
                        TextButton(
                            enabled = !optInBusy,
                            onClick = {
                                optInBusy = true
                                optInError = null
                                scope.launch {
                                    try {
                                        when (
                                            val res = container.auth.enrollBiometric(
                                                context = context,
                                                title = context.getString(R.string.biometric_prompt_enroll_title),
                                                subtitle = context.getString(R.string.biometric_prompt_enroll_subtitle),
                                                cancelText = context.getString(R.string.common_cancel),
                                            )
                                        ) {
                                            is BiometricSignResult.Success -> optInError = null
                                            BiometricSignResult.Cancelled -> {}
                                            BiometricSignResult.KeyInvalidated -> {
                                                optInError = context.getString(R.string.biometric_failed)
                                            }
                                            is BiometricSignResult.Error -> {
                                                optInError = res.message
                                            }
                                        }
                                    } catch (ce: CancellationException) {
                                        throw ce
                                    } catch (e: ApiException) {
                                        optInError = e.message
                                    } catch (e: Exception) {
                                        optInError = e.message ?: context.getString(R.string.biometric_failed)
                                    } finally {
                                        optInBusy = false
                                    }
                                }
                            },
                        ) {
                            if (optInBusy) {
                                InlineButtonSpinner()
                            } else {
                                Text(stringResource(R.string.biometric_opt_in_enable))
                            }
                        }
                    },
                    dismissButton = {
                        TextButton(
                            enabled = !optInBusy,
                            onClick = {
                                optInError = null
                                container.biometric.dismissOptIn(signedInMe.id)
                            },
                        ) {
                            Text(stringResource(R.string.biometric_opt_in_not_now))
                        }
                    },
                )
            }
            }
        }

        if (splashMounted) {
            GraceRideLandingSplash(
                visible = authState is AuthState.Loading || !splashMinElapsed,
                onFinished = {
                    container.launchSplashShown = true
                    splashMounted = false
                },
            )
        }
    }
}

private fun matchesRouteClass(currentRoute: String?, qualifiedName: String?): Boolean {
    if (currentRoute == null || qualifiedName == null) return false
    return currentRoute == qualifiedName ||
        currentRoute.startsWith("$qualifiedName/") ||
        currentRoute.startsWith("$qualifiedName?")
}

private fun isAuthRoute(route: String?): Boolean =
    route != null && Routes.authClasses.any { matchesRouteClass(route, it) }

private fun tabOrderIndex(route: String?): Int = when {
    matchesRouteClass(route, AppRoute.Home::class.qualifiedName) -> 0
    matchesRouteClass(route, AppRoute.Events::class.qualifiedName) -> 1
    matchesRouteClass(route, AppRoute.TrafficHub::class.qualifiedName) -> 2
    matchesRouteClass(route, AppRoute.Profile::class.qualifiedName) -> 3
    else -> -1
}

private data class FloatingNavTab(
    val route: AppRoute,
    val label: String,
    val selected: Boolean,
    val filledIcon: ImageVector?,
    val outlinedIcon: ImageVector?,
    val isProfile: Boolean = false,
)

@Composable
private fun FloatingGlassBottomBar(
    isHomeSelected: Boolean,
    isEventsSelected: Boolean,
    isTrafficSelected: Boolean,
    isProfileSelected: Boolean,
    canEvents: Boolean,
    canTraffic: Boolean,
    signedInMe: Me?,
    fileUrlProvider: (Long) -> String,
    onSelectTab: (AppRoute) -> Unit,
) {
    val homeLabel = stringResource(R.string.nav_home)
    val eventsLabel = stringResource(R.string.nav_events)
    val trafficLabel = stringResource(R.string.nav_traffic)
    val profileLabel = stringResource(R.string.nav_profile)
    val tabs = remember(
        isHomeSelected,
        isEventsSelected,
        isTrafficSelected,
        isProfileSelected,
        canEvents,
        canTraffic,
        homeLabel,
        eventsLabel,
        trafficLabel,
        profileLabel,
    ) {
        buildList {
            add(FloatingNavTab(AppRoute.Home, homeLabel, isHomeSelected, Icons.Filled.Home, Icons.Outlined.Home))
            if (canEvents) {
                add(FloatingNavTab(AppRoute.Events, eventsLabel, isEventsSelected, Icons.Filled.Event, Icons.Outlined.Event))
            }
            if (canTraffic) {
                add(FloatingNavTab(AppRoute.TrafficHub(), trafficLabel, isTrafficSelected, Icons.AutoMirrored.Filled.AltRoute, Icons.AutoMirrored.Outlined.AltRoute))
            }
            add(FloatingNavTab(AppRoute.Profile, profileLabel, isProfileSelected, null, null, isProfile = true))
        }
    }
    val selectedIndex = tabs.indexOfFirst { it.selected }
    val isDark = MaterialTheme.colorScheme.background.luminance() < 0.5f

    val capsuleGradient = if (isDark) {
        Brush.verticalGradient(listOf(Color(0xEB22252C), Color(0xF514161B)))
    } else {
        Brush.verticalGradient(listOf(Color(0xF2FFFFFF), Color(0xECF3F5F9)))
    }
    val specularBorder = if (isDark) {
        Brush.verticalGradient(listOf(Color.White.copy(alpha = 0.28f), Color.White.copy(alpha = 0.10f)))
    } else {
        Brush.verticalGradient(listOf(Color.White.copy(alpha = 0.96f), Color(0xFF0F172A).copy(alpha = 0.12f)))
    }
    val pillFill = if (isDark) Color.White.copy(alpha = 0.16f) else Color(0xFF0F172A).copy(alpha = 0.08f)
    val pillBorder = if (isDark) Color.White.copy(alpha = 0.14f) else Color.White.copy(alpha = 0.65f)
    val activeInk = if (isDark) Color.White else Color(0xFF141517)
    val inactiveInk = if (isDark) Color.White.copy(alpha = 0.68f) else Color(0xFF141517).copy(alpha = 0.64f)

    Box(
        modifier = Modifier
            .padding(horizontal = 24.dp, vertical = 12.dp)
            .widthIn(max = 360.dp)
            .fillMaxWidth()
            .height(56.dp)
            .shadow(
                elevation = 14.dp,
                shape = CircleShape,
                ambientColor = Color.Black.copy(alpha = if (isDark) 0.55f else 0.14f),
                spotColor = Color.Black.copy(alpha = if (isDark) 0.55f else 0.16f),
            )
            .clip(CircleShape)
            .background(brush = capsuleGradient, shape = CircleShape)
            .border(width = 1.dp, brush = specularBorder, shape = CircleShape)
            .padding(5.dp),
    ) {
        BoxWithConstraints(Modifier.fillMaxSize()) {
            val animOn = animatorsEnabled()
            val slotWidth = maxWidth / tabs.size.coerceAtLeast(1)
            var lastSelectedIndex by remember { mutableIntStateOf(0) }
            if (selectedIndex >= 0) {
                lastSelectedIndex = selectedIndex
            }
            val effectiveIndex = (if (selectedIndex >= 0) selectedIndex else lastSelectedIndex)
                .coerceIn(0, (tabs.size - 1).coerceAtLeast(0))
            val pillOffsetState = animateDpAsState(
                targetValue = slotWidth * effectiveIndex,
                animationSpec = if (animOn) spring(dampingRatio = 0.78f, stiffness = 420f) else snap(),
                label = "floating_tab_pill_offset",
            )
            val pillAlphaState = animateFloatAsState(
                targetValue = if (selectedIndex >= 0) 1f else 0f,
                animationSpec = if (animOn) tween(150) else snap(),
                label = "floating_tab_pill_alpha",
            )
            Box(
                Modifier
                    .offset { IntOffset(pillOffsetState.value.roundToPx(), 0) }
                    .width(slotWidth)
                    .fillMaxHeight()
                    .graphicsLayer { alpha = pillAlphaState.value }
                    .clip(CircleShape)
                    .background(pillFill, CircleShape)
                    .border(0.5.dp, pillBorder, CircleShape),
            )
            Row(Modifier.fillMaxSize()) {
                tabs.forEach { item ->
                    androidx.compose.runtime.key(item.route) {
                        val scaleState = animateFloatAsState(
                            targetValue = if (item.selected) 1.08f else 1f,
                            animationSpec = if (animOn) spring(dampingRatio = 0.72f, stiffness = 450f) else snap(),
                            label = "floating_tab_scale",
                        )
                        Box(
                            modifier = Modifier
                                .weight(1f)
                                .fillMaxHeight()
                                .clip(CircleShape)
                                .selectable(
                                    selected = item.selected,
                                    interactionSource = remember { MutableInteractionSource() },
                                    indication = null,
                                    role = Role.Tab,
                                    onClick = { onSelectTab(item.route) },
                                )
                                .semantics { contentDescription = item.label },
                            contentAlignment = Alignment.Center,
                        ) {
                            if (item.isProfile) {
                                Box(
                                    modifier = Modifier
                                        .graphicsLayer {
                                            val s = scaleState.value
                                            scaleX = s
                                            scaleY = s
                                        }
                                        .border(
                                            width = 2.dp,
                                            color = if (item.selected) activeInk else Color.Transparent,
                                            shape = CircleShape,
                                        )
                                        .padding(2.dp),
                                    contentAlignment = Alignment.Center,
                                ) {
                                    MemberAvatar(
                                        name = signedInMe?.name ?: "",
                                        photoUrl = signedInMe?.photoFileId?.let(fileUrlProvider),
                                        avatarColor = signedInMe?.avatarColor,
                                        avatarIcon = signedInMe?.avatarIcon,
                                        size = 26.dp,
                                    )
                                }
                            } else {
                                val icon = if (item.selected) item.filledIcon else item.outlinedIcon
                                if (icon != null) {
                                    Icon(
                                        imageVector = icon,
                                        contentDescription = null,
                                        tint = if (item.selected) activeInk else inactiveInk,
                                        modifier = Modifier
                                            .size(23.dp)
                                            .graphicsLayer {
                                                val s = scaleState.value
                                                scaleX = s
                                                scaleY = s
                                            },
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
