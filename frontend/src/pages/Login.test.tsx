import { describe, expect, it, vi, beforeEach } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { post } from '../api';
import Login from './Login';

const mockWebauthn = vi.hoisted(() => ({
  loginWithPasskey: vi.fn(),
  passkeySupported: vi.fn(() => false),
  getDevicePasskey: vi.fn(),
  clearDevicePasskey: vi.fn(),
  saveDevicePasskey: vi.fn(),
}));

vi.mock('../auth', () => ({
  useAuth: () => ({ me: null, loading: false, refresh: vi.fn() }),
}));

vi.mock('../api', () => ({
  api: vi.fn().mockResolvedValue([]),
  post: vi.fn(),
}));

vi.mock('../webauthn', () => mockWebauthn);

vi.mock('../i18n', () => ({
  setLanguage: vi.fn(),
  default: { language: 'en', changeLanguage: vi.fn() },
}));

vi.mock('react-i18next', () => ({
  initReactI18next: { type: '3rdParty', init: vi.fn() },
  useTranslation: () => ({
    t: (k: string, opts?: { email?: string }) => (opts?.email ? `${k}:${opts.email}` : k),
    i18n: { language: 'en' },
  }),
}));

vi.mock('../components/ui', () => ({
  useAction: () => ({
    busy: false,
    error: null,
    run: async (fn: () => Promise<void>) => {
      await fn();
      return true;
    },
  }),
  ErrorBox: ({ error }: { error?: string | null }) => (error ? <div>{error}</div> : null),
  Notice: ({ children }: { children?: React.ReactNode }) => <div role="status">{children}</div>,
  Modal: ({ title, children, onClose }: { title: string; children: React.ReactNode; onClose: () => void }) => (
    <div role="dialog" aria-label={title}>
      <h2>{title}</h2>
      <button type="button" aria-label="Close" onClick={onClose}>Close</button>
      {children}
    </div>
  ),
  Field: ({ label, children }: { label: string; children: React.ReactNode }) => (
    <label>
      {label}
      {children}
    </label>
  ),
  Button: ({ children, type = 'button', block: _block, loading: _loading, size: _size, ...rest }: React.ButtonHTMLAttributes<HTMLButtonElement> & { block?: boolean; loading?: boolean; size?: string }) => (
    <button type={type} {...rest}>
      {children}
    </button>
  ),
}));

describe('Login', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockWebauthn.passkeySupported.mockReturnValue(false);
    mockWebauthn.getDevicePasskey.mockReturnValue(null);
  });

  it('renders passkey email form and terms link', async () => {
    render(
      <MemoryRouter>
        <Login />
      </MemoryRouter>,
    );
    expect(screen.getByText('welcomeBack')).toBeInTheDocument();
    expect(screen.getByRole('textbox')).toBeInTheDocument();
    expect(screen.getByText('noPasskey')).toBeInTheDocument();
  });

  it('displays recognized device passkey card when device has passkey registered', async () => {
    mockWebauthn.passkeySupported.mockReturnValue(true);
    mockWebauthn.getDevicePasskey.mockReturnValue({ email: 'alice@example.com', savedAt: 12345 });

    render(
      <MemoryRouter>
        <Login />
      </MemoryRouter>,
    );

    expect(screen.getByText('passkeyDetected')).toBeInTheDocument();
    expect(screen.getByText('alice@example.com')).toBeInTheDocument();
    expect(screen.queryByRole('textbox')).toBeNull();
    expect(screen.getByRole('button', { name: /loginWithPasskey/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /switchAccount/i })).toBeInTheDocument();

    // Submit form logs in with recognized passkey email
    fireEvent.click(screen.getByRole('button', { name: /loginWithPasskey/i }));
    expect(mockWebauthn.loginWithPasskey).toHaveBeenCalledWith('alice@example.com');
  });

  it('allows switching to manual email input and back to recognized passkey', async () => {
    mockWebauthn.passkeySupported.mockReturnValue(true);
    mockWebauthn.getDevicePasskey.mockReturnValue({ email: 'alice@example.com', savedAt: 12345 });

    render(
      <MemoryRouter>
        <Login />
      </MemoryRouter>,
    );

    // Switch to another account
    fireEvent.click(screen.getByRole('button', { name: /switchAccount/i }));
    const input = screen.getByRole('textbox') as HTMLInputElement;
    expect(input).toBeInTheDocument();
    expect(input.value).toBe('');

    // Switch back to saved passkey
    fireEvent.click(screen.getByRole('button', { name: /useSavedPasskey/i }));
    expect(screen.queryByRole('textbox')).toBeNull();
    expect(screen.getByText('alice@example.com')).toBeInTheDocument();
  });

  it('shows embedded-browser guidance when opened inside WeChat or WhatsApp', () => {
    const origUa = navigator.userAgent;
    Object.defineProperty(navigator, 'userAgent', { value: 'Mozilla/5.0 MicroMessenger/8.0', configurable: true });
    try {
      render(
        <MemoryRouter>
          <Login />
        </MemoryRouter>,
      );
      expect(screen.getByText('inAppTitle')).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /copyLoginLink/i })).toBeInTheDocument();
    } finally {
      Object.defineProperty(navigator, 'userAgent', { value: origUa, configurable: true });
    }
  });

  it('opens guided recovery modal, blocks empty email, and submits email + chosen reason preset', async () => {
    vi.mocked(post).mockResolvedValueOnce({ ok: true });
    render(
      <MemoryRouter>
        <Login />
      </MemoryRouter>,
    );

    fireEvent.click(screen.getByRole('button', { name: /emailRecovery/i }));
    const dlg = screen.getByRole('dialog', { name: 'recoveryModalTitle' });
    expect(dlg).toBeInTheDocument();

    // Empty email is blocked locally without hitting the server
    fireEvent.click(screen.getByRole('button', { name: /recoverySubmit/i }));
    expect(screen.getByRole('alert')).toHaveTextContent('recoveryEmailRequired');
    expect(post).not.toHaveBeenCalled();

    // Fill email + pick reason preset chip and submit
    const inputs = dlg.querySelectorAll('input[type="email"]');
    fireEvent.change(inputs[0], { target: { value: 'elder@example.org' } });
    fireEvent.click(screen.getByRole('button', { name: 'recoveryPresetNewPhone' }));
    fireEvent.click(screen.getByRole('button', { name: /recoverySubmit/i }));

    await waitFor(() =>
      expect(post).toHaveBeenCalledWith('/auth/recovery-request', {
        email: 'elder@example.org',
        reason: 'recoveryPresetNewPhone',
      }),
    );
    expect(await screen.findByText('recoverySentTitle')).toBeInTheDocument();
  });
});
