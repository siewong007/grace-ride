import { describe, expect, it, vi } from 'vitest';
import {
  downloadCsv,
  escapeCsv,
  exportReportPdf,
  fmtRmExport,
  formatInclusivePeriodEnd,
} from './reportExport';

vi.mock('jspdf', () => ({
  default: class {
    setFont() {}
    setFontSize() {}
    splitTextToSize(s: string) {
      return [s];
    }
    getTextWidth() {
      return 10;
    }
    text() {}
    setFillColor() {}
    rect() {}
    addPage() {}
    save() {}
  },
}));

describe('reportExport helpers', () => {
  it('neutralizes formula-injection prefixes and escapes commas/quotes', () => {
    expect(escapeCsv('=SUM(A1:A2)')).toBe("'=SUM(A1:A2)");
    expect(escapeCsv('  +cmd')).toBe("'  +cmd");
    expect(escapeCsv('-1+1')).toBe("'-1+1");
    expect(escapeCsv('@HYPERLINK("x")')).toBe('"\'@HYPERLINK(""x"")"');
    expect(escapeCsv('Hello, "World"')).toBe('"Hello, ""World"""');
    expect(escapeCsv(42)).toBe('42');
    expect(escapeCsv(null)).toBe('');
  });

  it('formats positive and negative cents cleanly in RM', () => {
    expect(fmtRmExport(0)).toBe('RM 0.00');
    expect(fmtRmExport(1250)).toBe('RM 12.50');
    expect(fmtRmExport(-305)).toBe('-RM 3.05');
  });

  it('prepends UTF-8 BOM and joins lines with CRLF when downloading CSV', async () => {
    let capturedBlob: Blob | null = null;
    window.URL.createObjectURL = vi.fn((b: Blob) => {
      capturedBlob = b;
      return 'blob:test';
    });
    window.URL.revokeObjectURL = vi.fn();
    const clickSpy = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});

    downloadCsv('test-report', ['Col1,Col2', 'A,B']);
    expect(clickSpy).toHaveBeenCalledTimes(1);
    expect(capturedBlob).not.toBeNull();
    const bytes = new Uint8Array(await capturedBlob!.arrayBuffer());
    expect(Array.from(bytes.slice(0, 3))).toEqual([0xef, 0xbb, 0xbf]);
    const text = new TextDecoder('utf-8').decode(bytes);
    expect(text).toContain('Col1,Col2\r\nA,B');
    clickSpy.mockRestore();
  });

  it('exports multi-section PDFs without throwing', async () => {
    await expect(
      exportReportPdf({
        fileBase: 'pdf-test',
        title: 'Test Report',
        meta: ['Period: 2026-11'],
        summaryHeading: 'Summary',
        summaryRows: [['Total', '10']],
        sections: [
          { heading: 'Empty', items: [] },
          { heading: 'Items', items: ['Line 1', 'Line 2'] },
        ],
      }),
    ).resolves.toBeUndefined();
  });

  it('converts half-open month boundaries to inclusive dates while preserving inclusive dates', () => {
    expect(formatInclusivePeriodEnd('2026-12-01', '2026-11-30')).toBe('2026-11-30');
    expect(formatInclusivePeriodEnd('2026-12-01')).toBe('2026-11-30');
    expect(formatInclusivePeriodEnd('2024-03-01')).toBe('2024-02-29'); // leap year
    expect(formatInclusivePeriodEnd('2026-11-30')).toBe('2026-11-30');
  });
});
