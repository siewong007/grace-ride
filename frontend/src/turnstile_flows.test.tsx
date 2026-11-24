import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import i18n from './i18n';
import { ConfirmProvider } from './components/ui';
import Login from './pages/Login';
import Invite from './pages/Invite';
import Visit from './pages/Visit';
import { resetTurnstileForTests, type TurnstileApi, type TurnstileRenderOptions } from './turnstile';

const mockWebauthn = vi.hoisted(() => ({
  loginWithPasskey: vi.fn(),
  registerPasskey: vi.fn(),
  passkeySupported: vi.fn(() => true),
  getDevicePasskey: vi.fn(() => null),
  clearDevicePasskey: vi.fn(),
  saveDevicePasskey: vi.fn(),
}));

const mockApi = vi.hoisted(() => ({
  api: vi.fn().mockResolvedValue([]),
  post: vi.fn(),
  useApi: vi.fn(),
}));

vi.mock('./auth', () => ({
  useAuth: () => ({ me: null, loading: false, refresh: vi.fn() }),
}));

vi.mock('./webauthn', () => mockWebauthn);

vi.mock('./api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./api')>()),
  api: (...args: unknown[]) => mockApi.api(...args),
  post: (...args: unknown[]) => mockApi.post(...args),
  useApi: (...args: unknown[]) => mockApi.useApi(...args),
}));

describe('Turnstile enforcement across public web flows', () => {
  const callbacksByAction = new Map<string, TurnstileRenderOptions>();
  let resetMock: ReturnType<typeof vi.fn>;
  let seq = 0;

  beforeAll(async () => {
    await i18n.changeLanguage('en');
  });

  beforeEach(() => {
    vi.clearAllMocks();
    resetTurnstileForTests();
    callbacksByAction.clear();
    seq = 0;
    resetMock = vi.fn();

    vi.stubGlobal(
      'fetch',
      vi.fn(async (input: RequestInfo | URL) => {
        const url = typeof input === 'string' ? input : input.toString();
        if (url.endsWith('/api/auth/turnstile')) {
          return new Response(
            JSON.stringify({ enabled: true, site_key: '1x00000000000000000000AA' }),
            { status: 200, headers: { 'content-type': 'application/json' } },
          );
        }
        return new Response('{}', { status: 404 });
      }),
    );

    window.turnstile = {
      render: vi.fn((_el, opts: TurnstileRenderOptions) => {
        if (opts.action) {
          callbacksByAction.set(opts.action, opts);
        }
        return `w-${++seq}`;
      }),
      reset: resetMock,
      remove: vi.fn(),
      getResponse: vi.fn(),
    } as TurnstileApi;
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    delete window.turnstile;
  });

  it('gates passkey login and resets Turnstile on passkey error', async () => {
    mockWebauthn.loginWithPasskey.mockRejectedValueOnce(new Error('Verification failed'));

    render(
      <MemoryRouter>
        <Login />
      </MemoryRouter>,
    );

    await waitFor(() => expect(callbacksByAction.has('passkey_login')).toBe(true));

    const submitBtn = screen.getByRole('button', { name: /log in with passkey/i });
    expect(submitBtn).toBeDisabled();

    fireEvent.change(screen.getByLabelText(/email/i), { target: { value: 'member@example.org' } });
    act(() => callbacksByAction.get('passkey_login')?.callback?.('tok-login'));
    expect(submitBtn).toBeEnabled();

    fireEvent.click(submitBtn);
    await waitFor(() =>
      expect(mockWebauthn.loginWithPasskey).toHaveBeenCalledWith('member@example.org', 'tok-login'),
    );
    await waitFor(() => expect(resetMock).toHaveBeenCalled());
  });

  it('gates account recovery modal submission and includes turnstile_token', async () => {
    mockApi.post.mockResolvedValueOnce({ ok: true });

    render(
      <MemoryRouter>
        <Login />
      </MemoryRouter>,
    );

    fireEvent.click(screen.getByRole('button', { name: /email recovery/i }));
    const dialog = screen.getByRole('dialog');
    await waitFor(() => expect(callbacksByAction.has('recovery_request')).toBe(true));

    const submitBtn = screen.getByRole('button', { name: /send recovery request/i });
    expect(submitBtn).toBeDisabled();

    const emailInput = dialog.querySelector('input[type="email"]') as HTMLInputElement;
    fireEvent.change(emailInput, { target: { value: 'elder@example.org' } });

    act(() => callbacksByAction.get('recovery_request')?.callback?.('tok-rec'));
    expect(submitBtn).toBeEnabled();

    fireEvent.click(submitBtn);
    await waitFor(() =>
      expect(mockApi.post).toHaveBeenCalledWith('/auth/recovery-request', {
        email: 'elder@example.org',
        reason: '',
        turnstile_token: 'tok-rec',
      }),
    );
  });

  it('gates invite acceptance on step 2 and resets on error', async () => {
    mockApi.useApi.mockReturnValue({
      data: { kind: 'member', email: '', name: '' },
      loading: false,
      error: null,
    });
    mockApi.post.mockRejectedValueOnce(new Error('Verification expired'));

    render(
      <ConfirmProvider>
        <MemoryRouter initialEntries={['/invite/invtok']}>
          <Routes>
            <Route path="/invite/:token" element={<Invite />} />
          </Routes>
        </MemoryRouter>
      </ConfirmProvider>,
    );

    fireEvent.change(screen.getByLabelText(/name/i), { target: { value: 'New Member' } });
    fireEvent.change(screen.getByLabelText(/email/i), { target: { value: 'new@example.com' } });
    fireEvent.change(screen.getByLabelText(/phone/i), { target: { value: '0123456789' } });
    fireEvent.click(screen.getByRole('button', { name: /continue/i }));

    await waitFor(() => expect(callbacksByAction.has('invite_accept')).toBe(true));

    fireEvent.click(screen.getByRole('checkbox'));
    const agreeBtn = screen.getByRole('button', { name: /agree/i });
    expect(agreeBtn).toBeDisabled();

    act(() => callbacksByAction.get('invite_accept')?.callback?.('tok-inv'));
    expect(agreeBtn).toBeEnabled();

    fireEvent.click(agreeBtn);
    await waitFor(() =>
      expect(mockApi.post).toHaveBeenCalledWith('/invites/invtok/accept', {
        name: 'New Member',
        email: 'new@example.com',
        phone: '0123456789',
        terms: true,
        turnstile_token: 'tok-inv',
      }),
    );
    await waitFor(() => expect(resetMock).toHaveBeenCalled());
  });

  it('gates visitor claim and resets on failure', async () => {
    mockApi.useApi.mockReturnValue({
      data: {
        state: 'open',
        closed_reason: null,
        opens_at: '2026-11-21T09:00',
        name: 'Ann',
        host: 'Mary',
        event: { title: 'Youth camp', starts_at: '2026-11-22T10:00', location: 'Church hall' },
      },
      loading: false,
      error: null,
      reload: vi.fn(),
    });
    mockApi.post.mockRejectedValueOnce(new Error('Turnstile failed'));

    render(
      <MemoryRouter initialEntries={['/visit/vistok']}>
        <Routes>
          <Route path="/visit/:token" element={<Visit />} />
        </Routes>
      </MemoryRouter>,
    );

    await waitFor(() => expect(callbacksByAction.has('visitor_claim')).toBe(true));

    const openBtn = screen.getByRole('button', { name: /open my ride/i });
    expect(openBtn).toBeDisabled();

    act(() => callbacksByAction.get('visitor_claim')?.callback?.('tok-vis'));
    expect(openBtn).toBeEnabled();

    fireEvent.click(openBtn);
    await waitFor(() =>
      expect(mockApi.post).toHaveBeenCalledWith('/visit/vistok/claim', {
        turnstile_token: 'tok-vis',
      }),
    );
    await waitFor(() => expect(resetMock).toHaveBeenCalled());
  });
});
