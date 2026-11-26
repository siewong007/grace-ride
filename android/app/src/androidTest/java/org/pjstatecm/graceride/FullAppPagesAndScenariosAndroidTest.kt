package org.pjstatecm.graceride

import android.content.Context
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.hasScrollAction
import androidx.compose.ui.test.hasSetTextAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.isToggleable
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onFirst
import androidx.compose.ui.test.onLast
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performScrollToNode
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.performTextReplacement
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.toRoute
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.encodeToString
import okhttp3.mockwebserver.Dispatcher
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import okhttp3.mockwebserver.RecordedRequest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.GraceRideAppRoot
import org.pjstatecm.graceride.ui.screens.costs.CostsScreen
import org.pjstatecm.graceride.ui.screens.costs.CostsViewModel
import org.pjstatecm.graceride.ui.screens.drive.DriveHubScreen
import org.pjstatecm.graceride.ui.screens.events.EventDetailScreen
import org.pjstatecm.graceride.ui.screens.events.EventsScreen
import org.pjstatecm.graceride.ui.screens.history.HistoryScreen
import org.pjstatecm.graceride.ui.screens.home.HomeScreen
import org.pjstatecm.graceride.ui.screens.login.LoginScreen
import org.pjstatecm.graceride.ui.screens.login.TermsScreen
import org.pjstatecm.graceride.ui.screens.login.UnlockScreen
import org.pjstatecm.graceride.ui.screens.notifications.NotificationsScreen
import org.pjstatecm.graceride.ui.screens.profile.AppearanceSettingsScreen
import org.pjstatecm.graceride.ui.screens.profile.DriverSettingsScreen
import org.pjstatecm.graceride.ui.screens.profile.ProfileScreen
import org.pjstatecm.graceride.ui.screens.profile.ProxyRidersScreen
import org.pjstatecm.graceride.ui.screens.profile.SavedAddressesScreen
import org.pjstatecm.graceride.ui.screens.rides.RideDetailScreen
import org.pjstatecm.graceride.ui.screens.rides.RideNewScreen
import org.pjstatecm.graceride.ui.screens.rides.RidesScreen
import org.pjstatecm.graceride.ui.screens.rides.TripChatSheet
import org.pjstatecm.graceride.ui.theme.GraceRideTheme
import java.util.Collections

@RunWith(AndroidJUnit4::class)
class FullAppPagesAndScenariosAndroidTest {

    @get:Rule
    val compose = createComposeRule()

    private lateinit var server: MockWebServer
    private lateinit var container: AppContainer
    private val recordedCalls = Collections.synchronizedList(mutableListOf<String>())

    private val targetContext: Context
        get() = InstrumentationRegistry.getInstrumentation().targetContext

    private fun res(id: Int, vararg args: Any): String =
        if (args.isEmpty()) targetContext.getString(id) else targetContext.getString(id, *args)

    @Before
    fun setup() {
        recordedCalls.clear()
        server = MockWebServer()
        server.start()
        container = AppContainer(targetContext, InMemoryStore())
        container.setBaseUrl(server.url("/").toString())
        container.launchSplashShown = true
        container.secureStore.putString("notif_perm_asked", "1")
    }

    @After
    fun teardown() {
        runCatching { server.shutdown() }
    }

    private fun waitForText(text: String, substring: Boolean = false, timeoutMs: Long = 8_000) {
        compose.waitUntil(timeoutMs) {
            compose.onAllNodesWithText(text, substring = substring).fetchSemanticsNodes().isNotEmpty()
        }
    }

    @Composable
    private fun FreshScreenSlot(slotKey: Any, content: @Composable () -> Unit) {
        key(slotKey) {
            val owner = remember {
                object : ViewModelStoreOwner {
                    override val viewModelStore = ViewModelStore()
                }
            }
            DisposableEffect(owner) {
                onDispose { owner.viewModelStore.clear() }
            }
            CompositionLocalProvider(LocalViewModelStoreOwner provides owner) {
                content()
            }
        }
    }

    private fun buildMeJson(
        id: Long = 1L,
        name: String = "Role Tester",
        phone: String = "012-3456789",
        roles: List<String> = listOf("member", "driver"),
        permissions: List<String> = listOf(
            "rides.view",
            "rides.create",
            "rides.update",
            "rides.delete",
            "drive.view",
            "drive.create",
            "drive.update",
            "drive.delete",
            "events.view",
            "costs.view",
            "costs.update",
        ),
        status: String = "active",
    ): String {
        val rolesJson = apiJson.encodeToString(roles)
        val permsJson = apiJson.encodeToString(permissions)
        return """
            {
              "id": $id,
              "name": ${apiJson.encodeToString(name)},
              "email": "user$id@example.org",
              "phone": ${apiJson.encodeToString(phone)},
              "roles": $rolesJson,
              "role_names": {},
              "permissions": $permsJson,
              "status": "$status",
              "lang": "en",
              "terms_accepted": true,
              "mfa_ok": true,
              "passkey_count": 1
            }
        """.trimIndent()
    }

    private fun setupSignedInDispatcher(
        meJson: String = buildMeJson(),
        routes: Map<String, (RecordedRequest) -> MockResponse> = emptyMap(),
    ) {
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val method = request.method ?: "GET"
                val path = request.path ?: ""
                recordedCalls.add("$method $path")
                routes.entries.firstOrNull { path.startsWith(it.key) }?.let { return it.value(request) }
                return when {
                    path.startsWith("/api/auth/me") -> MockResponse().setBody(meJson)
                    path.startsWith("/api/auth/passkeys") ->
                        MockResponse().setBody("""[{"id":11,"label":"Pixel Passkey","created_at":"2026-11-01 09:00:00"}]""")
                    path.startsWith("/api/auth/biometrics") -> MockResponse().setBody("[]")
                    path.startsWith("/api/me") && method == "PUT" -> MockResponse().setBody("""{"ok":true}""")
                    path.startsWith("/api/notifications/preferences") && method == "PUT" -> MockResponse().setBody("""{"ok":true}""")
                    path.startsWith("/api/notifications/preferences") ->
                        MockResponse().setBody("""{"push_opt_in":true,"whatsapp_opt_in":false,"readiness_prompt_mins":45,"driver_remind_mins":60,"muted_categories":[],"quiet_hours_start":"","quiet_hours_end":"","hide_lock_details":false}""")
                    path.startsWith("/api/notifications") -> MockResponse().setBody("""{"items":[],"next_before":null,"unread":2}""")
                    path.startsWith("/api/announcements") ->
                        MockResponse().setBody("""[{"id":1,"title":"Sunday Notice","body":"Arrive 10 min early","pinned":true,"by":"Admin","created_at":"2026-11-25 10:00:00"}]""")
                    path.startsWith("/api/areas") -> MockResponse().setBody("""[{"id":1,"name":"PJ Section 14","active":true}]""")
                    path.startsWith("/api/driver/profile") && method == "PUT" -> MockResponse().setBody("""{"ok":true}""")
                    path.startsWith("/api/driver/profile") ->
                        MockResponse().setBody("""{"carpool_enabled":true,"spare_seats":3,"gate_wait_mins":5,"area_ids":[1],"vehicle":{"id":10,"model":"Perodua Alza","color":"Silver","plate":"WAA 1234","seats":7,"is_active":true},"vehicles":[{"id":10,"model":"Perodua Alza","color":"Silver","plate":"WAA 1234","seats":7,"is_active":true}],"active_vehicle_id":10}""")
                    path.startsWith("/api/driver/route") -> MockResponse().setBody("""{"route":null}""")
                    path.startsWith("/api/driver/requests") -> MockResponse().setBody("[]")
                    path.startsWith("/api/driver/schedule") -> MockResponse().setBody("[]")
                    path.startsWith("/api/events") -> MockResponse().setBody("[]")
                    path.startsWith("/api/rides") -> MockResponse().setBody("[]")
                    path.startsWith("/api/me/shares") -> MockResponse().setBody("[]")
                    else -> MockResponse().setBody("[]")
                }
            }
        }
        runBlocking { container.auth.refresh() }
    }

    @Test
    fun appRootNavigationAndZeroDuplicationSettingsFlow() {
        val nextRideJson = """{"id":40,"status":"confirmed","pickup_at":"2026-11-27 09:00:00","area":{"id":1,"name":"PJ Section 14"},"destination":"Church Sanctuary","seats":1,"passenger":{"id":1,"name":"Role Tester"},"viewer":{"is_passenger":true,"is_driver":false,"is_staff":false}}"""
        val upcomingEventJson = """[{"id":50,"title":"Youth Service","starts_at":"2026-12-20 09:00:00","location":"Church Sanctuary","status":"published","going":12,"capacity":50,"spots_left":38}]"""
        val shareJson = """[{"id":30,"settlement_id":3,"title":"Fuel Nov","event_title":"Worship","amount_cents":1250,"status":"unpaid","payee_name":"Finance"}]"""

        setupSignedInDispatcher(
            routes = mapOf(
                "/api/events?scope=past" to { MockResponse().setBody("[]") },
                "/api/events" to { MockResponse().setBody(upcomingEventJson) },
                "/api/rides" to { MockResponse().setBody("[$nextRideJson]") },
                "/api/me/shares" to { MockResponse().setBody(shareJson) },
            ),
        )

        compose.setContent {
            val mode by container.themeMode.collectAsState()
            GraceRideTheme(darkTheme = mode == "dark") {
                GraceRideAppRoot(container = container)
            }
        }

        // 1. Verify Home screen layout & zero filler "Addresses" quick action
        waitForText("Grace Ride")
        waitForText("Hello, Role Tester", substring = true)
        compose.onNodeWithText(res(R.string.home_quick_rides)).assertIsDisplayed()
        compose.onNodeWithText(res(R.string.home_quick_costs)).assertIsDisplayed()
        compose.onNodeWithText(res(R.string.shortcut_drive_short)).assertIsDisplayed()
        compose.onNodeWithText(res(R.string.history_title)).assertIsDisplayed()
        compose.onNodeWithText(res(R.string.home_quick_addresses)).assertDoesNotExist()
        compose.onAllNodesWithText("Church Sanctuary", substring = true).onFirst().assertIsDisplayed()

        // 2. Navigate to Events tab via FloatingGlassBottomBar
        compose.onNodeWithContentDescription(res(R.string.nav_events)).performClick()
        waitForText(res(R.string.events_tab_my_schedule))
        compose.onNodeWithContentDescription(res(R.string.notif_title)).assertIsDisplayed()
        compose.onNodeWithText(res(R.string.events_tab_my_schedule)).performClick()
        waitForText("Church Sanctuary", substring = true)
        compose.onNodeWithText(res(R.string.events_tab_past)).performClick()
        waitForText(res(R.string.events_empty))
        compose.onNodeWithText(res(R.string.events_tab_upcoming)).performClick()
        waitForText("Youth Service")

        // 3. Navigate to Profile tab & verify ZERO duplicated operational items
        compose.onNodeWithContentDescription(res(R.string.nav_profile)).performClick()
        waitForText(res(R.string.profile_personal))
        compose.onNodeWithText(res(R.string.rides_title)).assertDoesNotExist()
        compose.onNodeWithText(res(R.string.history_title)).assertDoesNotExist()
        compose.onNodeWithText(res(R.string.drive_hub_title)).assertDoesNotExist()
        compose.onNodeWithText(res(R.string.costs_title)).assertDoesNotExist()
        compose.onNodeWithText(res(R.string.notif_title)).assertDoesNotExist()
        compose.onNodeWithText(res(R.string.profile_logout_all)).assertDoesNotExist()

        // 4a. Personal Details sub-screen
        compose.onNodeWithText(res(R.string.profile_personal)).performClick()
        waitForText(res(R.string.profile_phone))
        compose.onNode(hasSetTextAction() and hasText("012-3456789")).performTextReplacement("012-8889999")
        compose.onNodeWithText(res(R.string.profile_save)).performClick()
        waitForText(res(R.string.profile_saved))
        assertTrue(recordedCalls.any { it.startsWith("PUT /api/me") })
        compose.onNodeWithContentDescription(res(R.string.a11y_back)).performClick()

        // 4b. Appearance & language sub-screen
        waitForText(res(R.string.profile_appearance_lang))
        compose.onNode(hasScrollAction()).performScrollToNode(hasText(res(R.string.profile_appearance_lang)))
        compose.onNodeWithText(res(R.string.profile_appearance_lang)).performClick()
        waitForText(res(R.string.profile_theme_dark))
        compose.onNodeWithText(res(R.string.profile_theme_dark)).performClick()
        assertEquals("dark", container.themeMode.value)
        compose.onNodeWithText(res(R.string.profile_theme_light)).performClick()
        assertEquals("light", container.themeMode.value)
        compose.onNodeWithText(res(R.string.profile_theme_system)).performClick()
        assertEquals("system", container.themeMode.value)
        compose.onNodeWithContentDescription(res(R.string.a11y_back)).performClick()

        // 4c. Notification preferences sub-screen
        waitForText(res(R.string.notif_prefs_title))
        compose.onNode(hasScrollAction()).performScrollToNode(hasText(res(R.string.notif_prefs_title)))
        compose.onNodeWithText(res(R.string.notif_prefs_title)).performClick()
        waitForText(res(R.string.notif_pref_push))
        compose.onNodeWithText(res(R.string.notif_cat_admin)).assertDoesNotExist()
        compose.onNode(hasScrollAction()).performScrollToNode(hasText(res(R.string.notif_pref_save)))
        compose.onNodeWithText(res(R.string.notif_pref_save)).performClick()
        waitForText(res(R.string.notif_pref_saved))
        assertTrue(recordedCalls.any { it.startsWith("PUT /api/notifications/preferences") })
        compose.onNodeWithContentDescription(res(R.string.a11y_back)).performClick()

        // 4d. Security sub-screen
        waitForText(res(R.string.profile_security))
        compose.onNode(hasScrollAction()).performScrollToNode(hasText(res(R.string.profile_security)))
        compose.onNodeWithText(res(R.string.profile_security)).performClick()
        waitForText("Pixel Passkey")
        compose.onNodeWithContentDescription(res(R.string.common_delete)).performClick()
        waitForText(res(R.string.profile_passkey_delete_title))
        compose.onNodeWithText(res(R.string.common_cancel)).performClick()
        compose.onNode(hasScrollAction()).performScrollToNode(hasText(res(R.string.profile_logout_all)))
        compose.onNodeWithText(res(R.string.profile_logout_all)).performClick()
        waitForText(res(R.string.profile_logout_all_confirm))
        compose.onNodeWithText(res(R.string.common_cancel)).performClick()
        compose.onNodeWithContentDescription(res(R.string.a11y_back)).performClick()
        waitForText(res(R.string.profile_logout))
    }

    @Test
    fun driverSettingsNavigationAndVehicleSheetFlow() {
        setupSignedInDispatcher(
            routes = mapOf(
                "/api/driver/vehicles" to { req ->
                    if (req.method == "POST") {
                        MockResponse().setBody("""{"id":12,"model":"Honda BR-V","color":"White","plate":"WBB 8888","seats":5,"vehicle_type":"standard","is_active":true}""")
                    } else {
                        MockResponse().setBody("[]")
                    }
                },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                val nav = rememberNavController()
                DriverSettingsScreen(container, nav)
            }
        }

        waitForText("WAA 1234")
        // Verify Navigation & arrival card
        compose.onNode(hasScrollAction()).performScrollToNode(hasText(res(R.string.driver_nav_arrival_section)))
        compose.onNodeWithText(res(R.string.nav_app_waze)).performClick()
        assertEquals("waze", container.navAppPref.value)
        compose.onNodeWithText(res(R.string.nav_app_google)).performClick()
        assertEquals("google", container.navAppPref.value)
        compose.onNodeWithText(res(R.string.nav_app_ask)).performClick()
        assertEquals("ask", container.navAppPref.value)

        // Toggle Auto-mark arrived at gate switch cleanly
        val initialAuto = container.autoArriveGate.value
        compose.onNode(hasScrollAction()).performScrollToNode(hasText(res(R.string.driver_auto_arrive_title)))
        compose.onNodeWithText(res(R.string.driver_auto_arrive_title)).performClick()
        assertEquals(!initialAuto, container.autoArriveGate.value)
        compose.onNodeWithText(res(R.string.driver_auto_arrive_title)).performClick()
        assertEquals(initialAuto, container.autoArriveGate.value)

        // Make form dirty via Spare Seats Increase and save settings
        compose.onNode(hasScrollAction()).performScrollToNode(hasText(res(R.string.driver_spare_seats)))
        compose.onNodeWithContentDescription(res(R.string.a11y_increase)).performClick()
        compose.onNode(hasScrollAction()).performScrollToNode(hasText(res(R.string.driver_save_settings)))
        compose.onNodeWithText(res(R.string.driver_save_settings)).performClick()
        waitForText(res(R.string.driver_settings_saved))
        assertTrue(recordedCalls.any { it.startsWith("PUT /api/driver/profile") })

        // Add vehicle sheet creation flow
        compose.onNodeWithContentDescription(res(R.string.driver_add_vehicle)).performClick()
        waitForText(res(R.string.driver_vehicle_model))
        compose.onNodeWithText(res(R.string.driver_vehicle_model)).performScrollTo().performTextInput("Honda BR-V")
        compose.onNodeWithText(res(R.string.driver_vehicle_color)).performScrollTo().performTextInput("White")
        compose.onNodeWithText(res(R.string.driver_vehicle_plate)).performScrollTo().performTextInput("WBB 8888")
        compose.onNodeWithText(res(R.string.common_save)).performScrollTo().performClick()
        waitForText(res(R.string.driver_vehicle_saved))
        assertTrue(recordedCalls.any { it.startsWith("POST /api/driver/vehicles") })
    }

    @Test
    fun driveHubLiveRouteMapStopsAndWazeChooserFlow() {
        container.setNavAppPref("ask")
        val openReq = """[{"id":55,"status":"pending_driver","pickup_at":"2026-11-26 09:00:00","area":{"id":1,"name":"PJ Section 14"},"destination":"Church Sanctuary","seats":1,"passenger":{"id":9,"name":"Grace"},"viewer":{"is_passenger":false,"is_driver":false,"is_staff":false}}]"""
        val schedRide = """[{"id":60,"status":"confirmed","pickup_at":"2026-11-26 08:30:00","area":{"id":1,"name":"PJ Section 14"},"destination":"Church Sanctuary","seats":1,"passenger":{"id":8,"name":"David"},"viewer":{"is_passenger":false,"is_driver":true,"is_staff":false}}]"""
        val routeJson = """
            {
              "route": {
                "trip_id": 99,
                "live": true,
                "origin": {"lat": 3.1073, "lng": 101.6067, "heading": 45.0, "updated_at": "2026-11-26 08:55:00"},
                "stops": [
                  {"ride_id": 61, "passenger": "Sarah", "address": "12 Jalan SS2", "pickup_at": "2026-11-26 08:40:00", "seats": 1, "started": true, "lat": 3.1100, "lng": 101.6090, "location": "live", "distance_m": 650.0, "eta": "2026-11-26 08:42:00"},
                  {"ride_id": 62, "passenger": "David", "address": "8 Jalan 14/2", "pickup_at": "2026-11-26 08:50:00", "seats": 1, "started": true, "lat": 3.1130, "lng": 101.6120, "location": "address", "distance_m": 1800.0, "eta": "2026-11-26 08:50:00"}
                ],
                "onboard": 1,
                "destination": {"name": "Church Sanctuary", "lat": 3.1180, "lng": 101.6150, "eta": "2026-11-26 09:10:00"},
                "distance_m": 3200.0,
                "duration_s": 720.0,
                "geometry": [[3.1073, 101.6067], [3.1100, 101.6090], [3.1130, 101.6120], [3.1180, 101.6150]]
              }
            }
        """.trimIndent()

        setupSignedInDispatcher(
            routes = mapOf(
                "/api/driver/requests" to { MockResponse().setBody(openReq) },
                "/api/driver/schedule" to { MockResponse().setBody(schedRide) },
                "/api/driver/route" to { MockResponse().setBody(routeJson) },
                "/api/rides/55/claim" to { MockResponse().setBody("""{"ok":true}""") },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                val nav = rememberNavController()
                DriveHubScreen(container, nav)
            }
        }

        // Requests tab
        waitForText("Grace")
        compose.onNodeWithText(res(R.string.drive_filter_my_areas)).performClick()
        compose.onNodeWithText(res(R.string.drive_claim)).performClick()
        compose.waitUntil(5_000) {
            recordedCalls.any { it.startsWith("POST /api/rides/55/claim") }
        }

        // Schedule tab
        compose.onNodeWithText(res(R.string.drive_tab_schedule)).performClick()
        waitForText("David")

        // Live Route tab with GraceRideLiveMap, numbered stops & Waze chooser
        compose.onNodeWithText(res(R.string.drive_tab_route)).performClick()
        waitForText("Sarah")
        compose.onNodeWithText(res(R.string.live_location_spot_shared)).assertIsDisplayed()
        compose.onAllNodesWithText("1", substring = false).onFirst().assertIsDisplayed()
        compose.onAllNodesWithText("2", substring = false).onFirst().assertIsDisplayed()

        // Tap Navigate to trigger Waze / Google Maps smart chooser sheet
        compose.onAllNodesWithText(res(R.string.drive_route_navigate)).onFirst().performClick()
        waitForText(res(R.string.nav_chooser_title))
        compose.onNodeWithText(res(R.string.nav_app_waze)).assertIsDisplayed()
        compose.onNodeWithText(res(R.string.nav_app_google)).assertIsDisplayed()
        compose.onNodeWithText(res(R.string.nav_remember_choice)).assertIsDisplayed()
        compose.onNodeWithText(res(R.string.common_cancel)).performClick()
    }

    @Test
    fun rideDetailDriverAndRiderLiveMapBoardingAutoOffFlow() {
        container.setNavAppPref("ask")
        var step by mutableIntStateOf(0)

        val ride71Confirmed = """{"id":71,"status":"confirmed","pickup_at":"2026-11-26 09:00:00","area":{"id":1,"name":"PJ Section 14"},"pickup_address":"12 Jalan SS2","destination":"Main Hall","seats":1,"trip_id":99,"passenger":{"id":2,"name":"Sarah"},"driver":{"id":1,"name":"Role Tester"},"viewer":{"is_passenger":false,"is_driver":true,"is_staff":false}}"""
        val ride72DriverPrePickup = """{"id":72,"status":"in_progress","pickup_at":"2026-11-26 09:00:00","started_at":"2026-11-26 08:45:00","area":{"id":1,"name":"PJ Section 14"},"pickup_address":"12 Jalan SS2","destination":"Main Hall","seats":1,"trip_id":99,"passenger":{"id":2,"name":"Sarah"},"driver":{"id":1,"name":"Role Tester"},"viewer":{"is_passenger":false,"is_driver":true,"is_staff":false}}"""
        val ride72DriverPickedUp = """{"id":72,"status":"in_progress","pickup_at":"2026-11-26 09:00:00","started_at":"2026-11-26 08:45:00","picked_up_at":"2026-11-26 08:58:00","area":{"id":1,"name":"PJ Section 14"},"pickup_address":"12 Jalan SS2","destination":"Main Hall","seats":1,"trip_id":99,"passenger":{"id":2,"name":"Sarah"},"driver":{"id":1,"name":"Role Tester"},"viewer":{"is_passenger":false,"is_driver":true,"is_staff":false}}"""
        val ride73RiderWaiting = """{"id":73,"status":"in_progress","pickup_at":"2026-11-26 09:00:00","started_at":"2026-11-26 08:45:00","area":{"id":1,"name":"PJ Section 14"},"pickup_address":"12 Jalan SS2","destination":"Main Hall","seats":1,"trip_id":99,"passenger":{"id":1,"name":"Role Tester"},"driver":{"id":5,"name":"Driver Dan"},"viewer":{"is_passenger":true,"is_driver":false,"is_staff":false}}"""
        val ride74RiderBoarded = """{"id":74,"status":"in_progress","pickup_at":"2026-11-26 09:00:00","started_at":"2026-11-26 08:45:00","picked_up_at":"2026-11-26 08:58:00","area":{"id":1,"name":"PJ Section 14"},"pickup_address":"12 Jalan SS2","destination":"Main Hall","seats":1,"trip_id":99,"passenger":{"id":1,"name":"Role Tester"},"driver":{"id":5,"name":"Driver Dan"},"viewer":{"is_passenger":true,"is_driver":false,"is_staff":false}}"""

        val locPrePickup = """{"sharing":true,"lat":3.1073,"lng":101.6067,"heading":30.0,"passenger":{"lat":3.1100,"lng":101.6090,"updated_at":"2026-11-26 08:50:00"},"eta":{"phase":"pickup","located":true,"stops_before":0,"distance_m":2400.0,"duration_s":360.0,"eta":"2026-11-26 09:05:00","destination":"Main Hall","target":[3.1100,101.6090],"path":[[3.1073,101.6067],[3.1100,101.6090]]}}"""
        val locDropoff = """{"sharing":true,"lat":3.1105,"lng":101.6095,"heading":40.0,"eta":{"phase":"dropoff","located":true,"stops_before":0,"distance_m":3200.0,"duration_s":480.0,"eta":"2026-11-26 09:20:00","destination":"Main Hall","target":[3.1180,101.6150],"path":[[3.1105,101.6095],[3.1180,101.6150]]}}"""

        setupSignedInDispatcher(
            routes = mapOf(
                "/api/rides/71/start" to { MockResponse().setBody("""{"ok":true}""") },
                "/api/rides/71" to { MockResponse().setBody(ride71Confirmed) },
                "/api/rides/72/location" to { MockResponse().setBody(if (step >= 3) locDropoff else locPrePickup) },
                "/api/rides/72/arrived-gate" to { MockResponse().setBody("""{"ok":true}""") },
                "/api/rides/72/pickup" to { MockResponse().setBody("""{"ok":true}""") },
                "/api/rides/72/complete" to { MockResponse().setBody("""{"ok":true}""") },
                "/api/rides/72" to { MockResponse().setBody(if (step >= 4) ride72DriverPickedUp else ride72DriverPrePickup) },
                "/api/rides/73/location" to { MockResponse().setBody(locPrePickup) },
                "/api/rides/73" to { MockResponse().setBody(ride73RiderWaiting) },
                "/api/rides/74/location" to { MockResponse().setBody(locDropoff) },
                "/api/rides/74" to { MockResponse().setBody(ride74RiderBoarded) },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                val nav = rememberNavController()
                FreshScreenSlot(step) {
                    when (step) {
                        0 -> RideDetailScreen(container, nav, id = 71L)
                        1, 4 -> RideDetailScreen(container, nav, id = 72L)
                        2 -> RideDetailScreen(container, nav, id = 73L)
                        else -> RideDetailScreen(container, nav, id = 74L)
                    }
                }
            }
        }

        // 0: Driver confirmed ride -> Start trip 1-step GPS rationale dialog -> Start without location
        waitForText(res(R.string.driver_start_trip))
        compose.onNodeWithText(res(R.string.driver_start_trip)).performScrollTo().performClick()
        waitForText(res(R.string.start_trip_loc_title))
        compose.onNodeWithText(res(R.string.start_trip_allow_and_start)).assertIsDisplayed()
        compose.onNodeWithText(res(R.string.start_trip_without_loc)).performClick()
        runCatching {
            compose.waitUntil(8_000) {
                recordedCalls.any { it.startsWith("POST /api/rides/71/start") }
            }
        }.onFailure { err ->
            throw AssertionError("Missing POST /api/rides/71/start; calls=${recordedCalls.toList()}", err)
        }

        // 1: Driver in_progress pre-pickup -> LiveTrackCard near top, Waze chooser, Arrived gate & Picked up
        compose.runOnIdle { step = 1 }
        waitForText(res(R.string.live_location_title))
        compose.onNodeWithText(res(R.string.drive_route_navigate)).performScrollTo().performClick()
        waitForText(res(R.string.nav_chooser_title))
        compose.onNodeWithText(res(R.string.common_cancel)).performClick()
        compose.waitUntil(5_000) {
            compose.onAllNodesWithText(res(R.string.nav_chooser_title)).fetchSemanticsNodes().isEmpty()
        }
        compose.onNodeWithText(res(R.string.driver_arrived_gate)).performScrollTo().performClick()
        compose.waitUntil(5_000) {
            recordedCalls.any { it.startsWith("POST /api/rides/72/arrived-gate") }
        }
        waitForText(res(R.string.driver_picked_up))
        compose.onNodeWithText(res(R.string.driver_picked_up)).performScrollTo().performClick()
        compose.waitUntil(5_000) {
            recordedCalls.any { it.startsWith("POST /api/rides/72/pickup") }
        }

        // 2: Waiting Rider -> Driver -> Rider map, personalized ETA (~6 min away + next stop + 2.4 km)
        compose.runOnIdle { step = 2 }
        waitForText(res(R.string.live_eta_away, 6))
        compose.onNodeWithText("2.4 km", substring = true).assertIsDisplayed()
        compose.onNodeWithText(res(R.string.live_next_stop), substring = true).assertIsDisplayed()
        compose.onNodeWithText(res(R.string.live_onboard_auto_off)).assertDoesNotExist()

        // 3: Boarded Rider -> Auto-off confirmation banner, Car -> Destination ETA, waiting pin button hidden
        compose.runOnIdle { step = 3 }
        waitForText(res(R.string.live_onboard_auto_off))
        compose.onNodeWithText(res(R.string.live_eta_arriving_dest, "Main Hall", 8)).assertIsDisplayed()
        compose.onNodeWithText(res(R.string.live_location_share_spot)).assertDoesNotExist()
        compose.onNodeWithText(res(R.string.live_location_stop_spot)).assertDoesNotExist()

        // 4: Driver post-pickup -> Complete trip
        compose.runOnIdle { step = 4 }
        waitForText(res(R.string.driver_complete_trip))
        compose.onNodeWithText(res(R.string.driver_complete_trip)).performScrollTo().performClick()
        compose.waitUntil(5_000) {
            recordedCalls.any { it.startsWith("POST /api/rides/72/complete") }
        }
    }

    @Test
    fun rideNewEventBookingAndProxyFlow() {
        var mode by mutableIntStateOf(0)
        val eventJson = """{"id":50,"title":"Sunday Gathering","starts_at":"2026-12-20 09:00:00","location":"Church Sanctuary","status":"published","going":10,"my_signup":{"status":"going"}}"""
        val savedJson = """[{"id":5,"label":"Home Spot","area_id":1,"area_name":"PJ Section 14","address":"22 Jalan 14/10","gate_notes":"Ring bell","is_default":1}]"""
        val proxyJson = """[{"id":9,"name":"Aunt May","phone":"012-7778888","relationship":"parent","notes":"","area_id":1,"pickup_address":"22 Jalan 14/10","gate_notes":""}]"""

        setupSignedInDispatcher(
            routes = mapOf(
                "/api/events/50" to { MockResponse().setBody(eventJson) },
                "/api/saved-places" to { MockResponse().setBody(savedJson) },
                "/api/proxy-riders" to { MockResponse().setBody(proxyJson) },
                "/api/rides" to { req ->
                    if (req.method == "POST") MockResponse().setBody("""{"ok":true,"id":101}""")
                    else MockResponse().setBody("[]")
                },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                FreshScreenSlot(mode) {
                    val nav = rememberNavController()
                    NavHost(
                        navController = nav,
                        startDestination = if (mode == 0) {
                            AppRoute.RideNew(eventId = 50L, proxy = false)
                        } else {
                            AppRoute.RideNew(eventId = 50L, proxy = true)
                        },
                    ) {
                        composable<AppRoute.RideNew> { entry ->
                            val args = entry.toRoute<AppRoute.RideNew>()
                            RideNewScreen(container, nav, eventId = args.eventId, initialProxy = args.proxy)
                        }
                        composable<AppRoute.RideDetail> { entry ->
                            val args = entry.toRoute<AppRoute.RideDetail>()
                            androidx.compose.material3.Text("Booked ride #${args.id}")
                        }
                    }
                }
            }
        }

        // 0: Self event ride booking with Round trip -> To event, saved place, wheelchair & submit
        waitForText("Sunday Gathering")
        compose.onNodeWithText(res(R.string.ride_new_trip_type_round)).performClick()
        waitForText(res(R.string.ride_new_return_time))
        compose.onNodeWithText(res(R.string.ride_new_trip_type_outbound)).performClick()
        compose.onNodeWithText("Home Spot").performScrollTo().performClick()
        compose.onNodeWithContentDescription(res(R.string.a11y_increase)).performScrollTo().performClick()
        compose.onNodeWithText(res(R.string.ride_new_assist_wheelchair)).performScrollTo().performClick()
        compose.onNodeWithText(res(R.string.ride_new_submit)).performClick()
        waitForText("Booked ride #101")
        assertTrue(recordedCalls.any { it.startsWith("POST /api/rides") })

        // 1: Proxy rider booking mode
        compose.runOnIdle { mode = 1 }
        waitForText(res(R.string.ride_new_frequent_riders))
        compose.onNodeWithText("Aunt May").assertIsDisplayed()
    }

    @Test
    fun eventDetailRsvpQrPassCostsValidationAndChatFlow() {
        var screen by mutableIntStateOf(0)
        val openEvent = """{"id":50,"title":"Outreach Day","starts_at":"2026-12-20 09:00:00","location":"Community Hall","status":"published","going":5,"capacity":30,"spots_left":25}"""
        val signedEvent = """{"id":51,"title":"Prayer Night","starts_at":"2026-12-21 19:30:00","location":"Upper Room","status":"published","attendance_enabled":true,"going":8,"capacity":40,"spots_left":32,"my_signup":{"status":"going","attended":false,"pass_code":"PASS51"}}"""
        val unpaidShare = """[{"id":30,"settlement_id":3,"title":"Fuel Nov","event_title":"Worship","amount_cents":1500,"status":"unpaid","payee_name":"Finance","payee_account":"1234567890"}]"""
        val openChat = """{"open":true,"can_post":true,"members":["Driver Dan","Role Tester"],"messages":[{"id":1,"user_id":5,"sender_name":"Driver Dan","body":"Heading out now","created_at":"2026-11-26 08:45:00","reactions":[]}]}"""
        val savedList = """[{"id":1,"label":"Home","area_id":1,"area_name":"PJ Section 14","address":"10 Jalan Test","gate_notes":"","is_default":1}]"""
        val proxyList = """[{"id":2,"name":"Uncle Tan","phone":"012-1112222","relationship":"parent","notes":"","area_id":1,"pickup_address":"10 Jalan Test","gate_notes":""}]"""
        val pastRide = """[{"id":95,"status":"completed","pickup_at":"2026-11-10 09:00:00","area":{"id":1,"name":"PJ Section 14"},"destination":"Completed Ride Alpha","seats":1,"passenger":{"id":1,"name":"Role Tester"},"viewer":{"is_passenger":true}}]"""

        setupSignedInDispatcher(
            routes = mapOf(
                "/api/events/50/signup" to { MockResponse().setBody("""{"ok":true}""") },
                "/api/events/50" to { MockResponse().setBody(openEvent) },
                "/api/events/51/withdraw" to { MockResponse().setBody("""{"ok":true}""") },
                "/api/events/51" to { MockResponse().setBody(signedEvent) },
                "/api/me/shares" to { MockResponse().setBody(unpaidShare) },
                "/api/rides/89/messages" to { req ->
                    if (req.method == "POST") MockResponse().setBody("""{"ok":true,"id":2}""")
                    else MockResponse().setBody(openChat)
                },
                "/api/saved-places" to { MockResponse().setBody(savedList) },
                "/api/proxy-riders" to { MockResponse().setBody(proxyList) },
                "/api/rides?scope=past" to { MockResponse().setBody(pastRide) },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                val nav = rememberNavController()
                FreshScreenSlot(screen) {
                    when (screen) {
                        0 -> EventDetailScreen(container, nav, id = 50L)
                        1 -> EventDetailScreen(container, nav, id = 51L)
                        2 -> CostsScreen(container, nav)
                        3 -> TripChatSheet(container, rideId = 89L, onDismiss = {})
                        4 -> SavedAddressesScreen(container, nav)
                        5 -> ProxyRidersScreen(container, nav)
                        6 -> HistoryScreen(container, nav)
                        else -> UnlockScreen(container)
                    }
                }
            }
        }

        // 0: Event RSVP sheet -> Need a ride -> Confirm
        waitForText("Outreach Day")
        compose.onNodeWithText(res(R.string.event_signup)).performScrollTo().performClick()
        waitForText(res(R.string.event_need_ride))
        compose.onNodeWithText(res(R.string.event_need_ride)).performClick()
        compose.onNodeWithText(res(R.string.common_confirm)).performClick()
        compose.waitUntil(5_000) {
            recordedCalls.any { it.startsWith("POST /api/events/50/signup") }
        }

        // 1: Signed-up Event Check-in Pass QR & Withdraw
        compose.runOnIdle { screen = 1 }
        waitForText("PASS51")
        compose.onNodeWithText("PASS51").performClick()
        waitForText(res(R.string.event_pass_hint))
        compose.onNodeWithText(res(R.string.common_close)).performClick()
        compose.waitUntil(5_000) {
            compose.onAllNodesWithText(res(R.string.event_pass_hint)).fetchSemanticsNodes().isEmpty()
        }
        compose.onNodeWithText(res(R.string.event_withdraw)).performScrollTo().performClick()
        waitForText(res(R.string.event_withdraw_confirm))
        compose.onNodeWithText(res(R.string.common_confirm)).performClick()
        compose.waitUntil(5_000) {
            recordedCalls.any { it.startsWith("POST /api/events/51/withdraw") }
        }

        // 2: Costs summary + Pay sheet validation
        compose.runOnIdle { screen = 2 }
        waitForText("RM 15.00")
        compose.onNodeWithText(res(R.string.costs_pay)).performClick()
        waitForText(res(R.string.costs_txn_ref))
        compose.onNodeWithText(res(R.string.costs_txn_ref)).performTextInput("12")
        compose.onNodeWithText(res(R.string.costs_submit_proof)).performClick()
        waitForText(res(R.string.costs_err_missing_photo))
        compose.onNodeWithText(res(R.string.common_cancel)).performClick()
        compose.waitUntil(5_000) {
            compose.onAllNodesWithText(res(R.string.costs_submit_proof)).fetchSemanticsNodes().isEmpty()
        }
        assertEquals(R.string.costs_err_ref_short, CostsViewModel.validateProofInput(true, "12"))
        assertNull(CostsViewModel.validateProofInput(true, "TXN12345"))

        // 3: TripChatSheet active send
        compose.runOnIdle { screen = 3 }
        waitForText("Heading out now")
        compose.onNodeWithText(res(R.string.chat_preset_at_gate)).performClick()
        compose.waitUntil(5_000) {
            recordedCalls.any { it.startsWith("POST /api/rides/89/messages") }
        }

        // 4: SavedAddressesScreen add sheet
        compose.runOnIdle { screen = 4 }
        waitForText("Home")
        compose.onNodeWithContentDescription(res(R.string.saved_addresses_add)).performClick()
        waitForText(res(R.string.saved_addresses_street))
        compose.onNodeWithText(res(R.string.common_cancel)).performClick()
        compose.waitUntil(5_000) {
            compose.onAllNodesWithText(res(R.string.saved_addresses_street)).fetchSemanticsNodes().isEmpty()
        }

        // 5: ProxyRidersScreen add sheet
        compose.runOnIdle { screen = 5 }
        waitForText("Uncle Tan")
        compose.onNodeWithContentDescription(res(R.string.proxy_riders_add)).performClick()
        waitForText(res(R.string.ride_new_rider_name))
        compose.onNodeWithText(res(R.string.common_cancel)).performClick()
        compose.waitUntil(5_000) {
            compose.onAllNodesWithText(res(R.string.ride_new_rider_name)).fetchSemanticsNodes().isEmpty()
        }

        // 6: HistoryScreen past ride
        compose.runOnIdle { screen = 6 }
        waitForText("Completed Ride Alpha", substring = true)

        // 7: UnlockScreen TOTP + Backup code toggle
        compose.runOnIdle { screen = 7 }
        waitForText(res(R.string.unlock_title))
        compose.onNodeWithText(res(R.string.unlock_backup)).performClick()
        compose.onNodeWithText(res(R.string.unlock_code)).assertIsDisplayed()
    }

    @Test
    fun darkThemeVisualSmokeAcrossKeyScreens() {
        var idx by mutableIntStateOf(0)
        val liveRide = """{"id":88,"status":"in_progress","pickup_at":"2026-11-26 09:00:00","area":{"id":1,"name":"PJ Section 14"},"destination":"Church Hall","seats":1,"trip_id":99,"passenger":{"id":1,"name":"Role Tester"},"viewer":{"is_passenger":true,"is_driver":false,"is_staff":false}}"""
        val notifItem = """{"items":[{"id":1,"title":"Dark Mode Alert","body":"Look sharp","link":"/home","read":false,"category":"ride","created_at":"2026-11-26 08:00:00"}],"next_before":null,"unread":1}"""
        val shareJson = """[{"id":30,"settlement_id":3,"title":"Fuel Nov","event_title":"Worship","amount_cents":1250,"status":"unpaid","payee_name":"Finance"}]"""

        setupSignedInDispatcher(
            routes = mapOf(
                "/api/auth/dev-users" to { MockResponse().setBody("[]") },
                "/api/rides/88/location" to { MockResponse().setBody("""{"sharing":true,"lat":3.1073,"lng":101.6067}""") },
                "/api/rides/88" to { MockResponse().setBody(liveRide) },
                "/api/rides" to { MockResponse().setBody("[$liveRide]") },
                "/api/notifications/preferences" to { MockResponse().setBody("""{"push_opt_in":true,"whatsapp_opt_in":false}""") },
                "/api/notifications" to { MockResponse().setBody(notifItem) },
                "/api/me/shares" to { MockResponse().setBody(shareJson) },
                "/api/events" to { MockResponse().setBody("""[{"id":50,"title":"Night Service","starts_at":"2026-12-20 19:00:00","location":"Sanctuary","status":"published","going":5}]""") },
            ),
        )

        compose.setContent {
            GraceRideTheme(darkTheme = true) {
                val nav = rememberNavController()
                FreshScreenSlot(idx) {
                    when (idx) {
                        0 -> LoginScreen(container)
                        1 -> TermsScreen(container)
                        2 -> HomeScreen(container, nav)
                        3 -> EventsScreen(container, nav)
                        4 -> RidesScreen(container, nav)
                        5 -> DriveHubScreen(container, nav)
                        6 -> RideDetailScreen(container, nav, id = 88L)
                        7 -> CostsScreen(container, nav)
                        8 -> NotificationsScreen(container, nav)
                        9 -> ProfileScreen(container, nav)
                        10 -> AppearanceSettingsScreen(container, nav)
                        else -> DriverSettingsScreen(container, nav)
                    }
                }
            }
        }

        waitForText("Grace Ride")
        compose.runOnIdle { idx = 1 }
        waitForText(res(R.string.terms_title))
        compose.runOnIdle { idx = 2 }
        waitForText("Hello, Role Tester", substring = true)
        compose.runOnIdle { idx = 3 }
        waitForText("Night Service")
        compose.runOnIdle { idx = 4 }
        waitForText("Church Hall", substring = true)
        compose.runOnIdle { idx = 5 }
        waitForText(res(R.string.drive_hub_title))
        compose.runOnIdle { idx = 6 }
        waitForText(res(R.string.live_location_title))
        compose.runOnIdle { idx = 7 }
        waitForText("RM 12.50")
        compose.runOnIdle { idx = 8 }
        waitForText("Dark Mode Alert")
        compose.runOnIdle { idx = 9 }
        waitForText(res(R.string.profile_personal))
        compose.runOnIdle { idx = 10 }
        waitForText(res(R.string.profile_theme_dark))
        compose.runOnIdle { idx = 11 }
        waitForText("WAA 1234")
    }
}
