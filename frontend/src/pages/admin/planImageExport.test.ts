import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  boxesOverlap,
  copyCanvasToClipboard,
  drawRoundedRect,
  exportPlanAsPng,
  measurePlanLayout,
  pathHitsRectInterior,
  pathsShareASegment,
  renderPlanToCanvas,
  sanitizeFilename,
  wrapText,
} from './planImageExport';
import { Plan } from './planLogic';

const mockPlan: Plan = {
  event: {
    id: 10,
    title: 'Sunday Worship Service',
    starts_at: '2026-11-22T09:30:00',
    location: 'Grace Community Church',
    status: 'published',
  },
  drivers: [
    {
      id: 1,
      name: 'Brother Barnabas',
      carpool: true,
      cap: 4,
      other_seats: 0,
      other_rides: 0,
      area_ids: [1],
      vehicle: {
        model: 'Toyota Innova',
        plate: 'WXY 8888',
        seats: 4,
        wheelchair_ok: true,
        child_seat: true,
      },
    },
    {
      id: 2,
      name: 'Sister Phoebe',
      carpool: true,
      cap: 6,
      other_seats: 0,
      other_rides: 0,
      area_ids: [2],
      vehicle: {
        model: 'Nissan Serena',
        plate: 'VAA 1234',
        seats: 6,
        low_step: true,
      },
    },
    {
      id: 3,
      name: 'Brother Timothy (Standby)',
      carpool: true,
      cap: 4,
      other_seats: 0,
      other_rides: 0,
      area_ids: [1],
      vehicle: {
        model: 'Honda City',
        plate: 'BCC 9999',
        seats: 4,
      },
    },
  ],
  rides: [
    {
      id: 101,
      passenger: { id: 201, name: 'Elder Wong' },
      seats: 1,
      area: { id: 1, name: 'Petaling Jaya' },
      pickup_at: '2026-11-22T08:45:00',
      pickup_address: '15 Jalan 14/20, Section 14, PJ',
      assistance: 'Wheelchair',
      notes: 'Needs help boarding',
      status: 'confirmed',
      driver_id: 1,
      leg: 'outbound',
      last: null,
    },
    {
      id: 102,
      passenger: { id: 202, name: 'Sister Mary' },
      seats: 2,
      area: { id: 1, name: 'Subang Jaya' },
      pickup_at: '2026-11-22T08:50:00',
      pickup_address: 'Block B, Subang Olives Condo',
      assistance: '',
      notes: '',
      status: 'confirmed',
      driver_id: 1,
      leg: 'return',
      last: null,
    },
    {
      id: 103,
      passenger: { id: 203, name: 'Brother David' },
      seats: 1,
      area: { id: 2, name: 'Cheras' },
      pickup_at: '2026-11-22T08:30:00',
      pickup_address: '22 Jalan Midah Timur',
      assistance: '',
      notes: '',
      status: 'pending_driver_confirm',
      driver_id: 2,
      leg: 'outbound',
      last: null,
    },
    {
      id: 104,
      passenger: { id: 204, name: 'Uncle Simon' },
      seats: 1,
      area: { id: 3, name: 'Klang' },
      pickup_at: '2026-11-22T08:15:00',
      pickup_address: '88 Jalan Meru',
      assistance: 'Walker',
      notes: '',
      status: 'pending_driver',
      driver_id: null,
      leg: 'outbound',
      last: null,
    },
  ],
  draft: {
    status: 'draft',
    assignments: [
      { ride_id: 101, driver_id: 1 },
      { ride_id: 102, driver_id: 1 },
      { ride_id: 103, driver_id: 2 },
      { ride_id: 104, driver_id: null },
    ],
  },
};

const mockT = (key: string, opts?: Record<string, unknown>) => {
  if (opts && typeof opts === 'object') {
    let res = key;
    for (const [k, v] of Object.entries(opts)) {
      res += ` ${k}=${v}`;
    }
    return res;
  }
  return key;
};

const mockFormatT = (_key: string) => '11/22/2026';

describe('planImageExport', () => {
  describe('wrapText', () => {
    it('wraps long strings without crashing and handles newlines', () => {
      const ctx = {
        measureText: (str: string) => ({ width: str.length * 10 }),
      } as unknown as CanvasRenderingContext2D;

      const lines = wrapText(ctx, 'Short text', 200);
      expect(lines).toEqual(['Short text']);

      const wrapped = wrapText(ctx, 'abcdefghijklmnopqrstuvwxyz', 100);
      expect(wrapped.length).toBeGreaterThan(1);

      const multiline = wrapText(ctx, 'Line 1\nLine 2', 200);
      expect(multiline).toEqual(['Line 1', 'Line 2']);

      const spaced = wrapText(ctx, 'Hello beautiful world', 120);
      expect(spaced).toEqual(['Hello', 'beautiful', 'world']);
    });
  });

  describe('sanitizeFilename', () => {
    it('removes invalid file characters and replaces whitespace', () => {
      expect(sanitizeFilename('Sunday / Worship * Service?')).toBe('Sunday___Worship___Service_');
      expect(sanitizeFilename('   ')).toBe('event');
    });
  });

  describe('measurePlanLayout', () => {
    const ctx = {
      measureText: (str: string) => ({ width: str.length * 8 }),
      font: '',
    } as unknown as CanvasRenderingContext2D;

    it('measures layout dimensions and organizes driver cards, unassigned and standby drivers', () => {
      const draft = {
        101: 1,
        102: 1,
        103: 2,
        104: null,
      };

      const layout = measurePlanLayout(ctx, mockPlan, draft, mockT, mockFormatT, false);

      expect(layout.width).toBe(1200);
      expect(layout.height).toBeGreaterThan(500);

      // Brother Barnabas and Sister Phoebe have assignments
      expect(layout.driverCards).toHaveLength(2);
      expect(layout.driverCards[0].driver.id).toBe(1);
      expect(layout.driverCards[0].passengers).toHaveLength(2);
      expect(layout.driverCards[0].assignedSeats).toBe(3); // 1 + 2

      expect(layout.driverCards[1].driver.id).toBe(2);
      expect(layout.driverCards[1].passengers).toHaveLength(1);

      // Brother Timothy is standby
      expect(layout.standbyDrivers).toHaveLength(1);
      expect(layout.standbyDrivers[0].id).toBe(3);
      expect(layout.standbyHeight).toBeGreaterThan(0);

      // Uncle Simon is unassigned
      expect(layout.unassignedItems).toHaveLength(1);
      expect(layout.unassignedItems[0].ride.id).toBe(104);
      expect(layout.unassignedHeight).toBeGreaterThan(0);
      expect(layout.allAssignedHeight).toBe(0);
    });

    it('shows all-assigned banner when every passenger has a driver', () => {
      const draft = {
        101: 1,
        102: 1,
        103: 2,
        104: 2,
      };

      const layout = measurePlanLayout(ctx, mockPlan, draft, mockT, mockFormatT, true);
      expect(layout.unassignedItems).toHaveLength(0);
      expect(layout.unassignedHeight).toBe(0);
      expect(layout.unassignedBox).toBeNull();
      expect(layout.allAssigned).toBe(true);
      expect(layout.eventEntity.attrs.some((a) => a.value === 'allAssigned')).toBe(true);
      expect(layout.statusBadge.tone).toBe('success');
    });
  });

  describe('visitors on the shared sheet', () => {
    const ctx = {
      measureText: (str: string) => ({ width: str.length * 8 }),
      font: '',
    } as unknown as CanvasRenderingContext2D;
    const visitor = (id: number, driver_id: number | null, seats = 1) => ({
      id, name: `Guest ${id}`, phone: '', notes: '', area: null, pickup_address: '', seats, leg: 'outbound' as const,
      driver_id, driver_name: null, created_by: null, created_at: '', updated_at: '', can_edit: true,
    });

    it('puts visitors in their car or under no driver, tagged as visitors without a time', () => {
      const plan = { ...mockPlan, visitors: [visitor(7, 3, 2), visitor(8, null)] };
      const draft = { 101: 1, 102: 1, 103: 2, 104: 2 };
      const layout = measurePlanLayout(ctx, plan, draft, mockT, mockFormatT, true);

      // Brother Timothy (3) was standby; his recorded visitor now gives him a card.
      const timothy = layout.driverCards.find((c) => c.driver.id === 3)!;
      expect(timothy.passengers).toHaveLength(1);
      expect(timothy.assignedSeats).toBe(2);
      expect(timothy.passengers[0].headerText).toContain('Guest 7 [visitorTag]');
      expect(timothy.passengers[0].headerText).not.toContain('\u00b7');
      expect(timothy.passengers[0].addrLines).toEqual(['\u2014']);
      expect(layout.standbyDrivers.map((d) => d.id)).not.toContain(3);

      // A visitor with no car is listed under "no driver yet".
      expect(layout.unassignedItems).toHaveLength(1);
      expect(layout.unassignedItems[0].headerText).toContain('Guest 8 [visitorTag]');
      expect(layout.areaEntities.every((a) => a.name.trim().length > 0 && a.name !== '\u2014')).toBe(true);
    });

    it('draws the card of a lone active driver, with the visitor in it', () => {
      const plan = { ...mockPlan, visitors: [visitor(7, 3, 2)] };
      const draft = { 101: null, 102: null, 103: null, 104: null };
      const layout = measurePlanLayout(ctx, plan, draft, mockT, mockFormatT, false);
      expect(layout.driverCards).toHaveLength(1);
      const card = layout.driverCards[0];
      expect(Number.isFinite(card.x) && Number.isFinite(card.y)).toBe(true);
      expect(card.passengers[0].headerText).toContain('Guest 7 [visitorTag]');
      // The unassigned table must not sit on top of the only driver table.
      expect(layout.unassignedBox).not.toBeNull();
      expect(
        boxesOverlap(
          { x: card.x, y: card.y, width: card.width, height: card.height },
          {
            x: layout.unassignedBox!.x,
            y: layout.unassignedBox!.y,
            width: layout.unassignedBox!.width,
            height: layout.unassignedBox!.height,
          },
        ),
      ).toBe(false);
    });

    it('leaves the sheet unchanged when there are no visitors', () => {
      const draft = { 101: 1, 102: 1, 103: 2, 104: null };
      const a = measurePlanLayout(ctx, mockPlan, draft, mockT, mockFormatT, false);
      const b = measurePlanLayout(ctx, { ...mockPlan, visitors: [] }, draft, mockT, mockFormatT, false);
      expect(b.height).toBe(a.height);
      expect(b.driverCards.map((c) => c.passengers.length)).toEqual(a.driverCards.map((c) => c.passengers.length));
    });
  });

  describe('renderPlanToCanvas', () => {
    const mockCtx = {
      fillStyle: '',
      strokeStyle: '',
      lineWidth: 1,
      font: '',
      textAlign: '',
      textBaseline: '',
      fillRect: vi.fn(),
      strokeRect: vi.fn(),
      fillText: vi.fn(),
      beginPath: vi.fn(),
      moveTo: vi.fn(),
      lineTo: vi.fn(),
      arcTo: vi.fn(),
      closePath: vi.fn(),
      fill: vi.fn(),
      stroke: vi.fn(),
      roundRect: vi.fn(),
      scale: vi.fn(),
      save: vi.fn(),
      restore: vi.fn(),
      setLineDash: vi.fn(),
      measureText: vi.fn((str: string) => ({ width: str.length * 8 })),
    };

    beforeEach(() => {
      vi.clearAllMocks();
      vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(mockCtx as unknown as CanvasRenderingContext2D);
    });

    it('renders plan onto a high-DPI scaled canvas with crisp 2x resolution', () => {
      const draft = { 101: 1, 102: 1, 103: 2, 104: null };
      const canvas = renderPlanToCanvas({
        plan: mockPlan,
        draft,
        t: mockT,
        formatT: mockFormatT,
        scale: 2,
      });

      expect(canvas.width).toBe(2400); // 1200 * 2
      expect(canvas.height).toBeGreaterThan(1000);
      expect(mockCtx.scale).toHaveBeenCalledWith(2, 2);
      expect(mockCtx.fillText).toHaveBeenCalled();
    });

    it('draws rounded rect using arcTo fallback when roundRect is missing', () => {
      const fallbackCtx = {
        beginPath: vi.fn(),
        moveTo: vi.fn(),
        arcTo: vi.fn(),
        closePath: vi.fn(),
      } as unknown as CanvasRenderingContext2D;

      drawRoundedRect(fallbackCtx, 10, 10, 100, 50, 8);
      expect(fallbackCtx.arcTo).toHaveBeenCalledTimes(4);
      expect(fallbackCtx.closePath).toHaveBeenCalled();
    });

    it('exports PNG data url and triggers download', () => {
      vi.spyOn(HTMLCanvasElement.prototype, 'toDataURL').mockReturnValue('data:image/png;base64,iVBORw0KGgo=');
      const clickSpy = vi.fn();
      const origCreateElement = document.createElement.bind(document);
      const createElementSpy = vi.spyOn(document, 'createElement').mockImplementation((tag: string) => {
        if (tag === 'a') {
          const el = origCreateElement('a');
          el.click = clickSpy;
          return el;
        }
        return origCreateElement(tag);
      });

      const draft = { 101: 1, 102: 1, 103: 2, 104: null };
      const url = exportPlanAsPng({
        plan: mockPlan,
        draft,
        t: mockT,
        formatT: mockFormatT,
      });

      expect(url).toContain('data:image/png;base64');
      expect(clickSpy).toHaveBeenCalled();
      createElementSpy.mockRestore();
    });

    it('handles clipboard copy failure gracefully when clipboard API is absent', async () => {
      const canvas = document.createElement('canvas');
      const ok = await copyCanvasToClipboard(canvas);
      expect(ok).toBe(false);
    });
  });

  describe('ER diagram layout', () => {
    const ctx = {
      measureText: (str: string) => ({ width: str.length * 8 }),
      font: '',
    } as unknown as CanvasRenderingContext2D;

    const draft = { 101: 1, 102: 1, 103: 2, 104: null };

    const allBoxes = (layout: ReturnType<typeof measurePlanLayout>) => {
      const boxes = [
        { x: layout.eventEntity.x, y: layout.eventEntity.y, width: layout.eventEntity.width, height: layout.eventEntity.height },
        ...layout.driverCards.map((c) => ({ x: c.x, y: c.y, width: c.width, height: c.height })),
        ...layout.areaEntities.map((a) => ({ x: a.x, y: a.y, width: a.width, height: a.height })),
      ];
      if (layout.unassignedBox) {
        boxes.push({
          x: layout.unassignedBox.x,
          y: layout.unassignedBox.y,
          width: layout.unassignedBox.width,
          height: layout.unassignedBox.height,
        });
      }
      if (layout.standbyHeight > 0) {
        boxes.push({ x: 48, y: layout.standbyY, width: 1104, height: layout.standbyHeight });
      }
      return boxes;
    };

    it('places an event entity, area entities, and labeled connectors', () => {
      const layout = measurePlanLayout(ctx, mockPlan, draft, mockT, mockFormatT, false);

      expect(layout.eventEntity.titleLines.join(' ')).toContain('Sunday Worship Service');
      expect(layout.eventEntity.width).toBeGreaterThan(0);
      expect(layout.eventEntity.y).toBeGreaterThan(layout.headerY);

      const areaNames = layout.areaEntities.map((a) => a.name).sort();
      expect(areaNames).toEqual(['Cheras', 'Klang', 'Petaling Jaya', 'Subang Jaya']);

      const drives = layout.connectors.filter((c) => c.kind === 'drivesTo');
      const picks = layout.connectors.filter((c) => c.kind === 'picksUpIn');
      expect(drives).toHaveLength(2);
      expect(drives.every((c) => c.label === 'relDrivesTo')).toBe(true);
      expect(picks.length).toBeGreaterThan(0);
      expect(picks.every((c) => c.label === 'relPicksUpIn')).toBe(true);
      expect(picks.some((c) => c.dashed)).toBe(true);
    });

    it('keeps connector paths out of table interiors and boxes off each other', () => {
      const layout = measurePlanLayout(ctx, mockPlan, draft, mockT, mockFormatT, false);
      const boxes = allBoxes(layout);

      for (let i = 0; i < boxes.length; i++) {
        for (let j = i + 1; j < boxes.length; j++) {
          expect(boxesOverlap(boxes[i], boxes[j])).toBe(false);
        }
      }

      for (const c of layout.connectors) {
        expect(c.points.length).toBeGreaterThanOrEqual(2);
        for (const box of boxes) {
          expect(pathHitsRectInterior(c.points, box)).toBe(false);
        }
      }
    });

    it('lays out many cars in a two-column grid without overlaps', () => {
      const extraDrivers = Array.from({ length: 4 }, (_, i) => ({
        id: 10 + i,
        name: `Driver ${i + 4}`,
        carpool: true,
        cap: 4,
        other_seats: 0,
        other_rides: 0,
        area_ids: [1],
        vehicle: { model: 'Vios', plate: `ABC ${i}`, seats: 4 },
      }));
      const extraRides = extraDrivers.map((d, i) => ({
        id: 201 + i,
        passenger: { id: 301 + i, name: `Rider ${i}` },
        seats: 1,
        area: { id: 1, name: 'Petaling Jaya' },
        pickup_at: '2026-11-22T08:45:00',
        pickup_address: `${i + 1} Jalan Test`,
        assistance: '',
        notes: '',
        status: 'confirmed',
        driver_id: d.id,
        leg: 'outbound' as const,
        last: null,
      }));
      const plan = {
        ...mockPlan,
        drivers: [...mockPlan.drivers, ...extraDrivers],
        rides: [...mockPlan.rides, ...extraRides],
      };
      const manyDraft: Record<number, number | null> = { ...draft };
      for (const r of extraRides) manyDraft[r.id] = r.driver_id;

      const layout = measurePlanLayout(ctx, plan, manyDraft, mockT, mockFormatT, false);
      expect(layout.driverCards).toHaveLength(6);
      expect(layout.width).toBe(1200);
      const boxes = allBoxes(layout);
      for (let i = 0; i < boxes.length; i++) {
        for (let j = i + 1; j < boxes.length; j++) {
          expect(boxesOverlap(boxes[i], boxes[j])).toBe(false);
        }
      }
      for (const c of layout.connectors) {
        for (const box of boxes) {
          expect(pathHitsRectInterior(c.points, box)).toBe(false);
        }
      }
    });

    it('keeps drives-to and picks-up-in on fully separate paths', () => {
      const layout = measurePlanLayout(ctx, mockPlan, draft, mockT, mockFormatT, false);
      const drives = layout.connectors.filter((c) => c.kind === 'drivesTo');
      const picks = layout.connectors.filter((c) => c.kind === 'picksUpIn');
      expect(drives.length).toBeGreaterThan(0);
      expect(picks.length).toBeGreaterThan(0);
      for (const d of drives) {
        for (const p of picks) {
          expect(pathsShareASegment(d.points, p.points)).toBe(false);
        }
      }
    });

    it('detects a shared colinear segment but not a mere endpoint touch', () => {
      expect(
        pathsShareASegment(
          [{ x: 0, y: 0 }, { x: 0, y: 50 }],
          [{ x: 0, y: 20 }, { x: 0, y: 80 }],
        ),
      ).toBe(true);
      expect(
        pathsShareASegment(
          [{ x: 0, y: 0 }, { x: 0, y: 20 }],
          [{ x: 0, y: 20 }, { x: 40, y: 20 }],
        ),
      ).toBe(false);
    });
  });
});
