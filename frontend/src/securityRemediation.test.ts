import { beforeEach, describe, expect, it } from 'vitest';
import { generateIcs } from './calendar';
import type { Ride } from './components/RideCard';
import { safeInternalPath } from './guideLinks';
import {
  clearOfflineSnapshots,
  loadTodayRidesSnapshot,
  saveTodayRidesSnapshot,
} from './offlineSnapshot';
import { readXlsxWorkbook, XlsxError } from './xlsxCodec';

const CRC32_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let i = 0; i < 256; i += 1) {
    let c = i;
    for (let k = 0; k < 8; k += 1) {
      c = (c & 1) !== 0 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
    table[i] = c >>> 0;
  }
  return table;
})();

function crc32(bytes: Uint8Array): number {
  let c = 0xffffffff;
  for (let i = 0; i < bytes.length; i += 1) {
    c = CRC32_TABLE[(c ^ bytes[i]) & 0xff] ^ (c >>> 8);
  }
  return (c ^ 0xffffffff) >>> 0;
}

function buildStoredZip(files: { name: string; content: string }[]): Uint8Array {
  const enc = new TextEncoder();
  const localParts: Uint8Array[] = [];
  const cdParts: Uint8Array[] = [];
  let offset = 0;

  for (const file of files) {
    const nameBytes = enc.encode(file.name);
    const dataBytes = enc.encode(file.content);
    const sum = crc32(dataBytes);
    const size = dataBytes.byteLength;

    const local = new Uint8Array(30 + nameBytes.length);
    const lv = new DataView(local.buffer);
    lv.setUint32(0, 0x04034b50, true);
    lv.setUint16(4, 20, true);
    lv.setUint16(6, 0x0800, true);
    lv.setUint16(8, 0, true); // STORE
    lv.setUint32(14, sum, true);
    lv.setUint32(18, size, true);
    lv.setUint32(22, size, true);
    lv.setUint16(26, nameBytes.length, true);
    local.set(nameBytes, 30);

    const cd = new Uint8Array(46 + nameBytes.length);
    const cv = new DataView(cd.buffer);
    cv.setUint32(0, 0x02014b50, true);
    cv.setUint16(4, 20, true);
    cv.setUint16(6, 20, true);
    cv.setUint16(8, 0x0800, true);
    cv.setUint16(10, 0, true);
    cv.setUint32(16, sum, true);
    cv.setUint32(20, size, true);
    cv.setUint32(24, size, true);
    cv.setUint16(28, nameBytes.length, true);
    cv.setUint32(42, offset, true);
    cd.set(nameBytes, 46);

    localParts.push(local, dataBytes);
    cdParts.push(cd);
    offset += local.byteLength + size;
  }

  let cdSize = 0;
  for (const p of cdParts) cdSize += p.byteLength;

  const eocd = new Uint8Array(22);
  const ev = new DataView(eocd.buffer);
  ev.setUint32(0, 0x06054b50, true);
  ev.setUint16(8, files.length, true);
  ev.setUint16(10, files.length, true);
  ev.setUint32(12, cdSize, true);
  ev.setUint32(16, offset, true);

  const out = new Uint8Array(offset + cdSize + eocd.byteLength);
  let pos = 0;
  for (const p of localParts) {
    out.set(p, pos);
    pos += p.byteLength;
  }
  for (const p of cdParts) {
    out.set(p, pos);
    pos += p.byteLength;
  }
  out.set(eocd, pos);
  return out;
}

function buildMinimalXlsx(opts: {
  sheets: { name: string; rId: string; target: string }[];
  worksheets: Record<string, string>;
}): Uint8Array {
  const sheetXmlTags = opts.sheets
    .map(
      (s, i) =>
        `<sheet name="${s.name}" sheetId="${i + 1}" r:id="${s.rId}"/>`,
    )
    .join('');
  const relsTags = opts.sheets
    .filter((s, idx, arr) => arr.findIndex((o) => o.rId === s.rId) === idx)
    .map(
      (s) =>
        `<Relationship Id="${s.rId}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="${s.target}"/>`,
    )
    .join('');

  const files: { name: string; content: string }[] = [
    {
      name: '_rels/.rels',
      content:
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>' +
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">' +
        '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>' +
        '</Relationships>',
    },
    {
      name: 'xl/workbook.xml',
      content:
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>' +
        '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" ' +
        'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">' +
        `<sheets>${sheetXmlTags}</sheets></workbook>`,
    },
    {
      name: 'xl/_rels/workbook.xml.rels',
      content:
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>' +
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">' +
        `${relsTags}</Relationships>`,
    },
  ];
  for (const [path, body] of Object.entries(opts.worksheets)) {
    files.push({
      name: path,
      content:
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>' +
        '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">' +
        `<sheetData>${body}</sheetData></worksheet>`,
    });
  }
  return buildStoredZip(files);
}

describe('BugHunter security remediation (frontend)', () => {
  beforeEach(() => {
    localStorage.clear();
    window.dispatchEvent(new Event('gr:session-unlocked'));
  });

  it('safeInternalPath allows safe internal routes and blocks traversal, protocol, and /api URLs', () => {
    expect(safeInternalPath('/rides', null)).toBe('/rides');
    expect(safeInternalPath('/events/12?tab=info#top', null)).toBe('/events/12?tab=info#top');

    const dangerous = [
      '//evil.com/phish',
      '/\\evil.com/phish',
      'javascript:alert(1)',
      '/rides:1',
      '/rides\nSet-Cookie:a=b',
      '/rides\x00/admin',
      '/api',
      '/api/auth/logout',
      '/rides/../api/admin',
      'https://evil.example/rides',
    ];
    for (const candidate of dangerous) {
      expect(safeInternalPath(candidate, null), candidate).toBeNull();
      expect(safeInternalPath(candidate, '/'), candidate).toBe('/');
    }
  });

  it('readXlsxWorkbook rejects >20 sheets, duplicate sheet targets, and oversized cell grids', async () => {
    const tooManySheets = Array.from({ length: 21 }, (_, idx) => ({
      name: `Sheet${idx + 1}`,
      rId: `rId${idx + 1}`,
      target: `worksheets/sheet${idx + 1}.xml`,
    }));
    const excessSheetsBytes = buildMinimalXlsx({
      sheets: tooManySheets,
      worksheets: {
        'xl/worksheets/sheet1.xml': '<row r="1"><c r="A1"><v>1</v></c></row>',
      },
    });
    await expect(readXlsxWorkbook(excessSheetsBytes)).rejects.toMatchObject({
      name: 'XlsxError',
      code: 'TOO_LARGE',
    } satisfies Partial<XlsxError>);

    const dupSheetBytes = buildMinimalXlsx({
      sheets: [
        { name: 'One', rId: 'rId1', target: 'worksheets/sheet1.xml' },
        { name: 'Two', rId: 'rId2', target: 'worksheets/sheet1.xml' },
      ],
      worksheets: {
        'xl/worksheets/sheet1.xml': '<row r="1"><c r="A1"><v>1</v></c></row>',
      },
    });
    await expect(readXlsxWorkbook(dupSheetBytes)).rejects.toMatchObject({
      name: 'XlsxError',
      code: 'INVALID_XLSX',
    } satisfies Partial<XlsxError>);

    // 5,001 rows exceeds MAX_SHEET_ROWS (5,000)
    const overRowBytes = buildMinimalXlsx({
      sheets: [{ name: 'OverRow', rId: 'rId1', target: 'worksheets/sheet1.xml' }],
      worksheets: {
        'xl/worksheets/sheet1.xml': '<row r="5001"><c r="A5001"><v>99</v></c></row>',
      },
    });
    await expect(readXlsxWorkbook(overRowBytes)).rejects.toMatchObject({
      name: 'XlsxError',
      code: 'TOO_LARGE',
    } satisfies Partial<XlsxError>);

    // Col IN (248, < 500) x Row 500 (< 5,000) = 124,000 dense cells > MAX_GRID_CELLS (100,000)
    const overGridBytes = buildMinimalXlsx({
      sheets: [{ name: 'OverGrid', rId: 'rId1', target: 'worksheets/sheet1.xml' }],
      worksheets: {
        'xl/worksheets/sheet1.xml': '<row r="500"><c r="IN500"><v>99</v></c></row>',
      },
    });
    await expect(readXlsxWorkbook(overGridBytes)).rejects.toMatchObject({
      name: 'XlsxError',
      code: 'TOO_LARGE',
    } satisfies Partial<XlsxError>);
  });

  it('generateIcs strips C0 controls and normalizes bare CR to escaped newlines', () => {
    const ics = generateIcs({
      title: 'Worship\rRide\x00\x07',
      description: 'Gate 4\r\nNote\x1fLine\x7fEnd',
      location: 'Main\rHall\x0bWing',
      startsAt: '2026-11-15T09:30:00',
    });

    const normalizedLineEndings = ics.replace(/\r\n/g, '\n');
    expect(normalizedLineEndings).not.toContain('\r');
    const hasControl = [...ics].some((ch) => {
      const code = ch.charCodeAt(0);
      return (
        code <= 0x08 ||
        code === 0x0b ||
        code === 0x0c ||
        (code >= 0x0e && code <= 0x1f) ||
        code === 0x7f
      );
    });
    expect(hasControl).toBe(false);
    expect(ics).toContain('SUMMARY:Worship\\nRide');
    expect(ics).toContain('DESCRIPTION:Gate 4\\nNoteLineEnd');
    expect(ics).toContain('LOCATION:Main\\nHallWing');
  });

  it('saveTodayRidesSnapshot refuses and clears snapshots while session is locked', () => {
    const now = new Date();
    const pad = (n: number) => String(n).padStart(2, '0');
    const todayIso = `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}T09:00:00`;
    const activeRide: Ride = {
      id: 42,
      status: 'confirmed',
      pickup_at: todayIso,
      area: { id: 1, name: 'SS2' },
      pickup_address: '10 Jalan SS2',
      destination: 'PJ State CMC',
      seats: 1,
      assistance: '',
      notes: '',
      event: null,
      trip_id: null,
      passenger: { id: 7, name: 'Member', phone: null },
      driver: { id: 8, name: 'Driver', phone: null, vehicle: null },
      needs_attention: false,
      attention_reason: '',
      started_at: null,
      picked_up_at: null,
      completed_at: null,
      updated_at: todayIso,
      viewer: { is_passenger: true, is_driver: false, is_staff: false },
    };

    saveTodayRidesSnapshot(7, [activeRide]);
    expect(loadTodayRidesSnapshot(7)?.rides).toHaveLength(1);

    localStorage.setItem('gr_locked_user', '1');
    window.dispatchEvent(new Event('gr:session-locked'));
    saveTodayRidesSnapshot(7, [activeRide]);

    expect(loadTodayRidesSnapshot(7)).toBeNull();
    expect(localStorage.getItem('gr_today_rides_v1')).toBeNull();

    localStorage.removeItem('gr_locked_user');
    window.dispatchEvent(new Event('gr:session-unlocked'));
    saveTodayRidesSnapshot(7, [activeRide]);
    expect(loadTodayRidesSnapshot(7)?.rides).toHaveLength(1);
    clearOfflineSnapshots();
  });
});
