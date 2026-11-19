/** Client-side avatar cropping. Re-encoding through a canvas also drops the photo's
 *  metadata (location, camera), so none of it reaches the server. */

/** Side of the saved square image, in pixels. */
export const AVATAR_OUT = 512;

/** Decode a picked file. Rejects when the browser cannot read it (e.g. HEIC outside Safari). */
export function loadImage(file: File): Promise<{ img: HTMLImageElement; url: string }> {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file);
    const img = new Image();
    img.onload = () => resolve({ img, url });
    img.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new Error('unreadable'));
    };
    img.src = url;
  });
}

export interface CropView {
  /** Viewport side in CSS pixels. */
  view: number;
  /** Zoom, 1 = the image just covers the viewport. */
  zoom: number;
  /** Image centre offset from the viewport centre, in viewport pixels. */
  x: number;
  y: number;
}

/** Displayed image size for a zoom level: at zoom 1 the shorter side fills the viewport. */
export function coverSize(w: number, h: number, view: number, zoom: number) {
  const base = Math.max(view / w, view / h);
  return { w: w * base * zoom, h: h * base * zoom };
}

/** Keep the image covering the whole viewport. */
export function clampOffset(w: number, h: number, c: CropView): { x: number; y: number } {
  const s = coverSize(w, h, c.view, c.zoom);
  const mx = Math.max(0, (s.w - c.view) / 2);
  const my = Math.max(0, (s.h - c.view) / 2);
  // `+ 0` turns a -0 from the clamp into 0.
  return { x: Math.min(mx, Math.max(-mx, c.x)) + 0, y: Math.min(my, Math.max(-my, c.y)) + 0 };
}

/** Draw the visible square to a JPEG of `AVATAR_OUT` pixels. */
export function cropToFile(img: HTMLImageElement, c: CropView, name = 'avatar.jpg'): Promise<File> {
  const canvas = document.createElement('canvas');
  canvas.width = AVATAR_OUT;
  canvas.height = AVATAR_OUT;
  const ctx = canvas.getContext('2d');
  if (!ctx) return Promise.reject(new Error('canvas'));
  const k = AVATAR_OUT / c.view;
  const s = coverSize(img.naturalWidth, img.naturalHeight, c.view, c.zoom);
  ctx.fillStyle = '#ffffff';
  ctx.fillRect(0, 0, AVATAR_OUT, AVATAR_OUT);
  ctx.imageSmoothingQuality = 'high';
  ctx.drawImage(
    img,
    AVATAR_OUT / 2 + (c.x - s.w / 2) * k,
    AVATAR_OUT / 2 + (c.y - s.h / 2) * k,
    s.w * k,
    s.h * k,
  );
  return new Promise((resolve, reject) =>
    canvas.toBlob(
      (b) => (b ? resolve(new File([b], name, { type: 'image/jpeg' })) : reject(new Error('encode'))),
      'image/jpeg',
      0.9,
    ),
  );
}
