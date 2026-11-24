import { createPdfWriter } from './pdfWriter';

/**
 * Escapes a CSV cell and neutralizes spreadsheet formula-injection prefixes
 * (`=`, `+`, `-`, `@`, tab, CR, or trimmed leading `=`, `+`, `-`, `@`).
 */
export function escapeCsv(v: unknown): string {
  const raw =
    typeof v === 'string' && (/^[=+\-@\t\r]/.test(v) || /^[=+\-@]/.test(v.trimStart()))
      ? `'${v}`
      : String(v ?? '');
  if (/[",\n\r]/.test(raw)) return `"${raw.replace(/"/g, '""')}"`;
  return raw;
}

/** Formats integer cents cleanly as `RM X.XX` (handling negative amounts safely). */
export function fmtRmExport(cents: number): string {
  const sign = cents < 0 ? '-' : '';
  const abs = Math.abs(cents);
  return `${sign}RM ${(abs / 100).toFixed(2)}`;
}

/** Downloads a CSV file with UTF-8 BOM so Excel renders Chinese & English text cleanly. */
export function downloadCsv(fileBase: string, lines: string[]): void {
  const csv = `\uFEFF${lines.join('\r\n')}`;
  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `${fileBase}.csv`;
  a.click();
  URL.revokeObjectURL(url);
}

export interface PdfReportSection {
  heading: string;
  items: string[];
}

export interface ExportReportPdfOptions {
  fileBase: string;
  title: string;
  meta: string[];
  summaryHeading?: string;
  summaryRows?: [string, string][];
  sections?: PdfReportSection[];
}

/** Renders a structured multi-section PDF report via the CJK-capable `createPdfWriter`. */
export async function exportReportPdf({
  fileBase,
  title,
  meta,
  summaryHeading,
  summaryRows = [],
  sections = [],
}: ExportReportPdfOptions): Promise<void> {
  const nonEmptySections = sections.filter((s) => s.items.length > 0);
  const allStrings = [
    title,
    ...meta,
    ...(summaryHeading ? [summaryHeading] : []),
    ...summaryRows.flat(),
    ...nonEmptySections.flatMap((s) => [s.heading, ...s.items]),
  ];
  const w = await createPdfWriter(allStrings);
  const M = 20;
  const body = w.width - M - 25;
  let y = 20;

  const room = () => {
    if (y > w.height - 40) {
      w.addPage();
      y = 20;
    }
  };
  const heading = (text: string) => {
    room();
    y += w.text(text, M, y, { size: 14, bold: true }) + 3;
  };
  const list = (items: string[]) => {
    for (const item of items) {
      room();
      y += w.text(item, M + 5, y, { maxWidth: body }) + 1.5;
    }
    y += 6;
  };

  y += w.text(title, M, y, { size: 18, bold: true }) + 4;
  for (const line of meta) {
    room();
    y += w.text(line, M, y, { maxWidth: body }) + 1;
  }
  y += 8;

  if (summaryHeading && summaryRows.length > 0) {
    heading(summaryHeading);
    for (const [label, val] of summaryRows) {
      room();
      w.text(label, M + 5, y);
      y += w.text(val, w.width - 40, y, { align: 'right' }) + 1;
    }
    y += 6;
  }

  for (const sec of nonEmptySections) {
    heading(sec.heading);
    list(sec.items);
  }

  w.save(`${fileBase}.pdf`);
}
