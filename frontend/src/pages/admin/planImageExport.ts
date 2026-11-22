// High-resolution canvas rendering for exporting transport assignments as an image.
// Drawn as an ER / database diagram so drivers can see cars, areas and the event at a glance.
// Designed for crystal-clear readability (Retina 2x/3x scale) when shared on mobile chats.
import { fmtDateTime, fmtTime } from '../../format';
import { Draft, Plan, PlanDriver, PlanRide } from './planLogic';

const CANVAS_WIDTH = 1200;
const PAD = 48;
const GAP = 16;
const CENTER_GUTTER = 80;
const SIDE_GUTTER = 32;
const TOP_GUTTER = 70;
const AREA_GAP = 16;
const SLIM_BAR_H = 52;
const TABLE_HEADER_H = 36;
const TABLE_SUB_H = 26;
const COL_HEAD_H = 22;
const RADIUS_TABLE = 8;
const RADIUS_PILL = 6;

const FONT_FAMILY =
  'system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "PingFang SC", "Noto Sans CJK SC", "Microsoft YaHei", sans-serif';

const COLOR = {
  canvas: '#f1f5f9',
  slimBar: '#0f172a',
  brand: '#38bdf8',
  white: '#ffffff',
  ink: '#0f172a',
  body: '#334155',
  muted: '#64748b',
  line: '#94a3b8',
  lineStrong: '#475569',
  driveLine: '#0369a1',
  areaLine: '#15803d',
  divider: '#e2e8f0',
  rowAlt: '#f8fafc',
  note: '#b45309',
  eventHeader: '#1e3a5f',
  carHeader: '#0c4a6e',
  unassignedHeader: '#92400e',
  areaHeader: '#14532d',
  standbyHeader: '#334155',
  seatsOkBg: '#ecfdf5',
  seatsOkFg: '#065f46',
  seatsOkBorder: '#a7f3d0',
  seatsFullBg: '#fef3c7',
  seatsFullFg: '#92400e',
  seatsFullBorder: '#fcd34d',
  labelBg: '#ffffff',
  labelFg: '#334155',
  labelBorder: '#cbd5e1',
};

export type TranslationFn = (key: string, opts?: Record<string, unknown>) => string;

export interface PlanImageExportOptions {
  plan: Plan;
  draft: Draft;
  t: TranslationFn;
  formatT: TranslationFn;
  scale?: number;
  isApproved?: boolean;
}

export interface Point {
  x: number;
  y: number;
}

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export type ConnectorKind = 'drivesTo' | 'picksUpIn';

export interface Connector {
  kind: ConnectorKind;
  label: string;
  points: Point[];
  fromCard: 'one' | 'many';
  toCard: 'one' | 'many';
  dashed?: boolean;
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

export interface MeasuredAreaEntity {
  id: number;
  name: string;
  passengerNames: string[];
  count: number;
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface MeasuredEventEntity {
  x: number;
  y: number;
  width: number;
  height: number;
  titleLines: string[];
  attrs: Array<{ key: string; value: string }>;
}

interface MeasuredUnassignedBox {
  x: number;
  y: number;
  width: number;
  height: number;
  vehicleText?: string;
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
  unassignedBox: MeasuredUnassignedBox | null;
  allAssignedY: number;
  allAssignedHeight: number;
  allAssigned: boolean;
  footerY: number;
  footerHeight: number;
  legendY: number;
  legendHeight: number;
  eventTitle: string;
  eventSubtitle: string;
  statusBadge: { text: string; tone: 'success' | 'amber' };
  stats: Array<{ label: string; value: string }>;
  timestamp: string;
  reminderText: string[];
  eventEntity: MeasuredEventEntity;
  areaEntities: MeasuredAreaEntity[];
  connectors: Connector[];
  standbyNames: string;
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
        const sp = currentLine.lastIndexOf(' ');
        if (sp > 0) {
          lines.push(currentLine.slice(0, sp));
          currentLine = currentLine.slice(sp + 1) + (char === ' ' ? '' : char);
        } else {
          lines.push(currentLine);
          currentLine = char === ' ' ? '' : char;
        }
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

/**
 * Visitors (record only, no ride) join the sheet as passenger rows with negative ids, in
 * the car they were recorded in, or under "no driver yet". They are tagged as visitors
 * and have no pickup time.
 */
export function withVisitorRows(plan: Plan, draft: Draft): { plan: Plan; draft: Draft } {
  const visitors = plan.visitors ?? [];
  if (visitors.length === 0) return { plan, draft };
  const rows: PlanRide[] = visitors.map((v) => ({
    id: -v.id,
    passenger: { id: -v.id, name: v.name },
    seats: v.seats,
    area: { id: v.area?.id ?? 0, name: v.area?.name ?? '' },
    pickup_at: plan.event.starts_at,
    pickup_address: v.pickup_address,
    assistance: '',
    notes: v.notes,
    status: 'visitor',
    driver_id: v.driver_id,
    leg: v.leg,
    last: null,
  }));
  return {
    plan: { ...plan, rides: [...plan.rides, ...rows] },
    draft: { ...draft, ...Object.fromEntries(rows.map((r) => [r.id, r.driver_id])) },
  };
}

const isVisitorRow = (r: PlanRide) => r.id < 0;

function rowHeader(r: PlanRide, t: TranslationFn, formatT: TranslationFn): string {
  const legLabel = r.leg === 'return' ? ` [${t('returnTag')}]` : '';
  const seatsLabel = ` (${t('passengerSeats', { count: r.seats })})`;
  if (isVisitorRow(r)) return `${r.passenger.name} [${t('visitorTag')}]${seatsLabel}${legLabel}`;
  return `${r.passenger.name}${seatsLabel}${legLabel} \u00b7 ${fmtTime(r.pickup_at, formatT)}`;
}

const rowAddress = (r: PlanRide) =>
  [r.area.name, r.pickup_address].filter((x) => x && x.trim()).join(' \u00b7 ') || '\u2014';

function areaKey(r: PlanRide): { id: number; name: string } | null {
  const name = (r.area?.name || '').trim();
  if (!name) return null;
  return { id: r.area?.id ?? 0, name };
}

function measurePassengerBlock(
  ctx: CanvasRenderingContext2D,
  r: PlanRide,
  innerWidth: number,
  t: TranslationFn,
  formatT: TranslationFn,
): MeasuredPassengerItem {
  const headerText = rowHeader(r, t, formatT);
  ctx.font = `13px ${FONT_FAMILY}`;
  const addrRaw = rowAddress(r);
  const addrLines = wrapText(ctx, addrRaw, innerWidth);
  const noteRaw = [
    r.assistance ? `${t('assistance', { what: r.assistance })}` : null,
    r.notes ? r.notes : null,
  ]
    .filter(Boolean)
    .join(' \u00b7 ');
  const noteLines = noteRaw ? wrapText(ctx, noteRaw, innerWidth) : [];
  const itemHeight =
    8 +
    16 +
    3 +
    Math.max(1, addrLines.length) * 16 +
    (noteLines.length > 0 ? 3 + noteLines.length * 15 : 0) +
    8;
  return {
    ride: r,
    headerText,
    addrLines: addrLines.length > 0 ? addrLines : [addrRaw],
    noteLines,
    height: itemHeight,
  };
}

function tableBodyHeight(passengers: MeasuredPassengerItem[], empty: boolean): number {
  if (empty) return COL_HEAD_H + 36;
  const items = passengers.reduce((acc, p) => acc + p.height, 0);
  return COL_HEAD_H + items;
}

function vehicleLine(d: PlanDriver, t: TranslationFn): string {
  return [
    d.vehicle.model,
    d.vehicle.plate,
    d.carpool ? t('carpool') : t('oneBooking'),
    d.vehicle.wheelchair_ok ? t('wheelchairTag') : null,
    d.vehicle.child_seat ? t('childSeatTag') : null,
    d.vehicle.low_step ? t('lowStepTag') : null,
  ]
    .filter(Boolean)
    .join(' \u00b7 ');
}

function collapseColinear(points: Point[]): Point[] {
  if (points.length <= 2) return points.slice();
  const out: Point[] = [points[0]];
  for (let i = 1; i < points.length - 1; i++) {
    const a = out[out.length - 1];
    const b = points[i];
    const c = points[i + 1];
    const colinear =
      (a.x === b.x && b.x === c.x) || (a.y === b.y && b.y === c.y);
    if (!colinear) out.push(b);
  }
  out.push(points[points.length - 1]);
  return out;
}

/** True when an orthogonal path crosses the interior of a box (edge ports are allowed). */
export function pathHitsRectInterior(points: Point[], rect: Rect, inset = 2): boolean {
  if (points.length < 2) return false;
  const x1 = rect.x + inset;
  const y1 = rect.y + inset;
  const x2 = rect.x + rect.width - inset;
  const y2 = rect.y + rect.height - inset;
  if (x2 <= x1 || y2 <= y1) return false;
  for (let i = 0; i < points.length - 1; i++) {
    const a = points[i];
    const b = points[i + 1];
    if (segmentHitsAabb(a.x, a.y, b.x, b.y, x1, y1, x2, y2)) return true;
  }
  return false;
}

function segmentHitsAabb(
  ax: number,
  ay: number,
  bx: number,
  by: number,
  x1: number,
  y1: number,
  x2: number,
  y2: number,
): boolean {
  const minX = Math.min(ax, bx);
  const maxX = Math.max(ax, bx);
  const minY = Math.min(ay, by);
  const maxY = Math.max(ay, by);
  const ix = Math.max(minX, x1);
  const iy = Math.max(minY, y1);
  const jx = Math.min(maxX, x2);
  const jy = Math.min(maxY, y2);
  // A touching point on the edge of the inset box is ignored; a positive overlap is a hit.
  return jx - ix > 0.5 && jy - iy > 0.5;
}


/** Colinear overlap of two orthogonal polylines, ignoring a mere touching of endpoints. */
export function pathsShareASegment(a: Point[], b: Point[], minOverlap = 2): boolean {
  const segs = (pts: Point[]) => {
    const out: Array<[Point, Point]> = [];
    for (let i = 0; i < pts.length - 1; i++) {
      const p = pts[i];
      const q = pts[i + 1];
      if (p.x === q.x && p.y === q.y) continue;
      out.push([p, q]);
    }
    return out;
  };
  const overlap1d = (a1: number, a2: number, b1: number, b2: number) => {
    const lo = Math.max(Math.min(a1, a2), Math.min(b1, b2));
    const hi = Math.min(Math.max(a1, a2), Math.max(b1, b2));
    return hi - lo;
  };
  for (const [p, q] of segs(a)) {
    for (const [r, t] of segs(b)) {
      const aH = p.y === q.y;
      const bH = r.y === t.y;
      if (aH && bH && Math.abs(p.y - r.y) < 0.5) {
        if (overlap1d(p.x, q.x, r.x, t.x) >= minOverlap) return true;
      } else if (!aH && !bH && Math.abs(p.x - r.x) < 0.5 && p.x === q.x && r.x === t.x) {
        if (overlap1d(p.y, q.y, r.y, t.y) >= minOverlap) return true;
      }
    }
  }
  return false;
}

export function boxesOverlap(a: Rect, b: Rect, gap = 0): boolean {
  return (
    a.x < b.x + b.width + gap &&
    a.x + a.width + gap > b.x &&
    a.y < b.y + b.height + gap &&
    a.y + a.height + gap > b.y
  );
}


export function measurePlanLayout(
  ctx: CanvasRenderingContext2D,
  basePlan: Plan,
  baseDraft: Draft,
  t: TranslationFn,
  formatT: TranslationFn,
  isApprovedOverride?: boolean,
): MeasuredLayout {
  const { plan, draft } = withVisitorRows(basePlan, baseDraft);
  const contentWidth = CANVAS_WIDTH - PAD * 2;
  const isApproved =
    isApprovedOverride ?? (plan.draft?.status === 'approved' || plan.event.status === 'confirmed');

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

  const activeDrivers = plan.drivers.filter((d) => (driverRidesMap.get(d.id)?.length ?? 0) > 0);
  const standbyDrivers = plan.drivers.filter((d) => (driverRidesMap.get(d.id)?.length ?? 0) === 0);
  const totalCap = plan.drivers.reduce((acc, d) => acc + d.cap, 0);

  const stats = [
    { label: t('totalPassengers', { count: totalRiders }), value: `${totalRiders}` },
    { label: t('activeDriversCount', { count: activeDrivers.length }), value: `${activeDrivers.length}` },
    { label: t('seatsCount', { used: totalAssignedSeats, cap: totalCap }), value: `${totalAssignedSeats}/${totalCap}` },
  ];

  const now = new Date();
  const timestamp = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')} ${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;

  const headerY = PAD;
  const headerHeight = SLIM_BAR_H;

  // --- Event entity ---
  const eventWidth = Math.min(560, contentWidth);
  const eventInner = eventWidth - 28;
  ctx.font = `700 20px ${FONT_FAMILY}`;
  const titleLines = wrapText(ctx, eventTitle, eventInner);
  const eventAttrs: Array<{ key: string; value: string }> = [
    { key: t('colStartsAt'), value: fmtDateTime(plan.event.starts_at, formatT) },
    { key: t('colLocation'), value: plan.event.location || '\u2014' },
    { key: t('colStatus'), value: statusBadge.text },
    {
      key: t('colSeats'),
      value: `${t('totalPassengers', { count: totalRiders })} \u00b7 ${t('seatsCount', { used: totalAssignedSeats, cap: totalCap })}`,
    },
  ];
  if (unassignedRides.length === 0 && plan.rides.length > 0) {
    eventAttrs.push({ key: t('colAssignment'), value: t('allAssigned') });
  }
  const eventHeight =
    TABLE_HEADER_H +
    10 +
    Math.max(1, titleLines.length) * 24 +
    8 +
    eventAttrs.length * 20 +
    12;
  const eventEntity: MeasuredEventEntity = {
    x: (CANVAS_WIDTH - eventWidth) / 2,
    y: headerY + headerHeight + GAP,
    width: eventWidth,
    height: eventHeight,
    titleLines: titleLines.length > 0 ? titleLines : [eventTitle],
    attrs: eventAttrs,
  };

  // --- Car / unassigned tables ---
  const tableCount = activeDrivers.length + (unassignedRides.length > 0 ? 1 : 0);
  const colCount = tableCount <= 1 ? 1 : 2;
  const sideGutter = colCount === 1 ? 0 : SIDE_GUTTER;
  const gridWidth =
    colCount === 1 ? Math.min(640, contentWidth) : contentWidth - sideGutter * 2;
  const colWidth = colCount === 1 ? gridWidth : (gridWidth - CENTER_GUTTER) / 2;
  const gridX = (CANVAS_WIDTH - gridWidth) / 2;
  const colXs = colCount === 1 ? [gridX] : [gridX, gridX + colWidth + CENTER_GUTTER];
  const gridTop = eventEntity.y + eventEntity.height + TOP_GUTTER;
  const colHeights = colCount === 1 ? [gridTop] : [gridTop, gridTop];
  const passengerInnerWidth = colWidth - 28;

  const measuredCards: MeasuredDriverCard[] = [];
  for (const d of activeDrivers) {
    const rides = driverRidesMap.get(d.id) ?? [];
    const assignedSeats = rides.reduce((acc, r) => acc + (r.seats || 1), 0);
    const passengers = rides.map((r) => measurePassengerBlock(ctx, r, passengerInnerWidth, t, formatT));
    const height = TABLE_HEADER_H + TABLE_SUB_H + tableBodyHeight(passengers, passengers.length === 0);
    const chosenCol = colCount === 1 || colHeights[0] <= colHeights[1] ? 0 : 1;
    const x = colXs[chosenCol];
    const y = colHeights[chosenCol];
    measuredCards.push({
      driver: d,
      rides,
      assignedSeats,
      passengers,
      x,
      y,
      width: colWidth,
      height,
    });
    colHeights[chosenCol] += height + GAP;
  }

  const measuredUnassigned: MeasuredUnassignedItem[] = [];
  let unassignedBox: MeasuredUnassignedBox | null = null;
  if (unassignedRides.length > 0) {
    for (const r of unassignedRides) {
      measuredUnassigned.push(measurePassengerBlock(ctx, r, passengerInnerWidth, t, formatT));
    }
    const height = TABLE_HEADER_H + tableBodyHeight(measuredUnassigned, false);
    const chosenCol = colCount === 1 || colHeights[0] <= colHeights[1] ? 0 : 1;
    unassignedBox = {
      x: colXs[chosenCol],
      y: colHeights[chosenCol],
      width: colWidth,
      height,
    };
    colHeights[chosenCol] += height + GAP;
  }

  const gridBottom = Math.max(colHeights[0], colHeights[colCount - 1]) - GAP;

  // --- Area entities ---
  const areaMap = new Map<string, { id: number; name: string; names: string[] }>();
  for (const r of plan.rides) {
    const a = areaKey(r);
    if (!a) continue;
    const key = `${a.id}::${a.name}`;
    const rec = areaMap.get(key) ?? { id: a.id, name: a.name, names: [] };
    rec.names.push(r.passenger.name);
    areaMap.set(key, rec);
  }
  const areas = [...areaMap.values()];
  const areaCount = Math.max(1, areas.length);
  const areaAvail = contentWidth;
  const areaWidth = Math.max(
    160,
    Math.min(260, (areaAvail - AREA_GAP * (Math.min(areaCount, 5) - 1)) / Math.min(areaCount, 5)),
  );
  const areasPerRow = Math.max(1, Math.floor((areaAvail + AREA_GAP) / (areaWidth + AREA_GAP)));

  ctx.font = `12px ${FONT_FAMILY}`;
  const measuredAreasUnplaced = areas.map((a) => {
    const nameLines = wrapText(ctx, a.name, areaWidth - 24);
    const shown = a.names.slice(0, 3);
    const extra = a.names.length - shown.length;
    const listRaw = extra > 0 ? `${shown.join(', ')} +${extra}` : shown.join(', ');
    const listLines = listRaw ? wrapText(ctx, listRaw, areaWidth - 24) : [];
    const height =
      TABLE_HEADER_H +
      8 +
      16 +
      (listLines.length > 0 ? 4 + listLines.length * 15 : 0) +
      10;
    return { ...a, nameLines, listLines, height };
  });

  const areaRowCount = Math.max(1, Math.ceil(Math.max(areas.length, 1) / areasPerRow));
  const areaRowHeights: number[] = Array.from({ length: areaRowCount }, () => 0);
  measuredAreasUnplaced.forEach((a, i) => {
    const row = Math.floor(i / areasPerRow);
    areaRowHeights[row] = Math.max(areaRowHeights[row], a.height);
  });

  const areaSources = [
    ...measuredCards.map((c) => ({
      x: c.x,
      y: c.y,
      width: c.width,
      height: c.height,
      rides: c.rides,
      col: c.x === colXs[0] ? 0 : 1,
    })),
    ...(unassignedBox
      ? [
          {
            x: unassignedBox.x,
            y: unassignedBox.y,
            width: unassignedBox.width,
            height: unassignedBox.height,
            rides: unassignedRides,
            col: unassignedBox.x === colXs[0] ? 0 : 1,
          },
        ]
      : []),
  ];

  const areaLaneCount = Math.max(1, areaSources.filter((s) => s.rides.length > 0).length);
  const bottomGutter = areas.length === 0 ? GAP : Math.max(64, 36 + areaLaneCount * 14);
  const areasTop = gridBottom + bottomGutter;

  const areaEntities: MeasuredAreaEntity[] = [];
  if (areas.length > 0) {
    let rowY = areasTop;
    for (let row = 0; row < areaRowCount; row++) {
      const rowItems = measuredAreasUnplaced.slice(row * areasPerRow, (row + 1) * areasPerRow);
      const rowWidth = rowItems.length * areaWidth + (rowItems.length - 1) * AREA_GAP;
      let x = (CANVAS_WIDTH - rowWidth) / 2;
      const rowH = areaRowHeights[row];
      for (const a of rowItems) {
        areaEntities.push({
          id: a.id,
          name: a.name,
          passengerNames: a.names,
          count: a.names.length,
          x,
          y: rowY,
          width: areaWidth,
          height: rowH,
        });
        x += areaWidth + AREA_GAP;
      }
      rowY += rowH + GAP;
    }
  }

  const areasBottom = areaEntities.length
    ? areaEntities.reduce((m, a) => Math.max(m, a.y + a.height), areasTop)
    : gridBottom;

  // --- Standby ---
  let standbyY = 0;
  let standbyHeight = 0;
  const standbyNames = standbyDrivers
    .map((d) => `${d.name} (${t('seatsCount', { used: 0, cap: d.cap })})`)
    .join(' \u00b7 ');
  if (standbyDrivers.length > 0) {
    standbyY = areasBottom + GAP;
    ctx.font = `13px ${FONT_FAMILY}`;
    const nameLines = wrapText(ctx, standbyNames, contentWidth - 28);
    standbyHeight = TABLE_HEADER_H + 12 + Math.max(1, nameLines.length) * 18 + 12;
  }

  // --- Footer ---
  ctx.font = `12px ${FONT_FAMILY}`;
  const reminderRaw = t('reminderNote');
  const reminderText = wrapText(ctx, reminderRaw, contentWidth - 260);
  const footerHeight = Math.max(56, reminderText.length * 18 + 24);
  const hasLegend = activeDrivers.length > 0 || unassignedRides.length > 0;
  const legendHeight = hasLegend ? 28 : 0;
  const legendY = (standbyHeight > 0 ? standbyY + standbyHeight : areasBottom) + GAP;
  const footerY = legendY + (legendHeight > 0 ? legendHeight + 8 : 0);

  // --- Connectors ---
  const connectors: Connector[] = [];
  const drivesLabel = t('relDrivesTo');
  const picksLabel = t('relPicksUpIn');
  // Drives-to uses the top gutter and outer side rails only.
  // Picks-up-in uses the bottom gutter and the center gutter only.
  // The two relationship types never travel along the same segment.
  const centerRailX = CANVAS_WIDTH / 2;
  const leftDriveRailX = gridX - 16;
  const rightDriveRailX = gridX + gridWidth + 16;
  const eventBusY = eventEntity.y + eventEntity.height + 28;
  const eventBottom: Point = {
    x: eventEntity.x + eventEntity.width / 2,
    y: eventEntity.y + eventEntity.height,
  };

  const lastYByCol: number[] = [];
  for (let c = 0; c < colCount; c++) {
    const inCol = [
      ...measuredCards.filter((card) => card.x === colXs[c]),
      ...(unassignedBox && unassignedBox.x === colXs[c] ? [unassignedBox] : []),
    ];
    lastYByCol[c] = inCol.reduce((m, b) => Math.max(m, b.y), 0);
  }

  const firstRowY = measuredCards.length ? Math.min(...measuredCards.map((c) => c.y)) : 0;
  for (const card of measuredCards) {
    const col = card.x === colXs[0] ? 0 : 1;
    const carMidX = card.x + card.width / 2;
    const straight =
      colCount === 1 &&
      measuredCards.length === 1 &&
      Math.abs(carMidX - eventBottom.x) < 4;
    const isFirstRow = Math.abs(card.y - firstRowY) < 1;
    let path: Point[];
    if (straight) {
      path = [eventBottom, { x: eventBottom.x, y: card.y }];
    } else if (isFirstRow) {
      // Fan out through the top gutter, down onto the car's top edge.
      path = collapseColinear([
        eventBottom,
        { x: eventBottom.x, y: eventBusY },
        { x: carMidX, y: eventBusY },
        { x: carMidX, y: card.y },
      ]);
    } else {
      // Lower cars climb an outer side rail so they never share the center trunk.
      const railX = colCount === 1 || col === 0 ? leftDriveRailX : rightDriveRailX;
      const outerX = colCount === 1 || col === 0 ? card.x : card.x + card.width;
      const portY = card.y + TABLE_HEADER_H / 2;
      path = collapseColinear([
        eventBottom,
        { x: eventBottom.x, y: eventBusY },
        { x: railX, y: eventBusY },
        { x: railX, y: portY },
        { x: outerX, y: portY },
      ]);
    }
    connectors.push({
      kind: 'drivesTo',
      label: drivesLabel,
      points: path,
      fromCard: 'one',
      toCard: 'many',
    });
  }

  const areaByKey = new Map<string, MeasuredAreaEntity>();
  for (const a of areaEntities) {
    areaByKey.set(`${a.id}::${a.name}`, a);
  }

  const areaBusTop = gridBottom + 20;
  let laneIndex = 0;
  for (const src of areaSources) {
    const targets = new Map<string, MeasuredAreaEntity>();
    for (const r of src.rides) {
      const a = areaKey(r);
      if (!a) continue;
      const key = `${a.id}::${a.name}`;
      const ent = areaByKey.get(key);
      if (ent) targets.set(key, ent);
    }
    if (targets.size === 0) continue;
    const laneY = areaBusTop + 10 + laneIndex * 14;
    laneIndex += 1;
    const isBottom = Math.abs(src.y - lastYByCol[src.col]) < 1;
    // 1-col: pick rail sits to the right of the stack (drives-to uses the left).
    // 2-col: pick rail is the center gutter, never the outer drive rails.
    const pickRailX = colCount === 1 ? gridX + gridWidth + 16 : centerRailX;
    const areaPortY = src.y + src.height - 16 + (colCount === 1 ? 0 : src.col === 0 ? -6 : 6);
    const innerX = colCount === 1 || src.col === 0 ? src.x + src.width : src.x;
    const start: Point = isBottom
      ? { x: src.x + src.width / 2, y: src.y + src.height }
      : { x: innerX, y: areaPortY };
    const viaRail: Point[] = isBottom
      ? [start, { x: start.x, y: laneY }]
      : [start, { x: pickRailX, y: areaPortY }, { x: pickRailX, y: laneY }];

    const dashed = unassignedBox != null && src.x === unassignedBox.x && src.y === unassignedBox.y;
    for (const ent of targets.values()) {
      const dest: Point = { x: ent.x + ent.width / 2, y: ent.y };
      const points = collapseColinear([
        ...viaRail,
        { x: dest.x, y: laneY },
        dest,
      ]);
      connectors.push({
        kind: 'picksUpIn',
        label: picksLabel,
        points,
        fromCard: 'many',
        toCard: 'many',
        dashed,
      });
    }
  }

  const unassignedY = unassignedBox?.y ?? 0;
  const unassignedHeight = unassignedBox?.height ?? 0;

  return {
    width: CANVAS_WIDTH,
    height: footerY + footerHeight + PAD,
    headerY,
    headerHeight,
    driverCards: measuredCards,
    standbyDrivers,
    standbyY,
    standbyHeight,
    unassignedItems: measuredUnassigned,
    unassignedY,
    unassignedHeight,
    unassignedBox,
    allAssignedY: 0,
    allAssignedHeight: 0,
    allAssigned: unassignedRides.length === 0,
    footerY,
    footerHeight,
    legendY,
    legendHeight,
    eventTitle,
    eventSubtitle,
    statusBadge,
    stats,
    timestamp,
    reminderText,
    eventEntity,
    areaEntities,
    connectors,
    standbyNames,
  };
}

function entityKindLabel(kind: 'event' | 'car' | 'area' | 'unassigned' | 'standby', t: TranslationFn): string {
  switch (kind) {
    case 'event':
      return t('entityEvent');
    case 'car':
      return t('entityCar');
    case 'area':
      return t('entityArea');
    case 'unassigned':
      return t('entityUnassigned');
    case 'standby':
      return t('standbyDrivers');
  }
}

function headerFill(kind: 'event' | 'car' | 'area' | 'unassigned' | 'standby'): string {
  switch (kind) {
    case 'event':
      return COLOR.eventHeader;
    case 'car':
      return COLOR.carHeader;
    case 'area':
      return COLOR.areaHeader;
    case 'unassigned':
      return COLOR.unassignedHeader;
    case 'standby':
      return COLOR.standbyHeader;
  }
}

function drawTableShell(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  kind: 'event' | 'car' | 'area' | 'unassigned' | 'standby',
  t: TranslationFn,
  title: string,
  opts?: { badge?: string; badgeTone?: 'success' | 'amber' | 'ok' | 'full' },
) {
  ctx.save();
  drawRoundedRect(ctx, x, y, w, h, RADIUS_TABLE);
  ctx.fillStyle = COLOR.white;
  ctx.fill();
  ctx.strokeStyle = COLOR.divider;
  ctx.lineWidth = 1.25;
  ctx.stroke();

  // Header bar (square off the bottom corners by painting over them after a rounded clip).
  ctx.beginPath();
  const r = RADIUS_TABLE;
  ctx.moveTo(x, y + TABLE_HEADER_H);
  ctx.lineTo(x, y + r);
  ctx.arcTo(x, y, x + r, y, r);
  ctx.lineTo(x + w - r, y);
  ctx.arcTo(x + w, y, x + w, y + r, r);
  ctx.lineTo(x + w, y + TABLE_HEADER_H);
  ctx.closePath();
  ctx.fillStyle = headerFill(kind);
  ctx.fill();

  const kindText = entityKindLabel(kind, t).toUpperCase();
  ctx.font = `700 10px ${FONT_FAMILY}`;
  ctx.fillStyle = 'rgba(255,255,255,0.7)';
  ctx.textAlign = 'left';
  ctx.textBaseline = 'middle';
  ctx.fillText(kindText, x + 12, y + TABLE_HEADER_H / 2);

  const kindWidth = ctx.measureText(kindText).width;
  const titleX = x + 12 + kindWidth + (title ? 10 : 0);
  let titleMax = w - 24 - kindWidth - (title ? 10 : 0);
  if (opts?.badge) {
    ctx.font = `600 11px ${FONT_FAMILY}`;
    const badgeW = ctx.measureText(opts.badge).width + 16;
    const badgeX = x + w - 10 - badgeW;
    const tone = opts.badgeTone ?? 'ok';
    const bg =
      tone === 'amber' || tone === 'full' ? COLOR.seatsFullBg : tone === 'success' || tone === 'ok' ? COLOR.seatsOkBg : COLOR.white;
    const fg =
      tone === 'amber' || tone === 'full' ? COLOR.seatsFullFg : tone === 'success' || tone === 'ok' ? COLOR.seatsOkFg : COLOR.ink;
    const border =
      tone === 'amber' || tone === 'full' ? COLOR.seatsFullBorder : COLOR.seatsOkBorder;
    drawBadge(ctx, opts.badge, badgeX, y + (TABLE_HEADER_H - 20) / 2, {
      bg,
      color: fg,
      border,
      fontSize: 11,
      padX: 8,
      padY: 3,
      radius: RADIUS_PILL,
    });
    titleMax = badgeX - titleX - 8;
  }

  ctx.font = `700 14px ${FONT_FAMILY}`;
  ctx.fillStyle = COLOR.white;
  ctx.textAlign = 'left';
  ctx.textBaseline = 'middle';
  if (!title) {
    ctx.restore();
    return;
  }
  let drawnTitle = title;
  if (ctx.measureText(drawnTitle).width > titleMax) {
    while (drawnTitle.length > 1 && ctx.measureText(`${drawnTitle}\u2026`).width > titleMax) {
      drawnTitle = drawnTitle.slice(0, -1);
    }
    drawnTitle = `${drawnTitle}\u2026`;
  }
  ctx.fillText(drawnTitle, titleX, y + TABLE_HEADER_H / 2);
  ctx.restore();
}

function drawPassengerRows(
  ctx: CanvasRenderingContext2D,
  x: number,
  startY: number,
  w: number,
  items: MeasuredPassengerItem[],
  colLabel: string,
  unassigned: boolean,
) {
  const innerX = x + 1;
  const innerW = w - 2;
  // Column header, like an attribute name in a table.
  ctx.fillStyle = COLOR.rowAlt;
  ctx.fillRect(innerX, startY, innerW, COL_HEAD_H);
  ctx.fillStyle = COLOR.muted;
  ctx.font = `700 10px ${FONT_FAMILY}`;
  ctx.textAlign = 'left';
  ctx.textBaseline = 'middle';
  ctx.fillText(colLabel, x + 12, startY + COL_HEAD_H / 2);
  ctx.beginPath();
  ctx.moveTo(x, startY + COL_HEAD_H);
  ctx.lineTo(x + w, startY + COL_HEAD_H);
  ctx.strokeStyle = COLOR.divider;
  ctx.lineWidth = 1;
  ctx.stroke();

  let y = startY + COL_HEAD_H;
  items.forEach((p, i) => {
    if (i % 2 === 1) {
      ctx.fillStyle = COLOR.rowAlt;
      ctx.fillRect(innerX, y, innerW, p.height);
    }
    ctx.fillStyle = unassigned ? COLOR.seatsFullFg : COLOR.ink;
    ctx.font = `700 13px ${FONT_FAMILY}`;
    ctx.textAlign = 'left';
    ctx.textBaseline = 'top';
    ctx.fillText(p.headerText, x + 12, y + 8);

    ctx.fillStyle = COLOR.body;
    ctx.font = `12px ${FONT_FAMILY}`;
    let ty = y + 8 + 16 + 3;
    for (const line of p.addrLines) {
      ctx.fillText(line, x + 12, ty);
      ty += 16;
    }
    if (p.noteLines.length > 0) {
      ctx.fillStyle = COLOR.note;
      ctx.font = `600 11px ${FONT_FAMILY}`;
      for (const line of p.noteLines) {
        ctx.fillText(`\u26a0 ${line}`, x + 12, ty);
        ty += 15;
      }
    }
    y += p.height;
    ctx.beginPath();
    ctx.moveTo(x + 8, y);
    ctx.lineTo(x + w - 8, y);
    ctx.strokeStyle = COLOR.divider;
    ctx.lineWidth = 1;
    ctx.stroke();
  });
}

function drawCrowFoot(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  toward: 'up' | 'down' | 'left' | 'right',
  kind: 'one' | 'many',
  color: string,
) {
  const arm = 9;
  ctx.save();
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.5;
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  ctx.beginPath();
  if (kind === 'one') {
    if (toward === 'up' || toward === 'down') {
      ctx.moveTo(x - 6, y);
      ctx.lineTo(x + 6, y);
    } else {
      ctx.moveTo(x, y - 6);
      ctx.lineTo(x, y + 6);
    }
  } else if (toward === 'down') {
    ctx.moveTo(x - arm, y);
    ctx.lineTo(x, y + arm);
    ctx.lineTo(x + arm, y);
    ctx.moveTo(x, y);
    ctx.lineTo(x, y + arm);
  } else if (toward === 'up') {
    ctx.moveTo(x - arm, y);
    ctx.lineTo(x, y - arm);
    ctx.lineTo(x + arm, y);
    ctx.moveTo(x, y);
    ctx.lineTo(x, y - arm);
  } else if (toward === 'right') {
    ctx.moveTo(x, y - arm);
    ctx.lineTo(x + arm, y);
    ctx.lineTo(x, y + arm);
    ctx.moveTo(x, y);
    ctx.lineTo(x + arm, y);
  } else {
    ctx.moveTo(x, y - arm);
    ctx.lineTo(x - arm, y);
    ctx.lineTo(x, y + arm);
    ctx.moveTo(x, y);
    ctx.lineTo(x - arm, y);
  }
  ctx.stroke();
  ctx.restore();
}

function endDirection(a: Point, b: Point): 'up' | 'down' | 'left' | 'right' {
  if (a.x === b.x) return b.y > a.y ? 'down' : 'up';
  return b.x > a.x ? 'right' : 'left';
}

function connectorColor(c: Connector): string {
  return c.kind === 'drivesTo' ? COLOR.driveLine : COLOR.areaLine;
}

function drawConnector(ctx: CanvasRenderingContext2D, c: Connector) {
  if (c.points.length < 2) return;
  const color = connectorColor(c);
  ctx.save();
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.75;
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  if (c.dashed && typeof ctx.setLineDash === 'function') {
    ctx.setLineDash([6, 4]);
  }
  ctx.beginPath();
  ctx.moveTo(c.points[0].x, c.points[0].y);
  for (let i = 1; i < c.points.length; i++) {
    ctx.lineTo(c.points[i].x, c.points[i].y);
  }
  ctx.stroke();
  if (c.dashed && typeof ctx.setLineDash === 'function') {
    ctx.setLineDash([]);
  }
  ctx.restore();

  const a0 = c.points[0];
  const a1 = c.points[1];
  const b0 = c.points[c.points.length - 2];
  const b1 = c.points[c.points.length - 1];
  // "Toward" is the direction the path is travelling as it arrives at the entity.
  drawCrowFoot(ctx, a0.x, a0.y, endDirection(a1, a0), c.fromCard, color);
  drawCrowFoot(ctx, b1.x, b1.y, endDirection(b0, b1), c.toCard, color);
}

function drawLabelPill(ctx: CanvasRenderingContext2D, text: string, x: number, y: number, color = COLOR.labelFg) {
  ctx.font = `600 11px ${FONT_FAMILY}`;
  const w = ctx.measureText(text).width + 14;
  const h = 18;
  drawRoundedRect(ctx, x - w / 2, y - h / 2, w, h, 4);
  ctx.fillStyle = COLOR.labelBg;
  ctx.fill();
  ctx.strokeStyle = color;
  ctx.lineWidth = 1;
  drawRoundedRect(ctx, x - w / 2, y - h / 2, w, h, 4);
  ctx.stroke();
  ctx.fillStyle = color;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(text, x, y);
}

function drawLegend(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  t: TranslationFn,
  showDrives: boolean,
  showPicks: boolean,
  showDashed: boolean,
) {
  const items: Array<{ label: string; color: string; dashed: boolean }> = [];
  if (showDrives) items.push({ label: t('relDrivesTo'), color: COLOR.driveLine, dashed: false });
  if (showPicks) items.push({ label: t('relPicksUpIn'), color: COLOR.areaLine, dashed: false });
  if (showDashed) items.push({ label: t('legendUnassigned'), color: COLOR.areaLine, dashed: true });
  if (items.length === 0) return;

  ctx.font = `600 11px ${FONT_FAMILY}`;
  const swatch = 22;
  const gap = 22;
  const parts = items.map((it) => swatch + 6 + ctx.measureText(it.label).width);
  const total = parts.reduce((a, b) => a + b, 0) + gap * (items.length - 1);
  let cx = x + (w - total) / 2;
  const mid = y + h / 2;
  for (let i = 0; i < items.length; i++) {
    const it = items[i];
    ctx.save();
    ctx.strokeStyle = it.color;
    ctx.lineWidth = 1.75;
    ctx.lineCap = 'round';
    if (it.dashed && typeof ctx.setLineDash === 'function') ctx.setLineDash([5, 3]);
    ctx.beginPath();
    ctx.moveTo(cx, mid);
    ctx.lineTo(cx + swatch, mid);
    ctx.stroke();
    ctx.restore();
    ctx.fillStyle = it.color;
    ctx.textAlign = 'left';
    ctx.textBaseline = 'middle';
    ctx.font = `600 11px ${FONT_FAMILY}`;
    ctx.fillText(it.label, cx + swatch + 6, mid);
    cx += parts[i] + gap;
  }
}

export function drawPlanLayout(
  ctx: CanvasRenderingContext2D,
  layout: MeasuredLayout,
  t: TranslationFn,
) {
  const contentWidth = CANVAS_WIDTH - PAD * 2;

  ctx.fillStyle = COLOR.canvas;
  ctx.fillRect(0, 0, layout.width, layout.height);

  // --- Slim brand bar ---
  const hx = PAD;
  const hy = layout.headerY;
  const hw = contentWidth;
  const hh = layout.headerHeight;
  drawRoundedRect(ctx, hx, hy, hw, hh, 10);
  ctx.fillStyle = COLOR.slimBar;
  ctx.fill();

  ctx.fillStyle = COLOR.brand;
  ctx.font = `700 13px ${FONT_FAMILY}`;
  ctx.textAlign = 'left';
  ctx.textBaseline = 'middle';
  ctx.fillText(`${t('brandTitle')} \u00b7 ${t('brandSubtitle')}`, hx + 18, hy + hh / 2);

  const badgeColor = layout.statusBadge.tone === 'success' ? '#10b981' : '#f59e0b';
  const badgeBg = layout.statusBadge.tone === 'success' ? 'rgba(16, 185, 129, 0.2)' : 'rgba(245, 158, 11, 0.2)';
  const badgeBorder = layout.statusBadge.tone === 'success' ? '#059669' : '#d97706';
  ctx.font = `600 12px ${FONT_FAMILY}`;
  const badgeW = ctx.measureText(layout.statusBadge.text).width + 24;
  drawBadge(ctx, layout.statusBadge.text, hx + hw - 12 - badgeW, hy + (hh - 24) / 2, {
    bg: badgeBg,
    color: badgeColor,
    border: badgeBorder,
    fontSize: 12,
    padX: 12,
    padY: 5,
    radius: 12,
  });

  // --- Connectors behind the tables so crow's feet sit on the edges once redrawn after ---
  for (const c of layout.connectors) {
    drawConnector(ctx, c);
  }

  // Relationship labels (once per kind), with a pill so lines never hide the text.
  const drives = layout.connectors.find((c) => c.kind === 'drivesTo');
  if (drives) {
    const ev = layout.eventEntity;
    drawLabelPill(ctx, drives.label, ev.x + ev.width / 2, ev.y + ev.height + 28, COLOR.driveLine);
  }
  const picks = layout.connectors.find((c) => c.kind === 'picksUpIn');
  if (picks && layout.areaEntities.length > 0 && layout.driverCards.length + (layout.unassignedBox ? 1 : 0) > 0) {
    const minAreaY = Math.min(...layout.areaEntities.map((a) => a.y));
    const maxTableY = Math.max(
      ...layout.driverCards.map((c) => c.y + c.height),
      layout.unassignedBox ? layout.unassignedBox.y + layout.unassignedBox.height : 0,
    );
    drawLabelPill(ctx, picks.label, CANVAS_WIDTH / 2, (maxTableY + minAreaY) / 2, COLOR.areaLine);
  }

  // --- Event entity ---
  const ev = layout.eventEntity;
  drawTableShell(ctx, ev.x, ev.y, ev.width, ev.height, 'event', t, '', {
    badge: layout.statusBadge.tone === 'success' ? t('statusApproved') : t('statusDraft'),
    badgeTone: layout.statusBadge.tone === 'success' ? 'success' : 'amber',
  });
  // If the title wrapped, print the remaining lines in the body as the entity name.
  let bodyY = ev.y + TABLE_HEADER_H + 10;
  ctx.textAlign = 'left';
  ctx.textBaseline = 'top';
  const titleLines = ev.titleLines;
  ctx.font = `700 18px ${FONT_FAMILY}`;
  ctx.fillStyle = COLOR.ink;
  for (const line of titleLines) {
    ctx.fillText(line, ev.x + 14, bodyY);
    bodyY += 24;
  }
  bodyY += 4;
  ctx.beginPath();
  ctx.moveTo(ev.x + 10, bodyY);
  ctx.lineTo(ev.x + ev.width - 10, bodyY);
  ctx.strokeStyle = COLOR.divider;
  ctx.lineWidth = 1;
  ctx.stroke();
  bodyY += 8;
  const keyW = 110;
  for (const attr of ev.attrs) {
    ctx.font = `600 11px ${FONT_FAMILY}`;
    ctx.fillStyle = COLOR.muted;
    ctx.fillText(attr.key, ev.x + 14, bodyY);
    ctx.font = `500 13px ${FONT_FAMILY}`;
    ctx.fillStyle = COLOR.ink;
    const valX = ev.x + 14 + keyW;
    const valMax = ev.width - 28 - keyW;
    let value = attr.value;
    if (ctx.measureText(value).width > valMax) {
      while (value.length > 1 && ctx.measureText(`${value}\u2026`).width > valMax) {
        value = value.slice(0, -1);
      }
      value = `${value}\u2026`;
    }
    ctx.fillText(value, valX, bodyY);
    bodyY += 20;
  }

  // --- Car tables ---
  for (const card of layout.driverCards) {
    const { x, y, width: cw, height: ch, driver: d, passengers, assignedSeats } = card;
    const seatsLabel = t('seatsCount', { used: assignedSeats, cap: d.cap });
    drawTableShell(ctx, x, y, cw, ch, 'car', t, d.name, {
      badge: seatsLabel,
      badgeTone: assignedSeats >= d.cap ? 'full' : 'ok',
    });
    ctx.font = `12px ${FONT_FAMILY}`;
    ctx.fillStyle = COLOR.muted;
    ctx.textAlign = 'left';
    ctx.textBaseline = 'middle';
    const sub = vehicleLine(d, t);
    const subMax = cw - 24;
    let subText = sub;
    if (ctx.measureText(subText).width > subMax) {
      while (subText.length > 1 && ctx.measureText(`${subText}\u2026`).width > subMax) {
        subText = subText.slice(0, -1);
      }
      subText = `${subText}\u2026`;
    }
    ctx.fillText(subText, x + 12, y + TABLE_HEADER_H + TABLE_SUB_H / 2);
    ctx.beginPath();
    ctx.moveTo(x, y + TABLE_HEADER_H + TABLE_SUB_H);
    ctx.lineTo(x + cw, y + TABLE_HEADER_H + TABLE_SUB_H);
    ctx.strokeStyle = COLOR.divider;
    ctx.lineWidth = 1;
    ctx.stroke();
    drawPassengerRows(
      ctx,
      x,
      y + TABLE_HEADER_H + TABLE_SUB_H,
      cw,
      passengers,
      t('colPassenger'),
      false,
    );
  }

  // --- Unassigned table ---
  if (layout.unassignedBox && layout.unassignedItems.length > 0) {
    const box = layout.unassignedBox;
    drawTableShell(ctx, box.x, box.y, box.width, box.height, 'unassigned', t, t('unassignedTitle'), {
      badge: String(layout.unassignedItems.length),
      badgeTone: 'amber',
    });
    drawPassengerRows(
      ctx,
      box.x,
      box.y + TABLE_HEADER_H,
      box.width,
      layout.unassignedItems,
      t('colPassenger'),
      true,
    );
  }

  // --- Area entities ---
  for (const area of layout.areaEntities) {
    drawTableShell(ctx, area.x, area.y, area.width, area.height, 'area', t, area.name, {
      badge: t('paxTag', { count: area.count }),
      badgeTone: 'ok',
    });
    let ay = area.y + TABLE_HEADER_H + 8;
    ctx.textAlign = 'left';
    ctx.textBaseline = 'top';
    ctx.font = `700 14px ${FONT_FAMILY}`;
    ctx.fillStyle = COLOR.ink;
    ctx.font = `600 11px ${FONT_FAMILY}`;
    ctx.fillStyle = COLOR.muted;
    ctx.fillText(t('colPassengers'), area.x + 12, ay);
    ay += 16;
    ctx.font = `12px ${FONT_FAMILY}`;
    ctx.fillStyle = COLOR.body;
    const shown = area.passengerNames.slice(0, 3);
    const extra = area.passengerNames.length - shown.length;
    const list = extra > 0 ? `${shown.join(', ')} +${extra}` : shown.join(', ') || '\u2014';
    ctx.font = `12px ${FONT_FAMILY}`;
    const lines = wrapText(ctx, list, area.width - 24);
    for (const line of lines) {
      ctx.fillText(line, area.x + 12, ay);
      ay += 15;
    }
  }

  // --- Standby ---
  if (layout.standbyHeight > 0) {
    const sx = PAD;
    const sy = layout.standbyY;
    const sw = contentWidth;
    const sh = layout.standbyHeight;
    drawTableShell(ctx, sx, sy, sw, sh, 'standby', t, '');
    ctx.font = `13px ${FONT_FAMILY}`;
    ctx.fillStyle = COLOR.body;
    ctx.textAlign = 'left';
    ctx.textBaseline = 'top';
    const lines = wrapText(ctx, layout.standbyNames, sw - 28);
    let ly = sy + TABLE_HEADER_H + 12;
    for (const line of lines) {
      ctx.fillText(line, sx + 14, ly);
      ly += 18;
    }
  }

  // --- Legend ---
  if (layout.legendHeight > 0) {
    drawLegend(
      ctx,
      PAD,
      layout.legendY,
      contentWidth,
      layout.legendHeight,
      t,
      layout.connectors.some((c) => c.kind === 'drivesTo'),
      layout.connectors.some((c) => c.kind === 'picksUpIn'),
      layout.connectors.some((c) => c.dashed),
    );
  }

  // --- Footer ---
  const fx = PAD;
  const fy = layout.footerY;
  const fw = contentWidth;
  const fh = layout.footerHeight;
  drawRoundedRect(ctx, fx, fy, fw, fh, 10);
  ctx.fillStyle = COLOR.white;
  ctx.fill();
  ctx.strokeStyle = COLOR.divider;
  ctx.lineWidth = 1;
  ctx.stroke();

  ctx.fillStyle = COLOR.muted;
  ctx.font = `500 12px ${FONT_FAMILY}`;
  ctx.textAlign = 'left';
  ctx.textBaseline = 'top';
  let lineY = fy + 14;
  for (const line of layout.reminderText) {
    ctx.fillText(line, fx + 16, lineY);
    lineY += 18;
  }
  ctx.fillStyle = COLOR.line;
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
