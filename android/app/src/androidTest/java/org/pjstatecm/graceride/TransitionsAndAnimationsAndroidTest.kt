package org.pjstatecm.graceride

import android.animation.ValueAnimator
import android.content.Context
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.click
import androidx.compose.ui.test.hasScrollAction
import androidx.compose.ui.test.isToggleable
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onFirst
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.swipeDown
import androidx.compose.ui.test.swipeLeft
import androidx.compose.ui.test.swipeRight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
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
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.pjstatecm.graceride.data.api.Badge
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.GraceRideAppRoot
import org.pjstatecm.graceride.ui.components.BrandMarkMode
import org.pjstatecm.graceride.ui.components.GraceRideBrandMark
import org.pjstatecm.graceride.ui.components.GraceRideLandingSplash
import org.pjstatecm.graceride.ui.components.GraceRideLoadingView
import org.pjstatecm.graceride.ui.components.GrokBotCanvas
import org.pjstatecm.graceride.ui.components.GrokBotSpec
import org.pjstatecm.graceride.ui.components.MemberAvatar
import org.pjstatecm.graceride.ui.components.ScheduleTimeline
import org.pjstatecm.graceride.ui.components.TieredBadgeMedallion
import org.pjstatecm.graceride.ui.components.TimelineBadge
import org.pjstatecm.graceride.ui.components.TimelineKind
import org.pjstatecm.graceride.ui.components.TimelineNode
import org.pjstatecm.graceride.ui.components.TimelineState
import org.pjstatecm.graceride.ui.screens.events.EventDetailScreen
import org.pjstatecm.graceride.ui.screens.events.EventsScreen
import org.pjstatecm.graceride.ui.screens.notifications.NotificationsScreen
import org.pjstatecm.graceride.ui.screens.rides.RideDetailScreen
import org.pjstatecm.graceride.ui.screens.rides.RideNewScreen
import org.pjstatecm.graceride.ui.screens.rides.RidesScreen
import org.pjstatecm.graceride.ui.screens.traffic.TrafficHubScreen
import org.pjstatecm.graceride.ui.theme.GraceRideTheme
import java.util.Collections

@RunWith(AndroidJUnit4::class)
class TransitionsAndAnimationsAndroidTest {

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
        name: String = "Animation Tester",
        roles: List<String> = listOf("member", "driver", "traffic_head"),
        permissions: List<String> = listOf(
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
            "admin_plan.view",
            "admin_plan.update",
            "admin_rides.view",
            "admin_rides.update",
            "admin_live.view",
            "admin_area_photos.view",
            "admin_fares.view",
            "admin_areas.view",
            "admin_dashboard.view",
        ),
    ): String {
        val rolesJson = apiJson.encodeToString(roles)
        val permsJson = apiJson.encodeToString(permissions)
        return """
            {
              "id": $id,
              "name": ${apiJson.encodeToString(name)},
              "email": "anim$id@example.org",
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
    fun landingSplashBlocksTouchThroughAndFinishesCleanly() {
        var visible by mutableStateOf(true)
        var mounted by mutableStateOf(true)
        var finishCount by mutableIntStateOf(0)
        var underlyingClicks by mutableIntStateOf(0)

        compose.setContent {
            GraceRideTheme {
                Box(Modifier.fillMaxSize()) {
                    Button(
                        onClick = { underlyingClicks++ },
                        modifier = Modifier.fillMaxSize(),
                    ) {
                        Text("Underlying Button")
                    }
                    if (mounted) {
                        GraceRideLandingSplash(
                            visible = visible,
                            onFinished = {
                                finishCount++
                                mounted = false
                            },
                        )
                    }
                }
            }
        }

        compose.waitForIdle()
        // Pointer taps on the splash overlay must be consumed, not passed through to underlying UI.
        compose.onRoot().performTouchInput { click(center) }
        compose.waitForIdle()
        assertEquals(0, underlyingClicks)

        // Rapid visible toggle during/before exit should complete cleanly and invoke onFinished.
        compose.runOnIdle { visible = false }
        compose.waitUntil(5_000) { finishCount == 1 && !mounted }

        // Remount & test interruption recovery when animations are enabled:
        // flip visible=false, advance 100ms (< 280ms exit), tap to verify exit still blocks touch-through,
        // then flip back visible=true and confirm splash stays mounted before finishing cleanly.
        compose.runOnIdle {
            visible = true
            mounted = true
        }
        compose.waitForIdle()
        if (ValueAnimator.areAnimatorsEnabled()) {
            compose.mainClock.autoAdvance = false
            try {
                visible = false
                compose.mainClock.advanceTimeBy(100)
                compose.onRoot().performTouchInput { click(center) }
                assertEquals(0, underlyingClicks)
                assertTrue(mounted)
                visible = true
                compose.mainClock.advanceTimeBy(60)
                assertTrue(mounted)
            } finally {
                compose.mainClock.autoAdvance = true
            }
            compose.waitForIdle()
            assertTrue(mounted)
        }
        compose.runOnIdle { visible = false }
        compose.waitUntil(5_000) { finishCount >= 2 && !mounted }
    }

    @Test
    fun brandMarkAllModesAndAnimatedComponentsRenderWithoutCrash() {
        var darkTheme by mutableStateOf(false)
        var botSpec by mutableStateOf(GrokBotSpec.DEFAULT)
        var botMove by mutableStateOf("idle")
        var nodes by mutableStateOf(
            listOf(
                TimelineNode(
                    id = "n1",
                    timeLabel = "09:00",
                    title = "Pickup Confirmed",
                    subtitle = "Section 14",
                    kind = TimelineKind.RIDE,
                    state = TimelineState.ACTIVE,
                    badge = TimelineBadge("Confirmed", RideStatusKind.ACTIVE),
                ),
                TimelineNode(
                    id = "n2",
                    timeLabel = "10:00",
                    title = "Church Hall",
                    kind = TimelineKind.EVENT,
                    state = TimelineState.UPCOMING,
                ),
            ),
        )

        compose.setContent {
            GraceRideTheme(darkTheme = darkTheme) {
                Column(Modifier.fillMaxSize()) {
                    Row {
                        GraceRideBrandMark(Modifier.size(48.dp), mode = BrandMarkMode.INTRO)
                        GraceRideBrandMark(Modifier.size(48.dp), mode = BrandMarkMode.LOADING)
                        GraceRideBrandMark(Modifier.size(48.dp), mode = BrandMarkMode.IDLE_HOP)
                        GraceRideBrandMark(Modifier.size(48.dp), mode = BrandMarkMode.STATIC)
                        GraceRideBrandMark(Modifier.size(0.dp), mode = BrandMarkMode.LOADING)
                    }
                    Row {
                        listOf("none", "bronze", "silver", "gold", "platinum").forEach { tier ->
                            TieredBadgeMedallion(
                                badge = Badge(
                                    key = "k-$tier",
                                    char = "★",
                                    tier = tier,
                                    earned = tier != "none",
                                ),
                                size = 40.dp,
                            )
                        }
                    }
                    Row {
                        MemberAvatar(name = "Grok", avatarIcon = "grok:hexagon:blue", avatarColor = "gray:hexagon", size = 36.dp)
                        MemberAvatar(name = "Muse", avatarIcon = "muse:cloud:pink", avatarColor = "amber:cloud", size = 36.dp)
                        MemberAvatar(name = "Zero", avatarIcon = "grok:circle", size = 0.dp)
                    }
                    GrokBotCanvas(
                        spec = botSpec,
                        move = botMove,
                        variant = "stage",
                        gazeOffset = Offset(0.4f, -0.3f),
                        modifier = Modifier.size(120.dp),
                    )
                    ScheduleTimeline(items = nodes)
                    GraceRideLoadingView(modifier = Modifier.size(80.dp), markSize = 48.dp, showLabel = true)
                }
            }
        }

        compose.waitForIdle()
        // Exercise shape morphing & every single GrokBot move under dark and light themes.
        GrokBotSpec.MOVES.forEachIndexed { idx, move ->
            val shape = GrokBotSpec.SHAPES[idx % GrokBotSpec.SHAPES.size]
            compose.runOnIdle {
                botMove = move
                botSpec = botSpec.copy(shape = shape, face = GrokBotSpec.FACES[idx % GrokBotSpec.FACES.size])
                if (idx % 5 == 0) darkTheme = !darkTheme
            }
            compose.waitForIdle()
        }

        // Prepend a new active timeline node to exercise key(node.id) stability.
        compose.runOnIdle {
            nodes = listOf(
                TimelineNode(
                    id = "n0",
                    timeLabel = "08:30",
                    title = "Driver En Route",
                    kind = TimelineKind.DRIVE,
                    state = TimelineState.ACTIVE,
                ),
            ) + nodes
        }
        compose.waitForIdle()
        compose.onNodeWithText("Driver En Route").assertIsDisplayed()
        compose.onNodeWithText("Pickup Confirmed").assertIsDisplayed()
    }

    @Test
    fun appRootTabTransitionsSwipeAndDetailPushPop() {
        val nextRideJson = """{"id":40,"status":"confirmed","pickup_at":"2026-11-27 09:00:00","area":{"id":1,"name":"PJ Section 14"},"destination":"Church Sanctuary","seats":1,"passenger":{"id":1,"name":"Animation Tester"},"viewer":{"is_passenger":true,"is_driver":false,"is_staff":false}}"""
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

        waitForText("Hello, Animation Tester", substring = true)

        // Bottom pill navigation: Home -> Events -> Traffic -> Profile -> Home
        compose.onNodeWithContentDescription(res(R.string.nav_events)).performClick()
        waitForText("Sunday Gathering")

        compose.onNodeWithContentDescription(res(R.string.nav_traffic)).performClick()
        waitForText("Plan Sunday")

        compose.onNodeWithContentDescription(res(R.string.nav_profile)).performClick()
        waitForText(res(R.string.profile_personal))

        compose.onNodeWithContentDescription(res(R.string.nav_home)).performClick()
        waitForText("Church Sanctuary", substring = true)

        // Horizontal drag gesture navigation: Home -> Events -> Home
        compose.onRoot().performTouchInput {
            swipeLeft(startX = right * 0.85f, endX = right * 0.1f, durationMillis = 300)
        }
        waitForText("Sunday Gathering")

        compose.onRoot().performTouchInput {
            swipeRight(startX = right * 0.15f, endX = right * 0.9f, durationMillis = 300)
        }
        waitForText("Church Sanctuary", substring = true)

        // Detail push hides floating bottom bar; pop restores it cleanly.
        compose.onNodeWithContentDescription(res(R.string.notif_title)).performClick()
        waitForText(res(R.string.notif_empty))
        compose.onNodeWithContentDescription(res(R.string.nav_events)).assertDoesNotExist()

        compose.onNodeWithContentDescription(res(R.string.a11y_back)).performClick()
        waitForText("Church Sanctuary", substring = true)
        compose.onNodeWithContentDescription(res(R.string.nav_events)).assertIsDisplayed()
    }

    @Test
    fun eventsAndRidesTabTransitionsPreserveContent() {
        var screenIdx by mutableIntStateOf(0)
        val upcomingEvent = """[{"id":50,"title":"Upcoming Worship","starts_at":"2026-12-20 09:00:00","location":"Sanctuary","status":"published","my_signup":{"status":"going"}}]"""
        val pastEvent = """[{"id":51,"title":"Past Conference","starts_at":"2025-12-01 09:00:00","location":"Old Hall","status":"finished"}]"""
        val draftEvent = """[{"id":52,"title":"Managing Draft Event","starts_at":"2026-12-28 09:00:00","location":"Room 2","status":"draft"}]"""
        val upcomingRide = """[{"id":60,"status":"confirmed","pickup_at":"2026-12-20 08:20:00","area":{"id":1,"name":"PJ Section 14"},"destination":"Sanctuary","seats":1,"passenger":{"id":1,"name":"Animation Tester"},"viewer":{"is_passenger":true,"is_driver":false,"is_staff":false}}]"""
        val pastRide = """[{"id":61,"status":"completed","pickup_at":"2025-12-01 08:20:00","area":{"id":1,"name":"SS2"},"destination":"Past Venue","seats":1,"passenger":{"id":1,"name":"Animation Tester"},"viewer":{"is_passenger":true,"is_driver":false,"is_staff":false}}]"""

        setupSignedInDispatcher(
            routes = mapOf(
                "/api/events?scope=past" to { MockResponse().setBody(pastEvent) },
                "/api/events?scope=managing" to { MockResponse().setBody(draftEvent) },
                "/api/events" to { MockResponse().setBody(upcomingEvent) },
                "/api/rides?scope=past" to { MockResponse().setBody(pastRide) },
                "/api/rides" to { MockResponse().setBody(upcomingRide) },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                val nav = rememberNavController()
                FreshScreenSlot(screenIdx) {
                    if (screenIdx == 0) {
                        EventsScreen(container = container, nav = nav)
                    } else {
                        RidesScreen(container = container, nav = nav)
                    }
                }
            }
        }

        waitForText("Upcoming Worship")
        compose.onNodeWithText(res(R.string.events_tab_my_schedule)).performClick()
        waitForText("Upcoming Worship", substring = true)
        compose.onNodeWithText(res(R.string.events_tab_past)).performClick()
        waitForText("Past Conference")
        compose.onNodeWithText(res(R.string.events_tab_managing)).performClick()
        waitForText("Managing Draft Event")
        compose.onNodeWithText(res(R.string.events_tab_upcoming)).performClick()
        waitForText("Upcoming Worship")

        compose.runOnIdle { screenIdx = 1 }
        waitForText("Sanctuary", substring = true)
        compose.onNodeWithText(res(R.string.rides_tab_past)).performClick()
        waitForText("Past Venue", substring = true)
        compose.onNodeWithText(res(R.string.rides_tab_upcoming)).performClick()
        waitForText("Sanctuary", substring = true)
    }

    @Test
    fun notificationsSwipeToggleWorksTwiceAndDeleteUndoRestores() {
        val today = KlTime.today()
        var isRead = false
        val itemJson = { readFlag: Boolean ->
            """{"items":[{"id":1,"title":"Driver Update","body":"Arriving in 5 minutes at Gate A.","read":$readFlag,"category":"ride","created_at":"$today 09:00:00"}],"next_before":null,"unread":${if (readFlag) 0 else 1}}"""
        }

        setupSignedInDispatcher(
            routes = mapOf(
                "/api/notifications/1/read" to {
                    isRead = true
                    MockResponse().setBody("""{"ok":true,"unread":0}""")
                },
                "/api/notifications/1/unread" to {
                    isRead = false
                    MockResponse().setBody("""{"ok":true,"unread":1}""")
                },
                "/api/notifications/1" to { req ->
                    if (req.method == "DELETE") MockResponse().setBody("""{"ok":true,"unread":0}""")
                    else MockResponse().setBody(itemJson(isRead))
                },
                "/api/notifications/preferences" to {
                    MockResponse().setBody("""{"push_opt_in":true,"whatsapp_opt_in":false}""")
                },
                "/api/notifications" to { MockResponse().setBody(itemJson(isRead)) },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                NotificationsScreen(container = container, nav = rememberNavController())
            }
        }

        waitForText("Driver Update")

        // Swipe #1 StartToEnd: unread -> read
        compose.onNodeWithText("Driver Update").performTouchInput {
            swipeRight(startX = left + 24f, endX = right - 24f, durationMillis = 300)
        }
        compose.waitUntil(5_000) {
            recordedCalls.any { it.startsWith("POST /api/notifications/1/read") }
        }
        compose.waitForIdle()

        // Swipe #2 StartToEnd on SAME card: must use fresh item & invoke unread (regression guard!)
        compose.onNodeWithText("Driver Update").performTouchInput {
            swipeRight(startX = left + 24f, endX = right - 24f, durationMillis = 300)
        }
        compose.waitUntil(5_000) {
            recordedCalls.any { it.startsWith("POST /api/notifications/1/unread") }
        }
        compose.waitForIdle()

        // Swipe EndToStart to stage delete, then click Undo and confirm restoration
        compose.onNodeWithText("Driver Update").performTouchInput {
            swipeLeft(startX = right - 24f, endX = left + 24f, durationMillis = 300)
        }
        waitForText(res(R.string.notif_undo))
        compose.onNodeWithText(res(R.string.notif_undo)).performClick()
        waitForText("Driver Update")
        assertTrue(recordedCalls.none { it.startsWith("DELETE /api/notifications/1") })
    }

    @Test
    fun detailScreensCrossfadeHandlesNullReloadWithoutCrash() {
        var failRide by mutableStateOf(false)
        var screen by mutableIntStateOf(0)
        val rideJson = """{"id":71,"status":"confirmed","pickup_at":"2026-12-20 09:00:00","area":{"id":1,"name":"PJ Section 14"},"destination":"Detail Hall","seats":1,"passenger":{"id":1,"name":"Animation Tester"},"viewer":{"is_passenger":true,"is_driver":false,"is_staff":false}}"""
        val eventJson = """{"id":50,"title":"Outreach Event","starts_at":"2026-12-20 09:00:00","location":"Main Sanctuary","status":"published"}"""

        setupSignedInDispatcher(
            routes = mapOf(
                "/api/rides/71" to {
                    if (failRide) MockResponse().setResponseCode(500).setBody("""{"error":"Temp glitch"}""")
                    else MockResponse().setBody(rideJson)
                },
                "/api/events/50" to { MockResponse().setBody(eventJson) },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                val nav = rememberNavController()
                FreshScreenSlot(screen) {
                    if (screen == 0) RideDetailScreen(container, nav, id = 71L)
                    else EventDetailScreen(container, nav, id = 50L)
                }
            }
        }

        waitForText("Detail Hall")
        // Trigger PullToRefresh with a failing response, then recover cleanly
        compose.runOnIdle { failRide = true }
        compose.onNode(hasScrollAction()).performTouchInput { swipeDown() }
        compose.waitForIdle()
        compose.runOnIdle { failRide = false }
        compose.onNode(hasScrollAction()).performTouchInput { swipeDown() }
        waitForText("Detail Hall")

        compose.runOnIdle { screen = 1 }
        waitForText("Outreach Event")
        compose.onNode(hasScrollAction()).performTouchInput { swipeDown() }
        waitForText("Outreach Event")
    }

    @Test
    fun rideNewAndTrafficHubAnimatedStatesWorkCleanly() {
        var mode by mutableIntStateOf(0)
        val eventJson = """{"id":50,"title":"Sunday Gathering","starts_at":"2026-12-20 09:00:00","location":"Sanctuary","status":"published","going":10,"my_signup":{"status":"going"}}"""
        val pendingRidePage = """{"items":[{"id":301,"status":"pending_approval","pickup_at":"2026-11-29 08:30:00","area":{"id":1,"name":"PJ Section 14"},"destination":"Church Hall","seats":1,"passenger":{"id":7,"name":"Sis Mary"},"viewer":{"is_passenger":false,"is_driver":false,"is_staff":true}}],"total":1,"page":1,"page_size":50,"total_pages":1}"""

        setupSignedInDispatcher(
            routes = mapOf(
                "/api/events/50" to { MockResponse().setBody(eventJson) },
                "/api/admin/rides" to { MockResponse().setBody(pendingRidePage) },
                "/api/rides/301/approve" to { MockResponse().setBody("""{"ok":true}""") },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                val nav = rememberNavController()
                FreshScreenSlot(mode) {
                    if (mode == 0) {
                        RideNewScreen(container, nav, eventId = 50L)
                    } else {
                        TrafficHubScreen(container, nav)
                    }
                }
            }
        }

        // RideNewScreen AnimatedVisibility sections: Round trip -> Return only -> Outbound -> Proxy toggle -> Save address
        waitForText("Sunday Gathering")
        compose.onNodeWithText(res(R.string.ride_new_trip_type_round)).performClick()
        waitForText(res(R.string.ride_new_return_time))
        compose.onNodeWithText(res(R.string.ride_new_trip_type_return)).performClick()
        waitForText(res(R.string.ride_new_time_hint_return))
        compose.onNodeWithText(res(R.string.ride_new_trip_type_outbound)).performClick()
        compose.onNodeWithText(res(R.string.ride_new_is_proxy)).performScrollTo()
        compose.onAllNodes(isToggleable()).onFirst().performClick()
        waitForText(res(R.string.ride_new_frequent_riders))
        compose.onAllNodes(isToggleable()).onFirst().performClick()
        compose.waitUntil(5_000) {
            compose.onAllNodesWithText(res(R.string.ride_new_frequent_riders)).fetchSemanticsNodes().isEmpty()
        }
        compose.waitForIdle()
        compose.onNodeWithText(res(R.string.ride_new_address)).performScrollTo().performTextInput("22 Jalan SS2")
        compose.waitForIdle()
        compose.onNodeWithText(res(R.string.ride_new_save_address)).performScrollTo()
        compose.waitForIdle()
        compose.onNodeWithText(res(R.string.ride_new_save_address)).performClick()
        compose.waitForIdle()
        waitForText(res(R.string.ride_new_save_address_label))

        // TrafficHubScreen tabs & batch selection bar enter/exit
        compose.runOnIdle { mode = 1 }
        waitForText(res(R.string.traffic_tab_rides))
        compose.onNodeWithText(res(R.string.traffic_tab_rides)).performClick()
        waitForText("Sis Mary")
        compose.onAllNodes(isToggleable()).onFirst().performClick()
        waitForText(res(R.string.traffic_rides_selected_count, 1))
        compose.onNodeWithText(res(R.string.traffic_rides_batch_clear)).performClick()
        compose.waitUntil(5_000) {
            compose.onAllNodesWithText(res(R.string.traffic_rides_batch_clear)).fetchSemanticsNodes().isEmpty()
        }
        compose.onNodeWithText(res(R.string.traffic_tab_live)).performClick()
        compose.onNodeWithText(res(R.string.traffic_tab_areas)).performClick()
        compose.onNodeWithText(res(R.string.traffic_tab_plans)).performClick()
    }
}
