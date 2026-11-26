package org.pjstatecm.graceride

import app.cash.turbine.test
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import okhttp3.mockwebserver.Dispatcher as MockDispatcher
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
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Area
import org.pjstatecm.graceride.data.api.AuthEvent
import org.pjstatecm.graceride.data.api.DraftVisitorEntryDto
import org.pjstatecm.graceride.data.api.PlanDraftEntryDto
import org.pjstatecm.graceride.data.api.PlanDriverDto
import org.pjstatecm.graceride.data.api.PlanLastMatchDto
import org.pjstatecm.graceride.data.api.PlanRideDto
import org.pjstatecm.graceride.data.api.PlanVehicleDto
import org.pjstatecm.graceride.data.api.PlanVisitorDto
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.RideArea
import org.pjstatecm.graceride.data.api.RideDraftInReq
import org.pjstatecm.graceride.data.api.RideDriver
import org.pjstatecm.graceride.data.api.RidePassenger
import org.pjstatecm.graceride.data.api.UserRefDto
import org.pjstatecm.graceride.data.api.apiCall
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.PlanLogic
import org.pjstatecm.graceride.ui.screens.rides.RideDetailViewModel
import org.pjstatecm.graceride.ui.screens.rides.legalRideDraftActions
import org.pjstatecm.graceride.ui.screens.traffic.PlanNotice
import org.pjstatecm.graceride.ui.screens.traffic.PlanTransportViewModel
import org.pjstatecm.graceride.ui.screens.traffic.TrafficHubViewModel
import kotlin.time.Duration.Companion.seconds

@OptIn(ExperimentalCoroutinesApi::class)
class TrafficAdminAndDraftsTest {

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

    private fun json(body: String, code: Int = 200) = MockResponse().setResponseCode(code).setBody(body)

    private suspend fun <T> StateFlow<T>.awaitUntil(predicate: (T) -> Boolean): T =
        kotlinx.coroutines.withTimeout(10.seconds) {
            var result: T? = null
            test {
                while (true) {
                    val item = awaitItem()
                    if (predicate(item)) {
                        result = item
                        break
                    }
                }
                cancelAndIgnoreRemainingEvents()
            }
            @Suppress("UNCHECKED_CAST")
            result as T
        }

    // ---------------- Pure PlanLogic Tests ----------------

    @Test
    fun planLogicChecksConstraintsLastTimeReturnAndVisitors() {
        val drivers = listOf(
            PlanDriverDto(
                id = 10L,
                name = "Driver A",
                carpool = true,
                cap = 4,
                areaIds = listOf(1L),
                driveGroupIds = listOf(100L),
                vehicle = PlanVehicleDto(model = "Vios", seats = 4, wheelchairOk = true),
            ),
            PlanDriverDto(
                id = 20L,
                name = "Driver B",
                carpool = false,
                cap = 2,
                areaIds = listOf(2L),
                driveGroupIds = listOf(200L),
                vehicle = PlanVehicleDto(model = "Myvi", seats = 3, wheelchairOk = false),
            ),
        )
        val driversById = drivers.associateBy { it.id }

        val rideOut = PlanRideDto(
            id = 1L,
            passenger = UserRefDto(50L, "Alice"),
            seats = 2,
            area = RideArea(1L, "PJ"),
            assistance = "wheelchair",
            driverId = null,
            leg = "outbound",
            matchGroupId = 100L,
            last = PlanLastMatchDto(driverId = 10L, driverName = "Driver A", times = 3),
        )
        val rideRet = PlanRideDto(
            id = 2L,
            passenger = UserRefDto(50L, "Alice"),
            seats = 2,
            area = RideArea(1L, "PJ"),
            assistance = "wheelchair",
            driverId = null,
            leg = "return",
            pairedRideId = 1L,
            matchGroupId = 100L,
        )
        val rides = listOf(rideOut, rideRet)

        assertTrue(PlanLogic.inGroup(rideOut, drivers[0]))
        assertFalse(PlanLogic.inGroup(rideOut, drivers[1]))
        assertTrue(PlanLogic.vehicleCompatible(rideOut, drivers[0]))
        assertFalse(PlanLogic.vehicleCompatible(rideOut, drivers[1]))
        assertTrue(PlanLogic.servesArea(rideOut, drivers[0]))
        assertFalse(PlanLogic.servesArea(rideOut, drivers[1]))
        assertTrue(PlanLogic.fits(drivers[0], rideOut, rides, emptyMap(), driversById))

        // applyLastTime places outbound ride onto Driver A
        val (afterLast, placedCount) = PlanLogic.applyLastTime(rides, mapOf(1L to null, 2L to null), driversById)
        assertEquals(1, placedCount)
        assertEquals(10L, afterLast[1L])
        assertNull(afterLast[2L])

        // applyMatchReturnDrivers mirrors outbound driver onto paired return ride
        val (afterReturn, matchedCount) = PlanLogic.applyMatchReturnDrivers(rides, afterLast, driversById)
        assertEquals(1, matchedCount)
        assertEquals(10L, afterReturn[2L])

        val changes = PlanLogic.changesOf(rides, afterReturn)
        assertEquals(2, changes.size)

        // Visitors merge + draft diff summary
        val liveVisitors = listOf(
            PlanVisitorDto(id = 7L, name = "Guest One", seats = 1, area = RideArea(1L, "PJ")),
            PlanVisitorDto(id = 8L, name = "Guest Two", seats = 2, area = RideArea(1L, "PJ")),
        )
        val stagedVisitors = listOf(
            DraftVisitorEntryDto(tempId = "u7", op = "update", liveVisitorId = 7L, name = "Guest One Edited", seats = 2),
            DraftVisitorEntryDto(tempId = "d8", op = "delete", liveVisitorId = 8L, name = "Guest Two"),
            DraftVisitorEntryDto(tempId = "a1", op = "add", name = "Guest Three", seats = 1),
        )
        val merged = PlanLogic.mergeEffectiveVisitors(liveVisitors, stagedVisitors)
        assertEquals(3, merged.size)
        assertEquals("Guest One Edited", merged[0].name)
        assertEquals("update", merged[0].draftOp)
        assertEquals("delete", merged[1].draftOp)
        assertEquals("Guest Three", merged[2].name)
        assertEquals("add", merged[2].draftOp)

        val summary = PlanLogic.summarizeDraftDiff(
            rides = rides,
            draftAssignments = listOf(PlanDraftEntryDto(1L, 10L), PlanDraftEntryDto(2L, null)),
            draftVisitors = stagedVisitors,
        )
        assertEquals(1, summary.assignmentChanges)
        assertEquals(1, summary.visitorAdds)
        assertEquals(1, summary.visitorUpdates)
        assertEquals(1, summary.visitorDeletes)
    }

    // ---------------- ApiClient 423 Draft Lock Guard ----------------

    @Test
    fun draftLocked423DoesNotEmitSessionLockedEvent() = runBlocking {
        server.enqueue(json("""{"error":"Bob is editing this draft","code":"DRAFT_LOCKED_BY_OTHER"}""", 423))
        container.authEvents.events.test {
            try {
                apiCall(container.authEvents) { container.api.me() }
                throw AssertionError("Expected ApiException")
            } catch (e: ApiException) {
                assertEquals(423, e.status)
                assertEquals("DRAFT_LOCKED_BY_OTHER", e.code)
            }
            expectNoEvents()
        }
    }

    // ---------------- PlanTransportViewModel Draft + Lease Lock + Visitor Sync Tests ----------------

    @Test
    fun planTransportAcquiresLockStagesVisitorsAndSyncsBeforeApprove() = runBlocking {
        val requests = mutableListOf<String>()
        val draftLockedJson = """
            {"id":5,"event_id":9,"version":1,"label":"Plan A","status":"draft",
             "assignments":[{"ride_id":101,"driver_id":null}],
             "visitors":[],
             "locked_by":{"id":20,"name":"Traffic Lead"},
             "locked_at":"2026-11-26 10:00:00",
             "lock_expires_at":"2026-11-26 10:10:00",
             "lock_is_mine":true}
        """.trimIndent()
        val planJson = """
            {
              "event":{"id":9,"title":"Sunday Service","starts_at":"2026-11-29 09:00:00","location":"Sanctuary","status":"published"},
              "draft":{"id":5,"event_id":9,"version":1,"label":"Plan A","status":"draft","assignments":[{"ride_id":101,"driver_id":null}],"visitors":[],"lock_is_mine":false},
              "drafts":[{"id":5,"event_id":9,"version":1,"label":"Plan A","status":"draft","assignments":[{"ride_id":101,"driver_id":null}],"visitors":[],"lock_is_mine":false}],
              "can_approve":true,
              "visitors":[],
              "rides":[{
                "id":101,"passenger":{"id":50,"name":"Grace Member"},"seats":1,
                "area":{"id":1,"name":"PJ"},"pickup_at":"2026-11-29 08:15:00","pickup_address":"12 Jalan",
                "assistance":"","notes":"","status":"pending_driver","driver_id":null,"leg":"outbound"
              }],
              "drivers":[{
                "id":30,"name":"Driver Tan","carpool":true,"cap":4,"area_ids":[1],"drive_group_ids":[],
                "vehicle":{"model":"City","plate":"WAA1234","seats":4,"wheelchair_ok":false,"low_step":false,"child_seat":false}
              }]
            }
        """.trimIndent()

        server.dispatcher = object : MockDispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val path = request.path.orEmpty()
                requests += "${request.method} $path"
                return when {
                    path.endsWith("/api/admin/events/9/plan") && request.method == "GET" -> json(planJson)
                    path == "/api/areas" -> json("""[{"id":1,"name":"PJ","active":true,"photo_count":0}]""")
                    path == "/api/admin/events/9/plan/drafts/5/lock" && request.method == "POST" ->
                        json("""{"ok":true,"draft":$draftLockedJson}""")
                    path == "/api/admin/events/9/plan/drafts/5" && request.method == "PUT" ->
                        json("""{"ok":true,"draft":$draftLockedJson}""")
                    path == "/api/admin/events/9/plan/approve" && request.method == "POST" ->
                        json("""{"ok":true}""")
                    else -> json("""{"ok":true}""")
                }
            }
        }

        val vm = PlanTransportViewModel(container, eventId = 9L, initialDraftId = 5L)
        val loaded = vm.state.awaitUntil { !it.loading && it.plan != null }
        assertEquals(5L, loaded.activeDraftId)

        // Stage a visitor -> automatically acquires lease lock and marks visitorsDirty
        vm.stageAddVisitor(
            name = "Visitor New",
            phone = "0123456789",
            notes = "",
            area = Area(1L, "PJ"),
            pickupAddress = "Gate A",
            seats = 1L,
            leg = "outbound",
            driver = loaded.plan!!.drivers.first(),
        )
        val afterStage = vm.state.awaitUntil { it.visitorsDirty && it.draftVisitors.size == 1 }
        assertTrue(afterStage.activeDraft?.lockIsMine == true)
        assertTrue(requests.any { it.contains("POST /api/admin/events/9/plan/drafts/5/lock") })

        // Assign ride 101 -> Driver Tan (30) and approve & notify -> PUT drafts/5 MUST precede POST plan/approve
        vm.assignRide(101L, 30L)
        vm.state.awaitUntil { it.assign[101L] == 30L }
        requests.clear()

        vm.approveAndNotify()
        val approved = vm.state.awaitUntil { it.notice == PlanNotice.APPROVED || (!it.busy && !it.visitorsDirty) }
        assertFalse(approved.visitorsDirty)
        val putIdx = requests.indexOfFirst { it.startsWith("PUT /api/admin/events/9/plan/drafts/5") }
        val appIdx = requests.indexOfFirst { it.startsWith("POST /api/admin/events/9/plan/approve") }
        assertTrue("Expected PUT draft before POST approve: $requests", putIdx >= 0 && appIdx > putIdx)
    }

    // ---------------- TrafficHubViewModel + RideDetailViewModel Draft Tests ----------------

    @Test
    fun trafficHubBatchApproveAndRideChangeDraftLifecycleWork() = runBlocking {
        server.enqueue(json("""{"id":9,"name":"Traffic Lead","email":"lead@example.org","status":"active","roles":["traffic_head"],"permissions":["admin_plan.view","admin_plan.update","admin_rides.view","admin_rides.update","admin_dashboard.view"],"mfa_ok":true}"""))
        container.auth.refresh()

        val rideDraftJson = """
            {"id":44,"ride_id":77,"version":1,"action":"reassign","driver_id":30,"driver_name":"Driver Tan",
             "reason":"Closer to area","status":"pending_approval","lock_is_mine":false,
             "ride":{"id":77,"status":"confirmed","pickup_at":"2026-11-29 09:00:00","passenger_name":"Lee","area_name":"PJ"}}
        """.trimIndent()

        val calls = mutableListOf<String>()
        server.dispatcher = object : MockDispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val path = request.path.orEmpty()
                calls += "${request.method} $path"
                return when {
                    path.startsWith("/api/admin/dashboard") ->
                        json("""{"pending_approval":2,"no_driver":1,"pending_plan_drafts":1,"pending_ride_drafts":1}""")
                    path.startsWith("/api/admin/rides") ->
                        json("""{"items":[{"id":77,"status":"pending_approval","pickup_at":"2026-11-29 09:00:00","area":{"id":1,"name":"PJ"},"destination":"Church","passenger":{"id":1,"name":"Lee"},"viewer":{"is_staff":true}}],"total":1,"page":1,"page_size":50,"total_pages":1}""")
                    path == "/api/rides/77/approve" && request.method == "POST" -> json("""{"ok":true}""")
                    path.startsWith("/api/admin/ride-drafts") -> json("""[$rideDraftJson]""")
                    path == "/api/rides/77" ->
                        json("""{"id":77,"status":"pending_approval","pickup_at":"2026-11-29 09:00:00","area":{"id":1,"name":"PJ"},"destination":"Church","passenger":{"id":1,"name":"Lee"},"viewer":{"is_staff":true}}""")
                    path == "/api/rides/77/drafts" && request.method == "GET" -> json("""[$rideDraftJson]""")
                    path == "/api/rides/77/drafts" && request.method == "POST" -> json("""{"ok":true,"draft":$rideDraftJson}""")
                    path == "/api/rides/77/drafts/44/approve" && request.method == "POST" -> json("""{"ok":true,"draft":$rideDraftJson}""")
                    path == "/api/admin/drivers" -> json("""[{"id":30,"name":"Driver Tan","carpool_enabled":true,"spare_seats":3,"areas":["PJ"]}]""")
                    else -> json("""{}""")
                }
            }
        }

        val hubVm = TrafficHubViewModel(
            container = container,
            initialTab = "rides",
            initialRidesView = "rides",
            initialAreaTab = "photos",
            initialRidesStatus = "pending_approval",
            initialRidesAttention = false,
        )
        hubVm.state.awaitUntil { !it.ridesLoading && it.ridesPage.items.isNotEmpty() }
        hubVm.toggleSelectRide(77L)
        hubVm.batchApproveSelected()
        hubVm.state.awaitUntil { !it.batchBusy && it.batchApprovedCount == 1 }
        assertTrue(calls.any { it == "POST /api/rides/77/approve" })

        val rideVm = RideDetailViewModel(container, 77L)
        val rState = rideVm.state.awaitUntil { !it.loading && it.ride != null && it.drafts.isNotEmpty() }
        assertEquals(1, rState.drafts.size)
        rideVm.createRideDraft(RideDraftInReq(action = "reassign", driver_id = 30L, reason = "Better match", submit = true))
        rideVm.state.awaitUntil { !it.draftBusy && calls.contains("POST /api/rides/77/drafts") }
        rideVm.approveRideDraft(44L)
        rideVm.state.awaitUntil { !it.draftBusy && calls.contains("POST /api/rides/77/drafts/44/approve") }
        assertTrue(calls.contains("POST /api/rides/77/drafts"))
        assertTrue(calls.contains("POST /api/rides/77/drafts/44/approve"))
    }

    @Test
    fun planTransportTracksUnsavedChangesAcrossAssignUndoResetAndSave() = runBlocking {
        val draftJson = """
            {"id":5,"event_id":9,"version":1,"label":"Plan A","status":"draft",
             "assignments":[{"ride_id":101,"driver_id":null}],
             "visitors":[],
             "lock_is_mine":true}
        """.trimIndent()
        val planJson = """
            {
              "event":{"id":9,"title":"Sunday Service","starts_at":"2026-11-29 09:00:00","location":"Sanctuary","status":"published"},
              "draft":$draftJson,
              "drafts":[$draftJson],
              "can_approve":true,
              "visitors":[],
              "rides":[{
                "id":101,"passenger":{"id":50,"name":"Grace Member"},"seats":1,
                "area":{"id":1,"name":"PJ"},"pickup_at":"2026-11-29 08:15:00","pickup_address":"12 Jalan",
                "assistance":"","notes":"","status":"pending_driver","driver_id":null,"leg":"outbound"
              }],
              "drivers":[{"id":30,"name":"Driver Tan","carpool":true,"cap":4,"area_ids":[1],"drive_group_ids":[]}]
            }
        """.trimIndent()

        var putCalled = false
        server.dispatcher = object : MockDispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val path = request.path.orEmpty()
                return when {
                    path.endsWith("/api/admin/events/9/plan") && request.method == "GET" -> json(planJson)
                    path == "/api/areas" -> json("""[{"id":1,"name":"PJ","active":true,"photo_count":0}]""")
                    path == "/api/admin/events/9/plan/drafts/5" && request.method == "PUT" -> {
                        putCalled = true
                        json("""{"ok":true,"draft":$draftJson}""")
                    }
                    else -> json("""{"ok":true}""")
                }
            }
        }

        val vm = PlanTransportViewModel(container, eventId = 9L, initialDraftId = 5L)
        val initial = vm.state.awaitUntil { !it.loading && it.plan != null }
        assertFalse(initial.hasUnsavedChanges)

        vm.assignRide(101L, 30L)
        val modified = vm.state.awaitUntil { it.assign[101L] == 30L }
        assertTrue(modified.hasUnsavedChanges)

        vm.undo()
        val undone = vm.state.awaitUntil { it.assign[101L] == null }
        assertFalse(undone.hasUnsavedChanges)

        vm.assignRide(101L, 30L)
        vm.state.awaitUntil { it.hasUnsavedChanges }

        var callbackRan = false
        vm.saveCurrentBoard { callbackRan = true }
        val saved = vm.state.awaitUntil { putCalled && callbackRan && !it.busy }
        assertTrue(putCalled)
        assertTrue(callbackRan)
        assertFalse(saved.hasUnsavedChanges)
    }

    @Test
    fun trafficHubSelectAllPartialBatchAndBannerJumpWork() = runBlocking {
        server.enqueue(json("""{"id":9,"name":"Traffic Lead","email":"lead@example.org","status":"active","roles":["traffic_head"],"permissions":["admin_plan.view","admin_plan.update","admin_rides.view","admin_rides.update","admin_dashboard.view","admin_areas.view"],"mfa_ok":true}"""))
        container.auth.refresh()

        server.dispatcher = object : MockDispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val path = request.path.orEmpty()
                return when {
                    path.startsWith("/api/admin/dashboard") ->
                        json("""{"pending_approval":2,"no_driver":0,"pending_plan_drafts":1,"pending_ride_drafts":1,"my_returned_plan_drafts":0,"my_returned_ride_drafts":1}""")
                    path.startsWith("/api/admin/traffic/plans") ->
                        json("""[{"id":5,"title":"Sunday Service","starts_at":"2026-11-29 09:00:00","ride_count":2,"unassigned_count":1,"pending_draft_count":1}]""")
                    path.startsWith("/api/admin/rides") ->
                        json("""{"items":[{"id":77,"status":"pending_approval","pickup_at":"2026-11-29 09:00:00","area":{"id":1,"name":"PJ"},"destination":"Church","passenger":{"id":1,"name":"Lee"},"viewer":{"is_staff":true}},{"id":78,"status":"pending_approval","pickup_at":"2026-11-29 09:10:00","area":{"id":1,"name":"PJ"},"destination":"Church","passenger":{"id":2,"name":"Tan"},"viewer":{"is_staff":true}}],"total":2,"page":1,"page_size":50,"total_pages":1}""")
                    path == "/api/rides/77/approve" -> json("""{"ok":true}""")
                    path == "/api/rides/78/approve" -> json("""{"error":"Conflict"}""", 409)
                    path.startsWith("/api/admin/ride-drafts") -> json("""[]""")
                    else -> json("""{}""")
                }
            }
        }

        val vm = TrafficHubViewModel(
            container = container,
            initialTab = "rides",
            initialRidesView = "rides",
            initialAreaTab = "photos",
            initialRidesStatus = null,
            initialRidesAttention = false,
            initialPlanFilter = null,
        )
        vm.state.awaitUntil { !it.ridesLoading && it.ridesPage.items.size == 2 }
        vm.selectAllPendingRides()
        val selected = vm.state.awaitUntil { it.selectedRideIds.size == 2 }
        assertEquals(setOf(77L, 78L), selected.selectedRideIds)

        vm.batchApproveSelected()
        val partial = vm.state.awaitUntil { !it.batchBusy && it.batchApprovedCount == 1 }
        assertEquals(1, partial.batchApprovedCount)
        assertEquals(1, partial.batchFailedCount)

        vm.jumpToPendingReview(isLead = true)
        val jumpedPlans = vm.state.awaitUntil { it.tab == "plans" && it.planFilter == "pending" }
        assertEquals("plans", jumpedPlans.tab)
        assertEquals("pending", jumpedPlans.planFilter)

        vm.jumpToReturnedDrafts()
        val jumpedDrafts = vm.state.awaitUntil { it.tab == "rides" && it.ridesView == "drafts" && it.rideDraftsFilter == "returned" }
        assertEquals("returned", jumpedDrafts.rideDraftsFilter)
    }

    @Test
    fun legalRideDraftActionsFiltersByRideStatusAndDriver() {
        val pendingApproval = Ride(
            id = 1L,
            status = "pending_approval",
            pickupAt = "2026-11-29 08:30:00",
            area = RideArea(1L, "PJ"),
            passenger = RidePassenger(id = 1L, name = "A"),
            driver = null,
        )
        val pendingActions = legalRideDraftActions(pendingApproval)
        assertTrue("approve" in pendingActions)
        assertTrue("assign" in pendingActions)
        assertFalse("reassign" in pendingActions)
        assertFalse("return" in pendingActions)

        val confirmedWithDriver = pendingApproval.copy(
            status = "confirmed",
            driver = RideDriver(id = 9L, name = "Driver X"),
        )
        val confirmedActions = legalRideDraftActions(confirmedWithDriver)
        assertFalse("approve" in confirmedActions)
        assertFalse("assign" in confirmedActions)
        assertTrue("reassign" in confirmedActions)
        assertTrue("return" in confirmedActions)

        val completedRide = confirmedWithDriver.copy(status = "completed")
        val completedActions = legalRideDraftActions(completedRide)
        assertEquals(listOf("reimbursement_excluded"), completedActions)
        assertTrue("cancel" in legalRideDraftActions(completedRide, includeAction = "cancel"))
    }
}
