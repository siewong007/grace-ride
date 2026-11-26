package org.pjstatecm.graceride

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.hasSetTextAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.isToggleable
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onLast
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performTextInput
import androidx.navigation.compose.rememberNavController
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import okhttp3.mockwebserver.Dispatcher
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import okhttp3.mockwebserver.RecordedRequest
import org.junit.After
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.screens.admin.AdminMembersScreen
import org.pjstatecm.graceride.ui.screens.announcements.AnnouncementsScreen
import org.pjstatecm.graceride.ui.theme.GraceRideTheme
import java.util.concurrent.CopyOnWriteArrayList

@RunWith(AndroidJUnit4::class)
class AdminMembersAndAnnouncementsAndroidTest {

    @get:Rule
    val compose = createComposeRule()

    private lateinit var server: MockWebServer
    private lateinit var container: AppContainer

    @Before
    fun setup() {
        server = MockWebServer()
        server.start()
        val ctx = InstrumentationRegistry.getInstrumentation().targetContext
        container = AppContainer(ctx, InMemoryStore())
        container.setBaseUrl(server.url("/").toString())
    }

    @After
    fun teardown() {
        server.shutdown()
    }

    private fun res(id: Int, vararg args: Any): String =
        InstrumentationRegistry.getInstrumentation().targetContext.getString(id, *args)

    private fun waitForText(text: String, timeoutMs: Long = 10_000L) {
        compose.waitUntil(timeoutMs) {
            compose.onAllNodesWithText(text, substring = true).fetchSemanticsNodes().isNotEmpty()
        }
    }

    private fun signInAdmin(extraRoutes: Map<String, (RecordedRequest) -> MockResponse> = emptyMap()) {
        val adminMe = """
            {"id":1,"name":"Admin Lim","email":"admin@example.org","status":"active",
             "roles":["admin"],
             "permissions":["rides.view","events.view","admin_dashboard.view","admin_members.view","admin_members.update","admin_roles.view","admin_roles.update","admin_invites.view","admin_invites.create","admin_invites.delete","admin_recovery.view","admin_recovery.update","admin_announcements.view","admin_announcements.create","admin_announcements.delete"],
             "mfa_ok":true}
        """.trimIndent()
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val path = request.path?.substringBefore('?').orEmpty()
                extraRoutes[path]?.let { return it(request) }
                return when (path) {
                    "/api/auth/me" -> MockResponse().setBody(adminMe)
                    "/api/admin/dashboard" -> MockResponse().setBody("""{"today":"2026-11-26","members":2,"pending_members":1,"pending_recovery":1,"open_invites":1}""")
                    else -> MockResponse().setBody("{}")
                }
            }
        }
        runBlocking { container.auth.refresh() }
    }

    @Test
    fun adminMembersRendersTabsFiltersAndEditSheet() {
        val usersBody = """
            {"items":[
              {"id":10,"name":"Pending Lee","email":"lee@example.org","phone":"012-111","roles":[],"status":"pending","created_at":"2026-11-26 08:00:00","passkeys":0,"groups":""}
            ],"total":1,"page":1,"page_size":50,"total_pages":1}
        """.trimIndent()
        val rolesBody = """{"roles":[{"key":"driver","name":"","implicit":false,"assignable":true,"permissions":["drive.view"]}]}"""
        val groupsBody = """[{"id":1,"name":"All","is_all":true,"is_visitor":false},{"id":2,"name":"Youth","is_all":false,"is_visitor":false,"member_count":5}]"""

        signInAdmin(
            mapOf(
                "/api/admin/users" to { MockResponse().setBody(usersBody) },
                "/api/admin/roles" to { MockResponse().setBody(rolesBody) },
                "/api/groups" to { MockResponse().setBody(groupsBody) },
                "/api/admin/invites" to { MockResponse().setBody("[]") },
                "/api/admin/recovery" to { MockResponse().setBody("[]") },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                AdminMembersScreen(container = container, nav = rememberNavController())
            }
        }

        waitForText("Pending Lee")
        compose.onNodeWithText("Pending Lee").assertIsDisplayed()
        compose.onNodeWithText(res(R.string.members_approve_btn)).performClick()
        waitForText("Youth")
        compose.onNodeWithText("Youth").assertIsDisplayed()
        compose.onNodeWithText(res(R.string.role_driver)).assertIsDisplayed()
    }

    @Test
    fun adminMembersInvitesAndRecoveryShowOneTimeLinks() {
        signInAdmin(
            mapOf(
                "/api/admin/users" to { MockResponse().setBody("""{"items":[],"total":0,"page":1,"page_size":50,"total_pages":1}""") },
                "/api/admin/roles" to { MockResponse().setBody("""{"roles":[]}""") },
                "/api/groups" to { MockResponse().setBody("[]") },
                "/api/admin/invites" to { req ->
                    if (req.method == "POST") {
                        MockResponse().setBody("""{"id":5,"token":"tok5","url":"https://graceride.saliminn.my/invite/tok5"}""")
                    } else {
                        MockResponse().setBody("[]")
                    }
                },
                "/api/admin/recovery" to {
                    MockResponse().setBody("""[{"id":8,"email":"lost@example.org","name":"Bro Tan","reason":"New phone","status":"pending","created_at":"2026-11-26 09:00:00"}]""")
                },
                "/api/admin/recovery/8/approve" to {
                    MockResponse().setBody("""{"url":"https://graceride.saliminn.my/invite/rec8"}""")
                },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                AdminMembersScreen(container = container, nav = rememberNavController())
            }
        }

        compose.onNodeWithText(res(R.string.members_tab_invites)).performClick()
        waitForText(res(R.string.members_create_invite))
        compose.onNodeWithText(res(R.string.members_create_invite)).performClick()
        waitForText("https://graceride.saliminn.my/invite/tok5")

        compose.onNodeWithText(res(R.string.members_tab_recovery)).performClick()
        waitForText("Bro Tan")
        compose.onNodeWithText(res(R.string.members_recovery_approve)).performClick()
        waitForText(res(R.string.common_cancel))
        compose.onAllNodesWithText(res(R.string.members_recovery_approve)).onLast().performClick()
        waitForText("https://graceride.saliminn.my/invite/rec8")
    }

    @Test
    fun announcementsScreenPublishesWithNotifyConfirmationAndPins() {
        val calls = CopyOnWriteArrayList<String>()
        signInAdmin(
            mapOf(
                "/api/announcements" to { req ->
                    calls += "${req.method} ${req.path}"
                    if (req.method == "POST") {
                        MockResponse().setBody("""{"id":22}""")
                    } else {
                        MockResponse().setBody(
                            """[{"id":21,"title":"Parking Notice","body":"Use Lot B","pinned":false,"by":"Admin Lim","created_at":"2026-11-26 08:00:00","targeted":true,"groups":[{"id":2,"name":"Youth"}]}]""",
                        )
                    }
                },
                "/api/announcements/21" to { req ->
                    calls += "${req.method} ${req.path}"
                    MockResponse().setBody("""{"ok":true}""")
                },
                "/api/groups" to {
                    MockResponse().setBody("""[{"id":1,"name":"All","is_all":true,"is_visitor":false},{"id":2,"name":"Youth","is_all":false,"is_visitor":false,"member_count":5}]""")
                },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                AnnouncementsScreen(container = container, nav = rememberNavController(), manage = true)
            }
        }

        waitForText("Parking Notice")
        compose.onNodeWithText("Parking Notice").assertIsDisplayed()

        // Toggle pin
        compose.onNodeWithText(res(R.string.announcements_pin)).performClick()
        compose.waitUntil(5_000L) { calls.any { it == "PUT /api/announcements/21" } }

        // Open Publish sheet and confirm notification dialog
        compose.onNodeWithText(res(R.string.announcements_publish_fab)).performClick()
        waitForText(res(R.string.announcements_create_title))
        compose.onNode(hasSetTextAction() and hasText(res(R.string.announcements_title_label)))
            .performTextInput("Camp Update")
        compose.onNode(hasSetTextAction() and hasText(res(R.string.announcements_body_label)))
            .performTextInput("Meet at 8am")
        compose.onAllNodesWithText(res(R.string.announcements_publish_fab))
            .onLast()
            .performScrollTo()
            .performClick()
        waitForText(res(R.string.announcements_notify_all_confirm_title))
        compose.onAllNodesWithText(res(R.string.announcements_publish_fab)).onLast().performClick()
        compose.waitUntil(5_000L) { calls.any { it.startsWith("POST /api/announcements") } }
        assertTrue(calls.any { it.startsWith("POST /api/announcements") })
    }

    @Test
    fun adminMembersTabTransitionsAndBatchBarAppearAndDismiss() {
        val usersBody = """
            {"items":[
              {"id":10,"name":"Pending Lee","email":"lee@example.org","phone":"012-111","roles":[],"status":"pending","created_at":"2026-11-26 08:00:00","passkeys":0,"groups":""}
            ],"total":1,"page":1,"page_size":50,"total_pages":1}
        """.trimIndent()
        signInAdmin(
            mapOf(
                "/api/admin/users" to { MockResponse().setBody(usersBody) },
                "/api/admin/roles" to { MockResponse().setBody("""{"roles":[]}""") },
                "/api/groups" to { MockResponse().setBody("[]") },
                "/api/admin/invites" to { MockResponse().setBody("[]") },
                "/api/admin/recovery" to { MockResponse().setBody("[]") },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                AdminMembersScreen(container = container, nav = rememberNavController())
            }
        }

        waitForText("Pending Lee")
        compose.onNode(isToggleable()).performClick()
        waitForText(res(R.string.members_selected_count, 1))
        compose.onNodeWithText(res(R.string.members_batch_approve, 1)).assertIsDisplayed()
        compose.onAllNodesWithText(res(R.string.common_cancel)).onLast().performClick()
        compose.waitUntil(5_000L) {
            compose.onAllNodesWithText(res(R.string.members_selected_count, 1)).fetchSemanticsNodes().isEmpty()
        }

        compose.onNodeWithText(res(R.string.members_tab_invites)).performClick()
        waitForText(res(R.string.members_create_invite))
        compose.onNodeWithText(res(R.string.members_tab_recovery)).performClick()
        waitForText(res(R.string.members_recovery_notice))
        compose.onNodeWithText(res(R.string.members_tab_members)).performClick()
        waitForText("Pending Lee")
        compose.onNodeWithText("Pending Lee").assertIsDisplayed()
    }
}
