import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import i18n from './i18n';
import { ConfirmProvider } from './components/ui';
import Login from './pages/Login';
import Invite from './pages/Invite';
import Profile from './pages/Profile';
import type { Me } from './auth';

const mockApi = vi.hoisted(() => ({
  api: vi.fn(),
  post: vi.fn(),
  put: vi.fn(),
  del: vi.fn(),
  useApi: vi.fn(),
}));

const mockAuthState = vi.hoisted(() => ({
  me: null as Me | null,
  refresh: vi.fn(),
  clearLocalAuth: vi.fn(),
  can: () => true,
}));

const mockWebauthn = vi.hoisted(() => ({
  loginWithPasskey: vi.fn(),
  registerPasskey: vi.fn(),
  passkeySupported: vi.fn(() => true),
  getDevicePasskey: vi.fn(() => null),
  clearDevicePasskey: vi.fn(),
  withRecentSignIn: vi.fn((fn: () => Promise<unknown>) => fn()),
}));

vi.mock('./auth', () => ({
  useAuth: () => mockAuthState,
}));

vi.mock('./webauthn', () => mockWebauthn);

vi.mock('./api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./api')>()),
  api: (...args: unknown[]) => mockApi.api(...args),
  post: (...args: unknown[]) => mockApi.post(...args),
  put: (...args: unknown[]) => mockApi.put(...args),
  del: (...args: unknown[]) => mockApi.del(...args),
  useApi: (...args: unknown[]) => mockApi.useApi(...args),
}));

vi.mock('./turnstile', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./turnstile')>()),
  useTurnstile: () => ({
    enabled: false,
    siteKey: null,
    token: null,
    resetKey: 0,
    setToken: vi.fn(),
    reset: vi.fn(),
  }),
}));

const baseMe: Me = {
  id: 10,
  name: 'Grace Member',
  email: 'grace@example.org',
  phone: '0123456789',
  photo_file_id: null,
  roles: ['member'],
  role_names: {},
  permissions: ['rides.view'],
  status: 'active',
  terms_accepted: true,
  mfa_ok: false,
  totp_enabled: false,
  has_password: false,
  passkey_count: 0,
  google_verified: false,
  google_email: null,
  badges_opt_out: false,
  privileged: false,
  unread: 0,
  groups: [],
  dev_mode: false,
};

describe('Google Sign-In, Password Login & Forgot Password UI', () => {
  beforeAll(async () => {
    await i18n.changeLanguage('en');
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mockAuthState.me = null;
    mockApi.api.mockImplementation(async (path: string) => {
      if (path === '/auth/google/config') return { enabled: true };
      if (path === '/auth/dev-users') return [];
      return [];
    });
    mockApi.post.mockResolvedValue({ ok: true });
    mockApi.del.mockResolvedValue({ ok: true });
    mockApi.useApi.mockReturnValue({ data: [], loading: false, error: null, reload: vi.fn() });
  });

  it('shows Google Sign-In button when enabled and hides it when disabled', async () => {
    render(
      <MemoryRouter initialEntries={['/login']}>
        <Login />
      </MemoryRouter>,
    );
    expect(await screen.findByRole('button', { name: /sign in with google/i })).toBeInTheDocument();

    mockApi.api.mockImplementation(async (path: string) => {
      if (path === '/auth/google/config') return { enabled: false };
      return [];
    });
    render(
      <MemoryRouter initialEntries={['/login']}>
        <Login />
      </MemoryRouter>,
    );
    await waitFor(() =>
      expect(screen.getAllByRole('button', { name: /sign in with google/i })).toHaveLength(1),
    );
  });

  it('shows localized error messages for non-member and unverified Google callback errors', async () => {
    render(
      <MemoryRouter initialEntries={['/login?google_error=GOOGLE_NOT_MEMBER']}>
        <Login />
      </MemoryRouter>,
    );
    expect(await screen.findByText(/not registered as a grace ride member/i)).toBeInTheDocument();

    render(
      <MemoryRouter initialEntries={['/login?google_error=GOOGLE_NOT_VERIFIED']}>
        <Login />
      </MemoryRouter>,
    );
    expect(
      await screen.findByText(/google account has not been verified yet/i),
    ).toBeInTheDocument();
  });

  it('supports password sign-in, 2FA challenge prompt, and Google-verified forgot password', async () => {
    const { ApiError } = await import('./api');
    mockApi.post.mockImplementation(async (path: string) => {
      if (path === '/auth/login') {
        throw new ApiError(400, '2FA code required', 'TOTP_REQUIRED');
      }
      if (path === '/auth/google/start') {
        return { url: 'https://accounts.google.com/o/oauth2/v2/auth?state=s1' };
      }
      return { ok: true };
    });

    const assignMock = vi.fn();
    Object.defineProperty(window, 'location', {
      configurable: true,
      value: { ...window.location, assign: assignMock },
    });

    render(
      <MemoryRouter initialEntries={['/login']}>
        <Login />
      </MemoryRouter>,
    );

    fireEvent.click(await screen.findByRole('button', { name: /sign in with password/i }));
    fireEvent.change(screen.getByLabelText(/email/i), { target: { value: 'grace@example.org' } });
    fireEvent.change(screen.getByLabelText(/password/i), { target: { value: 'GraceRide2026!' } });
    fireEvent.click(screen.getByRole('button', { name: /^log in$/i }));

    expect(await screen.findByLabelText(/2fa code or backup code/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /forgot password\?/i }));
    const dialog = await screen.findByRole('dialog');
    expect(dialog).toHaveTextContent(/only available for invited members who have verified their google account/i);
    fireEvent.click(screen.getByRole('button', { name: /verify with google/i }));

    await waitFor(() =>
      expect(mockApi.post).toHaveBeenCalledWith('/auth/google/start', {
        purpose: 'password_reset',
        email: 'grace@example.org',
      }),
    );
    await waitFor(() =>
      expect(assignMock).toHaveBeenCalledWith('https://accounts.google.com/o/oauth2/v2/auth?state=s1'),
    );
  });

  it('completes password reset modal when redirected with google_reset=1', async () => {
    mockApi.api.mockImplementation(async (path: string) => {
      if (path === '/auth/google/config') return { enabled: true };
      if (path === '/auth/password-reset/status') return { valid: true, email: 'grace@example.org' };
      return [];
    });
    mockApi.post.mockResolvedValue({ ok: true });

    render(
      <MemoryRouter initialEntries={['/login?google_reset=1']}>
        <Login />
      </MemoryRouter>,
    );

    expect(await screen.findByText(/google account verified for grace@example\.org/i)).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText(/^new password/i), { target: { value: 'NewSecret2026!' } });
    fireEvent.change(screen.getByLabelText(/^confirm password$/i), { target: { value: 'NewSecret2026!' } });
    fireEvent.click(screen.getByRole('button', { name: /reset password/i }));

    await waitFor(() =>
      expect(mockApi.post).toHaveBeenCalledWith('/auth/password-reset/complete', {
        new_password: 'NewSecret2026!',
      }),
    );
    expect(await screen.findByText(/password updated/i)).toBeInTheDocument();
  });

  it('shows Google verification and optional password setup on Invite Step 3', async () => {
    mockApi.useApi.mockReturnValue({
      data: { kind: 'member', email: 'invitee@example.org', name: 'Invitee' },
      loading: false,
      error: null,
      reload: vi.fn(),
    });
    mockApi.post.mockImplementation(async (path: string) => {
      if (path === '/auth/google/link/start') {
        return { url: 'https://accounts.google.com/o/oauth2/v2/auth?state=inv' };
      }
      return { ok: true };
    });

    const assignMock = vi.fn();
    Object.defineProperty(window, 'location', {
      configurable: true,
      value: { ...window.location, assign: assignMock },
    });

    render(
      <ConfirmProvider>
        <MemoryRouter initialEntries={['/invite/tok1']}>
          <Routes>
            <Route path="/invite/:token" element={<Invite />} />
          </Routes>
        </MemoryRouter>
      </ConfirmProvider>,
    );

    fireEvent.change(screen.getByLabelText(/phone/i), { target: { value: '0123456789' } });
    fireEvent.click(screen.getByRole('button', { name: /continue/i }));
    fireEvent.click(screen.getByRole('checkbox'));
    fireEvent.click(screen.getByRole('button', { name: /agree/i }));

    const googleBtn = await screen.findByRole('button', { name: /verify & connect google account/i });
    fireEvent.click(googleBtn);
    await waitFor(() =>
      expect(mockApi.post).toHaveBeenCalledWith('/auth/google/link/start', {
        redirect: '/me?section=security',
      }),
    );
    await waitFor(() =>
      expect(assignMock).toHaveBeenCalledWith('https://accounts.google.com/o/oauth2/v2/auth?state=inv'),
    );

    fireEvent.click(screen.getByRole('button', { name: /set a password/i }));
    fireEvent.change(screen.getByLabelText(/^new password/i), { target: { value: 'InviteePass2026!' } });
    fireEvent.change(screen.getByLabelText(/^confirm password$/i), { target: { value: 'InviteePass2026!' } });
    fireEvent.click(screen.getByRole('button', { name: /save password/i }));

    await waitFor(() =>
      expect(mockApi.post).toHaveBeenCalledWith('/auth/password', {
        new_password: 'InviteePass2026!',
      }),
    );
    expect(await screen.findByRole('button', { name: /continue to grace ride/i })).toBeInTheDocument();
  });

  it('allows linking, unlinking Google, and setting password in Profile Account Security', async () => {
    mockAuthState.me = { ...baseMe, google_verified: false, has_password: false };

    render(
      <ConfirmProvider>
        <MemoryRouter initialEntries={['/me?section=security&google=linked']}>
          <Profile />
        </MemoryRouter>
      </ConfirmProvider>,
    );

    expect(await screen.findByText(/google account verified and connected/i)).toBeInTheDocument();
    expect(screen.getByText('Not verified')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /verify & connect google account/i })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /set password/i }));
    fireEvent.change(screen.getByLabelText(/^new password/i), { target: { value: 'ProfilePass2026!' } });
    fireEvent.change(screen.getByLabelText(/^confirm password$/i), { target: { value: 'ProfilePass2026!' } });
    fireEvent.click(screen.getByRole('button', { name: /^save$/i }));

    await waitFor(() =>
      expect(mockApi.post).toHaveBeenCalledWith('/auth/password', {
        new_password: 'ProfilePass2026!',
      }),
    );
  });

  it('prompts for 2FA when redirected with google_totp=1 and completes sign-in', async () => {
    mockApi.api.mockImplementation(async (path: string) => {
      if (path === '/auth/google/config') return { enabled: true };
      if (path === '/auth/google/totp/status') return { valid: true, email: 'grace@example.org' };
      return [];
    });
    mockApi.post.mockResolvedValue({ ok: true, redirect: '/rides' });

    render(
      <MemoryRouter initialEntries={['/login?google_totp=1']}>
        <Login />
      </MemoryRouter>,
    );

    expect(await screen.findByText(/google account verified for grace@example\.org/i)).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText(/2fa code or backup code/i), { target: { value: '123456' } });
    fireEvent.click(screen.getByRole('button', { name: /verify & sign in/i }));

    await waitFor(() =>
      expect(mockApi.post).toHaveBeenCalledWith('/auth/google/totp/complete', {
        code: '123456',
      }),
    );
    await waitFor(() => expect(mockAuthState.refresh).toHaveBeenCalled());
  });

  it('shows Verify 2FA button on Profile for non-privileged TOTP members with mfa_ok=false', async () => {
    mockAuthState.me = {
      ...baseMe,
      privileged: false,
      totp_enabled: true,
      mfa_ok: false,
    };

    render(
      <ConfirmProvider>
        <MemoryRouter initialEntries={['/me?section=security']}>
          <Profile />
        </MemoryRouter>
      </ConfirmProvider>,
    );

    expect(await screen.findByRole('button', { name: /verify 2fa/i })).toBeInTheDocument();
  });

  it('shows in-app browser warning on Invite Step 3 under WhatsApp UA and refreshes auth after saving password', async () => {
    const origUa = navigator.userAgent;
    Object.defineProperty(navigator, 'userAgent', {
      value: 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0) WhatsApp/2.24',
      configurable: true,
    });
    mockApi.useApi.mockReturnValue({
      data: { kind: 'member', email: 'invitee@example.org', name: 'Invitee' },
      loading: false,
      error: null,
      reload: vi.fn(),
    });
    try {
      render(
        <ConfirmProvider>
          <MemoryRouter initialEntries={['/invite/tok-inapp']}>
            <Routes>
              <Route path="/invite/:token" element={<Invite />} />
            </Routes>
          </MemoryRouter>
        </ConfirmProvider>,
      );

      fireEvent.change(screen.getByLabelText(/phone/i), { target: { value: '0123456789' } });
      fireEvent.click(screen.getByRole('button', { name: /continue/i }));
      fireEvent.click(screen.getByRole('checkbox'));
      fireEvent.click(screen.getByRole('button', { name: /agree/i }));

      expect(await screen.findByText(/open in chrome or safari to verify/i)).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /copy security link/i })).toBeInTheDocument();

      fireEvent.click(screen.getByRole('button', { name: /set a password/i }));
      fireEvent.change(screen.getByLabelText(/^new password/i), { target: { value: 'InviteePass2026!' } });
      fireEvent.change(screen.getByLabelText(/^confirm password$/i), { target: { value: 'InviteePass2026!' } });
      fireEvent.click(screen.getByRole('button', { name: /save password/i }));

      await waitFor(() =>
        expect(mockApi.post).toHaveBeenCalledWith('/auth/password', {
          new_password: 'InviteePass2026!',
        }),
      );
      await waitFor(() => expect(mockAuthState.refresh).toHaveBeenCalledTimes(2));
    } finally {
      Object.defineProperty(navigator, 'userAgent', { value: origUa, configurable: true });
    }
  });
});
