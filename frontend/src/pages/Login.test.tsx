import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import Login from './Login';

vi.mock('../auth', () => ({
  useAuth: () => ({ me: null, loading: false, refresh: vi.fn() }),
}));

vi.mock('../api', () => ({
  api: vi.fn().mockResolvedValue([]),
  post: vi.fn(),
}));

vi.mock('../webauthn', () => ({
  loginWithPasskey: vi.fn(),
  passkeySupported: () => false,
}));

vi.mock('../i18n', () => ({
  setLanguage: vi.fn(),
  default: { language: 'en', changeLanguage: vi.fn() },
}));

vi.mock('react-i18next', () => ({
  initReactI18next: { type: '3rdParty', init: vi.fn() },
  useTranslation: () => ({
    t: (k: string) => k,
    i18n: { language: 'en' },
  }),
}));

vi.mock('../components/ui', () => ({
  useAction: () => ({ busy: false, error: null, run: (fn: () => Promise<void>) => fn() }),
  ErrorBox: ({ error }: { error?: string | null }) => (error ? <div>{error}</div> : null),
  Field: ({ label, children }: { label: string; children: unknown }) => (
    <label>
      {label}
      {children as never}
    </label>
  ),
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  Button: ({ children, type = 'button', block: _block, ...rest }: React.ButtonHTMLAttributes<HTMLButtonElement> & { block?: boolean }) => (
    <button type={type} {...rest}>
      {children}
    </button>
  ),
}));

describe('Login', () => {
  beforeEach(() => {
    vi.clearAllMocks();
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
});
