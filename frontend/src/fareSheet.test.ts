import { describe, expect, it } from 'vitest';
import {
  buildFaresCsv,
  buildFaresXlsx,
  parseCsv,
  parseFareAmountRm,
  parseFareGrid,
  readFareFile,
} from './fareSheet';
import { readXlsxWorkbook, XlsxError } from './xlsxCodec';
import type { AreaFare, FareArea } from './pages/admin/AreaFares';

const areas: FareArea[] = [
  { id: 1, name: 'UT', active: true },
  { id: 2, name: 'KK', active: true },
  { id: 3, name: 'Pacific/RM', active: true },
  { id: 4, name: '\u516b\u6253\u7075', active: true },
  { id: 5, name: 'Old Town', active: false },
];

const existingFares: AreaFare[] = [
  {
    id: 10,
    area_a: { id: 1, name: 'UT' },
    area_b: { id: 2, name: 'KK' },
    amount_cents: 1500,
    updated_at: '',
  },
];

function makeFile(bytes: Uint8Array | string, name: string, type = ''): File {
  const raw = typeof bytes === 'string' ? new TextEncoder().encode(bytes) : bytes;
  const copy = new Uint8Array(raw.byteLength);
  copy.set(raw);
  const file = new File([copy], name, { type });
  if (typeof file.arrayBuffer !== 'function') {
    Object.defineProperty(file, 'arrayBuffer', {
      value: async () => copy.buffer.slice(0),
    });
  }
  return file;
}

const crcTable = (() => {
  const t = new Uint32Array(256);
  for (let i = 0; i < 256; i++) {
    let c = i;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[i] = c >>> 0;
  }
  return t;
})();

function crc32(data: Uint8Array): number {
  let c = 0xffffffff;
  for (let i = 0; i < data.length; i++) c = crcTable[(c ^ data[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

async function deflateRaw(input: Uint8Array): Promise<Uint8Array> {
  const cs = new CompressionStream('deflate-raw');
  const writer = cs.writable.getWriter();
  const reader = cs.readable.getReader();
  const payload = new Uint8Array(input.byteLength);
  payload.set(input);
  const writeDone = (async () => {
    await writer.write(payload);
    await writer.close();
  })();
  const chunks: Uint8Array[] = [];
  let len = 0;
  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    if (value) {
      chunks.push(value);
      len += value.byteLength;
    }
  }
  await writeDone;
  const out = new Uint8Array(len);
  let off = 0;
  for (const c of chunks) {
    out.set(c, off);
    off += c.byteLength;
  }
  return out;
}

/**
 * Builds a DEFLATE-compressed ZIP where entries use bit-3 data descriptors
 * (0 sizes in Local File Header, real sizes in Central Directory + descriptor).
 */
async function buildDeflatedZipWithDataDescriptor(
  files: { name: string; data: Uint8Array }[],
): Promise<Uint8Array> {
  const enc = new TextEncoder();
  const localParts: Uint8Array[] = [];
  const cdParts: Uint8Array[] = [];
  let offset = 0;

  for (const f of files) {
    const nameBytes = enc.encode(f.name);
    const compressed = await deflateRaw(f.data);
    const crc = crc32(f.data);

    const lh = new Uint8Array(30 + nameBytes.length);
    const lv = new DataView(lh.buffer);
    lv.setUint32(0, 0x04034b50, true);
    lv.setUint16(4, 20, true);
    lv.setUint16(6, 0x0808, true); // bit 3 (data descriptor) + bit 11 (UTF-8)
    lv.setUint16(8, 8, true); // DEFLATE
    lv.setUint32(14, 0, true); // zeroed in LFH when bit 3 is set
    lv.setUint32(18, 0, true);
    lv.setUint32(22, 0, true);
    lv.setUint16(26, nameBytes.length, true);
    lh.set(nameBytes, 30);

    const dd = new Uint8Array(16);
    const dv = new DataView(dd.buffer);
    dv.setUint32(0, 0x08074b50, true);
    dv.setUint32(4, crc, true);
    dv.setUint32(8, compressed.byteLength, true);
    dv.setUint32(12, f.data.byteLength, true);

    const cd = new Uint8Array(46 + nameBytes.length);
    const cv = new DataView(cd.buffer);
    cv.setUint32(0, 0x02014b50, true);
    cv.setUint16(4, 20, true);
    cv.setUint16(6, 20, true);
    cv.setUint16(8, 0x0808, true);
    cv.setUint16(10, 8, true);
    cv.setUint32(16, crc, true);
    cv.setUint32(20, compressed.byteLength, true);
    cv.setUint32(24, f.data.byteLength, true);
    cv.setUint16(28, nameBytes.length, true);
    cv.setUint32(42, offset, true);
    cd.set(nameBytes, 46);

    localParts.push(lh, compressed, dd);
    cdParts.push(cd);
    offset += lh.byteLength + compressed.byteLength + dd.byteLength;
  }

  const cdSize = cdParts.reduce((s, p) => s + p.byteLength, 0);
  const eocd = new Uint8Array(22);
  const ev = new DataView(eocd.buffer);
  ev.setUint32(0, 0x06054b50, true);
  ev.setUint16(8, files.length, true);
  ev.setUint16(10, files.length, true);
  ev.setUint32(12, cdSize, true);
  ev.setUint32(16, offset, true);

  const out = new Uint8Array(offset + cdSize + eocd.byteLength);
  let cur = 0;
  for (const p of [...localParts, ...cdParts, eocd]) {
    out.set(p, cur);
    cur += p.byteLength;
  }
  return out;
}

describe('fareSheet and xlsxCodec', () => {
  it('round-trips .xlsx export -> read -> parse preserving Pacific/RM, CJK names, blank cells, and mirror formulas', async () => {
    const xlsxBytes = buildFaresXlsx(areas, existingFares, {
      sheetName: 'Area Fares',
      originLabel: 'From \\ To',
    });

    const wb = await readXlsxWorkbook(xlsxBytes);
    expect(wb.sheets).toHaveLength(1);
    expect(wb.sheets[0].name).toBe('Area Fares');

    const grid = wb.sheets[0].rows;
    // Lower-triangle mirror cells use IF(...="","",...) so blank cells do not evaluate to 0.00.
    const mirrorCells = grid.flat().filter((c) => Boolean(c.formula));
    expect(mirrorCells.length).toBeGreaterThan(0);
    expect(mirrorCells.every((c) => /^IF\([A-Z]+\d+="",\s*"",[A-Z]+\d+\)$/.test(c.formula!))).toBe(true);

    const parsed = parseFareGrid(grid, areas, existingFares);
    expect(parsed.layout).toBe('matrix');
    expect(parsed.issues).toEqual([]);
    expect(parsed.unknownAreas).toEqual([]);
    // Only the 1 set fare (KK <-> UT = 15.00) appears; blank cells are not parsed as 0.
    expect(parsed.rows).toHaveLength(1);
    expect(parsed.rows[0].status).toBe('unchanged');
    expect(parsed.rows[0].amountCents).toBe(1500);

    // Ensure Pacific/RM and CJK area appear in headers and never get stripped.
    const headerNames = grid[0].map((c) => c.value);
    expect(headerNames).toContain('Pacific/RM');
    expect(headerNames).toContain('\u516b\u6253\u7075');
    expect(headerNames).not.toContain('Old Town');
  });

  it('reads DEFLATE-compressed multi-sheet .xlsx with bit-3 descriptors, rich text + rPh, inlineStr, and IEEE floats', async () => {
    const enc = new TextEncoder();
    const zipBytes = await buildDeflatedZipWithDataDescriptor([
      {
        name: '[Content_Types].xml',
        data: enc.encode(
          `<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/></Types>`,
        ),
      },
      {
        name: '_rels/.rels',
        data: enc.encode(
          `<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="/xl/workbook.xml"/></Relationships>`,
        ),
      },
      {
        name: 'xl/workbook.xml',
        data: enc.encode(
          `<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Notes" sheetId="1" r:id="rId1"/><sheet name="Rates" sheetId="2" r:id="rId2"/></sheets></workbook>`,
        ),
      },
      {
        name: 'xl/_rels/workbook.xml.rels',
        data: enc.encode(
          `<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="SharedStrings.xml"/></Relationships>`,
        ),
      },
      {
        name: 'xl/SharedStrings.xml',
        data: enc.encode(
          `<?xml version="1.0" encoding="UTF-8"?><sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="2" uniqueCount="2"><si><r><t>\u516b\u6253</t></r><r><t>\u7075</t></r><rPh sb="0" eb="3"><t>IGNORE</t></rPh></si><si><t>UT</t></si></sst>`,
        ),
      },
      {
        name: 'xl/worksheets/sheet1.xml',
        data: enc.encode(
          `<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Header only</t></is></c></row></sheetData></worksheet>`,
        ),
      },
      {
        name: 'xl/worksheets/sheet2.xml',
        data: enc.encode(
          `<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>` +
            `<row r="1"><c r="A1" t="inlineStr"><is><t>2026 Fares</t></is></c></row>` +
            `<row r="2"><c r="A2" t="inlineStr"><is><t>From</t></is></c><c r="B2" t="s"><v>0</v></c><c r="C2" t="s"><v>1</v></c></row>` +
            `<row r="3"><c r="A3" t="s"><v>0</v></c><c r="B3"><v>5</v></c><c r="C3"><v>15.100000000000001</v></c></row>` +
            `<row r="4"><c r="A4" t="s"><v>1</v></c><c r="B4"><f>IF(C3="","",C3)</f><v>99</v></c><c r="C4"><f>10+5</f><v>15</v></c></row>` +
            `</sheetData></worksheet>`,
        ),
      },
    ]);

    const res = await readFareFile(makeFile(zipBytes, 'rates.xlsx'), areas, existingFares);
    expect(res.sheets).toHaveLength(2);
    expect(res.sheets[1].name).toBe('Rates');
    const parsed = res.sheets[1].parsed;
    expect(parsed.issues).toEqual([]);
    expect(parsed.rows).toHaveLength(3);
    const pjToUt = parsed.rows.find(
      (r) =>
        (r.areaAName === '\u516b\u6253\u7075' && r.areaBName === 'UT') ||
        (r.areaAName === 'UT' && r.areaBName === '\u516b\u6253\u7075'),
    );
    expect(pjToUt?.amountCents).toBe(1510);
    const utWithin = parsed.rows.find((r) => r.areaAName === 'UT' && r.areaBName === 'UT');
    expect(utWithin?.amountCents).toBe(1500);
  });

  it('rejects legacy .xls, corrupt zip, and oversized files with explicit codes', async () => {
    const ole2 = new Uint8Array([0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1, 0, 0]);
    await expect(readFareFile(makeFile(ole2, 'old.xls'), areas, [])).rejects.toMatchObject({
      name: 'XlsxError',
      code: 'LEGACY_XLS',
    });

    const badZip = new Uint8Array([0x50, 0x4b, 0x03, 0x04, 1, 2, 3, 4]);
    await expect(readXlsxWorkbook(badZip)).rejects.toBeInstanceOf(XlsxError);
    await expect(readFareFile(makeFile(badZip, 'broken.xlsx'), areas, [])).rejects.toMatchObject({
      code: 'INVALID_XLSX',
    });

    const bigFile = new File(['a'], 'huge.csv', { type: 'text/csv' });
    Object.defineProperty(bigFile, 'size', { value: 6 * 1024 * 1024 });
    await expect(readFareFile(bigFile, areas, [])).rejects.toMatchObject({
      code: 'TOO_LARGE',
    });
  });

  it('round-trips CSV export and supports semicolon/tab delimiters, RM/MYR prefixes, and formula neutralization', () => {
    const customAreas: FareArea[] = [
      ...areas,
      { id: 6, name: 'Bandar, Utama', active: true },
      { id: 7, name: '=FormulaArea', active: true },
    ];
    const csv = buildFaresCsv(customAreas, existingFares, 'From \\ To');
    expect(csv.startsWith('\uFEFF')).toBe(true);
    expect(csv).toContain("'=FormulaArea");
    expect(csv).toContain('"Bandar, Utama"');

    const rows = parseCsv(csv);
    expect(rows[0]).toContain('Bandar, Utama');

    // Semicolon-separated list with RM / MYR prefixes and blank markers
    const semiCsv = [
      'Origin;Destination;Fare (MYR)',
      'UT;KK;RM 18.00',
      'KK;Pacific/RM;MYR 12.5',
      'UT;UT;0',
      'KK;KK;\u2014',
      'UT;Pacific/RM;N/A',
    ].join('\n');
    const parsedSemi = parseFareGrid(
      parseCsv(semiCsv).map((r, ri) => r.map((v, ci) => ({ row: ri, col: ci, value: v }))),
      areas,
      existingFares,
    );
    expect(parsedSemi.layout).toBe('list');
    expect(parsedSemi.issues).toEqual([]);
    expect(parsedSemi.rows).toHaveLength(3);
    expect(parsedSemi.rows[0]).toMatchObject({ amountCents: 1800, status: 'update' });
    expect(parsedSemi.rows[1]).toMatchObject({ amountCents: 1250, status: 'new' });
    expect(parsedSemi.rows[2]).toMatchObject({ amountCents: 0, status: 'new' });

    expect(parseFareAmountRm('500')).toEqual({ kind: 'ok', cents: 50000 });
    expect(parseFareAmountRm('500.01')).toEqual({ kind: 'invalid' });
    expect(parseFareAmountRm('-5')).toEqual({ kind: 'invalid' });
    expect(parseFareAmountRm('Not set')).toEqual({ kind: 'blank' });
  });

  it('auto-detects 1-area matrix, 2-area matrix, ZH/EN 3-col lists, and headerless 3-col lists', () => {
    // 1-area matrix with blank corner
    const oneArea = parseFareGrid(
      [
        ['', 'UT'].map((v, c) => ({ row: 0, col: c, value: v })),
        ['UT', '5'].map((v, c) => ({ row: 1, col: c, value: v })),
      ],
      areas,
      [],
    );
    expect(oneArea.layout).toBe('matrix');
    expect(oneArea.rows).toHaveLength(1);
    expect(oneArea.rows[0].amountCents).toBe(500);

    // 2-area 3-column matrix must be detected as matrix, not list
    const twoArea = parseFareGrid(
      [
        ['From \\ To', 'UT', 'KK'],
        ['UT', '5', '15'],
        ['KK', '', '6'],
      ].map((r, ri) => r.map((v, ci) => ({ row: ri, col: ci, value: v }))),
      areas,
      existingFares,
    );
    expect(twoArea.layout).toBe('matrix');
    expect(twoArea.rows).toHaveLength(3);

    // Chinese header 3-column list
    const zhList = parseFareGrid(
      [
        ['\u51fa\u53d1\u5730\u533a', '\u5230\u8fbe\u5730\u533a', '\u8f66\u8d39 (RM)'],
        ['UT', 'KK', '15'],
      ].map((r, ri) => r.map((v, ci) => ({ row: ri, col: ci, value: v }))),
      areas,
      existingFares,
    );
    expect(zhList.layout).toBe('list');
    expect(zhList.rows).toHaveLength(1);
    expect(zhList.rows[0].status).toBe('unchanged');

    // Headerless 3-column list
    const headerless = parseFareGrid(
      [
        ['UT', 'KK', '12'],
        ['KK', 'Pacific/RM', '10'],
        ['Pacific/RM', 'UT', '14'],
      ].map((r, ri) => r.map((v, ci) => ({ row: ri, col: ci, value: v }))),
      areas,
      existingFares,
    );
    expect(headerless.layout).toBe('list');
    expect(headerless.rows).toHaveLength(3);
  });

  it('reports asymmetric conflicts, invalid amounts, and unknown areas', () => {
    const grid = [
      ['From \\ To', 'UT', 'KK', 'Subang'],
      ['UT', '5', '15', '20'],
      ['KK', '18', 'bad', ''],
    ].map((r, ri) => r.map((v, ci) => ({ row: ri, col: ci, value: v })));

    const parsed = parseFareGrid(grid, areas, existingFares);
    expect(parsed.issues.map((i) => i.kind)).toEqual(['conflict', 'invalid_amount']);
    expect(parsed.unknownAreas).toEqual(['Subang']);
    expect(parsed.rows.find((r) => r.areaBName === 'Subang')?.status).toBe('unknown_area');
  });
});
