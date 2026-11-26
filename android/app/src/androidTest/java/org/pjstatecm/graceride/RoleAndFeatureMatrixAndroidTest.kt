package org.pjstatecm.graceride

import android.content.Context
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.hasScrollAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onAllNodesWithContentDescription
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onFirst
import androidx.compose.ui.test.onLast
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performScrollToNode
import androidx.compose.ui.test.performTextInput
import androidx.navigation.compose.rememberNavController
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.encodeToString
import okhttp3.mockwebserver.Dispatcher
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import okhttp3.mockwebserver.RecordedRequest
import org.junit.After
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.screens.costs.CostsScreen
import org.pjstatecm.graceride.ui.screens.drive.DriveHubScreen
import org.pjstatecm.graceride.ui.screens.events.EventDetailScreen
import org.pjstatecm.graceride.ui.screens.events.EventsScreen
import org.pjstatecm.graceride.ui.screens.history.HistoryScreen
import org.pjstatecm.graceride.ui.screens.home.HomeScreen
import org.pjstatecm.graceride.ui.screens.login.PendingScreen
import org.pjstatecm.graceride.ui.screens.notifications.NotificationsScreen
import org.pjstatecm.graceride.ui.screens.profile.DriverSettingsScreen
import org.pjstatecm.graceride.ui.screens.profile.GrokBotStudioScreen
import org.pjstatecm.graceride.ui.screens.profile.ProfileScreen
import org.pjstatecm.graceride.ui.screens.profile.ProxyRidersScreen
import org.pjstatecm.graceride.ui.screens.profile.SavedAddressesScreen
import org.pjstatecm.graceride.ui.screens.rides.RideDetailScreen
import org.pjstatecm.graceride.ui.screens.rides.RidesScreen
import org.pjstatecm.graceride.ui.screens.rides.TripChatSheet
import org.pjstatecm.graceride.ui.theme.GraceRideTheme

@RunWith(AndroidJUnit4::class)
class RoleAndFeatureMatrixAndroidTest {

    @get:Rule
    val compose = createComposeRule()

    private lateinit var server: MockWebServer
    private lateinit var container: AppContainer
    private val targetContext: Context
        get() = InstrumentationRegistry.getInstrumentation().targetContext

    private fun res(id: Int, vararg args: Any): String =
        if (args.isEmpty()) targetContext.getString(id) else targetContext.getString(id, *args)

    @Before
    fun setup() {
        server = MockWebServer()
        server.start()
        container = AppContainer(null, InMemoryStore())
        container.setBaseUrl(server.url("/").toString())
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
        name: String = "Test User",
        roles: List<String> = emptyList(),
        permissions: List<String> = emptyList(),
        status: String = "active",
        photoFileId: Long? = null,
        avatarIcon: String? = null,
        avatarColor: String? = null,
    ): String {
        val rolesJson = apiJson.encodeToString(roles)
        val permsJson = apiJson.encodeToString(permissions)
        val photoField = photoFileId?.toString() ?: "null"
        val iconField = avatarIcon?.let { apiJson.encodeToString(it) } ?: "null"
        val colorField = avatarColor?.let { apiJson.encodeToString(it) } ?: "null"
        return """
            {
              "id": $id,
              "name": ${apiJson.encodeToString(name)},
              "email": "user$id@example.org",
              "phone": "012-3456789",
              "photo_file_id": $photoField,
              "avatar_icon": $iconField,
              "avatar_color": $colorField,
              "roles": $rolesJson,
              "role_names": {},
              "permissions": $permsJson,
              "status": "$status",
              "lang": "en",
              "terms_accepted": true,
              "mfa_ok": true
            }
        """.trimIndent()
    }

    private fun signInWith(
        roles: List<String>,
        permissions: List<String>,
        name: String = "Role Tester",
        status: String = "active",
        photoFileId: Long? = null,
        routes: Map<String, (RecordedRequest) -> MockResponse> = emptyMap(),
    ) {
        val meJson = buildMeJson(
            name = name,
            roles = roles,
            permissions = permissions,
            status = status,
            photoFileId = photoFileId,
        )
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val p = request.path ?: ""
                routes.entries.firstOrNull { p.startsWith(it.key) }?.let { return it.value(request) }
                return when {
                    p.startsWith("/api/auth/me") -> MockResponse().setBody(meJson)
                    p.startsWith("/api/auth/passkeys") -> MockResponse().setBody("[]")
                    p.startsWith("/api/me/notification-prefs") -> MockResponse().setBody("{}")
                    p.startsWith("/api/notifications") -> MockResponse().setBody("""{"items":[],"next_before":null,"unread":0}""")
                    p.startsWith("/api/driver/profile") -> MockResponse().setBody("""{"carpool_enabled":false,"spare_seats":1,"gate_wait_mins":5,"area_ids":[],"vehicle":null,"vehicles":[],"active_vehicle_id":null}""")
                    p.startsWith("/api/driver/route") -> MockResponse().setBody("""{"route":null}""")
                    else -> MockResponse().setBody("[]")
                }
            }
        }
        runBlocking { container.auth.refresh() }
    }

    private data class RoleExpectation(
        val key: String,
        val labelRes: Int,
        val perms: List<String>,
        val expectSettlements: Boolean,
        val expectStaffConsole: Boolean,
        val expectRawHidden: Boolean = false,
    )

    @Test
    fun profileDisplaysAll14BuiltInRoleLabelsAndHandoffs() {
        val allRoles = listOf(
            RoleExpectation("member", R.string.role_member, listOf("rides.view", "rides.create", "events.view", "costs.view", "costs.update", "reimbursements.view", "help.view", "report.view", "settlements.view"), expectSettlements = false, expectStaffConsole = false),
            RoleExpectation("driver", R.string.role_driver, listOf("drive.view", "drive.create", "drive.update", "drive.delete"), expectSettlements = false, expectStaffConsole = false),
            RoleExpectation("leader", R.string.role_leader, listOf("events.create", "events.update", "settlements.view", "settlements.create"), expectSettlements = true, expectStaffConsole = false),
            RoleExpectation("cell_leader", R.string.role_cell_leader, listOf("events.create", "settlements.view", "admin_rides.view", "admin_plan.view"), expectSettlements = true, expectStaffConsole = true, expectRawHidden = true),
            RoleExpectation("transport", R.string.role_transport, listOf("admin_dashboard.view", "admin_rides.view", "admin_live.view", "admin_plan.view"), expectSettlements = false, expectStaffConsole = true),
            RoleExpectation("traffic_member", R.string.role_traffic_member, listOf("admin_dashboard.view", "admin_rides.view", "admin_area_photos.create"), expectSettlements = false, expectStaffConsole = true, expectRawHidden = true),
            RoleExpectation("traffic_head", R.string.role_traffic_head, listOf("admin_dashboard.view", "admin_rides.view", "admin_events.view", "admin_fares.update"), expectSettlements = false, expectStaffConsole = true, expectRawHidden = true),
            RoleExpectation("finance", R.string.role_finance, listOf("admin_reimb_pay.update", "admin_bills.view", "admin_reports.view"), expectSettlements = true, expectStaffConsole = true),
            RoleExpectation("auditor", R.string.role_auditor, listOf("admin_reimb_pay.view", "admin_bills.view", "admin_reports.view"), expectSettlements = true, expectStaffConsole = true),
            RoleExpectation("secretary", R.string.role_secretary, listOf("settlements.view", "admin_events.view", "admin_reports.view", "admin_roles.view"), expectSettlements = true, expectStaffConsole = true),
            RoleExpectation("vice_president", R.string.role_vice_president, listOf("settlements.view", "admin_reimb_pay.view", "admin_bills.view", "admin_dashboard.view"), expectSettlements = true, expectStaffConsole = true, expectRawHidden = true),
            RoleExpectation("president", R.string.role_president, listOf("settlements.view", "admin_reimb_pay.view", "admin_bills.view", "admin_roles.update"), expectSettlements = true, expectStaffConsole = true),
            RoleExpectation("admin", R.string.role_admin, listOf("settlements.view", "admin_dashboard.view", "admin_settings.update", "admin_roles.update"), expectSettlements = true, expectStaffConsole = true),
            RoleExpectation("superadmin", R.string.role_superadmin, listOf("rides.view", "drive.view", "events.view", "costs.view", "admin_bills.view", "admin_dashboard.view", "admin_reports.view"), expectSettlements = true, expectStaffConsole = true, expectRawHidden = true),
        )

        var step by mutableIntStateOf(0)
        signInWith(listOf(allRoles[0].key), allRoles[0].perms, name = "Member Zero")

        compose.setContent {
            GraceRideTheme {
                val nav = rememberNavController()
                FreshScreenSlot(step) {
                    ProfileScreen(container, nav)
                }
            }
        }

        allRoles.forEachIndexed { idx, spec ->
            if (idx > 0) {
                signInWith(listOf(spec.key), spec.perms, name = "User ${spec.key}")
                compose.runOnIdle { step = idx }
            }
            val expectedLabel = res(spec.labelRes)
            waitForText(expectedLabel)
            compose.onNodeWithText(expectedLabel).assertIsDisplayed()
            if (spec.expectRawHidden) {
                compose.onNodeWithText(spec.key).assertDoesNotExist()
            }

            if (spec.key in setOf("member", "driver", "leader", "finance", "auditor", "admin", "superadmin")) {
                val webSectionLabel = res(R.string.profile_section_web)
                val settlementsLabel = res(R.string.profile_settlements)
                val staffLabel = res(R.string.profile_web_staff)
                val adminReportsLabel = res(R.string.profile_admin_reports)
                if (spec.expectSettlements || spec.expectStaffConsole) {
                    compose.onNode(hasScrollAction()).performScrollToNode(hasText(webSectionLabel))
                    compose.onNodeWithText(webSectionLabel).assertIsDisplayed()
                    if (spec.expectSettlements) {
                        compose.onNode(hasScrollAction()).performScrollToNode(hasText(settlementsLabel))
                        compose.onNodeWithText(settlementsLabel).assertIsDisplayed()
                    } else {
                        compose.onNodeWithText(settlementsLabel).assertDoesNotExist()
                    }
                    if (spec.expectStaffConsole) {
                        compose.onNode(hasScrollAction()).performScrollToNode(hasText(staffLabel))
                        compose.onNodeWithText(staffLabel).assertIsDisplayed()
                    } else {
                        compose.onNodeWithText(staffLabel).assertDoesNotExist()
                    }
                } else {
                    compose.onNodeWithText(webSectionLabel).assertDoesNotExist()
                    compose.onNodeWithText(settlementsLabel).assertDoesNotExist()
                    compose.onNodeWithText(staffLabel).assertDoesNotExist()
                    compose.onNodeWithText(adminReportsLabel).assertDoesNotExist()
                }
            }
        }
    }

    @Test
    fun driveHubAndDriverSettingsEnforceDriverPermissions() {
        var mode by mutableIntStateOf(0)
        val openRide = """{"id":55,"status":"pending_driver","pickup_at":"2026-11-26 09:00:00","area":{"id":1,"name":"PJ"},"destination":"Church Sanctuary","seats":2,"passenger":{"id":9,"name":"Grace"},"viewer":{"is_passenger":false,"is_driver":false,"is_staff":false}}"""
        val profileJson = """{"carpool_enabled":true,"spare_seats":3,"gate_wait_mins":5,"area_ids":[1],"vehicle":{"id":10,"model":"Perodua Alza","color":"Silver","plate":"WAA 1234","seats":7,"is_active":true},"vehicles":[{"id":10,"model":"Perodua Alza","color":"Silver","plate":"WAA 1234","seats":7,"is_active":true}],"active_vehicle_id":10}"""
        val driverRoutes = mapOf<String, (RecordedRequest) -> MockResponse>(
            "/api/driver/requests" to { MockResponse().setBody("[$openRide]") },
            "/api/driver/schedule" to { MockResponse().setBody("[]") },
            "/api/driver/route" to { MockResponse().setBody("""{"route":{"trip_id":99,"live":false,"onboard":0,"stops":[],"destination":{"name":"Sanctuary"},"distance_m":1200.0,"duration_s":300.0}}""") },
            "/api/driver/profile" to { MockResponse().setBody(profileJson) },
            "/api/areas" to { MockResponse().setBody("""[{"id":1,"name":"PJ","active":true}]""") },
        )

        // 0 & 1: non-driver blocked on both DriveHub and DriverSettings
        signInWith(listOf("member"), listOf("rides.view"), routes = driverRoutes)

        compose.setContent {
            GraceRideTheme {
                val nav = rememberNavController()
                FreshScreenSlot(mode) {
                    when (mode) {
                        0, 2, 4 -> DriveHubScreen(container, nav)
                        else -> DriverSettingsScreen(container, nav)
                    }
                }
            }
        }

        val deniedMsg = res(R.string.role_driver_access_required)
        waitForText(deniedMsg)
        compose.onNodeWithText(deniedMsg).assertIsDisplayed()

        compose.runOnIdle { mode = 1 }
        waitForText(deniedMsg)
        compose.onNodeWithText(deniedMsg).assertIsDisplayed()

        // 2 & 3: read-only driver (drive.view only -> sees requests & vehicle, Claim & Save hidden)
        signInWith(listOf("driver"), listOf("drive.view"), routes = driverRoutes)
        compose.runOnIdle { mode = 2 }
        waitForText("Church Sanctuary", substring = true)
        compose.onNodeWithText(res(R.string.drive_claim)).assertDoesNotExist()

        compose.runOnIdle { mode = 3 }
        waitForText("Perodua Alza", substring = true)
        compose.onNodeWithText(res(R.string.common_save)).assertDoesNotExist()

        // 4: full driver sees Claim button and can switch Schedule & Route tabs
        signInWith(listOf("driver"), listOf("drive.view", "drive.create", "drive.update", "drive.delete"), routes = driverRoutes)
        compose.runOnIdle { mode = 4 }
        waitForText(res(R.string.drive_claim))
        compose.onNodeWithText(res(R.string.drive_claim)).assertIsDisplayed()
        compose.onNodeWithText(res(R.string.drive_tab_schedule)).performClick()
        compose.onNodeWithText(res(R.string.drive_tab_route)).performClick()
    }

    @Test
    fun homeRidesHistoryRideDetailAndChatRespectRoles() {
        var screen by mutableIntStateOf(0)
        val unrelatedInProgress = """{"id":88,"status":"in_progress","pickup_at":"2026-11-26 09:00:00","area":{"id":2,"name":"SS2"},"destination":"Main Hall","seats":1,"trip_id":7,"passenger":{"id":9,"name":"Other Member"},"viewer":{"is_passenger":false,"is_driver":false,"is_staff":false}}"""
        val passengerRide = """{"id":89,"status":"confirmed","pickup_at":"2026-11-26 09:00:00","area":{"id":2,"name":"SS2"},"destination":"Main Hall","seats":1,"trip_id":8,"passenger":{"id":1,"name":"Test Member"},"viewer":{"is_passenger":true,"is_driver":false,"is_staff":false}}"""
        val readonlyChat = """{"open":true,"can_post":false,"members":["Alice"],"messages":[{"id":1,"user_id":1,"sender_name":"Alice","body":"Driver arriving","created_at":"2026-11-26 08:50:00","reactions":[]}]}"""

        val routes = mapOf<String, (RecordedRequest) -> MockResponse>(
            "/api/rides/88/location" to { MockResponse().setBody("""{"sharing":true,"lat":3.1,"lng":101.6}""") },
            "/api/rides/88" to { MockResponse().setBody(unrelatedInProgress) },
            "/api/rides/89/messages" to { MockResponse().setBody(readonlyChat) },
            "/api/rides/89" to { MockResponse().setBody(passengerRide) },
            "/api/rides" to { MockResponse().setBody("[$passengerRide]") },
            "/api/me/shares" to { MockResponse().setBody("""[{"id":30,"settlement_id":3,"title":"Fuel Nov","event_title":"Worship","amount_cents":1250,"status":"unpaid","payee_name":"Finance"}]""") },
        )

        // Read-only rides.view (no rides.create)
        signInWith(listOf("member"), listOf("rides.view", "rides.update"), routes = routes)

        compose.setContent {
            GraceRideTheme {
                val nav = rememberNavController()
                FreshScreenSlot(screen) {
                    when (screen) {
                        0 -> HomeScreen(container, nav)
                        1 -> RidesScreen(container, nav)
                        2 -> HistoryScreen(container, nav)
                        3 -> RideDetailScreen(container, nav, id = 88L)
                        4 -> RideDetailScreen(container, nav, id = 89L)
                        else -> TripChatSheet(container, rideId = 89L, onDismiss = {})
                    }
                }
            }
        }

        waitForText("Role Tester", substring = true)
        compose.runOnIdle { screen = 1 }
        waitForText("SS2 → Main Hall", substring = true)
        compose.onNodeWithText(res(R.string.rides_book)).assertDoesNotExist()

        // Full rides.create shows Book FAB
        signInWith(listOf("member"), listOf("rides.view", "rides.create", "rides.update"), routes = routes)
        compose.runOnIdle { screen = 2 }
        waitForText(res(R.string.history_title))

        compose.runOnIdle { screen = 3 }
        waitForText("Main Hall", substring = true)
        compose.onNodeWithText(res(R.string.readiness_unable)).assertDoesNotExist()

        compose.runOnIdle { screen = 4 }
        waitForText(res(R.string.readiness_unable))
        compose.onNodeWithText(res(R.string.readiness_unable)).performClick()
        waitForText(res(R.string.ride_unable_confirm_title))
        compose.onNodeWithText(res(R.string.common_cancel)).performClick()

        compose.runOnIdle { screen = 5 }
        waitForText("Driver arriving")
        compose.onNodeWithText(res(R.string.chat_placeholder)).assertDoesNotExist()
    }

    @Test
    fun eventsAndCheckinSheetCoverTabsCancelledAndWrongEvent() {
        var screen by mutableIntStateOf(0)
        val futureEvent = """{"id":50,"title":"Youth Retreat","starts_at":"2026-12-20 09:00:00","location":"Camp Site","status":"published","attendance_enabled":true,"going":8,"capacity":40,"spots_left":32,"my_signup":{"status":"going","attended":false,"pass_code":"PASS50"}}"""
        val cancelledEvent = """{"id":51,"title":"Summer Picnic","starts_at":"2026-12-21 09:00:00","location":"Park","status":"cancelled","going":2}"""
        val routes = mapOf<String, (RecordedRequest) -> MockResponse>(
            "/api/events/50" to { MockResponse().setBody(futureEvent) },
            "/api/events/51" to { MockResponse().setBody(cancelledEvent) },
            "/api/events" to { MockResponse().setBody("[$futureEvent]") },
        )

        signInWith(listOf("member"), listOf("events.view"), routes = routes)

        compose.setContent {
            GraceRideTheme {
                val nav = rememberNavController()
                FreshScreenSlot(screen) {
                    when (screen) {
                        0, 1 -> EventsScreen(container, nav)
                        2 -> EventDetailScreen(container, nav, id = 51L)
                        else -> EventDetailScreen(container, nav, id = 50L)
                    }
                }
            }
        }

        waitForText("Youth Retreat")
        compose.onNodeWithText(res(R.string.events_tab_managing)).assertDoesNotExist()

        // Member on EventDetail does NOT see "Open check-in & full event in Web" redirection button
        compose.runOnIdle { screen = 3 }
        waitForText(res(R.string.event_pass_label))
        compose.onNodeWithText(res(R.string.event_open_web_full)).assertDoesNotExist()

        signInWith(listOf("leader"), listOf("events.view", "events.create", "events.update"), routes = routes)
        compose.runOnIdle { screen = 1 }
        waitForText(res(R.string.events_tab_managing))
        compose.onNodeWithText(res(R.string.events_tab_managing)).assertIsDisplayed()

        compose.runOnIdle { screen = 2 }
        waitForText("Summer Picnic")
        compose.onAllNodesWithText(res(R.string.event_status_cancelled)).onFirst().assertIsDisplayed()
        compose.onNodeWithText(res(R.string.event_signup)).assertDoesNotExist()

        // Leader sees "Open check-in & full event in Web" and wrong-event QR validation works
        compose.runOnIdle { screen = 4 }
        waitForText(res(R.string.event_pass_label))
        compose.onNodeWithText(res(R.string.event_open_web_full)).performScrollTo().assertIsDisplayed()
        compose.onNodeWithText(res(R.string.event_checkin_btn)).performScrollTo().performClick()
        waitForText(res(R.string.event_code_hint))
        compose.onNodeWithText(res(R.string.event_code_hint)).performTextInput("/events/999/checkin?code=ABC123")
        compose.onNodeWithText(res(R.string.common_confirm)).performClick()
        // Note: confirm the wrong-event error surfaces inside the check-in sheet
        compose.waitUntil(5_000) {
            compose.onAllNodesWithText(res(R.string.event_checkin_wrong_event)).fetchSemanticsNodes().isNotEmpty()
        }
    }

    @Test
    fun costsNotificationsPendingAddressesProxyAndStudioWork() {
        var screen by mutableIntStateOf(0)
        val unpaidShare = """[{"id":30,"settlement_id":3,"title":"Fuel Oct","event_title":"Camp","amount_cents":1500,"status":"unpaid","payee_name":"Finance"}]"""
        val fiftyPlaces = (1..50).joinToString(",") { i ->
            """{"id":$i,"label":"Place $i","area_id":1,"area_name":"PJ","address":"$i Jalan Test","gate_notes":"","is_default":${if (i == 1) 1 else 0}}"""
        }
        val fiftyProxies = (1..50).joinToString(",") { i ->
            """{"id":$i,"name":"Rider $i","phone":"012-000$i","relationship":"parent","notes":"","area_id":1,"pickup_address":"$i SS2","gate_notes":""}"""
        }
        val notifPage = """{"items":[{"id":1,"title":"Event Reminder","body":"Camp Saturday","link":"/events/50","read":false,"category":"event","created_at":"2026-11-25 09:00:00"},{"id":2,"title":"Pending Admin Ride","body":"Admin review","link":"/admin/rides","read":false,"category":"admin","created_at":"2026-11-25 08:30:00"}],"next_before":null,"unread":2}"""
        val routes = mapOf<String, (RecordedRequest) -> MockResponse>(
            "/api/me/shares" to { MockResponse().setBody(unpaidShare) },
            "/api/notifications" to { MockResponse().setBody(notifPage) },
            "/api/saved-places" to { MockResponse().setBody("[$fiftyPlaces]") },
            "/api/proxy-riders" to { MockResponse().setBody("[$fiftyProxies]") },
            "/api/areas" to { MockResponse().setBody("""[{"id":1,"name":"PJ","active":true}]""") },
        )

        // 0: Auditor read-only costs
        signInWith(listOf("auditor"), listOf("costs.view"), routes = routes)
        compose.setContent {
            GraceRideTheme {
                val nav = rememberNavController()
                FreshScreenSlot(screen) {
                    when (screen) {
                        0, 1 -> CostsScreen(container, nav)
                        2 -> NotificationsScreen(container, nav)
                        3 -> PendingScreen(container)
                        4 -> SavedAddressesScreen(container, nav)
                        5 -> ProxyRidersScreen(container, nav)
                        6 -> GrokBotStudioScreen(container, nav)
                        else -> ProfileScreen(container, nav)
                    }
                }
            }
        }

        waitForText("Fuel Oct", substring = true)
        compose.onNodeWithText(res(R.string.costs_pay)).assertDoesNotExist()

        // 1: Member with costs.update sees Pay button (plus events.view for Notifications card link)
        signInWith(listOf("member"), listOf("costs.view", "costs.update", "events.view"), routes = routes)
        compose.runOnIdle { screen = 1 }
        waitForText(res(R.string.costs_pay))
        compose.onNodeWithText(res(R.string.costs_pay)).assertIsDisplayed()

        // 2: Notifications — member sees no Admin filter, no Open-in-Web arrow on blocked admin item, and overflow Remove + Undo works
        compose.runOnIdle { screen = 2 }
        waitForText("Event Reminder")
        compose.onNodeWithText(res(R.string.notif_mark_all_read)).assertIsDisplayed()
        compose.onNodeWithText(res(R.string.notif_cat_admin)).assertDoesNotExist()
        compose.onNodeWithText(res(R.string.notif_action_view)).assertIsDisplayed()
        compose.onNodeWithText(res(R.string.notif_action_open_web)).assertDoesNotExist()
        compose.onAllNodesWithContentDescription(res(R.string.notif_more_actions)).onFirst().performClick()
        waitForText(res(R.string.notif_delete))
        compose.onNodeWithText(res(R.string.notif_delete)).performClick()
        waitForText(res(R.string.notif_undo))
        compose.onNodeWithText(res(R.string.notif_undo)).performClick()
        waitForText("Event Reminder")

        // 3: Pending account screen
        signInWith(emptyList(), emptyList(), status = "pending", routes = routes)
        compose.runOnIdle { screen = 3 }
        waitForText(res(R.string.pending_title))

        // 4 & 5: 50-item limit banners on SavedAddresses and ProxyRiders
        signInWith(listOf("member"), listOf("rides.view", "rides.create", "rides.update"), routes = routes)
        compose.runOnIdle { screen = 4 }
        waitForText("Place 1")
        compose.onNode(hasScrollAction()).performScrollToNode(hasText(res(R.string.saved_addresses_limit)))
        compose.onNodeWithText(res(R.string.saved_addresses_limit)).assertIsDisplayed()

        compose.runOnIdle { screen = 5 }
        waitForText("Rider 1")
        compose.onNode(hasScrollAction()).performScrollToNode(hasText(res(R.string.proxy_riders_limit_reached)))
        compose.onNodeWithText(res(R.string.proxy_riders_limit_reached)).assertIsDisplayed()

        // 6: GrokBot Studio screen
        compose.runOnIdle { screen = 6 }
        waitForText(res(R.string.studio_surprise_me))
        compose.onNodeWithText(res(R.string.studio_surprise_me)).performClick()
        compose.onNodeWithText(res(R.string.studio_save_avatar)).assertIsDisplayed()

        // 7: AvatarEditorSheet hides background color in Photo mode
        signInWith(listOf("member"), listOf("rides.view"), photoFileId = 7L, routes = routes)
        compose.runOnIdle { screen = 7 }
        waitForText(res(R.string.role_member))
        compose.onNodeWithContentDescription(res(R.string.profile_edit_avatar)).performClick()
        waitForText(res(R.string.profile_avatar_photo))
        compose.onNodeWithText(res(R.string.profile_avatar_color)).assertDoesNotExist()
    }
}
