import type { Ride } from './components/RideCard';

const SNAPSHOT_KEY = 'gr_today_rides_v1';
const QUEUE_KEY = 'gr_offline_actions_v1';
const QUEUE_EVENT = 'gr:offline-queue-changed';

let lockedInMemory = false;
if (typeof window !== 'undefined') {
  window.addEventListener('gr:session-locked', () => {
    lockedInMemory = true;
  });
  window.addEventListener('gr:session-unlocked', () => {
    lockedInMemory = false;
  });
  window.addEventListener('gr:unauthorized', () => {
    lockedInMemory = false;
  });
}

function isLocalSessionLocked(): boolean {
  if (lockedInMemory) return true;
  try {
    return typeof localStorage !== 'undefined' && Boolean(localStorage.getItem('gr_locked_user'));
  } catch {
    return false;
  }
}

function todayStr(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

const ACTIVE_STATUSES = new Set([
  'pending_driver',
  'pending_passenger',
  'pending_approval',
  'pending_driver_confirm',
  'confirmed',
  'in_progress',
]);

export interface TodaySnapshot {
  userId: number;
  date: string;
  savedAt: string;
  rides: Ride[];
}

function nowLocalIso(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

function clearSnapshotOnly(): void {
  try {
    localStorage.removeItem(SNAPSHOT_KEY);
  } catch {
    // storage unavailable
  }
}

/** Wipe any saved offline trip snapshot and queue immediately (on logout, 401, or AutoLock). */
export function clearOfflineSnapshots(): void {
  try {
    localStorage.removeItem(SNAPSHOT_KEY);
    localStorage.removeItem(QUEUE_KEY);
    if (typeof window !== 'undefined') {
      window.dispatchEvent(new Event('gr:offline-queue-abort'));
      window.dispatchEvent(new Event(QUEUE_EVENT));
    }
  } catch {
    // storage unavailable
  }
}

/** Forget a saved lock flag. Call after a fresh sign-in or when a session is invalidated. */
export function clearAutoLock(): void {
  try {
    if (typeof localStorage !== 'undefined') {
      localStorage.removeItem('gr_locked_user');
    }
  } catch {
    /* ignore storage errors */
  }
}

/** Clear all client-side session caches, drafts, passkey hints, and lock markers. */
export function clearClientSessionStorage(): void {
  clearOfflineSnapshots();
  try {
    if (typeof localStorage !== 'undefined') {
      localStorage.removeItem('gr_autolock_mins');
      localStorage.removeItem('gr_locked_user');
      localStorage.removeItem('gr_last_active');
      localStorage.removeItem('gr_device_passkey');
      for (const k of Object.keys(localStorage)) {
        if (k.startsWith('graceride_help_draft') || k.startsWith('gr-share-loc-')) {
          localStorage.removeItem(k);
        }
      }
    }
    if (typeof sessionStorage !== 'undefined') {
      sessionStorage.removeItem('graceride_walkthrough');
    }
  } catch {
    /* ignore storage errors */
  }
}

function readRaw(userId: number): TodaySnapshot | null {
  try {
    const raw = localStorage.getItem(SNAPSHOT_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as TodaySnapshot;
    if (
      !parsed ||
      parsed.userId !== userId ||
      parsed.date !== todayStr() ||
      !Array.isArray(parsed.rides)
    ) {
      clearSnapshotOnly();
      return null;
    }
    return parsed;
  } catch {
    clearSnapshotOnly();
    return null;
  }
}

/**
 * Save only the signed-in member's or driver's own active rides for today.
 * Staff-only views and terminal/future rides are excluded to preserve privacy.
 */
export function saveTodayRidesSnapshot(userId: number, rides: Ride[]): void {
  if (isLocalSessionLocked()) {
    clearSnapshotOnly();
    return;
  }
  const today = todayStr();
  const eligible = rides.filter(
    (r) =>
      r.pickup_at?.slice(0, 10) === today &&
      ACTIVE_STATUSES.has(r.status) &&
      Boolean(r.viewer?.is_passenger || r.viewer?.is_driver),
  );
  const existing = readRaw(userId);
  const byId = new Map<number, Ride>();
  for (const r of existing?.rides ?? []) {
    if (ACTIVE_STATUSES.has(r.status) && r.pickup_at?.slice(0, 10) === today) {
      byId.set(r.id, r);
    }
  }
  for (const r of eligible) {
    byId.set(r.id, r);
  }
  // If a ride in today's batch transitioned to terminal status, drop it from the cache.
  for (const r of rides) {
    if (!ACTIVE_STATUSES.has(r.status)) {
      byId.delete(r.id);
    }
  }
  const merged = [...byId.values()].sort((a, b) => a.pickup_at.localeCompare(b.pickup_at));
  if (merged.length === 0) {
    clearSnapshotOnly();
    return;
  }
  try {
    const snap: TodaySnapshot = {
      userId,
      date: today,
      savedAt: nowLocalIso(),
      rides: merged,
    };
    localStorage.setItem(SNAPSHOT_KEY, JSON.stringify(snap));
  } catch {
    // storage unavailable
  }
}

export function loadTodayRidesSnapshot(userId: number): TodaySnapshot | null {
  return readRaw(userId);
}

export function loadTodayRideById(userId: number, rideId: number): { ride: Ride; savedAt: string } | null {
  const snap = readRaw(userId);
  if (!snap) return null;
  const ride = snap.rides.find((r) => r.id === rideId);
  return ride ? { ride, savedAt: snap.savedAt } : null;
}
