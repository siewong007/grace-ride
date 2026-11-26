package org.pjstatecm.graceride

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import kotlinx.serialization.builtins.serializer
import okhttp3.mockwebserver.Dispatcher
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import okhttp3.mockwebserver.RecordedRequest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.pjstatecm.graceride.data.api.Area
import org.pjstatecm.graceride.data.api.PasskeyStartReq
import org.pjstatecm.graceride.data.api.ProxyRider
import org.pjstatecm.graceride.data.api.ProxyRiderReq
import org.pjstatecm.graceride.data.api.SavedPlaceReq
import org.pjstatecm.graceride.data.api.TurnstileConfigDto
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.domain.inferAreaId
import org.pjstatecm.graceride.platform.CheckinParseResult
import org.pjstatecm.graceride.platform.parseCheckinInput
import org.pjstatecm.graceride.ui.NotifDestination
import org.pjstatecm.graceride.ui.NotifLinkRouter
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.screens.drive.DriveHubViewModel
import org.pjstatecm.graceride.ui.screens.events.EventDetailViewModel
import org.pjstatecm.graceride.ui.screens.rides.RideDetailViewModel
import org.pjstatecm.graceride.ui.screens.rides.RideNewViewModel
import org.pjstatecm.graceride.ui.screens.rides.TripType
import java.time.LocalDate
import kotlin.time.Duration.Companion.seconds
import app.cash.turbine.test

@OptIn(ExperimentalCoroutinesApi::class)
class BugfixRegressionTest {

    private lateinit var server: MockWebServer
    private lateinit var container: AppContainer

    @Before
    fun setup() {
        Dispatchers.setMain(UnconfinedTestDispatcher())
        server = MockWebServer()
        server.start()
        container = AppContainer(null, InMemoryStore())
        container.setBaseUrl(server.url("/").toString())
    }

    @After
    fun teardown() {
        server.shutdown()
        Dispatchers.resetMain()
    }

    private suspend fun <T> StateFlow<T>.awaitUntil(predicate: (T) -> Boolean): T =
        kotlinx.coroutines.withTimeout(5.seconds) {
            var out: T? = null
            test {
                while (true) {
                    val item = awaitItem()
                    if (predicate(item)) {
                        out = item
                        break
                    }
                }
                cancelAndIgnoreRemainingEvents()
            }
            @Suppress("UNCHECKED_CAST")
            out as T
        }

    private fun dispatchByPath(routes: Map<String, () -> MockResponse>) {
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val path = request.path ?: ""
                val matched = routes.entries.firstOrNull { (prefix, _) -> path.startsWith(prefix) }
                return matched?.value?.invoke() ?: MockResponse().setBody("[]")
            }
        }
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

    @Test
    fun formatKlTimeOffsetsMalformedRejectionAndMoneyMinLong() {
        assertEquals("-RM 92,233,720,368,547,758.08", Money.rm(Long.MIN_VALUE))

        val compactUtc = KlTime.parse("2026-11-22T01:30:00+0000")
        assertNotNull(compactUtc)
        assertEquals(9, compactUtc?.hour)
        assertEquals(30, compactUtc?.minute)

        val lowerZ = KlTime.parse("2026-11-22 01:30:00z")
        assertNotNull(lowerZ)
        assertEquals(9, lowerZ?.hour)

        assertNull(KlTime.parse("2026-11-21 bad"))
        assertNull(KlTime.parse("not-a-timestamp"))
        assertEquals("2026-11-22 09:30:00", KlTime.sortKey(" 2026-11-22t09:30:00 "))
    }

    @Test
    fun areaMatchAndRoutingAndQrEdgeCases() {
        val areas = listOf(Area(id = 7, name = "U1/2"))
        assertEquals(7L, inferAreaId("No 5 Jalan U1/2, Shah Alam", areas))
        assertNull(inferAreaId("Section U1/20", areas))
        assertNull(inferAreaId("BU1/2", areas))

        assertEquals(NotifDestination.Native(Routes.EVENTS), NotifLinkRouter.resolve("/events/"))
        assertEquals(NotifDestination.Native(Routes.rideDetail(42)), NotifLinkRouter.resolve("/rides/42/"))
        assertEquals(NotifDestination.Native(Routes.studio()), NotifLinkRouter.resolve("/studio"))
        val ridesMember = org.pjstatecm.graceride.data.api.Me(
            id = 1L,
            name = "M",
            email = "m@x.org",
            permissions = listOf("rides.view"),
        )
        assertEquals(NotifDestination.Native(Routes.FAMILY), NotifLinkRouter.resolveFor("/me/family/", ridesMember))

        val codeless = parseCheckinInput("https://graceride.example/events/91/checkin", 91L)
        assertEquals(CheckinParseResult.Code(""), codeless)
        val plusCode = parseCheckinInput("https://graceride.example/events/91/checkin?CODE=A%2BB", 91L)
        assertEquals(CheckinParseResult.Code("A+B"), plusCode)
        assertEquals(CheckinParseResult.Invalid, parseCheckinInput("GRPASS:91:XYZ", 91L))
    }

    @Test
    fun auth403KeepsSession401ClearsAndUserSwitchWipesCache() = runBlocking {
        server.enqueue(MockResponse().setBody("""{"id":1,"name":"User 1","email":"u1@x.org","status":"active","mfa_ok":true}"""))
        container.auth.refresh()
        assertTrue(container.auth.state.value is AuthState.SignedIn)

        container.offlineCache.write(1L, "k", "cached", String.serializer())
        assertEquals("cached", container.offlineCache.read(1L, "k", String.serializer())?.data)

        // 403 does not wipe the signed-in session.
        server.enqueue(MockResponse().setResponseCode(403).setBody("""{"error":"forbidden"}"""))
        container.auth.refresh()
        assertTrue(container.auth.state.value is AuthState.SignedIn)
        assertEquals("cached", container.offlineCache.read(1L, "k", String.serializer())?.data)

        // Switching user clears previous user's OfflineCache entries.
        server.enqueue(MockResponse().setBody("""{"id":2,"name":"User 2","email":"u2@x.org","status":"active","mfa_ok":true}"""))
        container.auth.refresh()
        assertEquals(2L, container.auth.me?.id)
        assertNull(container.offlineCache.read(1L, "k", String.serializer()))

        // 401 signs out.
        server.enqueue(MockResponse().setResponseCode(401).setBody("""{"error":"unauthorized"}"""))
        container.auth.refresh()
        assertTrue(container.auth.state.value is AuthState.SignedOut)
    }

    @Test
    fun rideDetailNoShowClaimAndSupperChoiceEndpoints() = runBlocking {
        server.enqueue(
            MockResponse().setBody(
                """{"id":2,"name":"Driver","email":"d@x.org","status":"active","permissions":["rides.view","rides.update","drive.create","drive.update"],"mfa_ok":true}""",
            ),
        )
        container.auth.refresh()

        val called = mutableListOf<String>()
        dispatchByPath(
            mapOf(
                "/api/rides/15/no-show" to {
                    called += "no-show"
                    MockResponse().setBody("""{"ok":true}""")
                },
                "/api/rides/15/claim" to {
                    called += "claim"
                    MockResponse().setBody("""{"ok":true}""")
                },
                "/api/rides/15/supper-choice" to {
                    called += "supper-choice"
                    MockResponse().setBody("""{"ok":true}""")
                },
                "/api/rides/15" to {
                    MockResponse().setBody(
                        """{"id":15,"status":"in_progress","leg":"return","supper_choice":"unspecified","pickup_at":"2026-11-22 12:00:00","area":{"id":1,"name":"PJ"},"destination":"Hall","passenger":{"id":1,"name":"Pax"},"viewer":{"is_passenger":true,"is_driver":true}}""",
                    )
                },
            ),
        )

        val vm = RideDetailViewModel(container, 15L)
        vm.state.awaitUntil { !it.loading && it.ride != null }

        vm.claim()
        vm.state.awaitUntil { !it.busy && "claim" in called }
        vm.supperChoice("supper")
        vm.state.awaitUntil { !it.busy && "supper-choice" in called }
        vm.noShow()
        vm.state.awaitUntil { !it.busy && "no-show" in called }
        assertEquals(listOf("claim", "supper-choice", "no-show"), called)
    }

    @Test
    fun driveHubKeepsRouteAndSurfacesClaimErrorInActionError() = runBlocking {
        server.enqueue(
            MockResponse().setBody(
                """{"id":2,"name":"Driver","email":"d@x.org","status":"active","permissions":["drive.view","drive.create"],"mfa_ok":true}""",
            ),
        )
        container.auth.refresh()

        var failRoute = false
        dispatchByPath(
            mapOf(
                "/api/driver/requests" to {
                    MockResponse().setBody(
                        """[{"id":55,"status":"pending_driver","pickup_at":"2026-11-22 09:00:00","area":{"id":1,"name":"PJ"},"destination":"Hall","passenger":{"id":1,"name":"Pax"}}]""",
                    )
                },
                "/api/driver/schedule" to { MockResponse().setBody("[]") },
                "/api/driver/route" to {
                    if (failRoute) {
                        MockResponse().setResponseCode(500).setBody("""{"error":"route down"}""")
                    } else {
                        MockResponse().setBody("""{"route":{"trip_id":99,"live":true,"status":"in_progress","stops":[]}}""")
                    }
                },
                "/api/rides/55/claim" to {
                    MockResponse().setResponseCode(409).setBody("""{"error":"Already claimed"}""")
                },
            ),
        )

        val vm = DriveHubViewModel(container)
        val initial = vm.state.awaitUntil { !it.loading && it.route != null }
        assertEquals(99L, initial.route?.tripId)

        failRoute = true
        vm.refresh()
        val refreshed = vm.state.awaitUntil { !it.refreshing && it.requests.isNotEmpty() }
        assertEquals(99L, refreshed.route?.tripId)
        assertNull(refreshed.error)

        vm.claim(55L)
        val failedClaim = vm.state.awaitUntil { it.actionError != null && it.busyRideId == null }
        assertEquals("Already claimed", failedClaim.actionError)
        assertNull(failedClaim.error)
    }

    @Test
    fun rideNewPreservesEditedOutboundTimeAcrossRoundToggleAndProxySwitch() = runBlocking {
        server.enqueue(MockResponse().setBody("""{"id":1,"name":"M","email":"m@x.org","status":"active","permissions":["rides.create"],"mfa_ok":true}"""))
        container.auth.refresh()

        dispatchByPath(
            mapOf(
                "/api/areas" to { MockResponse().setBody("""[{"id":1,"name":"PJ","active":true}]""") },
                "/api/saved-places" to {
                    MockResponse().setBody("""[{"id":4,"label":"Home","area_id":1,"address":"12 Jalan Home","gate_notes":"Blue gate","is_default":1}]""")
                },
                "/api/proxy-riders" to { MockResponse().setBody("[]") },
                "/api/events/9" to {
                    MockResponse().setBody("""{"id":9,"title":"Sunday","starts_at":"2026-12-06 09:00:00","location":"Church Hall","my_signup":{"status":"going"}}""")
                },
            ),
        )

        val vm = RideNewViewModel(container, eventId = 9L)
        vm.state.awaitUntil { !it.loading && it.event != null }

        val customDate = LocalDate.of(2026, 12, 7)
        vm.update { it.copy(date = customDate, hour = 7, minute = 45) }
        vm.setTripType(TripType.ROUND)
        assertEquals(customDate, vm.state.value.date)
        assertEquals(7, vm.state.value.hour)
        assertEquals(45, vm.state.value.minute)

        vm.setProxyMode(true)
        assertTrue(vm.state.value.isProxy)
        assertEquals("", vm.state.value.address)
        assertFalse(vm.state.value.notes.contains("Blue gate"))

        vm.setProxyMode(false)
        assertFalse(vm.state.value.isProxy)
        assertEquals("12 Jalan Home", vm.state.value.address)
        assertTrue(vm.state.value.notes.contains("Blue gate"))
    }

    @Test
    fun eventDetailSetTransportAndSelfTransportSignup() = runBlocking {
        server.enqueue(MockResponse().setBody("""{"id":1,"name":"M","email":"m@x.org","status":"active","permissions":["events.view"],"mfa_ok":true}"""))
        container.auth.refresh()

        val bodies = mutableListOf<String>()
        val eventJson = """{"id":80,"title":"Outing","starts_at":"2026-12-20 09:00:00","status":"published","my_signup":{"status":"going","self_transport":false}}"""
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse = when (request.path) {
                "/api/events/80/signup" -> {
                    bodies += "signup:" + request.body.readUtf8()
                    MockResponse().setBody("""{"ok":true}""")
                }
                "/api/events/80/transport" -> {
                    bodies += "transport:" + request.body.readUtf8()
                    MockResponse().setBody("""{"ok":true}""")
                }
                else -> MockResponse().setBody(eventJson)
            }
        }

        val vm = EventDetailViewModel(container, 80L)
        vm.state.awaitUntil { !it.loading && it.event != null }

        vm.signup(needRide = false, offerDrive = false, asGroupId = null, selfTransport = true)
        vm.state.awaitUntil { !it.busy && bodies.any { it.startsWith("signup:") } }
        assertTrue(bodies.first { it.startsWith("signup:") }.contains(""""self_transport":true"""))

        vm.setTransport(true)
        vm.state.awaitUntil { !it.busy && bodies.any { it.startsWith("transport:") } }
        assertTrue(bodies.first { it.startsWith("transport:") }.contains(""""self_transport":true"""))
    }
}
