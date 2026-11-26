package org.pjstatecm.graceride

import app.cash.turbine.test
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import kotlinx.coroutines.withTimeout
import okhttp3.mockwebserver.Dispatcher
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import okhttp3.mockwebserver.RecordedRequest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.pjstatecm.graceride.data.api.Area
import org.pjstatecm.graceride.data.api.AreaFareDto
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.RideArea
import org.pjstatecm.graceride.data.api.SettlementInReq
import org.pjstatecm.graceride.data.api.SettlementItemInReq
import org.pjstatecm.graceride.data.api.SettlementParticipantInReq
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.FareImportRowStatus
import org.pjstatecm.graceride.domain.FareIssueKind
import org.pjstatecm.graceride.domain.FareSheetCodec
import org.pjstatecm.graceride.domain.FareSheetLayout
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.domain.ParsedFareAmount
import org.pjstatecm.graceride.domain.XlsxCodec
import org.pjstatecm.graceride.domain.XlsxErrorCode
import org.pjstatecm.graceride.domain.XlsxException
import org.pjstatecm.graceride.domain.XlsxSheetCell
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.screens.admin.AdminSettingsViewModel
import org.pjstatecm.graceride.ui.screens.help.GuideStepDraft
import org.pjstatecm.graceride.ui.screens.help.HelpGuideEditorUiState
import org.pjstatecm.graceride.ui.screens.reimbursements.ReimbursementFormViewModel
import org.pjstatecm.graceride.ui.screens.reimbursements.ReimbursementsViewModel
import org.pjstatecm.graceride.ui.screens.reports.AdminReportsViewModel
import org.pjstatecm.graceride.ui.screens.reports.ReportViewModel
import org.pjstatecm.graceride.ui.screens.settlements.SettlementFormViewModel
import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.nio.charset.StandardCharsets
import java.time.YearMonth
import java.util.zip.CRC32
import java.util.zip.Deflater
import kotlin.time.Duration.Companion.seconds

@OptIn(ExperimentalCoroutinesApi::class)
class FinanceHelpAndAdminParityTest {

    private lateinit var server: MockWebServer
    private lateinit var container: AppContainer

    private val areas = listOf(
        Area(id = 1L, name = "UT", active = true),
        Area(id = 2L, name = "KK", active = true),
        Area(id = 3L, name = "Pacific/RM", active = true),
        Area(id = 4L, name = "八打灵", active = true),
        Area(id = 5L, name = "Old Town", active = false),
    )

    private val existingFares = listOf(
        AreaFareDto(
            id = 10L,
            areaA = RideArea(1L, "UT"),
            areaB = RideArea(2L, "KK"),
            amountCents = 1500L,
            updatedAt = "",
        ),
    )

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
        runCatching { server.shutdown() }
        Dispatchers.resetMain()
    }

    @Test
    fun moneyEqualSplitDistributesRemainderToLeadingSharesAndParseCentsValidates() {
        assertEquals(listOf(34L, 33L, 33L), Money.equalSplit(100L, 3))
        assertEquals(emptyList<Long>(), Money.equalSplit(100L, 0))
        assertEquals(1550L, Money.parseCents("15.5"))
        assertEquals(1500L, Money.parseCents("RM 15"))
        assertEquals(123450L, Money.parseCents("1,234.50"))
        assertEquals(null, Money.parseCents("-5"))
        assertEquals(null, Money.parseCents(""))
        assertEquals(null, Money.parseCents("abc"))
    }

    @Test
    fun fareSheetAndXlsxRoundTripPreservesSpecialNamesCjkAndMirrorFormulas() {
        val xlsxBytes = FareSheetCodec.buildFaresXlsx(
            areas = areas,
            fares = existingFares,
            sheetName = "Area Fares",
            originLabel = "From \\ To",
        )
        val wb = XlsxCodec.readWorkbook(xlsxBytes)
        assertEquals(1, wb.sheets.size)
        assertEquals("Area Fares", wb.sheets[0].name)

        val grid = wb.sheets[0].rows
        val mirrorCells = grid.flatten().filter { !it.formula.isNullOrBlank() }
        assertTrue(mirrorCells.isNotEmpty())
        assertTrue(mirrorCells.all { Regex("""^IF\([A-Z]+\d+="",\s*"",\s*[A-Z]+\d+\)$""").matches(it.formula!!) })

        val parsed = FareSheetCodec.parseFareGrid(grid, areas, existingFares)
        assertEquals(FareSheetLayout.MATRIX, parsed.layout)
        assertTrue(parsed.issues.isEmpty())
        assertTrue(parsed.unknownAreas.isEmpty())
        assertEquals(1, parsed.rows.size)
        assertEquals(FareImportRowStatus.UNCHANGED, parsed.rows[0].status)
        assertEquals(1500L, parsed.rows[0].amountCents)

        val headerNames = grid[0].map { it.value }
        assertTrue("Pacific/RM" in headerNames)
        assertTrue("八打灵" in headerNames)
        assertTrue("Old Town" !in headerNames)
    }

    @Test
    fun xlsxReadsBit3DataDescriptorsRichTextSkipRPhAndIeeeFloats() {
        val zipBytes = buildDeflatedZipWithDataDescriptor(
            listOf(
                "[Content_Types].xml" to """<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/></Types>""".toByteArray(),
                "_rels/.rels" to """<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="/xl/workbook.xml"/></Relationships>""".toByteArray(),
                "xl/workbook.xml" to """<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Notes" sheetId="1" r:id="rId1"/><sheet name="Rates" sheetId="2" r:id="rId2"/></sheets></workbook>""".toByteArray(),
                "xl/_rels/workbook.xml.rels" to """<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="SharedStrings.xml"/></Relationships>""".toByteArray(),
                "xl/SharedStrings.xml" to """<?xml version="1.0" encoding="UTF-8"?><sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="2" uniqueCount="2"><si><r><t>八打</t></r><r><t>灵</t></r><rPh sb="0" eb="3"><t>IGNORE</t></rPh></si><si><t>UT</t></si></sst>""".toByteArray(),
                "xl/worksheets/sheet1.xml" to """<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Header only</t></is></c></row></sheetData></worksheet>""".toByteArray(),
                "xl/worksheets/sheet2.xml" to (
                    """<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>""" +
                        """<row r="1"><c r="A1" t="inlineStr"><is><t>2026 Fares</t></is></c></row>""" +
                        """<row r="2"><c r="A2" t="inlineStr"><is><t>From</t></is></c><c r="B2" t="s"><v>0</v></c><c r="C2" t="s"><v>1</v></c></row>""" +
                        """<row r="3"><c r="A3" t="s"><v>0</v></c><c r="B3"><v>5</v></c><c r="C3"><v>15.100000000000001</v></c></row>""" +
                        """<row r="4"><c r="A4" t="s"><v>1</v></c><c r="B4"><f>IF(C3="","",C3)</f><v>99</v></c><c r="C4"><f>10+5</f><v>15</v></c></row>""" +
                        """</sheetData></worksheet>"""
                    ).toByteArray(),
            ),
        )
        val res = FareSheetCodec.readFareBytes(zipBytes, "rates.xlsx", areas, existingFares)
        assertEquals(2, res.sheets.size)
        assertEquals("Rates", res.sheets[1].name)
        val parsed = res.sheets[1].parsed
        assertTrue(parsed.issues.isEmpty())
        assertEquals(3, parsed.rows.size)
        val pjToUt = parsed.rows.first {
            (it.areaAName == "八打灵" && it.areaBName == "UT") || (it.areaAName == "UT" && it.areaBName == "八打灵")
        }
        assertEquals(1510L, pjToUt.amountCents)
        val utWithin = parsed.rows.first { it.areaAName == "UT" && it.areaBName == "UT" }
        assertEquals(1500L, utWithin.amountCents)
    }

    @Test
    fun xlsxRejectsLegacyXlsCorruptZipAndOversizedWithExplicitCodes() {
        val ole2 = byteArrayOf(0xD0.toByte(), 0xCF.toByte(), 0x11.toByte(), 0xE0.toByte(), 0xA1.toByte(), 0xB1.toByte(), 0x1A.toByte(), 0xE1.toByte(), 0, 0)
        val xlsErr = runCatching { FareSheetCodec.readFareBytes(ole2, "old.xls", areas, emptyList()) }.exceptionOrNull() as XlsxException
        assertEquals(XlsxErrorCode.LEGACY_XLS, xlsErr.code)

        val badZip = byteArrayOf(0x50, 0x4B, 0x03, 0x04, 1, 2, 3, 4)
        val zipErr = runCatching { FareSheetCodec.readFareBytes(badZip, "broken.xlsx", areas, emptyList()) }.exceptionOrNull() as XlsxException
        assertEquals(XlsxErrorCode.INVALID_XLSX, zipErr.code)

        val huge = ByteArray(FareSheetCodec.MAX_FARE_FILE_BYTES + 1)
        val sizeErr = runCatching { FareSheetCodec.readFareBytes(huge, "huge.csv", areas, emptyList()) }.exceptionOrNull() as XlsxException
        assertEquals(XlsxErrorCode.TOO_LARGE, sizeErr.code)
    }

    @Test
    fun csvAndAutoDetectionHandleListsMatricesConflictsAndBounds() {
        val customAreas = areas + listOf(
            Area(id = 6L, name = "Bandar, Utama", active = true),
            Area(id = 7L, name = "=FormulaArea", active = true),
        )
        val csv = FareSheetCodec.buildFaresCsv(customAreas, existingFares, "From \\ To")
        assertTrue(csv.startsWith("\uFEFF"))
        assertTrue("'=FormulaArea" in csv)
        assertTrue("\"Bandar, Utama\"" in csv)
        assertTrue("Bandar, Utama" in FareSheetCodec.parseCsv(csv)[0])

        val semiCsv = listOf(
            "Origin;Destination;Fare (MYR)",
            "UT;KK;RM 18.00",
            "KK;Pacific/RM;MYR 12.5",
            "UT;UT;0",
            "KK;KK;—",
            "UT;Pacific/RM;N/A",
        ).joinToString("\n")
        val parsedSemi = FareSheetCodec.parseFareGrid(
            FareSheetCodec.parseCsv(semiCsv).mapIndexed { r, row -> row.mapIndexed { c, v -> XlsxSheetCell(r, c, v) } },
            areas,
            existingFares,
        )
        assertEquals(FareSheetLayout.LIST, parsedSemi.layout)
        assertTrue(parsedSemi.issues.isEmpty())
        assertEquals(3, parsedSemi.rows.size)
        assertEquals(1800L, parsedSemi.rows[0].amountCents)
        assertEquals(FareImportRowStatus.UPDATE, parsedSemi.rows[0].status)
        assertEquals(1250L, parsedSemi.rows[1].amountCents)
        assertEquals(FareImportRowStatus.NEW, parsedSemi.rows[1].status)

        assertEquals(ParsedFareAmount.Ok(50_000L), FareSheetCodec.parseFareAmountRm("500"))
        assertEquals(ParsedFareAmount.Invalid, FareSheetCodec.parseFareAmountRm("500.01"))
        assertEquals(ParsedFareAmount.Invalid, FareSheetCodec.parseFareAmountRm("-5"))
        assertEquals(ParsedFareAmount.Blank, FareSheetCodec.parseFareAmountRm("Not set"))

        val oneArea = FareSheetCodec.parseFareGrid(
            listOf(listOf(XlsxSheetCell(0, 0, ""), XlsxSheetCell(0, 1, "UT")), listOf(XlsxSheetCell(1, 0, "UT"), XlsxSheetCell(1, 1, "5"))),
            areas,
            emptyList(),
        )
        assertEquals(FareSheetLayout.MATRIX, oneArea.layout)
        assertEquals(500L, oneArea.rows.single().amountCents)

        val twoArea = FareSheetCodec.parseFareGrid(
            listOf(listOf("From \\ To", "UT", "KK"), listOf("UT", "5", "15"), listOf("KK", "", "6")).mapIndexed { r, row ->
                row.mapIndexed { c, v -> XlsxSheetCell(r, c, v) }
            },
            areas,
            existingFares,
        )
        assertEquals(FareSheetLayout.MATRIX, twoArea.layout)
        assertEquals(3, twoArea.rows.size)

        val zhList = FareSheetCodec.parseFareGrid(
            listOf(listOf("出发地区", "到达地区", "车费 (RM)"), listOf("UT", "KK", "15")).mapIndexed { r, row ->
                row.mapIndexed { c, v -> XlsxSheetCell(r, c, v) }
            },
            areas,
            existingFares,
        )
        assertEquals(FareSheetLayout.LIST, zhList.layout)
        assertEquals(FareImportRowStatus.UNCHANGED, zhList.rows.single().status)

        val conflictGrid = listOf(
            listOf("From \\ To", "UT", "KK", "Subang"),
            listOf("UT", "5", "15", "20"),
            listOf("KK", "18", "bad", ""),
        ).mapIndexed { r, row -> row.mapIndexed { c, v -> XlsxSheetCell(r, c, v) } }
        val conflictParsed = FareSheetCodec.parseFareGrid(conflictGrid, areas, existingFares)
        assertEquals(listOf(FareIssueKind.CONFLICT, FareIssueKind.INVALID_AMOUNT), conflictParsed.issues.map { it.kind })
        assertEquals(listOf("Subang"), conflictParsed.unknownAreas)
        assertEquals(FareImportRowStatus.UNKNOWN_AREA, conflictParsed.rows.first { it.areaAName == "Subang" || it.areaBName == "Subang" }.status)
    }

    @Test
    fun adminGroupUpdateSerializesExplicitNullParentAndFinanceHelpWork() = runBlocking {
        var capturedGroupPutBody = ""
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val p = request.path.orEmpty()
                return when {
                    p == "/api/admin/groups/4" && request.method == "PUT" -> {
                        capturedGroupPutBody = request.body.readUtf8()
                        MockResponse().setBody("""{"ok":true}""")
                    }
                    p.startsWith("/api/settlements") && request.method == "GET" ->
                        MockResponse().setBody("""{"items":[],"total":0,"page":1,"page_size":25,"total_pages":1}""")
                    p == "/api/events/9/settlements" && request.method == "POST" ->
                        MockResponse().setBody("""{"id":55}""")
                    p == "/api/help/counts" ->
                        MockResponse().setBody("""{"open":2,"answered":1,"closed":3}""")
                    p.startsWith("/api/reports/monthly") &&
                        request.requestUrl?.queryParameter("month") == "2026-11" &&
                        request.requestUrl?.queryParameter("type") == "all" &&
                        request.requestUrl?.queryParameter("range") == "monthly" ->
                        MockResponse().setBody("""{"user":{"id":1,"name":"Tester"},"month":"2026-11","rides":{"total":4,"completed":4}}""")
                    else -> MockResponse().setResponseCode(404)
                }
            }
        }

        val updRes = container.admin.updateGroup(4L, "Youth", "Desc", parentId = null)
        assertTrue(updRes.isSuccess)
        assertTrue(""""parent_id":null""" in capturedGroupPutBody)

        val setId = container.finance.createSettlement(
            9L,
            SettlementInReq(
                title = "Camp Fuel",
                items = listOf(SettlementItemInReq("Fuel", 2000L)),
                participants = listOf(SettlementParticipantInReq(2L)),
                payee_name = "Finance",
                payee_account = "123456",
            ),
        )
        assertEquals(55L, setId.id)

        val counts = container.help.counts()
        assertEquals(2L, counts.open)
        assertEquals(1L, counts.answered)

        val monthly = container.help.monthlyReport(month = "2026-11", type = "all", range = "monthly")
        assertEquals("2026-11", monthly.month)
    }

    private suspend fun <T> StateFlow<T>.awaitUntil(predicate: (T) -> Boolean): T =
        withTimeout(10.seconds) {
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

    @Test
    fun settlementFormParticipantSelectionAndCustomSplitValidation() = runBlocking {
        var createCalls = 0
        var publishCalls = 0
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val p = request.path.orEmpty()
                return when {
                    p.startsWith("/api/events") && p.endsWith("/signups") ->
                        MockResponse().setBody(
                            """[
                                {"id":1,"user_id":10,"name":"Alice","status":"going","attended":1},
                                {"id":2,"user_id":20,"name":"Bob","status":"going","attended":0},
                                {"id":3,"user_id":30,"name":"Charlie","status":"going","attended":0}
                            ]""",
                        )
                    p.startsWith("/api/events") && request.method == "GET" ->
                        MockResponse().setBody("""[{"id":9,"title":"Camp","starts_at":"2026-12-01T09:00:00+08:00"}]""")
                    p == "/api/events/9/settlements" && request.method == "POST" -> {
                        createCalls += 1
                        MockResponse().setBody("""{"id":77}""")
                    }
                    p == "/api/settlements/77/publish" && request.method == "POST" -> {
                        publishCalls += 1
                        MockResponse().setBody("""{"ok":true}""")
                    }
                    else -> MockResponse().setResponseCode(404)
                }
            }
        }

        val vm = SettlementFormViewModel(container, initialSettlementId = null, initialEventId = null)
        val loaded = vm.state.awaitUntil { !it.loading }
        assertEquals(listOf(10L), loaded.selectedParticipants.map { it.userId })

        vm.selectParticipants(setOf(20L, 30L), selected = true)
        assertEquals(setOf(10L, 20L, 30L), vm.state.value.selectedParticipants.map { it.userId }.toSet())

        vm.selectParticipants(setOf(30L), selected = false)
        assertEquals(setOf(10L, 20L), vm.state.value.selectedParticipants.map { it.userId }.toSet())

        vm.updateItem(key = 1L, label = "Fuel", amount = "50.00")
        vm.update { it.copy(title = "Camp Fuel", splitMode = "custom") }
        vm.setParticipantAmount(10L, "20.00")
        vm.setParticipantAmount(20L, "20.00")
        assertEquals(4000L, vm.state.value.customTotalCents)
        assertFalse(vm.state.value.isCustomBalanced)

        vm.submit(publishAfterSave = true, fallbackErr = "Missing", unbalancedErr = "Unbalanced custom split")
        assertEquals("Unbalanced custom split", vm.state.value.error)
        assertEquals(0, createCalls)
        assertEquals(0, publishCalls)

        vm.setParticipantAmount(20L, "30.00")
        assertEquals(5000L, vm.state.value.customTotalCents)
        assertTrue(vm.state.value.isCustomBalanced)

        vm.submit(publishAfterSave = true, fallbackErr = "Missing", unbalancedErr = "Unbalanced custom split")
        val saved = vm.state.awaitUntil { it.savedId == 77L }
        assertEquals(77L, saved.savedId)
        assertEquals(1, createCalls)
        assertEquals(1, publishCalls)
    }

    @Test
    fun reimbursementOcrCandidateExtractionDedupesAndFallsBackOnError() = runBlocking {
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val p = request.path.orEmpty()
                return when {
                    p.startsWith("/api/events") -> MockResponse().setBody("""[]""")
                    p.startsWith("/api/rides") && p.contains("scope=upcoming") -> MockResponse().setBody("""[]""")
                    p.startsWith("/api/rides") -> MockResponse().setBody("""{"items":[],"next_before":null}""")
                    p == "/api/files/10/extract-ref" ->
                        MockResponse().setBody("""{"ref":"  REF-100 ","candidates":["REF-100","   ","REF-200 ","REF-200"]}""")
                    p == "/api/files/99/extract-ref" -> MockResponse().setResponseCode(500)
                    else -> MockResponse().setResponseCode(404)
                }
            }
        }

        val vm = ReimbursementFormViewModel(container, editId = null)
        vm.state.awaitUntil { !it.loading }

        assertEquals(listOf("REF-100", "REF-200"), vm.fetchOcrCandidates(10L))
        assertEquals(emptyList<String>(), vm.fetchOcrCandidates(99L))
    }

    @Test
    fun reimbursementsViewModelCachesUnfilteredSummaryAndHidesWhenPaginated() = runBlocking {
        var totalPages = 1
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val p = request.path.orEmpty()
                if (!p.startsWith("/api/reimbursements")) return MockResponse().setResponseCode(404)
                val status = request.requestUrl?.queryParameter("status").orEmpty()
                val body = when {
                    totalPages > 1 ->
                        """{"items":[{"id":1,"user":{"id":1,"name":"A"},"amount_cents":1200,"status":"pending"}],"total":26,"page":1,"page_size":25,"total_pages":2}"""
                    status == "paid" ->
                        """{"items":[{"id":4,"user":{"id":1,"name":"A"},"amount_cents":2500,"status":"paid"}],"total":1,"page":1,"page_size":25,"total_pages":1}"""
                    else ->
                        """{"items":[
                            {"id":1,"user":{"id":1,"name":"A"},"amount_cents":1200,"status":"pending"},
                            {"id":2,"user":{"id":1,"name":"A"},"amount_cents":800,"status":"returned"},
                            {"id":3,"user":{"id":1,"name":"A"},"amount_cents":1500,"status":"approved"},
                            {"id":4,"user":{"id":1,"name":"A"},"amount_cents":2500,"status":"paid"}
                        ],"total":4,"page":1,"page_size":25,"total_pages":1}"""
                }
                return MockResponse().setBody(body)
            }
        }

        val vm = ReimbursementsViewModel(container)
        val initial = vm.state.awaitUntil { !it.loading && it.items.size == 4 }
        assertTrue(initial.summaryLoaded)
        assertEquals(2000L, initial.summaryPendingCents)
        assertEquals(1500L, initial.summaryApprovedCents)
        assertEquals(2500L, initial.summaryPaidCents)

        vm.setStatusFilter("paid")
        val filtered = vm.state.awaitUntil { it.statusFilter == "paid" && !it.refreshing && it.items.size == 1 }
        assertTrue(filtered.summaryLoaded)
        assertEquals(2000L, filtered.summaryPendingCents)
        assertEquals(1500L, filtered.summaryApprovedCents)
        assertEquals(2500L, filtered.summaryPaidCents)

        totalPages = 2
        vm.setStatusFilter("all")
        val paginated = vm.state.awaitUntil { it.statusFilter == "all" && it.totalPages == 2 && !it.refreshing }
        assertFalse(paginated.summaryLoaded)
    }

    @Test
    fun helpGuideEditorCompletenessGapsTrackEnAndZhSeparately() {
        val empty = HelpGuideEditorUiState(loading = false)
        assertEquals(3, empty.enGaps)
        assertEquals(3, empty.zhGaps)
        assertEquals(5, empty.completenessGaps)

        val enOnly = empty.copy(
            titleEn = "Driver Guide",
            summaryEn = "How to accept rides",
            steps = listOf(GuideStepDraft(localKey = 1L, titleEn = "Open Drive")),
        )
        assertEquals(0, enOnly.enGaps)
        assertEquals(3, enOnly.zhGaps)
        assertEquals(3, enOnly.completenessGaps)

        val complete = enOnly.copy(
            titleZh = "司机指南",
            summaryZh = "如何接单",
            steps = listOf(GuideStepDraft(localKey = 1L, titleEn = "Open Drive", titleZh = "打开接单页")),
        )
        assertEquals(0, complete.enGaps)
        assertEquals(0, complete.zhGaps)
        assertEquals(0, complete.completenessGaps)
    }

    @Test
    fun adminSettingsSetAllGroupMembersReplacesSelection() = runBlocking {
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse = MockResponse().setResponseCode(404)
        }
        val vm = AdminSettingsViewModel(container)
        vm.state.awaitUntil { !it.loading }

        vm.setAllGroupMembers(setOf(1L, 3L))
        assertEquals(setOf(1L, 3L), vm.state.value.editingGroupMembersIds)

        vm.setAllGroupMembers(emptySet())
        assertEquals(emptySet<Long>(), vm.state.value.editingGroupMembersIds)
    }

    @Test
    fun routesPermitsReimbursementDetailForReviewersAndPayers() {
        val route = AppRoute.ReimbursementDetail(5L)
        val claimant = Me(id = 1L, name = "Claimant", email = "c@x.org", permissions = listOf("reimbursements.view"))
        val reviewer = Me(id = 2L, name = "Reviewer", email = "r@x.org", permissions = listOf("admin_reimb_review.view"))
        val payer = Me(id = 3L, name = "Payer", email = "p@x.org", permissions = listOf("admin_reimb_pay.view"))
        val rider = Me(id = 4L, name = "Rider", email = "u@x.org", permissions = listOf("rides.view"))

        assertTrue(Routes.isPermitted(route, claimant))
        assertTrue(Routes.isPermitted(route, reviewer))
        assertTrue(Routes.isPermitted(route, payer))
        assertFalse(Routes.isPermitted(route, rider))
        assertFalse(Routes.isPermitted(AppRoute.Reimbursements, reviewer))
    }

    @Test
    fun reportViewModelsStepMonthByRangeGranularity() = runBlocking {
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val p = request.path.orEmpty()
                return when {
                    p.startsWith("/api/admin/reports/capabilities") ->
                        MockResponse().setBody("""{"overview":false,"events":false,"demand":false,"finance":false,"member_reports_global":false}""")
                    p.startsWith("/api/reports/monthly") -> {
                        val m = request.requestUrl?.queryParameter("month") ?: "2026-11"
                        MockResponse().setBody("""{"user":{"id":1,"name":"Tester"},"month":"$m","rides":{"total":1,"completed":1}}""")
                    }
                    else -> MockResponse().setResponseCode(404)
                }
            }
        }

        val vm = ReportViewModel(container)
        val initial = vm.state.awaitUntil { !it.loading }
        val baseYm = YearMonth.parse(initial.month)

        vm.setRange("quarterly")
        vm.shiftMonth(3L)
        val afterQuarter = vm.state.awaitUntil { !it.refreshing && it.report?.month == baseYm.plusMonths(3).toString() }
        assertEquals(baseYm.plusMonths(3).toString(), afterQuarter.month)

        vm.setRange("yearly")
        vm.shiftMonth(-12L)
        val afterYear = vm.state.awaitUntil { !it.refreshing && it.report?.month == baseYm.minusMonths(9).toString() }
        assertEquals(baseYm.minusMonths(9).toString(), afterYear.month)

        val adminVm = AdminReportsViewModel(container)
        val adminInit = adminVm.state.awaitUntil { !it.loading }
        val adminBase = YearMonth.parse(adminInit.month)

        adminVm.setRange("quarterly")
        adminVm.shiftMonth(-3L)
        val adminQuarter = adminVm.state.awaitUntil { !it.refreshing && it.memberReport?.month == adminBase.minusMonths(3).toString() }
        assertEquals(adminBase.minusMonths(3).toString(), adminQuarter.month)

        adminVm.setRange("yearly")
        adminVm.shiftMonth(12L)
        val adminYear = adminVm.state.awaitUntil { !it.refreshing && it.memberReport?.month == adminBase.plusMonths(9).toString() }
        assertEquals(adminBase.plusMonths(9).toString(), adminYear.month)
    }

    private fun buildDeflatedZipWithDataDescriptor(files: List<Pair<String, ByteArray>>): ByteArray {
        val localOut = ByteArrayOutputStream()
        val cdOut = ByteArrayOutputStream()
        var offset = 0
        for ((name, raw) in files) {
            val nameBytes = name.toByteArray(StandardCharsets.UTF_8)
            val crc32 = CRC32().apply { update(raw) }.value
            val compressed = deflateRaw(raw)

            val lh = ByteBuffer.allocate(30 + nameBytes.size).order(ByteOrder.LITTLE_ENDIAN).apply {
                putInt(0x04034B50)
                putShort(20)
                putShort(0x0808.toShort()) // bit 3 + UTF-8
                putShort(8)
                putShort(0)
                putShort(0)
                putInt(0)
                putInt(0)
                putInt(0)
                putShort(nameBytes.size.toShort())
                putShort(0)
                put(nameBytes)
            }.array()
            val dd = ByteBuffer.allocate(16).order(ByteOrder.LITTLE_ENDIAN).apply {
                putInt(0x08074B50)
                putInt(crc32.toInt())
                putInt(compressed.size)
                putInt(raw.size)
            }.array()
            val cd = ByteBuffer.allocate(46 + nameBytes.size).order(ByteOrder.LITTLE_ENDIAN).apply {
                putInt(0x02014B50)
                putShort(20)
                putShort(20)
                putShort(0x0808.toShort())
                putShort(8)
                putShort(0)
                putShort(0)
                putInt(crc32.toInt())
                putInt(compressed.size)
                putInt(raw.size)
                putShort(nameBytes.size.toShort())
                putShort(0)
                putShort(0)
                putShort(0)
                putShort(0)
                putInt(0)
                putInt(offset)
                put(nameBytes)
            }.array()
            localOut.write(lh)
            localOut.write(compressed)
            localOut.write(dd)
            cdOut.write(cd)
            offset += lh.size + compressed.size + dd.size
        }
        val cdBytes = cdOut.toByteArray()
        val eocd = ByteBuffer.allocate(22).order(ByteOrder.LITTLE_ENDIAN).apply {
            putInt(0x06054B50)
            putShort(0)
            putShort(0)
            putShort(files.size.toShort())
            putShort(files.size.toShort())
            putInt(cdBytes.size)
            putInt(offset)
            putShort(0)
        }.array()
        return localOut.toByteArray() + cdBytes + eocd
    }

    private fun deflateRaw(input: ByteArray): ByteArray {
        val deflater = Deflater(Deflater.DEFAULT_COMPRESSION, true)
        deflater.setInput(input)
        deflater.finish()
        val out = ByteArrayOutputStream()
        val buf = ByteArray(1024)
        while (!deflater.finished()) {
            val n = deflater.deflate(buf)
            out.write(buf, 0, n)
        }
        deflater.end()
        return out.toByteArray()
    }
}
