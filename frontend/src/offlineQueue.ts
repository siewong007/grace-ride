import { useEffect, useState } from 'react';
import { ApiError, post } from './api';

const QUEUE_KEY = 'gr_offline_actions_v1';
export const OFFLINE_QUEUE_EVENT = 'gr:offline-queue-changed';

export interface OfflineAction {
  id: string;
  userId: number;
  rideId: number;
  path: string;
  body?: unknown;
  label: string;
  createdAt: string;
}

function isApiError(err: unknown): err is ApiError {
  if (typeof ApiError === 'function' && err instanceof ApiError) return true;
  return (
    typeof err === 'object' &&
    err !== null &&
    'status' in err &&
    typeof (err as { status?: unknown }).status === 'number'
  );
}

/**
 * True when a request could not reach the server (offline browser or network failure),
 * and never true for a structured HTTP response (4xx/5xx).
 */
export function isOfflineNetworkError(err: unknown): boolean {
  if (isApiError(err)) return false;
  if (typeof navigator !== 'undefined' && navigator.onLine === false) return true;
  return err instanceof TypeError || err instanceof Error;
}

function readAll(): OfflineAction[] {
  try {
    const raw = localStorage.getItem(QUEUE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      localStorage.removeItem(QUEUE_KEY);
      return [];
    }
    return parsed.filter(
      (item): item is OfflineAction =>
        Boolean(item) &&
        typeof item.id === 'string' &&
        typeof item.userId === 'number' &&
        typeof item.rideId === 'number' &&
        typeof item.path === 'string' &&
        typeof item.label === 'string' &&
        typeof item.createdAt === 'string',
    );
  } catch {
    return [];
  }
}

function emitQueueChange(): void {
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new Event(OFFLINE_QUEUE_EVENT));
  }
}

function writeAll(items: OfflineAction[]): void {
  try {
    if (items.length === 0) {
      localStorage.removeItem(QUEUE_KEY);
    } else {
      localStorage.setItem(QUEUE_KEY, JSON.stringify(items));
    }
    emitQueueChange();
  } catch {
    // storage unavailable
  }
}

export function getOfflineQueue(userId?: number): OfflineAction[] {
  const all = readAll();
  return userId === undefined ? all : all.filter((item) => item.userId === userId);
}

export function getQueuedActionsForRide(userId: number, rideId: number): OfflineAction[] {
  return readAll().filter((item) => item.userId === userId && item.rideId === rideId);
}

export function enqueueOfflineAction(
  item: Omit<OfflineAction, 'id' | 'createdAt'>,
): OfflineAction {
  const created: OfflineAction = {
    ...item,
    id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    createdAt: new Date().toISOString(),
  };
  const next = readAll().filter(
    (existing) =>
      !(
        existing.userId === item.userId &&
        existing.rideId === item.rideId &&
        existing.path === item.path
      ),
  );
  next.push(created);
  writeAll(next);
  return created;
}

export function clearOfflineQueue(): void {
  try {
    localStorage.removeItem(QUEUE_KEY);
    emitQueueChange();
  } catch {
    // storage unavailable
  }
}

export async function postOrQueue(opts: {
  userId: number;
  rideId: number;
  path: string;
  body?: unknown;
  label: string;
}): Promise<{ queued: boolean }> {
  if (typeof navigator !== 'undefined' && navigator.onLine === false) {
    enqueueOfflineAction({
      userId: opts.userId,
      rideId: opts.rideId,
      path: opts.path,
      body: opts.body,
      label: opts.label,
    });
    return { queued: true };
  }
  try {
    if (opts.body !== undefined) {
      await post(opts.path, opts.body);
    } else {
      await post(opts.path);
    }
    return { queued: false };
  } catch (err) {
    if (isOfflineNetworkError(err)) {
      enqueueOfflineAction({
        userId: opts.userId,
        rideId: opts.rideId,
        path: opts.path,
        body: opts.body,
        label: opts.label,
      });
      return { queued: true };
    }
    throw err;
  }
}

export interface FlushResult {
  synced: number;
  conflicts: number;
  remaining: number;
}

let activeFlush: Promise<FlushResult> | null = null;

export function flushOfflineQueue(userId: number): Promise<FlushResult> {
  if (activeFlush) return activeFlush;
  activeFlush = (async (): Promise<FlushResult> => {
    const queue = readAll();
    const mine = queue.filter((item) => item.userId === userId);
    if (mine.length === 0) {
      return { synced: 0, conflicts: 0, remaining: 0 };
    }

    let synced = 0;
    let conflicts = 0;
    const droppedIds = new Set<string>();

    for (const item of mine) {
      if (typeof navigator !== 'undefined' && navigator.onLine === false) {
        break;
      }
      try {
        if (item.body !== undefined) {
          await post(item.path, item.body);
        } else {
          await post(item.path);
        }
        droppedIds.add(item.id);
        synced += 1;
      } catch (err) {
        if (isApiError(err)) {
          if (err.status === 401 || err.status === 423) {
            break;
          }
          if ([400, 403, 404, 409, 422].includes(err.status)) {
            droppedIds.add(item.id);
            conflicts += 1;
            continue;
          }
          break;
        }
        break;
      }
    }

    if (droppedIds.size > 0) {
      const remainingAll = readAll().filter((item) => !droppedIds.has(item.id));
      writeAll(remainingAll);
    }
    const remaining = readAll().filter((item) => item.userId === userId).length;
    return { synced, conflicts, remaining };
  })().finally(() => {
    activeFlush = null;
  });

  return activeFlush;
}

/** Re-renders subscribing components when queued offline actions change. */
export function useOfflineQueueTick(): number {
  const [tick, setTick] = useState(0);
  useEffect(() => {
    const onChange = () => setTick((t) => t + 1);
    window.addEventListener(OFFLINE_QUEUE_EVENT, onChange);
    return () => window.removeEventListener(OFFLINE_QUEUE_EVENT, onChange);
  }, []);
  return tick;
}
