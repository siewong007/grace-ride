/** Client-side avatar cropping. Re-encoding through a canvas also drops the photo's
 *  metadata (location, camera), so none of it reaches the server. */

/** Side of the saved square image, in pixels. */
export const AVATAR_OUT = 512;

/** Decode a picked file. Rejects when the browser cannot read it (e.g. HEIC outside Safari). */
export function loadImage(file: File): Promise<{ img: HTMLImageElement; url: string; file: File }> {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file);
    const img = new Image();
    img.onload = () => resolve({ img, url, file });
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

/** Longest side of a saved pickup-area photo, in pixels. */
export const PHOTO_MAX_SIDE = 1600;

/** Width and height that fit within `max` on the longer side, never enlarged. */
export function fitWithin(w: number, h: number, max: number): { w: number; h: number } {
  const scale = Math.min(1, max / Math.max(w, h, 1));
  return { w: Math.max(1, Math.round(w * scale)), h: Math.max(1, Math.round(h * scale)) };
}

/**
 * Shrink a photo for upload: at most `max` pixels on the longer side, saved as JPEG.
 * Drawing it on a canvas applies the camera's rotation and drops its metadata (GPS
 * location included). A file the browser cannot read is returned unchanged; the server
 * still checks its type and size and strips JPEG metadata.
 */
export async function shrinkPhoto(file: File, max = PHOTO_MAX_SIDE, quality = 0.82): Promise<File> {
  let loaded: { img: HTMLImageElement; url: string };
  try {
    loaded = await loadImage(file);
  } catch {
    return file;
  }
  const { img, url } = loaded;
  try {
    const { w, h } = fitWithin(img.naturalWidth, img.naturalHeight, max);
    const canvas = document.createElement('canvas');
    canvas.width = w;
    canvas.height = h;
    const ctx = canvas.getContext('2d');
    if (!ctx) return file;
    ctx.fillStyle = '#fff';
    ctx.fillRect(0, 0, w, h);
    ctx.drawImage(img, 0, 0, w, h);
    const blob = await new Promise<Blob | null>((res) => canvas.toBlob(res, 'image/jpeg', quality));
    if (!blob) return file;
    const base = file.name.replace(/\.[^.]*$/, '') || 'photo';
    return new File([blob], `${base}.jpg`, { type: 'image/jpeg' });
  } finally {
    URL.revokeObjectURL(url);
  }
}
