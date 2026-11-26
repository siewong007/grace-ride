package org.pjstatecm.graceride

import app.cash.turbine.test
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.util.concurrent.TimeUnit
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.NotificationPageDto
import org.pjstatecm.graceride.data.api.NotificationPrefsDto
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.notifications.RealtimeStreamClient
import org.pjstatecm.graceride.ui.NotifDestination
import org.pjstatecm.graceride.ui.NotifLinkRouter
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.screens.notifications.NotificationsViewModel
import org.pjstatecm.graceride.ui.screens.profile.ProfileViewModel
import kotlin.time.Duration.Companion.seconds

@OptIn(ExperimentalCoroutinesApi::class)
class NotificationsTest {

    private lateinit var server: MockWebServer
    private lateinit var container: AppContainer

    @Before fun setup() {
        Dispatchers.setMain(UnconfinedTestDispatcher())
        server = MockWebServer()
        server.start()
        container = AppContainer(null, InMemoryStore())
        container.setBaseUrl(server.url("/").toString())
    }

    @After fun teardown() {
        server.shutdown()
        Dispatchers.resetMain()
    }

    private fun json(body: String) = MockResponse().setBody(body)

    @Test fun notifLinkRouterMapsNativeAndWebDestinationsSafely() {
        assertEquals(NotifDestination.Native(Routes.HOME), NotifLinkRouter.resolve("/"))
        assertEquals(NotifDestination.Native(Routes.rideDetail(42)), NotifLinkRouter.resolve("/rides/42?action=readiness"))
        assertEquals(NotifDestination.Native(Routes.RIDES), NotifLinkRouter.resolve("/rides"))
        assertEquals(NotifDestination.Native(Routes.eventDetail(9)), NotifLinkRouter.resolve("/events/9#top"))
        assertEquals(NotifDestination.Native(Routes.COSTS), NotifLinkRouter.resolve("/costs"))
        assertEquals(NotifDestination.Web("/settlements"), NotifLinkRouter.resolve("/settlements"))
        assertEquals(NotifDestination.Web("/settlements/5"), NotifLinkRouter.resolve("/settlements/5"))
        assertEquals(NotifDestination.Native(Routes.DRIVE_HUB), NotifLinkRouter.resolve("/drive"))
        assertEquals(NotifDestination.Native(Routes.rideNew()), NotifLinkRouter.resolve("/ride-new"))
        assertEquals(NotifDestination.Native(Routes.DRIVER_SETTINGS), NotifLinkRouter.resolve("/driver"))
        assertEquals(NotifDestination.Native(Routes.PROFILE), NotifLinkRouter.resolve("/me"))
        assertEquals(NotifDestination.Web("/help?ride=12"), NotifLinkRouter.resolve("/help?ride=12"))
        assertEquals(NotifDestination.Web("/admin/rides"), NotifLinkRouter.resolve("/admin/rides"))
        assertEquals(NotifDestination.None, NotifLinkRouter.resolve("//evil.example/phish"))
        assertEquals(NotifDestination.None, NotifLinkRouter.resolve("/api/auth/logout"))
        assertEquals(NotifDestination.None, NotifLinkRouter.resolve("javascript:alert(1)"))
    }

    @Test fun notifLinkRouterExtractsTrustedIntentUrisOnly() {
        assertEquals(
            "/rides/42?action=readiness",
            NotifLinkRouter.extractPathFromIntentUri("https://graceride.saliminn.my/rides/42?action=readiness"),
        )
        assertEquals(
            "/drive",
            NotifLinkRouter.extractPathFromIntentUri("graceride://open/drive"),
        )
        assertEquals(null, NotifLinkRouter.extractPathFromIntentUri("https://evil.example/rides/42"))
        assertEquals(null, NotifLinkRouter.extractPathFromIntentUri("http://graceride.saliminn.my/rides/42"))
        assertEquals(null, NotifLinkRouter.extractPathFromIntentUri("javascript:alert(1)"))
    }

    @Test fun dtosDecodeNotificationPageAndPrefs() {
        val pageJson = """
            {
              "items": [
                {"id":10,"title":"Ride reminder","body":"Ready?","link":"/rides/7?action=readiness","read":false,"created_at":"2026-11-24 08:00:00","category":"ride","event_type":"ride_reminder","priority":"urgent","entity_type":"ride","entity_id":7}
              ],
              "next_before": 10,
              "unread": 3
            }
        """.trimIndent()
        val page = apiJson.decodeFromString<NotificationPageDto>(pageJson)
        assertEquals(1, page.items.size)
        assertEquals("ride", page.items[0].category)
        assertEquals("urgent", page.items[0].priority)
        assertEquals(10L, page.nextBefore)
        assertEquals(3, page.unread)

        val prefsJson = """
            {
              "push_opt_in": true,
              "whatsapp_opt_in": false,
              "telegram_chat_id": "123456789",
              "telegram_blocked": true,
              "readiness_prompt_mins": 45,
              "driver_remind_mins": 90,
              "muted_categories": ["cost"],
              "quiet_hours_start": "22:00",
              "quiet_hours_end": "07:00",
              "hide_lock_details": true
            }
        """.trimIndent()
        val prefs = apiJson.decodeFromString<NotificationPrefsDto>(prefsJson)
        assertTrue(prefs.pushOptIn)
        assertFalse(prefs.whatsappOptIn)
        assertEquals(90, prefs.driverRemindMins)
        assertEquals(listOf("cost"), prefs.mutedCategories)
        assertEquals("22:00", prefs.quietHoursStart)
        assertTrue(prefs.hideLockDetails)
    }

    @Test fun authUpdateUnreadUpdatesStateWithoutNetworkCall() = runBlocking {
        server.enqueue(json(Fixtures.ME_MEMBER))
        container.auth.refresh()
        val before = container.auth.me!!.unread
        container.auth.updateUnread(before + 5)
        val signedIn = container.auth.state.value as AuthState.SignedIn
        assertEquals(before + 5, signedIn.me.unread)
        assertEquals(1, server.requestCount)
    }

    @Test fun notificationsViewModelFiltersPaginatesAndMutates() = runBlocking {
        val page1 = """
            {
              "items": [
                {"id":20,"title":"Newest","body":"Body","link":"/rides/5","read":false,"created_at":"2026-11-24 09:00:00","category":"ride"}
              ],
              "next_before": 20,
              "unread": 2
            }
        """.trimIndent()
        val filtered = """
            {
              "items": [
                {"id":20,"title":"Newest","body":"Body","link":"/rides/5","read":false,"created_at":"2026-11-24 09:00:00","category":"ride"}
              ],
              "next_before": 15,
              "unread": 2
            }
        """.trimIndent()
        val page2 = """
            {
              "items": [
                {"id":14,"title":"Older","body":"Body2","link":"/rides/4","read":false,"created_at":"2026-11-20 09:00:00","category":"ride"}
              ],
              "next_before": null,
              "unread": 2
            }
        """.trimIndent()
        server.enqueue(json(page1))
        val vm = NotificationsViewModel(container)
        kotlinx.coroutines.withTimeout(5.seconds) {
            vm.state.test {
                var st = awaitItem()
                while (st.loading) st = awaitItem()
                assertEquals(1, st.items.size)
                assertEquals(20L, st.nextBefore)
                cancelAndIgnoreRemainingEvents()
            }
        }
        val reqInit = server.takeRequest(5, TimeUnit.SECONDS)
        assertNotNull(reqInit)

        server.enqueue(json(filtered))
        vm.setCategory("ride")
        val reqFilter = server.takeRequest(5, TimeUnit.SECONDS)
        assertNotNull(reqFilter)
        assertTrue(reqFilter!!.path!!.contains("category=ride"))
        kotlinx.coroutines.withTimeout(5.seconds) {
            vm.state.test {
                var st = awaitItem()
                while (st.loading || st.nextBefore != 15L) st = awaitItem()
                cancelAndIgnoreRemainingEvents()
            }
        }

        server.enqueue(json(page2))
        vm.loadMore()
        val reqMore = server.takeRequest(5, TimeUnit.SECONDS)
        assertNotNull(reqMore)
        assertTrue(reqMore!!.path!!.contains("before=15"))
        kotlinx.coroutines.withTimeout(5.seconds) {
            vm.state.test {
                var st = awaitItem()
                while (st.loadingMore || st.items.size < 2) st = awaitItem()
                cancelAndIgnoreRemainingEvents()
            }
        }
        assertEquals(2, vm.state.value.items.size)

        server.enqueue(json("""{"ok":true,"unread":1}"""))
        vm.toggleRead(vm.state.value.items[0])
        assertTrue(vm.state.value.items[0].read)
        assertEquals("/api/notifications/20/read", server.takeRequest(5, TimeUnit.SECONDS)?.path)

        server.enqueue(json("""{"ok":true,"unread":2}"""))
        vm.toggleRead(vm.state.value.items[0])
        assertFalse(vm.state.value.items[0].read)
        assertEquals("/api/notifications/20/unread", server.takeRequest(5, TimeUnit.SECONDS)?.path)

        server.enqueue(json("""{"ok":true,"unread":1}"""))
        vm.delete(vm.state.value.items[1])
        assertEquals(1, vm.state.value.items.size)
        val delReq = server.takeRequest(5, TimeUnit.SECONDS)
        assertNotNull(delReq)
        assertEquals("DELETE", delReq!!.method)
        assertEquals("/api/notifications/14", delReq.path)
    }

    @Test fun profileViewModelSavesNameAndPhoneWithoutSendingTelegramChatId() = runBlocking {
        server.enqueue(json(Fixtures.ME_MEMBER))
        container.auth.refresh()
        server.takeRequest()
        server.enqueue(json("[]"))
        val vm = ProfileViewModel(container)
        server.takeRequest()

        server.enqueue(json("""{"ok":true}"""))
        server.enqueue(json(Fixtures.ME_MEMBER))
        vm.saveProfile("Alice", "+60123456789")
        kotlinx.coroutines.withTimeout(5.seconds) {
            vm.state.test {
                var st = awaitItem()
                while (st.message != "saved") st = awaitItem()
                cancelAndIgnoreRemainingEvents()
            }
        }
        val putReq = server.takeRequest(5, TimeUnit.SECONDS)
        assertNotNull(putReq)
        val body = apiJson.parseToJsonElement(putReq!!.body.readUtf8()).jsonObject
        assertEquals("Alice", body["name"]?.jsonPrimitive?.content)
        assertEquals("+60123456789", body["phone"]?.jsonPrimitive?.content)
        assertFalse(body.containsKey("telegram_chat_id"))
        assertEquals(4, server.requestCount)
    }

    @Test fun syncNotificationsPersistsPerUserWatermarkAndDeduplicates() = runBlocking {
        val store = InMemoryStore()
        val syncContainer = AppContainer(null, store)
        syncContainer.setBaseUrl(server.url("/").toString())

        // Initial baseline establishes watermark at 10 without surfacing old items.
        val initialPage = """
            {
              "items": [
                {"id":10,"title":"Existing","body":"Already there","link":"/rides/1","read":false,"created_at":"2026-11-24 08:00:00","category":"ride"}
              ],
              "next_before": null,
              "unread": 1
            }
        """.trimIndent()
        server.enqueue(json(initialPage))
        val first = RealtimeStreamClient.syncNotifications(
            community = syncContainer.community,
            auth = syncContainer.auth,
            store = store,
            userId = 1L,
            surfaceNew = true,
        )
        assertEquals(10L, first.newWatermark)
        assertTrue(first.surfaced.isEmpty())
        assertEquals("10", store.getString(RealtimeStreamClient.watermarkKey(1L)))

        // Subsequent poll surfaces only unread items above watermark 10.
        val secondPage = """
            {
              "items": [
                {"id":12,"title":"New unread","body":"Fresh","link":"/rides/2","read":false,"created_at":"2026-11-24 09:00:00","category":"ride"},
                {"id":11,"title":"Already read","body":"Read on web","link":"/rides/1","read":true,"created_at":"2026-11-24 08:30:00","category":"ride"},
                {"id":10,"title":"Existing","body":"Already there","link":"/rides/1","read":false,"created_at":"2026-11-24 08:00:00","category":"ride"}
              ],
              "next_before": null,
              "unread": 2
            }
        """.trimIndent()
        server.enqueue(json(secondPage))
        val second = RealtimeStreamClient.syncNotifications(
            community = syncContainer.community,
            auth = syncContainer.auth,
            store = store,
            userId = 1L,
            surfaceNew = true,
        )
        assertEquals(12L, second.newWatermark)
        assertEquals(listOf(12L), second.surfaced.map { it.id })
        assertEquals("12", store.getString(RealtimeStreamClient.watermarkKey(1L)))

        // Re-running with the same page does not duplicate notification 12.
        server.enqueue(json(secondPage))
        val third = RealtimeStreamClient.syncNotifications(
            community = syncContainer.community,
            auth = syncContainer.auth,
            store = store,
            userId = 1L,
            surfaceNew = true,
        )
        assertEquals(12L, third.newWatermark)
        assertTrue(third.surfaced.isEmpty())

        // Different user maintains an independent baseline watermark.
        server.enqueue(json(secondPage))
        val otherUser = RealtimeStreamClient.syncNotifications(
            community = syncContainer.community,
            auth = syncContainer.auth,
            store = store,
            userId = 2L,
            surfaceNew = true,
        )
        assertEquals(12L, otherUser.newWatermark)
        assertTrue(otherUser.surfaced.isEmpty())
        assertEquals("12", store.getString(RealtimeStreamClient.watermarkKey(2L)))
    }

    @Test fun notifLinkRouterParsesCheckinCodeAndProxyRideParams() {
        assertEquals(
            NotifDestination.Native(Routes.eventDetail(42L, checkinCode = "ABC123")),
            NotifLinkRouter.resolve("/events/42/checkin?code=ABC123"),
        )
        assertEquals(
            NotifDestination.Native(Routes.rideNew(eventId = 9L, proxy = true)),
            NotifLinkRouter.resolve("/ride-new?event=9&proxy=1"),
        )
    }

    @Test fun shouldSurfaceNotificationEnforcesOptInMutedQuietHoursAndUrgentBypass() {
        val normalRide = org.pjstatecm.graceride.data.api.NotificationItem(
            id = 1L,
            title = "Ride confirmed",
            category = "ride",
            priority = "normal",
        )
        val urgentRide = normalRide.copy(id = 2L, priority = "urgent")
        val prefs = NotificationPrefsDto(
            pushOptIn = true,
            mutedCategories = listOf("event"),
            quietHoursStart = "22:00",
            quietHoursEnd = "07:00",
        )

        assertFalse(RealtimeStreamClient.shouldSurfaceNotification(normalRide, prefs.copy(pushOptIn = false), "10:00"))
        assertFalse(RealtimeStreamClient.shouldSurfaceNotification(urgentRide, prefs.copy(pushOptIn = false), "10:00"))
        assertFalse(
            RealtimeStreamClient.shouldSurfaceNotification(
                normalRide.copy(category = "event"),
                prefs,
                "10:00",
            ),
        )
        assertFalse(RealtimeStreamClient.shouldSurfaceNotification(normalRide, prefs, "23:30"))
        assertFalse(RealtimeStreamClient.shouldSurfaceNotification(normalRide, prefs, "06:15"))
        assertTrue(RealtimeStreamClient.shouldSurfaceNotification(normalRide, prefs, "10:00"))
        assertTrue(RealtimeStreamClient.shouldSurfaceNotification(urgentRide, prefs, "23:30"))
    }

    @Test fun notifLinkRouterResolveForRoutesMemberAwayFromAdminAndWebSettlements() {
        val member = Me(
            id = 6L,
            name = "Member",
            email = "member@example.org",
            roles = listOf("member"),
            permissions = listOf("rides.view", "events.view", "costs.view", "settlements.view"),
        )
        assertEquals(NotifDestination.None, NotifLinkRouter.resolveFor("/admin/rides", member))
        assertEquals(NotifDestination.None, NotifLinkRouter.resolveFor("/admin/members", member))
        assertEquals(NotifDestination.None, NotifLinkRouter.resolveFor("/admin", member))
        assertEquals(NotifDestination.Native(Routes.COSTS), NotifLinkRouter.resolveFor("/settlements", member))
        assertEquals(NotifDestination.Native(Routes.COSTS), NotifLinkRouter.resolveFor("/settlements/5", member))
        assertEquals(NotifDestination.Native(Routes.PROXY_RIDERS), NotifLinkRouter.resolveFor("/me/family", member))
        assertEquals(NotifDestination.Native(Routes.eventDetail(9L)), NotifLinkRouter.resolveFor("/events/9", member))

        val finance = Me(
            id = 7L,
            name = "Finance",
            email = "finance@example.org",
            roles = listOf("finance"),
            permissions = listOf("admin_bills.view", "admin_reports.view"),
        )
        assertEquals(NotifDestination.Web("/settlements"), NotifLinkRouter.resolveFor("/settlements", finance))

        val transport = Me(
            id = 8L,
            name = "Transport",
            email = "transport@example.org",
            roles = listOf("transport"),
            permissions = listOf("admin_rides.view"),
        )
        assertEquals(NotifDestination.Web("/admin/rides"), NotifLinkRouter.resolveFor("/admin/rides", transport))
    }

    @Test fun notificationsViewModelStageDeleteSupportsUndoAndCommit() = runBlocking {
        val page = """
            {
              "items": [
                {"id":31,"title":"Event reminder","body":"Tonight","link":"/events/1","read":false,"created_at":"2026-11-25 09:00:00","category":"event"},
                {"id":30,"title":"Cost update","body":"RM 15","link":"/costs","read":true,"created_at":"2026-11-25 08:00:00","category":"cost"}
              ],
              "next_before": null,
              "unread": 1
            }
        """.trimIndent()
        server.enqueue(json(page))
        val vm = NotificationsViewModel(container)
        kotlinx.coroutines.withTimeout(5.seconds) {
            vm.state.test {
                var st = awaitItem()
                while (st.loading) st = awaitItem()
                assertEquals(2, st.items.size)
                cancelAndIgnoreRemainingEvents()
            }
        }
        assertNotNull(server.takeRequest(5, TimeUnit.SECONDS))

        val first = vm.state.value.items[0]
        vm.stageDelete(first)
        assertEquals(listOf(30L), vm.state.value.items.map { it.id })
        assertEquals(0, vm.state.value.unreadCount)
        assertEquals(1, server.requestCount)

        vm.undoDelete(first.id)
        assertEquals(listOf(31L, 30L), vm.state.value.items.map { it.id })
        assertEquals(1, vm.state.value.unreadCount)
        assertEquals(1, vm.state.value.restoreTick)
        assertEquals(1, server.requestCount)

        server.enqueue(json("""{"ok":true,"unread":0}"""))
        vm.stageDelete(first)
        vm.commitDelete(first.id)
        val delReq = server.takeRequest(5, TimeUnit.SECONDS)
        assertNotNull(delReq)
        assertEquals("DELETE", delReq!!.method)
        assertEquals("/api/notifications/31", delReq.path)
    }

    @Test fun syncNotificationsSendsPassivePollHeaderWhenRequested() = runBlocking {
        val page = """{"items":[],"next_before":null,"unread":0}"""
        server.enqueue(json(page))
        RealtimeStreamClient.syncNotifications(
            community = container.community,
            auth = container.auth,
            store = InMemoryStore(),
            userId = 1L,
            surfaceNew = false,
            passive = true,
        )
        val req = server.takeRequest(5, TimeUnit.SECONDS)
        assertNotNull(req)
        assertEquals("1", req!!.getHeader("X-Passive-Poll"))

        server.enqueue(json(page))
        RealtimeStreamClient.syncNotifications(
            community = container.community,
            auth = container.auth,
            store = InMemoryStore(),
            userId = 1L,
            surfaceNew = false,
            passive = false,
        )
        val activeReq = server.takeRequest(5, TimeUnit.SECONDS)
        assertNotNull(activeReq)
        assertEquals(null, activeReq!!.getHeader("X-Passive-Poll"))
    }
}
