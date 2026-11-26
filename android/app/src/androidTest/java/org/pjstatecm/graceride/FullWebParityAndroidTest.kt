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
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import androidx.navigation.compose.rememberNavController
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import okhttp3.mockwebserver.Dispatcher
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import okhttp3.mockwebserver.RecordedRequest
import org.junit.After
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.screens.admin.AdminAuditScreen
import org.pjstatecm.graceride.ui.screens.admin.AdminConsoleScreen
import org.pjstatecm.graceride.ui.screens.admin.AdminRolesScreen
import org.pjstatecm.graceride.ui.screens.admin.AdminSettingsScreen
import org.pjstatecm.graceride.ui.screens.help.HelpScreen
import org.pjstatecm.graceride.ui.screens.reimbursements.ReimbursementsScreen
import org.pjstatecm.graceride.ui.screens.reports.ReportScreen
import org.pjstatecm.graceride.ui.screens.settlements.SettlementsScreen
import org.pjstatecm.graceride.ui.theme.GraceRideTheme

@RunWith(AndroidJUnit4::class)
class FullWebParityAndroidTest {

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
        container = AppContainer(targetContext, InMemoryStore())
        container.setBaseUrl(server.url("/").toString())
    }

    @After
    fun teardown() {
        runCatching { server.shutdown() }
    }

    @Composable
    private fun FreshSlot(keyVal: Any, content: @Composable () -> Unit) {
        key(keyVal) {
            val owner = remember {
                object : ViewModelStoreOwner {
                    override val viewModelStore = ViewModelStore()
                }
            }
            DisposableEffect(Unit) { onDispose { owner.viewModelStore.clear() } }
            CompositionLocalProvider(LocalViewModelStoreOwner provides owner) {
                content()
            }
        }
    }

    private fun waitForText(text: String, substring: Boolean = false) {
        compose.waitUntil(8_000) {
            compose.onAllNodes(androidx.compose.ui.test.hasText(text, substring = substring))
                .fetchSemanticsNodes().isNotEmpty()
        }
    }

    @Test
    fun newWebParityAdminFinanceHelpAndReportScreensRender() {
        val superMe = """
            {
              "id": 1, "name": "Admin User", "email": "admin@example.org", "phone": "+6012",
              "roles": ["admin"], "status": "active", "terms_accepted": true, "lang": "en",
              "permissions": [
                "admin_dashboard.view", "admin_rides.view", "admin_live.view", "admin_roles.view",
                "admin_audit.view", "admin_settings.view", "admin_groups.view",
                "settlements.view", "settlements.create", "reimbursements.view", "reimbursements.create",
                "help.view", "help.create", "admin_help.view", "report.view", "admin_reports.view"
              ]
            }
        """.trimIndent()

        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val p = request.path.orEmpty()
                return when {
                    p.startsWith("/api/auth/me") -> MockResponse().setBody(superMe)
                    p.startsWith("/api/admin/dashboard") -> MockResponse().setBody("""{"today":"2026-11-26","today_total":5,"today_confirmed":4,"today_changes":1,"pending_approval":1,"no_driver":1,"live_drivers":2,"max_live_drivers":10,"members":42}""")
                    p.startsWith("/api/admin/rides") -> MockResponse().setBody("""{"items":[],"total":0,"page":1,"page_size":50,"total_pages":1}""")
                    p.startsWith("/api/admin/roles") -> MockResponse().setBody("""{"catalog":[{"page":"rides","area":"rides","actions":["view","create"]}],"roles":[{"key":"member","name":"","description":"","system":true,"implicit":true,"permissions":["rides.view"],"locked":["rides.view"],"members":42}]}""")
                    p.startsWith("/api/admin/audit") -> MockResponse().setBody("""{"items":[{"id":1,"actor":"Admin User","action":"role.update","entity":"role","detail":"rides.view","created_at":"2026-11-26 09:00:00"}],"next_before":null,"retention_days":365}""")
                    p.startsWith("/api/admin/settings/system") -> MockResponse().setBody("""{"audit_retention_days":365,"chat_retention_days":90,"max_live_drivers":10,"session_timeout_mins":15}""")
                    p.startsWith("/api/admin/settings/alerts") -> MockResponse().setBody("""{"telegram_bot_configured":false,"alert_webhook_configured":false,"mobile_push_webhook_configured":false,"vapid_private_configured":false,"push_subscriptions_count":0,"mobile_push_subscriptions_count":0}""")
                    p.startsWith("/api/groups") -> MockResponse().setBody("""[{"id":1,"name":"All Members","is_all":true,"is_visitor":false,"member_count":42}]""")
                    p.startsWith("/api/settlements") -> MockResponse().setBody("""{"items":[{"id":3,"title":"Camp Fuel","event_title":"Retreat","status":"published","total_cents":3000,"confirmed":1,"count":2}],"total":1,"page":1,"page_size":25,"total_pages":1}""")
                    p.startsWith("/api/reimbursements") -> MockResponse().setBody("""[]""")
                    p.startsWith("/api/help/counts") -> MockResponse().setBody("""{"open":0,"answered":0,"closed":0}""")
                    p.startsWith("/api/help/guides") -> MockResponse().setBody("""[]""")
                    p.startsWith("/api/help") -> MockResponse().setBody("""{"items":[],"next_before":null}""")
                    p.startsWith("/api/reports/monthly") -> MockResponse().setBody("""{"month":"2026-11","rides":{"total":3,"completed":3}}""")
                    else -> MockResponse().setBody("[]")
                }
            }
        }
        runBlocking { container.auth.refresh() }

        var screen by mutableIntStateOf(0)
        compose.setContent {
            GraceRideTheme {
                val nav = rememberNavController()
                FreshSlot(screen) {
                    when (screen) {
                        0 -> AdminConsoleScreen(container, nav)
                        1 -> AdminRolesScreen(container, nav)
                        2 -> AdminAuditScreen(container, nav)
                        3 -> AdminSettingsScreen(container, nav)
                        4 -> SettlementsScreen(container, nav)
                        5 -> ReimbursementsScreen(container, nav)
                        6 -> HelpScreen(container, nav)
                        else -> ReportScreen(container, nav)
                    }
                }
            }
        }

        waitForText(res(R.string.admin_queue_section))
        compose.onNodeWithText(res(R.string.admin_console_title)).assertIsDisplayed()

        compose.runOnIdle { screen = 1 }
        waitForText(res(R.string.roles_title))
        compose.onNodeWithText(res(R.string.roles_everyone_hint)).assertIsDisplayed()

        compose.runOnIdle { screen = 2 }
        waitForText("role.update")

        compose.runOnIdle { screen = 3 }
        waitForText(res(R.string.settings_system_title))

        compose.runOnIdle { screen = 4 }
        waitForText("Camp Fuel")

        compose.runOnIdle { screen = 5 }
        waitForText(res(R.string.reimb_title))

        compose.runOnIdle { screen = 6 }
        waitForText(res(R.string.help_title))

        compose.runOnIdle { screen = 7 }
        waitForText(res(R.string.report_title))
    }
}
