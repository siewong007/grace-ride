package org.pjstatecm.graceride

import android.content.Context
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import org.pjstatecm.graceride.data.api.BiometricEnrollment
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.domain.XlsxCodec
import org.pjstatecm.graceride.domain.XlsxErrorCode
import org.pjstatecm.graceride.domain.XlsxException
import org.pjstatecm.graceride.platform.BiometricAuthManager
import org.pjstatecm.graceride.platform.BiometricAvailability
import org.pjstatecm.graceride.platform.BiometricCryptoDriver
import org.pjstatecm.graceride.platform.BiometricSignResult
import org.pjstatecm.graceride.ui.NotifDestination
import org.pjstatecm.graceride.ui.NotifLinkRouter
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.components.isValidTurnstileSiteKey
import java.io.ByteArrayOutputStream
import java.nio.charset.StandardCharsets
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

class BughunterRemediationTest {

    private fun buildZip(entries: Map<String, String>): ByteArray {
        val baos = ByteArrayOutputStream()
        ZipOutputStream(baos).use { zos ->
            for ((name, body) in entries) {
                zos.putNextEntry(ZipEntry(name))
                zos.write(body.toByteArray(StandardCharsets.UTF_8))
                zos.closeEntry()
            }
        }
        return baos.toByteArray()
    }

    private fun buildSyntheticXlsx(
        sheetTags: String,
        relsTags: String,
        worksheets: Map<String, String>,
    ): ByteArray {
        val map = LinkedHashMap<String, String>()
        map["_rels/.rels"] =
            """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>""" +
            """<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">""" +
            """<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>""" +
            """</Relationships>"""
        map["xl/workbook.xml"] =
            """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>""" +
            """<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" """ +
            """xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">""" +
            """<sheets>$sheetTags</sheets></workbook>"""
        map["xl/_rels/workbook.xml.rels"] =
            """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>""" +
            """<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">""" +
            """$relsTags</Relationships>"""
        for ((path, body) in worksheets) {
            map[path] =
                """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>""" +
                """<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">""" +
                """<sheetData>$body</sheetData></worksheet>"""
        }
        return buildZip(map)
    }

    @Test
    fun routesAndNotifLinkRouterRejectTraversalAndControlChars() {
        val member = apiJson.decodeFromString(
            Me.serializer(),
            """{"id":1,"name":"Test Member","email":"member@example.org","status":"active","permissions":["rides.view","reimbursements.view"],"mfa_ok":true}""",
        )
        assertTrue(Routes.isWebPathPermitted("/reimbursements", member))
        assertEquals(NotifDestination.Native(Routes.rideDetail(42)), NotifLinkRouter.resolve("/rides/42"))

        val unsafePaths = listOf(
            "/../admin",
            "/rides/../admin",
            "/./rides",
            "/%2e%2e/admin",
            "/%2E%2E/admin",
            "/%2fadmin",
            "/%5cadmin",
            "/rides\\admin",
            "/rides\nfoo",
            "/rides\u0000",
            "//evil.com/rides",
            "/api/auth/me",
            "javascript:alert(1)",
        )
        for (candidate in unsafePaths) {
            assertFalse("Expected rejected web path for $candidate", Routes.isWebPathPermitted(candidate, member))
            assertEquals("Expected None destination for $candidate", NotifDestination.None, NotifLinkRouter.resolve(candidate))
        }
    }

    @Test
    fun xlsxCodecRejectsOversizedCoordinatesExcessSheetsAndDuplicateTargets() {
        val overRow = buildSyntheticXlsx(
            sheetTags = """<sheet name="S1" sheetId="1" r:id="rId1"/>""",
            relsTags = """<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>""",
            worksheets = mapOf("xl/worksheets/sheet1.xml" to """<row r="99999"><c r="Z99999"><v>1</v></c></row>"""),
        )
        try {
            XlsxCodec.readWorkbook(overRow)
            fail("Expected TOO_LARGE for Z99999")
        } catch (e: XlsxException) {
            assertEquals(XlsxErrorCode.TOO_LARGE, e.code)
        }

        // Col IT = 253 (< MAX_COLS=256), Row 499 (< MAX_ROWS=2000) -> 499 * 254 = 126,746 > MAX_GRID_CELLS (100,000)
        val overGrid = buildSyntheticXlsx(
            sheetTags = """<sheet name="S1" sheetId="1" r:id="rId1"/>""",
            relsTags = """<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>""",
            worksheets = mapOf("xl/worksheets/sheet1.xml" to """<row r="499"><c r="IT499"><v>1</v></c></row>"""),
        )
        try {
            XlsxCodec.readWorkbook(overGrid)
            fail("Expected TOO_LARGE for grid > 100,000 cells")
        } catch (e: XlsxException) {
            assertEquals(XlsxErrorCode.TOO_LARGE, e.code)
        }

        val sheetTags21 = (1..21).joinToString("") { i ->
            """<sheet name="S$i" sheetId="$i" r:id="rId$i"/>"""
        }
        val rels21 = (1..21).joinToString("") { i ->
            """<Relationship Id="rId$i" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet$i.xml"/>"""
        }
        val excessSheets = buildSyntheticXlsx(
            sheetTags = sheetTags21,
            relsTags = rels21,
            worksheets = mapOf("xl/worksheets/sheet1.xml" to """<row r="1"><c r="A1"><v>1</v></c></row>"""),
        )
        try {
            XlsxCodec.readWorkbook(excessSheets)
            fail("Expected TOO_LARGE for 21 sheets")
        } catch (e: XlsxException) {
            assertEquals(XlsxErrorCode.TOO_LARGE, e.code)
        }

        val duplicateTarget = buildSyntheticXlsx(
            sheetTags = """<sheet name="One" sheetId="1" r:id="rId1"/><sheet name="Two" sheetId="2" r:id="rId2"/>""",
            relsTags =
                """<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>""" +
                """<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>""",
            worksheets = mapOf("xl/worksheets/sheet1.xml" to """<row r="1"><c r="A1"><v>1</v></c></row>"""),
        )
        try {
            XlsxCodec.readWorkbook(duplicateTarget)
            fail("Expected INVALID_XLSX for duplicate worksheet target")
        } catch (e: XlsxException) {
            assertEquals(XlsxErrorCode.INVALID_XLSX, e.code)
        }
    }

    @Test
    fun biometricAuthManagerIdleRelockFiresCallbackAndGatesApp() {
        var nowMs = 1_000L
        var relockedCount = 0
        val stubDriver = object : BiometricCryptoDriver {
            override fun availability(context: Context?): BiometricAvailability = BiometricAvailability.AVAILABLE
            override suspend fun generateAndSign(
                context: Context,
                payload: ByteArray,
                title: String,
                subtitle: String,
                cancelText: String,
                attestationChallenge: ByteArray?,
            ): BiometricSignResult = BiometricSignResult.Cancelled

            override suspend fun signWithEnrolledKey(
                context: Context,
                payload: ByteArray,
                title: String,
                subtitle: String,
                cancelText: String,
            ): BiometricSignResult = BiometricSignResult.Cancelled

            override suspend fun verifyLocalUnlock(
                context: Context,
                title: String,
                subtitle: String,
                cancelText: String,
            ): BiometricSignResult = BiometricSignResult.Cancelled

            override fun deleteKey() {}
        }

        val manager = BiometricAuthManager(
            store = InMemoryStore(),
            driver = stubDriver,
            clockMs = { nowMs },
        )
        manager.saveEnrollment(
            BiometricEnrollment(
                id = 1L,
                credentialId = "cred-1",
                userId = 1L,
                email = "member@example.org",
                userName = "Test Member",
                label = "Test",
                enrolledAtMs = 500L,
            ),
        )
        manager.markAppUnlocked()
        assertFalse(manager.requiresAppGate(1L))

        manager.onRelocked = { relockedCount += 1 }
        manager.onAppBackgrounded(nowMs)

        // Advance well past IDLE_RELOCK_MS
        nowMs += BiometricAuthManager.IDLE_RELOCK_MS + 1_000L
        manager.onAppForegrounded(nowMs)

        assertEquals(1, relockedCount)
        assertTrue(manager.requiresAppGate(1L))
    }

    @Test
    fun turnstileSiteKeyValidatorAllowsOnlySafeTokens() {
        assertTrue(isValidTurnstileSiteKey("0x4AAAAAAA_valid-Key123"))
        assertFalse(isValidTurnstileSiteKey(""))
        assertFalse(isValidTurnstileSiteKey("key';alert(1)//"))
        assertFalse(isValidTurnstileSiteKey("key\"onload=1"))
        assertFalse(isValidTurnstileSiteKey("key with spaces"))
        assertFalse(isValidTurnstileSiteKey("a".repeat(129)))
    }
}
