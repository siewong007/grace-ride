package org.pjstatecm.graceride.domain

import org.w3c.dom.Document
import org.w3c.dom.Element
import org.w3c.dom.Node
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.nio.charset.StandardCharsets
import java.util.zip.ZipEntry
import java.util.zip.ZipInputStream
import java.util.zip.ZipOutputStream
import javax.xml.parsers.DocumentBuilderFactory

enum class XlsxErrorCode {
    LEGACY_XLS,
    INVALID_XLSX,
    TOO_LARGE,
}

class XlsxException(
    val code: XlsxErrorCode,
    message: String,
    cause: Throwable? = null,
) : Exception(message, cause)

data class XlsxSheetCell(
    val row: Int,
    val col: Int,
    val value: String,
    val formula: String? = null,
)

data class XlsxSheet(
    val name: String,
    val rows: List<List<XlsxSheetCell>>,
)

data class XlsxWorkbook(
    val sheets: List<XlsxSheet>,
)

enum class XlsxWriteStyle {
    DEFAULT,
    HEADER,
    FARE,
    DIAGONAL,
    MIRROR,
}

sealed interface XlsxCellValue {
    data class Text(val text: String) : XlsxCellValue
    data class Number(val numericValue: Double) : XlsxCellValue
    data class Formula(val formula: String, val cachedValue: Double? = null) : XlsxCellValue
    data object Blank : XlsxCellValue
}

data class XlsxWriteCell(
    val value: XlsxCellValue = XlsxCellValue.Blank,
    val style: XlsxWriteStyle = XlsxWriteStyle.DEFAULT,
)

/**
 * Pure-Kotlin `.xlsx` reader/writer built on `java.util.zip` and JDK XML DOM.
 * Supports DEFLATE & stored ZIP entries, bit-3 data descriptors (handled natively by [ZipInputStream]),
 * sharedStrings rich text while skipping `<rPh>` ruby guides, inline strings, and formula mirrors.
 */
object XlsxCodec {
    private const val MAX_FILE_BYTES = 5 * 1024 * 1024
    private const val MAX_UNCOMPRESSED_BYTES = 32 * 1024 * 1024
    private val OLE2_MAGIC = byteArrayOf(
        0xD0.toByte(),
        0xCF.toByte(),
        0x11.toByte(),
        0xE0.toByte(),
        0xA1.toByte(),
        0xB1.toByte(),
        0x1A.toByte(),
        0xE1.toByte(),
    )
    private val CELL_REF_REGEX = Regex("^\\$?([A-Za-z]+)\\$?(\\d+)$")
    private val DOCTYPE_OR_ENTITY = Regex("<!\\s*(DOCTYPE|ENTITY)\\b", RegexOption.IGNORE_CASE)

    fun isLegacyXls(bytes: ByteArray): Boolean =
        bytes.size >= OLE2_MAGIC.size && OLE2_MAGIC.indices.all { bytes[it] == OLE2_MAGIC[it] }

    fun colIndexToLetters(col: Int): String {
        require(col >= 0) { "Column index must be non-negative: $col" }
        var n = col
        val sb = StringBuilder()
        while (true) {
            val rem = n % 26
            sb.append(('A'.code + rem).toChar())
            n = (n / 26) - 1
            if (n < 0) break
        }
        return sb.reverse().toString()
    }

    fun lettersToColIndex(letters: String): Int {
        val clean = letters.trim().uppercase()
        require(clean.isNotEmpty() && clean.all { it in 'A'..'Z' }) { "Invalid column letters: $letters" }
        var acc = 0
        for (ch in clean) {
            acc = acc * 26 + (ch - 'A' + 1)
        }
        return acc - 1
    }

    fun parseCellRef(ref: String): Pair<Int, Int>? {
        val m = CELL_REF_REGEX.matchEntire(ref.trim()) ?: return null
        val col = lettersToColIndex(m.groupValues[1])
        val row = m.groupValues[2].toIntOrNull()?.minus(1) ?: return null
        if (row < 0) return null
        return row to col
    }

    fun assertSafeXlsxXml(text: String) {
        if (DOCTYPE_OR_ENTITY.containsMatchIn(text)) {
            throw XlsxException(XlsxErrorCode.INVALID_XLSX, "Unsafe XML declaration in workbook")
        }
    }

    fun readWorkbook(bytes: ByteArray): XlsxWorkbook {
        if (bytes.size > MAX_FILE_BYTES) {
            throw XlsxException(XlsxErrorCode.TOO_LARGE, "File exceeds 5 MiB limit")
        }
        if (isLegacyXls(bytes)) {
            throw XlsxException(XlsxErrorCode.LEGACY_XLS, "Legacy .xls BIFF format is not supported")
        }
        if (bytes.size < 4 || bytes[0] != 0x50.toByte() || bytes[1] != 0x4B.toByte()) {
            throw XlsxException(XlsxErrorCode.INVALID_XLSX, "Not a valid ZIP archive")
        }

        val entries = unzipEntries(bytes)
        val rootRels = entries.findText("_rels/.rels")?.let { parseRelationships(it) }.orEmpty()
        val workbookPath = normalizeZipPath(
            rootRels.firstOrNull { it.type.endsWith("/officeDocument") }?.target ?: "xl/workbook.xml",
        )
        val workbookXml = entries.findText(workbookPath)
            ?: entries.findText("xl/workbook.xml")
            ?: throw XlsxException(XlsxErrorCode.INVALID_XLSX, "Missing xl/workbook.xml")

        val wbDoc = parseXml(workbookXml)
        val wbRelsPath = relsPathFor(workbookPath)
        val wbRels = entries.findText(wbRelsPath)?.let { parseRelationships(it) }.orEmpty()
        val relById = wbRels.associateBy { it.id }

        val sharedStringsTarget = wbRels.firstOrNull { it.type.endsWith("/sharedStrings") }?.target
        val sharedStringsPath = if (sharedStringsTarget != null) {
            resolveTarget(workbookPath, sharedStringsTarget)
        } else {
            "xl/sharedStrings.xml"
        }
        val sharedStrings = entries.findText(sharedStringsPath)?.let { parseSharedStrings(it) }.orEmpty()

        val sheetNodes = wbDoc.elementsByLocalName("sheet")
        if (sheetNodes.isEmpty()) {
            throw XlsxException(XlsxErrorCode.INVALID_XLSX, "Workbook contains no sheets")
        }

        val sheets = mutableListOf<XlsxSheet>()
        sheetNodes.forEachIndexed { idx, sheetEl ->
            val sheetName = sheetEl.getAttribute("name").ifBlank { "Sheet ${idx + 1}" }
            val rId = sheetEl.attrNsOrLocal("r:id", "id")
            val target = relById[rId]?.target ?: "worksheets/sheet${idx + 1}.xml"
            val resolvedPath = resolveTarget(workbookPath, target)
            val sheetXml = entries.findText(resolvedPath)
                ?: throw XlsxException(XlsxErrorCode.INVALID_XLSX, "Missing worksheet $resolvedPath")
            sheets += XlsxSheet(name = sheetName, rows = parseWorksheet(sheetXml, sharedStrings))
        }

        return XlsxWorkbook(sheets = sheets)
    }

    fun buildWorkbook(
        sheetName: String,
        rows: List<List<XlsxWriteCell>>,
    ): ByteArray {
        val safeSheetName = sanitizeSheetName(sheetName)
        val baos = ByteArrayOutputStream()
        ZipOutputStream(baos).use { zos ->
            writeZipEntry(zos, "[Content_Types].xml", CONTENT_TYPES_XML)
            writeZipEntry(zos, "_rels/.rels", ROOT_RELS_XML)
            writeZipEntry(zos, "xl/workbook.xml", buildWorkbookXml(safeSheetName))
            writeZipEntry(zos, "xl/_rels/workbook.xml.rels", WORKBOOK_RELS_XML)
            writeZipEntry(zos, "xl/styles.xml", STYLES_XML)
            writeZipEntry(zos, "xl/worksheets/sheet1.xml", buildSheetXml(rows))
        }
        return baos.toByteArray()
    }

    private fun writeZipEntry(zos: ZipOutputStream, name: String, content: String) {
        zos.putNextEntry(ZipEntry(name))
        zos.write(content.toByteArray(StandardCharsets.UTF_8))
        zos.closeEntry()
    }

    private fun buildWorkbookXml(sheetName: String): String =
        """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>""" +
            """<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" """ +
            """xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">""" +
            """<sheets><sheet name="${escapeXmlAttr(sheetName)}" sheetId="1" r:id="rId1"/></sheets></workbook>"""

    private fun buildSheetXml(rows: List<List<XlsxWriteCell>>): String {
        val sb = StringBuilder()
        sb.append("""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>""")
        sb.append("""<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>""")
        rows.forEachIndexed { rIdx, row ->
            val rNum = rIdx + 1
            sb.append("""<row r="$rNum">""")
            row.forEachIndexed { cIdx, cell ->
                val ref = "${colIndexToLetters(cIdx)}$rNum"
                val styleIdx = when (cell.style) {
                    XlsxWriteStyle.DEFAULT -> 0
                    XlsxWriteStyle.HEADER -> 1
                    XlsxWriteStyle.FARE -> 2
                    XlsxWriteStyle.DIAGONAL -> 3
                    XlsxWriteStyle.MIRROR -> 4
                }
                when (val v = cell.value) {
                    is XlsxCellValue.Text -> {
                        sb.append("""<c r="$ref" s="$styleIdx" t="inlineStr"><is><t xml:space="preserve">${escapeXmlText(v.text)}</t></is></c>""")
                    }
                    is XlsxCellValue.Number -> {
                        val numStr = java.math.BigDecimal.valueOf(v.numericValue)
                            .setScale(2, java.math.RoundingMode.HALF_UP)
                            .toPlainString()
                        sb.append("""<c r="$ref" s="$styleIdx"><v>$numStr</v></c>""")
                    }
                    is XlsxCellValue.Formula -> {
                        sb.append("""<c r="$ref" s="$styleIdx"><f>${escapeXmlText(v.formula)}</f>""")
                        if (v.cachedValue != null) {
                            val numStr = java.math.BigDecimal.valueOf(v.cachedValue)
                                .setScale(2, java.math.RoundingMode.HALF_UP)
                                .toPlainString()
                            sb.append("<v>$numStr</v>")
                        }
                        sb.append("</c>")
                    }
                    XlsxCellValue.Blank -> {
                        if (styleIdx != 0) sb.append("""<c r="$ref" s="$styleIdx"/>""")
                    }
                }
            }
            sb.append("</row>")
        }
        sb.append("</sheetData></worksheet>")
        return sb.toString()
    }

    private fun unzipEntries(bytes: ByteArray): Map<String, ByteArray> {
        val map = LinkedHashMap<String, ByteArray>()
        var totalUncompressed = 0L
        try {
            ZipInputStream(ByteArrayInputStream(bytes)).use { zis ->
                val buf = ByteArray(8192)
                var entry = zis.nextEntry
                while (entry != null) {
                    if (!entry.isDirectory) {
                        val out = ByteArrayOutputStream()
                        while (true) {
                            val n = zis.read(buf)
                            if (n <= 0) break
                            totalUncompressed += n
                            if (totalUncompressed > MAX_UNCOMPRESSED_BYTES) {
                                throw XlsxException(XlsxErrorCode.TOO_LARGE, "Workbook uncompressed content too large")
                            }
                            out.write(buf, 0, n)
                        }
                        map[normalizeZipPath(entry.name)] = out.toByteArray()
                    }
                    zis.closeEntry()
                    entry = zis.nextEntry
                }
            }
        } catch (e: XlsxException) {
            throw e
        } catch (e: Exception) {
            throw XlsxException(XlsxErrorCode.INVALID_XLSX, "Corrupt ZIP container", e)
        }
        if (map.isEmpty()) {
            throw XlsxException(XlsxErrorCode.INVALID_XLSX, "Empty XLSX archive")
        }
        return map
    }

    private fun Map<String, ByteArray>.findText(path: String): String? {
        val norm = normalizeZipPath(path)
        val raw = this[norm] ?: entries.firstOrNull { it.key.equals(norm, ignoreCase = true) }?.value ?: return null
        return raw.toString(StandardCharsets.UTF_8)
    }

    private data class ZipRel(val id: String, val type: String, val target: String)

    private fun parseRelationships(xml: String): List<ZipRel> {
        val doc = parseXml(xml)
        return doc.elementsByLocalName("Relationship").mapNotNull { el ->
            val id = el.getAttribute("Id")
            val target = el.getAttribute("Target")
            if (id.isBlank() || target.isBlank()) null
            else ZipRel(id = id, type = el.getAttribute("Type"), target = target)
        }
    }

    private fun parseSharedStrings(xml: String): List<String> {
        val doc = parseXml(xml)
        return doc.elementsByLocalName("si").map { si -> collectTextSkippingRPh(si) }
    }

    private fun parseWorksheet(xml: String, sharedStrings: List<String>): List<List<XlsxSheetCell>> {
        val doc = parseXml(xml)
        val sparseRows = LinkedHashMap<Int, MutableMap<Int, XlsxSheetCell>>()
        var maxRow = -1
        var maxCol = -1
        var nextSequentialRow = 0

        for (rowEl in doc.elementsByLocalName("row")) {
            val rowNum1Based = rowEl.getAttribute("r").toIntOrNull()
            val rowIdx = if (rowNum1Based != null && rowNum1Based > 0) rowNum1Based - 1 else nextSequentialRow
            nextSequentialRow = rowIdx + 1
            var nextSequentialCol = 0

            val cells = rowEl.directChildElements("c")
            for (cEl in cells) {
                val parsedRef = cEl.getAttribute("r").takeIf { it.isNotBlank() }?.let { parseCellRef(it) }
                val r = parsedRef?.first ?: rowIdx
                val c = parsedRef?.second ?: nextSequentialCol
                nextSequentialCol = c + 1
                if (r > maxRow) maxRow = r
                if (c > maxCol) maxCol = c

                val typeAttr = cEl.getAttribute("t")
                val formulaEl = cEl.directChildElements("f").firstOrNull()
                val formula = formulaEl?.textContent?.trim()?.takeIf { it.isNotEmpty() }
                val valueEl = cEl.directChildElements("v").firstOrNull()
                val rawValue = valueEl?.textContent.orEmpty()

                val cellValue = when (typeAttr) {
                    "s" -> {
                        val idx = rawValue.trim().toIntOrNull()
                        if (idx != null && idx in sharedStrings.indices) sharedStrings[idx] else ""
                    }
                    "inlineStr" -> {
                        cEl.directChildElements("is").firstOrNull()?.let { collectTextSkippingRPh(it) }.orEmpty()
                    }
                    "str" -> rawValue
                    "b" -> if (rawValue.trim() == "1") "TRUE" else "FALSE"
                    else -> normalizeNumericText(rawValue)
                }

                val rowMap = sparseRows.getOrPut(r) { LinkedHashMap() }
                rowMap[c] = XlsxSheetCell(row = r, col = c, value = cellValue, formula = formula)
            }
        }

        if (maxRow < 0 || maxCol < 0) return emptyList()
        return (0..maxRow).map { r ->
            val rowMap = sparseRows[r]
            (0..maxCol).map { c ->
                rowMap?.get(c) ?: XlsxSheetCell(row = r, col = c, value = "")
            }
        }
    }

    private fun normalizeNumericText(raw: String): String {
        val trimmed = raw.trim()
        if (trimmed.isEmpty()) return ""
        val dbl = trimmed.toDoubleOrNull() ?: return trimmed
        if (!dbl.isFinite()) return trimmed
        // Round common IEEE-754 representation artifacts (e.g. 15.100000000000001 -> 15.1).
        val rounded = kotlin.math.round(dbl * 1_000_000.0) / 1_000_000.0
        val bd = java.math.BigDecimal.valueOf(rounded).stripTrailingZeros()
        return bd.toPlainString()
    }

    private fun collectTextSkippingRPh(node: Node): String {
        val sb = StringBuilder()
        fun walk(cur: Node) {
            if (cur.nodeType == Node.ELEMENT_NODE) {
                val el = cur as Element
                val name = el.localName ?: el.tagName
                if (name == "rPh") return
                if (name == "t") {
                    sb.append(el.textContent.orEmpty())
                    return
                }
            }
            val children = cur.childNodes
            for (i in 0 until children.length) {
                walk(children.item(i))
            }
        }
        walk(node)
        return sb.toString()
    }

    private fun parseXml(xml: String): Document {
        assertSafeXlsxXml(xml)
        return try {
            val dbf = DocumentBuilderFactory.newInstance().apply {
                isNamespaceAware = true
                runCatching { setFeature("http://apache.org/xml/features/disallow-doctype-decl", true) }
                runCatching { setFeature("http://xml.org/sax/features/external-general-entities", false) }
                runCatching { setFeature("http://xml.org/sax/features/external-parameter-entities", false) }
                runCatching { setFeature("http://apache.org/xml/features/nonvalidating/load-external-dtd", false) }
                isXIncludeAware = false
                isExpandEntityReferences = false
            }
            dbf.newDocumentBuilder().parse(ByteArrayInputStream(xml.toByteArray(StandardCharsets.UTF_8)))
        } catch (e: XlsxException) {
            throw e
        } catch (e: Exception) {
            throw XlsxException(XlsxErrorCode.INVALID_XLSX, "Malformed workbook XML", e)
        }
    }

    private fun Document.elementsByLocalName(local: String): List<Element> {
        val list = getElementsByTagNameNS("*", local)
        val out = ArrayList<Element>(list.length)
        for (i in 0 until list.length) {
            val n = list.item(i)
            if (n is Element) out += n
        }
        if (out.isEmpty()) {
            val fallback = getElementsByTagName(local)
            for (i in 0 until fallback.length) {
                val n = fallback.item(i)
                if (n is Element) out += n
            }
        }
        return out
    }

    private fun Element.directChildElements(local: String): List<Element> {
        val out = mutableListOf<Element>()
        val children = childNodes
        for (i in 0 until children.length) {
            val n = children.item(i)
            if (n is Element && (n.localName ?: n.tagName) == local) out += n
        }
        return out
    }

    private fun Element.attrNsOrLocal(qualified: String, local: String): String {
        val byQualified = getAttribute(qualified)
        if (byQualified.isNotBlank()) return byQualified
        val attrs = attributes
        for (i in 0 until attrs.length) {
            val attr = attrs.item(i)
            if ((attr.localName ?: attr.nodeName) == local) return attr.nodeValue.orEmpty()
        }
        return ""
    }

    private fun normalizeZipPath(path: String): String =
        path.replace('\\', '/').trimStart('/').let { clean ->
            val parts = mutableListOf<String>()
            for (seg in clean.split('/')) {
                when (seg) {
                    "", "." -> {}
                    ".." -> if (parts.isNotEmpty()) parts.removeAt(parts.lastIndex)
                    else -> parts += seg
                }
            }
            parts.joinToString("/")
        }

    private fun relsPathFor(partPath: String): String {
        val norm = normalizeZipPath(partPath)
        val slash = norm.lastIndexOf('/')
        return if (slash < 0) "_rels/$norm.rels"
        else "${norm.substring(0, slash)}/_rels/${norm.substring(slash + 1)}.rels"
    }

    private fun resolveTarget(baseFilePath: String, target: String): String {
        val cleaned = target.replace('\\', '/')
        if (cleaned.startsWith("/")) return normalizeZipPath(cleaned)
        val dir = baseFilePath.substringBeforeLast('/', "")
        return if (dir.isEmpty()) normalizeZipPath(cleaned) else normalizeZipPath("$dir/$cleaned")
    }

    private fun sanitizeSheetName(name: String): String {
        val stripped = name.replace(Regex("[\\\\/?*\\[\\]:]"), " ").trim()
        return (stripped.ifEmpty { "Sheet1" }).take(31)
    }

    private fun escapeXmlAttr(value: String): String =
        escapeXmlText(value).replace("\"", "&quot;").replace("'", "&apos;")

    private fun escapeXmlText(value: String): String =
        value
            .replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")

    private const val CONTENT_TYPES_XML =
        """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>""" +
            """<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">""" +
            """<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>""" +
            """<Default Extension="xml" ContentType="application/xml"/>""" +
            """<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>""" +
            """<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>""" +
            """<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>""" +
            """</Types>"""

    private const val ROOT_RELS_XML =
        """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>""" +
            """<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">""" +
            """<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>""" +
            """</Relationships>"""

    private const val WORKBOOK_RELS_XML =
        """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>""" +
            """<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">""" +
            """<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>""" +
            """<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>""" +
            """</Relationships>"""

    private const val STYLES_XML =
        """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>""" +
            """<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">""" +
            """<numFmts count="1"><numFmt numFmtId="164" formatCode="0.00"/></numFmts>""" +
            """<fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="11"/><name val="Calibri"/></font></fonts>""" +
            """<fills count="4"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill><fill><patternFill patternType="solid"><fgColor rgb="FFF3F4F6"/></patternFill></fill><fill><patternFill patternType="solid"><fgColor rgb="FFEEF2FF"/></patternFill></fill></fills>""" +
            """<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>""" +
            """<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>""" +
            """<cellXfs count="5">""" +
            """<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>""" +
            """<xf numFmtId="0" fontId="1" fillId="2" borderId="0" xfId="0" applyFont="1" applyFill="1"/>""" +
            """<xf numFmtId="164" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>""" +
            """<xf numFmtId="164" fontId="0" fillId="3" borderId="0" xfId="0" applyNumberFormat="1" applyFill="1"/>""" +
            """<xf numFmtId="164" fontId="0" fillId="2" borderId="0" xfId="0" applyNumberFormat="1" applyFill="1"/>""" +
            """</cellXfs></styleSheet>"""
}
