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

/** Formats an export/display end date as inclusive `YYYY-MM-DD`. */
export function formatInclusivePeriodEnd(periodEnd: string, periodEndInclusive?: string): string {
  if (periodEndInclusive) return periodEndInclusive;
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(periodEnd);
  if (!match) return periodEnd;
  const [, ys, ms, ds] = match;
  if (ds !== '01') return periodEnd;
  const dt = new Date(Date.UTC(Number(ys), Number(ms) - 1, 0));
  return `${dt.getUTCFullYear()}-${String(dt.getUTCMonth() + 1).padStart(2, '0')}-${String(dt.getUTCDate()).padStart(2, '0')}`;
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
  generatedAt?: string;
}

function defaultGeneratedAt(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** Renders a structured multi-section PDF report via the CJK-capable `createPdfWriter`. */
export async function exportReportPdf({
  fileBase,
  title,
  meta,
  summaryHeading,
  summaryRows = [],
  sections = [],
  generatedAt,
}: ExportReportPdfOptions): Promise<void> {
  const stamp = generatedAt ?? defaultGeneratedAt();
  const fullMeta = [...meta, stamp];
  const nonEmptySections = sections.filter((s) => s.items.length > 0);
  const allStrings = [
    title,
    ...fullMeta,
    ...(summaryHeading ? [summaryHeading] : []),
    ...summaryRows.flat(),
    ...nonEmptySections.flatMap((s) => [s.heading, ...s.items]),
    '- 1 -',
  ];
  const w = await createPdfWriter(allStrings);
  const M = 20;
  const body = w.width - 2 * M - 5;
  let y = 20;
  let pageNum = 1;

  const drawFooter = () => {
    w.text(`- ${pageNum} -`, w.width - M, w.height - 12, { size: 9, align: 'right' });
  };
  const nextPage = () => {
    drawFooter();
    w.addPage();
    pageNum += 1;
    y = 20;
  };
  const room = (needed = 16) => {
    if (y > w.height - 20 - needed) {
      nextPage();
    }
  };
  const heading = (text: string) => {
    room(18);
    w.band(M, y - 1, w.width - 2 * M, 7.5);
    y += w.text(text, M + 2, y + 0.5, { size: 13, bold: true }) + 4;
  };
  const list = (items: string[]) => {
    for (const item of items) {
      room(14);
      y += w.text(item, M + 4, y, { maxWidth: body }) + 1.5;
    }
    y += 5;
  };

  y += w.text(title, M, y, { size: 18, bold: true }) + 4;
  for (const line of fullMeta) {
    room(12);
    y += w.text(line, M, y, { size: 10, maxWidth: body + 5 }) + 1;
  }
  y += 6;

  if (summaryHeading && summaryRows.length > 0) {
    heading(summaryHeading);
    const valX = w.width - M;
    const labelW = w.width - 2 * M - 58;
    for (const [label, val] of summaryRows) {
      room(14);
      const lh = w.text(label, M + 4, y, { maxWidth: labelW });
      const vh = w.text(val, valX, y, { align: 'right' });
      y += Math.max(lh, vh) + 1.5;
    }
    y += 5;
  }

  for (const sec of nonEmptySections) {
    heading(sec.heading);
    list(sec.items);
  }

  drawFooter();
  w.save(`${fileBase}.pdf`);
}
