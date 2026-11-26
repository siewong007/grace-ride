package org.pjstatecm.graceride.domain

import org.pjstatecm.graceride.data.api.Area
import org.pjstatecm.graceride.data.api.AreaFareDto
import java.math.BigDecimal
import java.math.RoundingMode
import java.nio.charset.StandardCharsets

enum class FareSheetLayout {
    MATRIX,
    LIST,
}

enum class FareImportRowStatus {
    NEW,
    UPDATE,
    UNCHANGED,
    UNKNOWN_AREA,
}

data class ParsedFareRow(
    val areaAName: String,
    val areaBName: String,
    val amountCents: Long,
    val previousCents: Long? = null,
    val status: FareImportRowStatus,
    val row: Int,
    val col: Int,
)

enum class FareIssueKind {
    INVALID_AMOUNT,
    CONFLICT,
    INVALID_AREA_NAME,
    MISSING_AREA,
}

data class FareParseIssue(
    val kind: FareIssueKind,
    val row: Int,
    val col: Int,
    val route: String? = null,
    val raw: String? = null,
    val firstCents: Long? = null,
    val secondCents: Long? = null,
)

data class ParsedFareSheet(
    val layout: FareSheetLayout,
    val rows: List<ParsedFareRow>,
    val issues: List<FareParseIssue>,
    val unknownAreas: List<String>,
)

data class ParsedNamedSheet(
    val name: String,
    val parsed: ParsedFareSheet,
)

data class FareWorkbookParseResult(
    val fileName: String,
    val sheets: List<ParsedNamedSheet>,
)

sealed interface ParsedFareAmount {
    data object Blank : ParsedFareAmount
    data object Invalid : ParsedFareAmount
    data class Ok(val cents: Long) : ParsedFareAmount
}

/**
 * Pure-Kotlin CSV & XLSX fare matrix/list codec mirroring frontend `fareSheet.ts`.
 */
object FareSheetCodec {
    const val MAX_FARE_FILE_BYTES = 5 * 1024 * 1024
    const val MAX_AREA_NAME_CHARS = 80
    const val MAX_FARE_CENTS = 50_000L

    private val BLANK_MARKERS = setOf(
        "", "-", "—", "–", "n/a", "na", "none", "unset", "not set", "未设置", "无",
    )
    private val MATRIX_MIRROR_FORMULA = Regex(
        """^=?\s*IF\(\s*([A-Za-z]+\d+)\s*=\s*""\s*,\s*""\s*,\s*\1\s*\)$""",
        RegexOption.IGNORE_CASE,
    )
    private val CORNER_HINTS = setOf("from", "origin", "from \\ to", "from/to", "from -> to", "area", "出发", "起点")
    private val FORMULA_PREFIXES = charArrayOf('=', '+', '-', '@', '\t', '\r')

    fun normalizeAreaName(s: String): String = s.trim().replace(Regex("\\s+"), " ")

    fun areaLookupKey(s: String): String = normalizeAreaName(s).lowercase()

    fun isValidAreaName(s: String): Boolean {
        val n = normalizeAreaName(s)
        return n.isNotEmpty() && n.length <= MAX_AREA_NAME_CHARS && n.none { it.isISOControl() }
    }

    fun farePairLookupKey(nameA: String, nameB: String): Pair<String, String> {
        val a = areaLookupKey(nameA)
        val b = areaLookupKey(nameB)
        return if (a <= b) a to b else b to a
    }

    fun buildFaresCsv(
        areas: List<Area>,
        fares: List<AreaFareDto>,
        originLabel: String = "From \\ To",
    ): String {
        val open = areas.filter { it.active }.sortedBy { it.name.lowercase() }
        val byPair = indexFaresByPair(fares)
        val lines = mutableListOf<List<String>>()
        lines += listOf(originLabel) + open.map { csvSafe(it.name) }
        for (a in open) {
            val row = mutableListOf(csvSafe(a.name))
            for (b in open) {
                val key = farePairLookupKey(a.name, b.name)
                val cents = byPair[key]
                row += if (cents != null) formatRmPlain(cents) else ""
            }
            lines += row
        }
        return "\uFEFF" + lines.joinToString("\n") { cols -> cols.joinToString(",") { csvField(it) } }
    }

    fun buildFaresXlsx(
        areas: List<Area>,
        fares: List<AreaFareDto>,
        sheetName: String = "Area Fares",
        originLabel: String = "From \\ To",
    ): ByteArray {
        val open = areas.filter { it.active }.sortedBy { it.name.lowercase() }
        val byPair = indexFaresByPair(fares)
        val grid = mutableListOf<List<XlsxWriteCell>>()
        grid += listOf(XlsxWriteCell(XlsxCellValue.Text(originLabel), XlsxWriteStyle.HEADER)) +
            open.map { XlsxWriteCell(XlsxCellValue.Text(it.name), XlsxWriteStyle.HEADER) }

        open.forEachIndexed { rIdx, rowArea ->
            val rowCells = mutableListOf<XlsxWriteCell>()
            rowCells += XlsxWriteCell(XlsxCellValue.Text(rowArea.name), XlsxWriteStyle.HEADER)
            open.forEachIndexed { cIdx, colArea ->
                val pairKey = farePairLookupKey(rowArea.name, colArea.name)
                val existingCents = byPair[pairKey]
                rowCells += when {
                    cIdx < rIdx -> {
                        val upperRef = "${XlsxCodec.colIndexToLetters(rIdx + 1)}${cIdx + 2}"
                        XlsxWriteCell(
                            value = XlsxCellValue.Formula("""IF($upperRef="","",$upperRef)"""),
                            style = XlsxWriteStyle.MIRROR,
                        )
                    }
                    cIdx == rIdx -> {
                        if (existingCents != null) {
                            XlsxWriteCell(XlsxCellValue.Number(existingCents / 100.0), XlsxWriteStyle.DIAGONAL)
                        } else {
                            XlsxWriteCell(XlsxCellValue.Blank, XlsxWriteStyle.DIAGONAL)
                        }
                    }
                    else -> {
                        if (existingCents != null) {
                            XlsxWriteCell(XlsxCellValue.Number(existingCents / 100.0), XlsxWriteStyle.FARE)
                        } else {
                            XlsxWriteCell(XlsxCellValue.Blank, XlsxWriteStyle.FARE)
                        }
                    }
                }
            }
            grid += rowCells
        }
        return XlsxCodec.buildWorkbook(sheetName = sheetName, rows = grid)
    }

    fun parseCsv(rawText: String): List<List<String>> {
        val cleaned = rawText.removePrefix("\uFEFF")
        if (cleaned.isBlank()) return emptyList()
        val delimiter = detectDelimiter(cleaned)
        val rows = mutableListOf<List<String>>()
        var row = mutableListOf<String>()
        val field = StringBuilder()
        var inQuotes = false
        var i = 0
        while (i < cleaned.length) {
            val ch = cleaned[i]
            if (inQuotes) {
                if (ch == '"') {
                    if (i + 1 < cleaned.length && cleaned[i + 1] == '"') {
                        field.append('"')
                        i++
                    } else {
                        inQuotes = false
                    }
                } else {
                    field.append(ch)
                }
            } else {
                when (ch) {
                    '"' -> inQuotes = true
                    delimiter -> {
                        row += field.toString()
                        field.clear()
                    }
                    '\r' -> {
                        if (i + 1 < cleaned.length && cleaned[i + 1] == '\n') i++
                        row += field.toString()
                        rows += row
                        row = mutableListOf()
                        field.clear()
                    }
                    '\n' -> {
                        row += field.toString()
                        rows += row
                        row = mutableListOf()
                        field.clear()
                    }
                    else -> field.append(ch)
                }
            }
            i++
        }
        if (field.isNotEmpty() || row.isNotEmpty()) {
            row += field.toString()
            rows += row
        }
        return rows
    }

    fun parseFareAmountRm(raw: String): ParsedFareAmount {
        var text = raw.trim()
        if (text.lowercase() in BLANK_MARKERS) return ParsedFareAmount.Blank
        val upper = text.uppercase()
        when {
            upper.startsWith("MYR") -> text = text.substring(3).trim()
            upper.startsWith("RM") -> text = text.substring(2).trim()
        }
        if (text.lowercase() in BLANK_MARKERS) return ParsedFareAmount.Blank
        if (Regex("""^\d{1,3}(,\d{3})+(\.\d+)?$""").matches(text)) {
            text = text.replace(",", "")
        }
        if (text.startsWith("-")) return ParsedFareAmount.Invalid
        val bd = runCatching { BigDecimal(text) }.getOrNull() ?: return ParsedFareAmount.Invalid
        if (bd < BigDecimal.ZERO) return ParsedFareAmount.Invalid
        val centsBd = bd.movePointRight(2)
        // Reject more than 2 significant decimal places (allow negligible IEEE-754 float tails).
        val roundedCents = centsBd.setScale(0, RoundingMode.HALF_UP)
        if ((centsBd - roundedCents).abs() > BigDecimal("0.0001")) {
            return ParsedFareAmount.Invalid
        }
        val cents = runCatching { roundedCents.longValueExact() }.getOrNull() ?: return ParsedFareAmount.Invalid
        if (cents !in 0L..MAX_FARE_CENTS) return ParsedFareAmount.Invalid
        return ParsedFareAmount.Ok(cents)
    }

    fun readFareBytes(
        bytes: ByteArray,
        fileName: String,
        allAreas: List<Area>,
        existingFares: List<AreaFareDto>,
    ): FareWorkbookParseResult {
        if (bytes.size > MAX_FARE_FILE_BYTES) {
            throw XlsxException(XlsxErrorCode.TOO_LARGE, "Fare file exceeds 5 MiB limit")
        }
        if (XlsxCodec.isLegacyXls(bytes)) {
            throw XlsxException(XlsxErrorCode.LEGACY_XLS, "Legacy .xls files are not supported")
        }
        val ext = fileName.substringAfterLast('.', "").lowercase()
        val isZipHeader = bytes.size >= 2 && bytes[0] == 0x50.toByte() && bytes[1] == 0x4B.toByte()
        if (ext == "xlsx" || isZipHeader) {
            val wb = XlsxCodec.readWorkbook(bytes)
            val sheets = wb.sheets.map { s ->
                ParsedNamedSheet(
                    name = s.name,
                    parsed = parseFareGrid(s.rows, allAreas, existingFares),
                )
            }
            return FareWorkbookParseResult(fileName = fileName, sheets = sheets)
        }
        val text = bytes.toString(StandardCharsets.UTF_8)
        val csvRows = parseCsv(text)
        val grid = csvRows.mapIndexed { rIdx, cols ->
            cols.mapIndexed { cIdx, value -> XlsxSheetCell(row = rIdx, col = cIdx, value = value) }
        }
        val parsed = parseFareGrid(grid, allAreas, existingFares)
        return FareWorkbookParseResult(
            fileName = fileName,
            sheets = listOf(ParsedNamedSheet(name = fileName.ifBlank { "CSV" }, parsed = parsed)),
        )
    }

    fun parseFareGrid(
        grid: List<List<XlsxSheetCell>>,
        allAreas: List<Area>,
        existingFares: List<AreaFareDto>,
    ): ParsedFareSheet {
        val nonEmptyRows = grid.mapNotNull { row ->
            val trimmedLast = row.indexOfLast { it.value.isNotBlank() || it.formula != null }
            if (trimmedLast < 0) null else row.subList(0, trimmedLast + 1)
        }
        if (nonEmptyRows.isEmpty()) {
            return ParsedFareSheet(FareSheetLayout.LIST, emptyList(), emptyList(), emptyList())
        }

        val knownAreas = LinkedHashMap<String, Area>()
        allAreas.forEach { area -> knownAreas.putIfAbsent(areaLookupKey(area.name), area) }
        val existingByPair = indexFaresByPair(existingFares)

        // Skip optional single-cell banner/title row (e.g. "2026 Fares") before table header,
        // without misclassifying a 1-area matrix header row (["", "UT"]) as a banner.
        val r0NonBlank = nonEmptyRows.first().count { it.value.isNotBlank() || it.formula != null }
        val startIdx = if (
            nonEmptyRows.size >= 2 &&
            r0NonBlank <= 1 &&
            nonEmptyRows[1].size >= 2 &&
            (nonEmptyRows[0].size == 1 || nonEmptyRows[1].size > nonEmptyRows[0].size)
        ) {
            1
        } else {
            0
        }
        val bodyRows = nonEmptyRows.drop(startIdx)
        val layout = detectLayout(bodyRows)

        return if (layout == FareSheetLayout.MATRIX) {
            parseMatrix(bodyRows, knownAreas, existingByPair)
        } else {
            parseList(bodyRows, knownAreas, existingByPair)
        }
    }

    private fun detectLayout(rows: List<List<XlsxSheetCell>>): FareSheetLayout {
        if (rows.isEmpty()) return FareSheetLayout.LIST
        val first = rows[0]
        if (first.size < 2) return FareSheetLayout.LIST

        val col0Text = normalizeAreaName(first[0].value).lowercase()

        // Evaluate 3-column list header first: words like "Origin" can otherwise collide with matrix corner hints.
        if (first.size >= 3 && rows.size >= 2) {
            val c1 = normalizeAreaName(first[1].value).lowercase()
            val c2 = normalizeAreaName(first[2].value).lowercase()
            val isExplicitListHeader =
                (col0Text.contains("from") || col0Text.contains("origin") || col0Text.contains("出发") || col0Text.contains("起点") || col0Text.contains("area a")) &&
                    (c1.contains("to") || c1.contains("dest") || c1.contains("到达") || c1.contains("终点") || c1.contains("area b")) &&
                    (c2.contains("fare") || c2.contains("amount") || c2.contains("rm") || c2.contains("myr") || c2.contains("车费") || c2.contains("金额") || c2.contains("价格"))
            if (isExplicitListHeader) return FareSheetLayout.LIST
        }

        val hasMatrixCorner = col0Text.isEmpty() || CORNER_HINTS.any { col0Text == it || col0Text.contains("\\") }
        if (hasMatrixCorner) return FareSheetLayout.MATRIX

        // 2-area matrix (3 columns: [label, A, B]) vs headerless 3-column list ([A, B, amount]):
        // In a matrix, row 1 col 1 holds a numeric fare/blank diagonal, whereas in a 3-col list row 0 col 2 holds an amount.
        val row0Col2IsAmount = first.getOrNull(2)?.value?.let { parseFareAmountRm(it) is ParsedFareAmount.Ok } == true
        if (row0Col2IsAmount) return FareSheetLayout.LIST

        val colHeaders = first.drop(1).map { areaLookupKey(it.value) }.filter { it.isNotEmpty() }.toSet()
        val rowHeaders = rows.drop(1).mapNotNull { it.firstOrNull()?.value?.let(::areaLookupKey) }.filter { it.isNotEmpty() }.toSet()
        if (colHeaders.isNotEmpty() && colHeaders.intersect(rowHeaders).isNotEmpty()) {
            return FareSheetLayout.MATRIX
        }
        return if (first.size > 3) FareSheetLayout.MATRIX else FareSheetLayout.LIST
    }

    private fun parseMatrix(
        rows: List<List<XlsxSheetCell>>,
        knownAreas: Map<String, Area>,
        existingByPair: Map<Pair<String, String>, Long>,
    ): ParsedFareSheet {
        val issues = mutableListOf<FareParseIssue>()
        val headerRow = rows.firstOrNull().orEmpty()
        if (headerRow.size < 2) {
            return ParsedFareSheet(FareSheetLayout.MATRIX, emptyList(), emptyList(), emptyList())
        }

        val colAreas = mutableMapOf<Int, String>()
        for (c in 1 until headerRow.size) {
            val cell = headerRow[c]
            val rawName = cell.value.removePrefix("'").trim()
            val name = normalizeAreaName(rawName)
            if (name.isEmpty()) {
                issues += FareParseIssue(FareIssueKind.MISSING_AREA, row = cell.row + 1, col = cell.col + 1)
            } else if (!isValidAreaName(name)) {
                issues += FareParseIssue(FareIssueKind.INVALID_AREA_NAME, row = cell.row + 1, col = cell.col + 1, raw = cell.value)
            } else {
                colAreas[c] = name
            }
        }

        val collector = FareRowCollector(knownAreas, existingByPair)
        for (rIdx in 1 until rows.size) {
            val row = rows[rIdx]
            val headerCell = row.firstOrNull() ?: continue
            val rowAreaRaw = headerCell.value.removePrefix("'").trim()
            val rowArea = normalizeAreaName(rowAreaRaw)
            if (rowArea.isEmpty()) {
                val hasValues = row.drop(1).any { it.value.isNotBlank() }
                if (hasValues) {
                    issues += FareParseIssue(FareIssueKind.MISSING_AREA, row = headerCell.row + 1, col = headerCell.col + 1)
                }
                continue
            }
            if (!isValidAreaName(rowArea)) {
                issues += FareParseIssue(FareIssueKind.INVALID_AREA_NAME, row = headerCell.row + 1, col = headerCell.col + 1, raw = headerCell.value)
                continue
            }

            for (c in 1 until headerRow.size) {
                val colArea = colAreas[c] ?: continue
                val cell = row.getOrNull(c) ?: continue
                if (isMirrorFormulaFor(cell, headerRow[c], headerCell)) continue
                when (val amount = parseFareAmountRm(cell.value)) {
                    ParsedFareAmount.Blank -> {}
                    ParsedFareAmount.Invalid -> {
                        issues += FareParseIssue(
                            kind = FareIssueKind.INVALID_AMOUNT,
                            row = cell.row + 1,
                            col = cell.col + 1,
                            route = "$rowArea ↔ $colArea",
                            raw = cell.value,
                        )
                    }
                    is ParsedFareAmount.Ok -> {
                        collector.record(
                            areaAInput = rowArea,
                            areaBInput = colArea,
                            amountCents = amount.cents,
                            row1Based = cell.row + 1,
                            col1Based = cell.col + 1,
                            issues = issues,
                        )
                    }
                }
            }
        }
        return collector.buildResult(FareSheetLayout.MATRIX, issues)
    }

    private fun isMirrorFormulaFor(
        cell: XlsxSheetCell,
        colHeader: XlsxSheetCell,
        rowHeader: XlsxSheetCell,
    ): Boolean {
        val formula = cell.formula?.trim() ?: return false
        val m = MATRIX_MIRROR_FORMULA.matchEntire(formula) ?: return false
        val targetRef = XlsxCodec.parseCellRef(m.groupValues[1]) ?: return false
        val originRow = colHeader.row + 1
        val originCol = rowHeader.col + 1
        val dr = cell.row - originRow
        val dc = cell.col - originCol
        return dc in 0 until dr && targetRef.first == originRow + dc && targetRef.second == originCol + dr
    }

    private fun parseList(
        rows: List<List<XlsxSheetCell>>,
        knownAreas: Map<String, Area>,
        existingByPair: Map<Pair<String, String>, Long>,
    ): ParsedFareSheet {
        val issues = mutableListOf<FareParseIssue>()
        if (rows.isEmpty()) {
            return ParsedFareSheet(FareSheetLayout.LIST, emptyList(), emptyList(), emptyList())
        }
        val first = rows[0]
        val hasHeaderRow = first.getOrNull(2)?.value?.let {
            parseFareAmountRm(it) is ParsedFareAmount.Invalid ||
                normalizeAreaName(it).lowercase().let { lower ->
                    lower.contains("fare") || lower.contains("amount") || lower.contains("rm") || lower.contains("车费")
                }
        } == true
        val dataRows = if (hasHeaderRow) rows.drop(1) else rows
        val collector = FareRowCollector(knownAreas, existingByPair)

        for (row in dataRows) {
            val c0 = row.getOrNull(0)
            val c1 = row.getOrNull(1)
            val c2 = row.getOrNull(2)
            val rawA = normalizeAreaName(c0?.value?.removePrefix("'").orEmpty())
            val rawB = normalizeAreaName(c1?.value?.removePrefix("'").orEmpty())
            val rawAmt = c2?.value.orEmpty()
            if (rawA.isEmpty() && rawB.isEmpty() && rawAmt.isBlank()) continue

            if (rawA.isEmpty()) {
                issues += FareParseIssue(FareIssueKind.MISSING_AREA, row = (c0?.row ?: 0) + 1, col = 1)
                continue
            }
            if (!isValidAreaName(rawA)) {
                issues += FareParseIssue(FareIssueKind.INVALID_AREA_NAME, row = (c0?.row ?: 0) + 1, col = 1, raw = c0?.value)
                continue
            }
            if (rawB.isEmpty()) {
                issues += FareParseIssue(FareIssueKind.MISSING_AREA, row = (c1?.row ?: 0) + 1, col = 2)
                continue
            }
            if (!isValidAreaName(rawB)) {
                issues += FareParseIssue(FareIssueKind.INVALID_AREA_NAME, row = (c1?.row ?: 0) + 1, col = 2, raw = c1?.value)
                continue
            }

            when (val amount = parseFareAmountRm(rawAmt)) {
                ParsedFareAmount.Blank -> {}
                ParsedFareAmount.Invalid -> {
                    issues += FareParseIssue(
                        kind = FareIssueKind.INVALID_AMOUNT,
                        row = (c2?.row ?: 0) + 1,
                        col = 3,
                        route = "$rawA ↔ $rawB",
                        raw = rawAmt,
                    )
                }
                is ParsedFareAmount.Ok -> {
                    collector.record(
                        areaAInput = rawA,
                        areaBInput = rawB,
                        amountCents = amount.cents,
                        row1Based = (c2?.row ?: 0) + 1,
                        col1Based = 3,
                        issues = issues,
                    )
                }
            }
        }
        return collector.buildResult(FareSheetLayout.LIST, issues)
    }

    private class FareRowCollector(
        private val knownAreas: Map<String, Area>,
        private val existingByPair: Map<Pair<String, String>, Long>,
    ) {
        private val seenByPair = LinkedHashMap<Pair<String, String>, ParsedFareRow>()
        private val unknownOrder = LinkedHashMap<String, String>()

        fun record(
            areaAInput: String,
            areaBInput: String,
            amountCents: Long,
            row1Based: Int,
            col1Based: Int,
            issues: MutableList<FareParseIssue>,
        ) {
            val keyA = areaLookupKey(areaAInput)
            val keyB = areaLookupKey(areaBInput)
            val knownA = knownAreas[keyA]
            val knownB = knownAreas[keyB]
            val canonicalA = knownA?.name ?: normalizeAreaName(areaAInput)
            val canonicalB = knownB?.name ?: normalizeAreaName(areaBInput)
            if (knownA == null) unknownOrder.putIfAbsent(keyA, canonicalA)
            if (knownB == null) unknownOrder.putIfAbsent(keyB, canonicalB)

            val pairKey = farePairLookupKey(canonicalA, canonicalB)
            val existingRow = seenByPair[pairKey]
            if (existingRow != null) {
                if (existingRow.amountCents != amountCents) {
                    val alreadyConflicted = issues.any {
                        it.kind == FareIssueKind.CONFLICT && it.row == row1Based && it.col == col1Based
                    }
                    if (!alreadyConflicted) {
                        issues += FareParseIssue(
                            kind = FareIssueKind.CONFLICT,
                            row = row1Based,
                            col = col1Based,
                            route = "$canonicalA ↔ $canonicalB",
                            firstCents = existingRow.amountCents,
                            secondCents = amountCents,
                        )
                    }
                }
                return
            }

            val prev = existingByPair[pairKey]
            val status = when {
                knownA == null || knownB == null -> FareImportRowStatus.UNKNOWN_AREA
                prev == null -> FareImportRowStatus.NEW
                prev != amountCents -> FareImportRowStatus.UPDATE
                else -> FareImportRowStatus.UNCHANGED
            }
            val (outA, outB) = if (areaLookupKey(canonicalA) <= areaLookupKey(canonicalB)) {
                canonicalA to canonicalB
            } else {
                canonicalB to canonicalA
            }
            seenByPair[pairKey] = ParsedFareRow(
                areaAName = outA,
                areaBName = outB,
                amountCents = amountCents,
                previousCents = prev,
                status = status,
                row = row1Based,
                col = col1Based,
            )
        }

        fun buildResult(layout: FareSheetLayout, issues: List<FareParseIssue>): ParsedFareSheet =
            ParsedFareSheet(
                layout = layout,
                rows = seenByPair.values.toList(),
                issues = issues,
                unknownAreas = unknownOrder.values.toList(),
            )
    }

    private fun indexFaresByPair(fares: List<AreaFareDto>): Map<Pair<String, String>, Long> {
        val map = LinkedHashMap<Pair<String, String>, Long>()
        for (f in fares) {
            map[farePairLookupKey(f.areaA.name, f.areaB.name)] = f.amountCents
        }
        return map
    }

    private fun detectDelimiter(text: String): Char {
        val sampleLines = text.lineSequence().filter { it.isNotBlank() }.take(5).toList()
        val candidates = charArrayOf(',', ';', '\t')
        var best = ','
        var bestScore = -1
        for (delim in candidates) {
            val counts = sampleLines.map { line ->
                var c = 0
                var inQ = false
                for (ch in line) {
                    if (ch == '"') inQ = !inQ
                    else if (!inQ && ch == delim) c++
                }
                c
            }
            val total = counts.sum()
            if (total > bestScore) {
                bestScore = total
                best = delim
            }
        }
        return if (bestScore > 0) best else ','
    }

    private fun formatRmPlain(cents: Long): String =
        BigDecimal.valueOf(cents, 2).toPlainString()

    private fun csvSafe(value: String): String {
        val trimmed = value.trimStart()
        return if (trimmed.isNotEmpty() && trimmed[0] in FORMULA_PREFIXES) "'$value" else value
    }

    private fun csvField(value: String): String {
        val needsQuotes = value.any { it == ',' || it == '"' || it == '\n' || it == '\r' }
        return if (needsQuotes) "\"${value.replace("\"", "\"\"")}\"" else value
    }
}
