package org.pjstatecm.graceride

import app.cash.turbine.test
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
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.AuthEvent
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.apiCall
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.data.session.InMemoryStore

object Fixtures {
    const val ME_MEMBER = """{"id":1,"name":"Test Member","email":"member@example.org","status":"active",
        "permissions":["rides.view","rides.create","events.view","costs.view","drive.view"],"mfa_ok":true}"""
    const val ME_LOCKED = """{"id":1,"name":"Test Member","email":"member@example.org","status":"active","locked":true}"""
    const val ME_PENDING = """{"id":1,"name":"Test Member","email":"member@example.org","status":"pending"}"""
    const val RIDE = """{"id":10,"status":"confirmed","pickup_at":"2026-11-22 09:30:00",
        "area":{"id":2,"name":"PJ Section 14"},"destination":"Church Hall","seats":2,
        "passenger":{"id":1,"name":"Test Member"},"viewer":{"is_passenger":true,"is_driver":false,"is_staff":false}}"""
    const val EVENT = """{"id":20,"title":"Fellowship Lunch","starts_at":"2026-11-23 12:00:00","going":10,"capacity":40,"spots_left":30}"""
    const val SHARE = """{"id":30,"settlement_id":3,"title":"Fuel Nov","event_title":"Fellowship Lunch",
        "amount_cents":1250,"status":"unpaid","payee_name":"Finance","payee_qr_file_id":7}"""
    const val ANN = """{"id":40,"title":"Welcome","body":"Grace Ride is live","pinned":true}"""
}

@OptIn(ExperimentalCoroutinesApi::class)
class ApiSessionTest {

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

    @Test fun errorBodyBecomesApiExceptionWithCode() = runTest {
        server.enqueue(
            MockResponse().setResponseCode(409)
                .setBody("""{"error":"Ride moved on","code":"RIDE_STATUS_CHANGED"}""")
        )
        try {
            apiCall(container.authEvents) { container.api.ride(10) }
            throw AssertionError("expected ApiException")
        } catch (e: ApiException) {
            assertEquals(409, e.status)
            assertEquals("RIDE_STATUS_CHANGED", e.code)
            assertEquals("Ride moved on", e.message)
        }
    }

    @Test fun unauthorizedEmitsSessionEvent() = runTest {
        server.enqueue(MockResponse().setResponseCode(401).setBody("""{"error":"Sign in","code":"UNAUTHORIZED"}"""))
        container.authEvents.events.test {
            try {
                apiCall(container.authEvents) { container.api.me() }
            } catch (_: ApiException) { }
            assertEquals(AuthEvent.Unauthorized, awaitItem())
            cancelAndIgnoreRemainingEvents()
        }
    }

    @Test fun lockedEmitsSessionEvent() = runTest {
        server.enqueue(MockResponse().setResponseCode(423).setBody("""{"error":"Locked","code":"SESSION_LOCKED"}"""))
        container.authEvents.events.test {
            try {
                apiCall(container.authEvents) { container.api.me() }
            } catch (_: ApiException) { }
            assertEquals(AuthEvent.Locked, awaitItem())
            cancelAndIgnoreRemainingEvents()
        }
    }

    @Test fun refreshSignedIn() = runTest {
        server.enqueue(MockResponse().setBody(Fixtures.ME_MEMBER))
        container.auth.refresh()
        val state = container.auth.state.value
        assertTrue(state is AuthState.SignedIn)
        assertEquals("Test Member", (state as AuthState.SignedIn).me.name)
        assertTrue(state.me.can("rides", "create"))
    }

    @Test fun refreshLockedSession() = runTest {
        server.enqueue(MockResponse().setBody(Fixtures.ME_LOCKED))
        container.auth.refresh()
        assertTrue(container.auth.state.value is AuthState.Locked)
    }

    @Test fun refreshPendingMember() = runTest {
        server.enqueue(MockResponse().setBody(Fixtures.ME_PENDING))
        container.auth.refresh()
        assertTrue(container.auth.state.value is AuthState.Pending)
    }

    @Test fun refresh401SignsOut() = runTest {
        server.enqueue(MockResponse().setResponseCode(401).setBody("{}"))
        container.auth.refresh()
        assertTrue(container.auth.state.value is AuthState.SignedOut)
    }

    @Test fun devLoginStoresSessionCookie() = runTest {
        container.lang = "zh"
        server.enqueue(
            MockResponse().setBody("""{"ok":true}""")
                .addHeader("Set-Cookie", "gr_session=${"t".repeat(48)}; Path=/; HttpOnly")
        )
        server.enqueue(MockResponse().setBody(Fixtures.ME_MEMBER))
        container.auth.devLogin("member@example.org")
        assertTrue(container.auth.state.value is AuthState.SignedIn)
        val cookieUrl = server.url("/api/auth/me")
        val cookies = container.cookieJar.loadForRequest(cookieUrl)
        assertEquals(1, cookies.size)
        assertEquals("gr_session", cookies[0].name)
        // The request carried the session cookie.
        server.takeRequest() // dev-login
        val meReq = server.takeRequest()
        assertTrue(meReq.getHeader("Cookie")?.contains("gr_session") == true)
        assertEquals("zh", meReq.getHeader("Accept-Language"))
    }

    @Test fun logoutClearsCookies() = runTest {
        server.enqueue(
            MockResponse().setBody("""{"ok":true}""")
                .addHeader("Set-Cookie", "gr_session=${"t".repeat(48)}; Path=/; HttpOnly")
        )
        server.enqueue(MockResponse().setBody(Fixtures.ME_MEMBER))
        container.auth.devLogin("member@example.org")
        server.enqueue(MockResponse().setBody("""{"ok":true}"""))
        container.auth.logout()
        assertTrue(container.auth.state.value is AuthState.SignedOut)
        assertEquals(0, container.cookieJar.loadForRequest(server.url("/api/auth/me")).size)
    }

    @Test fun noOriginHeaderSent() = runTest {
        server.enqueue(MockResponse().setBody(Fixtures.ME_MEMBER))
        container.auth.refresh()
        val req = server.takeRequest()
        assertEquals(null, req.getHeader("Origin"))
    }
}
