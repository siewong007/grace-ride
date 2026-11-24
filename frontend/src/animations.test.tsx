import fs from 'node:fs';
import path from 'node:path';
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import i18n from './i18n';
import { ToastProvider, useToast } from './components/ui';
import Layout from './components/Layout';
import { BackNavigationProvider } from './components/BackNavigation';
import { QrCameraScanner } from './components/event/QrCameraScanner';
import { AutoLockProvider, clearAutoLock, useAutoLock } from './components/AutoLock';

const mocks = vi.hoisted(() => ({
  post: vi.fn().mockResolvedValue({ ok: true }),
  reauth: vi.fn().mockRejectedValue(new Error('Passkey failed')),
}));

vi.mock('./api', () => ({
  api: vi.fn(),
  post: mocks.post,
  subscribe: () => () => {},
  useApi: () => ({ data: null, loading: false, error: null, reload: vi.fn() }),
}));

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: {
      id: 1,
      name: 'Grace Member',
      email: 'grace@example.com',
      phone: '0123456789',
      photo_file_id: null,
      roles: ['member'],
      groups: [{ id: 10, name: 'Youth' }],
      status: 'active',
      terms_accepted: true,
      privileged: false,
      mfa_ok: true,
      unread: 0,
      passkey_count: 1,
      auto_lock_mins: 15,
      session_timeout_mins: 15,
    },
    can: (page: string) => ['events', 'rides', 'costs', 'reimbursements', 'help', 'report'].includes(page),
    refresh: vi.fn(),
  }),
}));

vi.mock('./webauthn', () => ({
  passkeySupported: () => true,
  reauthWithPasskey: () => mocks.reauth(),
}));

const CSS_ANIM_KEYWORDS = new Set([
  'none',
  'inherit',
  'initial',
  'unset',
  'revert',
  'revert-layer',
  'infinite',
  'linear',
  'ease',
  'ease-in',
  'ease-out',
  'ease-in-out',
  'step-start',
  'step-end',
  'both',
  'forwards',
  'backwards',
  'normal',
  'reverse',
  'alternate',
  'alternate-reverse',
  'running',
  'paused',
]);

function extractAnimationNames(css: string): string[] {
  const withoutComments = css.replace(/\/\*[\s\S]*?\*\//g, '');
  const names: string[] = [];
  const declRe = /animation(?:-name)?\s*:\s*([^;}]+)/g;
  let match: RegExpExecArray | null;

  while ((match = declRe.exec(withoutComments)) !== null) {
    const rawValue = match[1].replace(/!important/g, '').trim();
    let stripped = rawValue;
    while (/[a-z-]*\([^()]*\)/i.test(stripped)) {
      stripped = stripped.replace(/[a-z-]*\([^()]*\)/gi, ' ');
    }
    const parts = stripped.split(',');
    for (const part of parts) {
      const tokens = part.trim().split(/\s+/);
      for (const tok of tokens) {
        if (!tok) continue;
        if (CSS_ANIM_KEYWORDS.has(tok)) continue;
        if (/^-?(\d+\.?\d*|\.\d+)(ms|s)?$/i.test(tok)) continue;
        names.push(tok);
      }
    }
  }
  return names;
}

function mockMatchMedia({ mobile = false, reducedMotion = false } = {}) {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value: (query: string) => ({
      matches: query.includes('max-width: 767px')
        ? mobile
        : query.includes('prefers-reduced-motion: reduce')
          ? reducedMotion
          : false,
      media: query,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
      dispatchEvent: () => false,
      onchange: null,
    }),
  });
}

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  clearAutoLock();
  localStorage.clear();
});

describe('CSS animation & keyframe integrity', () => {
  it('defines @keyframes fadeIn and every referenced animation name in styles.css', () => {
    const css = fs.readFileSync(path.resolve(__dirname, 'styles.css'), 'utf-8');
    const definedKeyframes = new Set(
      Array.from(css.matchAll(/@keyframes\s+([a-zA-Z0-9_-]+)/g), (m) => m[1]),
    );

    expect(definedKeyframes.has('fadeIn')).toBe(true);

    const referenced = extractAnimationNames(css);
    expect(referenced.length).toBeGreaterThan(0);

    const missing = Array.from(new Set(referenced)).filter((name) => !definedKeyframes.has(name));
    expect(missing).toEqual([]);
  });
});

describe('Toast enter/exit and progress bar', () => {
  function ToastTester() {
    const toast = useToast();
    return (
      <button type="button" onClick={() => toast.success('Saved!')}>
        Show toast
      </button>
    );
  }

  beforeEach(() => {
    vi.useFakeTimers();
  });

  it('renders progress bar and marks .is-leaving before removing toast on close', () => {
    mockMatchMedia({ reducedMotion: false });
    render(
      <ToastProvider>
        <ToastTester />
      </ToastProvider>,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Show toast' }));
    const toastEl = screen.getByRole('status');
    expect(toastEl).toBeInTheDocument();
    expect(toastEl.querySelector('.toast-progress')).toBeInTheDocument();
    expect(toastEl).not.toHaveClass('is-leaving');

    fireEvent.click(screen.getByRole('button', { name: 'Close' }));
    expect(screen.getByRole('status')).toHaveClass('is-leaving');

    act(() => {
      vi.advanceTimersByTime(160);
    });
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });

  it('removes toast immediately on close when prefers-reduced-motion: reduce matches', () => {
    mockMatchMedia({ reducedMotion: true });
    render(
      <ToastProvider>
        <ToastTester />
      </ToastProvider>,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Show toast' }));
    expect(screen.getByRole('status')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Close' }));
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });
});

describe('Mobile SwipeTabs direction indicator on Layout', () => {
  it('sets data-nav-dir on <main> matching swipe direction', () => {
    vi.useFakeTimers();
    mockMatchMedia({ mobile: true, reducedMotion: false });
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 390 });

    render(
      <MemoryRouter initialEntries={['/']}>
        <BackNavigationProvider>
          <Routes>
            <Route element={<Layout />}>
              <Route path="/" element={<div>Home page</div>} />
              <Route path="/events" element={<div>Events page</div>} />
            </Route>
          </Routes>
        </BackNavigationProvider>
      </MemoryRouter>,
    );

    const main = screen.getByRole('main');
    fireEvent.touchStart(main, { touches: [{ clientX: 200, clientY: 200 }] });
    fireEvent.touchMove(main, { touches: [{ clientX: 80, clientY: 200 }] });
    fireEvent.touchEnd(main, { changedTouches: [{ clientX: 80, clientY: 200 }] });

    expect(screen.getByText('Events page')).toBeInTheDocument();
    expect(main).toHaveAttribute('data-nav-dir', 'left');

    act(() => {
      vi.advanceTimersByTime(300);
    });
    expect(main).not.toHaveAttribute('data-nav-dir');
  });
});

describe('QrCameraScanner scan sweep', () => {
  it('renders .qr-scan-line when active and hides it when inactive', async () => {
    const stop = vi.fn();
    Object.defineProperty(navigator, 'mediaDevices', {
      configurable: true,
      value: {
        getUserMedia: vi.fn().mockResolvedValue({
          getTracks: () => [{ stop }],
        }),
      },
    });
    vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue(undefined);

    const { container, rerender } = render(<QrCameraScanner active onScan={vi.fn()} />);
    expect(container.querySelector('.qr-scan-line')).toBeInTheDocument();

    rerender(<QrCameraScanner active={false} onScan={vi.fn()} />);
    await waitFor(() => {
      expect(container.querySelector('.qr-scan-line')).not.toBeInTheDocument();
    });
  });
});

describe('AutoLock failure shake animation', () => {
  function LockTrigger() {
    const { lock } = useAutoLock();
    return (
      <button type="button" onClick={lock}>
        Lock session
      </button>
    );
  }

  it('adds .is-shaking to .lock-card when unlock fails', async () => {
    mockMatchMedia({ reducedMotion: false });
    mocks.reauth.mockRejectedValueOnce(new Error('Passkey cancelled'));
    vi.spyOn(window, 'requestAnimationFrame').mockImplementation((cb) => {
      cb(0);
      return 1;
    });

    const { container } = render(
      <MemoryRouter>
        <AutoLockProvider>
          <LockTrigger />
        </AutoLockProvider>
      </MemoryRouter>,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Lock session' }));
    const unlockBtn = screen.getByRole('button', { name: /^unlock$/i });
    fireEvent.click(unlockBtn);

    await waitFor(() => {
      expect(container.querySelector('.lock-card')).toHaveClass('is-shaking');
    });
  });
});
