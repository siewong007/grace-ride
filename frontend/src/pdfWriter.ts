/**
 * One small drawing surface for the app's PDF exports (A4, millimetres).
 *
 * jsPDF's built-in fonts only know Latin letters, so Chinese would come out as garbage. When
 * any text is not Latin the pages are drawn on a canvas instead, using the device's own fonts,
 * and placed in the PDF as images (the text is then not selectable, but it is right). Latin-only
 * exports keep real, selectable PDF text. Callers lay out the page the same way for both.
 */

export interface TextOptions {
  /** Point size; defaults to 11. */
  size?: number;
  bold?: boolean;
  /** Wrap to this width (mm). Without `maxLines` a long text takes as many lines as it needs. */
  maxWidth?: number;
  /** Cut off after this many lines, ending in an ellipsis. */
  maxLines?: number;
  /** `right`: `x` is the right edge of the text. */
  align?: 'left' | 'right';
}

export interface PdfWriter {
  readonly width: number;
  readonly height: number;
  /** Draws text with its top-left at (x, y); returns the height it used, in mm. */
  text(s: string, x: number, y: number, o?: TextOptions): number;
  /** A light-grey filled box, e.g. a table header band. */
  band(x: number, y: number, w: number, h: number): void;
  addPage(): void;
  save(filename: string): void;
}

const PAGE_W = 210;
const PAGE_H = 297;
const MM_PER_PT = 25.4 / 72;
const LINE = 1.35; // line height as a multiple of the font size

const CP1252_EXTRA = new Set([
  0x20ac, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0x02c6, 0x2030,
  0x0160, 0x2039, 0x0152, 0x017d, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022,
  0x2013, 0x2014, 0x02dc, 0x2122, 0x0161, 0x203a, 0x0153, 0x017e, 0x0178,
]);

/** True when every character is representable in PDF WinAnsi/CP1252 encoding. */
export function isLatin(s: string): boolean {
  for (let i = 0; i < s.length; i++) {
    const c = s.charCodeAt(i);
    const isAscii = c === 0x09 || c === 0x0a || c === 0x0d || (c >= 0x20 && c <= 0x7e);
    const isLatin1 = c >= 0xa0 && c <= 0xff;
    if (!isAscii && !isLatin1 && !CP1252_EXTRA.has(c)) return false;
  }
  return true;
}

export async function createPdfWriter(texts: string[]): Promise<PdfWriter> {
  const { default: jsPDF } = await import('jspdf');
  return texts.every(isLatin) ? vectorWriter(new jsPDF()) : canvasWriter(jsPDF);
}

type Doc = InstanceType<typeof import('jspdf').default>;

function vectorWriter(doc: Doc): PdfWriter {
  const lineH = (size: number) => size * MM_PER_PT * LINE;
  return {
    width: PAGE_W,
    height: PAGE_H,
    text(s, x, y, { size = 11, bold = false, maxWidth, maxLines, align = 'left' } = {}) {
      doc.setFont('helvetica', bold ? 'bold' : 'normal');
      doc.setFontSize(size);
      let lines: string[] = maxWidth ? doc.splitTextToSize(s, maxWidth) : [s];
      if (maxLines && lines.length > maxLines) {
        lines = lines.slice(0, maxLines);
        lines[maxLines - 1] = `${lines[maxLines - 1].replace(/\s+\S*$/, '')}…`;
      }
      lines.forEach((ln, i) => {
        const w = align === 'right' ? doc.getTextWidth(ln) : 0;
        // jsPDF positions the baseline, so move down by about one font size.
        doc.text(ln, x - w, y + size * MM_PER_PT + i * lineH(size));
      });
      return lines.length * lineH(size);
    },
    band(x, y, w, h) {
      doc.setFillColor(245, 245, 245);
      doc.rect(x, y, w, h, 'F');
    },
    addPage: () => void doc.addPage(),
    save: (filename) => void doc.save(filename),
  };
}

const SCALE = 8; // canvas pixels per mm (about 200 dpi)
const FONT = '"PingFang SC", "Noto Sans CJK SC", "Microsoft YaHei", "Hiragino Sans GB", "Heiti SC", sans-serif';

function canvasWriter(JsPdf: typeof import('jspdf').default): PdfWriter {
  const pages: HTMLCanvasElement[] = [];
  let ctx: CanvasRenderingContext2D;
  const newPage = () => {
    const canvas = document.createElement('canvas');
    canvas.width = PAGE_W * SCALE;
    canvas.height = PAGE_H * SCALE;
    const c = canvas.getContext('2d');
    if (!c) throw new Error('Canvas is not available');
    c.fillStyle = '#fff';
    c.fillRect(0, 0, canvas.width, canvas.height);
    c.fillStyle = '#000';
    c.textBaseline = 'top';
    pages.push(canvas);
    ctx = c;
  };
  newPage();

  const lineH = (size: number) => size * MM_PER_PT * LINE;
  /** Greedy wrap that also breaks inside runs of Chinese and honours explicit newlines. */
  const wrap = (s: string, maxPx: number): string[] => {
    const out: string[] = [];
    for (const para of s.split(/\r?\n/)) {
      let line = '';
      for (const ch of para) {
        if (line && ctx.measureText(line + ch).width > maxPx) {
          out.push(line);
          line = ch === ' ' ? '' : ch;
        } else {
          line += ch;
        }
      }
      out.push(line);
    }
    return out;
  };

  return {
    width: PAGE_W,
    height: PAGE_H,
    text(s, x, y, { size = 11, bold = false, maxWidth, maxLines, align = 'left' } = {}) {
      ctx.font = `${bold ? '700 ' : ''}${size * MM_PER_PT * SCALE}px ${FONT}`;
      let lines = maxWidth ? wrap(s, maxWidth * SCALE) : [s];
      if (maxLines && lines.length > maxLines) {
        lines = lines.slice(0, maxLines);
        let last = lines[maxLines - 1];
        while (last && ctx.measureText(`${last}…`).width > (maxWidth ?? Infinity) * SCALE) last = last.slice(0, -1);
        lines[maxLines - 1] = `${last}…`;
      }
      ctx.textAlign = align === 'right' ? 'right' : 'left';
      lines.forEach((ln, i) => ctx.fillText(ln, x * SCALE, (y + i * lineH(size)) * SCALE));
      return lines.length * lineH(size);
    },
    band(x, y, w, h) {
      ctx.fillStyle = '#f5f5f5';
      ctx.fillRect(x * SCALE, y * SCALE, w * SCALE, h * SCALE);
      ctx.fillStyle = '#000';
    },
    addPage: newPage,
    save(filename) {
      const doc = new JsPdf({ unit: 'mm', format: 'a4' });
      pages.forEach((canvas, i) => {
        if (i > 0) doc.addPage();
        doc.addImage(canvas.toDataURL('image/jpeg', 0.92), 'JPEG', 0, 0, PAGE_W, PAGE_H);
      });
      doc.save(filename);
    },
  };
}
