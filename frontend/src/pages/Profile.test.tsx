import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { Link, MemoryRouter } from 'react-router-dom';
import { put, post } from '../api';
import { _setInstalledForTesting, _setDeferredPromptForTesting, BeforeInstallPromptEvent } from '../pwaInstall';
import Profile from './Profile';

const baseMe = {
  id: 1,
  name: 'Grace Member',
  email: 'grace@example.com',
  phone: '0123456789',
  photo_file_id: null as number | null,
  roles: ['member'],
  role_names: {},
  permissions: ['costs', 'reimbursements', 'help', 'report'],
  status: 'active' as 'active' | 'pending' | 'suspended',
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
const emptyFamily = {
  family: null as { id: number; name: string; owner_id: number } | null,
  members: [] as { user_id: number; name: string; status: string }[],
  requests: [] as { id: number; user_id: number; name: string }[],
  invites: [] as { id: number; phone: string; expires_at: string; url: string }[],
  incoming: null as { id: number; family_name: string; owner_name: string } | null,
};
let familyData = { ...emptyFamily };
const confirmMock = vi.fn().mockResolvedValue(false);

vi.mock('../auth', () => ({
  useAuth: () => ({
    me: currentMe,
    can: (page: string) =>
      ['costs', 'reimbursements', 'help', 'report', 'rides', 'events', 'drive'].includes(page),
    refresh: vi.fn(),
  }),
}));

vi.mock('../api', () => ({
  del: vi.fn(),
  post: vi.fn().mockResolvedValue(undefined),
  put: vi.fn().mockResolvedValue(undefined),
  useApi: (path: string | null) => ({
    data: path === '/me/family' ? familyData : [],
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
}));

vi.mock('../webauthn', () => ({
  passkeySupported: () => false,
  registerPasskey: vi.fn(),
  withRecentSignIn: (fn: () => Promise<unknown>) => fn(),
}));

vi.mock('../components/AutoLock', () => ({
  useAutoLock: () => ({ lock: vi.fn() }),
}));

vi.mock('../components/AvatarEditor', () => ({
  default: ({ onClose }: { onClose: () => void }) => (
    <div data-testid="avatar-editor">
      <button type="button" onClick={onClose}>close editor</button>
    </div>
  ),
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
  myRideHistory: 'My ride history',
  myDrivingHistory: 'My driving history',
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
  installApp: 'Add App to Home Screen',
  installAppBannerTitle: 'Add Grace Ride to Home Screen',
  installAppBannerDesc: 'No app store download needed — open instantly with one tap',
  installAppOneClick: '1-Click Install to Desktop',
  installAppGuide: 'Setup Guide',
  installAppGuideTitle: 'Add App to Home Screen (No App Store Needed)',
  installAppInstalled: 'Added to Home Screen',
  installAppInstalledDesc: 'Currently running as a standalone mobile app with full-screen experience.',
  family: 'Family',
  familyName: 'Family name',
  createFamily: 'Create family',
  addPhone: 'Phone number',
  leaveFamily: 'Leave family',
  waitingApproval: 'Waiting for approval',
  copyLink: 'Copy link',
  copySecret: 'Copy setup key',
  secretCopied: 'Setup key copied',
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
    loading: _loading,
    variant: _variant,
    size: _size,
    block: _block,
    icon: _icon,
    ...rest
  }: React.ButtonHTMLAttributes<HTMLButtonElement> & {
    children?: React.ReactNode;
    loading?: boolean;
    variant?: string;
    size?: string;
    block?: boolean;
    icon?: React.ReactNode;
  }) => (
    <button type={type} data-testid="button" {...rest}>
      {children}
    </button>
  ),
  Card: ({ children }: { children?: React.ReactNode }) => <div data-testid="card">{children}</div>,
  ErrorBox: ({ error }: { error?: string | null }) =>
    error ? <div data-testid="error-box">{error}</div> : null,
  Field: ({ label, children }: { label: string; children: React.ReactNode }) => (
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
    size?: string;
    initialFocusRef?: React.RefObject<HTMLElement | null>;
  }) => (
    <div data-testid="modal" role="dialog" aria-label={title}>
      <h2>{title}</h2>
      <button type="button" aria-label="Close" onClick={onClose}>
        Close
      </button>
      {children}
    </div>
  ),
  PageHeader: ({ title, back }: { title?: React.ReactNode; back?: string }) => (
    <div data-testid="page-header" data-back={back}>
      <h1>{title}</h1>
    </div>
  ),
  Notice: ({ children }: { children?: React.ReactNode }) => (
    <div data-testid="notice">{children}</div>
  ),
  Pill: ({ children, tone }: { children?: React.ReactNode; tone?: string }) => (
    <span data-testid={`pill-${tone}`}>{children}</span>
  ),
  useAction: () => ({ busy: false, error: null, run: (fn: () => Promise<unknown>) => fn() }),
  useConfirm: () => confirmMock,
  SheetLink: ({
    to,
    icon,
    label,
    onClick,
    chevron: _chevron,
    hint,
    tone: _tone,
    badge,
    danger: _danger,
    ...rest
  }: {
    to?: string;
    icon?: React.ReactNode;
    label?: React.ReactNode;
    onClick?: () => void;
    chevron?: boolean;
    hint?: React.ReactNode;
    tone?: string;
    badge?: React.ReactNode;
    danger?: boolean;
    [k: string]: unknown;
  }) => {
    const content = (
      <>
        {icon}
        {label}
        {hint && <span>{hint}</span>}
        {badge}
      </>
    );
    if (to) return <Link to={to}>{content}</Link>;
    return (
      <button type="button" onClick={onClick} {...rest}>
        {content}
      </button>
    );
  },
}));

beforeEach(() => {
  vi.clearAllMocks();
  currentMe = baseMe;
  familyData = {
    family: null,
    members: [],
    requests: [],
    invites: [],
    incoming: null,
  };
  confirmMock.mockResolvedValue(false);
});

const renderProfile = (me: typeof baseMe, path = '/me') => {
  currentMe = me;
  return render(
    <MemoryRouter initialEntries={[path]}>
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
    expect(screen.queryByTestId('page-header')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Preferences/i }));
    expect(screen.getByTestId('page-header')).toHaveTextContent(/Preferences/i);
    expect(screen.getByTestId('page-header')).toHaveAttribute('data-back', '/me');
    expect(screen.queryByRole('button', { name: /Personal details/i })).not.toBeInTheDocument();
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

  it('keeps ride and driving history reachable from the profile', () => {
    renderProfile(baseMe);
    expect(screen.getByText('My ride history').closest('a')).toHaveAttribute('href', '/rides');
    expect(screen.getByText('My driving history').closest('a')).toHaveAttribute('href', '/drive');
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
    expect(putMock).toHaveBeenCalledWith('/me', { name: 'New Name', phone: '0123456789' });
  });

  it('summarises each section under its row', () => {
    renderProfile({ ...baseMe, passkey_count: 2, totp_enabled: true });
    expect(screen.getByText('0123456789')).toBeInTheDocument();
    expect(screen.getByText('pushOnShort · remindShort')).toBeInTheDocument();
    expect(screen.getByText('passkeyCount · twoFaOnShort')).toBeInTheDocument();
  });

  it('opens the avatar editor from the avatar and from personal details', () => {
    renderProfile(baseMe);
    expect(screen.queryByTestId('avatar-editor')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'avatarEdit' }));
    expect(screen.getByTestId('avatar-editor')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'close editor' }));
    fireEvent.click(screen.getByRole('button', { name: /Personal details/i }));
    fireEvent.click(screen.getByRole('button', { name: 'avatarEdit' }));
    expect(screen.getByTestId('avatar-editor')).toBeInTheDocument();
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

  it('shows a scannable QR code when setting up 2FA', async () => {
    vi.mocked(post).mockResolvedValueOnce({ secret: 'JBSWY3DPEHPK3PXP', url: 'otpauth://totp/Grace%20Ride:a@b.c?secret=JBSWY3DPEHPK3PXP' });
    renderProfile(baseMe);
    fireEvent.click(screen.getByRole('button', { name: /Account security/i }));
    fireEvent.click(screen.getByRole('button', { name: 'setupTotp' }));
    const qr = await screen.findByAltText('totpQrAlt');
    expect(qr.getAttribute('src')).toMatch(/^data:image\/png;base64,/);
    expect(screen.getByText('JBSWY3DPEHPK3PXP')).toBeInTheDocument();
  });

  it('renders a single clean PWA install row when not yet installed, and opens setup guide modal', () => {
    _setInstalledForTesting(false);
    _setDeferredPromptForTesting(null);
    renderProfile(baseMe);

    expect(screen.queryByText('Add Grace Ride to Home Screen')).not.toBeInTheDocument();
    const installRow = screen.getByRole('button', { name: /Add App to Home Screen/i });
    expect(installRow).toBeInTheDocument();

    fireEvent.click(installRow);
    expect(screen.getByTestId('modal')).toBeInTheDocument();
    expect(screen.getByText('Add App to Home Screen (No App Store Needed)')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Close' }));
    expect(screen.queryByTestId('modal')).not.toBeInTheDocument();
  });

  it('triggers prompt directly when 1-click install prompt is available', async () => {
    _setInstalledForTesting(false);
    const promptMock = vi.fn().mockResolvedValue(undefined);
    const mockPromptEvent = {
      platforms: ['web'],
      userChoice: Promise.resolve({ outcome: 'accepted' as const, platform: 'web' }),
      prompt: promptMock,
    } as unknown as BeforeInstallPromptEvent;
    _setDeferredPromptForTesting(mockPromptEvent);

    renderProfile(baseMe);
    const installRow = screen.getByRole('button', { name: /Add App to Home Screen/i });
    fireEvent.click(installRow);

    await waitFor(() => expect(promptMock).toHaveBeenCalled());
  });

  it('shows installed status badge when already installed', () => {
    _setInstalledForTesting(true);
    _setDeferredPromptForTesting(null);
    renderProfile(baseMe);

    expect(screen.queryByText('Add Grace Ride to Home Screen')).not.toBeInTheDocument();
    expect(screen.getByText('Added to Home Screen')).toBeInTheDocument();
  });

  it('opens an empty family form from the Family row', () => {
    renderProfile(baseMe);
    fireEvent.click(screen.getByRole('link', { name: /Family/i }));
    expect(screen.getByLabelText('Family name')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Create family' })).toBeInTheDocument();
    expect(screen.queryByText('0123456789')).not.toBeInTheDocument();
  });

  it('lists family names and the invite phone separately', () => {
    familyData = {
      family: { id: 9, name: 'Grace Home', owner_id: baseMe.id },
      members: [
        { user_id: 1, name: 'Grace Member', status: 'active' },
        { user_id: 2, name: 'Kin Pending', status: 'pending' },
      ],
      requests: [],
      invites: [{ id: 3, phone: '019-700 2001', expires_at: '2026-11-22T00:00:00', url: 'http://localhost/invite/abc' }],
      incoming: null,
    };
    renderProfile(baseMe, '/me/family');
    expect(screen.getByText('Grace Member')).toBeInTheDocument();
    expect(screen.getByText('Kin Pending')).toBeInTheDocument();
    expect(screen.getByText('Waiting for approval')).toBeInTheDocument();
    expect(screen.getByText('019-700 2001')).toBeInTheDocument();
    for (const row of screen.getAllByTestId('family-member')) {
      expect(row).not.toHaveTextContent('019-700 2001');
    }
  });

  it('hides the create form for a pending member with no family', () => {
    currentMe = { ...baseMe, status: 'pending' };
    renderProfile({ ...baseMe, status: 'pending' }, '/me/family');
    expect(screen.queryByRole('button', { name: 'Create family' })).not.toBeInTheDocument();
  });

  it('allows copying the 2FA secret setup key with visual feedback', async () => {
    const writeMock = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, { clipboard: { writeText: writeMock } });
    vi.mocked(post).mockResolvedValueOnce({ secret: 'JBSWY3DPEHPK3PXP', url: 'otpauth://totp/Grace%20Ride:a@b.c?secret=JBSWY3DPEHPK3PXP' });
    renderProfile(baseMe);
    fireEvent.click(screen.getByRole('button', { name: /Account security/i }));
    fireEvent.click(screen.getByRole('button', { name: 'setupTotp' }));
    await screen.findByAltText('totpQrAlt');

    const copyBtn = screen.getByRole('button', { name: /Copy setup key/i });
    expect(copyBtn).toBeInTheDocument();
    fireEvent.click(copyBtn);
    expect(writeMock).toHaveBeenCalledWith('JBSWY3DPEHPK3PXP');
    expect(screen.getByText('Setup key copied')).toBeInTheDocument();
  });

  it('provides accessible autocomplete attributes for name and phone in personal details', () => {
    renderProfile(baseMe);
    fireEvent.click(screen.getByRole('button', { name: /Personal details/i }));
    const nameInput = screen.getByDisplayValue(baseMe.name);
    const phoneInput = screen.getByDisplayValue(baseMe.phone);
    expect(nameInput).toHaveAttribute('autoComplete', 'name');
    expect(phoneInput).toHaveAttribute('autoComplete', 'tel');
  });
});