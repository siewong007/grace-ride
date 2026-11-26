package org.pjstatecm.graceride

import app.cash.turbine.test
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.StateFlow
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
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.pjstatecm.graceride.data.api.AdminDashboardStatsDto
import org.pjstatecm.graceride.data.api.AdminRoleDefDto
import org.pjstatecm.graceride.data.api.AdminUserDto
import org.pjstatecm.graceride.data.api.Announcement
import org.pjstatecm.graceride.data.api.GroupDto
import org.pjstatecm.graceride.data.api.GroupRef
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.GroupTree
import org.pjstatecm.graceride.domain.MemberAdminRules
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.NotifDestination
import org.pjstatecm.graceride.ui.NotifLinkRouter
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.screens.admin.AdminMembersNotice
import org.pjstatecm.graceride.ui.screens.admin.AdminMembersViewModel
import org.pjstatecm.graceride.ui.screens.announcements.AnnouncementNotice
import org.pjstatecm.graceride.ui.screens.announcements.AnnouncementsViewModel
import java.util.concurrent.CopyOnWriteArrayList
import kotlin.time.Duration.Companion.seconds

@OptIn(ExperimentalCoroutinesApi::class)
class AdminMembersAndAnnouncementsTest {

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
        try {
            kotlinx.coroutines.withTimeout(10.seconds) {
                var result: T? = null
                test(timeout = 8.seconds) {
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
        } catch (t: Throwable) {
            throw AssertionError("Timed out waiting for state; last=$value", t)
        }

    // ---------------- Pure Domain Tests ----------------

    @Test
    fun groupTreeOrdersHierarchyFiltersVisitorAndHandlesCycles() {
        val groups = listOf(
            GroupDto(id = 1L, name = "All Members", isAll = true),
            GroupDto(id = 2L, name = "Youth", parentId = null),
            GroupDto(id = 3L, name = "Youth Choir", parentId = 2L),
            GroupDto(id = 4L, name = "Visitors", isVisitor = true),
            GroupDto(id = 5L, name = "Orphan Sub", parentId = 999L),
            GroupDto(id = 6L, name = "Cycle A", parentId = 7L),
            GroupDto(id = 7L, name = "Cycle B", parentId = 6L),
        )

        val ordinary = GroupTree.ordinary(groups)
        assertFalse(ordinary.any { it.isVisitor })
        assertEquals(6, ordinary.size)

        val assignable = GroupTree.assignableForMembers(groups)
        assertFalse(assignable.any { it.isAll || it.isVisitor })
        assertEquals(5, assignable.size)

        val ordered = GroupTree.treeOrder(ordinary)
        assertEquals(listOf(1L, 2L, 3L, 5L, 6L, 7L), ordered.map { it.group.id })
        assertEquals(0, ordered.first { it.group.id == 2L }.depth)
        assertEquals(1, ordered.first { it.group.id == 3L }.depth)
        assertEquals(0, ordered.first { it.group.id == 5L }.depth)
    }

    @Test
    fun memberAdminRulesEnforceSuperadminEditabilityVicePresidentAckAndGroupParsing() {
        val adminMe = Me(
            id = 1L,
            name = "Admin",
            email = "admin@example.org",
            roles = listOf("admin"),
            permissions = listOf("admin_members.view", "admin_members.update", "admin_roles.update"),
        )
        val superMe = adminMe.copy(roles = listOf("superadmin"))
        val vpMe = adminMe.copy(roles = listOf("vice_president"))
        val vpPresMe = adminMe.copy(roles = listOf("vice_president", "president"))

        val targetSuper = AdminUserDto(id = 10L, name = "Super", roles = listOf("superadmin"))
        val targetNormal = AdminUserDto(id = 11L, name = "John", roles = emptyList())
        val targetCustomAdmin = AdminUserDto(id = 12L, name = "Mary", roles = listOf("role_mgr"))

        assertFalse(MemberAdminRules.isEditable(adminMe, targetSuper))
        assertTrue(MemberAdminRules.isEditable(superMe, targetSuper))
        assertTrue(MemberAdminRules.isEditable(adminMe, targetNormal))
        assertFalse(MemberAdminRules.isEditable(adminMe.copy(permissions = listOf("admin_members.view")), targetNormal))

        assertTrue(MemberAdminRules.viceNeedsPresidentAck(vpMe))
        assertFalse(MemberAdminRules.viceNeedsPresidentAck(vpPresMe))
        assertFalse(MemberAdminRules.viceNeedsPresidentAck(superMe))

        val rolesCatalog = listOf(
            AdminRoleDefDto(key = "admin", permissions = listOf("admin_roles.update")),
            AdminRoleDefDto(key = "role_mgr", name = "Role Mgr", permissions = listOf("admin_roles.update")),
            AdminRoleDefDto(key = "driver", permissions = listOf("drive.view")),
        )
        assertTrue(MemberAdminRules.promotingToAdmin(emptyList(), listOf("admin"), rolesCatalog))
        assertTrue(MemberAdminRules.promotingToAdmin(listOf("driver"), listOf("role_mgr"), rolesCatalog))
        assertFalse(MemberAdminRules.promotingToAdmin(targetCustomAdmin.roles, listOf("admin", "role_mgr"), rolesCatalog))
        assertFalse(MemberAdminRules.promotingToAdmin(emptyList(), listOf("driver"), rolesCatalog))

        val groups = listOf(
            GroupDto(id = 1L, name = "All", isAll = true),
            GroupDto(id = 2L, name = "Youth"),
            GroupDto(id = 3L, name = "Adults"),
            GroupDto(id = 4L, name = "Visitors", isVisitor = true),
        )
        assertEquals(setOf(2L, 3L), MemberAdminRules.parseInitialGroupIds("Youth、Adults、All", groups))
        assertFalse(MemberAdminRules.announcementAudienceChanged(listOf(2L, 3L), listOf(3L, 2L)))
        assertTrue(MemberAdminRules.announcementAudienceChanged(listOf(2L), listOf(2L, 3L)))
    }

    @Test
    fun dashboardStatsDeserializesStringTodayAndMemberCounters() {
        val raw = """
            {
              "today": "2026-11-26",
              "today_total": 12,
              "today_confirmed": 10,
              "today_changes": 1,
              "pending_approval": 2,
              "no_driver": 1,
              "needs_attention": 0,
              "live_drivers": 3,
              "max_live_drivers": 10,
              "members": 84,
              "pending_members": 4,
              "pending_recovery": 2,
              "open_invites": 5,
              "pending_plan_drafts": 1,
              "pending_ride_drafts": 0
            }
        """.trimIndent()
        val stats = apiJson.decodeFromString(AdminDashboardStatsDto.serializer(), raw)
        assertEquals("2026-11-26", stats.today)
        assertEquals(84L, stats.members)
        assertEquals(4L, stats.pendingMembers)
        assertEquals(2L, stats.pendingRecovery)
        assertEquals(5L, stats.openInvites)
    }

    @Test
    fun routesAndNotifLinkRouterGateAdminMembersAndAnnouncements() {
        val member = Me(
            id = 5L,
            name = "Member",
            email = "m@example.org",
            roles = listOf("member"),
            permissions = listOf("rides.view", "events.view"),
        )
        val admin = Me(
            id = 1L,
            name = "Admin",
            email = "a@example.org",
            roles = listOf("admin"),
            permissions = listOf(
                "admin_members.view",
                "admin_invites.view",
                "admin_recovery.view",
                "admin_announcements.view",
            ),
        )

        assertEquals(
            NotifDestination.Native(Routes.adminMembers(tab = "recovery")),
            NotifLinkRouter.resolveFor("/admin/members?tab=recovery", admin),
        )
        assertEquals(
            NotifDestination.Native(Routes.announcements(manage = true)),
            NotifLinkRouter.resolveFor("/admin/announcements", admin),
        )
        assertEquals(
            NotifDestination.Native(Routes.announcements(manage = false)),
            NotifLinkRouter.resolveFor("/announcements", member),
        )
        assertEquals(NotifDestination.None, NotifLinkRouter.resolveFor("/admin/members", member))
        assertEquals(NotifDestination.None, NotifLinkRouter.resolveFor("/admin/announcements", member))

        assertTrue(Routes.isPermitted(AppRoute.AdminMembers(tab = "members"), admin))
        assertFalse(Routes.isPermitted(AppRoute.AdminMembers(tab = "members"), member))
        assertFalse(Routes.isPermitted(AppRoute.Announcements(manage = true), member))
        assertTrue(Routes.isPermitted(AppRoute.Announcements(manage = false), member))
    }

    // ---------------- ViewModel Integration Tests ----------------

    @Test
    fun adminMembersViewModelFullLifecycle() = runBlocking<Unit> {
        val vpMeJson = """
            {"id":2,"name":"VP Tan","email":"vp@example.org","status":"active",
             "roles":["vice_president"],
             "permissions":["admin_dashboard.view","admin_members.view","admin_members.update","admin_roles.view","admin_roles.update","admin_invites.view","admin_invites.create","admin_invites.delete","admin_recovery.view","admin_recovery.update"],
             "mfa_ok":true}
        """.trimIndent()
        val usersJson = """
            {"items":[
              {"id":10,"name":"Pending Lee","email":"lee@example.org","phone":"012-111","roles":[],"status":"pending","created_at":"2026-11-25 09:00:00","passkeys":0,"groups":""},
              {"id":11,"name":"Active Wong","email":"wong@example.org","phone":"012-222","roles":["driver"],"status":"active","created_at":"2026-11-20 09:00:00","passkeys":1,"groups":"Youth"}
            ],"total":2,"page":1,"page_size":50,"total_pages":1}
        """.trimIndent()
        val rolesJson = """
            {"roles":[
              {"key":"member","name":"","implicit":true,"assignable":false,"permissions":["rides.view"]},
              {"key":"admin","name":"","implicit":false,"assignable":true,"permissions":["admin_roles.update"]},
              {"key":"driver","name":"","implicit":false,"assignable":true,"permissions":["drive.view"]}
            ]}
        """.trimIndent()
        val groupsJson = """
            [{"id":1,"name":"All","is_all":true,"is_visitor":false,"member_count":10},
             {"id":2,"name":"Youth","is_all":false,"is_visitor":false,"member_count":5}]
        """.trimIndent()

        val recorded = CopyOnWriteArrayList<Pair<String, String>>()
        var recoveryCalls = 0
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val path = request.path.orEmpty()
                val body = request.body.readUtf8()
                recorded += "${request.method} $path" to body
                return when {
                    path == "/api/auth/me" -> json(vpMeJson)
                    path == "/api/admin/dashboard" -> json("""{"today":"2026-11-26","members":2,"pending_members":1,"pending_recovery":1,"open_invites":1}""")
                    path == "/api/admin/roles" -> json(rolesJson)
                    path == "/api/groups" -> json(groupsJson)
                    path.startsWith("/api/admin/users") && request.method == "GET" -> json(usersJson)
                    path == "/api/admin/users/10" && request.method == "PUT" -> json("""{"ok":true}""")
                    path == "/api/admin/users/11" && request.method == "PUT" ->
                        json("""{"error":"Cannot modify","code":"RANK_TOO_LOW"}""", 403)
                    path == "/api/admin/invites" && request.method == "GET" ->
                        json("""[{"id":31,"kind":"invite","email":"","roles":[],"status":"open","created_at":"2026-11-25 10:00:00","url":"https://graceride.saliminn.my/invite/tok"}]""")
                    path == "/api/admin/invites" && request.method == "POST" ->
                        json("""{"id":32,"token":"abc","url":"https://graceride.saliminn.my/invite/abc"}""")
                    path == "/api/admin/invites/31" && request.method == "DELETE" -> json("""{"ok":true}""")
                    path == "/api/admin/recovery" && request.method == "GET" ->
                        json("""[{"id":41,"email":"wong@example.org","name":"Active Wong","reason":"Lost phone","status":"pending","created_at":"2026-11-26 08:00:00"}]""")
                    path == "/api/admin/recovery/41/approve" && request.method == "POST" -> {
                        recoveryCalls++
                        if (recoveryCalls == 1) json("""{"awaiting_second":true}""")
                        else json("""{"url":"https://graceride.saliminn.my/invite/rec41"}""")
                    }
                    path == "/api/admin/recovery/41/reject" && request.method == "POST" -> json("""{"ok":true}""")
                    else -> json("""{"ok":true}""")
                }
            }
        }

        container.auth.refresh()
        val vm = AdminMembersViewModel(container)
        val loaded = vm.state.awaitUntil { !it.membersLoading && it.usersPage.items.size == 2 && it.groupsLoaded }
        assertEquals(1L, loaded.stats?.pendingMembers)

        // Edit/Approve single user with president_acknowledged
        var saved = false
        vm.saveUser(
            user = loaded.usersPage.items[0],
            roles = listOf("admin"),
            status = "active",
            groupIds = listOf(2L),
            presidentAcknowledged = true,
            isApproveMode = true,
            onSuccess = { saved = true },
        )
        val afterSave = vm.state.awaitUntil { it.notice == AdminMembersNotice.MEMBER_APPROVED }
        assertTrue(saved)
        assertNotNull(afterSave)
        val putCall = recorded.first { it.first == "PUT /api/admin/users/10" }
        assertTrue(putCall.second.contains("\"president_acknowledged\":true"))
        assertTrue(putCall.second.contains("\"group_ids\":[2]"))

        // Batch approve partial failure: user 10 succeeds, user 11 fails 403
        vm.toggleSelectUser(10L)
        vm.toggleSelectUser(11L)
        var batchDone = false
        vm.batchApprove(
            users = loaded.usersPage.items,
            roleKey = "driver",
            groupIds = listOf(2L),
            groupsTouched = true,
            presidentAcknowledged = false,
            onDone = { batchDone = true },
        )
        val afterBatch = vm.state.awaitUntil { !it.batchBusy && it.batchSuccessCount != null }
        assertTrue(batchDone)
        assertEquals(1, afterBatch.batchSuccessCount)
        assertEquals(1, afterBatch.batchFailureCount)
        assertEquals(setOf(11L), afterBatch.selectedUserIds)

        // Invites create + revoke
        vm.setTab("invites")
        vm.state.awaitUntil { !it.invitesLoading && it.invites.size == 1 }
        vm.clearNotices()
        vm.createInvite()
        val afterInvite = vm.state.awaitUntil { it.lastCreatedInviteUrl == "https://graceride.saliminn.my/invite/abc" }
        assertEquals(AdminMembersNotice.INVITE_CREATED, afterInvite.notice)
        vm.clearNotices()
        vm.revokeInvite(31L)
        vm.state.awaitUntil { it.notice == AdminMembersNotice.INVITE_REVOKED }

        // Recovery approve (awaiting_second -> url) + reject
        vm.setTab("recovery")
        vm.state.awaitUntil { !it.recoveryLoading && it.recovery.size == 1 }
        vm.clearNotices()
        vm.approveRecovery(41L)
        vm.state.awaitUntil { it.notice == AdminMembersNotice.RECOVERY_AWAITING_SECOND }
        vm.clearNotices()
        vm.approveRecovery(41L)
        val afterRecUrl = vm.state.awaitUntil {
            it.lastRecoveryUrl == "https://graceride.saliminn.my/invite/rec41" &&
                it.notice == AdminMembersNotice.RECOVERY_APPROVED
        }
        assertEquals(AdminMembersNotice.RECOVERY_APPROVED, afterRecUrl.notice)
        vm.clearNotices()
        vm.rejectRecovery(41L)
        vm.state.awaitUntil { it.notice == AdminMembersNotice.RECOVERY_REJECTED }
    }

    @Test
    fun announcementsViewModelFullLifecycle() = runBlocking<Unit> {
        val adminMeJson = """
            {"id":1,"name":"Secretary Lim","email":"sec@example.org","status":"active",
             "roles":["secretary"],
             "permissions":["admin_announcements.view","admin_announcements.create","admin_announcements.delete"],
             "mfa_ok":true}
        """.trimIndent()
        val annListJson = """
            [{"id":11,"title":"Bus Gate","body":"Wait at Gate B","pinned":false,"by":"Secretary Lim","created_at":"2026-11-26 08:00:00","targeted":true,"groups":[{"id":2,"name":"Youth"}]}]
        """.trimIndent()
        val groupsJson = """
            [{"id":1,"name":"All","is_all":true,"is_visitor":false,"member_count":20},
             {"id":2,"name":"Youth","is_all":false,"is_visitor":false,"member_count":8},
             {"id":3,"name":"Adults","is_all":false,"is_visitor":false,"member_count":12}]
        """.trimIndent()

        val recorded = CopyOnWriteArrayList<Pair<String, String>>()
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val path = request.path.orEmpty()
                val body = request.body.readUtf8()
                recorded += "${request.method} $path" to body
                return when {
                    path == "/api/auth/me" -> json(adminMeJson)
                    path.startsWith("/api/announcements") && request.method == "GET" -> json(annListJson)
                    path == "/api/groups" -> json(groupsJson)
                    path == "/api/announcements" && request.method == "POST" -> json("""{"id":12}""")
                    path == "/api/announcements/11" && request.method == "PUT" -> json("""{"ok":true}""")
                    path == "/api/announcements/11" && request.method == "DELETE" -> json("""{"ok":true}""")
                    else -> json("""{"ok":true}""")
                }
            }
        }

        container.auth.refresh()
        val vm = AnnouncementsViewModel(container, initialManage = true)
        val loaded = vm.state.awaitUntil { !it.loading && it.items.size == 1 && it.groups.size == 3 }
        assertTrue(loaded.manageMode)
        assertTrue("Expected manage=true query, got: ${recorded.map { it.first }}", recorded.any { it.first == "GET /api/announcements?manage=true" })

        // Create announcement with notify_all & group_ids
        var createdOk = false
        vm.createAnnouncement(
            title = "Camp Briefing",
            body = "Bring umbrella",
            pinned = true,
            notifyAll = true,
            groupIds = listOf(2L),
            onSuccess = { createdOk = true },
        )
        vm.state.awaitUntil { it.notice == AnnouncementNotice.PUBLISHED }
        assertTrue(createdOk)
        val postCall = recorded.first { it.first == "POST /api/announcements" }
        assertTrue(postCall.second.contains("\"notify_all\":true"))
        assertTrue(postCall.second.contains("\"group_ids\":[2]"))

        // Update with unchanged audience MUST omit group_ids
        val item = Announcement(
            id = 11L,
            title = "Bus Gate",
            body = "Wait at Gate B",
            pinned = false,
            groups = listOf(GroupRef(2L, "Youth")),
        )
        recorded.clear()
        vm.clearNotice()
        vm.updateAnnouncement(
            item = item,
            title = "Bus Gate B",
            body = "Wait at Gate B2",
            pinned = false,
            groupIds = listOf(2L),
            onSuccess = {},
        )
        vm.state.awaitUntil { it.notice == AnnouncementNotice.UPDATED }
        val unchangedPut = recorded.first { it.first == "PUT /api/announcements/11" }
        assertFalse("Unchanged audience should omit group_ids: ${unchangedPut.second}", unchangedPut.second.contains("group_ids"))

        // Update with changed audience MUST include group_ids
        recorded.clear()
        vm.clearNotice()
        vm.updateAnnouncement(
            item = item,
            title = "Bus Gate B",
            body = "Wait at Gate B2",
            pinned = false,
            groupIds = listOf(3L),
            onSuccess = {},
        )
        vm.state.awaitUntil { it.notice == AnnouncementNotice.UPDATED }
        val changedPut = recorded.first { it.first == "PUT /api/announcements/11" }
        assertTrue(changedPut.second.contains("\"group_ids\":[3]"))

        // Toggle pin omits group_ids
        recorded.clear()
        vm.clearNotice()
        vm.togglePin(item)
        vm.state.awaitUntil { it.notice == AnnouncementNotice.PINNED }
        val pinPut = recorded.first { it.first == "PUT /api/announcements/11" }
        assertFalse(pinPut.second.contains("group_ids"))
        assertTrue(pinPut.second.contains("\"pinned\":true"))

        // Delete
        vm.clearNotice()
        vm.deleteAnnouncement(11L)
        vm.state.awaitUntil { it.notice == AnnouncementNotice.DELETED }
        assertTrue(recorded.any { it.first == "DELETE /api/announcements/11" })
    }
}
