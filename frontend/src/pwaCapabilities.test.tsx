import { act, fireEvent, render, renderHook, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useWakeLock } from './hooks/useWakeLock';
import { canNativeShare, shareOrFallback } from './webShare';
import {
  __resetUpdateStateForTests,
  checkForNewBuild,
  isUpdateAvailable,
  useUpdateAvailable,
} from './appUpdate';
import { UpdateBanner } from './components/AppChrome';

function setNavigator(key: string, value: unknown) {
  Object.defineProperty(navigator, key, { value, configurable: true, writable: true });
}

describe('useWakeLock', () => {
  let release: ReturnType<typeof vi.fn>;
  let request: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    release = vi.fn().mockResolvedValue(undefined);
    request = vi.fn().mockImplementation(async () => ({ released: false, release }));
    setNavigator('wakeLock', { request });
  });
  afterEach(() => {
    delete (navigator as unknown as Record<string, unknown>).wakeLock;
  });

  it('requests a screen lock while active and releases it on unmount', async () => {
    const { unmount } = renderHook(() => useWakeLock(true));
    await act(async () => {});
    expect(request).toHaveBeenCalledWith('screen');
    unmount();
    expect(release).toHaveBeenCalled();
  });

  it('does nothing while inactive', async () => {
    renderHook(() => useWakeLock(false));
    await act(async () => {});
    expect(request).not.toHaveBeenCalled();
  });

  it('is a no-op when the API is missing', async () => {
    delete (navigator as unknown as Record<string, unknown>).wakeLock;
    expect(() => renderHook(() => useWakeLock(true))).not.toThrow();
  });
});

describe('shareOrFallback', () => {
  afterEach(() => {
    delete (navigator as unknown as Record<string, unknown>).share;
    delete (navigator as unknown as Record<string, unknown>).canShare;
  });

  it('uses the native share sheet when available', async () => {
    const share = vi.fn().mockResolvedValue(undefined);
    setNavigator('share', share);
    const fallback = vi.fn();
    expect(canNativeShare()).toBe(true);
    await expect(shareOrFallback({ url: 'https://x.test/a' }, fallback)).resolves.toBe('shared');
    expect(share).toHaveBeenCalledWith({ url: 'https://x.test/a' });
    expect(fallback).not.toHaveBeenCalled();
  });

  it('treats a dismissed sheet as cancelled, not a failure', async () => {
    setNavigator('share', vi.fn().mockRejectedValue(new DOMException('no', 'AbortError')));
    const fallback = vi.fn();
    await expect(shareOrFallback({ text: 'hi' }, fallback)).resolves.toBe('cancelled');
    expect(fallback).not.toHaveBeenCalled();
  });

  it('falls back when sharing is unsupported or refused', async () => {
    const fallback = vi.fn();
    expect(canNativeShare()).toBe(false);
    await expect(shareOrFallback({ text: 'hi' }, fallback)).resolves.toBe('fallback');
    setNavigator('share', vi.fn().mockRejectedValue(new DOMException('no', 'NotAllowedError')));
    await expect(shareOrFallback({ text: 'hi' }, fallback)).resolves.toBe('fallback');
    expect(fallback).toHaveBeenCalledTimes(2);
  });
});

describe('app update detection', () => {
  beforeEach(() => __resetUpdateStateForTests());
  afterEach(() => vi.unstubAllGlobals());

  it('flags an update when the deployed build id differs', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, json: async () => ({ build: 'new' }) }));
    const { result } = renderHook(() => useUpdateAvailable());
    expect(result.current).toBe(false);
    await act(async () => {
      expect(await checkForNewBuild('old')).toBe(true);
    });
    expect(result.current).toBe(true);
  });

  it('stays quiet for the same build, errors, or an HTML fallback', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, json: async () => ({ build: 'same' }) }));
    expect(await checkForNewBuild('same')).toBe(false);
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('offline')));
    expect(await checkForNewBuild('same')).toBe(false);
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, json: async () => { throw new SyntaxError('html'); } }));
    expect(await checkForNewBuild('same')).toBe(false);
    expect(isUpdateAvailable()).toBe(false);
  });

  it('shows a dismissible banner once an update is available', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, json: async () => ({ build: 'new' }) }));
    render(<UpdateBanner />);
    expect(screen.queryByRole('status')).toBeNull();
    await act(async () => {
      await checkForNewBuild('old');
    });
    expect(screen.getByRole('status')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /later/i }));
    expect(screen.queryByRole('status')).toBeNull();
  });
});
