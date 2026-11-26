package org.pjstatecm.graceride

import kotlinx.coroutines.runBlocking
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.pjstatecm.graceride.data.api.PasskeyStartReq
import org.pjstatecm.graceride.data.api.ProxyRider
import org.pjstatecm.graceride.data.api.ProxyRiderReq
import org.pjstatecm.graceride.data.api.SavedPlaceReq
import org.pjstatecm.graceride.data.api.TurnstileConfigDto
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.NotifDestination
import org.pjstatecm.graceride.ui.NotifLinkRouter
import org.pjstatecm.graceride.ui.Routes

class BugfixRegressionTest {

    private lateinit var server: MockWebServer
    private lateinit var container: AppContainer

    @Before
    fun setup() {
        server = MockWebServer()
        server.start()
        container = AppContainer(null, InMemoryStore())
        container.setBaseUrl(server.url("/").toString())
    }

    @After
    fun teardown() {
        server.shutdown()
    }

    @Test
    fun notifLinkRouterHandlesHomeCostsAndSettlementsWebFallback() {
        assertEquals(NotifDestination.Native(Routes.HOME), NotifLinkRouter.resolve("/"))
        assertEquals(NotifDestination.Native(Routes.COSTS), NotifLinkRouter.resolve("/costs?tab=unpaid"))
        assertEquals(NotifDestination.Web("/settlements"), NotifLinkRouter.resolve("/settlements"))
        assertEquals(NotifDestination.Web("/settlements/5?tab=1"), NotifLinkRouter.resolve("/settlements/5?tab=1"))
    }

    @Test
    fun dtoFieldsEncodeAndDecodeNewAuditProperties() {
        val cfg = apiJson.decodeFromString<TurnstileConfigDto>(
            """{"enabled":true,"site_key":"0x4AAAAAAA"}""",
        )
        assertTrue(cfg.enabled)
        assertEquals("0x4AAAAAAA", cfg.siteKey)

        val pkReqJson = apiJson.encodeToString(
            PasskeyStartReq.serializer(),
            PasskeyStartReq(email = "m@example.org", turnstile_token = "tok-123"),
        )
        assertTrue(pkReqJson.contains(""""turnstile_token":"tok-123""""))

        val rider = apiJson.decodeFromString<ProxyRider>(
            """{"id":7,"name":"Mum","caregiver_user_id":42}""",
        )
        assertEquals(42L, rider.caregiverUserId)

        val riderReqJson = apiJson.encodeToString(
            ProxyRiderReq.serializer(),
            ProxyRiderReq(name = "Mum", caregiver_user_id = 42L),
        )
        assertTrue(riderReqJson.contains(""""caregiver_user_id":42"""))

        val clearReqJson = apiJson.encodeToString(
            SavedPlaceReq.serializer(),
            SavedPlaceReq(label = "Home", address = "12 Jalan SS2", clear_coords = true),
        )
        assertTrue(clearReqJson.contains(""""clear_coords":true"""))
    }

    @Test
    fun authRepositoryCallsTurnstileAndPasskeyReauthEndpoints() = runBlocking {
        server.enqueue(MockResponse().setBody("""{"enabled":true,"site_key":"0x4TestKey"}"""))
        val cfg = container.auth.turnstileConfig()
        assertTrue(cfg.enabled)
        assertEquals("0x4TestKey", cfg.siteKey)
        assertEquals("/api/auth/turnstile", server.takeRequest().path)

        server.enqueue(
            MockResponse().setBody(
                """{"challenge_id":"c-login","options":{"challenge":"AA"}}""",
            ),
        )
        val loginStart = container.auth.passkeyLoginStart("member@example.org", "cf-tok")
        assertEquals("c-login", loginStart.challenge_id)
        val loginReq = server.takeRequest()
        assertEquals("/api/auth/passkey/login/start", loginReq.path)
        assertTrue(loginReq.body.readUtf8().contains(""""turnstile_token":"cf-tok""""))

        server.enqueue(
            MockResponse().setBody(
                """{"challenge_id":"c-reauth","options":{"challenge":"BB"}}""",
            ),
        )
        val reauthStart = container.auth.passkeyReauthStart()
        assertEquals("c-reauth", reauthStart.challenge_id)
        assertEquals("/api/auth/passkey/reauth/start", server.takeRequest().path)

        server.enqueue(MockResponse().setBody("""{"ok":true}"""))
        server.enqueue(
            MockResponse().setBody(
                """{"id":1,"name":"Member","email":"member@example.org","status":"active"}""",
            ),
        )
        container.auth.passkeyReauthFinish("c-reauth", """{"id":"cred"}""")
        val finishReq = server.takeRequest()
        assertEquals("/api/auth/passkey/reauth/finish", finishReq.path)
        assertFalse(finishReq.body.readUtf8().isBlank())
        assertEquals("/api/auth/me", server.takeRequest().path)
        assertEquals(1L, container.auth.me?.id)
        assertNull(container.auth.me?.photoFileId)
    }
}
