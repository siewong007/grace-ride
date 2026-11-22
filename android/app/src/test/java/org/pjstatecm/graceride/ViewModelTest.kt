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
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.ui.screens.events.EventsViewModel
import org.pjstatecm.graceride.ui.screens.home.HomeViewModel
import org.pjstatecm.graceride.ui.screens.login.LoginViewModel
import org.pjstatecm.graceride.ui.screens.profile.ProfileViewModel
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

    @Test fun homeLoadsAllSections() = runBlocking {
        signIn()
        server.enqueue(json("[${Fixtures.RIDE}]"))       // rides
        server.enqueue(json("[${Fixtures.EVENT}]"))      // events
        server.enqueue(json("[${Fixtures.SHARE}]"))      // shares
        server.enqueue(json("[${Fixtures.ANN}]"))        // announcements
        server.enqueue(json("[]"))                       // driver schedule

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
        server.enqueue(MockResponse().setResponseCode(500).setBody("""{"error":"boom"}""")) // rides
        server.enqueue(json("[${Fixtures.EVENT}]"))
        server.enqueue(json("[]"))
        server.enqueue(json("[${Fixtures.ANN}]"))
        server.enqueue(json("[]"))

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

        server.enqueue(json("[]"))
        server.enqueue(json("[]"))
        server.enqueue(json("[]"))
        server.enqueue(json("[]"))
        server.enqueue(json("[$driveToday,$driveLater]"))

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

        server.enqueue(json("[$passengerRide]"))
        server.enqueue(json("[]"))
        server.enqueue(json("[]"))
        server.enqueue(json("[]"))
        server.enqueue(json("[$driverRide]"))

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
        server.enqueue(json(goingEvent))
        server.enqueue(json("[$out,$ret]"))
        server.enqueue(json("[$drive,$otherDrive]"))

        val vm = org.pjstatecm.graceride.ui.screens.events.EventDetailViewModel(container, 20L)
        val state = vm.state.awaitUntil { !it.loading && it.event != null }
        assertEquals("/api/events/20", server.takeRequest().path)
        assertTrue(server.takeRequest().path!!.startsWith("/api/rides"))
        assertEquals("/api/driver/schedule?from=2026-11-29&to=2026-11-29", server.takeRequest().path)
        assertEquals(44L, state.outboundRide?.id)
        assertEquals(45L, state.returnRide?.id)
        assertEquals(listOf(50L), state.drivingForEvent.map { it.id })

        // Not signed up: no ride or driving calls
        val notSigned = """{"id":21,"title":"Prayer","starts_at":"2026-12-02 20:00:00"}"""
        server.enqueue(json(notSigned))
        val vm2 = org.pjstatecm.graceride.ui.screens.events.EventDetailViewModel(container, 21L)
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

        // Test 1: Return-only swaps address/destination and clears return fields
        server.enqueue(json(areasJson))
        server.enqueue(json(savedJson))
        server.enqueue(json("[]"))
        server.enqueue(json(eventJson))

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
        server.enqueue(json(areasJson))
        server.enqueue(json(savedJson))
        server.enqueue(json("[]"))
        server.enqueue(json(eventJson))

        val roundVm = RideNewViewModel(container, 20L, initialProxy = false)
        roundVm.state.awaitUntil { !it.loading && it.event != null }
        repeat(4) { server.takeRequest() }
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

        // Signup gate blocks submission when not going
        server.enqueue(json(areasJson))
        server.enqueue(json(savedJson))
        server.enqueue(json("[]"))
        server.enqueue(json(notGoingEvent))
        val blockedVm = RideNewViewModel(container, 20L)
        blockedVm.state.awaitUntil { !it.loading && it.event != null }
        repeat(4) { server.takeRequest() }
        blockedVm.submit()
        assertNotNull(blockedVm.state.value.error)

        // Proxy booking sends proxy fields and rejects blank riderName
        server.enqueue(json(areasJson))
        server.enqueue(json(savedJson))
        server.enqueue(json("[]"))
        server.enqueue(json(goingEvent))
        val proxyVm = RideNewViewModel(container, 20L, initialProxy = true)
        val initState = proxyVm.state.awaitUntil { !it.loading && it.event != null }
        assertTrue(initState.isProxy)
        repeat(4) { server.takeRequest() }

        proxyVm.submit()
        assertNotNull(proxyVm.state.value.error)

        proxyVm.update {
            it.copy(
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

    @Test fun profileNormalizesTelegramAndSavesAnimatedAvatar() = runBlocking {
        signIn()
        server.takeRequest() // /api/auth/me
        server.enqueue(json("[]")) // /api/auth/passkeys from init
        val vm = ProfileViewModel(container)
        server.takeRequest() // drain passkeys

        server.enqueue(json("""{"ok":true}"""))
        server.enqueue(json(Fixtures.ME_MEMBER))
        vm.saveProfile("Alice", "+60123456789", "  @alice_tg  ")
        vm.state.awaitUntil { it.message == "saved" }
        val profilePut = apiJson.parseToJsonElement(server.takeRequest().body.readUtf8()).jsonObject
        server.takeRequest() // /api/auth/me refresh
        assertEquals("Alice", profilePut["name"]?.jsonPrimitive?.content)
        assertEquals("+60123456789", profilePut["phone"]?.jsonPrimitive?.content)
        assertEquals("alice_tg", profilePut["telegram_chat_id"]?.jsonPrimitive?.content)

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
}
