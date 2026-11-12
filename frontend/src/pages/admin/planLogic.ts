// Pure rules for the transport planning board (PlanTransport.tsx), mirroring the server's
// checks in backend/src/routes/plan.rs so the board can warn before anything is sent.

export interface PlanRide {
  id: number;
  passenger: { id: number; name: string };
  seats: number;
  area: { id: number; name: string };
  pickup_at: string;
  pickup_address: string;
  assistance: string;
  notes: string;
  status: string;
  driver_id: number | null;
  last: { driver_id: number; driver_name: string; times: number; at: string } | null;
}

export interface PlanDriver {
  id: number;
  name: string;
  carpool: boolean;
  cap: number;
  other_seats: number;
  other_rides: number;
  area_ids: number[];
  vehicle: { model: string | null; plate: string | null; seats: number | null };
}

export interface Plan {
  event: { id: number; title: string; starts_at: string; location: string; status: string };
  rides: PlanRide[];
  drivers: PlanDriver[];
}

/** Draft assignment: ride id → driver id, or null for "no driver yet". */
export type Draft = Record<number, number | null>;

export const savedDraft = (plan: Plan): Draft =>
  Object.fromEntries(plan.rides.map((r) => [r.id, r.driver_id]));

export interface Load {
  seats: number;
  rides: number;
  /** Seats used beyond the car's capacity, or a second booking for a driver who doesn't carpool. */
  over: boolean;
}

export function driverLoad(plan: Plan, draft: Draft, d: PlanDriver): Load {
  const mine = plan.rides.filter((r) => draft[r.id] === d.id);
  const seats = d.other_seats + mine.reduce((n, r) => n + r.seats, 0);
  const rides = d.other_rides + mine.length;
  return { seats, rides, over: seats > d.cap || (!d.carpool && rides > 1) };
}

/** Would adding this ride keep the driver within their seats and booking rules? */
export function fits(plan: Plan, draft: Draft, d: PlanDriver, ride: PlanRide): boolean {
  if (ride.passenger.id === d.id) return false;
  return !driverLoad(plan, { ...draft, [ride.id]: d.id }, d).over;
}

export const servesArea = (d: PlanDriver, ride: PlanRide) => d.area_ids.includes(ride.area.id);

/**
 * "Same as last time": give each passenger without a driver their previous driver, when
 * that driver is on the board and still has room. Returns the new draft and how many
 * passengers were placed.
 */
export function applyLastTime(plan: Plan, draft: Draft): { draft: Draft; placed: number; candidates: number } {
  const next: Draft = { ...draft };
  const byId = new Map(plan.drivers.map((d) => [d.id, d]));
  let placed = 0;
  let candidates = 0;
  // Most-travelled pairs first, so long-standing pairings win a contested seat.
  const open = plan.rides
    .filter((r) => next[r.id] == null && r.last && byId.has(r.last.driver_id))
    .sort((a, b) => (b.last?.times ?? 0) - (a.last?.times ?? 0));
  for (const r of open) {
    candidates++;
    const d = byId.get(r.last!.driver_id)!;
    if (fits(plan, next, d, r)) {
      next[r.id] = d.id;
      placed++;
    }
  }
  return { draft: next, placed, candidates };
}

export interface Change {
  ride_id: number;
  from_driver_id: number | null;
  driver_id: number | null;
}

/** What the server needs: every ride whose draft driver differs from the saved one. */
export const changesOf = (plan: Plan, draft: Draft): Change[] =>
  plan.rides
    .filter((r) => (draft[r.id] ?? null) !== r.driver_id)
    .map((r) => ({ ride_id: r.id, from_driver_id: r.driver_id, driver_id: draft[r.id] ?? null }));

/** Why Auto-arrange put a passenger in a car (from POST …/plan/arrange). */
export interface Reason {
  driver_id: number;
  near: { ride_id: number; name: string; km: number | null } | null;
  in_area: boolean;
  together: number;
}

export interface ArrangeResult {
  placed: (Reason & { ride_id: number })[];
  unplaced: number[];
  /** Pickups without cached coordinates, grouped by area instead of distance. */
  approximate: number;
}

export const draftEntries = (plan: Plan, draft: Draft) =>
  plan.rides.map((r) => ({ ride_id: r.id, driver_id: draft[r.id] ?? null }));
