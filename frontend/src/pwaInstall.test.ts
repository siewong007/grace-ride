import { describe, expect, it, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import {
  usePwaInstall,
  promptInstall,
  _setDeferredPromptForTesting,
  _setInstalledForTesting,
  BeforeInstallPromptEvent,
  detectPlatform,
  detectSafari,
  detectInApp,
} from './pwaInstall';

describe('pwaInstall', () => {
  beforeEach(() => {
    _setDeferredPromptForTesting(null);
    _setInstalledForTesting(null);
  });

  it('reports initial state accurately', () => {
    const { result } = renderHook(() => usePwaInstall());
    expect(result.current.canPrompt).toBe(false);
    expect(result.current.isInstalled).toBe(false);
  });

  it('updates state when deferred prompt is captured', () => {
    const { result } = renderHook(() => usePwaInstall());
    expect(result.current.canPrompt).toBe(false);

    const mockPromptEvent = {
      platforms: ['web'],
      userChoice: Promise.resolve({ outcome: 'accepted' as const, platform: 'web' }),
      prompt: vi.fn().mockResolvedValue(undefined),
    } as unknown as BeforeInstallPromptEvent;

    act(() => {
      _setDeferredPromptForTesting(mockPromptEvent);
    });

    expect(result.current.canPrompt).toBe(true);
  });

  it('handles promptInstall call and updates installed state upon acceptance', async () => {
    const promptMock = vi.fn().mockResolvedValue(undefined);
    const mockPromptEvent = {
      platforms: ['web'],
      userChoice: Promise.resolve({ outcome: 'accepted' as const, platform: 'web' }),
      prompt: promptMock,
    } as unknown as BeforeInstallPromptEvent;

    _setDeferredPromptForTesting(mockPromptEvent);

    let outcome: string | null = null;
    await act(async () => {
      outcome = await promptInstall();
    });

    expect(outcome).toBe('accepted');
    expect(promptMock).toHaveBeenCalled();
  });

  it('returns null when promptInstall is called without a deferred prompt', async () => {
    _setDeferredPromptForTesting(null);
    const outcome = await promptInstall();
    expect(outcome).toBeNull();
  });

  describe('device detection helpers', () => {
    const originalUA = navigator.userAgent;

    afterEach(() => {
      Object.defineProperty(navigator, 'userAgent', { value: originalUA, configurable: true });
    });

    it('detects iOS devices', () => {
      Object.defineProperty(navigator, 'userAgent', {
        value: 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15',
        configurable: true,
      });
      expect(detectPlatform()).toBe('ios');
    });

    it('detects Android devices', () => {
      Object.defineProperty(navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 Chrome/120.0 Mobile Safari/537.36',
        configurable: true,
      });
      expect(detectPlatform()).toBe('android');
    });

    it('detects In-App browsers (e.g. WeChat / WhatsApp)', () => {
      Object.defineProperty(navigator, 'userAgent', {
        value: 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0) Mobile/15E148 MicroMessenger/8.0.38',
        configurable: true,
      });
      expect(detectInApp()).toBe(true);
    });

    it('detects genuine Safari on iOS', () => {
      Object.defineProperty(navigator, 'userAgent', {
        value: 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1',
        configurable: true,
      });
      expect(detectSafari()).toBe(true);
    });
  });
});
