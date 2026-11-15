import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { put, post } from '../api';
import Profile from './Profile';

const baseMe = {
  id: 1,
  name: 'Grace Member',
  email: 'grace@example.com',
  phone: '0123456789',
  photo_file_id: null,
  roles: ['member'],
  role_names: {},
  permissions: ['costs', 'reimbursements', 'help', 'report'],
  status: 'active' as const,
  terms_accepted: true,
  mfa_ok: true,
  totp_enabled: false,
  has_password: false,
  passkey_count: 0,
  badges_opt_out: false,
  readiness_prompt_mins: 45,
  auto_lock_mins: 0,
  backup_codes_count: 8,
  privileged: false,
  unread: 0,
  groups: [{ id: 10, name: 'Youth' }],
  dev_mode: false,
};

let currentMe = baseMe;
const confirmMock = vi.fn().mockResolvedValue(false);

vi.mock('../auth', () => ({
  useAuth: () => ({
    me: currentMe,
    can: (page: string) =>
      ['costs', 'reimbursements', 'help', 'report', 'rides', 'events'].includes(page),
    refresh: vi.fn(),
  }),
}));

vi.mock('../api', () => ({
  del: vi.fn(),
  post: vi.fn().mockResolvedValue(undefined),
  put: vi.fn().mockResolvedValue(undefined),
  useApi: () => ({ data: [], loading: false, error: null, reload: vi.fn() }),
}));

vi.mock('../webauthn', () => ({
  passkeySupported: () => false,
  registerPasskey: vi.fn(),
}));

vi.mock('../components/AutoLock', () => ({
  useAutoLock: () => ({ lock: vi.fn() }),
}));

vi.mock('../components/AppChrome', () => ({
  DisplayPreferences: () => <div data-testid="display-preferences">Display preferences</div>,
}));

vi.mock('../format', () => ({
  fmtAgo: (d: string) => d,
  roleLabel: (key: string) => key,
  useFormatT: () => (k: string) => k,
}));

const profileStrings: Record<string, string> = {
  logout: 'Log out',
  loggedOut: 'Logged out',
  member: 'Member',
  administrator: 'Administrator',
  pending: 'Pending approval',
  personalInfo: 'Personal details',
  preferences: 'Preferences',
  security: 'Account security',
  account: 'Account',
  relatedPages: 'Related pages',
  myCosts: 'My costs',
  myReimb: 'My reimbursements',
  myReport: 'My report',
  myHelp: 'Help centre',
  name: 'Name',
  email: 'Email',
  phone: 'Phone',
  status: 'Status',
  save: 'Save',
  photo: 'Photo',
  optOut: 'Opt out of thank-you badges',
  readinessPromptLabel: 'Pre-trip readiness reminder',
  autoLock: 'Inactivity auto-lock',
  passkeys: 'Passkeys',
  totp: 'Two-factor authentication (2FA)',
  autoLockNever: 'Never (Disabled)',
  logoutAll: 'Log out all devices',
};

vi.mock('react-i18next', () => ({
  initReactI18next: { type: '3rdParty', init: vi.fn() },
  useTranslation: () => ({
    t: (k: string) => profileStrings[k] ?? k,
    i18n: { language: 'en' },
  }),
}));

vi.mock('../components/ui', () => ({
  Avatar: ({ name }: { name: string; fileId?: number | null; size?: string }) => (
    <div data-testid="avatar">{name.slice(0, 1)}</div>
  ),
  Button: ({
    children,
    type = 'button',
    ...rest
  }: React.ButtonHTMLAttributes<HTMLButtonElement> & { children?: React.ReactNode }) => (
    <button type={type} data-testid="button" {...rest}>
      {children}
    </button>
  ),
  Card: ({ children }: { children?: React.ReactNode }) => <div data-testid="card">{children}</div>,
  ErrorBox: ({ error }: { error?: string | null }) =>
    error ? <div data-testid="error-box">{error}</div> : null,
  Field: ({ label, children }: { label: string; children: unknown }) => (
    <label data-testid="field">{label}{children}</label>
  ),
  FileInput: ({ label }: { label: string }) => (
    <label data-testid="fileinput">{label}</label>
  ),
  Modal: ({
    children,
    title,
    onClose,
  }: {
    children?: React.ReactNode;
    title?: string;
    onClose?: () => void;
  }) => (
    <div data-testid="modal" role="dialog" aria-label={title}>
      <h2>{title}</h2>
      <button type="button" aria-label="Close" onClick={onClose}>
        Close
      </button>
      {children}
    </div>
  ),
  Pill: ({ children, tone }: { children?: React.ReactNode; tone?: string }) => (
    <span data-testid={`pill-${tone}`}>{children}</span>
  ),
  useAction: () => ({ busy: false, error: null, run: (fn: () => Promise<unknown>) => fn() }),
  useConfirm: () => confirmMock,
}));

beforeEach(() => {
  vi.clearAllMocks();
  currentMe = baseMe;
  confirmMock.mockResolvedValue(false);
});

const renderProfile = (me: typeof baseMe) => {
  currentMe = me;
  return render(
    <MemoryRouter initialEntries={['/me']}>
      <Profile />
    </MemoryRouter>,
  );
};

describe('Profile compact layout', () => {
  it('renders avatar, name, and a single role label for a member', () => {
    renderProfile(baseMe);
    expect(screen.getByText('Grace Member')).toBeInTheDocument();
    expect(screen.getAllByText('Member').length).toBeGreaterThan(0);
    expect(screen.queryByText('Administrator')).not.toBeInTheDocument();
    expect(screen.queryByText('Pending approval')).not.toBeInTheDocument();
  });

  it('shows Administrator for privileged accounts', () => {
    renderProfile({ ...baseMe, privileged: true, roles: ['admin', 'member'] });
    expect(screen.getAllByText('Administrator').length).toBe(1);
    expect(screen.queryByText('Pending approval')).not.toBeInTheDocument();
  });

  it('shows Pending approval for pending accounts', () => {
    renderProfile({ ...baseMe, status: 'pending', privileged: false });
    expect(screen.getAllByText('Pending approval').length).toBeGreaterThan(0);
  });

  it('lists section rows and opens one section sheet at a time', () => {
    renderProfile(baseMe);
    expect(screen.getByRole('button', { name: /Personal details/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Preferences/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Account security/i })).toBeInTheDocument();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Preferences/i }));
    expect(screen.getByRole('dialog', { name: /Preferences/i })).toBeInTheDocument();
    expect(screen.queryByRole('dialog', { name: /Personal details/i })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Close' }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Personal details/i }));
    expect(screen.getByRole('dialog', { name: /Personal details/i })).toBeInTheDocument();
    expect(screen.queryByRole('dialog', { name: /Preferences/i })).not.toBeInTheDocument();
  });

  it('retains edits when a section sheet is closed and reopened', () => {
    renderProfile(baseMe);
    fireEvent.click(screen.getByRole('button', { name: /Personal details/i }));
    const nameInput = screen.getByLabelText('Name');
    fireEvent.change(nameInput, { target: { value: 'Edited Name' } });
    expect(nameInput).toHaveValue('Edited Name');

    fireEvent.click(screen.getByRole('button', { name: 'Close' }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Personal details/i }));
    expect(screen.getByLabelText('Name')).toHaveValue('Edited Name');
  });

  it('does not include administrative shortcuts in the profile', () => {
    renderProfile({ ...baseMe, privileged: true, roles: ['admin'] });
    expect(screen.queryByText('Transport dashboard')).not.toBeInTheDocument();
    expect(screen.queryByText('Roles & permissions')).not.toBeInTheDocument();
    expect(screen.queryByText('Members & invites')).not.toBeInTheDocument();
    expect(screen.queryByText('Reimbursement review')).not.toBeInTheDocument();
  });

  it('keeps permitted non-admin shortcuts reachable for a member', () => {
    renderProfile(baseMe);
    for (const label of ['My costs', 'My reimbursements', 'Help centre']) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
  });

  it('hides related pages shortcuts for pending accounts', () => {
    renderProfile({ ...baseMe, status: 'pending' });
    expect(screen.queryByText('My costs')).not.toBeInTheDocument();
    expect(screen.queryByText('Related pages')).not.toBeInTheDocument();
  });

  it('sends only personal fields on personal save and preserves photo', () => {
    const putMock = vi.mocked(put);
    renderProfile({ ...baseMe, photo_file_id: 42 });
    fireEvent.click(screen.getByRole('button', { name: /Personal details/i }));
    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'New Name' } });
    fireEvent.click(screen.getByRole('button', { name: /^Save$/ }));
    expect(putMock).toHaveBeenCalledWith(
      '/me',
      expect.objectContaining({ name: 'New Name', phone: '0123456789', photo_file_id: 42 }),
    );
  });

  it('renders a Log out action below the sections', () => {
    renderProfile(baseMe);
    expect(screen.getByRole('button', { name: /^Log out$/ })).toBeInTheDocument();
  });

  it('sends badge opt-out and readiness prompt on preferences save', () => {
    const putMock = vi.mocked(put);
    renderProfile(baseMe);
    fireEvent.click(screen.getByRole('button', { name: /Preferences/i }));
    fireEvent.click(screen.getByLabelText('Opt out of thank-you badges'));
    const readiness = screen.getByRole('combobox');
    fireEvent.change(readiness, { target: { value: '30' } });
    fireEvent.click(screen.getByRole('button', { name: /^Save$/ }));
    expect(putMock).toHaveBeenCalledWith(
      '/me',
      expect.objectContaining({ badges_opt_out: true, readiness_prompt_mins: 30 }),
    );
  });

  it('logs out on Log out', () => {
    const postMock = vi.mocked(post);
    renderProfile(baseMe);
    fireEvent.click(screen.getByRole('button', { name: /^Log out$/ }));
    expect(postMock).toHaveBeenCalledWith('/auth/logout');
  });

  it('logs out all devices when confirmation is accepted', async () => {
    vi.mocked(confirmMock).mockResolvedValueOnce(true);
    const postMock = vi.mocked(post);
    renderProfile(baseMe);
    fireEvent.click(screen.getByRole('button', { name: /Account security/i }));
    fireEvent.click(screen.getByRole('button', { name: /^Log out all devices$/ }));
    await waitFor(() => expect(postMock).toHaveBeenCalledWith('/auth/logout-all'));
  });

  it('does not log out all devices when confirmation is declined', async () => {
    const postMock = vi.mocked(post);
    renderProfile(baseMe);
    fireEvent.click(screen.getByRole('button', { name: /Account security/i }));
    fireEvent.click(screen.getByRole('button', { name: /^Log out all devices$/ }));
    await waitFor(() => expect(confirmMock).toHaveBeenCalled());
    expect(postMock).not.toHaveBeenCalledWith('/auth/logout-all');
  });
});