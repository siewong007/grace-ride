package org.pjstatecm.graceride

import android.content.ContextWrapper
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.data.session.OfflineSnapshotStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.notifications.NotificationHelper
import org.pjstatecm.graceride.ui.screens.login.LoginViewModel

@OptIn(ExperimentalCoroutinesApi::class)
class SecurityAndOfflineTest {

    private lateinit var server: MockWebServer
    private lateinit var store: InMemoryStore
    private lateinit var container: AppContainer

    @Before fun setup() {
        Dispatchers.setMain(UnconfinedTestDispatcher())
        server = MockWebServer()
        server.start()
        store = InMemoryStore()
        container = AppContainer(null, store)
        container.setBaseUrl(server.url("/").toString())
    }

    @After fun teardown() {
        runCatching { server.shutdown() }
        Dispatchers.resetMain()
    }

    @Test fun authRepositoryRestoresCachedMeOnOfflineColdStartWhenCookiesPresent() = runBlocking {
        server.enqueue(
            MockResponse()
                .addHeader("Set-Cookie", "gr_session=tok123; Path=/; Max-Age=3600; HttpOnly")
                .setBody(Fixtures.ME_MEMBER)
        )
        container.auth.refresh()
        assertTrue(container.auth.state.value is AuthState.SignedIn)
        assertTrue(container.cookieJar.hasCookies())
        assertNotNull(store.getString("cached_me_v1"))

        val offlineContainer = AppContainer(null, store)
        offlineContainer.setBaseUrl(server.url("/").toString())
        server.shutdown() // force network IOException (status == 0)

        offlineContainer.auth.refresh()
        val restored = offlineContainer.auth.state.value as? AuthState.SignedIn
        assertNotNull("Expected cached SignedIn state during offline cold start", restored)
        assertEquals("Test Member", restored!!.me.name)
    }

    @Test fun authRepositoryUnauthorizedAndLogoutClearAllLocalArtifacts() = runBlocking {
        server.enqueue(
            MockResponse()
                .addHeader("Set-Cookie", "gr_session=tok123; Path=/; Max-Age=3600; HttpOnly")
                .setBody(Fixtures.ME_MEMBER)
        )
        container.auth.refresh()
        val today = KlTime.today()
        val rideJson = """{"id":10,"status":"confirmed","pickup_at":"$today 08:30:00",
            "area":{"id":2,"name":"PJ Section 14"},"destination":"Church Hall","seats":1,
            "passenger":{"id":1,"name":"Test Member"},"viewer":{"is_passenger":true,"is_driver":false,"is_staff":false}}"""
        OfflineSnapshotStore.save(store, 1L, listOf(apiJson.decodeFromString(Ride.serializer(), rideJson)))
        store.putString("notif_hide_lock_details", "1")

        var clearedCount = 0
        container.onSessionCleared = { clearedCount++ }

        assertNotNull(OfflineSnapshotStore.load(store, 1L))
        assertTrue(container.cookieJar.hasCookies())

        // Simulate 401 on refresh()
        server.enqueue(MockResponse().setResponseCode(401).setBody("""{"error":"expired"}"""))
        container.auth.refresh()

        assertEquals(AuthState.SignedOut, container.auth.state.value)
        assertFalse(container.cookieJar.hasCookies())
        assertNull(store.getString("cached_me_v1"))
        assertNull(store.getString("notif_hide_lock_details"))
        assertNull(OfflineSnapshotStore.load(store, 1L))
        assertEquals(1, clearedCount)

        // Simulate logout() cleanup even if server fails
        server.enqueue(
            MockResponse()
                .addHeader("Set-Cookie", "gr_session=tok456; Path=/; Max-Age=3600; HttpOnly")
                .setBody(Fixtures.ME_MEMBER)
        )
        container.auth.refresh()
        store.putString("notif_hide_lock_details", "1")
        OfflineSnapshotStore.save(store, 1L, listOf(apiJson.decodeFromString(Ride.serializer(), rideJson)))
        server.enqueue(MockResponse().setResponseCode(500).setBody("""{"error":"down"}"""))
        container.auth.logout()

        assertEquals(AuthState.SignedOut, container.auth.state.value)
        assertFalse(container.cookieJar.hasCookies())
        assertNull(store.getString("cached_me_v1"))
        assertNull(store.getString("notif_hide_lock_details"))
        assertNull(OfflineSnapshotStore.load(store, 1L))
        assertEquals(2, clearedCount)
    }

    @Test fun loginViewModelValidatesEmailAndServerUrl() {
        store.putString("last_email", "saved@example.org")
        val vm = LoginViewModel(container, devAvailable = false)
        assertEquals("saved@example.org", vm.state.value.email)

        vm.applyServer("not-a-valid-url")
        assertEquals(R.string.login_invalid_server, vm.state.value.errorRes)

        val dummyCtx = ContextWrapper(null)
        vm.setEmail("   ")
        vm.signInWithPasskey(dummyCtx)
        assertEquals(R.string.login_email_required, vm.state.value.errorRes)

        vm.setEmail("missing-at-symbol")
        vm.signInWithPasskey(dummyCtx)
        assertEquals(R.string.login_email_required, vm.state.value.errorRes)
    }

    @Test fun notificationHelperRoutesChannelsByPriorityAndType() {
        assertEquals(NotificationHelper.CHANNEL_RIDES_URGENT, NotificationHelper.channelFor("cost", "other", "urgent"))
        assertEquals(NotificationHelper.CHANNEL_CHAT, NotificationHelper.channelFor("ride", "trip_chat", "normal"))
        assertEquals(NotificationHelper.CHANNEL_RIDES_UPDATES, NotificationHelper.channelFor("ride", "assigned", "normal"))
        assertEquals(NotificationHelper.CHANNEL_COMMUNITY, NotificationHelper.channelFor("announcement", "news", "low"))
    }
}
