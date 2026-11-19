import { describe, expect, it } from 'vitest';
import { clampOffset, coverSize } from './cropImage';

describe('avatar crop geometry', () => {
  it('covers the viewport with the shorter side at zoom 1', () => {
    expect(coverSize(800, 400, 200, 1)).toEqual({ w: 400, h: 200 });
    expect(coverSize(300, 600, 200, 2)).toEqual({ w: 400, h: 800 });
  });

  it('keeps the image covering the circle when dragged', () => {
    // 400×200 shown in a 200 viewport: 100px of slack each side horizontally, none vertically.
    expect(clampOffset(800, 400, { view: 200, zoom: 1, x: 150, y: 40 })).toEqual({ x: 100, y: 0 });
    expect(clampOffset(800, 400, { view: 200, zoom: 1, x: -30, y: -5 })).toEqual({ x: -30, y: 0 });
  });
});
