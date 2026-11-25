package org.pjstatecm.graceride

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonPrimitive
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
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.SavedPlace
import org.pjstatecm.graceride.data.api.TermsReq
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.data.session.PersistentCookieJar
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.Assistance
import org.pjstatecm.graceride.domain.DatePickerUtc
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.PasskeyOptions
import org.pjstatecm.graceride.domain.Readiness
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.screens.events.EventsViewModel
import org.pjstatecm.graceride.ui.screens.home.HomeViewModel
import org.pjstatecm.graceride.ui.screens.profile.SavedAddressesViewModel
import org.pjstatecm.graceride.ui.screens.rides.RideDetailViewModel
import java.time.LocalDate
import java.util.TimeZone

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
        // The auth repo captured at construction must not keep the old base URL,
        // and existing collectors on old.state must still receive updates via sharedAuthState.
        val old = container.auth
        val server2 = MockWebServer()
        server2.start()
        try {
            container.setBaseUrl(server2.url("/").toString())
            assertTrue(container.auth !== old)
            server2.enqueue(MockResponse().setBody(Fixtures.ME_MEMBER))
            container.auth.refresh()
            assertTrue(container.auth.state.value is AuthState.SignedIn)
            assertTrue(old.state.value is AuthState.SignedIn)
            assertEquals("/api/auth/me", server2.takeRequest().path)
            assertEquals(0, server.requestCount)
        } finally {
            server2.shutdown()
        }
    }

    @Test fun invalidServerUrlIsRejectedWithoutMutatingContainer() {
        val before = container.baseUrl
        for (bad in listOf("", "   ", "192.168.1.5:8080", "ftp://example.org/")) {
            var threw = false
            try {
                container.setBaseUrl(bad)
            } catch (_: IllegalArgumentException) {
                threw = true
            }
            assertTrue("Expected IllegalArgumentException for '$bad'", threw)
            assertEquals(before, container.baseUrl)
        }
    }

    @Test fun mySignupAttendedDecodesIntegerAndBooleanFormats() {
        val ev0 = apiJson.decodeFromString<EventItem>(
            """{"id":1,"title":"A","starts_at":"2026-11-20 09:00:00","my_signup":{"status":"going","attended":0}}"""
        )
        assertEquals(false, ev0.mySignup?.attended)

        val ev1 = apiJson.decodeFromString<EventItem>(
            """{"id":2,"title":"B","starts_at":"2026-11-20 09:00:00","my_signup":{"status":"going","attended":1}}"""
        )
        assertEquals(true, ev1.mySignup?.attended)

        val evNull = apiJson.decodeFromString<EventItem>(
            """{"id":3,"title":"C","starts_at":"2026-11-20 09:00:00","my_signup":{"status":"going","attended":null,"as_group":{"id":5,"name":null}},"area":{"id":2,"name":null}}"""
        )
        assertNull(evNull.mySignup?.attended)
        assertEquals("", evNull.mySignup?.asGroup?.name)
        assertEquals("", evNull.area?.name)

        val evBool = apiJson.decodeFromString<EventItem>(
            """{"id":4,"title":"D","starts_at":"2026-11-20 09:00:00","my_signup":{"status":"going","attended":true}}"""
        )
        assertEquals(true, evBool.mySignup?.attended)
    }

    @Test fun passkeyLoginStartSendsEmailJsonBody() = runBlocking {
        server.enqueue(MockResponse().setBody("""{"challenge_id":"c1","options":{"publicKey":{"challenge":"x"}}}"""))
        container.auth.passkeyLoginStart("  member@example.org ")
        val req = server.takeRequest()
        assertEquals("/api/auth/passkey/login/start", req.path)
        assertEquals("""{"email":"member@example.org"}""", req.body.readUtf8())
    }

    @Test fun klTimeAndDatePickerUtcAreTimezoneIndependent() {
        val prev = TimeZone.getDefault()
        try {
            TimeZone.setDefault(TimeZone.getTimeZone("America/Los_Angeles"))
            assertEquals("Asia/Kuala_Lumpur", KlTime.zone.id)
            val utcMidnight = java.time.Instant.parse("2026-11-25T00:00:00Z").toEpochMilli()
            assertEquals("08:00", KlTime.formatEpochMillis(utcMidnight))
            val picked = LocalDate.of(2026, 12, 25)
            val millis = DatePickerUtc.toPickerMillis(picked)
            assertEquals(picked, DatePickerUtc.fromPickerMillis(millis))
        } finally {
            TimeZone.setDefault(prev)
        }
    }

    @Test fun homeIncludesNeedsInfoSharesAndNotError() = runBlocking {
        server.enqueue(MockResponse().setBody(Fixtures.ME_MEMBER))
        container.auth.refresh()
        server.takeRequest()

        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val path = request.path ?: ""
                return when {
                    path.startsWith("/api/me/shares") -> MockResponse().setBody(
                        """[{"id":9,"settlement_id":2,"title":"Fuel share","amount_cents":1200,"status":"needs_info","note":"Upload clearer slip"}]"""
                    )
                    path.startsWith("/api/events") -> MockResponse().setResponseCode(500).setBody("""{"error":"err"}""")
                    else -> MockResponse().setBody("[]")
                }
            }
        }

        val vm = HomeViewModel(container)
        var st = vm.state.value
        val deadline = System.currentTimeMillis() + 5_000
        while (st.loading && System.currentTimeMillis() < deadline) {
            kotlinx.coroutines.delay(20)
            st = vm.state.value
        }
        assertFalse(st.loading)
        assertEquals(1, st.unpaidShares.size)
        assertEquals("needs_info", st.unpaidShares[0].status)
        assertEquals("Upload clearer slip", st.unpaidShares[0].note)
        assertNull(st.error)
    }

    @Test fun savedAddressesSetDefaultPreservesCoordinates() = runBlocking {
        server.enqueue(MockResponse().setBody("[]"))
        server.enqueue(MockResponse().setBody("[]"))
        val vm = SavedAddressesViewModel(container)
        val deadline = System.currentTimeMillis() + 5_000
        while (vm.state.value.loading && System.currentTimeMillis() < deadline) {
            kotlinx.coroutines.delay(20)
        }
        repeat(2) { server.takeRequest() }

        val place = SavedPlace(
            id = 7,
            label = "Home",
            areaId = 2,
            address = "12 Jalan Pin",
            gateNotes = "Red gate",
            isDefault = 0,
            lat = 3.112,
            lng = 101.627,
        )
        server.enqueue(MockResponse().setBody("""{"ok":true}"""))
        server.enqueue(MockResponse().setBody("[]"))
        vm.setDefault(place)
        val req = server.takeRequest()
        assertEquals("PUT", req.method)
        assertEquals("/api/saved-places/7", req.path)
        val body = req.body.readUtf8()
        assertTrue(body.contains("\"lat\":3.112"))
        assertTrue(body.contains("\"lng\":101.627"))
    }

    @Test fun routesStudioEncodesHexHashCharacter() {
        val route = Routes.studio(shape = "hexagon", color = "#38BDF8", face = "happy")
        assertFalse("Hex # must be percent-encoded", route.contains("#"))
        assertTrue(route.contains("%2338BDF8"))
    }

    @Test fun rideDetailActionAwaitsReloadBeforeClearingBusy() = runBlocking {
        val initialRide = """{"id":55,"status":"confirmed","pickup_at":"2026-11-29 09:00:00","area":{"id":2,"name":"PJ"},"destination":"Church","passenger":{"id":1,"name":"Member"},"viewer":{"is_passenger":true}}"""
        val reloadedRide = """{"id":55,"status":"confirmed","pickup_at":"2026-11-29 09:00:00","area":{"id":2,"name":"PJ"},"destination":"Church","readiness_status":"ready","passenger":{"id":1,"name":"Member"},"viewer":{"is_passenger":true}}"""
        server.enqueue(MockResponse().setBody(initialRide))
        val vm = RideDetailViewModel(container, 55L)
        val dl = System.currentTimeMillis() + 5_000
        while (vm.state.value.loading && System.currentTimeMillis() < dl) kotlinx.coroutines.delay(10)

        server.enqueue(MockResponse().setBody("""{"ok":true}"""))
        server.enqueue(MockResponse().setBody(reloadedRide).setBodyDelay(150, java.util.concurrent.TimeUnit.MILLISECONDS))
        vm.readiness("ready")
        val dl2 = System.currentTimeMillis() + 5_000
        while (vm.state.value.busy && System.currentTimeMillis() < dl2) kotlinx.coroutines.delay(10)
        assertEquals("ready", vm.state.value.ride?.readinessStatus)
    }

    @Test fun eventsViewModelCancelsStaleTabRequest() = runBlocking {
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val path = request.path ?: ""
                return when {
                    path.contains("scope=upcoming") -> MockResponse()
                        .setBody("""[{"id":1,"title":"Stale Upcoming","starts_at":"2026-12-01 09:00:00"}]""")
                        .setBodyDelay(300, java.util.concurrent.TimeUnit.MILLISECONDS)
                    path.contains("scope=past") -> MockResponse()
                        .setBody("""[{"id":2,"title":"Fresh Past","starts_at":"2026-10-01 09:00:00"}]""")
                    else -> MockResponse().setBody("[]")
                }
            }
        }
        val vm = EventsViewModel(container)
        kotlinx.coroutines.delay(20)
        vm.setTab(2)
        val dl = System.currentTimeMillis() + 5_000
        while ((vm.state.value.loading || vm.state.value.events.isEmpty()) && System.currentTimeMillis() < dl) {
            kotlinx.coroutines.delay(15)
        }
        assertEquals(2, vm.state.value.tab)
        assertEquals(listOf(2L), vm.state.value.events.map { it.id })
    }

    @Test fun assistanceLabelResMapsCanonicalKeysAndAllowsFreeText() {
        assertEquals(R.string.ride_new_assist_none, Assistance.labelRes(""))
        assertEquals(R.string.ride_new_assist_wheelchair, Assistance.labelRes("wheelchair"))
        assertEquals(R.string.ride_new_assist_stroller, Assistance.labelRes("Stroller"))
        assertEquals(R.string.ride_new_assist_oxygen, Assistance.labelRes("oxygen"))
        assertEquals(R.string.ride_new_assist_other, Assistance.labelRes("other"))
        assertNull(Assistance.labelRes("walking cane"))
    }
}
