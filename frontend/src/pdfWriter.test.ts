import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPdfWriter, isLatin } from './pdfWriter';

const mocks = vi.hoisted(() => ({
  vector: { text: vi.fn(), setFont: vi.fn(), setFontSize: vi.fn(), getTextWidth: vi.fn(() => 10), setFillColor: vi.fn(), rect: vi.fn(), addPage: vi.fn(), save: vi.fn(), splitTextToSize: vi.fn((s: string) => [s]), addImage: vi.fn() },
}));

vi.mock('jspdf', () => ({ default: class { constructor() { return mocks.vector; } } }));

describe('isLatin', () => {
  it('accepts letters the built-in PDF fonts can draw and rejects Chinese', () => {
    expect(isLatin('Sister Mary — Café 2026 · #4')).toBe(true);
    expect(isLatin('\u9648\u739b\u4e3d')).toBe(false);
    expect(isLatin('Mary \u9648')).toBe(false);
  });
});

describe('createPdfWriter', () => {
  const fills: string[] = [];
  const ctx = {
    fillStyle: '', font: '', textAlign: '', textBaseline: '',
    fillRect: vi.fn(),
    fillText: vi.fn((s: string) => fills.push(s)),
    measureText: (s: string) => ({ width: [...s].length * 10 }),
  };

  beforeEach(() => {
    Object.values(mocks.vector).forEach((f) => f.mockClear());
    fills.length = 0;
    ctx.fillText.mockClear();
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(ctx as unknown as CanvasRenderingContext2D);
    vi.spyOn(HTMLCanvasElement.prototype, 'toDataURL').mockReturnValue('data:image/jpeg;base64,AAAA');
  });

  it('keeps real PDF text when everything is Latin', async () => {
    const w = await createPdfWriter(['Member report', 'Mary']);
    const h = w.text('Member report', 20, 20, { size: 18, bold: true });
    expect(h).toBeGreaterThan(0);
    expect(mocks.vector.text).toHaveBeenCalledWith('Member report', 20, expect.any(Number));
    expect(mocks.vector.setFont).toHaveBeenCalledWith('helvetica', 'bold');
    w.save('r.pdf');
    expect(mocks.vector.save).toHaveBeenCalledWith('r.pdf');
    expect(mocks.vector.addImage).not.toHaveBeenCalled();
  });

  it('draws Chinese on a canvas with the device fonts and puts the pages in the PDF as images', async () => {
    const w = await createPdfWriter(['\u4f1a\u5458\u62a5\u544a', 'Mary']);
    w.text('\u4f1a\u5458\u62a5\u544a', 20, 20, { size: 18 });
    expect(fills).toEqual(['\u4f1a\u5458\u62a5\u544a']);
    expect(mocks.vector.text).not.toHaveBeenCalled();
    expect(ctx.font).toContain('PingFang SC');

    w.addPage();
    w.text('\u7b2c\u4e8c\u9875', 20, 20);
    w.save('r.pdf');
    expect(mocks.vector.addImage).toHaveBeenCalledTimes(2);
    expect(mocks.vector.addPage).toHaveBeenCalledTimes(1);
    expect(mocks.vector.save).toHaveBeenCalledWith('r.pdf');
  });

  it('wraps long Chinese text inside runs with no spaces, and can cut it with an ellipsis', async () => {
    const w = await createPdfWriter(['\u957f'.repeat(30)]);
    // 10 canvas px per character, 8 px per mm: 5 mm fits 4 characters.
    const wrapped = w.text('\u957f'.repeat(10), 20, 20, { maxWidth: 5 });
    expect(fills).toEqual(['\u957f\u957f\u957f\u957f', '\u957f\u957f\u957f\u957f', '\u957f\u957f']);
    const oneLine = w.text('\u957f'.repeat(10), 20, 40, { maxWidth: 5, maxLines: 1 });
    expect(fills.slice(3)).toEqual(['\u957f\u957f\u957f…']);
    expect(wrapped).toBeGreaterThan(oneLine);
  });

  it('right-aligns text against its x', async () => {
    const w = await createPdfWriter(['\u9648']);
    w.text('\u9648', 100, 20, { align: 'right' });
    expect(ctx.textAlign).toBe('right');
  });
});
