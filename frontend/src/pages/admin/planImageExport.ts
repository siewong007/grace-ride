// High-resolution canvas rendering for exporting transport assignments as an image.
// Drawn as an ER / database diagram so drivers can see cars, areas and the event at a glance.
// Designed for crystal-clear readability (Retina 2x/3x scale) when shared on mobile chats.
import { fmtDateTime, fmtTime } from '../../format';
import { Draft, Plan, PlanDriver, PlanRide } from './planLogic';

const CANVAS_WIDTH = 1200;
const PAD = 56;
const GAP = 22;
const CENTER_GUTTER = 84;
const SIDE_GUTTER = 32;
const TOP_GUTTER = 76;
const AREA_GAP = 24;
const SLIM_BAR_H = 36;
const TABLE_HEADER_H = 34;
const TABLE_SUB_H = 22;
const ACCENT_H = 3;
const RADIUS_TABLE = 5;

const FONT_FAMILY =
  'system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "PingFang SC", "Noto Sans CJK SC", "Microsoft YaHei", sans-serif';

const COLOR = {
  canvas: '#f7f7f5',
  brand: '#57534e',
  white: '#ffffff',
  ink: '#292524',
  body: '#44403c',
  muted: '#78716c',
  line: '#a8a29e',
  lineStrong: '#78716c',
  driveLine: '#4a6fa5',
  areaLine: '#5a8a6a',
  divider: '#e8e6e3',
  note: '#a16207',
  eventAccent: '#4a6fa5',
  carAccent: '#4a6fa5',
  unassignedAccent: '#b45309',
  areaAccent: '#5a8a6a',
  standbyAccent: '#a8a29e',
  seatsFullFg: '#92400e',
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
  const seatsLabel = r.seats > 1 ? ` (${t('passengerSeats', { count: r.seats })})` : '';
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
    6 +
    15 +
    2 +
    Math.max(1, addrLines.length) * 15 +
    (noteLines.length > 0 ? 2 + noteLines.length * 14 : 0) +
    6;
  return {
    ride: r,
    headerText,
    addrLines: addrLines.length > 0 ? addrLines : [addrRaw],
    noteLines,
    height: itemHeight,
  };
}

function tableBodyHeight(passengers: MeasuredPassengerItem[], empty: boolean): number {
  if (empty) return 32;
  return passengers.reduce((acc, p) => acc + p.height, 0);
}

function vehicleLine(d: PlanDriver, t: TranslationFn): string {
  return [
    d.vehicle.model,
    d.vehicle.plate,
    d.carpool ? null : t('oneBooking'),
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

  const driverIds = new Set(plan.drivers.map((d) => d.id));
  const totalRiders = plan.rides.reduce((acc, r) => acc + (r.seats || 1), 0);
  const unassignedRides = plan.rides.filter((r) => draft[r.id] == null || !driverIds.has(draft[r.id]!));
  const assignedRides = plan.rides.filter((r) => draft[r.id] != null && driverIds.has(draft[r.id]!));
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
  const eventWidth = Math.min(520, contentWidth);
  const eventInner = eventWidth - 32;
  ctx.font = `600 20px ${FONT_FAMILY}`;
  const titleLines = wrapText(ctx, eventTitle, eventInner);
  const whenWhere = [fmtDateTime(plan.event.starts_at, formatT), plan.event.location || '']
    .filter((x) => x && String(x).trim())
    .join(' \u00b7 ');
  const seatsLine = `${t('totalPassengers', { count: totalRiders })} \u00b7 ${t('seatsCount', { used: totalAssignedSeats, cap: totalCap })}`;
  const eventAttrs: Array<{ key: string; value: string }> = [
    { key: 'when', value: whenWhere },
    { key: 'seats', value: seatsLine },
  ];
  const eventHeight =
    TABLE_HEADER_H +
    8 +
    Math.max(1, titleLines.length) * 24 +
    6 +
    18 +
    16 +
    14;
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
    140,
    Math.min(200, (areaAvail - AREA_GAP * (Math.min(areaCount, 5) - 1)) / Math.min(areaCount, 5)),
  );
  const areasPerRow = Math.max(1, Math.floor((areaAvail + AREA_GAP) / (areaWidth + AREA_GAP)));

  ctx.font = `600 13px ${FONT_FAMILY}`;
  const measuredAreasUnplaced = areas.map((a) => {
    const nameLines = wrapText(ctx, a.name, areaWidth - 40);
    const height = Math.max(44, 12 + Math.max(1, nameLines.length) * 18 + 12);
    return { ...a, nameLines, listLines: [] as string[], height };
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
    .map((d) => `${d.name} (${d.cap})`)
    .join(' \u00b7 ');
  if (standbyDrivers.length > 0) {
    standbyY = areasBottom + GAP;
    ctx.font = `13px ${FONT_FAMILY}`;
    const nameLines = wrapText(ctx, `${t('standbyDrivers')}  ·  ${standbyNames}`, contentWidth);
    standbyHeight = Math.max(24, nameLines.length * 18);
  }

  // --- Footer ---
  ctx.font = `12px ${FONT_FAMILY}`;
  const reminderRaw = t('reminderNote');
  const reminderText = wrapText(ctx, reminderRaw, contentWidth - 260);
  const footerHeight = Math.max(32, reminderText.length * 18 + 8);
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

function accentColor(kind: 'event' | 'car' | 'area' | 'unassigned' | 'standby'): string {
  switch (kind) {
    case 'event':
      return COLOR.eventAccent;
    case 'car':
      return COLOR.carAccent;
    case 'area':
      return COLOR.areaAccent;
    case 'unassigned':
      return COLOR.unassignedAccent;
    case 'standby':
      return COLOR.standbyAccent;
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
  opts?: { badge?: string; badgeTone?: 'success' | 'amber' | 'ok' | 'full'; hideKind?: boolean },
) {
  ctx.save();
  drawRoundedRect(ctx, x, y, w, h, RADIUS_TABLE);
  ctx.fillStyle = COLOR.white;
  ctx.fill();
  ctx.strokeStyle = COLOR.divider;
  ctx.lineWidth = 1;
  ctx.stroke();

  // Thin colored top accent, clipped to the rounded corners.
  ctx.save();
  drawRoundedRect(ctx, x, y, w, h, RADIUS_TABLE);
  if (typeof ctx.clip === 'function') ctx.clip();
  ctx.fillStyle = accentColor(kind);
  ctx.fillRect(x, y, w, ACCENT_H);
  ctx.restore();

  const midY = y + ACCENT_H + (TABLE_HEADER_H - ACCENT_H) / 2;
  let titleX = x + 12;
  let titleMax = w - 24;
  if (!opts?.hideKind) {
    const kindText = entityKindLabel(kind, t).toUpperCase();
    ctx.font = `600 9px ${FONT_FAMILY}`;
    ctx.fillStyle = accentColor(kind);
    ctx.textAlign = 'left';
    ctx.textBaseline = 'middle';
    ctx.fillText(kindText, x + 12, midY);
    titleX = x + 12 + ctx.measureText(kindText).width + (title ? 8 : 0);
    titleMax = w - 24 - (titleX - x - 12);
  }

  if (opts?.badge) {
    ctx.font = `500 12px ${FONT_FAMILY}`;
    const badgeW = ctx.measureText(opts.badge).width;
    const badgeX = x + w - 12 - badgeW;
    const fg = opts.badgeTone === 'amber' || opts.badgeTone === 'full' ? COLOR.seatsFullFg : COLOR.muted;
    ctx.fillStyle = fg;
    ctx.textAlign = 'left';
    ctx.textBaseline = 'middle';
    ctx.fillText(opts.badge, badgeX, midY);
    titleMax = badgeX - titleX - 8;
  }

  if (!title) {
    ctx.restore();
    return;
  }
  ctx.font = `600 14px ${FONT_FAMILY}`;
  ctx.fillStyle = COLOR.ink;
  ctx.textAlign = 'left';
  ctx.textBaseline = 'middle';
  let drawnTitle = title;
  if (ctx.measureText(drawnTitle).width > titleMax) {
    while (drawnTitle.length > 1 && ctx.measureText(`${drawnTitle}\u2026`).width > titleMax) {
      drawnTitle = drawnTitle.slice(0, -1);
    }
    drawnTitle = `${drawnTitle}\u2026`;
  }
  ctx.fillText(drawnTitle, titleX, midY);
  ctx.restore();
}

function drawPassengerRows(
  ctx: CanvasRenderingContext2D,
  x: number,
  startY: number,
  w: number,
  items: MeasuredPassengerItem[],
  unassigned: boolean,
) {
  let y = startY;
  items.forEach((p, i) => {
    if (i > 0) {
      ctx.beginPath();
      ctx.moveTo(x + 12, y);
      ctx.lineTo(x + w - 12, y);
      ctx.strokeStyle = COLOR.divider;
      ctx.lineWidth = 1;
      ctx.stroke();
    }
    ctx.fillStyle = unassigned ? COLOR.seatsFullFg : COLOR.ink;
    ctx.font = `600 13px ${FONT_FAMILY}`;
    ctx.textAlign = 'left';
    ctx.textBaseline = 'top';
    ctx.fillText(p.headerText, x + 12, y + 6);

    ctx.fillStyle = COLOR.muted;
    ctx.font = `12px ${FONT_FAMILY}`;
    let ty = y + 6 + 15 + 2;
    for (const line of p.addrLines) {
      ctx.fillText(line, x + 12, ty);
      ty += 15;
    }
    if (p.noteLines.length > 0) {
      ctx.fillStyle = COLOR.note;
      ctx.font = `500 11px ${FONT_FAMILY}`;
      for (const line of p.noteLines) {
        ctx.fillText(line, x + 12, ty);
        ty += 14;
      }
    }
    y += p.height;
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
  const arm = 7;
  ctx.save();
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.2;
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
  ctx.lineWidth = 1.25;
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  if (c.dashed && typeof ctx.setLineDash === 'function') {
    ctx.setLineDash([5, 4]);
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
    ctx.lineWidth = 1.25;
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

  // --- Brand line (no filled bar, no status badge — status sits once on the event card) ---
  const hx = PAD;
  const hy = layout.headerY;
  const hh = layout.headerHeight;
  ctx.fillStyle = COLOR.brand;
  ctx.font = `500 12px ${FONT_FAMILY}`;
  ctx.textAlign = 'left';
  ctx.textBaseline = 'middle';
  ctx.fillText(`${t('brandTitle')} \u00b7 ${t('brandSubtitle')}`, hx, hy + hh / 2);

  // --- Connectors behind the tables so crow's feet sit on the edges once redrawn after ---
  for (const c of layout.connectors) {
    drawConnector(ctx, c);
  }

  // --- Event entity ---
  const ev = layout.eventEntity;
  drawTableShell(ctx, ev.x, ev.y, ev.width, ev.height, 'event', t, '', {
    badge: layout.statusBadge.tone === 'success' ? t('statusApproved') : t('statusDraft'),
  });
  let bodyY = ev.y + TABLE_HEADER_H + 8;
  ctx.textAlign = 'left';
  ctx.textBaseline = 'top';
  ctx.font = `600 18px ${FONT_FAMILY}`;
  ctx.fillStyle = COLOR.ink;
  for (const line of ev.titleLines) {
    ctx.fillText(line, ev.x + 16, bodyY);
    bodyY += 24;
  }
  bodyY += 6;
  ctx.font = `400 13px ${FONT_FAMILY}`;
  ctx.fillStyle = COLOR.muted;
  for (const attr of ev.attrs) {
    ctx.fillText(attr.value, ev.x + 16, bodyY);
    bodyY += 18;
  }

  // --- Car tables ---
  for (const card of layout.driverCards) {
    const { x, y, width: cw, height: ch, driver: d, passengers, assignedSeats } = card;
    const seatsLabel = `${assignedSeats}/${d.cap}`;
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
    drawPassengerRows(ctx, x, y + TABLE_HEADER_H + TABLE_SUB_H, cw, passengers, false);
  }

  // --- Unassigned table ---
  if (layout.unassignedBox && layout.unassignedItems.length > 0) {
    const box = layout.unassignedBox;
    drawTableShell(ctx, box.x, box.y, box.width, box.height, 'unassigned', t, t('unassignedTitle'), {
      badge: String(layout.unassignedItems.length),
      badgeTone: 'amber',
      hideKind: true,
    });
    drawPassengerRows(ctx, box.x, box.y + TABLE_HEADER_H, box.width, layout.unassignedItems, true);
  }

  // --- Area chips ---
  for (const area of layout.areaEntities) {
    drawTableShell(ctx, area.x, area.y, area.width, area.height, 'area', t, area.name, {
      badge: String(area.count),
      hideKind: true,
    });
  }

  // --- Standby (quiet line, no card) ---
  if (layout.standbyHeight > 0) {
    ctx.font = `400 13px ${FONT_FAMILY}`;
    ctx.fillStyle = COLOR.muted;
    ctx.textAlign = 'left';
    ctx.textBaseline = 'top';
    const lines = wrapText(ctx, `${t('standbyDrivers')}  \u00b7  ${layout.standbyNames}`, contentWidth);
    let ly = layout.standbyY;
    for (const line of lines) {
      ctx.fillText(line, PAD, ly);
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
  ctx.fillStyle = COLOR.muted;
  ctx.font = `400 12px ${FONT_FAMILY}`;
  ctx.textAlign = 'left';
  ctx.textBaseline = 'top';
  let lineY = fy;
  for (const line of layout.reminderText) {
    ctx.fillText(line, fx, lineY);
    lineY += 18;
  }
  ctx.fillStyle = COLOR.line;
  ctx.textAlign = 'right';
  ctx.fillText(t('generatedAt', { time: layout.timestamp }), fx + fw, fy);
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
