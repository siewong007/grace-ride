package org.pjstatecm.graceride

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import okhttp3.mockwebserver.Dispatcher
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import okhttp3.mockwebserver.RecordedRequest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.pjstatecm.graceride.data.cache.OfflineActionQueue
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.NotifLinkRouter

@OptIn(ExperimentalCoroutinesApi::class)
class PublicAndSecurityParityTest {

    private lateinit var server: MockWebServer
    private lateinit var store: InMemoryStore
    private lateinit var container: AppContainer

    @Before
    fun setup() {
        Dispatchers.setMain(UnconfinedTestDispatcher())
        server = MockWebServer()
        server.start()
        store = InMemoryStore()
        container = AppContainer(null, store)
        container.setBaseUrl(server.url("/").toString())
    }

    @After
    fun teardown() {
        runCatching { server.shutdown() }
        Dispatchers.resetMain()
    }

    @Test
    fun resolvePublicHandlesInviteVisitCareTermsLicensesOnly() {
        assertEquals(AppRoute.Invite("abc123"), NotifLinkRouter.resolvePublic("/invite/abc123"))
        assertEquals(AppRoute.Visit("vis999"), NotifLinkRouter.resolvePublic("/visit/vis999?x=1"))
        assertEquals(AppRoute.CarePass("care77"), NotifLinkRouter.resolvePublic("/care/care77"))
        assertEquals(AppRoute.TermsPublic, NotifLinkRouter.resolvePublic("/terms"))
        assertEquals(AppRoute.Licenses, NotifLinkRouter.resolvePublic("/licences"))
        assertEquals(AppRoute.Licenses, NotifLinkRouter.resolvePublic("/licenses"))

        assertNull(NotifLinkRouter.resolvePublic("/rides/12"))
        assertNull(NotifLinkRouter.resolvePublic("//evil.example/invite/abc"))
        assertNull(NotifLinkRouter.resolvePublic("/api/auth/logout"))
    }

    @Test
    fun publicFlowRepositoryFetchesPublicEndpoints() = runBlocking {
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val p = request.path.orEmpty()
                return when {
                    p == "/api/auth/turnstile" -> MockResponse().setBody("""{"enabled":true,"site_key":"pk_test_123"}""")
                    p == "/api/invites/tok1" -> MockResponse().setBody("""{"kind":"member","email":"new@example.org","name":"Alice","family_name":"Lim"}""")
                    p == "/api/visit/v1" -> MockResponse().setBody("""{"state":"open","name":"Visitor Bob","host":"John","event":{"title":"Sunday Service","starts_at":"2026-12-06 09:00:00","location":"Sanctuary"}}""")
                    p == "/api/care/c1" && request.method == "GET" -> MockResponse().setBody("""{"status":"confirmed","destination":"Church","rider_name":"Grandma Lim","pickup_at":"2026-12-06 08:30:00"}""")
                    p == "/api/care/c1/readiness" && request.method == "POST" -> MockResponse().setBody("""{"ok":true}""")
                    else -> MockResponse().setResponseCode(404)
                }
            }
        }

        val ts = container.publicFlow.turnstileConfig()
        assertTrue(ts.enabled)
        assertEquals("pk_test_123", ts.siteKey)

        val inv = container.publicFlow.invitePreview("tok1")
        assertEquals("Alice", inv.name)
        assertEquals("Lim", inv.familyName)

        val vis = container.publicFlow.visitPreview("v1")
        assertEquals("open", vis.state)
        assertEquals("Visitor Bob", vis.name)

        val care = container.publicFlow.careStatus("c1")
        assertEquals("Grandma Lim", care.riderName)

        val res = container.publicFlow.careReadiness("c1", "ready", "At lobby")
        assertTrue(res.ok)
    }

    @Test
    fun offlineActionQueuePersistsDeduplicatesFlushesAndHandlesErrors() = runBlocking {
        val q1 = OfflineActionQueue(store)
        q1.enqueue(rideId = 10L, kind = OfflineActionQueue.KIND_ARRIVED_GATE)
        q1.enqueue(rideId = 10L, kind = OfflineActionQueue.KIND_ARRIVED_GATE)
        q1.enqueue(rideId = 11L, kind = OfflineActionQueue.KIND_READINESS, readinessStatus = "ready", notes = "lobby")
        q1.enqueue(rideId = 12L, kind = OfflineActionQueue.KIND_PICKUP)

        assertEquals(3, q1.pending.value.size)
        assertEquals(1, q1.pendingForRide(10L).size)

        // Survives fresh construction using the same SecureStore
        val q2 = OfflineActionQueue(store)
        assertEquals(3, q2.pending.value.size)

        val calls = mutableListOf<String>()
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val p = request.path.orEmpty()
                calls += p
                return when (p) {
                    "/api/rides/10/arrived-gate" -> MockResponse().setBody("""{"ok":true}""")
                    "/api/rides/11/readiness" -> MockResponse().setResponseCode(409).setBody("""{"error":"Conflict"}""")
                    "/api/rides/12/pickup" -> MockResponse().setResponseCode(503).setBody("""{"error":"Unavailable"}""")
                    else -> MockResponse().setResponseCode(404)
                }
            }
        }

        // First flush: 10 succeeds (synced=1, removed), 11 returns 409 (dropped without incrementing synced), 12 returns 503 (stops & kept)
        val synced = q2.flush(container.rides)
        assertEquals(1, synced)
        assertEquals(1, q2.pending.value.size)
        assertEquals(12L, q2.pending.value.first().rideId)

        q2.clear()
        assertFalse(q2.pending.value.isNotEmpty())
    }
}
