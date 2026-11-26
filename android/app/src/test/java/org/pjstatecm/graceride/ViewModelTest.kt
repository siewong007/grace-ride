package org.pjstatecm.graceride

import app.cash.turbine.test
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.setMain
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.double
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
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
import org.pjstatecm.graceride.data.api.Area
import org.pjstatecm.graceride.data.api.DriverVehicleReq
import org.pjstatecm.graceride.data.api.ProxyRiderReq
import org.pjstatecm.graceride.data.api.SavedPlace
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.inferAreaId
import org.pjstatecm.graceride.ui.screens.costs.CostsViewModel
import org.pjstatecm.graceride.ui.screens.events.EventActionNotice
import org.pjstatecm.graceride.ui.screens.events.EventDetailViewModel
import org.pjstatecm.graceride.ui.screens.events.EventsViewModel
import org.pjstatecm.graceride.ui.screens.home.HomeViewModel
import org.pjstatecm.graceride.ui.screens.login.LoginViewModel
import org.pjstatecm.graceride.ui.screens.profile.DriverSettingsViewModel
import org.pjstatecm.graceride.ui.screens.profile.ProfileViewModel
import org.pjstatecm.graceride.ui.screens.profile.ProxyRidersViewModel
import org.pjstatecm.graceride.ui.screens.profile.SavedAddressesViewModel
import org.pjstatecm.graceride.ui.screens.rides.RideDetailViewModel
import org.pjstatecm.graceride.ui.screens.rides.RideNewViewModel
import org.pjstatecm.graceride.ui.screens.rides.RidesViewModel
import org.pjstatecm.graceride.ui.screens.rides.TripType
import kotlin.time.Duration.Companion.seconds

@OptIn(ExperimentalCoroutinesApi::class)
class ViewModelTest {

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

    private suspend fun <T> StateFlow<T>.awaitUntil(predicate: (T) -> Boolean): T =
        kotlinx.coroutines.withTimeout(10.seconds) {
            var result: T? = null
            test {
                while (true) {
                    val item = awaitItem()
                    if (predicate(item)) { result = item; break }
                }
                cancelAndIgnoreRemainingEvents()
            }
            @Suppress("UNCHECKED_CAST")
            result as T
        }

    private suspend fun signIn() {
        server.enqueue(json(Fixtures.ME_MEMBER))
        container.auth.refresh()
    }

    private fun dispatchByPath(routes: Map<String, () -> MockResponse>) {
        server.dispatcher = object : okhttp3.mockwebserver.Dispatcher() {
            override fun dispatch(request: okhttp3.mockwebserver.RecordedRequest): MockResponse {
                val rawPath = request.path ?: ""
                val matched = routes.entries.firstOrNull { (prefix, _) -> rawPath.startsWith(prefix) }
                return matched?.value?.invoke() ?: MockResponse().setBody("[]")
            }
        }
    }

    @Test fun homeLoadsAllSections() = runBlocking {
        signIn()
        dispatchByPath(
            mapOf(
                "/api/rides" to { json("[${Fixtures.RIDE}]") },
                "/api/events" to { json("[${Fixtures.EVENT}]") },
                "/api/me/shares" to { json("[${Fixtures.SHARE}]") },
                "/api/announcements" to { json("[${Fixtures.ANN}]") },
                "/api/driver/schedule" to { json("[]") },
            )
        )

        val vm = HomeViewModel(container)
        val state = vm.state.awaitUntil { !it.loading }
        assertEquals("Test Member", state.me?.name)
        assertEquals(10L, state.nextRide?.id)
        assertEquals(1, state.events.size)
        assertEquals(1, state.unpaidShares.size)
        assertEquals(1, state.announcements.size)
        assertNull(state.error)
    }

    @Test fun homeSurvivesSectionFailure() = runBlocking {
        signIn()
        dispatchByPath(
            mapOf(
                "/api/rides" to { MockResponse().setResponseCode(500).setBody("""{"error":"boom"}""") },
                "/api/events" to { json("[${Fixtures.EVENT}]") },
                "/api/me/shares" to { json("[]") },
                "/api/announcements" to { json("[${Fixtures.ANN}]") },
                "/api/driver/schedule" to { json("[]") },
            )
        )

        val vm = HomeViewModel(container)
        val state = vm.state.awaitUntil { !it.loading }
        assertNull(state.nextRide)
        assertEquals(1, state.events.size)
        assertEquals(1, state.announcements.size)
    }

    @Test fun homeOfflineFlag() = runBlocking {
        signIn()
        server.shutdown() // every section now fails with IOException
        val vm = HomeViewModel(container)
        val state = vm.state.awaitUntil { !it.loading }
        assertTrue(state.offline)
    }

    @Test fun ridesTabsSwitchScopes() = runBlocking {
        server.enqueue(json("[${Fixtures.RIDE}]")) // upcoming
        val vm = RidesViewModel(container)
        val first = vm.state.awaitUntil { !it.loading }
        assertEquals(1, first.rides.size)
        assertEquals("/api/rides?scope=upcoming", server.takeRequest().path)

        server.enqueue(json("[]")) // past
        vm.setTab(1)
        val second = vm.state.awaitUntil { !it.loading && it.tab == 1 }
        assertEquals(0, second.rides.size)
        assertEquals("/api/rides?scope=past", server.takeRequest().path)
    }

    @Test fun loginDevAccountsAndSignIn() = runBlocking {
        server.enqueue(json("""[{"name":"Demo Member","email":"member@example.org","roles":"member"}]"""))
        val vm = LoginViewModel(container, devAvailable = true)
        val withUsers = vm.state.awaitUntil { it.devUsers.isNotEmpty() }
        assertEquals("member@example.org", withUsers.devUsers[0].email)

        server.enqueue(json("""{"ok":true}"""))
        server.enqueue(json(Fixtures.ME_MEMBER))
        vm.devLogin("member@example.org")
        vm.state.awaitUntil { !it.signingIn }
        assertTrue(container.auth.state.value is org.pjstatecm.graceride.data.repo.AuthState.SignedIn)
    }

    @Test fun loginFailureSurfacesError() = runBlocking {
        server.enqueue(json("[]")) // dev users
        val vm = LoginViewModel(container, devAvailable = true)
        // Wait until the dev-users request actually landed before queueing the
        // failure response, so the dev-login POST cannot consume the wrong one.
        server.takeRequest(5, java.util.concurrent.TimeUnit.SECONDS)
        server.enqueue(MockResponse().setResponseCode(400).setBody("""{"error":"No such demo account","code":"INVALID_CREDENTIALS"}"""))
        vm.devLogin("nobody@example.org")
        val state = vm.state.awaitUntil { !it.signingIn && it.error != null }
        assertEquals("No such demo account", state.error)
    }

    @Test fun homeSplitsTodayAndUpcomingDrives() = runBlocking {
        signIn()
        server.takeRequest() // /api/auth/me
        val today = KlTime.today()
        val later = KlTime.todayPlusDays(2)
        val nextWeek = KlTime.todayPlusDays(7)
        val driveToday = """{"id":101,"status":"confirmed","pickup_at":"$today 09:00:00",
            "area":{"id":2,"name":"PJ Section 14"},"destination":"Church Hall","seats":1,
            "passenger":{"id":2,"name":"Passenger Today"},"viewer":{"is_passenger":false,"is_driver":true,"is_staff":false}}"""
        val driveLater = """{"id":102,"status":"confirmed","pickup_at":"$later 14:30:00",
            "area":{"id":2,"name":"PJ Section 14"},"destination":"Church Hall","seats":1,
            "passenger":{"id":3,"name":"Passenger Later"},"viewer":{"is_passenger":false,"is_driver":true,"is_staff":false}}"""

        dispatchByPath(
            mapOf(
                "/api/driver/schedule" to { json("[$driveToday,$driveLater]") },
            )
        )

        val vm = HomeViewModel(container)
        val state = vm.state.awaitUntil { !it.loading }
        assertEquals(listOf(101L), state.drivingToday.map { it.id })
        assertEquals(listOf(102L), state.upcomingDrives.map { it.id })
        assertEquals(102L, state.nextUpcomingDrive?.id)

        val paths = List(5) { server.takeRequest().path }
        assertTrue(paths.contains("/api/driver/schedule?from=$today&to=$nextWeek"))
    }

    @Test fun homeOfflineSnapshotFallback() = runBlocking {
        signIn()
        server.takeRequest() // /api/auth/me
        val today = KlTime.today()
        val tomorrow = KlTime.todayPlusDays(1)
        val passengerRide = """{"id":10,"status":"confirmed","pickup_at":"$today 08:30:00",
            "area":{"id":2,"name":"PJ Section 14"},"destination":"Church Hall","seats":1,
            "passenger":{"id":1,"name":"Test Member"},"viewer":{"is_passenger":true,"is_driver":false,"is_staff":false}}"""
        val driverRide = """{"id":20,"status":"confirmed","pickup_at":"$tomorrow 10:00:00",
            "area":{"id":2,"name":"PJ Section 14"},"destination":"Church Hall","seats":1,
            "passenger":{"id":2,"name":"Rider Two"},"viewer":{"is_passenger":false,"is_driver":true,"is_staff":false}}"""

        dispatchByPath(
            mapOf(
                "/api/rides" to { json("[$passengerRide]") },
                "/api/driver/schedule" to { json("[$driverRide]") },
            )
        )

        val vm = HomeViewModel(container)
        val online = vm.state.awaitUntil { !it.loading }
        assertEquals(10L, online.nextRide?.id)
        assertEquals(1, online.upcomingDrives.size)
        assertNull(online.offlineSnapshotAt)

        server.shutdown()
        vm.refresh()
        val offlineState = vm.state.awaitUntil { !it.refreshing && it.offline }
        assertEquals(10L, offlineState.nextRide?.id)
        assertEquals(20L, offlineState.nextUpcomingDrive?.id)
        assertNotNull(offlineState.offlineSnapshotAt)
    }

    @Test fun eventsLoadsOpenRequestsCountForDrivers() = runBlocking {
        signIn()
        server.takeRequest() // /api/auth/me
        val ev1 = """{"id":20,"title":"Sunday Service","starts_at":"2026-11-29 09:00:00"}"""
        val ev2 = """{"id":21,"title":"Prayer Meeting","starts_at":"2026-12-02 20:00:00"}"""
        val r1 = """{"id":1,"status":"pending_driver","pickup_at":"2026-11-29 08:20:00",
            "area":{"id":2,"name":"PJ"},"destination":"Church","event":{"id":20,"title":"Sunday Service"},
            "passenger":{"id":2,"name":"A"}}"""
        val r2 = """{"id":2,"status":"pending_driver","pickup_at":"2026-11-29 08:25:00",
            "area":{"id":2,"name":"PJ"},"destination":"Church","event":{"id":20,"title":"Sunday Service"},
            "passenger":{"id":3,"name":"B"}}"""
        val r3 = """{"id":3,"status":"pending_driver","pickup_at":"2026-12-02 19:20:00",
            "area":{"id":2,"name":"PJ"},"destination":"Church","event":{"id":21,"title":"Prayer Meeting"},
            "passenger":{"id":4,"name":"C"}}"""

        server.enqueue(json("[$ev1,$ev2]"))
        server.enqueue(json("[$r1,$r2,$r3]"))

        val vm = EventsViewModel(container)
        val state = vm.state.awaitUntil { !it.loading }
        assertEquals("/api/events?scope=upcoming", server.takeRequest().path)
        assertEquals("/api/driver/requests?mine_areas=true", server.takeRequest().path)
        assertEquals(mapOf(20L to 2, 21L to 1), state.requestsByEvent)

        // My schedule: upcoming events + my rides + my driving schedule
        server.enqueue(json("[$ev1,$ev2]"))
        server.enqueue(json("[$r1]"))
        server.enqueue(json("[$r3]"))
        vm.setTab(1)
        val scheduleState = vm.state.awaitUntil { !it.loading && it.tab == 1 }
        assertEquals("/api/events?scope=upcoming", server.takeRequest().path)
        assertTrue(server.takeRequest().path!!.startsWith("/api/rides"))
        assertTrue(server.takeRequest().path!!.startsWith("/api/driver/schedule"))
        assertEquals(2, scheduleState.events.size)
        assertEquals(1, scheduleState.myRides.size)
        assertEquals(1, scheduleState.myDrives.size)
        assertTrue(scheduleState.requestsByEvent.isEmpty())
        vm.selectDay("2026-11-29")
        assertEquals("2026-11-29", vm.state.value.selectedDay)

        server.enqueue(json("[]"))
        vm.setTab(2)
        val pastState = vm.state.awaitUntil { !it.loading && it.tab == 2 }
        assertEquals("/api/events?scope=past", server.takeRequest().path)
        assertTrue(pastState.requestsByEvent.isEmpty())
        assertTrue(pastState.myRides.isEmpty())
        assertEquals("all", pastState.selectedDay)
    }

    @Test fun eventDetailLoadsScheduleRidesAndDriving() = runBlocking {
        signIn()
        server.takeRequest() // /api/auth/me
        val goingEvent = """{"id":20,"title":"Sunday Service","starts_at":"2026-11-29 09:00:00",
            "my_signup":{"status":"going"},"my_ride":{"id":44,"status":"confirmed"},"my_return_ride":{"id":45,"status":"pending_driver"}}"""
        val out = """{"id":44,"status":"confirmed","pickup_at":"2026-11-29 08:20:00","area":{"id":2,"name":"PJ"},
            "destination":"Church","event":{"id":20},"passenger":{"id":1,"name":"Test Member"}}"""
        val ret = """{"id":45,"status":"pending_driver","pickup_at":"2026-11-29 12:00:00","leg":"return","area":{"id":2,"name":"PJ"},
            "destination":"Home","event":{"id":20},"passenger":{"id":1,"name":"Test Member"}}"""
        val drive = """{"id":50,"status":"confirmed","pickup_at":"2026-11-29 08:10:00","area":{"id":2,"name":"PJ"},
            "destination":"Church","event":{"id":20},"passenger":{"id":7,"name":"Auntie"}}"""
        val otherDrive = """{"id":51,"status":"confirmed","pickup_at":"2026-11-29 18:10:00","area":{"id":2,"name":"PJ"},
            "destination":"Hall","event":{"id":99},"passenger":{"id":8,"name":"Uncle"}}"""
        val notSigned = """{"id":21,"title":"Prayer","starts_at":"2026-12-02 20:00:00"}"""

        dispatchByPath(
            mapOf(
                "/api/events/20" to { json(goingEvent) },
                "/api/events/21" to { json(notSigned) },
                "/api/rides" to { json("[$out,$ret]") },
                "/api/driver/schedule" to { json("[$drive,$otherDrive]") },
            )
        )

        val vm = EventDetailViewModel(container, 20L)
        val state = vm.state.awaitUntil { !it.loading && it.event != null }
        assertEquals("/api/events/20", server.takeRequest().path)
        val followPaths = setOf(server.takeRequest().path!!, server.takeRequest().path!!)
        assertTrue(followPaths.any { it.startsWith("/api/rides") })
        assertTrue(followPaths.contains("/api/driver/schedule?from=2026-11-29&to=2026-11-29"))
        assertEquals(44L, state.outboundRide?.id)
        assertEquals(45L, state.returnRide?.id)
        assertEquals(listOf(50L), state.drivingForEvent.map { it.id })

        // Not signed up: no ride or driving calls
        val vm2 = EventDetailViewModel(container, 21L)
        val s2 = vm2.state.awaitUntil { !it.loading && it.event != null }
        assertEquals("/api/events/21", server.takeRequest().path)
        assertNull(s2.outboundRide)
        assertTrue(s2.drivingForEvent.isEmpty())
        assertEquals(0, server.requestCount - 5)
    }

    @Test fun rideNewTripTypesAndReturnAddressSwap() = runBlocking {
        val areasJson = """[{"id":2,"name":"PJ Section 14","active":true}]"""
        val savedJson = """[{"id":5,"label":"Home","area_id":2,"address":"12 Jalan Home","is_default":1}]"""
        val eventJson = """{"id":20,"title":"Sunday Service","starts_at":"2026-11-29 10:00:00","location":"Church Hall","my_signup":{"status":"going"}}"""

        dispatchByPath(
            mapOf(
                "/api/areas" to { json(areasJson) },
                "/api/saved-places" to { json(savedJson) },
                "/api/proxy-riders" to { json("[]") },
                "/api/events/20" to { json(eventJson) },
            )
        )

        // Test 1: Return-only swaps address/destination and clears return fields
        val returnVm = RideNewViewModel(container, 20L, initialProxy = false)
        val loaded = returnVm.state.awaitUntil { !it.loading && it.event != null }
        assertEquals(TripType.OUTBOUND, loaded.tripType)
        assertEquals(9, loaded.hour)
        assertEquals(20, loaded.minute)
        assertEquals("12 Jalan Home", loaded.address)
        assertEquals("Church Hall", loaded.destination)

        returnVm.setTripType(TripType.RETURN)
        val retState = returnVm.state.value
        assertEquals(TripType.RETURN, retState.tripType)
        assertEquals(12, retState.hour)
        assertEquals(0, retState.minute)
        assertEquals("Church Hall", retState.address)
        assertEquals("12 Jalan Home", retState.destination)

        repeat(4) { server.takeRequest() } // drain init calls
        server.dispatcher = okhttp3.mockwebserver.QueueDispatcher()
        server.enqueue(json("""{"id":99}"""))
        returnVm.submit()
        returnVm.state.awaitUntil { it.createdId == 99L }
        val retPost = apiJson.parseToJsonElement(server.takeRequest().body.readUtf8()).jsonObject
        assertEquals("return", retPost["leg"]?.jsonPrimitive?.content)
        assertEquals("Church Hall", retPost["pickup_address"]?.jsonPrimitive?.content)
        assertEquals("12 Jalan Home", retPost["destination"]?.jsonPrimitive?.content)
        assertNull(retPost["return_pickup_at"])
        assertNull(retPost["return_pickup_address"])
        assertNull(retPost["return_destination"])

        // Test 2: Round-trip swaps return_pickup_address and return_destination
        // (/api/areas is served from the 5-minute repository cache on the same container)
        dispatchByPath(
            mapOf(
                "/api/areas" to { json(areasJson) },
                "/api/saved-places" to { json(savedJson) },
                "/api/proxy-riders" to { json("[]") },
                "/api/events/20" to { json(eventJson) },
            )
        )
        val roundVm = RideNewViewModel(container, 20L, initialProxy = false)
        roundVm.state.awaitUntil { !it.loading && it.event != null }
        repeat(3) { server.takeRequest() }
        server.dispatcher = okhttp3.mockwebserver.QueueDispatcher()
        roundVm.setTripType(TripType.ROUND)
        server.enqueue(json("""{"id":100}"""))
        roundVm.submit()
        roundVm.state.awaitUntil { it.createdId == 100L }
        val roundPost = apiJson.parseToJsonElement(server.takeRequest().body.readUtf8()).jsonObject
        assertEquals("outbound", roundPost["leg"]?.jsonPrimitive?.content)
        assertEquals("12 Jalan Home", roundPost["pickup_address"]?.jsonPrimitive?.content)
        assertEquals("Church Hall", roundPost["destination"]?.jsonPrimitive?.content)
        assertEquals("Church Hall", roundPost["return_pickup_address"]?.jsonPrimitive?.content)
        assertEquals("12 Jalan Home", roundPost["return_destination"]?.jsonPrimitive?.content)
        assertEquals("2026-11-29 12:00", roundPost["return_pickup_at"]?.jsonPrimitive?.content)
    }

    @Test fun rideNewProxyBookingAndSignupGate() = runBlocking {
        val areasJson = """[{"id":2,"name":"PJ Section 14","active":true}]"""
        val savedJson = """[{"id":5,"label":"Home","area_id":2,"address":"12 Jalan Home","is_default":1}]"""
        val notGoingEvent = """{"id":20,"title":"Sunday Service","starts_at":"2026-11-29 10:00:00","location":"Church Hall"}"""
        val goingEvent = """{"id":20,"title":"Sunday Service","starts_at":"2026-11-29 10:00:00","location":"Church Hall","my_signup":{"status":"going"}}"""

        var currentEventJson = notGoingEvent
        dispatchByPath(
            mapOf(
                "/api/areas" to { json(areasJson) },
                "/api/saved-places" to { json(savedJson) },
                "/api/proxy-riders" to { json("[]") },
                "/api/events/20" to { json(currentEventJson) },
            )
        )

        // Signup gate blocks submission when not going
        val blockedVm = RideNewViewModel(container, 20L)
        blockedVm.state.awaitUntil { !it.loading && it.event != null }
        repeat(4) { server.takeRequest() }
        blockedVm.submit()
        assertNotNull(blockedVm.state.value.error ?: blockedVm.state.value.errorRes)

        // Proxy booking sends proxy fields and rejects blank riderName
        currentEventJson = goingEvent
        val proxyVm = RideNewViewModel(container, 20L, initialProxy = true)
        val initState = proxyVm.state.awaitUntil { !it.loading && it.event != null }
        assertTrue(initState.isProxy)
        assertEquals("", initState.address)
        assertNull(initState.areaId)
        repeat(3) { server.takeRequest() } // /api/areas served from cache on the same container
        server.dispatcher = okhttp3.mockwebserver.QueueDispatcher()

        proxyVm.submit()
        assertNotNull(proxyVm.state.value.error ?: proxyVm.state.value.errorRes)

        proxyVm.update {
            it.copy(
                areaId = 2L,
                address = "88 Jalan Grandma",
                riderName = "Grandma Lee",
                riderPhone = "0123456789",
                riderRelationship = "parent",
                saveProxyRider = true,
            )
        }
        server.enqueue(json("""{"id":101}"""))
        proxyVm.submit()
        proxyVm.state.awaitUntil { it.createdId == 101L }
        val post = apiJson.parseToJsonElement(server.takeRequest().body.readUtf8()).jsonObject
        assertEquals(true, post["is_proxy"]?.jsonPrimitive?.boolean)
        assertEquals("Grandma Lee", post["rider_name"]?.jsonPrimitive?.content)
        assertEquals("0123456789", post["rider_phone"]?.jsonPrimitive?.content)
        assertEquals("parent", post["rider_relationship"]?.jsonPrimitive?.content)
        assertEquals(true, post["save_proxy_rider"]?.jsonPrimitive?.boolean)
    }

    @Test fun profileSavesNamePhoneAndAnimatedAvatarWithoutTelegram() = runBlocking {
        signIn()
        server.takeRequest() // /api/auth/me
        server.enqueue(json("[]")) // /api/auth/passkeys from init
        val vm = ProfileViewModel(container)
        server.takeRequest() // drain passkeys

        server.enqueue(json("""{"ok":true}"""))
        server.enqueue(json(Fixtures.ME_MEMBER))
        vm.saveProfile("Alice", "+60123456789")
        vm.state.awaitUntil { it.message == "saved" }
        val profilePut = apiJson.parseToJsonElement(server.takeRequest().body.readUtf8()).jsonObject
        server.takeRequest() // /api/auth/me refresh
        assertEquals("Alice", profilePut["name"]?.jsonPrimitive?.content)
        assertEquals("+60123456789", profilePut["phone"]?.jsonPrimitive?.content)
        assertFalse(profilePut.containsKey("telegram_chat_id"))

        vm.clearMessage()
        server.enqueue(json("""{"ok":true}"""))
        server.enqueue(json(Fixtures.ME_MEMBER))
        vm.saveAvatarStyle("grok", "purple")
        vm.state.awaitUntil { it.message == "saved" }
        val avatarPut = apiJson.parseToJsonElement(server.takeRequest().body.readUtf8()).jsonObject
        assertEquals(true, avatarPut["clear_photo"]?.jsonPrimitive?.boolean)
        assertEquals("grok", avatarPut["avatar_icon"]?.jsonPrimitive?.content)
        assertEquals("purple", avatarPut["avatar_color"]?.jsonPrimitive?.content)
        assertFalse(avatarPut.containsKey("photo_file_id"))
    }

    @Test fun savedAddressesCrudAndDefaultPromotion() = runBlocking {
        val initialPlaces = """[{"id":1,"label":"Home","area_id":2,"area_name":"PJ","address":"10 Jalan SS2","gate_notes":"Gate 2","is_default":1}]"""
        val twoPlaces = """[
            {"id":1,"label":"Home","area_id":2,"area_name":"PJ","address":"10 Jalan SS2","gate_notes":"Gate 2","is_default":1},
            {"id":2,"label":"Work","area_id":3,"area_name":"Subang","address":"88 Tower B","gate_notes":"Lobby","is_default":0}
        ]"""
        val promotedPlaces = """[
            {"id":2,"label":"Work","area_id":3,"area_name":"Subang","address":"88 Tower B","gate_notes":"Lobby","is_default":1},
            {"id":1,"label":"Home","area_id":2,"area_name":"PJ","address":"10 Jalan SS2","gate_notes":"Gate 2","is_default":0}
        ]"""
        val editedPlaces = """[
            {"id":2,"label":"Work HQ","area_id":3,"area_name":"Subang","address":"88 Tower B","gate_notes":"Gate 3","is_default":1},
            {"id":1,"label":"Home","area_id":2,"area_name":"PJ","address":"10 Jalan SS2","gate_notes":"Gate 2","is_default":0}
        ]"""
        val afterDelete = """[{"id":2,"label":"Work HQ","area_id":3,"area_name":"Subang","address":"88 Tower B","gate_notes":"Gate 3","is_default":1}]"""

        dispatchByPath(
            mapOf(
                "/api/saved-places" to { json(initialPlaces) },
                "/api/areas" to { json("""[{"id":2,"name":"PJ","active":true},{"id":3,"name":"Subang","active":true}]""") },
            )
        )
        val vm = SavedAddressesViewModel(container)
        val loaded = vm.state.awaitUntil { !it.loading }
        assertEquals(1, loaded.places.size)
        repeat(2) { server.takeRequest() }
        server.dispatcher = okhttp3.mockwebserver.QueueDispatcher()

        // Create second saved address
        server.enqueue(json("""{"id":2,"ok":true}"""))
        server.enqueue(json(twoPlaces))
        var doneCount = 0
        vm.save(null, "Work", 3L, "88 Tower B", "Lobby", false) { doneCount++ }
        vm.state.awaitUntil { it.message == "saved" }
        assertEquals(1, doneCount)
        assertEquals(2, vm.state.value.places.size)
        val postReq = server.takeRequest()
        assertEquals("POST", postReq.method)
        assertEquals("/api/saved-places", postReq.path)
        val postBody = apiJson.parseToJsonElement(postReq.body.readUtf8()).jsonObject
        assertEquals("Work", postBody["label"]?.jsonPrimitive?.content)
        assertEquals(false, postBody["is_default"]?.jsonPrimitive?.boolean)
        server.takeRequest() // GET /api/saved-places

        // 1-tap default promotion
        vm.clearMessage()
        server.enqueue(json("""{"ok":true}"""))
        server.enqueue(json(promotedPlaces))
        val work = vm.state.value.places.first { it.id == 2L }
        vm.setDefault(work)
        vm.state.awaitUntil { it.message == "default_updated" }
        assertEquals(2L, vm.state.value.places.first().id)
        val starReq = server.takeRequest()
        assertEquals("PUT", starReq.method)
        assertEquals("/api/saved-places/2", starReq.path)
        assertEquals(true, apiJson.parseToJsonElement(starReq.body.readUtf8()).jsonObject["is_default"]?.jsonPrimitive?.boolean)
        server.takeRequest() // GET /api/saved-places

        // Edit saved address
        vm.clearMessage()
        server.enqueue(json("""{"ok":true}"""))
        server.enqueue(json(editedPlaces))
        vm.save(2L, "Work HQ", 3L, "88 Tower B", "Gate 3", true) { doneCount++ }
        vm.state.awaitUntil { it.message == "saved" }
        assertEquals(2, doneCount)
        assertEquals("Work HQ", vm.state.value.places.first().label)
        repeat(2) { server.takeRequest() }

        // Delete saved address
        vm.clearMessage()
        server.enqueue(json("""{"ok":true}"""))
        server.enqueue(json(afterDelete))
        vm.delete(1L) { doneCount++ }
        vm.state.awaitUntil { it.message == "removed" }
        assertEquals(3, doneCount)
        assertEquals(1, vm.state.value.places.size)
        val delReq = server.takeRequest()
        assertEquals("DELETE", delReq.method)
        assertEquals("/api/saved-places/1", delReq.path)
    }

    @Test fun rideNewAppliesMultipleSavedPlacesAndSavesNewAddress() = runBlocking {
        val areasJson = """[{"id":2,"name":"PJ","active":true},{"id":3,"name":"Subang","active":true}]"""
        val savedJson = """[
            {"id":1,"label":"Home","area_id":2,"area_name":"PJ","address":"12 Jalan Home","gate_notes":"Ring bell 1","is_default":1},
            {"id":2,"label":"Office","area_id":3,"area_name":"Subang","address":"88 Tower B","gate_notes":"Guardhouse","is_default":0}
        ]"""
        val eventJson = """{"id":20,"title":"Sunday Service","starts_at":"2026-11-29 10:00:00","location":"Church Hall","my_signup":{"status":"going"}}"""

        dispatchByPath(
            mapOf(
                "/api/areas" to { json(areasJson) },
                "/api/saved-places" to { json(savedJson) },
                "/api/proxy-riders" to { json("[]") },
                "/api/events/20" to { json(eventJson) },
            )
        )

        val vm = RideNewViewModel(container, 20L, initialProxy = false)
        val init = vm.state.awaitUntil { !it.loading && it.event != null }
        assertEquals("12 Jalan Home", init.address)
        assertEquals("Ring bell 1", init.notes)
        repeat(4) { server.takeRequest() }
        server.dispatcher = okhttp3.mockwebserver.QueueDispatcher()

        val home = SavedPlace(1L, "Home", 2L, "PJ", "12 Jalan Home", "Ring bell 1", 1)
        val office = SavedPlace(2L, "Office", 3L, "Subang", "88 Tower B", "Guardhouse", 0)

        // Switching to Office swaps area, address, and gate notes cleanly
        vm.applySavedPlace(office)
        assertEquals(3L, vm.state.value.areaId)
        assertEquals("88 Tower B", vm.state.value.address)
        assertEquals("Guardhouse", vm.state.value.notes)

        // Switching to return trip & picking Home populates destination (not pickup)
        vm.setTripType(TripType.RETURN)
        vm.applySavedPlace(home)
        assertEquals("Church Hall", vm.state.value.address)
        assertEquals("12 Jalan Home", vm.state.value.destination)
        assertEquals("Ring bell 1", vm.state.value.notes)

        // Switching back to outbound, typing a new address and opting to save it on submit
        vm.setTripType(TripType.OUTBOUND)
        vm.update {
            it.copy(
                areaId = 2L,
                address = "77 Jalan Baru",
                saveAddress = true,
                saveAddressLabel = "Parents",
            )
        }
        server.enqueue(json("""{"id":105}"""))
        server.enqueue(json("""{"id":9,"ok":true}"""))
        vm.submit()
        vm.state.awaitUntil { it.createdId == 105L }

        val rideReq = server.takeRequest()
        assertEquals("/api/rides", rideReq.path)
        val saveReq = server.takeRequest()
        assertEquals("/api/saved-places", saveReq.path)
        val saveBody = apiJson.parseToJsonElement(saveReq.body.readUtf8()).jsonObject
        assertEquals("Parents", saveBody["label"]?.jsonPrimitive?.content)
        assertEquals(2, saveBody["area_id"]?.jsonPrimitive?.int)
        assertEquals("77 Jalan Baru", saveBody["address"]?.jsonPrimitive?.content)
        assertEquals(false, saveBody["is_default"]?.jsonPrimitive?.boolean)
    }

    @Test fun rideDetailDriverActionsAndWebUrl() = runBlocking {
        val driverRide = """{"id":77,"status":"pending_driver_confirm","pickup_at":"2026-11-29 09:00:00",
            "area":{"id":2,"name":"PJ"},"destination":"Church","seats":1,
            "passenger":{"id":5,"name":"Rider"},"viewer":{"is_passenger":false,"is_driver":true,"is_staff":false}}"""
        server.enqueue(json(driverRide))
        val vm = RideDetailViewModel(container, 77L)
        vm.state.awaitUntil { !it.loading && it.ride != null }
        assertEquals("/api/rides/77", server.takeRequest().path)

        fun enqueueAction() {
            server.enqueue(json("""{"ok":true}"""))
            server.enqueue(json(driverRide))
        }

        enqueueAction()
        vm.confirmAssignment()
        vm.state.awaitUntil { !it.busy }
        assertEquals("/api/rides/77/confirm-assignment", server.takeRequest().path)
        assertEquals("/api/rides/77", server.takeRequest().path)

        enqueueAction()
        vm.startRide()
        vm.state.awaitUntil { !it.busy }
        assertEquals("/api/rides/77/start", server.takeRequest().path)
        server.takeRequest()

        enqueueAction()
        vm.arrivedGate()
        vm.state.awaitUntil { !it.busy }
        assertEquals("/api/rides/77/arrived-gate", server.takeRequest().path)
        server.takeRequest()

        enqueueAction()
        vm.pickup()
        vm.state.awaitUntil { !it.busy }
        assertEquals("/api/rides/77/pickup", server.takeRequest().path)
        server.takeRequest()

        enqueueAction()
        vm.complete()
        vm.state.awaitUntil { !it.busy }
        assertEquals("/api/rides/77/complete", server.takeRequest().path)
        server.takeRequest()

        enqueueAction()
        vm.driverDelay(5)
        vm.state.awaitUntil { !it.busy }
        val delayReq = server.takeRequest()
        assertEquals("/api/rides/77/driver-delay", delayReq.path)
        assertEquals(5, apiJson.parseToJsonElement(delayReq.body.readUtf8()).jsonObject["delay_minutes"]?.jsonPrimitive?.int)
        server.takeRequest()

        enqueueAction()
        vm.withdraw("Car trouble")
        vm.state.awaitUntil { !it.busy }
        val withdrawReq = server.takeRequest()
        assertEquals("/api/rides/77/withdraw", withdrawReq.path)
        assertEquals("Car trouble", apiJson.parseToJsonElement(withdrawReq.body.readUtf8()).jsonObject["reason"]?.jsonPrimitive?.content)
        server.takeRequest()

        val web = container.webUrl("/rides/77")
        assertTrue(web.endsWith("/rides/77"))
        assertFalse(web.contains("//rides"))
    }

    @Test fun driverSettingsHandlesZeroVehiclesAndVehicleCrud() = runBlocking {
        signIn()
        server.takeRequest()
        val areasJson = """[{"id":1,"name":"Petaling Jaya","active":true},{"id":2,"name":"Subang Jaya","active":true}]"""
        val emptyProfile = """{"carpool_enabled":false,"spare_seats":1,"gate_wait_mins":5,"area_ids":[],"vehicle":null,"vehicles":[],"active_vehicle_id":null}"""
        val oneVehicleProfile = """{"carpool_enabled":false,"spare_seats":1,"gate_wait_mins":5,"area_ids":[],"vehicle":{"id":10,"model":"Toyota Vios","color":"Silver","plate":"WXX 8888","seats":5,"is_active":true},"vehicles":[{"id":10,"model":"Toyota Vios","color":"Silver","plate":"WXX 8888","seats":5,"is_active":true}],"active_vehicle_id":10}"""
        val twoVehicleClampedProfile = """{"carpool_enabled":true,"spare_seats":1,"gate_wait_mins":5,"area_ids":[1],"vehicle":{"id":11,"model":"Smart Fortwo","color":"Yellow","plate":"SMT 202","seats":2,"is_active":true},"vehicles":[{"id":10,"model":"Toyota Vios","color":"Silver","plate":"WXX 8888","seats":5,"is_active":false},{"id":11,"model":"Smart Fortwo","color":"Yellow","plate":"SMT 202","seats":2,"is_active":true}],"active_vehicle_id":11}"""

        dispatchByPath(
            mapOf(
                "/api/driver/profile" to { json(emptyProfile) },
                "/api/areas" to { json(areasJson) },
            )
        )

        val vm = DriverSettingsViewModel(container)
        val init = vm.state.awaitUntil { !it.loading }
        assertTrue(init.vehicles.isEmpty())
        assertFalse(init.dirty)
        assertNull(init.error)
        repeat(2) { server.takeRequest() }
        server.dispatcher = okhttp3.mockwebserver.QueueDispatcher()

        // Add first vehicle -> dirty stays false
        server.enqueue(json("""{"id":10,"model":"Toyota Vios","color":"Silver","plate":"WXX 8888","seats":5,"is_active":true}"""))
        server.enqueue(json(oneVehicleProfile))
        var done = 0
        vm.saveVehicle(null, DriverVehicleReq("Toyota Vios", "Silver", "WXX 8888", 5, is_active = true)) { done++ }
        vm.state.awaitUntil { it.message == "vehicle_saved" }
        assertEquals(1, done)
        assertEquals(1, vm.state.value.vehicles.size)
        assertEquals(4, vm.state.value.maxSpareSeats)
        assertFalse(vm.state.value.dirty)
        repeat(2) { server.takeRequest() }

        // Toggle area + carpool + raise spare seats to 3 -> dirty becomes true, save via PUT /api/driver/profile
        vm.toggleArea(1L)
        vm.setCarpool(true)
        vm.stepSpareSeats(2)
        assertTrue(vm.state.value.dirty)
        assertEquals(3, vm.state.value.spareSeats)

        server.enqueue(json("""{"ok":true}"""))
        vm.saveProfile()
        vm.state.awaitUntil { it.message == "saved" }
        assertFalse(vm.state.value.dirty)
        val putReq = server.takeRequest()
        assertEquals("PUT", putReq.method)
        assertEquals("/api/driver/profile", putReq.path)
        val putBody = apiJson.parseToJsonElement(putReq.body.readUtf8()).jsonObject
        assertEquals(3, putBody["spare_seats"]?.jsonPrimitive?.int)
        assertEquals(1, putBody["area_ids"]?.jsonArray?.size)

        // Switching active vehicle to 2-seat car clamps spareSeats to 1 and keeps dirty false
        server.enqueue(json("""{"ok":true,"active_vehicle_id":11}"""))
        server.enqueue(json(twoVehicleClampedProfile))
        vm.selectVehicle(11L)
        vm.state.awaitUntil { it.message == "vehicle_selected" }
        assertEquals(1, vm.state.value.spareSeats)
        assertEquals(1, vm.state.value.maxSpareSeats)
        assertFalse(vm.state.value.dirty)
    }

    @Test fun savedAddressesCapturesCoordsAndInfersArea() = runBlocking {
        val areas = listOf(
            Area(1L, "PJ", true),
            Area(2L, "Subang Jaya", true),
        )
        assertEquals(2L, inferAreaId("12 Jalan SS15, Subang Jaya, Selangor", areas))
        assertEquals(1L, inferAreaId("88 Jalan Barat, PJ", areas))
        assertNull(inferAreaId("Unknown Street", areas))

        dispatchByPath(
            mapOf(
                "/api/saved-places" to { json("[]") },
                "/api/areas" to { json("""[{"id":1,"name":"PJ","active":true},{"id":2,"name":"Subang Jaya","active":true}]""") },
            )
        )
        val vm = SavedAddressesViewModel(container)
        vm.state.awaitUntil { !it.loading }
        repeat(2) { server.takeRequest() }
        server.dispatcher = okhttp3.mockwebserver.QueueDispatcher()

        server.enqueue(json("""{"id":5,"ok":true,"lat":3.073,"lng":101.588}"""))
        server.enqueue(json("""[{"id":5,"label":"Work","area_id":2,"area_name":"Subang Jaya","address":"Jalan SS15, Subang Jaya","gate_notes":"","is_default":1,"lat":3.073,"lng":101.588}]"""))
        vm.save(null, "Work", 2L, "Jalan SS15, Subang Jaya", "", true, lat = 3.073, lng = 101.588)
        vm.state.awaitUntil { it.message == "saved" }
        val postReq = server.takeRequest()
        val postBody = apiJson.parseToJsonElement(postReq.body.readUtf8()).jsonObject
        assertEquals(3.073, postBody["lat"]?.jsonPrimitive?.double ?: 0.0, 0.0001)
        assertEquals(101.588, postBody["lng"]?.jsonPrimitive?.double ?: 0.0, 0.0001)
    }

    @Test fun proxyRidersCrudAndRefresh() = runBlocking {
        dispatchByPath(
            mapOf(
                "/api/proxy-riders" to { json("[]") },
                "/api/areas" to { json("""[{"id":1,"name":"PJ","active":true}]""") },
            )
        )
        val vm = ProxyRidersViewModel(container)
        val init = vm.state.awaitUntil { !it.loading }
        assertTrue(init.riders.isEmpty())
        repeat(2) { server.takeRequest() }
        server.dispatcher = okhttp3.mockwebserver.QueueDispatcher()

        server.enqueue(json("""{"id":9,"ok":true}"""))
        server.enqueue(json("""[{"id":9,"name":"Sister Chen","phone":"012-3456","relationship":"parent","notes":"Cane","area_id":1,"pickup_address":"10 SS2","gate_notes":"Gate 1"}]"""))
        var done = 0
        vm.save(null, ProxyRiderReq("Sister Chen", "012-3456", "parent", "Cane", 1L, "10 SS2", "Gate 1")) { done++ }
        vm.state.awaitUntil { it.message == "saved" }
        assertEquals(1, done)
        assertEquals(1, vm.state.value.riders.size)
        val createReq = server.takeRequest()
        assertEquals("POST", createReq.method)
        assertEquals("/api/proxy-riders", createReq.path)
        server.takeRequest()

        vm.clearMessage()
        server.enqueue(json("""{"ok":true}"""))
        server.enqueue(json("""[{"id":9,"name":"Elder Chen","phone":"012-3456","relationship":"parent","notes":"Walker","area_id":1,"pickup_address":"10 SS2","gate_notes":"Gate 1"}]"""))
        vm.save(9L, ProxyRiderReq("Elder Chen", "012-3456", "parent", "Walker", 1L, "10 SS2", "Gate 1")) { done++ }
        vm.state.awaitUntil { it.message == "saved" }
        assertEquals(2, done)
        assertEquals("Elder Chen", vm.state.value.riders.first().name)
        val updReq = server.takeRequest()
        assertEquals("PUT", updReq.method)
        assertEquals("/api/proxy-riders/9", updReq.path)
        server.takeRequest()

        vm.clearMessage()
        server.enqueue(json("""{"ok":true}"""))
        server.enqueue(json("[]"))
        vm.delete(9L) { done++ }
        vm.state.awaitUntil { it.message == "removed" }
        assertEquals(3, done)
        assertTrue(vm.state.value.riders.isEmpty())
        val delReq = server.takeRequest()
        assertEquals("DELETE", delReq.method)
        assertEquals("/api/proxy-riders/9", delReq.path)
    }

    @Test fun eventsManagingHideDraftsPublishAndUnpublish() = runBlocking {
        signIn()
        server.takeRequest() // /api/auth/me

        val draftEv = """{"id":71,"title":"Secret Retreat","kind":"other","icon":"tent","starts_at":"2026-12-10 09:00:00","status":"draft","can_manage":true,"can_publish":true,"can_unpublish":false}"""
        val pubEv = """{"id":72,"title":"Open Praise","kind":"other","icon":"music","starts_at":"2026-12-11 19:00:00","status":"published","can_manage":true,"can_publish":false,"can_unpublish":true}"""

        server.enqueue(json("[]")) // init upcoming
        server.enqueue(json("[]")) // driver requests
        val vm = EventsViewModel(container)
        vm.state.awaitUntil { !it.loading }
        repeat(2) { server.takeRequest() }

        server.enqueue(json("[$draftEv,$pubEv]"))
        vm.setTab(3)
        val managingState = vm.state.awaitUntil { !it.loading && it.tab == 3 }
        assertEquals("/api/events?scope=managing", server.takeRequest().path)
        assertEquals(2, managingState.events.size)
        assertEquals(1, managingState.draftCount)
        assertFalse(managingState.hideDrafts)

        server.enqueue(json("[$pubEv]"))
        vm.toggleHideDrafts()
        val hiddenState = vm.state.awaitUntil { !it.refreshing && it.hideDrafts }
        assertEquals("/api/events?scope=managing&hide_drafts=true", server.takeRequest().path)
        assertEquals(1, hiddenState.events.size)
        assertEquals(72L, hiddenState.events[0].id)

        // Toggle back to show drafts and publish draft 71
        server.enqueue(json("[$draftEv,$pubEv]"))
        vm.toggleHideDrafts()
        vm.state.awaitUntil { !it.refreshing && !it.hideDrafts }
        server.takeRequest()

        server.enqueue(json("""{"ok":true}"""))
        server.enqueue(json("[$pubEv]"))
        vm.publishEvent(71L)
        val publishedState = vm.state.awaitUntil { it.actionNotice == EventActionNotice.PUBLISHED && !it.refreshing }
        val pubReq = server.takeRequest()
        assertEquals("POST", pubReq.method)
        assertEquals("/api/events/71/publish", pubReq.path)
        assertEquals("/api/events?scope=managing", server.takeRequest().path)
        assertEquals(0, publishedState.draftCount)

        // Unpublish event 72
        vm.clearNotice()
        server.enqueue(json("""{"ok":true}"""))
        server.enqueue(json("[$draftEv]"))
        vm.unpublishEvent(72L)
        val unpubState = vm.state.awaitUntil { it.actionNotice == EventActionNotice.UNPUBLISHED && !it.refreshing }
        val unpubReq = server.takeRequest()
        assertEquals("POST", unpubReq.method)
        assertEquals("/api/events/72/unpublish", unpubReq.path)
        assertEquals("/api/events?scope=managing", server.takeRequest().path)
        assertEquals(EventActionNotice.UNPUBLISHED, unpubState.actionNotice)
    }

    @Test fun eventDetailPublishAndUnpublishConflict() = runBlocking {
        signIn()
        server.takeRequest() // /api/auth/me

        val draftDetail = """{"id":81,"title":"Draft Prayer","kind":"other","icon":"flame","starts_at":"2026-12-12 20:00:00","status":"draft","can_manage":true,"can_publish":true,"can_unpublish":false}"""
        val pubDetail = """{"id":81,"title":"Draft Prayer","kind":"other","icon":"flame","starts_at":"2026-12-12 20:00:00","status":"published","can_manage":true,"can_publish":false,"can_unpublish":false}"""

        server.enqueue(json(draftDetail))
        val vm = EventDetailViewModel(container, 81L)
        val init = vm.state.awaitUntil { !it.loading && it.event != null }
        assertEquals("/api/events/81", server.takeRequest().path)
        assertEquals("draft", init.event?.status)

        server.enqueue(json("""{"ok":true}"""))
        server.enqueue(json(pubDetail))
        vm.publish()
        val afterPub = vm.state.awaitUntil { !it.busy && it.actionNotice == EventActionNotice.PUBLISHED && it.event?.status == "published" }
        val pubReq = server.takeRequest()
        assertEquals("POST", pubReq.method)
        assertEquals("/api/events/81/publish", pubReq.path)
        assertEquals("/api/events/81", server.takeRequest().path)
        assertEquals("published", afterPub.event?.status)

        server.enqueue(
            MockResponse()
                .setResponseCode(409)
                .setBody("""{"error":"Cannot hide event with active signups","code":"EVENT_CANNOT_HIDE_WITH_SIGNUPS"}""")
        )
        vm.unpublish()
        val conflict = vm.state.awaitUntil { !it.busy && it.message != null }
        val unpubReq = server.takeRequest()
        assertEquals("POST", unpubReq.method)
        assertEquals("/api/events/81/unpublish", unpubReq.path)
        assertEquals("Cannot hide event with active signups", conflict.message)
    }

    @Test fun rideDetailRefreshKeepsContentAndUpdatesRide() = runBlocking {
        signIn()
        server.takeRequest() // /api/auth/me

        val ridePending = """{"id":10,"status":"pending_driver","pickup_at":"2026-11-22 09:30:00","area":{"id":2,"name":"SS2"},"destination":"Church Hall","passenger":{"id":1,"name":"Test Member"},"viewer":{"is_passenger":true}}"""
        val rideConfirmed = """{"id":10,"status":"confirmed","pickup_at":"2026-11-22 09:30:00","area":{"id":2,"name":"SS2"},"destination":"Church Hall","passenger":{"id":1,"name":"Test Member"},"viewer":{"is_passenger":true}}"""

        server.enqueue(json(ridePending))
        val vm = RideDetailViewModel(container, 10L)
        val initial = vm.state.awaitUntil { !it.loading && it.ride != null }
        assertEquals("pending_driver", initial.ride?.status)
        server.takeRequest()

        server.enqueue(json(rideConfirmed))
        vm.refresh()
        val refreshed = vm.state.awaitUntil { !it.refreshing && it.ride?.status == "confirmed" }
        assertFalse(refreshed.loading)
        assertEquals("confirmed", refreshed.ride?.status)
    }

    @Test fun eventDetailRefreshUpdatesEvent() = runBlocking {
        signIn()
        server.takeRequest() // /api/auth/me

        val event1 = """{"id":91,"title":"Prayer Night","starts_at":"2026-12-15 20:00:00","status":"published","going":5,"capacity":20}"""
        val event2 = """{"id":91,"title":"Prayer Night","starts_at":"2026-12-15 20:00:00","status":"published","going":8,"capacity":20}"""

        server.enqueue(json(event1))
        val vm = EventDetailViewModel(container, 91L)
        val initial = vm.state.awaitUntil { !it.loading && it.event != null }
        assertEquals(5L, initial.event?.going)
        server.takeRequest()

        server.enqueue(json(event2))
        vm.refresh()
        val refreshed = vm.state.awaitUntil { !it.refreshing && it.event?.going == 8L }
        assertFalse(refreshed.loading)
        assertEquals(8L, refreshed.event?.going)
    }

    @Test fun tripChatViewModelLoadsSendsAndReacts() = runBlocking {
        signIn()
        val recordedPaths = java.util.concurrent.CopyOnWriteArrayList<String>()
        var messagesJson = """{"open":true,"members":["Alice","Bob"],"reactions_allowed":["👍","❤️"],"messages":[{"id":1,"name":"Alice","body":"At gate","created_at":"2026-11-22 09:00:00","mine":false,"reactions":[]}]}"""

        server.dispatcher = object : okhttp3.mockwebserver.Dispatcher() {
            override fun dispatch(request: okhttp3.mockwebserver.RecordedRequest): MockResponse {
                val p = request.path ?: ""
                recordedPaths.add("${request.method} $p")
                return when {
                    p == "/api/rides/10/messages" && request.method == "GET" -> json(messagesJson)
                    p == "/api/rides/10/messages" && request.method == "POST" -> {
                        messagesJson = """{"open":true,"members":["Alice","Bob"],"reactions_allowed":["👍","❤️"],"messages":[{"id":1,"name":"Alice","body":"At gate","created_at":"2026-11-22 09:00:00","mine":false,"reactions":[]},{"id":2,"name":"Test Member","body":"On my way","created_at":"2026-11-22 09:01:00","mine":true,"status":"sent","reactions":[]}]}"""
                        json("""{"ok":true,"id":2}""")
                    }
                    p == "/api/rides/10/messages/read" -> json("""{"ok":true}""")
                    p == "/api/rides/10/messages/1/reactions" -> json("""{"ok":true,"emoji":"👍"}""")
                    else -> MockResponse().setBody("{}")
                }
            }
        }

        val vm = org.pjstatecm.graceride.ui.screens.rides.TripChatViewModel(container, 10L)
        val loaded = vm.state.awaitUntil { !it.loading && it.messages.size == 1 }
        assertEquals(listOf("Alice", "Bob"), loaded.members)
        assertEquals("At gate", loaded.messages.first().body)

        var sentCallbackFired = false
        vm.send("On my way") { sentCallbackFired = true }
        val afterSend = vm.state.awaitUntil { !it.sending && it.messages.size == 2 }
        assertTrue(sentCallbackFired)
        assertEquals("On my way", afterSend.messages.last().body)

        vm.react(1L, "👍")
        val afterReact = kotlinx.coroutines.withTimeout(5.seconds) {
            while (!recordedPaths.any { it == "POST /api/rides/10/messages/1/reactions" }) {
                kotlinx.coroutines.delay(15)
            }
            vm.state.value
        }
        assertNotNull(afterReact)
        assertTrue(recordedPaths.contains("POST /api/rides/10/messages/read"))
        assertTrue(recordedPaths.contains("POST /api/rides/10/messages"))
        assertTrue(recordedPaths.contains("POST /api/rides/10/messages/1/reactions"))
    }

    @Test fun driveHubViewModelLoadsAndClaimsRide() = runBlocking {
        signIn()
        val recordedPaths = java.util.concurrent.CopyOnWriteArrayList<String>()
        val openRide = """{"id":55,"status":"pending_driver","pickup_at":"2026-11-25 09:00:00","area":{"id":1,"name":"PJ"},"destination":"Main Sanctuary","seats":2,"passenger":{"id":9,"name":"Grace"},"viewer":{"is_passenger":false,"is_driver":false,"is_staff":false}}"""
        val scheduledRide = """{"id":56,"status":"confirmed","pickup_at":"2026-11-26 09:00:00","area":{"id":1,"name":"PJ"},"destination":"Church Hall","seats":1,"passenger":{"id":8,"name":"Peter"},"viewer":{"is_passenger":false,"is_driver":true,"is_staff":false}}"""
        val routeJson = """{"route":{"trip_id":99,"live":true,"onboard":1,"stops":[],"destination":{"name":"Church"},"distance_m":3200.0,"duration_s":420.0}}"""

        server.dispatcher = object : okhttp3.mockwebserver.Dispatcher() {
            override fun dispatch(request: okhttp3.mockwebserver.RecordedRequest): MockResponse {
                val p = request.path ?: ""
                recordedPaths.add("${request.method} $p")
                return when {
                    p.startsWith("/api/driver/requests") -> json("""[$openRide]""")
                    p.startsWith("/api/driver/schedule") -> json("""[$scheduledRide]""")
                    p.startsWith("/api/rides/55/claim") -> json("""{"ok":true}""")
                    p == "/api/driver/route" -> json(routeJson)
                    else -> MockResponse().setBody("[]")
                }
            }
        }

        val vm = org.pjstatecm.graceride.ui.screens.drive.DriveHubViewModel(container)
        val loaded = vm.state.awaitUntil { !it.loading && it.requests.isNotEmpty() && it.route != null }
        assertEquals(55L, loaded.requests.single().id)
        assertEquals(56L, loaded.schedule.single().id)
        assertEquals(99L, loaded.route?.tripId)

        vm.claim(55L)
        val claimed = vm.state.awaitUntil { it.claimedNotice && it.busyRideId == null }
        assertTrue(claimed.claimedNotice)
        assertTrue(recordedPaths.contains("POST /api/rides/55/claim"))
    }

    @Test fun rideDetailLoadsLiveLocationAndSharesPassengerPin() = runBlocking {
        signIn()
        var passengerShared = false
        val rideJson = """{"id":10,"status":"in_progress","pickup_at":"2026-11-22 09:30:00","area":{"id":2,"name":"SS2"},"destination":"Church Hall","trip_id":7,"passenger":{"id":1,"name":"Test Member"},"viewer":{"is_passenger":true}}"""

        dispatchByPath(
            mapOf(
                "/api/rides/10/location" to {
                    val pJson = if (passengerShared) ""","passenger":{"lat":3.1234,"lng":101.5678,"updated_at":"2026-11-22 09:35:00"}""" else ""
                    json("""{"sharing":true,"lat":3.11,"lng":101.55,"heading":90.0$pJson,"eta":{"phase":"pickup","located":true,"stops_before":0,"eta":"09:38"}}""")
                },
                "/api/rides/10/passenger-location" to {
                    passengerShared = !passengerShared
                    json("""{"ok":true,"sharing":$passengerShared}""")
                },
                "/api/rides/10" to { json(rideJson) },
            ),
        )

        val vm = RideDetailViewModel(container, 10L)
        val initial = vm.state.awaitUntil { !it.loading && it.liveLocation != null }
        assertTrue(initial.liveLocation?.sharing == true)
        assertEquals("09:38", initial.liveLocation?.eta?.eta)

        vm.sharePassengerSpot(3.1234, 101.5678)
        val withPin = vm.state.awaitUntil { it.sharingPassengerPin && it.liveLocation?.passenger != null }
        assertEquals(3.1234, withPin.liveLocation?.passenger?.lat ?: 0.0, 0.0001)

        vm.stopPassengerSpot()
        val afterStop = vm.state.awaitUntil { !it.sharingPassengerPin && it.liveLocation?.passenger == null }
        assertFalse(afterStop.sharingPassengerPin)
    }

    @Test fun eventDetailSelfCheckinAndScanMemberPass() = runBlocking {
        signIn()
        val eventJson = """{"id":91,"title":"Sunday Service","starts_at":"2026-12-15 09:00:00","status":"published","attendance_enabled":true,"can_manage":true,"going":12,"my_signup":{"status":"going","attended":false,"pass_code":"ABCD"}}"""

        dispatchByPath(
            mapOf(
                "/api/events/91/checkin/scan-member" to {
                    json("""{"ok":true,"user_id":14,"member_name":"Grace Lim","already_attended":false}""")
                },
                "/api/events/91/checkin" to {
                    json("""{"ok":true,"already_attended":false,"event_id":91,"title":"Sunday Service","starts_at":"2026-12-15 09:00:00","location":"Main Hall"}""")
                },
                "/api/events/91" to { json(eventJson) },
            ),
        )

        val vm = EventDetailViewModel(container, 91L)
        vm.state.awaitUntil { !it.loading && it.event != null }

        vm.selfCheckin("GR2026")
        val checkedIn = vm.state.awaitUntil { !it.busy && it.checkinResult != null }
        assertEquals("Sunday Service", checkedIn.checkinResult?.title)
        assertFalse(checkedIn.checkinResult?.alreadyAttended ?: true)

        vm.scanMemberPass("GRPASS:91:XYZ")
        val scanned = vm.state.awaitUntil { !it.busy && it.scanMemberResult != null }
        assertEquals("Grace Lim", scanned.scanMemberResult?.memberName)
        assertEquals(14L, scanned.scanMemberResult?.userId)
    }

    @Test fun costsValidateProofInputChecksPhotoAndMinimumRefLength() {
        assertEquals(R.string.costs_err_missing_photo, CostsViewModel.validateProofInput(hasPhoto = false, txnRef = "1234"))
        assertEquals(R.string.costs_err_missing_ref, CostsViewModel.validateProofInput(hasPhoto = true, txnRef = "   "))
        assertEquals(R.string.costs_err_ref_short, CostsViewModel.validateProofInput(hasPhoto = true, txnRef = " 123 "))
        assertNull(CostsViewModel.validateProofInput(hasPhoto = true, txnRef = "1234"))
    }

    @Test fun driveHubNonDriverSetsAccessDeniedWithoutNetworkCalls() = runBlocking {
        server.enqueue(json("""{"id":1,"name":"Member","email":"m@x.org","status":"active","permissions":["rides.view"],"mfa_ok":true}"""))
        container.auth.refresh()
        var driverCalls = 0
        dispatchByPath(
            mapOf(
                "/api/driver" to {
                    driverCalls++
                    json("[]")
                },
                "/api/rides/open" to {
                    driverCalls++
                    json("[]")
                },
            ),
        )
        val vm = org.pjstatecm.graceride.ui.screens.drive.DriveHubViewModel(container)
        val denied = vm.state.awaitUntil { !it.loading && it.accessDenied }
        assertTrue(denied.accessDenied)
        assertEquals(0, driverCalls)
    }

    @Test fun homeViewModelExposesFullUnpaidShareCountAndFiltersNoShowDrives() = runBlocking {
        signIn()
        val tomorrow = KlTime.todayPlusDays(1)
        val share2 = Fixtures.SHARE.replace("\"id\":30", "\"id\":31")
        val share3 = Fixtures.SHARE.replace("\"id\":30", "\"id\":32")
        val driveConfirmed = """{"id":101,"status":"confirmed","pickup_at":"$tomorrow 09:00:00","area":{"id":1,"name":"PJ"},"destination":"Hall","seats":1,"passenger":{"id":2,"name":"Rider A"}}"""
        val driveNoShow = """{"id":102,"status":"no_show","pickup_at":"$tomorrow 10:00:00","area":{"id":1,"name":"PJ"},"destination":"Hall","seats":1,"passenger":{"id":3,"name":"Rider B"}}"""
        dispatchByPath(
            mapOf(
                "/api/rides" to { json("[]") },
                "/api/events" to { json("[]") },
                "/api/me/shares" to { json("[${Fixtures.SHARE},$share2,$share3]") },
                "/api/announcements" to { json("[]") },
                "/api/driver/schedule" to { json("[$driveConfirmed,$driveNoShow]") },
            ),
        )
        val vm = HomeViewModel(container)
        val state = vm.state.awaitUntil { !it.loading }
        assertEquals(3, state.unpaidShareCount)
        assertEquals(listOf(101L), state.upcomingDrives.map { it.id })
    }

    @Test fun rideDetailSkipsLocationFetchWhenViewerIsUnrelated() = runBlocking {
        signIn()
        var unrelatedLocationCalls = 0
        var passengerLocationCalls = 0
        val unrelatedRide = """{"id":88,"status":"in_progress","pickup_at":"2026-11-22 09:30:00","area":{"id":2,"name":"SS2"},"destination":"Church Hall","trip_id":7,"passenger":{"id":5,"name":"Other Member"},"viewer":{"is_passenger":false,"is_driver":false,"is_staff":false}}"""
        val passengerRide = """{"id":89,"status":"in_progress","pickup_at":"2026-11-22 09:30:00","area":{"id":2,"name":"SS2"},"destination":"Church Hall","trip_id":8,"passenger":{"id":1,"name":"Test Member"},"viewer":{"is_passenger":true,"is_driver":false,"is_staff":false}}"""
        dispatchByPath(
            mapOf(
                "/api/rides/88/location" to {
                    unrelatedLocationCalls++
                    json("""{"sharing":true}""")
                },
                "/api/rides/88" to { json(unrelatedRide) },
                "/api/rides/89/location" to {
                    passengerLocationCalls++
                    json("""{"sharing":true}""")
                },
                "/api/rides/89" to { json(passengerRide) },
            ),
        )
        val vm88 = RideDetailViewModel(container, 88L)
        val st88 = vm88.state.awaitUntil { !it.loading && it.ride != null }
        assertEquals(0, unrelatedLocationCalls)
        assertNull(st88.liveLocation)

        val vm89 = RideDetailViewModel(container, 89L)
        val st89 = vm89.state.awaitUntil { !it.loading && it.liveLocation != null }
        assertTrue(passengerLocationCalls >= 1)
        assertTrue(st89.liveLocation?.sharing == true)
    }

    @Test fun rideDetailIgnoresPassengerSpotWhenNotInProgressOrAlreadyPickedUp() = runBlocking {
        signIn()
        var passengerSpotPosts = 0
        val confirmedRide = """{"id":90,"status":"confirmed","pickup_at":"2026-11-22 09:30:00","area":{"id":2,"name":"SS2"},"destination":"Church Hall","trip_id":9,"passenger":{"id":1,"name":"Test Member"},"viewer":{"is_passenger":true,"is_driver":false,"is_staff":false}}"""
        val pickedUpRide = """{"id":91,"status":"in_progress","picked_up_at":"2026-11-22 09:15:00","pickup_at":"2026-11-22 09:30:00","area":{"id":2,"name":"SS2"},"destination":"Church Hall","trip_id":9,"passenger":{"id":1,"name":"Test Member"},"viewer":{"is_passenger":true,"is_driver":false,"is_staff":false}}"""
        dispatchByPath(
            mapOf(
                "/api/rides/90/passenger-location" to {
                    passengerSpotPosts++
                    json("""{"ok":true,"sharing":true}""")
                },
                "/api/rides/90" to { json(confirmedRide) },
                "/api/rides/91/passenger-location" to {
                    passengerSpotPosts++
                    json("""{"ok":true,"sharing":true}""")
                },
                "/api/rides/91/location" to { json("""{"sharing":true}""") },
                "/api/rides/91" to { json(pickedUpRide) },
            ),
        )
        val vm90 = RideDetailViewModel(container, 90L)
        vm90.state.awaitUntil { !it.loading && it.ride != null }
        vm90.sharePassengerSpot(3.12, 101.62)

        val vm91 = RideDetailViewModel(container, 91L)
        vm91.state.awaitUntil { !it.loading && it.ride != null }
        vm91.sharePassengerSpot(3.12, 101.62)

        assertEquals(0, passengerSpotPosts)
    }
}
