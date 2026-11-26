package org.pjstatecm.graceride.ui

import android.Manifest
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
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
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Event
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.outlined.CloudOff
import androidx.compose.material.icons.outlined.Event
import androidx.compose.material.icons.outlined.Home
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.adaptive.currentWindowAdaptiveInfo
import androidx.compose.material3.adaptive.navigationsuite.NavigationSuiteDefaults
import androidx.compose.material3.adaptive.navigationsuite.NavigationSuiteScaffold
import androidx.compose.material3.adaptive.navigationsuite.NavigationSuiteScaffoldDefaults
import androidx.compose.material3.adaptive.navigationsuite.NavigationSuiteType
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
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
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Me
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
import org.pjstatecm.graceride.ui.screens.costs.CostsScreen
import org.pjstatecm.graceride.ui.screens.events.EventDetailScreen
import org.pjstatecm.graceride.ui.screens.events.EventsScreen
import org.pjstatecm.graceride.ui.screens.history.HistoryScreen
import org.pjstatecm.graceride.ui.screens.home.HomeScreen
import org.pjstatecm.graceride.ui.screens.login.LoginScreen
import org.pjstatecm.graceride.ui.screens.login.PendingScreen
import org.pjstatecm.graceride.ui.screens.login.TermsScreen
import org.pjstatecm.graceride.ui.screens.login.UnlockScreen
import org.pjstatecm.graceride.ui.screens.notifications.NotificationsScreen
import org.pjstatecm.graceride.ui.screens.profile.DriverSettingsScreen
import org.pjstatecm.graceride.ui.screens.profile.GrokBotStudioScreen
import org.pjstatecm.graceride.ui.screens.profile.ProfileScreen
import org.pjstatecm.graceride.ui.screens.profile.ProxyRidersScreen
import org.pjstatecm.graceride.ui.screens.profile.SavedAddressesScreen
import org.pjstatecm.graceride.ui.screens.rides.RideDetailScreen
import org.pjstatecm.graceride.ui.screens.rides.RideNewScreen
import org.pjstatecm.graceride.ui.screens.rides.RidesScreen

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
    val context = LocalContext.current
    val view = LocalView.current
    val scope = rememberCoroutineScope()
    var optInBusy by remember { mutableStateOf(false) }
    var optInError by remember { mutableStateOf<String?>(null) }
    val notifPermLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { /* Permission result handled gracefully by NotificationHelper. */ }

    LaunchedEffect(Unit) { container.auth.refresh() }

    LaunchedEffect(authState, biometricEnrollment, biometricAppUnlocked) {
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
                    android.net.Uri.parse("graceride://open/drive"),
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
            runCatching {
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

    val animatorsOn = remember { animatorsEnabled() }
    var splashMinElapsed by remember { mutableStateOf(container.launchSplashShown || !animatorsOn) }
    var splashMounted by remember { mutableStateOf(!container.launchSplashShown || authState is AuthState.Loading) }

    LaunchedEffect(Unit) {
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
            val sessionUserId = signedInMe?.id
            androidx.compose.runtime.key(sessionUserId) {
                val navController = rememberNavController()
                val backStack by navController.currentBackStackEntryAsState()
                val currentRoute = backStack?.destination?.route
                val deepLink by (
                    pendingDeepLink?.collectAsStateWithLifecycle()
                        ?: remember { mutableStateOf<String?>(null) }
                    )

                LaunchedEffect(authState, deepLink, currentRoute) {
                    val signedIn = authState as? AuthState.SignedIn ?: return@LaunchedEffect
                    if (!signedIn.me.termsAccepted) return@LaunchedEffect
                    val rawLink = deepLink ?: return@LaunchedEffect
                    val onAuthRoute = currentRoute == null ||
                        Routes.authClasses.any { currentRoute == it || currentRoute.startsWith("$it/") }
                    if (onAuthRoute) {
                        return@LaunchedEffect
                    }
                    when (val dest = NotifLinkRouter.resolveFor(rawLink, signedIn.me)) {
                        is NotifDestination.Native -> {
                            navController.navigateRoute(dest.route) {
                                launchSingleTop = true
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
                val target: AppRoute? = when (authState) {
                    is AuthState.SignedOut -> AppRoute.Login
                    is AuthState.Locked -> AppRoute.Unlock
                    is AuthState.Pending -> AppRoute.Pending
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

            val isOnline by container.networkMonitor.isOnline.collectAsStateWithLifecycle()
            val isTabRoute = currentRoute != null && Routes.tabClasses.any {
                currentRoute == it || currentRoute.startsWith("$it/")
            }
            val adaptiveInfo = currentWindowAdaptiveInfo()
            val calculatedSuiteType = NavigationSuiteScaffoldDefaults.calculateFromAdaptiveInfo(adaptiveInfo)
            val useFloatingBottomPill = isTabRoute && calculatedSuiteType == NavigationSuiteType.NavigationBar
            val layoutType = if (!isTabRoute || useFloatingBottomPill) {
                NavigationSuiteType.None
            } else {
                calculatedSuiteType
            }
            val isHomeSelected = currentRoute == AppRoute.Home::class.qualifiedName
            val isEventsSelected = currentRoute == AppRoute.Events::class.qualifiedName
            val isProfileSelected = currentRoute == AppRoute.Profile::class.qualifiedName
            val canEventsTab = (authState as? AuthState.SignedIn)?.me?.can("events") != false
            val swipeTabs = remember(canEventsTab) {
                buildList<AppRoute> {
                    add(AppRoute.Home)
                    if (canEventsTab) add(AppRoute.Events)
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
            )
            val suiteColors = NavigationSuiteDefaults.colors(
                navigationBarContainerColor = MaterialTheme.colorScheme.background,
                navigationBarContentColor = MaterialTheme.colorScheme.onSurfaceVariant,
                navigationRailContainerColor = MaterialTheme.colorScheme.background,
                navigationRailContentColor = MaterialTheme.colorScheme.onSurfaceVariant,
            )

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
                        icon = {
                            Icon(
                                if (isHomeSelected) Icons.Filled.Home else Icons.Outlined.Home,
                                contentDescription = null,
                            )
                        },
                        label = { Text(stringResource(R.string.nav_home)) },
                        colors = navItemColors,
                    )
                    if (canEventsTab) {
                        item(
                            selected = isEventsSelected,
                            onClick = {
                                UiHaptics.tick(view)
                                navController.navigateTab(AppRoute.Events)
                            },
                            icon = {
                                Icon(
                                    if (isEventsSelected) Icons.Filled.Event else Icons.Outlined.Event,
                                    contentDescription = null,
                                )
                            },
                            label = { Text(stringResource(R.string.nav_events)) },
                            colors = navItemColors,
                        )
                    }
                    item(
                        selected = isProfileSelected,
                        onClick = {
                            UiHaptics.tick(view)
                            navController.navigateTab(AppRoute.Profile)
                        },
                        icon = {
                            MemberAvatar(
                                name = signedInMe?.name ?: "",
                                photoUrl = signedInMe?.photoFileId?.let(container::fileUrl),
                                avatarColor = signedInMe?.avatarColor,
                                avatarIcon = signedInMe?.avatarIcon,
                                size = 24.dp,
                            )
                        },
                        label = { Text(stringResource(R.string.nav_profile)) },
                        colors = navItemColors,
                    )
                },
            ) {
                CompositionLocalProvider(
                    LocalBottomBarClearance provides if (useFloatingBottomPill) 80.dp else 0.dp,
                ) {
                    Scaffold { padding ->
                        Box(
                            Modifier
                                .fillMaxSize()
                                .padding(padding)
                                .consumeWindowInsets(padding),
                        ) {
                            Column(Modifier.fillMaxSize()) {
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
                        Box(
                            Modifier
                                .weight(1f)
                                .pointerInput(currentRoute, swipeTabs, isTabRoute, animatorsOn) {
                                    if (!isTabRoute || !animatorsOn) return@pointerInput
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
                                                currentRoute == it::class.qualifiedName
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
                                    val fromIdx = tabOrderIndex(initialState.destination.route)
                                    val toIdx = tabOrderIndex(targetState.destination.route)
                                    val anim = animatorsEnabled()
                                    when {
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
                                    val fromIdx = tabOrderIndex(initialState.destination.route)
                                    val toIdx = tabOrderIndex(targetState.destination.route)
                                    val anim = animatorsEnabled()
                                    when {
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
                                    val fromIdx = tabOrderIndex(initialState.destination.route)
                                    val toIdx = tabOrderIndex(targetState.destination.route)
                                    val anim = animatorsEnabled()
                                    when {
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
                                    val fromIdx = tabOrderIndex(initialState.destination.route)
                                    val toIdx = tabOrderIndex(targetState.destination.route)
                                    val anim = animatorsEnabled()
                                    when {
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
                                composable<AppRoute.Login> { LoginScreen(container) }
                                composable<AppRoute.Unlock> { UnlockScreen(container) }
                                composable<AppRoute.Pending> { PendingScreen(container) }
                                composable<AppRoute.Terms> { TermsScreen(container) }
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
                                composable<AppRoute.EventDetail> { entry ->
                                    val args = entry.toRoute<AppRoute.EventDetail>()
                                    EventDetailScreen(container, navController, args.id, args.checkinCode)
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
                                composable<AppRoute.SavedAddresses> { SavedAddressesScreen(container, navController) }
                                composable<AppRoute.Notifications> { NotificationsScreen(container, navController) }
                                composable<AppRoute.DriveHub> { DriveHubScreen(container, navController) }
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
                            }
                        }
                            }
                            AnimatedVisibility(
                                visible = useFloatingBottomPill,
                                modifier = Modifier.align(Alignment.BottomCenter),
                                enter = slideInVertically(spring(dampingRatio = 0.82f, stiffness = 420f)) { it } + fadeIn(tween(180)),
                                exit = slideOutVertically(tween(160)) { it } + fadeOut(tween(140)),
                            ) {
                                FloatingGlassBottomBar(
                                    isHomeSelected = isHomeSelected,
                                    isEventsSelected = isEventsSelected,
                                    isProfileSelected = isProfileSelected,
                                    canEvents = canEventsTab,
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

private fun tabOrderIndex(route: String?): Int = when (route) {
    AppRoute.Home::class.qualifiedName -> 0
    AppRoute.Events::class.qualifiedName -> 1
    AppRoute.Profile::class.qualifiedName -> 2
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
    isProfileSelected: Boolean,
    canEvents: Boolean,
    signedInMe: Me?,
    fileUrlProvider: (Long) -> String,
    onSelectTab: (AppRoute) -> Unit,
) {
    val homeLabel = stringResource(R.string.nav_home)
    val eventsLabel = stringResource(R.string.nav_events)
    val profileLabel = stringResource(R.string.nav_profile)
    val tabs = remember(isHomeSelected, isEventsSelected, isProfileSelected, canEvents, homeLabel, eventsLabel, profileLabel) {
        buildList {
            add(FloatingNavTab(AppRoute.Home, homeLabel, isHomeSelected, Icons.Filled.Home, Icons.Outlined.Home))
            if (canEvents) {
                add(FloatingNavTab(AppRoute.Events, eventsLabel, isEventsSelected, Icons.Filled.Event, Icons.Outlined.Event))
            }
            add(FloatingNavTab(AppRoute.Profile, profileLabel, isProfileSelected, null, null, isProfile = true))
        }
    }
    val selectedIndex = tabs.indexOfFirst { it.selected }
    val isDark = MaterialTheme.colorScheme.background.luminance() < 0.5f

    val capsuleGradient = if (isDark) {
        Brush.verticalGradient(listOf(Color(0xC7202228), Color(0xD6121317)))
    } else {
        Brush.verticalGradient(listOf(Color(0xDBFFFFFF), Color(0xCCF3F5F9)))
    }
    val specularBorder = if (isDark) {
        Brush.verticalGradient(listOf(Color.White.copy(alpha = 0.24f), Color.White.copy(alpha = 0.06f)))
    } else {
        Brush.verticalGradient(listOf(Color.White.copy(alpha = 0.92f), Color(0xFF0F172A).copy(alpha = 0.08f)))
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
            val slotWidth = maxWidth / tabs.size.coerceAtLeast(1)
            val pillOffset by animateDpAsState(
                targetValue = slotWidth * selectedIndex.coerceAtLeast(0),
                animationSpec = spring(dampingRatio = 0.78f, stiffness = 420f),
                label = "floating_tab_pill_offset",
            )
            val pillAlpha by animateFloatAsState(
                targetValue = if (selectedIndex >= 0) 1f else 0f,
                animationSpec = tween(150),
                label = "floating_tab_pill_alpha",
            )
            if (pillAlpha > 0f) {
                Box(
                    Modifier
                        .offset { IntOffset(pillOffset.roundToPx(), 0) }
                        .width(slotWidth)
                        .fillMaxHeight()
                        .graphicsLayer { alpha = pillAlpha }
                        .clip(CircleShape)
                        .background(pillFill, CircleShape)
                        .border(0.5.dp, pillBorder, CircleShape),
                )
            }
            Row(Modifier.fillMaxSize()) {
                tabs.forEach { item ->
                    val scale by animateFloatAsState(
                        targetValue = if (item.selected) 1.08f else 1f,
                        animationSpec = spring(dampingRatio = 0.72f, stiffness = 450f),
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
                                        scaleX = scale
                                        scaleY = scale
                                    }
                                    .then(
                                        if (item.selected) {
                                            Modifier
                                                .border(2.dp, activeInk, CircleShape)
                                                .padding(2.dp)
                                        } else {
                                            Modifier
                                        },
                                    ),
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
                                            scaleX = scale
                                            scaleY = scale
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
