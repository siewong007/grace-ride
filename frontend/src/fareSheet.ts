import type { AreaFare, FareArea } from './pages/admin/AreaFares';
import {
  buildXlsxWorkbook,
  colIndexToLetters,
  parseCellRef,
  readXlsxWorkbook,
  XlsxError,
  type XlsxSheetCell,
  type XlsxWriteCell,
} from './xlsxCodec';

export const MAX_FARE_FILE_BYTES = 5 * 1024 * 1024;
export const MAX_AREA_NAME_CHARS = 80;
export const MAX_FARE_CENTS = 50_000;

export type FareSheetLayout = 'matrix' | 'list';

export type FareImportRowStatus = 'new' | 'update' | 'unchanged' | 'unknown_area';

export interface ParsedFareRow {
  areaAName: string;
  areaBName: string;
  areaAExists: boolean;
  areaBExists: boolean;
  areaAId: number | null;
  areaBId: number | null;
  currentCents: number | null;
  amountCents: number;
  status: FareImportRowStatus;
  sourceRow: number;
  sourceCol: number;
}

export type FareIssueKind =
  | 'invalid_amount'
  | 'conflict'
  | 'missing_area'
  | 'invalid_area_name';

export interface FareParseIssue {
  kind: FareIssueKind;
  row: number;
  col: number;
  raw?: string;
  route?: string;
  firstCents?: number;
  secondCents?: number;
}

export interface ParsedFareSheet {
  layout: FareSheetLayout;
  rows: ParsedFareRow[];
  issues: FareParseIssue[];
  unknownAreas: string[];
}

export interface FareWorkbookParseResult {
  fileName: string;
  sheets: { name: string; parsed: ParsedFareSheet }[];
}

const pairIdKey = (a: number, b: number) => (a <= b ? `${a}-${b}` : `${b}-${a}`);

export function normalizeAreaName(s: string): string {
  return s.trim().replace(/\s+/g, ' ');
}

export function areaLookupKey(s: string): string {
  return normalizeAreaName(s).toLowerCase();
}

export function isValidAreaName(s: string): boolean {
  const norm = normalizeAreaName(s);
  if (!norm || norm.length > MAX_AREA_NAME_CHARS) return false;
  for (let i = 0; i < norm.length; i++) {
    const code = norm.charCodeAt(i);
    if (code < 32 || code === 127) return false;
  }
  return true;
}

function escapeCsvCell(s: string): string {
  const trimmed = s.trimStart();
  const neutralized =
    s.startsWith('\t') ||
    s.startsWith('\r') ||
    trimmed.startsWith('=') ||
    trimmed.startsWith('+') ||
    trimmed.startsWith('-') ||
    trimmed.startsWith('@') ||
    trimmed.startsWith('|') ||
    trimmed.startsWith('%')
      ? `'${s}`
      : s;
  if (
    neutralized.includes(',') ||
    neutralized.includes('"') ||
    neutralized.includes('\n') ||
    neutralized.includes('\r')
  ) {
    return `"${neutralized.replace(/"/g, '""')}"`;
  }
  return neutralized;
}

export function buildFaresCsv(
  areas: FareArea[],
  fares: AreaFare[],
  originLabel: string,
): string {
  const open = areas.filter((a) => a.active).sort((x, y) => x.name.localeCompare(y.name));
  const byPair = new Map<string, number>();
  for (const f of fares) {
    byPair.set(pairIdKey(f.area_a.id, f.area_b.id), f.amount_cents);
  }

  const lines: string[] = [];
  lines.push([originLabel, ...open.map((a) => a.name)].map(escapeCsvCell).join(','));
  for (let i = 0; i < open.length; i++) {
    const row: string[] = [escapeCsvCell(open[i].name)];
    for (let j = 0; j < open.length; j++) {
      if (j < i) {
        row.push('');
      } else {
        const cents = byPair.get(pairIdKey(open[i].id, open[j].id));
        row.push(cents !== undefined ? (cents / 100).toFixed(2) : '');
      }
    }
    lines.push(row.join(','));
  }
  return `\uFEFF${lines.join('\r\n')}\r\n`;
}

export function buildFaresXlsx(
  areas: FareArea[],
  fares: AreaFare[],
  opts: { sheetName: string; originLabel: string },
): Uint8Array {
  const open = areas.filter((a) => a.active).sort((x, y) => x.name.localeCompare(y.name));
  const byPair = new Map<string, number>();
  for (const f of fares) {
    byPair.set(pairIdKey(f.area_a.id, f.area_b.id), f.amount_cents);
  }

  const rows: XlsxWriteCell[][] = [];
  const header: XlsxWriteCell[] = [{ value: opts.originLabel, style: 'header' }];
  for (const a of open) {
    header.push({ value: a.name, style: 'header' });
  }
  rows.push(header);

  for (let i = 0; i < open.length; i++) {
    const row: XlsxWriteCell[] = [{ value: open[i].name, style: 'header' }];
    for (let j = 0; j < open.length; j++) {
      const cents = byPair.get(pairIdKey(open[i].id, open[j].id));
      const rmValue = cents !== undefined ? Number((cents / 100).toFixed(2)) : undefined;
      if (j >= i) {
        row.push({
          ...(rmValue !== undefined ? { value: rmValue } : {}),
          style: i === j ? 'diagonal' : 'fare',
        });
      } else {
        const upperRef = `${colIndexToLetters(i + 1)}${j + 2}`;
        row.push({
          formula: `IF(${upperRef}="","",${upperRef})`,
          value: rmValue !== undefined ? rmValue : '',
          style: 'mirror',
        });
      }
    }
    rows.push(row);
  }

  const firstColWidth = Math.min(
    30,
    Math.max(14, opts.originLabel.length + 4, ...open.map((a) => a.name.length + 4)),
  );
  const colWidths = [
    firstColWidth,
    ...open.map((a) => Math.min(30, Math.max(12, a.name.length + 4))),
  ];

  return buildXlsxWorkbook(opts.sheetName, rows, colWidths);
}

function detectDelimiter(text: string): ',' | '\t' | ';' {
  let lineHasDelim = false;
  let commas = 0;
  let tabs = 0;
  let semis = 0;
  let inQuotes = false;

  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (ch === '"') {
      if (inQuotes && text[i + 1] === '"') {
        i++;
      } else {
        inQuotes = !inQuotes;
      }
      continue;
    }
    if (!inQuotes) {
      if (ch === ',') {
        commas++;
        lineHasDelim = true;
      } else if (ch === '\t') {
        tabs++;
        lineHasDelim = true;
      } else if (ch === ';') {
        semis++;
        lineHasDelim = true;
      } else if (ch === '\n' || ch === '\r') {
        if (lineHasDelim) break;
      }
    }
  }

  if (tabs > commas && tabs >= semis) return '\t';
  if (semis > commas && semis > tabs) return ';';
  return ',';
}

export function parseCsv(rawText: string): string[][] {
  const text = rawText.charCodeAt(0) === 0xfeff ? rawText.slice(1) : rawText;
  if (!text.trim()) return [];
  const delim = detectDelimiter(text);

  const rows: string[][] = [];
  let row: string[] = [];
  let field = '';
  let inQuotes = false;
  let atFieldStart = true;

  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (inQuotes) {
      if (ch === '"') {
        if (text[i + 1] === '"') {
          field += '"';
          i++;
        } else {
          inQuotes = false;
        }
      } else {
        field += ch;
      }
      continue;
    }

    if (atFieldStart && ch === '"') {
      inQuotes = true;
      atFieldStart = false;
      continue;
    }

    if (ch === delim) {
      row.push(field);
      field = '';
      atFieldStart = true;
      continue;
    }

    if (ch === '\r' || ch === '\n') {
      if (ch === '\r' && text[i + 1] === '\n') i++;
      row.push(field);
      rows.push(row);
      row = [];
      field = '';
      atFieldStart = true;
      continue;
    }

    field += ch;
    atFieldStart = false;
  }

  if (field.length > 0 || row.length > 0) {
    row.push(field);
    rows.push(row);
  }

  while (rows.length > 0 && rows[rows.length - 1].every((c) => c === '')) {
    rows.pop();
  }
  return rows;
}

export type ParsedFareAmount =
  | { kind: 'blank' }
  | { kind: 'ok'; cents: number }
  | { kind: 'invalid' };

const BLANK_MARKERS = new Set([
  '',
  '-',
  '\u2014',
  '\u2013',
  'n/a',
  'na',
  'not set',
  'unset',
  '\u672a\u8bbe\u5b9a',
  '\u672a\u8a2d\u5b9a',
]);

export function parseFareAmountRm(raw: string): ParsedFareAmount {
  const trimmed = raw.trim();
  const lower = trimmed.toLowerCase();
  if (BLANK_MARKERS.has(lower)) return { kind: 'blank' };

  const stripped = trimmed.replace(/^(?:myr|rm)\s*/i, '').trim();
  if (!/^\d+(?:\.\d+)?$/.test(stripped)) return { kind: 'invalid' };
  const val = Number(stripped);
  if (!Number.isFinite(val) || val < 0 || val > 500) return { kind: 'invalid' };
  const cents = Math.round(val * 100);
  if (cents < 0 || cents > MAX_FARE_CENTS) return { kind: 'invalid' };
  return { kind: 'ok', cents };
}

const FROM_HEADER_ALIASES = new Set([
  'from',
  'origin',
  'area a',
  'area 1',
  'area1',
  'from area',
  'pickup',
  'pickup area',
  '\u51fa\u53d1\u5730\u533a',
  '\u8d77\u70b9',
  '\u5730\u533a1',
  '\u5730\u533aa',
  '\u4e0a\u8f66\u5730\u533a',
]);

const TO_HEADER_ALIASES = new Set([
  'to',
  'dest',
  'destination',
  'area b',
  'area 2',
  'area2',
  'to area',
  'other area',
  'event area',
  'dropoff',
  '\u5230\u8fbe\u5730\u533a',
  '\u76ee\u7684\u5730\u533a',
  '\u7ec8\u70b9',
  '\u5730\u533a2',
  '\u5730\u533ab',
  '\u6d3b\u52a8\u5730\u533a',
  '\u53e6\u4e00\u5730\u533a',
]);

const FARE_HEADER_ALIASES = new Set([
  'fare',
  'amount',
  'price',
  'rate',
  'rm',
  'myr',
  'fare rm',
  'fare myr',
  'amount rm',
  'amount myr',
  'price rm',
  'rate rm',
  'rate myr',
  '\u8f66\u8d39',
  '\u91d1\u989d',
  '\u8d39\u7528',
  '\u4ef7\u683c',
  '\u8f66\u8d39 rm',
  '\u91d1\u989d rm',
  '\u8d39\u7528 rm',
]);

function normalizeHeaderToken(s: string): string {
  return s
    .trim()
    .toLowerCase()
    .replace(/[_()[\]]+/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}

function trimOuterEmpty(cells: XlsxSheetCell[][]): XlsxSheetCell[][] {
  const nonEmptyRows = cells.filter((row) =>
    row.some((c) => c.value.trim() !== '' || Boolean(c.formula)),
  );
  if (nonEmptyRows.length === 0) return [];

  const maxLen = Math.max(...nonEmptyRows.map((r) => r.length));
  let firstCol = maxLen;
  let lastCol = -1;
  for (const row of nonEmptyRows) {
    for (let c = 0; c < row.length; c++) {
      if (row[c].value.trim() !== '' || row[c].formula) {
        if (c < firstCol) firstCol = c;
        if (c > lastCol) lastCol = c;
      }
    }
  }
  if (lastCol < firstCol) return [];

  const trimmed = nonEmptyRows.map((row) => {
    const baseRow = row[0]?.row ?? 0;
    const baseCol = row[firstCol]?.col ?? firstCol;
    const slice: XlsxSheetCell[] = [];
    for (let c = firstCol; c <= lastCol; c++) {
      slice.push(row[c] ?? { row: baseRow, col: baseCol + (c - firstCol), value: '' });
    }
    return slice;
  });

  // Skip single-cell banner/title rows placed above the header table.
  while (
    trimmed.length >= 3 &&
    trimmed[0][0].value.trim() !== '' &&
    trimmed[0].filter((c) => c.value.trim() !== '').length === 1 &&
    trimmed[1].filter((c) => c.value.trim() !== '').length >= 2
  ) {
    trimmed.shift();
  }

  return trimmed;
}

function parseTransposeMirrorRef(formula: string): { row: number; col: number } | null {
  const cleaned = formula.trim().replace(/^=/, '').replace(/\$/g, '').trim();
  const direct = parseCellRef(cleaned);
  if (direct) return direct;

  const ifMatch =
    /^IF\s*\(\s*([A-Za-z]+\d+)\s*=\s*""\s*,\s*""\s*,\s*([A-Za-z]+\d+)\s*\)$/i.exec(cleaned);
  if (ifMatch && ifMatch[1].toUpperCase() === ifMatch[2].toUpperCase()) {
    return parseCellRef(ifMatch[1]);
  }
  return null;
}

function detectLayout(grid: XlsxSheetCell[][]): { layout: FareSheetLayout; hasHeader: boolean } {
  const colCount = grid[0]?.length ?? 0;
  if (colCount !== 3) {
    return { layout: 'matrix', hasHeader: true };
  }

  const h0 = normalizeHeaderToken(grid[0][0].value);
  const h1 = normalizeHeaderToken(grid[0][1].value);
  const h2 = normalizeHeaderToken(grid[0][2].value);
  if (
    FROM_HEADER_ALIASES.has(h0) &&
    TO_HEADER_ALIASES.has(h1) &&
    FARE_HEADER_ALIASES.has(h2)
  ) {
    return { layout: 'list', hasHeader: true };
  }

  // Distinguish a 2-area 3-column matrix (where col headers match row labels)
  // from a headerless 3-column route list.
  const isTwoAreaMatrix =
    grid.length >= 3 &&
    grid[0][1].value.trim() !== '' &&
    grid[0][2].value.trim() !== '' &&
    areaLookupKey(grid[0][1].value) === areaLookupKey(grid[1][0].value) &&
    areaLookupKey(grid[0][2].value) === areaLookupKey(grid[2][0].value);
  if (isTwoAreaMatrix) {
    return { layout: 'matrix', hasHeader: true };
  }

  if (
    parseFareAmountRm(grid[0][2].value).kind === 'ok' &&
    isValidAreaName(grid[0][0].value) &&
    isValidAreaName(grid[0][1].value)
  ) {
    return { layout: 'list', hasHeader: false };
  }

  return { layout: 'matrix', hasHeader: true };
}

export function parseFareGrid(
  rawCells: XlsxSheetCell[][],
  allAreas: FareArea[],
  existingFares: AreaFare[],
): ParsedFareSheet {
  const grid = trimOuterEmpty(rawCells);
  if (grid.length === 0) {
    return { layout: 'matrix', rows: [], issues: [], unknownAreas: [] };
  }

  // Prefer active areas if duplicate names ever exist in legacy rows.
  const areaByKey = new Map<string, FareArea>();
  const sortedAreas = [...allAreas].sort((a, b) => Number(b.active) - Number(a.active));
  for (const area of sortedAreas) {
    const key = areaLookupKey(area.name);
    if (!areaByKey.has(key)) areaByKey.set(key, area);
  }

  const fareByPairId = new Map<string, AreaFare>();
  for (const f of existingFares) {
    fareByPairId.set(pairIdKey(f.area_a.id, f.area_b.id), f);
  }

  const { layout, hasHeader } = detectLayout(grid);
  const issues: FareParseIssue[] = [];
  const seenRoutes = new Map<
    string,
    {
      areaAName: string;
      areaBName: string;
      amountCents: number;
      sourceRow: number;
      sourceCol: number;
    }
  >();

  const recordRoute = (
    rawA: string,
    rawB: string,
    cents: number,
    row1: number,
    col1: number,
  ) => {
    const normA = normalizeAreaName(rawA);
    const normB = normalizeAreaName(rawB);
    const keyA = areaLookupKey(normA);
    const keyB = areaLookupKey(normB);
    const knownA = areaByKey.get(keyA);
    const knownB = areaByKey.get(keyB);
    const dispA = knownA?.name ?? normA;
    const dispB = knownB?.name ?? normB;
    const routeKey = keyA <= keyB ? `${keyA}\0${keyB}` : `${keyB}\0${keyA}`;

    const prev = seenRoutes.get(routeKey);
    if (prev) {
      if (prev.amountCents !== cents) {
        issues.push({
          kind: 'conflict',
          row: row1,
          col: col1,
          route: dispA === dispB ? dispA : `${dispA} \u2194 ${dispB}`,
          firstCents: prev.amountCents,
          secondCents: cents,
        });
      }
      return;
    }

    seenRoutes.set(routeKey, {
      areaAName: dispA,
      areaBName: dispB,
      amountCents: cents,
      sourceRow: row1,
      sourceCol: col1,
    });
  };

  if (layout === 'list') {
    const startRow = hasHeader ? 1 : 0;
    for (let r = startRow; r < grid.length; r++) {
      const row = grid[r];
      const c0 = row[0] ?? { row: r, col: 0, value: '' };
      const c1 = row[1] ?? { row: r, col: 1, value: '' };
      const c2 = row[2] ?? { row: r, col: 2, value: '' };
      const a = normalizeAreaName(c0.value);
      const b = normalizeAreaName(c1.value);
      const parsed = parseFareAmountRm(c2.value);

      if (!a && !b && parsed.kind === 'blank') continue;
      if (parsed.kind === 'blank') continue;
      if (!a || !b) {
        issues.push({ kind: 'missing_area', row: c0.row + 1, col: (!a ? c0.col : c1.col) + 1 });
        continue;
      }
      if (!isValidAreaName(a) || !isValidAreaName(b)) {
        issues.push({
          kind: 'invalid_area_name',
          row: c0.row + 1,
          col: (!isValidAreaName(a) ? c0.col : c1.col) + 1,
          raw: !isValidAreaName(a) ? a : b,
        });
        continue;
      }
      if (parsed.kind === 'invalid') {
        issues.push({
          kind: 'invalid_amount',
          row: c2.row + 1,
          col: c2.col + 1,
          raw: c2.value,
          route: a === b ? a : `${a} \u2194 ${b}`,
        });
        continue;
      }
      recordRoute(a, b, parsed.cents, c2.row + 1, c2.col + 1);
    }
  } else {
    const headerRow = grid[0];
    // Map sheet coordinates back to header labels so transpose mirror formulas are verified accurately.
    const rowLabelBySheetRow = new Map<number, string>();
    const colHeaderBySheetCol = new Map<number, string>();
    for (let c = 1; c < headerRow.length; c++) {
      colHeaderBySheetCol.set(headerRow[c].col, normalizeAreaName(headerRow[c].value));
    }
    for (let r = 1; r < grid.length; r++) {
      rowLabelBySheetRow.set(grid[r][0].row, normalizeAreaName(grid[r][0].value));
    }

    for (let r = 1; r < grid.length; r++) {
      const fromCell = grid[r][0];
      const fromName = normalizeAreaName(fromCell.value);
      for (let c = 1; c < headerRow.length; c++) {
        const cell = grid[r][c];
        const toCell = headerRow[c];
        const toName = normalizeAreaName(toCell.value);

        if (cell.formula) {
          const mirrorTarget = parseTransposeMirrorRef(cell.formula);
          if (mirrorTarget) {
            const targetFrom = rowLabelBySheetRow.get(mirrorTarget.row);
            const targetTo = colHeaderBySheetCol.get(mirrorTarget.col);
            if (
              targetFrom &&
              targetTo &&
              areaLookupKey(targetFrom) === areaLookupKey(toName) &&
              areaLookupKey(targetTo) === areaLookupKey(fromName)
            ) {
              continue;
            }
          }
        }

        const parsed = parseFareAmountRm(cell.value);
        if (parsed.kind === 'blank') continue;

        if (!fromName || !toName) {
          issues.push({
            kind: 'missing_area',
            row: cell.row + 1,
            col: cell.col + 1,
            raw: cell.value,
          });
          continue;
        }
        if (!isValidAreaName(fromName) || !isValidAreaName(toName)) {
          issues.push({
            kind: 'invalid_area_name',
            row: cell.row + 1,
            col: cell.col + 1,
            raw: !isValidAreaName(fromName) ? fromName : toName,
          });
          continue;
        }
        if (parsed.kind === 'invalid') {
          issues.push({
            kind: 'invalid_amount',
            row: cell.row + 1,
            col: cell.col + 1,
            raw: cell.value,
            route: fromName === toName ? fromName : `${fromName} \u2194 ${toName}`,
          });
          continue;
        }
        recordRoute(fromName, toName, parsed.cents, cell.row + 1, cell.col + 1);
      }
    }
  }

  const rows: ParsedFareRow[] = [];
  const unknownMap = new Map<string, string>();
  for (const item of seenRoutes.values()) {
    const areaA = areaByKey.get(areaLookupKey(item.areaAName));
    const areaB = areaByKey.get(areaLookupKey(item.areaBName));
    if (!areaA) {
      const k = areaLookupKey(item.areaAName);
      if (!unknownMap.has(k)) unknownMap.set(k, item.areaAName);
    }
    if (!areaB) {
      const k = areaLookupKey(item.areaBName);
      if (!unknownMap.has(k)) unknownMap.set(k, item.areaBName);
    }

    let currentCents: number | null = null;
    let status: FareImportRowStatus = 'unknown_area';
    if (areaA && areaB) {
      const existing = fareByPairId.get(pairIdKey(areaA.id, areaB.id));
      currentCents = existing ? existing.amount_cents : null;
      status =
        currentCents === null
          ? 'new'
          : currentCents === item.amountCents
            ? 'unchanged'
            : 'update';
    }

    rows.push({
      areaAName: item.areaAName,
      areaBName: item.areaBName,
      areaAExists: Boolean(areaA),
      areaBExists: Boolean(areaB),
      areaAId: areaA?.id ?? null,
      areaBId: areaB?.id ?? null,
      currentCents,
      amountCents: item.amountCents,
      status,
      sourceRow: item.sourceRow,
      sourceCol: item.sourceCol,
    });
  }

  return {
    layout,
    rows,
    issues,
    unknownAreas: Array.from(unknownMap.values()),
  };
}

function isZipSignature(bytes: Uint8Array): boolean {
  return (
    bytes.length >= 4 &&
    bytes[0] === 0x50 &&
    bytes[1] === 0x4b &&
    ((bytes[2] === 0x03 && bytes[3] === 0x04) || (bytes[2] === 0x05 && bytes[3] === 0x06))
  );
}

function isOle2Signature(bytes: Uint8Array): boolean {
  return (
    bytes.length >= 8 &&
    bytes[0] === 0xd0 &&
    bytes[1] === 0xcf &&
    bytes[2] === 0x11 &&
    bytes[3] === 0xe0 &&
    bytes[4] === 0xa1 &&
    bytes[5] === 0xb1 &&
    bytes[6] === 0x1a &&
    bytes[7] === 0xe1
  );
}

export async function readFareFile(
  file: File,
  allAreas: FareArea[],
  existingFares: AreaFare[],
): Promise<FareWorkbookParseResult> {
  if (file.size > MAX_FARE_FILE_BYTES) {
    throw new XlsxError('TOO_LARGE', 'File exceeds the 5 MB limit');
  }
  const buffer = await file.arrayBuffer();
  const bytes = new Uint8Array(buffer);
  if (bytes.byteLength > MAX_FARE_FILE_BYTES) {
    throw new XlsxError('TOO_LARGE', 'File exceeds the 5 MB limit');
  }

  const lowerName = file.name.toLowerCase();
  if (isOle2Signature(bytes) || lowerName.endsWith('.xls')) {
    throw new XlsxError('LEGACY_XLS', 'Legacy .xls files must be saved as .xlsx or .csv');
  }

  if (isZipSignature(bytes) || lowerName.endsWith('.xlsx')) {
    const wb = await readXlsxWorkbook(bytes);
    return {
      fileName: file.name,
      sheets: wb.sheets.map((s) => ({
        name: s.name,
        parsed: parseFareGrid(s.rows, allAreas, existingFares),
      })),
    };
  }

  const text = new TextDecoder('utf-8').decode(bytes);
  const csvRows = parseCsv(text);
  const grid: XlsxSheetCell[][] = csvRows.map((row, r) =>
    row.map((val, c) => ({ row: r, col: c, value: val })),
  );
  return {
    fileName: file.name,
    sheets: [
      {
        name: file.name,
        parsed: parseFareGrid(grid, allAreas, existingFares),
      },
    ],
  };
}

export function triggerBlobDownload(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
