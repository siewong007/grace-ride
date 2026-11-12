import { describe, expect, it } from 'vitest';
import { applyLastTime, changesOf, driverLoad, fits, Plan, PlanRide, savedDraft } from './planLogic';

const ride = (id: number, last?: { driver_id: number; times: number }, seats = 1): PlanRide => ({
  id,
  passenger: { id: 100 + id, name: `P${id}` },
  seats,
  area: { id: 1, name: 'PJ' },
  pickup_at: '2026-11-20T19:00',
  pickup_address: '',
  assistance: '',
  notes: '',
  status: 'pending_driver',
  driver_id: null,
  last: last ? { ...last, driver_name: `D${last.driver_id}`, at: '2026-11-01T09:00' } : null,
});

const plan: Plan = {
  event: { id: 9, title: 'Night', starts_at: '2026-11-20T19:30', location: 'Hall', status: 'published' },
  rides: [ride(1, { driver_id: 1, times: 5 }), ride(2, { driver_id: 1, times: 1 }), ride(3, { driver_id: 2, times: 2 }), ride(4)],
  drivers: [
    { id: 1, name: 'Carpooler', carpool: true, cap: 1, other_seats: 0, other_rides: 0, area_ids: [1], vehicle: { model: null, plate: null, seats: 5 } },
    { id: 2, name: 'Solo', carpool: false, cap: 2, other_seats: 0, other_rides: 0, area_ids: [], vehicle: { model: null, plate: null, seats: 5 } },
  ],
};

describe('planLogic', () => {
  it('counts seats and the one-booking rule', () => {
    const solo = plan.drivers[1];
    expect(driverLoad(plan, { 3: 2 }, solo)).toEqual({ seats: 1, rides: 1, over: false });
    expect(driverLoad(plan, { 3: 2, 4: 2 }, solo).over).toBe(true); // doesn't carpool
    expect(fits(plan, {}, plan.drivers[0], plan.rides[0])).toBe(true);
  });

  it('same-as-last-time favours long-standing pairs and respects seats', () => {
    const { draft, placed, candidates } = applyLastTime(plan, savedDraft(plan));
    expect(candidates).toBe(3);
    expect(placed).toBe(2);
    expect(draft[1]).toBe(1); // 5 rides together beats 1 for the single seat
    expect(draft[2]).toBeNull();
    expect(draft[3]).toBe(2);
  });

  it('lists only real changes with the driver the board saw', () => {
    const p = { ...plan, rides: plan.rides.map((r) => (r.id === 4 ? { ...r, driver_id: 2 } : r)) };
    const draft = { ...savedDraft(p), 1: 1, 4: null };
    expect(changesOf(p, draft)).toEqual([
      { ride_id: 1, from_driver_id: null, driver_id: 1 },
      { ride_id: 4, from_driver_id: 2, driver_id: null },
    ]);
  });
});
