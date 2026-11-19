// PWA installation state manager and hook for Grace Ride.
// Supports 1-click install via beforeinstallprompt on Chromium browsers
// and platform detection / step guidance on iOS Safari and in-app webviews.

import { useEffect, useState } from 'react';

export interface BeforeInstallPromptEvent extends Event {
  readonly platforms: string[];
  readonly userChoice: Promise<{
    outcome: 'accepted' | 'dismissed';
    platform: string;
  }>;
  prompt(): Promise<void>;
}

export type PwaPlatform = 'ios' | 'android' | 'other';

export interface PwaInstallState {
  isInstalled: boolean;
  canPrompt: boolean;
  platform: PwaPlatform;
  isSafari: boolean;
  isInApp: boolean;
}

let deferredPrompt: BeforeInstallPromptEvent | null = null;
let isInstalledOverride: boolean | null = null;
const listeners = new Set<() => void>();

function notify() {
  for (const listener of listeners) {
    listener();
  }
}

export function isStandalone(): boolean {
  if (isInstalledOverride !== null) return isInstalledOverride;
  if (typeof window === 'undefined') return false;
  const nav = typeof navigator !== 'undefined' ? (navigator as unknown as { standalone?: boolean }) : undefined;
  return Boolean(
    window.matchMedia?.('(display-mode: standalone)').matches ||
    nav?.standalone ||
    (typeof document !== 'undefined' && document.referrer?.includes('android-app://'))
  );
}

export function detectPlatform(): PwaPlatform {
  if (typeof navigator === 'undefined') return 'other';
  const ua = navigator.userAgent;
  const isIos =
    /iPad|iPhone|iPod/.test(ua) ||
    (navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1);
  if (isIos) return 'ios';
  if (/Android/.test(ua)) return 'android';
  return 'other';
}

export function detectSafari(): boolean {
  if (typeof navigator === 'undefined') return false;
  const ua = navigator.userAgent;
  const isIos =
    /iPad|iPhone|iPod/.test(ua) ||
    (navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1);
  return isIos && /Safari/i.test(ua) && !/CriOS|FxiOS|OPiOS|mercury/i.test(ua);
}

export function detectInApp(): boolean {
  if (typeof navigator === 'undefined') return false;
  const ua = navigator.userAgent;
  return /MicroMessenger|Line|FBAV|Instagram|FBAN|WhatsApp/i.test(ua);
}

if (typeof window !== 'undefined') {
  window.addEventListener('beforeinstallprompt', (e) => {
    e.preventDefault();
    deferredPrompt = e as BeforeInstallPromptEvent;
    notify();
  });

  window.addEventListener('appinstalled', () => {
    isInstalledOverride = true;
    deferredPrompt = null;
    notify();
  });
}

export async function promptInstall(): Promise<'accepted' | 'dismissed' | null> {
  if (!deferredPrompt) return null;
  try {
    await deferredPrompt.prompt();
    const choice = await deferredPrompt.userChoice;
    if (choice.outcome === 'accepted') {
      isInstalledOverride = true;
      deferredPrompt = null;
    }
    notify();
    return choice.outcome;
  } catch (err) {
    console.error('Failed to prompt PWA install:', err);
    return null;
  }
}

export function usePwaInstall() {
  const [state, setState] = useState<PwaInstallState>(() => ({
    isInstalled: isStandalone(),
    canPrompt: Boolean(deferredPrompt),
    platform: detectPlatform(),
    isSafari: detectSafari(),
    isInApp: detectInApp(),
  }));

  useEffect(() => {
    const update = () => {
      setState({
        isInstalled: isStandalone(),
        canPrompt: Boolean(deferredPrompt),
        platform: detectPlatform(),
        isSafari: detectSafari(),
        isInApp: detectInApp(),
      });
    };
    listeners.add(update);
    const mql = window.matchMedia?.('(display-mode: standalone)');
    mql?.addEventListener?.('change', update);
    return () => {
      listeners.delete(update);
      mql?.removeEventListener?.('change', update);
    };
  }, []);

  return {
    ...state,
    promptInstall,
  };
}

// Test / debugging utilities
export function _setDeferredPromptForTesting(prompt: BeforeInstallPromptEvent | null) {
  deferredPrompt = prompt;
  notify();
}

export function _setInstalledForTesting(installed: boolean | null) {
  isInstalledOverride = installed;
  notify();
}
