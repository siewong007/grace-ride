package org.pjstatecm.graceride

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonPrimitive
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.pjstatecm.graceride.data.api.TermsReq
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.data.session.PersistentCookieJar
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.PasskeyOptions
import org.pjstatecm.graceride.domain.Readiness

/** Regressions for the bug sweep: backend-contract mismatches found by code audit + live E2E. */
@OptIn(ExperimentalCoroutinesApi::class)
class BugfixRegressionTest {

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

    @Test fun readinessValuesMatchBackendEnum() {
        // backend/src/routes/rides/readiness.rs: valid_statuses
        assertEquals(listOf("ready", "delayed_5m", "delayed_10m", "unable_to_join"), Readiness.values)
    }

    @Test fun passkeyOptionsUnwrapsPublicKey() {
        val wrapped = apiJson.parseToJsonElement(
            """{"publicKey":{"challenge":"abc","rpId":"graceride.example.org","allowCredentials":[{"type":"public-key","id":"xyz"}]}}"""
        ) as JsonObject
        val pk = PasskeyOptions.publicKey(wrapped)
        assertEquals("abc", pk["challenge"]?.jsonPrimitive?.content)
        assertEquals("graceride.example.org", pk["rpId"]?.jsonPrimitive?.content)
        assertTrue(pk.containsKey("allowCredentials"))
        assertFalse(pk.containsKey("publicKey"))
    }

    @Test fun passkeyOptionsFallsBackToFlatShape() {
        val flat = apiJson.parseToJsonElement("""{"challenge":"abc","rpId":"x"}""") as JsonObject
        assertEquals("abc", PasskeyOptions.publicKey(flat)["challenge"]?.jsonPrimitive?.content)
    }

    @Test fun cookiePathMatchIsRfc6265() {
        assertTrue(PersistentCookieJar.pathMatch("/api/auth/me", "/"))
        assertTrue(PersistentCookieJar.pathMatch("/api", "/api"))
        assertTrue(PersistentCookieJar.pathMatch("/api/x", "/api"))
        assertFalse(PersistentCookieJar.pathMatch("/apix", "/api"))
        assertFalse(PersistentCookieJar.pathMatch("/other", "/api"))
        assertTrue(PersistentCookieJar.pathMatch("/api/x", "/api/"))
    }

    @Test fun termsRequestSendsAcceptedTrue() {
        val body = apiJson.encodeToString(TermsReq.serializer(), TermsReq(accepted = true))
        assertEquals("""{"accepted":true}""", body)
    }

    @Test fun termsGateStateAndAcceptFlow() = runTest {
        // me with terms_accepted=false stays SignedIn (the nav layer gates on the flag)
        server.enqueue(
            MockResponse().setBody(
                """{"id":1,"name":"New Member","email":"new@example.org","status":"active","terms_accepted":false}"""
            )
        )
        container.auth.refresh()
        val state = container.auth.state.value
        assertTrue(state is AuthState.SignedIn)
        assertFalse((state as AuthState.SignedIn).me.termsAccepted)
        server.takeRequest() // drain the first /auth/me

        // accept sends the required body and refreshes
        server.enqueue(MockResponse().setBody("""{"ok":true}"""))
        server.enqueue(
            MockResponse().setBody(
                """{"id":1,"name":"New Member","email":"new@example.org","status":"active","terms_accepted":true}"""
            )
        )
        container.auth.acceptTerms()
        val req = server.takeRequest()
        assertEquals("/api/auth/terms", req.path)
        assertTrue(req.body.readUtf8().contains("\"accepted\":true"))
        assertTrue((container.auth.state.value as AuthState.SignedIn).me.termsAccepted)
    }

    @Test fun serverOverrideRebuildsRepositories() = runTest {
        // The auth repo captured at construction must not keep the old base URL.
        val old = container.auth
        val server2 = MockWebServer()
        server2.start()
        try {
            container.setBaseUrl(server2.url("/").toString())
            assertTrue(container.auth !== old)
            server2.enqueue(MockResponse().setBody(Fixtures.ME_MEMBER))
            container.auth.refresh()
            assertTrue(container.auth.state.value is AuthState.SignedIn)
            assertEquals("/api/auth/me", server2.takeRequest().path)
            assertEquals(0, server.requestCount)
        } finally {
            server2.shutdown()
        }
    }
}
