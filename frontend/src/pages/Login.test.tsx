import { describe, expect, it, vi, beforeEach } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
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
  useAction: () => ({ busy: false, error: null, run: (fn: () => Promise<void>) => fn() }),
  ErrorBox: ({ error }: { error?: string | null }) => (error ? <div>{error}</div> : null),
  Field: ({ label, children }: { label: string; children: React.ReactNode }) => (
    <label>
      {label}
      {children}
    </label>
  ),
  Button: ({ children, type = 'button', block: _block, loading: _loading, ...rest }: React.ButtonHTMLAttributes<HTMLButtonElement> & { block?: boolean; loading?: boolean }) => (
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
    expect(screen.getByText('thisDevice')).toBeInTheDocument();
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
});
