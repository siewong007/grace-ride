package org.pjstatecm.graceride

import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.hasAnyAncestor
import androidx.compose.ui.test.hasContentDescription
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.isDialog
import androidx.compose.ui.test.isToggleable
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onFirst
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.navigation.compose.rememberNavController
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import okhttp3.mockwebserver.Dispatcher
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import okhttp3.mockwebserver.RecordedRequest
import org.junit.After
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.screens.rides.RideDetailScreen
import org.pjstatecm.graceride.ui.screens.traffic.PlanTransportScreen
import org.pjstatecm.graceride.ui.screens.traffic.TrafficHubScreen
import org.pjstatecm.graceride.ui.theme.GraceRideTheme

@RunWith(AndroidJUnit4::class)
class TrafficAdminAndroidTest {

    @get:Rule
    val compose = createComposeRule()

    private lateinit var server: MockWebServer
    private lateinit var container: AppContainer

    @Before
    fun setup() {
        server = MockWebServer()
        server.start()
        val ctx = InstrumentationRegistry.getInstrumentation().targetContext
        container = AppContainer(ctx, InMemoryStore())
        container.setBaseUrl(server.url("/").toString())
    }

    @After
    fun teardown() {
        server.shutdown()
    }

    private fun res(id: Int, vararg args: Any): String =
        InstrumentationRegistry.getInstrumentation().targetContext.getString(id, *args)

    private fun waitForText(text: String, timeoutMs: Long = 10_000L) {
        compose.waitUntil(timeoutMs) {
            compose.onAllNodesWithText(text, substring = true).fetchSemanticsNodes().isNotEmpty()
        }
    }

    private fun signInTrafficLead(extraRoutes: Map<String, (RecordedRequest) -> MockResponse> = emptyMap()) {
        val leadMe = """
            {"id":9,"name":"Traffic Lead","email":"lead@example.org","status":"active",
             "roles":["traffic_head"],
             "permissions":["rides.view","events.view","admin_plan.view","admin_plan.update","admin_rides.view","admin_rides.update","admin_live.view","admin_area_photos.view","admin_fares.view","admin_areas.view","admin_dashboard.view"],
             "mfa_ok":true}
        """.trimIndent()
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val path = request.path?.substringBefore('?').orEmpty()
                extraRoutes[path]?.let { return it(request) }
                return when (path) {
                    "/api/auth/me" -> MockResponse().setBody(leadMe)
                    "/api/admin/dashboard" -> MockResponse().setBody("""{"pending_approval":1,"no_driver":1,"pending_plan_drafts":1,"pending_ride_drafts":1}""")
                    else -> MockResponse().setBody("{}")
                }
            }
        }
        runBlocking { container.auth.refresh() }
    }

    @Test
    fun trafficHubPlansAndProposedRideDraftsRenderAndApprove() {
        val approvedPaths = mutableListOf<String>()
        signInTrafficLead(
            mapOf(
                "/api/admin/traffic/plans" to {
                    MockResponse().setBody(
                        """[{"id":5,"title":"Sunday Service","starts_at":"2026-11-29 09:00:00","location":"Church Hall","ride_count":4,"unassigned_count":1,"pending_confirm_count":0,"visitor_count":1,"draft_count":1,"pending_draft_count":1,"returned_draft_count":0,"latest_draft":{"id":12,"event_id":5,"version":1,"label":"Main Plan","status":"pending_approval","assignments":[],"visitors":[]}}]""",
                    )
                },
                "/api/admin/rides" to {
                    MockResponse().setBody("""{"items":[],"total":0,"page":1,"page_size":50,"total_pages":1}""")
                },
                "/api/admin/ride-drafts" to {
                    MockResponse().setBody(
                        """[{"id":71,"ride_id":301,"version":1,"action":"reassign","driver_id":10,"driver_name":"Brother Lim","reason":"Same street","status":"pending_approval","ride":{"id":301,"status":"confirmed","pickup_at":"2026-11-29 08:30:00","passenger_name":"Sis Mary","area_name":"PJ"}}]""",
                    )
                },
                "/api/rides/301/drafts/71/approve" to {
                    approvedPaths += "/api/rides/301/drafts/71/approve"
                    MockResponse().setBody("""{"ok":true}""")
                },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                TrafficHubScreen(container = container, nav = rememberNavController())
            }
        }

        waitForText("Sunday Service")
        compose.onNodeWithText("Sunday Service").assertIsDisplayed()

        // Switch to Rides tab -> Proposed changes sub-view
        compose.onNodeWithText(res(R.string.traffic_tab_rides)).performClick()
        waitForText(res(R.string.traffic_rides_drafts_view))
        compose.onNodeWithText(res(R.string.traffic_rides_drafts_view)).performClick()
        waitForText("Sis Mary")
        compose.onNodeWithText(res(R.string.ride_draft_approve)).performClick()
        compose.waitUntil(5_000L) { approvedPaths.isNotEmpty() }
        assertTrue(approvedPaths.contains("/api/rides/301/drafts/71/approve"))
    }

    @Test
    fun planTransportShowsLockBannerTakeoverAndAutoArrangeDiagnosis() {
        var lockForceCalled = false
        val planBody = """
            {
              "event":{"id":5,"title":"Camp Retreat","starts_at":"2026-12-01 08:00:00","location":"Cameron","status":"published"},
              "draft":{"id":12,"event_id":5,"version":1,"label":"Team Draft","status":"draft","assignments":[{"ride_id":501,"driver_id":null}],"visitors":[],"locked_by":{"id":2,"name":"Alex"},"locked_at":"2026-11-26 10:00:00","lock_expires_at":"2026-11-26 10:10:00","lock_is_mine":false},
              "drafts":[{"id":12,"event_id":5,"version":1,"label":"Team Draft","status":"draft","assignments":[{"ride_id":501,"driver_id":null}],"visitors":[],"locked_by":{"id":2,"name":"Alex"},"locked_at":"2026-11-26 10:00:00","lock_expires_at":"2026-11-26 10:10:00","lock_is_mine":false}],
              "can_approve":true,
              "visitors":[{"id":88,"name":"Visitor Ken","phone":"012","notes":"","area":{"id":1,"name":"PJ"},"pickup_address":"Street 1","seats":1,"leg":"outbound","can_edit":true}],
              "rides":[{"id":501,"passenger":{"id":7,"name":"John"},"seats":1,"area":{"id":1,"name":"PJ"},"pickup_at":"2026-12-01 07:30:00","pickup_address":"SS2","assistance":"","notes":"","status":"pending_driver","driver_id":null,"leg":"outbound"}],
              "drivers":[{"id":10,"name":"Driver Ho","carpool":true,"cap":4,"area_ids":[1],"drive_group_ids":[]}]
            }
        """.trimIndent()
        val draftMine = """{"id":12,"event_id":5,"version":1,"label":"Team Draft","status":"draft","assignments":[{"ride_id":501,"driver_id":null}],"visitors":[],"locked_by":{"id":9,"name":"Traffic Lead"},"locked_at":"2026-11-26 10:01:00","lock_expires_at":"2026-11-26 10:11:00","lock_is_mine":true}"""

        signInTrafficLead(
            mapOf(
                "/api/areas" to { MockResponse().setBody("""[{"id":1,"name":"PJ","active":true}]""") },
                "/api/admin/events/5/plan" to { MockResponse().setBody(planBody) },
                "/api/admin/events/5/plan/drafts/12/lock" to { req ->
                    val body = req.body.readUtf8()
                    if (body.contains("\"force\":true")) lockForceCalled = true
                    MockResponse().setBody("""{"ok":true,"draft":$draftMine}""")
                },
                "/api/admin/events/5/plan/arrange" to {
                    MockResponse().setBody("""{"placed":[{"ride_id":501,"driver_id":10,"in_area":true,"together":false}],"unplaced":[],"approximate":false}""")
                },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                PlanTransportScreen(container = container, nav = rememberNavController(), eventId = 5L, initialDraftId = 12L)
            }
        }

        waitForText("Alex")
        compose.onNodeWithText(res(R.string.plan_lock_takeover)).assertIsDisplayed().performClick()
        waitForText(res(R.string.plan_lock_mine))
        assertTrue(lockForceCalled)

        // Trigger auto-arrange -> shows Diagnosis Bottom Sheet
        compose.onNodeWithText(res(R.string.plan_action_auto_arrange)).performClick()
        waitForText(res(R.string.plan_arrange_diag_title))
        compose.onNodeWithText(res(R.string.plan_arrange_diag_title)).assertIsDisplayed()
    }

    @Test
    fun rideDetailShowsStaffControlsAndProposedChangeDrafts() {
        val rideJson = """
            {"id":301,"status":"pending_approval","pickup_at":"2026-11-29 08:30:00",
             "area":{"id":1,"name":"PJ"},"pickup_address":"14 Jalan","destination":"Church",
             "seats":2,"passenger":{"id":4,"name":"Sis Mary"},"viewer":{"is_staff":true},
             "reimbursement_excluded":false}
        """.trimIndent()
        val driversJson = """[{"id":10,"name":"Driver Ho","carpool_enabled":true,"spare_seats":3,"areas":["PJ"]}]"""
        val draftsJson = """[{"id":71,"ride_id":301,"version":1,"action":"assign","driver_id":10,"driver_name":"Driver Ho","reason":"Close by","status":"draft","lock_is_mine":true}]"""

        signInTrafficLead(
            mapOf(
                "/api/rides/301" to { MockResponse().setBody(rideJson) },
                "/api/rides/301/messages" to { MockResponse().setBody("""{"items":[],"unread":0}""") },
                "/api/admin/drivers" to { MockResponse().setBody(driversJson) },
                "/api/rides/301/drafts" to { MockResponse().setBody(draftsJson) },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                RideDetailScreen(container = container, nav = rememberNavController(), id = 301L)
            }
        }

        waitForText(res(R.string.ride_staff_section))
        compose.onNodeWithText(res(R.string.ride_staff_section)).performScrollTo().assertIsDisplayed()
        compose.onNodeWithText(res(R.string.ride_staff_approve)).performScrollTo().assertIsDisplayed()
        compose.onNodeWithText(res(R.string.ride_draft_section)).performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Driver Ho", substring = true).performScrollTo().assertIsDisplayed()
    }

    @Test
    fun trafficHubTabTransitionsFiltersBatchBarLiveMapAndAreaSubTabs() {
        val pendingRidesJson = """
            {
              "items":[
                {"id":301,"status":"pending_approval","pickup_at":"2026-11-29 08:30:00","area":{"id":1,"name":"PJ"},"pickup_address":"14 Jalan","destination":"Church","seats":1,"passenger":{"id":4,"name":"Sis Mary"}},
                {"id":302,"status":"pending_approval","pickup_at":"2026-11-29 08:40:00","area":{"id":2,"name":"Subang"},"pickup_address":"SS15","destination":"Church","seats":2,"passenger":{"id":5,"name":"Bro Tan"}}
              ],
              "total":2,"page":1,"page_size":50,"total_pages":1
            }
        """.trimIndent()
        val liveDriversJson = """
            {"max":20,"drivers":[{"driver_id":10,"name":"Driver Lim","lat":3.112,"lng":101.635,"updated_at":"2026-11-26 14:00:00","rides":2,"first_pickup":"2026-11-29 08:30:00"}]}
        """.trimIndent()
        val areasJson = """[{"id":1,"name":"PJ","active":true,"photo_count":1},{"id":2,"name":"Subang","active":true,"photo_count":0}]"""
        val areaPhotosJson = """{"area":{"id":1,"name":"PJ"},"photos":[],"max":20,"can_add":true,"can_reorder":false}"""
        val faresJson = """{"fares":[{"id":1,"area_a":{"id":1,"name":"PJ"},"area_b":{"id":2,"name":"Subang"},"amount_cents":1500,"updated_at":"2026-11-26 10:00:00"}]}"""

        signInTrafficLead(
            mapOf(
                "/api/admin/traffic/plans" to {
                    MockResponse().setBody(
                        """[
                            {"id":5,"title":"Sunday Service","starts_at":"2026-11-29 09:00:00","location":"Church Hall","ride_count":4,"unassigned_count":1,"pending_confirm_count":0,"visitor_count":0,"draft_count":1,"pending_draft_count":1,"returned_draft_count":0,"latest_draft":{"id":12,"event_id":5,"version":1,"label":"Main Plan","status":"pending_approval","assignments":[],"visitors":[]}},
                            {"id":6,"title":"Prayer Night","starts_at":"2026-12-03 20:00:00","location":"Main Hall","ride_count":2,"unassigned_count":0,"pending_confirm_count":0,"visitor_count":0,"draft_count":0,"pending_draft_count":0,"returned_draft_count":0}
                        ]""",
                    )
                },
                "/api/admin/rides" to { MockResponse().setBody(pendingRidesJson) },
                "/api/admin/ride-drafts" to { MockResponse().setBody("[]") },
                "/api/admin/live" to { MockResponse().setBody(liveDriversJson) },
                "/api/areas" to { MockResponse().setBody(areasJson) },
                "/api/areas/1/photos" to { MockResponse().setBody(areaPhotosJson) },
                "/api/admin/settings/fares" to { MockResponse().setBody(faresJson) },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                TrafficHubScreen(container = container, nav = rememberNavController())
            }
        }

        // Plans tab: Hero + remaining card render cleanly
        waitForText("Sunday Service")
        compose.onNodeWithText("Sunday Service").assertIsDisplayed()
        compose.onNodeWithText("Prayer Night").assertIsDisplayed()

        // Transition to Rides tab -> Filters bottom sheet -> Batch approve dock
        compose.onNodeWithText(res(R.string.traffic_tab_rides)).performClick()
        waitForText("Sis Mary")
        compose.onNodeWithText(res(R.string.traffic_rides_filters)).performClick()
        waitForText(res(R.string.traffic_rides_attention_only))
        compose.onNode(hasAnyAncestor(isDialog()) and hasText(res(R.string.common_confirm))).performClick()

        val selectAllLabel = res(R.string.traffic_rides_select_all_pending, 2)
        waitForText(selectAllLabel)
        compose.onNodeWithText(selectAllLabel).performClick()
        val batchApproveLabel = res(R.string.traffic_rides_batch_approve, 2)
        waitForText(batchApproveLabel)
        compose.onNodeWithText(batchApproveLabel).assertIsDisplayed()
        compose.onNodeWithText(res(R.string.traffic_rides_batch_clear)).performClick()
        compose.waitUntil(5_000L) {
            compose.onAllNodesWithText(batchApproveLabel).fetchSemanticsNodes().isEmpty()
        }

        // Transition between Rides and Proposed changes sub-view
        compose.onNodeWithText(res(R.string.traffic_rides_drafts_view)).performClick()
        waitForText(res(R.string.traffic_ride_drafts_empty))
        compose.onNodeWithText(res(R.string.traffic_rides_list_view)).performClick()
        waitForText("Sis Mary")

        // Transition to Live tab & exercise map zoom controls + Fit all
        compose.onNodeWithText(res(R.string.traffic_tab_live)).performClick()
        waitForText("Driver Lim")
        compose.onNodeWithContentDescription(res(R.string.traffic_live_zoom_in)).performClick()
        waitForText(res(R.string.traffic_live_fit_all))
        compose.onNodeWithText(res(R.string.traffic_live_fit_all)).performClick()

        // Transition to Areas tab & switch between Photos -> Fares -> Areas sub-tabs
        compose.onNodeWithText(res(R.string.traffic_tab_areas)).performClick()
        waitForText(res(R.string.traffic_area_photos_empty))
        compose.onNodeWithText(res(R.string.traffic_areas_sub_fares)).performClick()
        waitForText("PJ ↔ Subang")
        compose.onNode(
            SemanticsMatcher.expectValue(SemanticsProperties.Role, Role.RadioButton) and
                hasText(res(R.string.traffic_areas_sub_manage)),
        ).performClick()
        waitForText(res(R.string.traffic_area_add))
        compose.onNodeWithText("Subang").assertIsDisplayed()
    }

    @Test
    fun planTransportTransitionsUnsavedDockVisitorAndReimbursementSheets() {
        val planBody = """
            {
              "event":{"id":5,"title":"Camp Retreat","starts_at":"2026-12-01 08:00:00","location":"Cameron","status":"published"},
              "draft":{"id":12,"event_id":5,"version":1,"label":"Team Draft","status":"draft","assignments":[{"ride_id":501,"driver_id":null}],"visitors":[],"locked_by":{"id":9,"name":"Traffic Lead"},"locked_at":"2026-11-26 10:00:00","lock_expires_at":"2026-11-26 10:30:00","lock_is_mine":true},
              "drafts":[{"id":12,"event_id":5,"version":1,"label":"Team Draft","status":"draft","assignments":[{"ride_id":501,"driver_id":null}],"visitors":[],"locked_by":{"id":9,"name":"Traffic Lead"},"locked_at":"2026-11-26 10:00:00","lock_expires_at":"2026-11-26 10:30:00","lock_is_mine":true}],
              "can_approve":true,
              "visitors":[],
              "rides":[{"id":501,"passenger":{"id":7,"name":"John"},"seats":1,"area":{"id":1,"name":"PJ"},"pickup_at":"2026-12-01 07:30:00","pickup_address":"SS2","assistance":"","notes":"","status":"pending_driver","driver_id":null,"leg":"outbound"}],
              "drivers":[{"id":10,"name":"Driver Ho","carpool":true,"cap":4,"area_ids":[1],"drive_group_ids":[]}]
            }
        """.trimIndent()
        val postEventBody = """
            {
              "event_id":5,
              "event_title":"Camp Retreat",
              "event_area":{"id":1,"name":"PJ","fare_cents":1500},
              "available_areas":[{"id":1,"name":"PJ","fare_cents":1500},{"id":2,"name":"Subang","fare_cents":2000}],
              "drivers":[
                {
                  "id":10,
                  "name":"Driver Ho",
                  "vehicle":"Myvi",
                  "default_area":{"id":1,"name":"PJ","fare_cents":1500},
                  "passenger_stats":{"total":2,"supper":1,"direct_return":1,"direct_home":0,"unspecified":0},
                  "suggested_trip_type":"supper",
                  "already_claimed":false,
                  "reimbursement_excluded":false,
                  "excluded_ride_count":0,
                  "total_ride_count":2
                }
              ]
            }
        """.trimIndent()

        signInTrafficLead(
            mapOf(
                "/api/areas" to { MockResponse().setBody("""[{"id":1,"name":"PJ","active":true}]""") },
                "/api/admin/events/5/plan" to { MockResponse().setBody(planBody) },
                "/api/admin/events/5/plan/arrange" to {
                    MockResponse().setBody("""{"placed":[{"ride_id":501,"driver_id":10,"in_area":true,"together":false}],"unplaced":[],"approximate":false}""")
                },
                "/api/events/5/post-event-drivers" to { MockResponse().setBody(postEventBody) },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                PlanTransportScreen(container = container, nav = rememberNavController(), eventId = 5L, initialDraftId = 12L)
            }
        }

        waitForText("John")

        // Auto-arrange -> Diagnosis Sheet -> dismiss -> verify Unsaved badge & Undo in sticky dock
        compose.onNodeWithText(res(R.string.plan_action_auto_arrange)).performClick()
        waitForText(res(R.string.plan_arrange_diag_title))
        compose.onNodeWithContentDescription(res(R.string.common_close)).performClick()
        waitForText(res(R.string.plan_unsaved_badge))
        compose.onNodeWithText(res(R.string.plan_unsaved_badge)).assertIsDisplayed()
        compose.onNodeWithText(res(R.string.plan_action_undo), substring = true).assertIsDisplayed().performClick()
        compose.waitUntil(5_000L) {
            compose.onAllNodesWithText(res(R.string.plan_unsaved_badge)).fetchSemanticsNodes().isEmpty()
        }

        // Visitor editor bottom sheet
        compose.onNodeWithText(res(R.string.plan_visitor_add)).performScrollTo().performClick()
        waitForText(res(R.string.plan_visitor_name_hint))
        compose.onNode(hasAnyAncestor(isDialog()) and hasText(res(R.string.common_cancel))).performClick()
        compose.waitUntil(5_000L) {
            compose.onAllNodesWithText(res(R.string.plan_visitor_name_hint)).fetchSemanticsNodes().isEmpty()
        }

        // Post-event reimbursement sheet with collapsible AnimatedVisibility card & RM total CTA
        compose.onNodeWithContentDescription(res(R.string.plan_action_post_event)).performClick()
        waitForText(res(R.string.plan_post_event_title))
        waitForText(res(R.string.plan_post_event_trip_supper))
        // Collapse card by unchecking checkbox, then re-expand
        compose.onAllNodes(isToggleable()).onFirst().performClick()
        compose.waitUntil(5_000L) {
            compose.onAllNodesWithText(res(R.string.plan_post_event_trip_supper)).fetchSemanticsNodes().isEmpty()
        }
        compose.onAllNodes(isToggleable()).onFirst().performClick()
        waitForText(res(R.string.plan_post_event_trip_supper))
        compose.onNodeWithText("RM 15.00", substring = true).assertIsDisplayed()
    }

    @Test
    fun rideDraftEditorInlineDriverPickerTransitionWorksWithoutNestedModal() {
        val rideJson = """
            {"id":301,"status":"pending_approval","pickup_at":"2026-11-29 08:30:00",
             "area":{"id":1,"name":"PJ"},"pickup_address":"14 Jalan","destination":"Church",
             "seats":2,"passenger":{"id":4,"name":"Sis Mary"},"viewer":{"is_staff":true},
             "reimbursement_excluded":false}
        """.trimIndent()
        val driversJson = """[{"id":10,"name":"Driver Ho","carpool_enabled":true,"spare_seats":3,"areas":["PJ"]}]"""

        signInTrafficLead(
            mapOf(
                "/api/rides/301" to { MockResponse().setBody(rideJson) },
                "/api/rides/301/messages" to { MockResponse().setBody("""{"items":[],"unread":0}""") },
                "/api/admin/drivers" to { MockResponse().setBody(driversJson) },
                "/api/rides/301/drafts" to { MockResponse().setBody("[]") },
            ),
        )

        compose.setContent {
            GraceRideTheme {
                RideDetailScreen(container = container, nav = rememberNavController(), id = 301L)
            }
        }

        waitForText(res(R.string.ride_draft_new))
        compose.onNodeWithText(res(R.string.ride_draft_new)).performScrollTo().performClick()
        waitForText(res(R.string.ride_draft_save))

        // Select Assign driver action and transition inline into driver picker step inside same ModalBottomSheet
        compose.onNodeWithText(res(R.string.ride_draft_action_assign)).performClick()
        waitForText(res(R.string.ride_staff_reassign))
        compose.onNode(hasAnyAncestor(isDialog()) and hasText(res(R.string.ride_staff_reassign))).performClick()

        // Inline picker shows back button + searchable driver card
        waitForText(res(R.string.plan_driver_search_hint))
        compose.onNode(hasAnyAncestor(isDialog()) and hasContentDescription(res(R.string.a11y_back))).assertIsDisplayed()
        compose.onNodeWithText("Driver Ho").performClick()

        // Returned to form step with Driver Ho selected and Save/Submit buttons enabled
        waitForText(res(R.string.ride_draft_submit))
        compose.onNode(hasAnyAncestor(isDialog()) and hasText("Driver Ho", substring = true)).assertIsDisplayed()
        compose.onNodeWithText(res(R.string.ride_draft_submit)).assertIsEnabled()
    }
}
