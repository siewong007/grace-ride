package org.pjstatecm.graceride

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.pjstatecm.graceride.data.api.Area
import org.pjstatecm.graceride.data.api.DriverVehicleReq
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.domain.RideStatus
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.domain.inferAreaId
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.components.BadgeTierStyle
import org.pjstatecm.graceride.ui.theme.ThemeMode
import java.time.LocalDate
import java.util.Locale

class MoneyTest {
    @Test fun zero() = assertEquals("RM 0.00", Money.rm(0))
    @Test fun senOnly() = assertEquals("RM 0.05", Money.rm(5))
    @Test fun ringgitAndSen() = assertEquals("RM 12.34", Money.rm(1234))
    @Test fun thousandsGrouped() = assertEquals("RM 1,000.00", Money.rm(100000))
    @Test fun negative() = assertEquals("-RM 3.50", Money.rm(-350))
    @Test fun largeAmount() = assertEquals("RM 12,345.67", Money.rm(1234567))
}

class KlTimeTest {
    @Test fun parsesFullStamp() {
        val dt = KlTime.parse("2026-11-21 09:30:00")
        assertEquals(2026, dt?.year)
        assertEquals(9, dt?.hour)
        assertEquals(30, dt?.minute)
    }

    @Test fun parsesMinutePrecision() {
        val dt = KlTime.parse("2026-11-21 09:30")
        assertEquals(30, dt?.minute)
    }

    @Test fun parsesDateOnly() {
        val dt = KlTime.parse("2026-11-21")
        assertEquals(0, dt?.hour)
    }

    @Test fun parsesIsoTSeparator() {
        val dt = KlTime.parse("2026-11-22T07:00")
        assertEquals(7, dt?.hour)
        assertEquals("Sun, 22 Nov · 07:00", KlTime.display("2026-11-22T07:00"))
    }

    @Test fun rejectsGarbage() {
        assertEquals(null, KlTime.parse("not a date"))
        assertEquals(null, KlTime.parse(""))
    }

    @Test fun apiStampFormat() {
        assertEquals("2026-11-21 09:05", KlTime.apiStamp(LocalDate.of(2026, 11, 21), 9, 5))
    }

    @Test fun displayFallsBackToRawText() {
        assertEquals("whenever", KlTime.display("whenever"))
    }

    @Test fun displayReflectsLocaleChangesWithoutStaleFormatter() {
        val prev = Locale.getDefault()
        try {
            Locale.setDefault(Locale.US)
            val en = KlTime.display("2026-11-22 07:00:00")
            assertEquals("Sun, 22 Nov · 07:00", en)

            Locale.setDefault(Locale.SIMPLIFIED_CHINESE)
            val zh = KlTime.display("2026-11-22 07:00:00")
            assertNotEquals(en, zh)
            assertEquals("07:00", KlTime.time("2026-11-22 07:00:00"))
            assertEquals("2026-11-22", KlTime.date("2026-11-22 07:00:00"))
        } finally {
            Locale.setDefault(prev)
        }
    }
}

class AreaMatchAndRoutesTest {
    private val areas = listOf(
        Area(id = 1, name = "PJ", active = true),
        Area(id = 2, name = "PJ State", active = true),
        Area(id = 3, name = "八达灵再也", active = true),
        Area(id = 4, name = "Inactive Area", active = false),
    )

    @Test fun inferAreaRespectsAsciiWordBoundariesAndLongestMatch() {
        // Must not partially match "pj" inside "pj2"
        assertNull(inferAreaId("12 Jalan PJ2", areas))
        // Matches later valid occurrence after partial false positive
        assertEquals(1L, inferAreaId("PJ2 block B, PJ", areas))
        // Longest active match wins ("PJ State" over "PJ")
        assertEquals(2L, inferAreaId("Jalan Yong Shook Lin, PJ State", areas))
        // Ignores inactive areas
        assertNull(inferAreaId("Inactive Area", areas))
        // Matches Chinese area names
        assertEquals(3L, inferAreaId("雪兰莪八达灵再也新市镇", areas))
    }

    @Test fun studioRouteEncodesSpacesAndSpecialCharsRfc3986() {
        assertEquals("studio?shape=my%20shape&color=deep%2Bblue&face=o%2Fo", Routes.studio("my shape", "deep+blue", "o/o"))
    }
}

class RideStatusTest {
    @Test fun activeStatuses() {
        assertEquals(RideStatusKind.ACTIVE, RideStatus.kind("confirmed"))
        assertEquals(RideStatusKind.ACTIVE, RideStatus.kind("in_progress"))
    }

    @Test fun pendingStatuses() {
        listOf("pending_driver", "pending_passenger", "pending_approval", "pending_driver_confirm").forEach {
            assertEquals(RideStatusKind.PENDING, RideStatus.kind(it))
        }
    }

    @Test fun terminalStatuses() {
        assertEquals(RideStatusKind.DONE, RideStatus.kind("completed"))
        assertEquals(RideStatusKind.CANCELLED, RideStatus.kind("cancelled"))
        assertEquals(RideStatusKind.CANCELLED, RideStatus.kind("no_show"))
        assertEquals(RideStatusKind.CANCELLED, RideStatus.kind("alt_transport"))
    }

    @Test fun cancellableMirrorsBackendGuard() {
        listOf("pending_driver", "pending_passenger", "pending_approval", "pending_driver_confirm", "confirmed").forEach {
            assertEquals(true, RideStatus.cancellable(it))
        }
        listOf("in_progress", "completed", "cancelled", "no_show", "alt_transport").forEach {
            assertEquals(false, RideStatus.cancellable(it))
        }
    }

    @Test fun everyKnownStatusHasALabel() {
        (RideStatus.active + RideStatus.terminal).forEach {
            assertEquals(true, RideStatus.labelRes(it) != 0)
        }
    }
}

class ThemeAndBadgeTest {
    @Test fun themeModeFromKeyDefaultsToSystem() {
        assertEquals(ThemeMode.SYSTEM, ThemeMode.fromKey(null))
        assertEquals(ThemeMode.SYSTEM, ThemeMode.fromKey("invalid"))
        assertEquals(ThemeMode.LIGHT, ThemeMode.fromKey("light"))
        assertEquals(ThemeMode.DARK, ThemeMode.fromKey("dark"))
    }

    @Test fun appContainerPersistsThemeModeAndValidatesInput() {
        val store = InMemoryStore()
        val container = AppContainer(null, store)
        assertEquals("system", container.themeMode.value)

        container.setThemeMode("dark")
        assertEquals("dark", container.themeMode.value)
        assertEquals("dark", store.getString("theme_mode"))

        container.setThemeMode("unknown")
        assertEquals("system", container.themeMode.value)
        assertEquals("system", store.getString("theme_mode"))
    }

    @Test fun badgeTierStyleProvidesDistinctDarkModePalettes() {
        BadgeTierStyle.ALL_TIERS.forEach { tier ->
            val light = BadgeTierStyle.palette(tier, darkTheme = false)
            val dark = BadgeTierStyle.palette(tier, darkTheme = true)
            assertNotEquals(light.bg, dark.bg)
            assertNotEquals(light.ink, dark.ink)
        }
    }

    @Test fun appContainerTracksLaunchSplashShownState() {
        val container = AppContainer(null, InMemoryStore())
        assertEquals(false, container.launchSplashShown)
        container.launchSplashShown = true
        assertEquals(true, container.launchSplashShown)
    }
}

class ArchitectureAndPlatformTest {
    @Test fun qrMatrixGeneratesValidSquareGridAndRejectsBlankInput() {
        assertEquals(null, org.pjstatecm.graceride.platform.generateQrMatrix("   "))
        val matrix = org.pjstatecm.graceride.platform.generateQrMatrix("GRPASS:42:ABCD", targetSize = 0)
        assertEquals(true, matrix != null)
        matrix!!
        assertEquals(true, matrix.size >= 21)
        assertEquals(matrix.size * matrix.size, matrix.bits.size)
        assertEquals(true, matrix.bits.any { it } && matrix.bits.any { !it })
    }

    @Test fun appRouteFromLegacyRouteParsesAllTypedDestinations() {
        assertEquals(
            org.pjstatecm.graceride.ui.AppRoute.DriveHub,
            org.pjstatecm.graceride.ui.AppRoute.fromLegacyRoute(org.pjstatecm.graceride.ui.Routes.DRIVE_HUB),
        )
        assertEquals(
            org.pjstatecm.graceride.ui.AppRoute.RideDetail(id = 12L, justBooked = true),
            org.pjstatecm.graceride.ui.AppRoute.fromLegacyRoute(
                org.pjstatecm.graceride.ui.Routes.rideDetail(12L, justBooked = true),
            ),
        )
        assertEquals(
            org.pjstatecm.graceride.ui.AppRoute.RideNew(eventId = 5L, proxy = true),
            org.pjstatecm.graceride.ui.AppRoute.fromLegacyRoute(
                org.pjstatecm.graceride.ui.Routes.rideNew(eventId = 5L, proxy = true),
            ),
        )
        assertEquals(
            org.pjstatecm.graceride.ui.AppRoute.Studio(shape = "cloud", color = "blue", face = "happy"),
            org.pjstatecm.graceride.ui.AppRoute.fromLegacyRoute(
                org.pjstatecm.graceride.ui.Routes.studio(shape = "cloud", color = "blue", face = "happy"),
            ),
        )
    }

    @Test fun offlineCachePersistsIsolatesPerUserAndClears() {
        val ser = kotlinx.serialization.builtins.ListSerializer(kotlinx.serialization.serializer<String>())
        val memCache = org.pjstatecm.graceride.data.cache.OfflineCache(cacheDir = null)
        memCache.write(1L, "rides", listOf("r1", "r2"), ser, nowMillis = 1234L)
        memCache.write(2L, "rides", listOf("other"), ser, nowMillis = 1235L)

        assertEquals(listOf("r1", "r2"), memCache.read(1L, "rides", ser)?.data)
        assertEquals(1234L, memCache.read(1L, "rides", ser)?.savedAtMillis)
        memCache.clearUser(1L)
        assertEquals(null, memCache.read(1L, "rides", ser))
        assertEquals(listOf("other"), memCache.read(2L, "rides", ser)?.data)
        memCache.clearAll()
        assertEquals(null, memCache.read(2L, "rides", ser))

        val tmpDir = java.nio.file.Files.createTempDirectory("gr_cache_test").toFile()
        try {
            val diskWriter = org.pjstatecm.graceride.data.cache.OfflineCache(tmpDir)
            diskWriter.write(7L, "saved_places", listOf("Home", "Church"), ser, nowMillis = 999L)
            val diskReader = org.pjstatecm.graceride.data.cache.OfflineCache(tmpDir)
            assertEquals(listOf("Home", "Church"), diskReader.read(7L, "saved_places", ser)?.data)
            diskReader.clearUser(7L)
            assertEquals(null, diskReader.read(7L, "saved_places", ser))
        } finally {
            tmpDir.deleteRecursively()
        }
    }

    @Test fun driverVehicleReqSerializesDefaultFalseBooleansExplicitly() {
        val raw = apiJson.encodeToString(
            DriverVehicleReq.serializer(),
            DriverVehicleReq(model = "Perodua Alza", color = "Silver", plate = "WAA 1234", seats = 6),
        )
        val obj = apiJson.parseToJsonElement(raw) as kotlinx.serialization.json.JsonObject
        assertEquals("standard", (obj["vehicle_type"] as kotlinx.serialization.json.JsonPrimitive).content)
        assertEquals("false", (obj["wheelchair_ok"] as kotlinx.serialization.json.JsonPrimitive).content)
        assertEquals("false", (obj["low_step"] as kotlinx.serialization.json.JsonPrimitive).content)
        assertEquals("false", (obj["child_seat"] as kotlinx.serialization.json.JsonPrimitive).content)
        assertEquals("false", (obj["is_active"] as kotlinx.serialization.json.JsonPrimitive).content)
    }

    @Test fun qrCheckinAndMemberPassParsersValidateEventScope() {
        assertEquals(
            org.pjstatecm.graceride.platform.CheckinParseResult.Code("ABC123"),
            org.pjstatecm.graceride.platform.parseCheckinInput("ABC123", 42L),
        )
        assertEquals(
            org.pjstatecm.graceride.platform.CheckinParseResult.Code("ABC123"),
            org.pjstatecm.graceride.platform.parseCheckinInput("https://graceride.app/events/42/checkin?code=ABC123#qr", 42L),
        )
        assertEquals(
            org.pjstatecm.graceride.platform.CheckinParseResult.WrongEvent,
            org.pjstatecm.graceride.platform.parseCheckinInput("/events/99/checkin?code=ABC123", 42L),
        )
        assertEquals(
            org.pjstatecm.graceride.platform.CheckinParseResult.Code("XYZ"),
            org.pjstatecm.graceride.platform.parseMemberPassInput("GRPASS:42:XYZ", 42L),
        )
        assertEquals(
            org.pjstatecm.graceride.platform.CheckinParseResult.WrongEvent,
            org.pjstatecm.graceride.platform.parseMemberPassInput("GRPASS:99:XYZ", 42L),
        )
        assertEquals(
            org.pjstatecm.graceride.platform.CheckinParseResult.Invalid,
            org.pjstatecm.graceride.platform.parseMemberPassInput("   ", 42L),
        )
    }

    @Test fun klTimeParsesOffsetsAndFormatsChineseLocale() {
        val fromOffset = KlTime.parse("2026-11-22T09:30:00+08:00")
        assertEquals(9, fromOffset?.hour)
        assertEquals(30, fromOffset?.minute)

        val fromUtc = KlTime.parse("2026-11-22T01:30:00Z")
        assertEquals(9, fromUtc?.hour)
        assertEquals(30, fromUtc?.minute)

        val minutePrec = KlTime.parse("2026-11-22 09:30")
        assertEquals(9, minutePrec?.hour)
        assertEquals(30, minutePrec?.minute)

        val prev = Locale.getDefault()
        try {
            Locale.setDefault(Locale.SIMPLIFIED_CHINESE)
            val disp = KlTime.display("2026-11-22 09:30:00")
            val dateOnly = KlTime.displayDate("2026-11-22 09:30:00")
            val short = KlTime.dayMonthShort("2026-11-22 09:30:00")
            assertTrue("Expected Chinese date in '$disp'", disp.contains("11月22日"))
            assertTrue("Expected Chinese date in '$dateOnly'", dateOnly.contains("11月22日"))
            assertEquals("11月22日", short)
        } finally {
            Locale.setDefault(prev)
        }
    }

    @Test fun routesIsPermittedEnforcesRoleMatrix() {
        val member = Me(
            id = 1L,
            name = "Member",
            email = "m@x.org",
            permissions = listOf("rides.view", "events.view", "costs.view"),
        )
        assertTrue(Routes.isPermitted(org.pjstatecm.graceride.ui.AppRoute.Home, member))
        assertTrue(Routes.isPermitted(org.pjstatecm.graceride.ui.AppRoute.Rides, member))
        assertTrue(Routes.isPermitted(org.pjstatecm.graceride.ui.AppRoute.Events, member))
        assertTrue(Routes.isPermitted(org.pjstatecm.graceride.ui.AppRoute.EventDetail(1L), member))
        assertTrue(Routes.isPermitted(org.pjstatecm.graceride.ui.AppRoute.Costs, member))
        assertFalse(Routes.isPermitted(org.pjstatecm.graceride.ui.AppRoute.DriveHub, member))
        assertFalse(Routes.isPermitted(org.pjstatecm.graceride.ui.AppRoute.DriverSettings, member))
        assertFalse(Routes.isPermitted(org.pjstatecm.graceride.ui.AppRoute.RideNew(), member))
        assertFalse(Routes.isLegacyRoutePermitted(Routes.DRIVE_HUB, member))

        val driver = Me(id = 2L, name = "Driver", email = "d@x.org", permissions = listOf("drive.view"))
        assertTrue(Routes.isPermitted(org.pjstatecm.graceride.ui.AppRoute.DriveHub, driver))
        assertTrue(Routes.isPermitted(org.pjstatecm.graceride.ui.AppRoute.DriverSettings, driver))
        assertFalse(Routes.isPermitted(org.pjstatecm.graceride.ui.AppRoute.Rides, driver))

        val eventAdmin = Me(id = 3L, name = "Admin", email = "a@x.org", permissions = listOf("admin_events.view"))
        assertTrue(Routes.isPermitted(org.pjstatecm.graceride.ui.AppRoute.EventDetail(5L), eventAdmin))
        assertFalse(Routes.isPermitted(org.pjstatecm.graceride.ui.AppRoute.Events, eventAdmin))
    }

    @Test fun grokBotGeometryOffsetsEyesByBodyShape() {
        val (cL, cR) = org.pjstatecm.graceride.ui.components.GrokBotGeometry.computeFaceEyeTransforms("neutral", shape = "circle")
        val (dL, dR) = org.pjstatecm.graceride.ui.components.GrokBotGeometry.computeFaceEyeTransforms("neutral", shape = "droplet")
        assertNotEquals(cL.tx, dL.tx)
        assertEquals(-3.7f + cL.tx * 0.62f, dL.tx, 0.001f)
        assertEquals(3f + cL.ty * 0.62f, dL.ty, 0.001f)
        assertEquals(-3.7f + cR.tx * 0.62f, dR.tx, 0.001f)
    }

    @Test fun webPathPermissionAndStaffHelpersBlockNormalMembersFromAdmin() {
        val member = Me(
            id = 6L,
            name = "Normal Member",
            email = "member@example.org",
            roles = listOf("member"),
            permissions = listOf(
                "rides.view", "rides.create", "rides.update", "rides.delete",
                "events.view", "costs.view", "costs.update",
                "reimbursements.view", "reimbursements.create", "reimbursements.update",
                "help.view", "help.create", "help.update",
                "report.view", "settlements.view",
            ),
        )
        assertFalse(member.hasStaffAccess())
        assertFalse(Routes.canManageSettlements(member))
        assertFalse(Routes.canUseStaffWeb(member))
        listOf(
            "/admin",
            "/admin/menu",
            "/admin/traffic",
            "/admin/rides",
            "/admin/rides/10",
            "/admin/live",
            "/admin/members",
            "/admin/settings",
            "/admin/reimbursements",
            "/admin/announcements",
            "/admin/audit",
            "/admin/reports",
            "/admin/roles",
            "/admin/events/5/plan",
            "/admin/unknown",
            "/settlements",
            "/settlements/3",
            "/events/new",
            "/events/5/edit",
            "/events/5/settle",
            "/help/guides/new",
            "/unknown-page",
        ).forEach { blocked ->
            assertFalse("Member should not open $blocked", Routes.isWebPathPermitted(blocked, member))
        }

        val driver = Me(
            id = 7L,
            name = "Driver",
            email = "driver@example.org",
            roles = listOf("driver"),
            permissions = listOf("drive.view", "drive.create", "drive.update", "drive.delete"),
        )
        assertFalse(driver.hasStaffAccess())
        assertFalse(Routes.canUseStaffWeb(driver))
        assertFalse(Routes.isWebPathPermitted("/admin/rides", driver))

        val leader = Me(
            id = 8L,
            name = "Leader",
            email = "leader@example.org",
            roles = listOf("leader"),
            permissions = listOf("events.create", "events.update", "settlements.view", "settlements.create"),
        )
        assertFalse(leader.hasStaffAccess())
        assertTrue(Routes.canManageSettlements(leader))
        assertTrue(Routes.canUseStaffWeb(leader))
        assertTrue(Routes.isWebPathPermitted("/settlements", leader))
        assertTrue(Routes.isWebPathPermitted("/settlements/9", leader))
        assertTrue(Routes.isWebPathPermitted("/events/new", leader))
        assertTrue(Routes.isWebPathPermitted("/events/9/edit", leader))
        assertTrue(Routes.isWebPathPermitted("/events/9/settle", leader))
        assertFalse(Routes.isWebPathPermitted("/admin/rides", leader))
        assertFalse(Routes.isWebPathPermitted("/admin", leader))

        val transport = Me(
            id = 9L,
            name = "Transport",
            email = "transport@example.org",
            roles = listOf("transport"),
            permissions = listOf("admin_dashboard.view", "admin_rides.view", "admin_plan.view"),
        )
        assertTrue(transport.hasStaffAccess())
        assertTrue(Routes.canUseStaffWeb(transport))
        assertTrue(Routes.isWebPathPermitted("/admin", transport))
        assertTrue(Routes.isWebPathPermitted("/admin/rides", transport))
        assertTrue(Routes.isWebPathPermitted("/admin/rides/12", transport))
        assertTrue(Routes.isWebPathPermitted("/admin/events/4/plan", transport))
        assertFalse(Routes.isWebPathPermitted("/admin/roles", transport))

        val finance = Me(
            id = 10L,
            name = "Finance",
            email = "finance@example.org",
            roles = listOf("finance"),
            permissions = listOf("admin_reimb_pay.view", "admin_reimb_pay.update", "admin_bills.view", "admin_reports.view"),
        )
        assertTrue(finance.hasStaffAccess())
        assertTrue(Routes.canManageSettlements(finance))
        assertTrue(Routes.isWebPathPermitted("/settlements", finance))
        assertTrue(Routes.isWebPathPermitted("/admin/reports", finance))
        assertTrue(Routes.isWebPathPermitted("/admin/reimbursements", finance))
        assertFalse(Routes.isWebPathPermitted("/admin/rides", finance))
    }
}
