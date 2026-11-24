package org.pjstatecm.graceride.ui

import android.Manifest
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Event
import androidx.compose.material.icons.outlined.Home
import androidx.compose.material.icons.outlined.Person
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.flow.first
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

@Composable
fun GraceRideAppRoot(
    container: AppContainer,
    pendingDeepLink: MutableStateFlow<String?>? = null,
) {
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val notifPermLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { /* Permission result handled gracefully by NotificationHelper. */ }

    LaunchedEffect(Unit) {
        container.authEvents.events.collect { event ->
            when (event) {
                AuthEvent.Unauthorized -> container.auth.onUnauthorized()
                AuthEvent.Locked -> container.auth.onLocked()
                AuthEvent.Pending -> container.auth.onPending()
            }
        }
    }
    LaunchedEffect(Unit) { container.auth.refresh() }

    LaunchedEffect(authState) {
        val signedIn = authState as? AuthState.SignedIn ?: return@LaunchedEffect
        if (!signedIn.me.termsAccepted) return@LaunchedEffect
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            !NotificationHelper.hasPostPermission(context) &&
            container.secureStore.getString("notif_perm_asked") != "1"
        ) {
            container.secureStore.putString("notif_perm_asked", "1")
            notifPermLauncher.launch(Manifest.permission.POST_NOTIFICATIONS)
        }
    }

    if (authState is AuthState.Loading) {
        Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            CircularProgressIndicator()
        }
        return
    }

    val navController = rememberNavController()
    val backStack by navController.currentBackStackEntryAsState()
    val currentRoute = backStack?.destination?.route
    val deepLink by (
        pendingDeepLink?.collectAsStateWithLifecycle()
            ?: androidx.compose.runtime.remember { androidx.compose.runtime.mutableStateOf<String?>(null) }
        )

    LaunchedEffect(authState, deepLink, currentRoute) {
        val signedIn = authState as? AuthState.SignedIn ?: return@LaunchedEffect
        if (!signedIn.me.termsAccepted) return@LaunchedEffect
        val rawLink = deepLink ?: return@LaunchedEffect
        if (currentRoute == null) return@LaunchedEffect
        when (val dest = NotifLinkRouter.resolve(rawLink)) {
            is NotifDestination.Native -> navController.navigate(dest.route) {
                launchSingleTop = true
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
        val route = androidx.compose.runtime.snapshotFlow { currentRoute }
            .filterNotNull()
            .first()
        val target = when (authState) {
            is AuthState.SignedOut -> Routes.LOGIN
            is AuthState.Locked -> Routes.UNLOCK
            is AuthState.Pending -> Routes.PENDING
            is AuthState.SignedIn -> {
                val signedIn = authState as AuthState.SignedIn
                if (!signedIn.me.termsAccepted) {
                    Routes.TERMS
                } else if (route in setOf(Routes.LOGIN, Routes.UNLOCK, Routes.PENDING, Routes.TERMS)) {
                    Routes.HOME
                } else {
                    null
                }
            }
            else -> null
        }
        if (target != null && target != route) {
            navController.navigate(target) {
                popUpTo(0) { inclusive = true }
                launchSingleTop = true
            }
        }
    }

    val start = when (authState) {
        is AuthState.SignedIn ->
            if ((authState as AuthState.SignedIn).me.termsAccepted) Routes.HOME else Routes.TERMS
        is AuthState.Locked -> Routes.UNLOCK
        is AuthState.Pending -> Routes.PENDING
        else -> Routes.LOGIN
    }

    Scaffold(
        bottomBar = {
            if (currentRoute in Routes.tabs) {
                NavigationBar {
                    NavigationBarItem(
                        selected = currentRoute == Routes.HOME,
                        onClick = { navController.navigateTab(Routes.HOME) },
                        icon = { Icon(Icons.Outlined.Home, contentDescription = null) },
                        label = { Text(stringResource(R.string.nav_home)) },
                    )
                    NavigationBarItem(
                        selected = currentRoute == Routes.EVENTS,
                        onClick = { navController.navigateTab(Routes.EVENTS) },
                        icon = { Icon(Icons.Outlined.Event, contentDescription = null) },
                        label = { Text(stringResource(R.string.nav_events)) },
                    )
                    NavigationBarItem(
                        selected = currentRoute == Routes.PROFILE,
                        onClick = { navController.navigateTab(Routes.PROFILE) },
                        icon = { Icon(Icons.Outlined.Person, contentDescription = null) },
                        label = { Text(stringResource(R.string.nav_profile)) },
                    )
                }
            }
        },
    ) { padding ->
        NavHost(
            navController = navController,
            startDestination = start,
            modifier = Modifier.padding(padding),
        ) {
            composable(Routes.LOGIN) { LoginScreen(container) }
            composable(Routes.UNLOCK) { UnlockScreen(container) }
            composable(Routes.PENDING) { PendingScreen(container) }
            composable(Routes.TERMS) { TermsScreen(container) }
            composable(Routes.HOME) { HomeScreen(container, navController) }
            composable(Routes.RIDES) { RidesScreen(container, navController) }
            composable(Routes.RIDE_DETAIL) { entry ->
                RideDetailScreen(container, navController, entry.arguments?.getString("id")?.toLongOrNull() ?: 0L)
            }
            composable(
                route = Routes.RIDE_NEW,
                arguments = listOf(
                    androidx.navigation.navArgument("eventId") { nullable = true; defaultValue = null },
                    androidx.navigation.navArgument("proxy") { nullable = true; defaultValue = null },
                ),
            ) { entry ->
                RideNewScreen(
                    container,
                    navController,
                    entry.arguments?.getString("eventId")?.toLongOrNull(),
                    entry.arguments?.getString("proxy") == "1",
                )
            }
            composable(Routes.EVENTS) { EventsScreen(container, navController) }
            composable(Routes.EVENT_DETAIL) { entry ->
                EventDetailScreen(container, navController, entry.arguments?.getString("id")?.toLongOrNull() ?: 0L)
            }
            composable(Routes.COSTS) { CostsScreen(container, navController) }
            composable(Routes.HISTORY) { HistoryScreen(container, navController) }
            composable(Routes.PROFILE) { ProfileScreen(container, navController) }
            composable(Routes.DRIVER_SETTINGS) { DriverSettingsScreen(container, navController) }
            composable(Routes.PROXY_RIDERS) { ProxyRidersScreen(container, navController) }
            composable(Routes.SAVED_ADDRESSES) { SavedAddressesScreen(container, navController) }
            composable(Routes.NOTIFICATIONS) { NotificationsScreen(container, navController) }
            composable(
                route = Routes.STUDIO,
                arguments = listOf(
                    androidx.navigation.navArgument("shape") { nullable = true; defaultValue = null },
                    androidx.navigation.navArgument("color") { nullable = true; defaultValue = null },
                    androidx.navigation.navArgument("face") { nullable = true; defaultValue = null },
                ),
            ) { entry ->
                GrokBotStudioScreen(
                    container = container,
                    nav = navController,
                    initialShape = entry.arguments?.getString("shape"),
                    initialColor = entry.arguments?.getString("color"),
                    initialFace = entry.arguments?.getString("face"),
                )
            }
        }
    }
}

private fun androidx.navigation.NavHostController.navigateTab(route: String) {
    navigate(route) {
        popUpTo(graph.findStartDestination().id) { saveState = true }
        launchSingleTop = true
        restoreState = true
    }
}
