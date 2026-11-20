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
  leg?: 'outbound' | 'return';
  paired_ride_id?: number | null;
  last: { driver_id: number; driver_name: string; times: number; at: string } | null;
  /** The group drivers are matched on (the host's for a visitor). None: any driver. */
  match_group_id?: number | null;
}

export interface PlanDriver {
  id: number;
  name: string;
  carpool: boolean;
  cap: number;
  other_seats: number;
  other_rides: number;
  other_outbound_seats?: number;
  other_outbound_rides?: number;
  other_return_seats?: number;
  other_return_rides?: number;
  area_ids: number[];
  /** Groups this driver may drive for: their groups and every group below them. */
  drive_group_ids?: number[];
  vehicle: {
    model: string | null;
    plate: string | null;
    seats: number | null;
    wheelchair_ok?: boolean;
    low_step?: boolean;
    child_seat?: boolean;
  };
}

/** May this driver take this rider without an admin override? (`audience::drives_for`) */
export const inGroup = (d: PlanDriver, r: PlanRide): boolean =>
  r.match_group_id == null || (d.drive_group_ids ?? []).includes(r.match_group_id);

export interface PlanDraft {
  status: 'draft' | 'pending_approval' | 'approved' | 'returned';
  assignments: Array<{ ride_id: number; driver_id: number | null }>;
  updated_by?: { id: number; name: string };
  updated_at?: string;
  submitted_by?: { id: number; name: string } | null;
  submitted_at?: string | null;
  reviewed_by?: { id: number; name: string } | null;
  reviewed_at?: string | null;
  review_notes?: string;
}

/** Someone without an account noted on the plan for the record (no ride, no messages). */
export interface PlanVisitor {
  id: number;
  name: string;
  phone: string;
  notes: string;
  area: { id: number; name: string | null } | null;
  pickup_address: string;
  seats: number;
  leg: 'outbound' | 'return';
  /** The car they ride in, for the record; the driver is not notified. */
  driver_id: number | null;
  driver_name: string | null;
  created_by: { id: number; name: string | null } | null;
  created_at: string;
  updated_at: string;
  /** The recorder, or a traffic lead. */
  can_edit: boolean;
}

export interface Plan {
  event: { id: number; title: string; starts_at: string; location: string; status: string };
  rides: PlanRide[];
  drivers: PlanDriver[];
  draft?: PlanDraft | null;
  can_approve?: boolean;
  visitors?: PlanVisitor[];
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

export function driverLoad(plan: Plan, draft: Draft, d: PlanDriver, leg?: 'outbound' | 'return'): Load {
  if (leg) {
    const mine = plan.rides.filter(
      (r) => draft[r.id] === d.id && (r.leg === 'return' ? 'return' : 'outbound') === leg,
    );
    const baseSeats =
      leg === 'return' ? (d.other_return_seats ?? 0) : (d.other_outbound_seats ?? d.other_seats ?? 0);
    const baseRides =
      leg === 'return' ? (d.other_return_rides ?? 0) : (d.other_outbound_rides ?? d.other_rides ?? 0);
    // Visitors recorded in this car take seats but are not bookings.
    const visitorSeats = (plan.visitors ?? [])
      .filter((v) => v.driver_id === d.id && v.leg === leg)
      .reduce((n, v) => n + v.seats, 0);
    const seats = baseSeats + visitorSeats + mine.reduce((n, r) => n + r.seats, 0);
    const rides = baseRides + mine.length;
    return { seats, rides, over: seats > d.cap || (!d.carpool && rides > 1) };
  }
  const out = driverLoad(plan, draft, d, 'outbound');
  const hasReturn =
    plan.rides.some((r) => r.leg === 'return') ||
    (d.other_return_rides ?? 0) > 0 ||
    (plan.visitors ?? []).some((v) => v.driver_id === d.id && v.leg === 'return');
  if (!hasReturn) return out;
  const ret = driverLoad(plan, draft, d, 'return');
  return {
    seats: Math.max(out.seats, ret.seats),
    rides: out.rides + ret.rides,
    over: out.over || ret.over,
  };
}

/** Check whether a driver's vehicle supports the rider's mobility/equipment requirements. */
export function vehicleCompatible(d: PlanDriver, ride: PlanRide): boolean {
  const raw = ride.assistance || '';
  const lower = raw.toLowerCase();
  if (
    (lower.includes('wheelchair') ||
      lower.includes('kerusi roda') ||
      raw.includes('\u8f6e\u6905') ||
      raw.includes('\u8f2a\u6905')) &&
    !d.vehicle.wheelchair_ok
  ) {
    return false;
  }
  if (
    (lower.includes('stroller') ||
      lower.includes('child seat') ||
      lower.includes('child_seat') ||
      lower.includes('kereta sorong') ||
      raw.includes('\u5a74\u513f\u8f66') ||
      raw.includes('\u5b30\u5152\u8eca') ||
      raw.includes('\u513f\u7ae5\u5ea7\u6905') ||
      raw.includes('\u5152\u7ae5\u5ea7\u6905') ||
      raw.includes('\u5b89\u5168\u5ea7\u6905')) &&
    !d.vehicle.child_seat
  ) {
    return false;
  }
  return true;
}

/** Would adding this ride keep the driver within their seats, booking rules, and vehicle accessibility? */
export function fits(plan: Plan, draft: Draft, d: PlanDriver, ride: PlanRide): boolean {
  if (ride.passenger.id === d.id) return false;
  if (!vehicleCompatible(d, ride)) return false;
  const leg = ride.leg === 'return' ? 'return' : 'outbound';
  return !driverLoad(plan, { ...draft, [ride.id]: d.id }, d, leg).over;
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

/**
 * "Match return drivers": assign unassigned return-leg rides to the same driver handling
 * their paired outbound ride (or the same passenger's outbound ride) when the car fits.
 */
export function applyMatchReturnDrivers(
  plan: Plan,
  draft: Draft,
): { draft: Draft; placed: number; candidates: number } {
  const next: Draft = { ...draft };
  const byDriverId = new Map(plan.drivers.map((d) => [d.id, d]));
  const rideById = new Map(plan.rides.map((r) => [r.id, r]));
  let placed = 0;
  let candidates = 0;
  for (const r of plan.rides) {
    if (r.leg !== 'return' || next[r.id] != null) continue;
    const paired =
      (r.paired_ride_id != null ? rideById.get(r.paired_ride_id) : undefined) ??
      plan.rides.find((o) => o.id !== r.id && o.passenger.id === r.passenger.id && o.leg !== 'return');
    if (!paired) continue;
    const outboundDriverId = next[paired.id] ?? null;
    if (outboundDriverId == null) continue;
    const d = byDriverId.get(outboundDriverId);
    if (!d) continue;
    candidates++;
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
