import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiError, post } from './api';
import {
  clearOfflineQueue,
  flushOfflineQueue,
  getOfflineQueue,
  getQueuedActionsForRide,
  postOrQueue,
} from './offlineQueue';
import { clearOfflineSnapshots } from './offlineSnapshot';

vi.mock('./api', async () => {
  const actual = await vi.importActual<typeof import('./api')>('./api');
  return {
    ...actual,
    post: vi.fn(),
  };
});

describe('offlineQueue', () => {
  beforeEach(() => {
    localStorage.clear();
    clearOfflineQueue();
    vi.clearAllMocks();
    vi.mocked(post).mockReset();
    Object.defineProperty(navigator, 'onLine', { value: true, configurable: true });
  });

  it('queues immediately when browser is offline without hitting post', async () => {
    Object.defineProperty(navigator, 'onLine', { value: false, configurable: true });
    const res = await postOrQueue({
      userId: 7,
      rideId: 42,
      path: '/rides/42/pickup',
      label: 'Picked up',
    });

    expect(res).toEqual({ queued: true });
    expect(post).not.toHaveBeenCalled();
    expect(getQueuedActionsForRide(7, 42)).toHaveLength(1);
  });

  it('queues on network TypeError and deduplicates repeated (userId, rideId, path)', async () => {
    vi.mocked(post).mockRejectedValue(new TypeError('Failed to fetch'));

    await postOrQueue({
      userId: 7,
      rideId: 42,
      path: '/rides/42/driver-delay',
      body: { delay_minutes: 5 },
      label: '+5 min',
    });
    await postOrQueue({
      userId: 7,
      rideId: 42,
      path: '/rides/42/driver-delay',
      body: { delay_minutes: 10 },
      label: '+10 min',
    });

    const q = getOfflineQueue(7);
    expect(q).toHaveLength(1);
    expect(q[0].body).toEqual({ delay_minutes: 10 });
    expect(q[0].label).toBe('+10 min');
  });

  it('does not queue structured HTTP ApiError rejections (4xx/5xx)', async () => {
    vi.mocked(post).mockRejectedValueOnce(new ApiError(409, 'Ride changed', 'RIDE_STATUS_CHANGED'));

    await expect(
      postOrQueue({
        userId: 7,
        rideId: 42,
        path: '/rides/42/pickup',
        label: 'Picked up',
      }),
    ).rejects.toThrow('Ride changed');

    expect(getOfflineQueue(7)).toHaveLength(0);
  });

  it('flushes FIFO, drops 409 conflicts, and stops cleanly on mid-flush network loss', async () => {
    Object.defineProperty(navigator, 'onLine', { value: false, configurable: true });
    await postOrQueue({ userId: 7, rideId: 10, path: '/rides/10/arrived-gate', label: 'At gate' });
    await postOrQueue({ userId: 7, rideId: 10, path: '/rides/10/pickup', label: 'Picked up' });
    await postOrQueue({ userId: 7, rideId: 10, path: '/rides/10/complete', label: 'Completed' });

    Object.defineProperty(navigator, 'onLine', { value: true, configurable: true });
    vi.mocked(post)
      .mockResolvedValueOnce({ ok: true })
      .mockRejectedValueOnce(new ApiError(409, 'Stale', 'RIDE_STATUS_CHANGED'))
      .mockRejectedValueOnce(new TypeError('Network dropped'));

    const firstFlush = await flushOfflineQueue(7);
    expect(firstFlush).toEqual({ synced: 1, conflicts: 1, remaining: 1 });
    expect(getOfflineQueue(7).map((i) => i.path)).toEqual(['/rides/10/complete']);

    vi.mocked(post).mockResolvedValueOnce({ ok: true });
    const secondFlush = await flushOfflineQueue(7);
    expect(secondFlush).toEqual({ synced: 1, conflicts: 0, remaining: 0 });
    expect(getOfflineQueue(7)).toHaveLength(0);
  });

  it('wipes offline action queue when clearOfflineSnapshots is called for privacy', async () => {
    Object.defineProperty(navigator, 'onLine', { value: false, configurable: true });
    await postOrQueue({ userId: 7, rideId: 99, path: '/rides/99/pickup', label: 'Picked up' });
    expect(getOfflineQueue(7)).toHaveLength(1);

    clearOfflineSnapshots();
    expect(getOfflineQueue(7)).toHaveLength(0);
  });
});
