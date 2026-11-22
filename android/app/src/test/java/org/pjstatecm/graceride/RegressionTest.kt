package org.pjstatecm.graceride

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Badge
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.apiCall
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.components.AvatarStyle
import org.pjstatecm.graceride.ui.components.BadgeIcons

/** Regressions found by live emulator testing against the real dev backend. */
@OptIn(ExperimentalCoroutinesApi::class)
class RegressionTest {

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

    @Test fun eventGroupsIsADisplayStringNotAnArray() {
        // event_json renders `groups` as joined display names ("全体会员"),
        // while `join_as` (detail only) is the array of {id,name}.
        val json = """{"id":4,"title":"晨走","starts_at":"2026-11-22 07:00:00","kind":"walk",
            "going":0,"groups":"全体会员","join_as":[{"id":1,"name":"全体会员"}]}"""
        val event = apiJson.decodeFromString(EventItem.serializer(), json)
        assertEquals("全体会员", event.groups)
        assertEquals(1, event.joinAs.size)
        assertEquals("全体会员", event.joinAs[0].name)
    }

    @Test fun unexpectedJsonShapeDegradesToErrorNotCrash() = runTest {
        server.enqueue(MockResponse().setBody("""[{"id":"not-a-number","status":"confirmed"}]"""))
        try {
            apiCall(container.authEvents) { container.api.myRides("upcoming") }
            throw AssertionError("expected ApiException")
        } catch (e: ApiException) {
            assertEquals("DECODE_ERROR", e.code)
            assertTrue(e.status != 0) // not misreported as offline
        }
    }

    @Test fun eventItemDeserializesMyRide() {
        val json = """{"id":11,"title":"Worship","starts_at":"2026-11-29 09:00:00","my_ride":{"id":55,"status":"confirmed"}}"""
        val event = apiJson.decodeFromString(EventItem.serializer(), json)
        assertEquals(55L, event.myRide?.id)
        assertEquals("confirmed", event.myRide?.status)
    }

    @Test fun badgeDeserializesCustomIcon() {
        val json = """{"key":"night_owl","char":"夜","icon":"moon","name":"Night Owl","rule":"3 late rides"}"""
        val badge = apiJson.decodeFromString(Badge.serializer(), json)
        assertEquals("moon", badge.icon)
        assertEquals("night_owl", badge.key)
    }

    @Test fun rideNewRouteBuildsQueryCombinations() {
        assertEquals("ride-new", Routes.rideNew())
        assertEquals("ride-new?event=12", Routes.rideNew(eventId = 12L))
        assertEquals("ride-new?proxy=1", Routes.rideNew(proxy = true))
        assertEquals("ride-new?event=12&proxy=1", Routes.rideNew(eventId = 12L, proxy = true))
    }

    @Test fun animatedAvatarsAndBadgeIconsResolve() {
        assertTrue(AvatarStyle.isAnimatedAvatar("grok"))
        assertTrue(AvatarStyle.isAnimatedAvatar("muse"))
        assertFalse(AvatarStyle.isAnimatedAvatar("car"))
        assertFalse(AvatarStyle.isAnimatedAvatar(null))

        val reg = BadgeIcons.resolve(icon = "moon")
        assertTrue(reg is BadgeIcons.ResolvedBadgeIcon.Vector)

        val emoji = BadgeIcons.resolve(icon = "🦉")
        assertEquals(BadgeIcons.ResolvedBadgeIcon.Text("🦉"), emoji)

        val badge = Badge(key = "k", char = "N", icon = null)
        val fallback = BadgeIcons.resolve(badge = badge, fallbackChar = "R")
        assertEquals(BadgeIcons.ResolvedBadgeIcon.Text("R"), fallback)
    }
}
