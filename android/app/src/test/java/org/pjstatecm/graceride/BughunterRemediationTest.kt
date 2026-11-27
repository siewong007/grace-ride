package org.pjstatecm.graceride

import android.content.Context
import android.content.ContextWrapper
import android.content.SharedPreferences
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import org.pjstatecm.graceride.data.api.ApiProvider
import org.pjstatecm.graceride.data.api.ApiService
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.BiometricEnrollment
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.repo.AuthRepository
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.data.session.PersistentCookieJar
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
import org.pjstatecm.graceride.ui.isSafeExternalWebUrl
import org.pjstatecm.graceride.widget.WidgetSnapshotStore
import org.pjstatecm.graceride.widget.WidgetSummary
import java.io.ByteArrayOutputStream
import java.lang.reflect.Proxy
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

    private fun fakePrefsContext(initial: MutableMap<String, MutableMap<String, Any?>>): Context {
        return object : ContextWrapper(null) {
            override fun getApplicationContext(): Context = this
            override fun getSharedPreferences(name: String, mode: Int): SharedPreferences {
                val data = initial.getOrPut(name) { linkedMapOf() }
                return object : SharedPreferences {
                    override fun getAll(): MutableMap<String, *> = HashMap(data)
                    override fun getString(key: String, defValue: String?): String? =
                        data[key] as? String ?: defValue
                    @Suppress("UNCHECKED_CAST")
                    override fun getStringSet(key: String, defValues: MutableSet<String>?): MutableSet<String>? =
                        data[key] as? MutableSet<String> ?: defValues
                    override fun getInt(key: String, defValue: Int): Int = data[key] as? Int ?: defValue
                    override fun getLong(key: String, defValue: Long): Long = data[key] as? Long ?: defValue
                    override fun getFloat(key: String, defValue: Float): Float = data[key] as? Float ?: defValue
                    override fun getBoolean(key: String, defValue: Boolean): Boolean = data[key] as? Boolean ?: defValue
                    override fun contains(key: String): Boolean = data.containsKey(key)
                    override fun edit(): SharedPreferences.Editor = object : SharedPreferences.Editor {
                        private val staged = linkedMapOf<String, Any?>()
                        private val removals = hashSetOf<String>()
                        private var cleared = false
                        override fun putString(key: String, value: String?): SharedPreferences.Editor = apply { staged[key] = value }
                        override fun putStringSet(key: String, values: MutableSet<String>?): SharedPreferences.Editor = apply { staged[key] = values }
                        override fun putInt(key: String, value: Int): SharedPreferences.Editor = apply { staged[key] = value }
                        override fun putLong(key: String, value: Long): SharedPreferences.Editor = apply { staged[key] = value }
                        override fun putFloat(key: String, value: Float): SharedPreferences.Editor = apply { staged[key] = value }
                        override fun putBoolean(key: String, value: Boolean): SharedPreferences.Editor = apply { staged[key] = value }
                        override fun remove(key: String): SharedPreferences.Editor = apply { removals.add(key) }
                        override fun clear(): SharedPreferences.Editor = apply { cleared = true }
                        override fun commit(): Boolean { apply(); return true }
                        override fun apply() {
                            if (cleared) data.clear()
                            removals.forEach { data.remove(it) }
                            for ((k, v) in staged) {
                                if (v == null) data.remove(k) else data[k] = v
                            }
                        }
                    }
                    override fun registerOnSharedPreferenceChangeListener(listener: SharedPreferences.OnSharedPreferenceChangeListener?) {}
                    override fun unregisterOnSharedPreferenceChangeListener(listener: SharedPreferences.OnSharedPreferenceChangeListener?) {}
                }
            }

            override fun deleteSharedPreferences(name: String): Boolean {
                return initial.remove(name) != null
            }
        }
    }

    @Test
    fun widgetSnapshotStoreUsesSecureStoreAndScrubsLegacyPlaintext() {
        val prefsBacking = linkedMapOf<String, MutableMap<String, Any?>>(
            "gr_widget_snapshot" to linkedMapOf("summary_v1" to """{"signedIn":true,"title":"LEGACY_PII"}"""),
        )
        val ctx = fakePrefsContext(prefsBacking)
        val store = InMemoryStore()

        val initialLoad = WidgetSnapshotStore.load(ctx, store)
        assertFalse(initialLoad.signedIn)
        assertFalse(prefsBacking.containsKey("gr_widget_snapshot"))

        val summary = WidgetSummary(
            signedIn = true,
            isRide = true,
            rideId = 88L,
            scheduledAt = "2026-12-06 08:30:00",
            badge = "Next Ride",
            title = "PJ State CMC",
            subtitle = "Confirmed",
            showReadyButton = true,
        )
        WidgetSnapshotStore.saveSummary(ctx, summary, store)
        assertFalse(prefsBacking.containsKey("gr_widget_snapshot"))
        assertNotNull(store.getString(WidgetSnapshotStore.SECURE_KEY_SUMMARY))

        val loaded = WidgetSnapshotStore.load(ctx, store)
        assertTrue(loaded.signedIn)
        assertEquals(88L, loaded.rideId)
        assertEquals("PJ State CMC", loaded.title)

        WidgetSnapshotStore.clear(ctx, store)
        assertNull(store.getString(WidgetSnapshotStore.SECURE_KEY_SUMMARY))
        assertFalse(WidgetSnapshotStore.load(ctx, store).signedIn)
    }

    @Test
    fun widgetSnapshotClearedOnAuthLockAndUnauthorized() {
        val store = InMemoryStore()
        val me = apiJson.decodeFromString(
            Me.serializer(),
            """{"id":5,"name":"Member","email":"m@example.org","status":"active"}""",
        )
        val dummyApi = Proxy.newProxyInstance(
            ApiService::class.java.classLoader,
            arrayOf(ApiService::class.java),
        ) { _, _, _ -> throw UnsupportedOperationException("network unused") } as ApiService
        val cookies = PersistentCookieJar(store)
        val auth = AuthRepository(
            apiProvider = ApiProvider(dummyApi),
            bus = AuthEventBus(),
            cookies = cookies,
            _state = MutableStateFlow(AuthState.SignedIn(me)),
            store = store,
        )

        store.putString(WidgetSnapshotStore.SECURE_KEY_SUMMARY, """{"signedIn":true,"title":"Secret"}""")
        auth.onLocked()
        assertNull(store.getString(WidgetSnapshotStore.SECURE_KEY_SUMMARY))

        store.putString(WidgetSnapshotStore.SECURE_KEY_SUMMARY, """{"signedIn":true,"title":"Secret"}""")
        auth.onUnauthorized()
        assertNull(store.getString(WidgetSnapshotStore.SECURE_KEY_SUMMARY))
    }

    @Test
    fun isSafeExternalWebUrlRejectsUnsafeSchemesAndMalformedUrls() {
        assertTrue(isSafeExternalWebUrl("https://graceride.saliminn.my/invite/abc", allowDebugHttp = false))
        assertTrue(isSafeExternalWebUrl("tg://resolve?domain=graceride", allowDebugHttp = false))
        assertFalse(isSafeExternalWebUrl("http://evil.example/phish", allowDebugHttp = false))
        assertFalse(isSafeExternalWebUrl("javascript:alert(1)", allowDebugHttp = true))
        assertFalse(isSafeExternalWebUrl("file:///etc/passwd", allowDebugHttp = true))
        assertFalse(isSafeExternalWebUrl("intent://scan/#Intent;end", allowDebugHttp = true))
        assertFalse(isSafeExternalWebUrl("https://user:pass@evil.example/", allowDebugHttp = true))
        assertFalse(isSafeExternalWebUrl("https://evil.example/\nSet-Cookie:a=b", allowDebugHttp = true))
        assertFalse(isSafeExternalWebUrl("tg://open?url=http://evil", allowDebugHttp = true))
        assertFalse(isSafeExternalWebUrl("", allowDebugHttp = true))
    }
}
