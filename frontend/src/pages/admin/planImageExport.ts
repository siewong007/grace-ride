// High-resolution canvas rendering for exporting transport assignments as an image.
// Designed for crystal-clear readability (Retina 2x/3x scale) when shared on mobile chats.
import { fmtDateTime, fmtTime } from '../../format';
import { Draft, Plan, PlanDriver, PlanRide } from './planLogic';

const CANVAS_WIDTH = 1200;
const PAD = 36;
const GAP = 20;

const FONT_FAMILY =
  'system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "PingFang SC", "Noto Sans CJK SC", "Microsoft YaHei", sans-serif';

export type TranslationFn = (key: string, opts?: Record<string, unknown>) => string;

export interface PlanImageExportOptions {
  plan: Plan;
  draft: Draft;
  t: TranslationFn;
  formatT: TranslationFn;
  scale?: number;
  isApproved?: boolean;
}

interface MeasuredPassengerItem {
  ride: PlanRide;
  headerText: string;
  addrLines: string[];
  noteLines: string[];
  height: number;
}

interface MeasuredDriverCard {
  driver: PlanDriver;
  rides: PlanRide[];
  assignedSeats: number;
  passengers: MeasuredPassengerItem[];
  x: number;
  y: number;
  width: number;
  height: number;
}

interface MeasuredUnassignedItem {
  ride: PlanRide;
  headerText: string;
  addrLines: string[];
  noteLines: string[];
  height: number;
}

interface MeasuredLayout {
  width: number;
  height: number;
  headerY: number;
  headerHeight: number;
  driverCards: MeasuredDriverCard[];
  standbyDrivers: PlanDriver[];
  standbyY: number;
  standbyHeight: number;
  unassignedItems: MeasuredUnassignedItem[];
  unassignedY: number;
  unassignedHeight: number;
  allAssignedY: number;
  allAssignedHeight: number;
  footerY: number;
  footerHeight: number;
  // Metadata for rendering
  eventTitle: string;
  eventSubtitle: string;
  statusBadge: { text: string; tone: 'success' | 'amber' };
  stats: Array<{ label: string; value: string }>;
  timestamp: string;
  reminderText: string[];
}

export function wrapText(
  ctx: CanvasRenderingContext2D,
  text: string,
  maxWidth: number,
): string[] {
  if (!text) return [];
  const lines: string[] = [];
  const paragraphs = text.split('\n');
  for (const para of paragraphs) {
    const trimmed = para.trim();
    if (!trimmed) continue;
    let currentLine = '';
    for (const char of trimmed) {
      const testLine = currentLine + char;
      if (currentLine && ctx.measureText(testLine).width > maxWidth) {
        lines.push(currentLine);
        currentLine = char === ' ' ? '' : char;
      } else {
        currentLine = testLine;
      }
    }
    if (currentLine) {
      lines.push(currentLine);
    }
  }
  return lines;
}

export function drawRoundedRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number,
) {
  const radius = Math.min(r, w / 2, h / 2);
  ctx.beginPath();
  if (typeof ctx.roundRect === 'function') {
    ctx.roundRect(x, y, w, h, radius);
  } else {
    ctx.moveTo(x + radius, y);
    ctx.arcTo(x + w, y, x + w, y + h, radius);
    ctx.arcTo(x + w, y + h, x, y + h, radius);
    ctx.arcTo(x, y + h, x, y, radius);
    ctx.arcTo(x, y, x + w, y, radius);
    ctx.closePath();
  }
}

function drawBadge(
  ctx: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  opts: {
    bg: string;
    color: string;
    border?: string;
    fontSize?: number;
    padX?: number;
    padY?: number;
    radius?: number;
  },
): number {
  const { bg, color, border, fontSize = 11, padX = 8, padY = 4, radius = 6 } = opts;
  ctx.font = `600 ${fontSize}px ${FONT_FAMILY}`;
  const textWidth = ctx.measureText(text).width;
  const badgeWidth = textWidth + padX * 2;
  const badgeHeight = fontSize + padY * 2 + 2;

  ctx.fillStyle = bg;
  drawRoundedRect(ctx, x, y, badgeWidth, badgeHeight, radius);
  ctx.fill();

  if (border) {
    ctx.strokeStyle = border;
    ctx.lineWidth = 1;
    drawRoundedRect(ctx, x, y, badgeWidth, badgeHeight, radius);
    ctx.stroke();
  }

  ctx.fillStyle = color;
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText(text, x + padX, y + badgeHeight / 2);

  return badgeWidth;
}

export function measurePlanLayout(
  ctx: CanvasRenderingContext2D,
  plan: Plan,
  draft: Draft,
  t: TranslationFn,
  formatT: TranslationFn,
  isApprovedOverride?: boolean,
): MeasuredLayout {
  const contentWidth = CANVAS_WIDTH - PAD * 2;
  const isApproved =
    isApprovedOverride ?? (plan.draft?.status === 'approved' || plan.event.status === 'confirmed');

  // 1. Header measurement
  const eventTitle = plan.event.title || t('title');
  const eventSubtitle = `${fmtDateTime(plan.event.starts_at, formatT)} \u00b7 ${plan.event.location || ''}`;
  const statusBadge = {
    text: isApproved ? t('officialArrangement') : t('draftArrangement'),
    tone: (isApproved ? 'success' : 'amber') as 'success' | 'amber',
  };

  const totalRiders = plan.rides.reduce((acc, r) => acc + (r.seats || 1), 0);
  const unassignedRides = plan.rides.filter((r) => draft[r.id] == null);
  const assignedRides = plan.rides.filter((r) => draft[r.id] != null);
  const totalAssignedSeats = assignedRides.reduce((acc, r) => acc + (r.seats || 1), 0);

  // Group rides by driver
  const driverRidesMap = new Map<number, PlanRide[]>();
  for (const d of plan.drivers) {
    driverRidesMap.set(d.id, []);
  }
  for (const r of plan.rides) {
    const dId = draft[r.id];
    if (dId != null && driverRidesMap.has(dId)) {
      driverRidesMap.get(dId)!.push(r);
    }
  }

  const activeDrivers = plan.drivers.filter(
    (d) => (driverRidesMap.get(d.id)?.length ?? 0) > 0,
  );
  const standbyDrivers = plan.drivers.filter(
    (d) => (driverRidesMap.get(d.id)?.length ?? 0) === 0,
  );
  const totalCap = plan.drivers.reduce((acc, d) => acc + d.cap, 0);

  const stats = [
    { label: t('totalPassengers', { count: totalRiders }), value: `${totalRiders}` },
    { label: t('activeDriversCount', { count: activeDrivers.length }), value: `${activeDrivers.length}` },
    { label: t('seatsCount', { used: totalAssignedSeats, cap: totalCap }), value: `${totalAssignedSeats}/${totalCap}` },
  ];

  const now = new Date();
  const timestamp = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')} ${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;

  const headerY = PAD;
  const headerHeight = 170;

  let currentY = headerY + headerHeight + GAP;

  // 2. Driver cards measurement
  const colCount = activeDrivers.length <= 1 ? 1 : 2;
  const colWidth = colCount === 1 ? contentWidth : (contentWidth - GAP) / 2;
  const colXs = colCount === 1 ? [PAD] : [PAD, PAD + colWidth + GAP];
  const colHeights = colCount === 1 ? [currentY] : [currentY, currentY];

  const cardInnerWidth = colWidth - 32;
  const passengerInnerWidth = cardInnerWidth - 24;

  const measuredCards: MeasuredDriverCard[] = [];

  for (const d of activeDrivers) {
    const rides = driverRidesMap.get(d.id) ?? [];
    const assignedSeats = rides.reduce((acc, r) => acc + (r.seats || 1), 0);

    const passengerItems: MeasuredPassengerItem[] = [];
    let itemsTotalHeight = 0;

    for (const r of rides) {
      const legLabel = r.leg === 'return' ? ` [${t('returnTag')}]` : '';
      const seatsLabel = ` (${t('passengerSeats', { count: r.seats })})`;
      const timeLabel = ` \u00b7 ${fmtTime(r.pickup_at, formatT)}`;
      const headerText = `${r.passenger.name}${seatsLabel}${legLabel}${timeLabel}`;

      ctx.font = `13px ${FONT_FAMILY}`;
      const addrRaw = `${r.area.name} \u00b7 ${r.pickup_address || ''}`;
      const addrLines = wrapText(ctx, addrRaw, passengerInnerWidth);

      const noteRaw = [
        r.assistance ? `${t('assistance', { what: r.assistance })}` : null,
        r.notes ? r.notes : null,
      ]
        .filter(Boolean)
        .join(' \u00b7 ');
      const noteLines = noteRaw ? wrapText(ctx, noteRaw, passengerInnerWidth) : [];

      const itemHeight =
        10 + // top pad
        18 + // header text
        4 + // gap
        Math.max(1, addrLines.length) * 17 + // address lines
        (noteLines.length > 0 ? 4 + noteLines.length * 16 : 0) + // notes
        10; // bottom pad

      passengerItems.push({
        ride: r,
        headerText,
        addrLines: addrLines.length > 0 ? addrLines : [addrRaw],
        noteLines,
        height: itemHeight,
      });

      itemsTotalHeight += itemHeight + 8;
    }

    const cardHeaderHeight = 56;
    const cardPadding = 32;
    const cardHeight = cardHeaderHeight + itemsTotalHeight + cardPadding;

    const chosenCol = colHeights[0] <= colHeights[1] ? 0 : 1;
    const x = colXs[chosenCol];
    const y = colHeights[chosenCol];

    measuredCards.push({
      driver: d,
      rides,
      assignedSeats,
      passengers: passengerItems,
      x,
      y,
      width: colWidth,
      height: cardHeight,
    });

    colHeights[chosenCol] += cardHeight + GAP;
  }

  currentY = Math.max(colHeights[0], colHeights[colCount - 1]);

  // 3. Unassigned passengers section
  let unassignedY = 0;
  let unassignedHeight = 0;
  const measuredUnassigned: MeasuredUnassignedItem[] = [];

  let allAssignedY = 0;
  let allAssignedHeight = 0;

  if (unassignedRides.length > 0) {
    unassignedY = currentY;
    let itemsH = 0;
    const itemW = contentWidth - 32 - 24;

    for (const r of unassignedRides) {
      const legLabel = r.leg === 'return' ? ` [${t('returnTag')}]` : '';
      const seatsLabel = ` (${t('passengerSeats', { count: r.seats })})`;
      const timeLabel = ` \u00b7 ${fmtTime(r.pickup_at, formatT)}`;
      const headerText = `${r.passenger.name}${seatsLabel}${legLabel}${timeLabel}`;

      ctx.font = `13px ${FONT_FAMILY}`;
      const addrRaw = `${r.area.name} \u00b7 ${r.pickup_address || ''}`;
      const addrLines = wrapText(ctx, addrRaw, itemW);

      const noteRaw = r.assistance ? `${t('assistance', { what: r.assistance })}` : '';
      const noteLines = noteRaw ? wrapText(ctx, noteRaw, itemW) : [];

      const itemHeight =
        10 + 18 + 4 + Math.max(1, addrLines.length) * 17 + (noteLines.length > 0 ? 4 + noteLines.length * 16 : 0) + 10;

      measuredUnassigned.push({
        ride: r,
        headerText,
        addrLines: addrLines.length > 0 ? addrLines : [addrRaw],
        noteLines,
        height: itemHeight,
      });

      itemsH += itemHeight + 8;
    }

    unassignedHeight = 48 + itemsH + 16;
    currentY += unassignedHeight + GAP;
  } else {
    allAssignedY = currentY;
    allAssignedHeight = 56;
    currentY += allAssignedHeight + GAP;
  }

  // 4. Standby drivers section
  let standbyY = 0;
  let standbyHeight = 0;
  if (standbyDrivers.length > 0) {
    standbyY = currentY;
    standbyHeight = 64;
    currentY += standbyHeight + GAP;
  }

  // 5. Footer measurement
  ctx.font = `12px ${FONT_FAMILY}`;
  const reminderRaw = t('reminderNote');
  const reminderText = wrapText(ctx, reminderRaw, contentWidth - 260);
  const footerHeight = Math.max(64, reminderText.length * 18 + 24);
  const footerY = currentY;

  const totalHeight = footerY + footerHeight + PAD;

  return {
    width: CANVAS_WIDTH,
    height: totalHeight,
    headerY,
    headerHeight,
    driverCards: measuredCards,
    standbyDrivers,
    standbyY,
    standbyHeight,
    unassignedItems: measuredUnassigned,
    unassignedY,
    unassignedHeight,
    allAssignedY,
    allAssignedHeight,
    footerY,
    footerHeight,
    eventTitle,
    eventSubtitle,
    statusBadge,
    stats,
    timestamp,
    reminderText,
  };
}

export function drawPlanLayout(
  ctx: CanvasRenderingContext2D,
  layout: MeasuredLayout,
  t: TranslationFn,
) {
  const contentWidth = CANVAS_WIDTH - PAD * 2;

  // Background
  ctx.fillStyle = '#f8fafc';
  ctx.fillRect(0, 0, layout.width, layout.height);

  // --- 1. HEADER BANNER ---
  const hx = PAD;
  const hy = layout.headerY;
  const hw = contentWidth;
  const hh = layout.headerHeight;

  drawRoundedRect(ctx, hx, hy, hw, hh, 16);
  ctx.fillStyle = '#0f172a';
  ctx.fill();

  // Header Brand Line
  ctx.fillStyle = '#38bdf8';
  ctx.font = `700 13px ${FONT_FAMILY}`;
  ctx.textAlign = 'left';
  ctx.textBaseline = 'top';
  ctx.fillText(`${t('brandTitle')} \u00b7 ${t('brandSubtitle')}`, hx + 24, hy + 20);

  // Status Badge in Header
  const badgeColor = layout.statusBadge.tone === 'success' ? '#10b981' : '#f59e0b';
  const badgeBg = layout.statusBadge.tone === 'success' ? 'rgba(16, 185, 129, 0.2)' : 'rgba(245, 158, 11, 0.2)';
  const badgeBorder = layout.statusBadge.tone === 'success' ? '#059669' : '#d97706';
  drawBadge(ctx, layout.statusBadge.text, hx + hw - 180, hy + 20, {
    bg: badgeBg,
    color: badgeColor,
    border: badgeBorder,
    fontSize: 12,
    padX: 12,
    padY: 5,
    radius: 12,
  });

  // Event Title
  ctx.fillStyle = '#ffffff';
  ctx.font = `700 24px ${FONT_FAMILY}`;
  ctx.fillText(layout.eventTitle, hx + 24, hy + 46);

  // Event Subtitle
  ctx.fillStyle = '#94a3b8';
  ctx.font = `500 14px ${FONT_FAMILY}`;
  ctx.fillText(layout.eventSubtitle, hx + 24, hy + 82);

  // Stats bar inside Header
  const statBoxY = hy + 112;
  const statBoxH = 38;
  drawRoundedRect(ctx, hx + 24, statBoxY, hw - 48, statBoxH, 8);
  ctx.fillStyle = 'rgba(255, 255, 255, 0.08)';
  ctx.fill();

  ctx.font = `600 13px ${FONT_FAMILY}`;
  ctx.fillStyle = '#e2e8f0';
  ctx.textBaseline = 'middle';
  let curStatX = hx + 40;
  for (let i = 0; i < layout.stats.length; i++) {
    const st = layout.stats[i];
    ctx.fillText(`${st.label}`, curStatX, statBoxY + statBoxH / 2);
    curStatX += ctx.measureText(`${st.label}`).width + 24;
    if (i < layout.stats.length - 1) {
      ctx.fillStyle = '#64748b';
      ctx.fillText('\u00b7', curStatX - 12, statBoxY + statBoxH / 2);
      ctx.fillStyle = '#e2e8f0';
    }
  }

  // --- 2. DRIVER CARDS ---
  for (const card of layout.driverCards) {
    const { x, y, width: cw, height: ch, driver: d, passengers, assignedSeats } = card;

    // Card background & border
    drawRoundedRect(ctx, x, y, cw, ch, 12);
    ctx.fillStyle = '#ffffff';
    ctx.fill();
    ctx.strokeStyle = '#e2e8f0';
    ctx.lineWidth = 1;
    ctx.stroke();

    // Driver Card Header
    const dHeaderH = 50;
    drawRoundedRect(ctx, x, y, cw, dHeaderH, 12);
    ctx.fillStyle = '#f8fafc';
    ctx.fill();

    // Bottom border for header
    ctx.beginPath();
    ctx.moveTo(x, y + dHeaderH);
    ctx.lineTo(x + cw, y + dHeaderH);
    ctx.strokeStyle = '#e2e8f0';
    ctx.stroke();

    // Driver Name
    ctx.fillStyle = '#0f172a';
    ctx.font = `700 16px ${FONT_FAMILY}`;
    ctx.textAlign = 'left';
    ctx.textBaseline = 'middle';
    ctx.fillText(`\ud83d\ude97 ${d.name}`, x + 16, y + dHeaderH / 2);

    // Seats badge on right
    const seatsLabel = `${assignedSeats}/${d.cap} ${t('seatsUsed', { count: assignedSeats })}`;
    const isFull = assignedSeats >= d.cap;
    drawBadge(ctx, seatsLabel, x + cw - 120, y + 14, {
      bg: isFull ? '#fef3c7' : '#ecfdf5',
      color: isFull ? '#92400e' : '#065f46',
      border: isFull ? '#fcd34d' : '#a7f3d0',
      fontSize: 11,
      padX: 8,
      padY: 3,
      radius: 6,
    });

    // Sub-header: Vehicle Info & Tags
    ctx.font = `12px ${FONT_FAMILY}`;
    ctx.fillStyle = '#64748b';
    ctx.textBaseline = 'top';
    const vehicleParts = [
      d.vehicle.model,
      d.vehicle.plate,
      d.carpool ? t('carpool') : t('oneBooking'),
      d.vehicle.wheelchair_ok ? t('wheelchairTag') : null,
      d.vehicle.child_seat ? t('childSeatTag') : null,
      d.vehicle.low_step ? t('lowStepTag') : null,
    ].filter(Boolean);
    const vehicleText = vehicleParts.join(' \u00b7 ');
    ctx.fillText(vehicleText, x + 16, y + dHeaderH + 10);

    // Passenger items
    let curItemY = y + dHeaderH + 32;
    const itemW = cw - 32;

    for (const p of passengers) {
      drawRoundedRect(ctx, x + 16, curItemY, itemW, p.height, 8);
      ctx.fillStyle = '#f8fafc';
      ctx.fill();
      ctx.strokeStyle = '#e2e8f0';
      ctx.lineWidth = 1;
      ctx.stroke();

      // Left blue accent indicator
      drawRoundedRect(ctx, x + 16, curItemY, 4, p.height, 2);
      ctx.fillStyle = '#0284c7';
      ctx.fill();

      // Passenger header line
      ctx.fillStyle = '#0f172a';
      ctx.font = `700 14px ${FONT_FAMILY}`;
      ctx.textAlign = 'left';
      ctx.textBaseline = 'top';
      ctx.fillText(p.headerText, x + 28, curItemY + 10);

      // Address lines
      ctx.fillStyle = '#475569';
      ctx.font = `13px ${FONT_FAMILY}`;
      let textLineY = curItemY + 32;
      for (const line of p.addrLines) {
        ctx.fillText(line, x + 28, textLineY);
        textLineY += 17;
      }

      // Notes lines (if any)
      if (p.noteLines.length > 0) {
        ctx.fillStyle = '#b45309';
        ctx.font = `600 12px ${FONT_FAMILY}`;
        for (const line of p.noteLines) {
          ctx.fillText(`\u26a0\ufe0f ${line}`, x + 28, textLineY);
          textLineY += 16;
        }
      }

      curItemY += p.height + 8;
    }
  }

  // --- 3. ALL ASSIGNED BANNER ---
  if (layout.allAssignedHeight > 0) {
    const ax = PAD;
    const ay = layout.allAssignedY;
    const aw = contentWidth;
    const ah = layout.allAssignedHeight;

    drawRoundedRect(ctx, ax, ay, aw, ah, 10);
    ctx.fillStyle = '#ecfdf5';
    ctx.fill();
    ctx.strokeStyle = '#a7f3d0';
    ctx.lineWidth = 1;
    ctx.stroke();

    ctx.fillStyle = '#065f46';
    ctx.font = `700 15px ${FONT_FAMILY}`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(`\u2705 ${t('allAssigned')}`, ax + aw / 2, ay + ah / 2);
  }

  // --- 4. UNASSIGNED PASSENGERS SECTION ---
  if (layout.unassignedHeight > 0) {
    const ux = PAD;
    const uy = layout.unassignedY;
    const uw = contentWidth;
    const uh = layout.unassignedHeight;

    drawRoundedRect(ctx, ux, uy, uw, uh, 12);
    ctx.fillStyle = '#fffbeb';
    ctx.fill();
    ctx.strokeStyle = '#fcd34d';
    ctx.lineWidth = 1;
    ctx.stroke();

    // Section title
    ctx.fillStyle = '#92400e';
    ctx.font = `700 16px ${FONT_FAMILY}`;
    ctx.textAlign = 'left';
    ctx.textBaseline = 'top';
    ctx.fillText(`\u26a0\ufe0f ${t('unassignedTitle')} (${layout.unassignedItems.length})`, ux + 16, uy + 14);

    let curUY = uy + 44;
    const itemW = uw - 32;

    for (const p of layout.unassignedItems) {
      drawRoundedRect(ctx, ux + 16, curUY, itemW, p.height, 8);
      ctx.fillStyle = '#ffffff';
      ctx.fill();
      ctx.strokeStyle = '#fde68a';
      ctx.lineWidth = 1;
      ctx.stroke();

      ctx.fillStyle = '#92400e';
      ctx.font = `700 14px ${FONT_FAMILY}`;
      ctx.textAlign = 'left';
      ctx.textBaseline = 'top';
      ctx.fillText(p.headerText, ux + 28, curUY + 10);

      ctx.fillStyle = '#475569';
      ctx.font = `13px ${FONT_FAMILY}`;
      let textLineY = curUY + 32;
      for (const line of p.addrLines) {
        ctx.fillText(line, ux + 28, textLineY);
        textLineY += 17;
      }

      curUY += p.height + 8;
    }
  }

  // --- 5. STANDBY DRIVERS ---
  if (layout.standbyHeight > 0) {
    const sx = PAD;
    const sy = layout.standbyY;
    const sw = contentWidth;
    const sh = layout.standbyHeight;

    drawRoundedRect(ctx, sx, sy, sw, sh, 10);
    ctx.fillStyle = '#f1f5f9';
    ctx.fill();
    ctx.strokeStyle = '#cbd5e1';
    ctx.lineWidth = 1;
    ctx.stroke();

    ctx.fillStyle = '#334155';
    ctx.font = `700 14px ${FONT_FAMILY}`;
    ctx.textAlign = 'left';
    ctx.textBaseline = 'top';
    ctx.fillText(`\ud83d\ude97 ${t('standbyDrivers')}:`, sx + 16, sy + 14);

    ctx.font = `500 13px ${FONT_FAMILY}`;
    ctx.fillStyle = '#64748b';
    const names = layout.standbyDrivers
      .map((d) => `${d.name} (${d.cap} ${t('seatsUsed', { count: d.cap })})`)
      .join(' \u00b7 ');
    ctx.fillText(names, sx + 16, sy + 36);
  }

  // --- 6. FOOTER ---
  const fx = PAD;
  const fy = layout.footerY;
  const fw = contentWidth;
  const fh = layout.footerHeight;

  drawRoundedRect(ctx, fx, fy, fw, fh, 12);
  ctx.fillStyle = '#ffffff';
  ctx.fill();
  ctx.strokeStyle = '#e2e8f0';
  ctx.lineWidth = 1;
  ctx.stroke();

  ctx.fillStyle = '#64748b';
  ctx.font = `500 12px ${FONT_FAMILY}`;
  ctx.textAlign = 'left';
  ctx.textBaseline = 'top';
  let lineY = fy + 14;
  for (const line of layout.reminderText) {
    ctx.fillText(line, fx + 16, lineY);
    lineY += 18;
  }

  // Timestamp on the right
  ctx.fillStyle = '#94a3b8';
  ctx.font = `12px ${FONT_FAMILY}`;
  ctx.textAlign = 'right';
  ctx.fillText(t('generatedAt', { time: layout.timestamp }), fx + fw - 16, fy + 14);
}

export function renderPlanToCanvas(options: PlanImageExportOptions): HTMLCanvasElement {
  const { plan, draft, t, formatT, scale = 2, isApproved } = options;
  const canvas = document.createElement('canvas');
  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('Canvas 2D context not available');

  const layout = measurePlanLayout(ctx, plan, draft, t, formatT, isApproved);

  canvas.width = Math.round(layout.width * scale);
  canvas.height = Math.round(layout.height * scale);

  ctx.scale(scale, scale);
  drawPlanLayout(ctx, layout, t);

  return canvas;
}

export function exportPlanAsPng(options: PlanImageExportOptions, filename?: string): string {
  const canvas = renderPlanToCanvas(options);
  const dataUrl = canvas.toDataURL('image/png');
  const sanitized = sanitizeFilename(options.plan.event.title || 'event');
  const actualFilename = filename || `${sanitized}-transport-assignments.png`;
  downloadDataUrl(dataUrl, actualFilename);
  return dataUrl;
}

export function sanitizeFilename(name: string): string {
  return name.trim().replace(/[/\\?%*:|"<>]/g, '_').replace(/\s+/g, '_') || 'event';
}

export function downloadDataUrl(dataUrl: string, filename: string) {
  const a = document.createElement('a');
  a.href = dataUrl;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
}

export async function copyCanvasToClipboard(canvas: HTMLCanvasElement): Promise<boolean> {
  if (typeof navigator === 'undefined' || !navigator.clipboard || typeof ClipboardItem === 'undefined') {
    return false;
  }
  if (!canvas.toBlob) {
    return false;
  }
  return new Promise((resolve) => {
    canvas.toBlob(async (blob) => {
      if (!blob) {
        resolve(false);
        return;
      }
      try {
        await navigator.clipboard.write([
          new ClipboardItem({ 'image/png': blob }),
        ]);
        resolve(true);
      } catch {
        resolve(false);
      }
    }, 'image/png');
  });
}
