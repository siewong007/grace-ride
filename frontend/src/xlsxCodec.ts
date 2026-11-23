/**
 * Minimal zero-dependency OOXML (.xlsx) reader and writer using standard browser
 * Web Streams (`DecompressionStream('deflate-raw')`) and `DOMParser`.
 */

export type XlsxErrorCode = 'LEGACY_XLS' | 'INVALID_XLSX' | 'TOO_LARGE';

export class XlsxError extends Error {
  code: XlsxErrorCode;

  constructor(code: XlsxErrorCode, message?: string) {
    super(message ?? code);
    this.name = 'XlsxError';
    this.code = code;
  }
}

export interface XlsxSheetCell {
  row: number;
  col: number;
  value: string;
  formula?: string;
}

export interface XlsxSheet {
  name: string;
  rows: XlsxSheetCell[][];
}

export interface XlsxWorkbook {
  sheets: XlsxSheet[];
}

export type XlsxWriteStyle = 'default' | 'header' | 'fare' | 'diagonal' | 'mirror';

export interface XlsxWriteCell {
  value?: string | number;
  formula?: string;
  style?: XlsxWriteStyle;
}

const OLE2_MAGIC = [0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1];
const MAX_ZIP_ENTRIES = 500;
const MAX_ENTRY_BYTES = 10 * 1024 * 1024;
const MAX_TOTAL_BYTES = 20 * 1024 * 1024;
const MAX_SHEET_ROWS = 5_000;
const MAX_SHEET_COLS = 500;

const utf8Decoder = new TextDecoder('utf-8');
const utf8Encoder = new TextEncoder();

export function colIndexToLetters(col: number): string {
  let n = col + 1;
  let out = '';
  while (n > 0) {
    const rem = (n - 1) % 26;
    out = String.fromCharCode(65 + rem) + out;
    n = Math.floor((n - 1) / 26);
  }
  return out;
}

export function lettersToColIndex(letters: string): number {
  let col = 0;
  const upper = letters.toUpperCase();
  for (let i = 0; i < upper.length; i++) {
    const code = upper.charCodeAt(i);
    if (code < 65 || code > 90) return -1;
    col = col * 26 + (code - 64);
  }
  return col - 1;
}

export function parseCellRef(ref: string): { row: number; col: number } | null {
  const m = /^([A-Za-z]+)(\d+)$/.exec(ref.trim());
  if (!m) return null;
  const col = lettersToColIndex(m[1]);
  const row = Number.parseInt(m[2], 10) - 1;
  if (col < 0 || !Number.isFinite(row) || row < 0) return null;
  return { row, col };
}

function hasOle2Magic(bytes: Uint8Array): boolean {
  if (bytes.length < OLE2_MAGIC.length) return false;
  for (let i = 0; i < OLE2_MAGIC.length; i++) {
    if (bytes[i] !== OLE2_MAGIC[i]) return false;
  }
  return true;
}

interface ZipCdEntry {
  name: string;
  compressionMethod: number;
  compressedSize: number;
  uncompressedSize: number;
  localHeaderOffset: number;
}

function findEocdOffset(bytes: Uint8Array, view: DataView): number {
  const minPos = Math.max(0, bytes.length - 22 - 65_535);
  for (let pos = bytes.length - 22; pos >= minPos; pos--) {
    if (view.getUint32(pos, true) !== 0x06054b50) continue;
    const commentLen = view.getUint16(pos + 20, true);
    if (pos + 22 + commentLen !== bytes.length) continue;
    const cdSize = view.getUint32(pos + 12, true);
    const cdOffset = view.getUint32(pos + 16, true);
    if (cdOffset + cdSize <= pos) return pos;
  }
  return -1;
}

function parseCentralDirectory(bytes: Uint8Array): Map<string, ZipCdEntry> {
  if (hasOle2Magic(bytes)) {
    throw new XlsxError('LEGACY_XLS', 'Legacy .xls binary format is not supported');
  }
  if (bytes.length < 22) {
    throw new XlsxError('INVALID_XLSX', 'File is too small to be an .xlsx archive');
  }

  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const eocdPos = findEocdOffset(bytes, view);
  if (eocdPos < 0) {
    throw new XlsxError('INVALID_XLSX', 'Missing ZIP end of central directory');
  }

  const totalEntries = view.getUint16(eocdPos + 10, true);
  if (totalEntries > MAX_ZIP_ENTRIES) {
    throw new XlsxError('TOO_LARGE', 'Archive has too many entries');
  }

  const cdSize = view.getUint32(eocdPos + 12, true);
  const cdOffset = view.getUint32(eocdPos + 16, true);
  const cdEnd = cdOffset + cdSize;
  if (cdEnd > eocdPos) {
    throw new XlsxError('INVALID_XLSX', 'Invalid ZIP central directory bounds');
  }

  const entries = new Map<string, ZipCdEntry>();
  let pos = cdOffset;
  for (let i = 0; i < totalEntries; i++) {
    if (pos + 46 > cdEnd || view.getUint32(pos, true) !== 0x02014b50) {
      throw new XlsxError('INVALID_XLSX', 'Invalid ZIP central directory header');
    }
    const compressionMethod = view.getUint16(pos + 10, true);
    const compressedSize = view.getUint32(pos + 20, true);
    const uncompressedSize = view.getUint32(pos + 24, true);
    const nameLen = view.getUint16(pos + 28, true);
    const extraLen = view.getUint16(pos + 30, true);
    const commentLen = view.getUint16(pos + 32, true);
    const localHeaderOffset = view.getUint32(pos + 42, true);
    const nextPos = pos + 46 + nameLen + extraLen + commentLen;
    if (nextPos > cdEnd) {
      throw new XlsxError('INVALID_XLSX', 'Truncated ZIP central directory entry');
    }
    if (uncompressedSize > MAX_ENTRY_BYTES) {
      throw new XlsxError('TOO_LARGE', 'Worksheet XML exceeds size limit');
    }
    const name = normalizeZipPath(utf8Decoder.decode(bytes.subarray(pos + 46, pos + 46 + nameLen)));
    entries.set(name.toLowerCase(), {
      name,
      compressionMethod,
      compressedSize,
      uncompressedSize,
      localHeaderOffset,
    });
    pos = nextPos;
  }

  return entries;
}

async function inflateRaw(
  compressed: Uint8Array,
  remainingTotalBudget: number,
): Promise<Uint8Array> {
  if (typeof DecompressionStream === 'undefined') {
    throw new XlsxError('INVALID_XLSX', 'DecompressionStream is unavailable in this environment');
  }
  const ds = new DecompressionStream('deflate-raw');
  const writer = ds.writable.getWriter();
  const reader = ds.readable.getReader();

  const payload = new Uint8Array(compressed.byteLength);
  payload.set(compressed);

  const writePromise = (async () => {
    try {
      await writer.write(payload);
      await writer.close();
    } catch {
      /* reader loop surfaces stream failures */
    }
  })();

  const chunks: Uint8Array[] = [];
  let total = 0;
  try {
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      if (!value) continue;
      total += value.byteLength;
      if (total > MAX_ENTRY_BYTES || total > remainingTotalBudget) {
        await reader.cancel().catch(() => {});
        await writer.abort().catch(() => {});
        throw new XlsxError('TOO_LARGE', 'Decompressed XML exceeds size limit');
      }
      chunks.push(value);
    }
  } catch (err) {
    if (err instanceof XlsxError) throw err;
    throw new XlsxError('INVALID_XLSX', 'Failed to decompress .xlsx entry');
  } finally {
    await writePromise;
  }

  const out = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    out.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return out;
}

async function extractZipEntry(
  bytes: Uint8Array,
  view: DataView,
  entry: ZipCdEntry,
  budget: { remaining: number },
): Promise<Uint8Array> {
  const lh = entry.localHeaderOffset;
  if (lh + 30 > bytes.length || view.getUint32(lh, true) !== 0x04034b50) {
    throw new XlsxError('INVALID_XLSX', 'Invalid ZIP local file header');
  }
  const lhNameLen = view.getUint16(lh + 26, true);
  const lhExtraLen = view.getUint16(lh + 28, true);
  const dataStart = lh + 30 + lhNameLen + lhExtraLen;
  const dataEnd = dataStart + entry.compressedSize;
  if (dataEnd > bytes.length) {
    throw new XlsxError('INVALID_XLSX', 'Truncated ZIP entry payload');
  }
  const slice = bytes.subarray(dataStart, dataEnd);

  let data: Uint8Array;
  if (entry.compressionMethod === 0) {
    if (slice.byteLength > MAX_ENTRY_BYTES || slice.byteLength > budget.remaining) {
      throw new XlsxError('TOO_LARGE', 'Workbook entry exceeds size limit');
    }
    data = slice;
  } else if (entry.compressionMethod === 8) {
    data = await inflateRaw(slice, budget.remaining);
  } else {
    throw new XlsxError('INVALID_XLSX', `Unsupported ZIP compression method ${entry.compressionMethod}`);
  }
  budget.remaining -= data.byteLength;
  return data;
}

function normalizeZipPath(path: string): string {
  const parts: string[] = [];
  for (const seg of path.replace(/\\/g, '/').split('/')) {
    if (!seg || seg === '.') continue;
    if (seg === '..') {
      parts.pop();
    } else {
      parts.push(seg);
    }
  }
  return parts.join('/');
}

function resolveTarget(baseDir: string, target: string): string {
  if (target.startsWith('/')) return normalizeZipPath(target.slice(1));
  return normalizeZipPath(baseDir ? `${baseDir}/${target}` : target);
}

function splitDirFile(path: string): { dir: string; file: string } {
  const idx = path.lastIndexOf('/');
  if (idx < 0) return { dir: '', file: path };
  return { dir: path.slice(0, idx), file: path.slice(idx + 1) };
}

function parseXml(bytes: Uint8Array): Document {
  const text = utf8Decoder.decode(bytes);
  const doc = new DOMParser().parseFromString(text, 'application/xml');
  if (
    !doc.documentElement ||
    doc.documentElement.localName === 'parsererror' ||
    doc.getElementsByTagName('parsererror').length > 0
  ) {
    throw new XlsxError('INVALID_XLSX', 'Malformed XML inside .xlsx archive');
  }
  return doc;
}

function childElements(el: Element, localName?: string): Element[] {
  const out: Element[] = [];
  for (let i = 0; i < el.children.length; i++) {
    const child = el.children[i];
    if (!localName || child.localName === localName) out.push(child);
  }
  return out;
}

function findDirectChild(el: Element, localName: string): Element | null {
  for (let i = 0; i < el.children.length; i++) {
    if (el.children[i].localName === localName) return el.children[i];
  }
  return null;
}

function collectTextSkippingPhonetics(el: Element): string {
  let text = '';
  for (let i = 0; i < el.children.length; i++) {
    const child = el.children[i];
    if (child.localName === 'rPh' || child.localName === 'phoneticPr') continue;
    if (child.localName === 't') {
      text += child.textContent ?? '';
    } else {
      text += collectTextSkippingPhonetics(child);
    }
  }
  return text;
}

interface RelEntry {
  id: string;
  type: string;
  target: string;
}

function parseRels(doc: Document, baseDir: string): Map<string, RelEntry> {
  const map = new Map<string, RelEntry>();
  for (const rel of childElements(doc.documentElement, 'Relationship')) {
    const id = rel.getAttribute('Id') ?? '';
    const type = rel.getAttribute('Type') ?? '';
    const rawTarget = rel.getAttribute('Target') ?? '';
    if (!id || !rawTarget) continue;
    map.set(id, { id, type, target: resolveTarget(baseDir, rawTarget) });
  }
  return map;
}

function parseSharedStrings(doc: Document): string[] {
  const out: string[] = [];
  for (const si of childElements(doc.documentElement, 'si')) {
    out.push(collectTextSkippingPhonetics(si));
  }
  return out;
}

function parseWorksheet(doc: Document, sharedStrings: string[]): XlsxSheetCell[][] {
  const sheetData = findDirectChild(doc.documentElement, 'sheetData');
  if (!sheetData) return [];

  const rowMap = new Map<number, Map<number, XlsxSheetCell>>();
  let nextRowIdx = 0;
  let maxRow = -1;
  let maxCol = -1;

  for (const rowEl of childElements(sheetData, 'row')) {
    const rAttr = rowEl.getAttribute('r');
    const parsedRow = rAttr ? Number.parseInt(rAttr, 10) - 1 : nextRowIdx;
    const rowIdx = Number.isFinite(parsedRow) && parsedRow >= 0 ? parsedRow : nextRowIdx;
    nextRowIdx = rowIdx + 1;

    let nextColIdx = 0;
    for (const cellEl of childElements(rowEl, 'c')) {
      const refAttr = cellEl.getAttribute('r');
      const parsedRef = refAttr ? parseCellRef(refAttr) : null;
      const cellRow = parsedRef ? parsedRef.row : rowIdx;
      const cellCol = parsedRef ? parsedRef.col : nextColIdx;
      nextColIdx = cellCol + 1;

      const fEl = findDirectChild(cellEl, 'f');
      const vEl = findDirectChild(cellEl, 'v');
      const isEl = findDirectChild(cellEl, 'is');
      const typeAttr = cellEl.getAttribute('t') ?? '';

      const rawFormula = fEl?.textContent?.trim();
      const formula = rawFormula ? rawFormula : undefined;
      const rawV = vEl?.textContent ?? '';

      let value: string;
      if (typeAttr === 's') {
        const idx = Number.parseInt(rawV, 10);
        value = Number.isFinite(idx) && idx >= 0 && idx < sharedStrings.length ? sharedStrings[idx] : '';
      } else if (typeAttr === 'inlineStr') {
        value = isEl ? collectTextSkippingPhonetics(isEl) : '';
      } else if (typeAttr === 'b') {
        value = rawV === '1' || rawV.toLowerCase() === 'true' ? 'TRUE' : 'FALSE';
      } else {
        value = rawV;
      }

      if (value === '' && !formula) continue;
      if (cellRow >= MAX_SHEET_ROWS || cellCol >= MAX_SHEET_COLS) {
        throw new XlsxError('TOO_LARGE', 'Worksheet dimensions exceed supported bounds');
      }

      let cols = rowMap.get(cellRow);
      if (!cols) {
        cols = new Map();
        rowMap.set(cellRow, cols);
      }
      cols.set(cellCol, {
        row: cellRow,
        col: cellCol,
        value,
        ...(formula !== undefined ? { formula } : {}),
      });
      if (cellRow > maxRow) maxRow = cellRow;
      if (cellCol > maxCol) maxCol = cellCol;
    }
  }

  if (maxRow < 0 || maxCol < 0) return [];

  const grid: XlsxSheetCell[][] = [];
  for (let r = 0; r <= maxRow; r++) {
    const srcRow = rowMap.get(r);
    const outRow: XlsxSheetCell[] = [];
    for (let c = 0; c <= maxCol; c++) {
      outRow.push(srcRow?.get(c) ?? { row: r, col: c, value: '' });
    }
    grid.push(outRow);
  }
  return grid;
}

export async function readXlsxWorkbook(bytes: Uint8Array): Promise<XlsxWorkbook> {
  const entries = parseCentralDirectory(bytes);
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const budget = { remaining: MAX_TOTAL_BYTES };

  const readEntryDoc = async (path: string): Promise<Document | null> => {
    const entry = entries.get(normalizeZipPath(path).toLowerCase());
    if (!entry) return null;
    const raw = await extractZipEntry(bytes, view, entry, budget);
    return parseXml(raw);
  };

  let workbookPath = 'xl/workbook.xml';
  const rootRelsDoc = await readEntryDoc('_rels/.rels');
  if (rootRelsDoc) {
    const rootRels = parseRels(rootRelsDoc, '');
    for (const rel of rootRels.values()) {
      if (rel.type.endsWith('/officeDocument')) {
        workbookPath = rel.target;
        break;
      }
    }
  }

  const workbookDoc = await readEntryDoc(workbookPath);
  if (!workbookDoc) {
    throw new XlsxError('INVALID_XLSX', 'Missing workbook.xml in .xlsx archive');
  }

  const { dir: wbDir, file: wbFile } = splitDirFile(workbookPath);
  const wbRelsPath = wbDir ? `${wbDir}/_rels/${wbFile}.rels` : `_rels/${wbFile}.rels`;
  const wbRelsDoc = await readEntryDoc(wbRelsPath);
  const wbRels = wbRelsDoc ? parseRels(wbRelsDoc, wbDir) : new Map<string, RelEntry>();

  let sharedStringsPath = wbDir ? `${wbDir}/sharedStrings.xml` : 'sharedStrings.xml';
  for (const rel of wbRels.values()) {
    if (rel.type.endsWith('/sharedStrings')) {
      sharedStringsPath = rel.target;
      break;
    }
  }
  const sstDoc = await readEntryDoc(sharedStringsPath);
  const sharedStrings = sstDoc ? parseSharedStrings(sstDoc) : [];

  const sheetsEl = findDirectChild(workbookDoc.documentElement, 'sheets');
  if (!sheetsEl) {
    throw new XlsxError('INVALID_XLSX', 'Workbook contains no sheets list');
  }

  const sheets: XlsxSheet[] = [];
  const sheetEls = childElements(sheetsEl, 'sheet');
  for (let i = 0; i < sheetEls.length; i++) {
    const sheetEl = sheetEls[i];
    const name = sheetEl.getAttribute('name') || `Sheet${i + 1}`;
    const rid =
      sheetEl.getAttribute('r:id') ||
      sheetEl.getAttributeNS('http://schemas.openxmlformats.org/officeDocument/2006/relationships', 'id') ||
      '';
    const rel = rid ? wbRels.get(rid) : undefined;
    if (rel && rel.type && !rel.type.endsWith('/worksheet')) continue;
    const sheetPath = rel?.target ?? (wbDir ? `${wbDir}/worksheets/sheet${i + 1}.xml` : `worksheets/sheet${i + 1}.xml`);
    const sheetDoc = await readEntryDoc(sheetPath);
    if (!sheetDoc) {
      if (rel) {
        throw new XlsxError('INVALID_XLSX', `Missing worksheet file ${sheetPath}`);
      }
      continue;
    }
    sheets.push({
      name,
      rows: parseWorksheet(sheetDoc, sharedStrings),
    });
  }

  if (sheets.length === 0) {
    throw new XlsxError('INVALID_XLSX', 'Workbook contains no readable worksheets');
  }

  return { sheets };
}

function escapeXml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;');
}

const STYLE_INDEX: Record<XlsxWriteStyle, number> = {
  default: 0,
  header: 1,
  fare: 2,
  diagonal: 3,
  mirror: 4,
};

const CRC32_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let i = 0; i < 256; i++) {
    let c = i;
    for (let k = 0; k < 8; k++) {
      c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
    table[i] = c >>> 0;
  }
  return table;
})();

function crc32(data: Uint8Array): number {
  let c = 0xffffffff;
  for (let i = 0; i < data.length; i++) {
    c = CRC32_TABLE[(c ^ data[i]) & 0xff] ^ (c >>> 8);
  }
  return (c ^ 0xffffffff) >>> 0;
}

function buildStoredZip(files: { name: string; data: Uint8Array }[]): Uint8Array {
  const localParts: Uint8Array[] = [];
  const cdParts: Uint8Array[] = [];
  let offset = 0;

  for (const file of files) {
    const nameBytes = utf8Encoder.encode(file.name);
    const crc = crc32(file.data);
    const size = file.data.byteLength;

    const localHeader = new Uint8Array(30 + nameBytes.length);
    const lv = new DataView(localHeader.buffer);
    lv.setUint32(0, 0x04034b50, true);
    lv.setUint16(4, 20, true);
    lv.setUint16(6, 0x0800, true); // UTF-8 filename flag
    lv.setUint16(8, 0, true); // STORE
    lv.setUint16(10, 0, true);
    lv.setUint16(12, 0x5421, true);
    lv.setUint32(14, crc, true);
    lv.setUint32(18, size, true);
    lv.setUint32(22, size, true);
    lv.setUint16(26, nameBytes.length, true);
    lv.setUint16(28, 0, true);
    localHeader.set(nameBytes, 30);

    const cdHeader = new Uint8Array(46 + nameBytes.length);
    const cv = new DataView(cdHeader.buffer);
    cv.setUint32(0, 0x02014b50, true);
    cv.setUint16(4, 20, true);
    cv.setUint16(6, 20, true);
    cv.setUint16(8, 0x0800, true); // UTF-8 filename flag
    cv.setUint16(10, 0, true); // STORE
    cv.setUint16(12, 0, true);
    cv.setUint16(14, 0x5421, true);
    cv.setUint32(16, crc, true);
    cv.setUint32(20, size, true);
    cv.setUint32(24, size, true);
    cv.setUint16(28, nameBytes.length, true);
    cv.setUint16(30, 0, true);
    cv.setUint16(32, 0, true);
    cv.setUint16(34, 0, true);
    cv.setUint16(36, 0, true);
    cv.setUint32(38, 0, true);
    cv.setUint32(42, offset, true);
    cdHeader.set(nameBytes, 46);

    localParts.push(localHeader, file.data);
    cdParts.push(cdHeader);
    offset += localHeader.byteLength + size;
  }

  let cdSize = 0;
  for (const part of cdParts) cdSize += part.byteLength;

  const eocd = new Uint8Array(22);
  const ev = new DataView(eocd.buffer);
  ev.setUint32(0, 0x06054b50, true);
  ev.setUint16(4, 0, true);
  ev.setUint16(6, 0, true);
  ev.setUint16(8, files.length, true);
  ev.setUint16(10, files.length, true);
  ev.setUint32(12, cdSize, true);
  ev.setUint32(16, offset, true);
  ev.setUint16(20, 0, true);

  const totalSize = offset + cdSize + eocd.byteLength;
  const out = new Uint8Array(totalSize);
  let cursor = 0;
  for (const part of localParts) {
    out.set(part, cursor);
    cursor += part.byteLength;
  }
  for (const part of cdParts) {
    out.set(part, cursor);
    cursor += part.byteLength;
  }
  out.set(eocd, cursor);
  return out;
}

export function buildXlsxWorkbook(
  sheetName: string,
  rows: XlsxWriteCell[][],
  colWidths?: number[],
): Uint8Array {
  const safeSheetName =
    sheetName
      .replace(/[\\/?*[\]:]/g, ' ')
      .trim()
      .slice(0, 31) || 'Fares';

  const sharedStrings: string[] = [];
  const sstMap = new Map<string, number>();
  const internString = (s: string): number => {
    const existing = sstMap.get(s);
    if (existing !== undefined) return existing;
    const idx = sharedStrings.length;
    sharedStrings.push(s);
    sstMap.set(s, idx);
    return idx;
  };

  let rowXml = '';
  for (let r = 0; r < rows.length; r++) {
    const row = rows[r];
    let cellsXml = '';
    for (let c = 0; c < row.length; c++) {
      const cell = row[c];
      const ref = `${colIndexToLetters(c)}${r + 1}`;
      const styleIdx = STYLE_INDEX[cell.style ?? 'default'];
      const sAttr = styleIdx > 0 ? ` s="${styleIdx}"` : '';

      if (cell.formula) {
        const expr = cell.formula.startsWith('=') ? cell.formula.slice(1) : cell.formula;
        if (typeof cell.value === 'number' && Number.isFinite(cell.value)) {
          cellsXml += `<c r="${ref}"${sAttr}><f>${escapeXml(expr)}</f><v>${cell.value}</v></c>`;
        } else {
          const cached = cell.value === undefined ? '' : String(cell.value);
          cellsXml += `<c r="${ref}"${sAttr} t="str"><f>${escapeXml(expr)}</f><v>${escapeXml(cached)}</v></c>`;
        }
      } else if (typeof cell.value === 'number' && Number.isFinite(cell.value)) {
        cellsXml += `<c r="${ref}"${sAttr}><v>${cell.value}</v></c>`;
      } else if (typeof cell.value === 'string' && cell.value !== '') {
        const idx = internString(cell.value);
        cellsXml += `<c r="${ref}"${sAttr} t="s"><v>${idx}</v></c>`;
      } else if (styleIdx > 0) {
        cellsXml += `<c r="${ref}"${sAttr}/>`;
      }
    }
    if (cellsXml) {
      rowXml += `<row r="${r + 1}">${cellsXml}</row>`;
    }
  }

  const colsXml =
    colWidths && colWidths.length > 0
      ? `<cols>${colWidths
          .map(
            (w, i) =>
              `<col min="${i + 1}" max="${i + 1}" width="${Math.max(8, Math.min(60, w))}" customWidth="1"/>`,
          )
          .join('')}</cols>`
      : '';

  const sheetXml =
    `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>` +
    `<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">` +
    `<sheetViews><sheetView workbookViewId="0"><pane xSplit="1" ySplit="1" topLeftCell="B2" activePane="bottomRight" state="frozen"/></sheetView></sheetViews>` +
    colsXml +
    `<sheetData>${rowXml}</sheetData>` +
    `</worksheet>`;

  const sstItemsXml = sharedStrings
    .map((s) => {
      const preserve = /^\s|\s$/.test(s) ? ' xml:space="preserve"' : '';
      return `<si><t${preserve}>${escapeXml(s)}</t></si>`;
    })
    .join('');
  const sharedStringsXml =
    `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>` +
    `<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="${sharedStrings.length}" uniqueCount="${sharedStrings.length}">${sstItemsXml}</sst>`;

  const stylesXml =
    `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>` +
    `<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">` +
    `<numFmts count="1"><numFmt numFmtId="164" formatCode="0.00"/></numFmts>` +
    `<fonts count="3">` +
    `<font><sz val="11"/><name val="Calibri"/></font>` +
    `<font><b/><sz val="11"/><color rgb="FF1E293B"/><name val="Calibri"/></font>` +
    `<font><i/><sz val="11"/><color rgb="FF64748B"/><name val="Calibri"/></font>` +
    `</fonts>` +
    `<fills count="5">` +
    `<fill><patternFill patternType="none"/></fill>` +
    `<fill><patternFill patternType="gray125"/></fill>` +
    `<fill><patternFill patternType="solid"><fgColor rgb="FFEEF2FF"/></patternFill></fill>` +
    `<fill><patternFill patternType="solid"><fgColor rgb="FFF8FAFC"/></patternFill></fill>` +
    `<fill><patternFill patternType="solid"><fgColor rgb="FFF1F5F9"/></patternFill></fill>` +
    `</fills>` +
    `<borders count="2">` +
    `<border><left/><right/><top/><bottom/><diagonal/></border>` +
    `<border>` +
    `<left style="thin"><color rgb="FFCBD5E1"/></left>` +
    `<right style="thin"><color rgb="FFCBD5E1"/></right>` +
    `<top style="thin"><color rgb="FFCBD5E1"/></top>` +
    `<bottom style="thin"><color rgb="FFCBD5E1"/></bottom>` +
    `<diagonal/></border>` +
    `</borders>` +
    `<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>` +
    `<cellXfs count="5">` +
    `<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>` +
    `<xf numFmtId="0" fontId="1" fillId="2" borderId="1" xfId="0" applyFont="1" applyFill="1" applyBorder="1" applyAlignment="1"><alignment horizontal="center" vertical="center"/></xf>` +
    `<xf numFmtId="164" fontId="0" fillId="0" borderId="1" xfId="0" applyNumberFormat="1" applyBorder="1" applyAlignment="1"><alignment horizontal="right" vertical="center"/></xf>` +
    `<xf numFmtId="164" fontId="0" fillId="3" borderId="1" xfId="0" applyNumberFormat="1" applyFill="1" applyBorder="1" applyAlignment="1"><alignment horizontal="right" vertical="center"/></xf>` +
    `<xf numFmtId="164" fontId="2" fillId="4" borderId="1" xfId="0" applyNumberFormat="1" applyFont="1" applyFill="1" applyBorder="1" applyAlignment="1"><alignment horizontal="right" vertical="center"/></xf>` +
    `</cellXfs>` +
    `</styleSheet>`;

  const workbookXml =
    `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>` +
    `<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">` +
    `<sheets><sheet name="${escapeXml(safeSheetName)}" sheetId="1" r:id="rId1"/></sheets>` +
    `</workbook>`;

  const workbookRelsXml =
    `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>` +
    `<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">` +
    `<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>` +
    `<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>` +
    `<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>` +
    `</Relationships>`;

  const rootRelsXml =
    `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>` +
    `<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">` +
    `<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>` +
    `</Relationships>`;

  const contentTypesXml =
    `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>` +
    `<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">` +
    `<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>` +
    `<Default Extension="xml" ContentType="application/xml"/>` +
    `<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>` +
    `<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>` +
    `<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>` +
    `<Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>` +
    `</Types>`;

  return buildStoredZip([
    { name: '[Content_Types].xml', data: utf8Encoder.encode(contentTypesXml) },
    { name: '_rels/.rels', data: utf8Encoder.encode(rootRelsXml) },
    { name: 'xl/workbook.xml', data: utf8Encoder.encode(workbookXml) },
    { name: 'xl/_rels/workbook.xml.rels', data: utf8Encoder.encode(workbookRelsXml) },
    { name: 'xl/styles.xml', data: utf8Encoder.encode(stylesXml) },
    { name: 'xl/sharedStrings.xml', data: utf8Encoder.encode(sharedStringsXml) },
    { name: 'xl/worksheets/sheet1.xml', data: utf8Encoder.encode(sheetXml) },
  ]);
}
