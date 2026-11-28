package org.pjstatecm.graceride

import android.content.Context
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.requiredSize
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.getBoundsInRoot
import androidx.compose.ui.test.hasAnyAncestor
import androidx.compose.ui.test.hasScrollAction
import androidx.compose.ui.test.hasSetTextAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.isDialog
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performScrollToNode
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.performTextReplacement
import androidx.compose.ui.unit.dp
import androidx.navigation.compose.rememberNavController
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.encodeToString
import okhttp3.mockwebserver.Dispatcher
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import okhttp3.mockwebserver.RecordedRequest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.GraceRideAppRoot
import org.pjstatecm.graceride.ui.NotifLinkRouter
import org.pjstatecm.graceride.ui.components.LocationPinPickerSheet
import org.pjstatecm.graceride.ui.components.QuickJumpSheet
import org.pjstatecm.graceride.ui.screens.admin.AdminSettingsScreen
import org.pjstatecm.graceride.ui.screens.login.LoginScreen
import org.pjstatecm.graceride.ui.screens.profile.SecuritySettingsScreen
import org.pjstatecm.graceride.ui.screens.rides.TimePickerDialog
import org.pjstatecm.graceride.ui.screens.rides.TripChatSheet
import org.pjstatecm.graceride.ui.screens.traffic.TrafficHubScreen
import org.pjstatecm.graceride.ui.theme.GraceRideTheme
import java.util.Collections
import kotlin.math.abs

/**
 * Instrumentation regression checks for foldable / responsive layouts and interactive sheets
 * (Adaptive NavigationRail vs FloatingGlassBottomBar, QuickJumpSheet, LocationPinPickerSheet,
 * TripChatSheet, and LoginScreen recovery dialog).
 */
@RunWith(AndroidJUnit4::class)
class FoldableLayoutAndFlowsAndroidTest {

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

    private fun waitForText(text: String, substring: Boolean = false, timeoutMs: Long = 10_000) {
        compose.waitUntil(timeoutMs) {
            compose.onAllNodesWithText(text, substring = substring).fetchSemanticsNodes().isNotEmpty()
        }
    }

    private fun adminPerms(): List<String> = listOf(
        "rides.view",
        "rides.create",
        "rides.update",
        "rides.delete",
        "drive.view",
        "drive.create",
        "drive.update",
        "events.view",
        "events.create",
        "events.publish",
        "costs.view",
        "costs.update",
        "reimbursements.view",
        "reimbursements.create",
        "admin_plan.view",
        "admin_plan.update",
        "admin_rides.view",
        "admin_rides.update",
        "admin_live.view",
        "admin_area_photos.view",
        "admin_fares.view",
        "admin_areas.view",
        "admin_dashboard.view",
        "admin_members.view",
        "admin_invites.view",
        "admin_invites.create",
        "admin_recovery.view",
        "admin_announcements.view",
        "admin_roles.view",
        "admin_audit.view",
        "admin_settings.view",
    )

    private fun buildMeJson(
        id: Long = 1L,
        name: String = "Foldable Tester",
        roles: List<String> = listOf("superadmin", "admin", "driver", "traffic_head"),
        permissions: List<String> = adminPerms(),
    ): String {
        val rolesJson = apiJson.encodeToString(roles)
        val permsJson = apiJson.encodeToString(permissions)
        return """
            {
              "id": $id,
              "name": ${apiJson.encodeToString(name)},
              "email": "fold$id@example.org",
              "phone": "012-3456789",
              "roles": $rolesJson,
              "role_names": {},
              "permissions": $permsJson,
              "status": "active",
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
                    path.startsWith("/api/auth/passkeys") -> MockResponse().setBody("[]")
                    path.startsWith("/api/auth/biometrics") -> MockResponse().setBody("[]")
                    path.startsWith("/api/notifications/preferences") ->
                        MockResponse().setBody("""{"push_opt_in":true,"whatsapp_opt_in":false,"readiness_prompt_mins":45,"driver_remind_mins":60,"muted_categories":[],"quiet_hours_start":"","quiet_hours_end":"","hide_lock_details":false}""")
                    path.startsWith("/api/notifications") -> MockResponse().setBody("""{"items":[],"next_before":null,"unread":0}""")
                    path.startsWith("/api/announcements") -> MockResponse().setBody("[]")
                    path.startsWith("/api/areas") -> MockResponse().setBody("""[{"id":1,"name":"PJ Section 14","active":true}]""")
                    path.startsWith("/api/driver/profile") -> MockResponse().setBody("""{"carpool_enabled":true,"spare_seats":3,"gate_wait_mins":5,"area_ids":[1]}""")
                    path.startsWith("/api/driver/route") -> MockResponse().setBody("""{"route":null}""")
                    path.startsWith("/api/driver/requests") -> MockResponse().setBody("[]")
                    path.startsWith("/api/driver/schedule") -> MockResponse().setBody("[]")
                    path.startsWith("/api/driver/drivers") || path.startsWith("/api/admin/drivers") -> MockResponse().setBody("[]")
                    path.startsWith("/api/admin/dashboard") -> MockResponse().setBody("""{"pending_approval":0,"no_driver":0,"pending_plan_drafts":0,"pending_ride_drafts":0}""")
                    path.startsWith("/api/admin/traffic/plans") -> MockResponse().setBody("[]")
                    path.startsWith("/api/admin/rides") -> MockResponse().setBody("""{"items":[],"total":0,"page":1,"page_size":50,"total_pages":1}""")
                    path.startsWith("/api/admin/ride-drafts") -> MockResponse().setBody("[]")
                    path.startsWith("/api/admin/live") -> MockResponse().setBody("""{"drivers":[]}""")
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
    fun adaptiveAppRootNavSemanticsAndDetailChrome() {
        val nextRideJson = """{"id":40,"status":"confirmed","pickup_at":"2026-11-27 09:00:00","area":{"id":1,"name":"PJ Section 14"},"destination":"Church Sanctuary","seats":1,"passenger":{"id":1,"name":"Foldable Tester"},"viewer":{"is_passenger":true,"is_driver":false,"is_staff":false}}"""
        setupSignedInDispatcher(
            routes = mapOf(
                "/api/events" to { MockResponse().setBody("""[{"id":50,"title":"Sunday Gathering","starts_at":"2026-12-20 09:00:00","location":"Church Hall","status":"published"}]""") },
                "/api/rides" to { MockResponse().setBody("[$nextRideJson]") },
                "/api/admin/traffic/plans" to { MockResponse().setBody("""[{"id":5,"title":"Plan Sunday","starts_at":"2026-11-29 09:00:00","location":"Main Hall","ride_count":1,"unassigned_count":0,"pending_confirm_count":0,"visitor_count":0,"draft_count":0,"pending_draft_count":0,"returned_draft_count":0}]""") },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                GraceRideAppRoot(container = container)
            }
        }

        waitForText("Hello, Foldable Tester", substring = true)

        compose.onNodeWithContentDescription(res(R.string.nav_events)).performClick()
        waitForText("Sunday Gathering")

        compose.onNodeWithContentDescription(res(R.string.nav_traffic)).performClick()
        waitForText("Plan Sunday")

        compose.onNodeWithContentDescription(res(R.string.nav_profile)).performClick()
        waitForText(res(R.string.profile_personal))

        compose.onNodeWithContentDescription(res(R.string.nav_home)).performClick()
        waitForText("Hello, Foldable Tester", substring = true)
        compose.onNode(hasScrollAction()).performScrollToNode(hasText("Church Sanctuary", substring = true))
        waitForText("Church Sanctuary", substring = true)
        compose.onNode(hasScrollAction()).performScrollToNode(hasText("Hello, Foldable Tester", substring = true))

        // Detail route hides primary navigation bar/rail; Back restores it cleanly.
        compose.onNodeWithContentDescription(res(R.string.notif_title)).performClick()
        waitForText(res(R.string.notif_empty))
        compose.onNodeWithContentDescription(res(R.string.nav_events)).assertDoesNotExist()

        compose.onNodeWithContentDescription(res(R.string.a11y_back)).performClick()
        waitForText("Hello, Foldable Tester", substring = true)
        compose.onNodeWithContentDescription(res(R.string.nav_events)).assertIsDisplayed()
    }

    @Test
    fun liveFoldUnfoldSwitchesRailAndBottomBarPreservingTab() {
        setupSignedInDispatcher(
            routes = mapOf(
                "/api/events" to { MockResponse().setBody("""[{"id":50,"title":"Sunday Gathering","starts_at":"2026-12-20 09:00:00","location":"Church Hall","status":"published"}]""") },
                "/api/admin/traffic/plans" to { MockResponse().setBody("""[{"id":5,"title":"Plan Sunday","starts_at":"2026-11-29 09:00:00","location":"Main Hall","ride_count":1,"unassigned_count":0,"pending_confirm_count":0,"visitor_count":0,"draft_count":0,"pending_draft_count":0,"returned_draft_count":0}]""") },
            ),
        )
        var unfolded by mutableStateOf(true)

        compose.setContent {
            GraceRideTheme {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.TopStart) {
                    Box(Modifier.requiredSize(width = if (unfolded) 841.dp else 411.dp, height = 680.dp)) {
                        GraceRideAppRoot(container = container)
                    }
                }
            }
        }

        waitForText("Hello, Foldable Tester", substring = true)

        // Start unfolded -> NavigationRail on left; switch to Events tab.
        val railHomeBounds = compose.onNodeWithContentDescription(res(R.string.nav_home)).getBoundsInRoot()
        assertTrue("Expected NavigationRail on left when unfolded, was $railHomeBounds", railHomeBounds.left < 120.dp && railHomeBounds.top < 450.dp)

        compose.onNodeWithContentDescription(res(R.string.nav_events)).performClick()
        waitForText("Sunday Gathering")

        // Fold live -> must switch to FloatingGlassBottomBar at bottom while keeping Events active.
        compose.runOnUiThread { unfolded = false }
        compose.waitForIdle()
        waitForText("Sunday Gathering")
        compose.waitUntil(5_000) {
            compose.onNodeWithContentDescription(res(R.string.nav_home)).getBoundsInRoot().top > 480.dp
        }
        val foldedHomeBounds = compose.onNodeWithContentDescription(res(R.string.nav_home)).getBoundsInRoot()
        assertTrue("Expected bottom bar when folded, was $foldedHomeBounds", foldedHomeBounds.top > 480.dp)

        // Unfold live -> must switch back to NavigationRail while keeping Events active.
        compose.runOnUiThread { unfolded = true }
        compose.waitForIdle()
        waitForText("Sunday Gathering")
        compose.waitUntil(5_000) {
            compose.onNodeWithContentDescription(res(R.string.nav_home)).getBoundsInRoot().top < 450.dp
        }
        val reopenedHomeBounds = compose.onNodeWithContentDescription(res(R.string.nav_home)).getBoundsInRoot()
        assertTrue("Expected NavigationRail when reopened, was $reopenedHomeBounds", reopenedHomeBounds.left < 120.dp && reopenedHomeBounds.top < 450.dp)
    }

    @Test
    fun quickJumpSheetFiltersAndScrollsCleanly() {
        val adminMe = Me(
            id = 1L,
            name = "Foldable Admin",
            email = "admin@example.org",
            roles = listOf("superadmin", "admin", "driver", "traffic_head"),
            permissions = adminPerms(),
            status = "active",
            lang = "en",
            termsAccepted = true,
        )
        var pickedRoute: AppRoute? = null

        compose.setContent {
            GraceRideTheme {
                QuickJumpSheet(
                    me = adminMe,
                    onNavigate = { pickedRoute = it },
                    onDismiss = {},
                )
            }
        }

        waitForText(res(R.string.quick_jump_title))

        compose.onNodeWithText(res(R.string.quick_jump_filter_actions)).performClick()
        waitForText(res(R.string.quick_jump_action_book_ride))

        compose.onNodeWithText(res(R.string.quick_jump_filter_admin)).performClick()
        waitForText(res(R.string.admin_console_title))

        compose.onNodeWithText(res(R.string.quick_jump_filter_all)).performClick()
        compose.onNodeWithText(res(R.string.quick_jump_placeholder)).performTextInput(">Create")
        waitForText(res(R.string.quick_jump_action_new_event))

        compose.onNodeWithText(res(R.string.quick_jump_action_new_event)).performClick()
        compose.waitForIdle()
        assertTrue(pickedRoute is AppRoute.EventNew)
    }

    @Test
    fun locationPinPickerSheetConfirmAndResetReachable() {
        var confirmedLat: Double? = null
        var confirmedLng: Double? = null
        var dismissed = false

        compose.setContent {
            GraceRideTheme {
                LocationPinPickerSheet(
                    initialLat = 3.1390,
                    initialLng = 101.6869,
                    onDismiss = { dismissed = true },
                    onConfirm = { lat, lng ->
                        confirmedLat = lat
                        confirmedLng = lng
                    },
                )
            }
        }

        waitForText(res(R.string.pin_picker_title))
        compose.onNodeWithText(res(R.string.pin_picker_reset)).performScrollTo().assertIsDisplayed()
        compose.onNodeWithText(res(R.string.pin_picker_confirm)).performScrollTo().assertIsDisplayed().performClick()
        compose.waitForIdle()

        assertTrue(dismissed)
        assertTrue(abs((confirmedLat ?: 0.0) - 3.1390) < 0.001)
        assertTrue(abs((confirmedLng ?: 0.0) - 101.6869) < 0.001)
    }

    @Test
    fun tripChatSheetComposerReachableWithMessages() {
        val messagesJson = (1..15).joinToString(",") { idx ->
            val mine = idx % 2 == 0
            """{"id":$idx,"user_id":${if (mine) 1 else 2},"name":${if (mine) "\"Foldable Tester\"" else "\"Driver Lee\""},"body":"Chat item #$idx","created_at":"2026-11-27 08:${10 + idx}:00","mine":$mine,"status":"delivered","reactions":[]}"""
        }
        val threadJson = """{"open":true,"can_post":true,"members":["Foldable Tester","Driver Lee"],"reactions_allowed":["👍","🙏"],"messages":[$messagesJson]}"""

        setupSignedInDispatcher(
            routes = mapOf(
                "/api/rides/60/messages" to { req ->
                    if (req.method == "POST") {
                        MockResponse().setBody("""{"ok":true,"id":99}""")
                    } else {
                        MockResponse().setBody(threadJson)
                    }
                },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                TripChatSheet(container = container, rideId = 60L, onDismiss = {})
            }
        }

        waitForText("Chat item #15")
        compose.onNodeWithText(res(R.string.chat_placeholder)).assertIsDisplayed().performTextInput("On my way")
        compose.onNodeWithContentDescription(res(R.string.chat_send)).assertIsDisplayed().performClick()

        compose.waitUntil(5_000) {
            recordedCalls.any { it.startsWith("POST /api/rides/60/messages") }
        }
        assertEquals(true, recordedCalls.any { it.startsWith("POST /api/rides/60/messages") })
    }

    @Test
    fun loginScreenWidthCappedAndRecoveryDialogReachable() {
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val path = request.path ?: ""
                recordedCalls.add("${request.method} $path")
                return when {
                    path.startsWith("/api/auth/dev-users") -> MockResponse().setBody("[]")
                    path.startsWith("/api/auth/turnstile") -> MockResponse().setBody("""{"enabled":false}""")
                    else -> MockResponse().setBody("{}")
                }
            }
        }

        compose.setContent {
            GraceRideTheme {
                LoginScreen(container = container)
            }
        }

        waitForText(res(R.string.login_passkey))
        val passkeyBounds = compose.onNodeWithText(res(R.string.login_passkey)).performScrollTo().getBoundsInRoot()
        assertTrue(
            "Expected LoginScreen CTA width capped <= 560.dp, was ${passkeyBounds.right - passkeyBounds.left}",
            passkeyBounds.right - passkeyBounds.left <= 560.dp,
        )
        compose.onNodeWithText(res(R.string.login_recovery_link)).performScrollTo().performClick()

        waitForText(res(R.string.login_recovery_title))
        compose.onNode(hasAnyAncestor(isDialog()) and hasText(res(R.string.login_email)) and hasSetTextAction())
            .performScrollTo()
            .performTextReplacement("lost@example.org")
        compose.onNode(hasAnyAncestor(isDialog()) and hasText(res(R.string.login_recovery_submit))).assertIsDisplayed()
        compose.onNode(hasAnyAncestor(isDialog()) and hasText(res(R.string.common_cancel))).assertIsDisplayed().performClick()

        compose.waitUntil(5_000) {
            compose.onAllNodesWithText(res(R.string.login_recovery_title)).fetchSemanticsNodes().isEmpty()
        }
    }

    @OptIn(ExperimentalMaterial3Api::class)
    @Test
    fun timePickerDialogFitsCompactCoverAndExpandedWidths() {
        var screenWidthDp = 0.dp
        var confirmedHour by mutableStateOf(-1)
        var confirmedMinute by mutableStateOf(-1)

        compose.setContent {
            GraceRideTheme {
                screenWidthDp = LocalConfiguration.current.screenWidthDp.dp
                TimePickerDialog(
                    title = "Pick pickup time",
                    initialHour = 14,
                    initialMinute = 35,
                    onConfirm = { h, m ->
                        confirmedHour = h
                        confirmedMinute = m
                    },
                    onDismiss = {},
                )
            }
        }

        waitForText("Pick pickup time")
        val maxExpectedRight = minOf(screenWidthDp + 1.dp, 600.dp)
        val confirmBounds = compose.onNodeWithText(res(R.string.common_confirm)).assertIsDisplayed().getBoundsInRoot()
        val cancelBounds = compose.onNodeWithText(res(R.string.common_cancel)).assertIsDisplayed().getBoundsInRoot()
        assertTrue(
            "Confirm clipped for screenWidth=$screenWidthDp: $confirmBounds",
            confirmBounds.left >= 0.dp && confirmBounds.right <= maxExpectedRight,
        )
        assertTrue("Cancel clipped: $cancelBounds", cancelBounds.left >= 0.dp && cancelBounds.right <= confirmBounds.left)
        compose.onAllNodesWithText("35").fetchSemanticsNodes().forEach { node ->
            val rightDp = with(compose.density) { node.boundsInRoot.right.toDp() }
            val leftDp = with(compose.density) { node.boundsInRoot.left.toDp() }
            assertTrue("Minute clipped: left=$leftDp right=$rightDp", leftDp >= 0.dp && rightDp <= maxExpectedRight)
        }

        compose.onNodeWithText(res(R.string.common_confirm)).performClick()
        assertEquals(14, confirmedHour)
        assertEquals(35, confirmedMinute)
    }

    @Test
    fun deepLinkTabRootAndQueryArgsSwitchActiveDestination() {
        val deepLink = MutableStateFlow<String?>(null)
        setupSignedInDispatcher(
            routes = mapOf(
                "/api/events" to { MockResponse().setBody("""[{"id":50,"title":"Sunday Gathering","starts_at":"2026-12-20 09:00:00","location":"Church Hall","status":"published"}]""") },
                "/api/admin/traffic/plans" to { MockResponse().setBody("""[{"id":5,"title":"Plan Sunday","starts_at":"2026-11-29 09:00:00","location":"Main Hall","ride_count":1,"unassigned_count":0,"pending_confirm_count":0,"visitor_count":0,"draft_count":0,"pending_draft_count":0,"returned_draft_count":0}]""") },
                "/api/admin/rides" to { MockResponse().setBody("""{"items":[],"total":0,"page":1,"page_size":50,"total_pages":1}""") },
                "/api/admin/ride-drafts" to { MockResponse().setBody("[]") },
                "/api/admin/users" to { MockResponse().setBody("[]") },
                "/api/admin/invites" to { MockResponse().setBody("[]") },
                "/api/admin/recovery" to { MockResponse().setBody("[]") },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                GraceRideAppRoot(container = container, pendingDeepLink = deepLink)
            }
        }

        waitForText("Hello, Foldable Tester", substring = true)

        // Open Events tab -> navigate into nested EventNew child
        compose.onNodeWithContentDescription(res(R.string.nav_events)).performClick()
        waitForText("Sunday Gathering")
        compose.onNodeWithText(res(R.string.event_create_btn)).performClick()
        waitForText(res(R.string.event_form_name))

        // Deep link to Events tab must land on Events root rather than keeping EventNew
        compose.runOnUiThread {
            deepLink.value = NotifLinkRouter.extractPathFromIntentUri("graceride://open/events")!!
        }
        waitForText("Sunday Gathering")

        // Deep link directly into TrafficHub Rides tab and AdminMembers Invites tab
        compose.runOnUiThread {
            deepLink.value = NotifLinkRouter.extractPathFromIntentUri("graceride://open/admin/traffic?tab=rides")!!
        }
        waitForText(res(R.string.traffic_rides_search_hint))

        compose.runOnUiThread {
            deepLink.value = NotifLinkRouter.extractPathFromIntentUri("graceride://open/admin/members?tab=invites")!!
        }
        waitForText(res(R.string.members_create_invite))
    }

    @Test
    fun quickJumpSheetCloseButtonDismissesCleanly() {
        val adminMe = Me(
            id = 1L,
            name = "Foldable Admin",
            email = "admin@example.org",
            roles = listOf("superadmin", "admin"),
            permissions = adminPerms(),
            status = "active",
            lang = "en",
            termsAccepted = true,
        )
        var dismissed = false

        compose.setContent {
            GraceRideTheme {
                QuickJumpSheet(
                    me = adminMe,
                    onNavigate = {},
                    onDismiss = { dismissed = true },
                )
            }
        }

        waitForText(res(R.string.quick_jump_title))
        compose.onNodeWithContentDescription(res(R.string.common_close)).assertIsDisplayed().performClick()
        compose.waitForIdle()
        assertTrue(dismissed)
    }

    @Test
    fun adminSettingsTabsDisplayWithoutClippingOnCompactPhone() {
        setupSignedInDispatcher(
            meJson = buildMeJson(permissions = adminPerms() + "admin_groups.view" + "admin_settings.update"),
            routes = mapOf(
                "/api/admin/settings/system" to {
                    MockResponse().setBody("""{"audit_retention_days":365,"chat_retention_days":90,"max_live_drivers":10,"session_timeout_mins":15,"auto_lock_mins":5}""")
                },
                "/api/admin/settings/alerts" to { MockResponse().setBody("{}") },
                "/api/groups" to { MockResponse().setBody("[]") },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                val nav = rememberNavController()
                Box(Modifier.requiredSize(width = 411.dp, height = 820.dp)) {
                    AdminSettingsScreen(container = container, nav = nav)
                }
            }
        }

        waitForText(res(R.string.settings_tab_rates))
        listOf(
            R.string.settings_tab_rates,
            R.string.settings_tab_alerts,
            R.string.settings_tab_groups,
        ).forEach { tabRes ->
            val bounds = compose.onNodeWithText(res(tabRes)).assertIsDisplayed().getBoundsInRoot()
            assertTrue("Tab clipped out of 411dp bounds: $bounds", bounds.left >= 0.dp && bounds.right <= 412.dp)
        }
        compose.onNodeWithText(res(R.string.settings_tab_alerts)).performClick()
        compose.onNodeWithText(res(R.string.settings_tab_groups)).performClick()
        waitForText(res(R.string.settings_empty_groups))
    }

    @Test
    fun proxyRiderDisplayDoesNotDuplicateRiderNameInTrafficRides() {
        val proxyRideJson = """{"id":77,"status":"pending_driver","pickup_at":"2026-11-27 09:00:00","area":{"id":1,"name":"PJ Section 14"},"destination":"Church Sanctuary","seats":1,"is_proxy":true,"proxy":{"is_proxy":true,"rider_name":"林小薇 (新朋友)","booker_name":"陈法拉"},"passenger":{"id":9,"name":"林小薇 (新朋友) (由 陈法拉 代订)"},"viewer":{"is_passenger":false,"is_driver":false,"is_staff":true}}"""
        setupSignedInDispatcher(
            routes = mapOf(
                "/api/admin/rides" to {
                    MockResponse().setBody("""{"items":[$proxyRideJson],"total":1,"page":1,"page_size":50,"total_pages":1}""")
                },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                val nav = rememberNavController()
                Box(Modifier.requiredSize(width = 841.dp, height = 701.dp)) {
                    TrafficHubScreen(container = container, nav = nav, initialTab = "rides")
                }
            }
        }

        waitForText("林小薇 (新朋友)", substring = true)
        assertTrue(
            "Proxy rider string should not repeat rider name",
            compose.onAllNodesWithText("林小薇 (新朋友) (林小薇", substring = true).fetchSemanticsNodes().isEmpty(),
        )
    }

    @Test
    fun securitySettingsShowsLogoutAllWithoutClippingOnUnfoldedWidth() {
        setupSignedInDispatcher()

        compose.setContent {
            GraceRideTheme {
                val nav = rememberNavController()
                Box(Modifier.requiredSize(width = 841.dp, height = 701.dp)) {
                    SecuritySettingsScreen(container = container, nav = nav)
                }
            }
        }

        waitForText(res(R.string.profile_logout_all))
        val bounds = compose.onNodeWithText(res(R.string.profile_logout_all)).assertIsDisplayed().getBoundsInRoot()
        assertTrue(
            "Expected logout-all visible within 841x701dp canvas, was $bounds",
            bounds.left >= 0.dp && bounds.right <= 842.dp && bounds.bottom <= 702.dp,
        )
    }
}
