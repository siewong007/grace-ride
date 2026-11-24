import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { Link, MemoryRouter } from 'react-router-dom';
import { del, put, post } from '../api';
import { _setInstalledForTesting, _setDeferredPromptForTesting, BeforeInstallPromptEvent } from '../pwaInstall';
import { _resetPushStateForTesting } from '../pushNotifications';
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
let savedPlacesData: {
  id: number;
  label: string;
  area_id: number | null;
  area_name: string | null;
  address: string;
  gate_notes: string;
  lat?: number | null;
  lng?: number | null;
  is_default: number;
}[] = [];
let proxyRidersData: {
  id: number;
  name: string;
  phone: string;
  relationship: string;
  notes: string;
  area_id?: number | null;
  pickup_address?: string;
  gate_notes?: string;
}[] = [];
const areasData = [
  { id: 1, name: 'Petaling Jaya', active: true },
  { id: 2, name: 'Subang Jaya', active: true },
];
let pushConfigData: Record<string, unknown> | undefined = undefined;
const confirmMock = vi.fn().mockResolvedValue(false);

vi.mock('../auth', () => ({
  useAuth: () => ({
    me: currentMe,
    can: (page: string) =>
      ['costs', 'reimbursements', 'help', 'report', 'rides', 'events', 'drive'].includes(page),
    refresh: vi.fn(),
  }),
}));

let notifPrefsData: Record<string, unknown> | null = null;
let pushSubsData: Record<string, unknown> | null = null;

vi.mock('../api', () => ({
  api: vi.fn().mockResolvedValue([]),
  del: vi.fn().mockResolvedValue({ ok: true }),
  post: vi.fn().mockResolvedValue(undefined),
  put: vi.fn().mockResolvedValue(undefined),
  useApi: (path: string | null) => ({
    data:
      path === '/me/family'
        ? familyData
        : path === '/saved-places'
          ? savedPlacesData
          : path === '/areas'
            ? areasData
            : path === '/proxy-riders'
              ? proxyRidersData
              : path === '/push/config'
                ? pushConfigData
                : path === '/notifications/preferences'
                  ? notifPrefsData
                  : path === '/push/subscriptions'
                    ? pushSubsData
                    : [],
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

let mockCanLock = false;
const lockMock = vi.fn();

vi.mock('../components/AutoLock', () => ({
  useAutoLock: () => ({ lock: lockMock, canLock: mockCanLock }),
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
  activitySection: 'Activity',
  moreActivity: 'More activity ({count})',
  hideMoreActivity: 'Hide extra activity',
  lockNow: 'Lock screen now',
  myCosts: 'My costs',
  myRideHistory: 'My ride history',
  myDrivingHistory: 'Driving & schedule',
  driverSettings: 'Vehicle & driving settings',
  driverSettingsHint: 'Car details, service areas & carpool',
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
  enableBrowserPush: 'Enable browser notifications',
  enableNotifications: 'Enable Notifications',
  pushEnabled: 'Browser push alerts enabled.',
  pushEnabledLocal: 'Browser notification permission granted.',
  pushUnsupported: 'Browser notifications are not supported on this device.',
  pushDisabledDevice: 'Browser push disconnected on this device.',
  pushNeedsInstallIos: 'On iPhone and iPad, add Grace Ride to your Home Screen first, then open the installed app and tap Enable Notifications.',
  pushInstallGuideAction: 'How to add to Home Screen',
  pushDeniedIos: 'You previously chose Don’t Allow. Open iPhone Settings → Notifications → Grace Ride and turn on Allow Notifications.',
  pushDeniedAndroid: 'Notifications are blocked. Open Settings → Notifications (or Chrome site settings) and allow notifications for Grace Ride.',
  pushDeniedDesktop: 'Notifications are blocked in your browser. Use the address bar site settings to allow Notifications.',
  pushDismissed: 'Permission prompt was closed. Tap Enable Notifications and choose Allow to turn on alerts.',
  installAppStep2NotifTitle: 'Step 2: Allow notifications',
  installAppStep2NotifDesc: 'Tap Enable Notifications below, then choose Allow when your phone displays the system permission dialog.',
  installAppStep2NotifDone: 'Notifications are enabled on this device.',
  installAppIosStep4: 'Open Grace Ride from your Home Screen, tap Enable Notifications, and choose Allow when iOS asks.',
  installAppAndroidStep4: 'Open Grace Ride from your Home Screen, tap Enable Notifications, and choose Allow when Android asks.',
  installAppDesktopStep2: 'Open the installed app, click Enable Notifications, and choose Allow when prompted.',
  family: 'Family',
  familyName: 'Family name',
  createFamily: 'Create family',
  addPhone: 'Phone number',
  leaveFamily: 'Leave family',
  waitingApproval: 'Waiting for approval',
  copyLink: 'Copy link',
  copySecret: 'Copy setup key',
  secretCopied: 'Setup key copied',
  savedAddresses: 'Saved addresses',
  savedAddressesHint: 'Home, office, or family',
  savedAddressesCountHint: '{label} · {count} saved',
  savedAddressesEmptyTitle: 'No saved addresses yet',
  savedAddressesEmptyDesc: 'Save Home, Work, or family addresses for 1-tap booking.',
  addAddress: 'Add address',
  editAddress: 'Edit address',
  editAddressAria: 'Edit {label}',
  defaultAddress: 'Default',
  setDefaultAddress: 'Set {label} as default',
  setAsDefault: 'Use as default address',
  placePresetHome: 'Home',
  placePresetWork: 'Work',
  placePresetParents: 'Parents',
  placeLabel: 'Label',
  placeLabelPlaceholder: 'E.g. Home / Mom’s house',
  placeAddress: 'Street address',
  placeAddressPlaceholder: 'E.g. 12 Jalan SS2/10, Petaling Jaya',
  placeArea: 'Area',
  placeAreaHint: 'Needed when booking a ride so volunteer drivers in your area see your request.',
  placeAreaMissing: 'Area not set',
  placePinned: 'Map pin saved',
  placeNoPin: 'No map pin yet',
  useCurrentLocation: 'Use my current location',
  locatingGps: 'Getting location…',
  placeGpsCaptured: 'GPS coordinates captured for driver routing.',
  placeGpsDenied: 'Location access was declined. You can pick a suggestion from the address box instead.',
  placeGateNotes: 'Gate / pickup notes (optional)',
  placeGatePlaceholder: 'E.g. Ring guardhouse bell #4',
  remove: 'Remove',
  cancel: 'Cancel',
  calendarFeed: 'Calendar feed',
  calendarFeedShort: 'Sync rides & events (.ics)',
  whatsappNotConfigured: 'WhatsApp alerts are not enabled by the church office yet.',
  telegramNotConfigured: 'Telegram alerts are not enabled by the church office yet.',
  telegramOpenBot: 'Open @{bot} in Telegram',
  telegramGuidedHint: '1. Tap above and press Start in Telegram. 2. Copy the numeric Chat ID shown and paste it below.',
  pickOnMap: 'Adjust pin on map',
  hideMapPicker: 'Hide map',
  mapPickerHint: 'Tap the map or drag the pin to mark your exact pickup gate.',
  proxyRidersTitle: 'Frequent riders',
  proxyRidersDesc: 'People you often book rides for (parents, seniors, cell members).',
  proxyRidersEmptyTitle: 'No frequent riders saved yet',
  proxyRidersEmptyDesc: 'Save parents or cell members here for 1-tap booking.',
  addProxyRider: 'Add frequent rider',
  editProxyRider: 'Edit frequent rider',
  proxyRiderSaved: 'Frequent rider saved',
  removeProxyConfirm: 'Remove this frequent rider?',
  proxyName: 'Passenger name',
  proxyNamePlaceholder: 'E.g. Sister Chen / Uncle Lim',
  proxyRelationship: 'Relationship',
  relParent: 'Parent / Senior',
  relCellMember: 'Cell group member',
  relSpouse: 'Spouse / Child',
  relFriend: 'Fellow church member',
  relOther: 'Other',
  proxyPhone: 'Phone (optional)',
  proxyNotes: 'Mobility or assistance notes (optional)',
  proxyNotesPlaceholder: 'E.g. Uses wheelchair or needs front-seat space',
};

vi.mock('react-i18next', () => ({
  initReactI18next: { type: '3rdParty', init: vi.fn() },
  useTranslation: () => ({
    t: (k: string, vars?: Record<string, unknown>) => {
      const base = profileStrings[k];
      if (!base) return k;
      if (!vars) return base;
      return Object.entries(vars).reduce(
        (acc, [vk, vv]) => acc.replaceAll(`{${vk}}`, String(vv)),
        base,
      );
    },
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
  Field: ({ label, hint, children }: { label: string; hint?: React.ReactNode; children: React.ReactNode }) => (
    <label data-testid="field">{label}{children}{hint && <small>{hint}</small>}</label>
  ),
  FileInput: ({ label }: { label: string }) => (
    <label data-testid="fileinput">{label}</label>
  ),
  Modal: ({
    children,
    title,
    footer,
    onClose,
  }: {
    children?: React.ReactNode;
    title?: string;
    footer?: React.ReactNode;
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
      {footer && <div data-testid="modal-footer">{footer}</div>}
    </div>
  ),
  PageHeader: ({
    title,
    subtitle,
    back,
    actions,
  }: {
    title?: React.ReactNode;
    subtitle?: React.ReactNode;
    back?: string;
    actions?: React.ReactNode;
  }) => (
    <div data-testid="page-header" data-back={back}>
      <h1>{title}</h1>
      {subtitle && <span>{subtitle}</span>}
      {actions}
    </div>
  ),
  EmptyState: ({
    title,
    description,
    action,
  }: {
    icon?: React.ReactNode;
    title: React.ReactNode;
    description?: React.ReactNode;
    action?: React.ReactNode;
  }) => (
    <div data-testid="empty-state">
      <div>{title}</div>
      {description && <div>{description}</div>}
      {action}
    </div>
  ),
  IconButton: ({
    label,
    icon,
    onClick,
    disabled,
    className,
    ...rest
  }: {
    label: string;
    icon: React.ReactNode;
    onClick?: () => void;
    disabled?: boolean;
    className?: string;
    variant?: string;
    size?: string;
    [k: string]: unknown;
  }) => (
    <button
      type="button"
      aria-label={label}
      disabled={disabled}
      className={className}
      onClick={onClick}
      {...rest}
    >
      {icon}
    </button>
  ),
  ListSkeleton: () => <div data-testid="list-skeleton" />,
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
  _resetPushStateForTesting();
  mockCanLock = false;
  currentMe = baseMe;
  savedPlacesData = [];
  proxyRidersData = [];
  pushConfigData = undefined;
  notifPrefsData = null;
  pushSubsData = null;
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
    for (const label of ['My costs', 'Help centre']) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
    expect(screen.queryByText('My report')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /More activity/i }));
    expect(screen.getByText('My reimbursements')).toBeInTheDocument();
    expect(screen.getByText('My report')).toBeInTheDocument();
  });

  it('keeps ride and driving schedule above Account without duplicating Vehicle & driving settings', () => {
    renderProfile(baseMe);
    expect(screen.getByText('My ride history').closest('a')).toHaveAttribute('href', '/rides');
    const driveLink = screen.getByText('Driving & schedule').closest('a');
    expect(driveLink).toHaveAttribute('href', '/drive');
    expect(driveLink).toHaveTextContent('Car details, service areas & carpool');
    expect(screen.queryByText('Vehicle & driving settings')).not.toBeInTheDocument();

    const activityHeading = screen.getByText('Activity');
    const accountHeading = screen.getByText('Account');
    expect(activityHeading.compareDocumentPosition(accountHeading) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it('hides related pages shortcuts for pending accounts', () => {
    renderProfile({ ...baseMe, status: 'pending' });
    expect(screen.queryByText('My costs')).not.toBeInTheDocument();
    expect(screen.queryByText('Activity')).not.toBeInTheDocument();
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

  it('opens the avatar editor from the avatar (+ badge) and from personal details', () => {
    renderProfile(baseMe);
    const avatarBtn = screen.getByRole('button', { name: 'avatarEdit' });
    expect(avatarBtn.querySelector('.profile-avatar-badge')).toBeInTheDocument();
    expect(screen.queryByTestId('avatar-editor')).not.toBeInTheDocument();
    fireEvent.click(avatarBtn);
    expect(screen.getByTestId('avatar-editor')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'close editor' }));
    fireEvent.click(screen.getByRole('button', { name: /Personal details/i }));
    fireEvent.click(screen.getByRole('button', { name: 'avatarEdit' }));
    expect(screen.getByTestId('avatar-editor')).toBeInTheDocument();
  });

  it('renders Lock screen when canLock is enabled and Log out below the sections', () => {
    mockCanLock = true;
    renderProfile(baseMe);
    fireEvent.click(screen.getByRole('button', { name: /Lock screen now/i }));
    expect(lockMock).toHaveBeenCalledTimes(1);
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

  it('folds Calendar feed and PWA setup guide into Preferences instead of cluttering root /me when prompt unavailable', () => {
    _setInstalledForTesting(false);
    _setDeferredPromptForTesting(null);
    renderProfile(baseMe);

    expect(screen.queryByText('Add Grace Ride to Home Screen')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Add App to Home Screen/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Calendar feed/i })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Preferences/i }));
    expect(screen.getByRole('button', { name: /Calendar feed/i })).toBeInTheDocument();
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

  it('shows installed status badge inside Preferences when already installed', () => {
    _setInstalledForTesting(true);
    _setDeferredPromptForTesting(null);
    renderProfile(baseMe);

    expect(screen.queryByText('Add Grace Ride to Home Screen')).not.toBeInTheDocument();
    expect(screen.queryByText('Added to Home Screen')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /Preferences/i }));
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

  it('shows empty state when user has no saved addresses and adds first address as default', async () => {
    const postMock = vi.mocked(post);
    renderProfile(baseMe, '/me?section=places');

    expect(screen.getByText('No saved addresses yet')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /Add address/i }));

    const dlg = screen.getByRole('dialog', { name: 'Add address' });
    expect(dlg).toBeInTheDocument();
    const defaultCheckbox = screen.getByLabelText('Use as default address') as HTMLInputElement;
    expect(defaultCheckbox.checked).toBe(true);

    fireEvent.click(screen.getByRole('button', { name: 'Home' }));
    fireEvent.change(screen.getByRole('combobox', { name: 'Street address' }), { target: { value: '12 Jalan SS2' } });
    fireEvent.change(screen.getByLabelText('Area'), { target: { value: '1' } });
    fireEvent.change(screen.getByLabelText('Gate / pickup notes (optional)'), { target: { value: 'Gate B' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() =>
      expect(postMock).toHaveBeenCalledWith('/saved-places', {
        label: 'Home',
        area_id: 1,
        address: '12 Jalan SS2',
        gate_notes: 'Gate B',
        is_default: true,
      }),
    );
  });

  it('lists multiple saved addresses, promotes default via star button, edits and deletes via modal', async () => {
    const putMock = vi.mocked(put);
    const delMock = vi.mocked(del);
    savedPlacesData = [
      { id: 1, label: 'Home', area_id: 1, area_name: 'Petaling Jaya', address: '10 Jalan SS2', gate_notes: 'Gate 2', is_default: 1 },
      { id: 2, label: 'Work', area_id: 2, area_name: 'Subang Jaya', address: '88 Tower B', gate_notes: '', is_default: 0 },
    ];
    renderProfile(baseMe, '/me?section=places');

    expect(screen.getByText('Home')).toBeInTheDocument();
    expect(screen.getByText('Default')).toBeInTheDocument();
    expect(screen.getByText('Work')).toBeInTheDocument();

    // 1-tap default promotion
    fireEvent.click(screen.getByRole('button', { name: 'Set Work as default' }));
    await waitFor(() =>
      expect(putMock).toHaveBeenCalledWith('/saved-places/2', {
        label: 'Work',
        area_id: 2,
        address: '88 Tower B',
        gate_notes: '',
        is_default: true,
      }),
    );

    // Edit and save
    fireEvent.click(screen.getByRole('button', { name: 'Edit Work' }));
    expect(screen.getByRole('dialog', { name: 'Edit address' })).toBeInTheDocument();
    fireEvent.change(screen.getByRole('combobox', { name: 'Street address' }), { target: { value: '99 Tower C' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() =>
      expect(putMock).toHaveBeenCalledWith('/saved-places/2', {
        label: 'Work',
        area_id: 2,
        address: '99 Tower C',
        gate_notes: '',
        is_default: false,
      }),
    );

    // Edit and remove with confirmation
    vi.mocked(confirmMock).mockResolvedValueOnce(true);
    fireEvent.click(screen.getByRole('button', { name: 'Edit Home' }));
    fireEvent.click(screen.getByRole('button', { name: 'Remove' }));
    await waitFor(() => expect(delMock).toHaveBeenCalledWith('/saved-places/1'));
  });

  it('hides unconfigured WhatsApp/Telegram controls and shows availability notices', () => {
    pushConfigData = { whatsapp_configured: false, telegram_configured: false, telegram_bot_username: '' };
    renderProfile(baseMe, '/me?section=preferences');

    expect(screen.getByText('WhatsApp alerts are not enabled by the church office yet.')).toBeInTheDocument();
    expect(screen.getByText('Telegram alerts are not enabled by the church office yet.')).toBeInTheDocument();
    expect(screen.queryByPlaceholderText('123456789')).not.toBeInTheDocument();
  });

  it('renders guided Telegram bot link when channel and username are configured', () => {
    pushConfigData = { whatsapp_configured: true, telegram_configured: true, telegram_bot_username: 'GraceRideBot' };
    renderProfile(baseMe, '/me?section=preferences');

    const botLink = screen.getByRole('link', { name: 'Open @GraceRideBot in Telegram' });
    expect(botLink).toHaveAttribute('href', 'https://t.me/GraceRideBot');
    expect(screen.getByPlaceholderText('123456789')).toBeInTheDocument();
  });

  it('infers pickup area, shows missing-area/pin status, and captures GPS coords on saved address', async () => {
    const postMock = vi.mocked(post);
    postMock.mockClear();
    savedPlacesData = [
      { id: 10, label: 'Pinned', area_id: null, area_name: null, address: '5 Jalan Barat', gate_notes: '', lat: 3.11, lng: 101.64, is_default: 0 },
    ];
    const origGeo = navigator.geolocation;
    Object.defineProperty(navigator, 'geolocation', {
      configurable: true,
      value: {
        getCurrentPosition: (cb: (pos: { coords: { latitude: number; longitude: number } }) => void) =>
          cb({ coords: { latitude: 3.123456, longitude: 101.654321 } }),
      },
    });

    try {
      renderProfile(baseMe, '/me?section=places');
      expect(screen.getByText('Area not set')).toBeInTheDocument();
      expect(screen.getByText('Map pin saved')).toBeInTheDocument();

      fireEvent.click(screen.getByRole('button', { name: /Add address/i }));
      expect(screen.getByText(/Needed when booking a ride so volunteer drivers/i)).toBeInTheDocument();

      fireEvent.click(screen.getByRole('button', { name: 'Home' }));
      fireEvent.change(screen.getByRole('combobox', { name: 'Street address' }), {
        target: { value: '18 Jalan Utara, Subang Jaya' },
      });
      // Area "Subang Jaya" (id 2) is inferred automatically
      expect((screen.getByLabelText(/Area/i) as HTMLSelectElement).value).toBe('2');

      fireEvent.click(screen.getByRole('button', { name: /Use my current location/i }));
      expect(screen.getByText('GPS coordinates captured for driver routing.')).toBeInTheDocument();

      fireEvent.click(screen.getByRole('button', { name: 'Save' }));
      await waitFor(() =>
        expect(postMock).toHaveBeenCalledWith('/saved-places', {
          label: 'Home',
          area_id: 2,
          address: '18 Jalan Utara, Subang Jaya',
          gate_notes: '',
          is_default: false,
          lat: 3.123456,
          lng: 101.654321,
        }),
      );
    } finally {
      Object.defineProperty(navigator, 'geolocation', { configurable: true, value: origGeo });
    }
  });

  it('manages frequent proxy riders (add, edit, remove) under /me/family when active and hides for pending', async () => {
    const postMock = vi.mocked(post);
    const putMock = vi.mocked(put);
    const delMock = vi.mocked(del);
    postMock.mockClear();
    putMock.mockClear();
    delMock.mockClear();

    proxyRidersData = [
      { id: 7, name: 'Uncle Lim', phone: '012-1112222', relationship: 'parent', notes: 'Cane', area_id: 1, pickup_address: '10 SS2', gate_notes: 'Gate 1' },
    ];
    renderProfile(baseMe, '/me/family');
    expect(screen.getByText('Frequent riders')).toBeInTheDocument();
    expect(screen.getByText('Uncle Lim')).toBeInTheDocument();

    // Add new frequent rider
    fireEvent.click(screen.getByRole('button', { name: /Add frequent rider/i }));
    expect(screen.getByRole('dialog', { name: 'Add frequent rider' })).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Passenger name'), { target: { value: 'Auntie Mary' } });
    fireEvent.change(screen.getByLabelText('Phone (optional)'), { target: { value: '013-9998888' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() =>
      expect(postMock).toHaveBeenCalledWith('/proxy-riders', {
        name: 'Auntie Mary',
        phone: '013-9998888',
        relationship: 'parent',
        pickup_address: '',
        area_id: null,
        gate_notes: '',
        notes: '',
      }),
    );

    // Edit and remove existing rider
    vi.mocked(confirmMock).mockResolvedValueOnce(true);
    fireEvent.click(screen.getByRole('button', { name: 'Edit Uncle Lim' }));
    fireEvent.change(screen.getByLabelText('Passenger name'), { target: { value: 'Elder Lim' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() =>
      expect(putMock).toHaveBeenCalledWith(
        '/proxy-riders/7',
        expect.objectContaining({ name: 'Elder Lim' }),
      ),
    );

    fireEvent.click(screen.getByRole('button', { name: 'Edit Uncle Lim' }));
    fireEvent.click(screen.getByRole('button', { name: 'Remove' }));
    await waitFor(() => expect(delMock).toHaveBeenCalledWith('/proxy-riders/7'));

    // Pending accounts do not see ProxyRidersSection
    renderProfile({ ...baseMe, status: 'pending' }, '/me/family');
    expect(screen.getAllByText('Frequent riders')).toHaveLength(1);
  });

  it('shows Step 2 Allow notifications inside setup modal when installed and requests permission on tap', async () => {
    const origNotification = globalThis.Notification;
    const origPushManager = (window as unknown as { PushManager?: unknown }).PushManager;
    const requestPermission = vi.fn().mockImplementation(async () => {
      Object.defineProperty(globalThis, 'Notification', {
        configurable: true,
        writable: true,
        value: { permission: 'granted', requestPermission },
      });
      return 'granted' as NotificationPermission;
    });

    Object.defineProperty(globalThis, 'Notification', {
      configurable: true,
      writable: true,
      value: { permission: 'default', requestPermission },
    });
    Object.defineProperty(window, 'PushManager', {
      configurable: true,
      writable: true,
      value: function MockPushManager() {},
    });

    try {
      _setInstalledForTesting(true);
      _setDeferredPromptForTesting(null);
      renderProfile(baseMe, '/me?section=preferences');

      fireEvent.click(screen.getByRole('button', { name: /Add App to Home Screen/i }));
      expect(screen.getByText('Step 2: Allow notifications')).toBeInTheDocument();

      const enableBtn = screen.getByRole('button', { name: 'Enable Notifications' });
      fireEvent.click(enableBtn);

      await waitFor(() => expect(requestPermission).toHaveBeenCalledTimes(1));
      expect(await screen.findByText('Notifications are enabled on this device.')).toBeInTheDocument();
    } finally {
      _setInstalledForTesting(null);
      if (origNotification === undefined) {
        delete (globalThis as { Notification?: unknown }).Notification;
      } else {
        Object.defineProperty(globalThis, 'Notification', {
          configurable: true,
          writable: true,
          value: origNotification,
        });
      }
      if (origPushManager === undefined) {
        delete (window as unknown as { PushManager?: unknown }).PushManager;
      } else {
        Object.defineProperty(window, 'PushManager', {
          configurable: true,
          writable: true,
          value: origPushManager,
        });
      }
    }
  });

  it('shows iOS install-first push guidance in Preferences and opens setup modal', () => {
    const origUA = navigator.userAgent;
    const origNotification = globalThis.Notification;
    const origPushManager = (window as unknown as { PushManager?: unknown }).PushManager;

    Object.defineProperty(navigator, 'userAgent', {
      configurable: true,
      value: 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 Safari/604.1',
    });
    Object.defineProperty(globalThis, 'Notification', {
      configurable: true,
      writable: true,
      value: undefined,
    });
    Object.defineProperty(window, 'PushManager', {
      configurable: true,
      writable: true,
      value: undefined,
    });

    try {
      _setInstalledForTesting(false);
      _setDeferredPromptForTesting(null);
      renderProfile(baseMe, '/me?section=preferences');

      expect(
        screen.getByText(
          'On iPhone and iPad, add Grace Ride to your Home Screen first, then open the installed app and tap Enable Notifications.',
        ),
      ).toBeInTheDocument();

      fireEvent.click(screen.getByRole('button', { name: 'How to add to Home Screen' }));
      expect(screen.getByTestId('modal')).toBeInTheDocument();
      expect(screen.getByText('Add App to Home Screen (No App Store Needed)')).toBeInTheDocument();
    } finally {
      _setInstalledForTesting(null);
      Object.defineProperty(navigator, 'userAgent', { configurable: true, value: origUA });
      if (origNotification === undefined) {
        delete (globalThis as { Notification?: unknown }).Notification;
      } else {
        Object.defineProperty(globalThis, 'Notification', {
          configurable: true,
          writable: true,
          value: origNotification,
        });
      }
      if (origPushManager === undefined) {
        delete (window as unknown as { PushManager?: unknown }).PushManager;
      } else {
        Object.defineProperty(window, 'PushManager', {
          configurable: true,
          writable: true,
          value: origPushManager,
        });
      }
    }
  });

  it('warns when /push/test delivers to 0 devices, removes connected web device, and blocks non-numeric Telegram Chat ID', async () => {
    pushConfigData = {
      enabled: true,
      configured: true,
      subscription_count: 1,
      telegram_configured: true,
      telegram_bot_username: 'GraceTestBot',
    };
    pushSubsData = {
      web: [
        {
          id: 77,
          device_label: 'Chrome Desktop',
          endpoint_preview: 'https://fcm.googleapis.com/***',
          created_at: '2026-11-20 08:00:00',
          last_delivered_at: null,
        },
      ],
      mobile: [],
    };
    vi.mocked(post).mockResolvedValueOnce({ ok: true, delivered: 0, total: 1 });

    renderProfile(baseMe, '/me?section=preferences');

    fireEvent.click(screen.getByRole('button', { name: 'sendTestPush' }));
    expect(await screen.findByText('pushTestZeroDelivered')).toBeInTheDocument();

    expect(screen.getByText('Chrome Desktop')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'pushRemoveDevice' }));
    await waitFor(() => expect(del).toHaveBeenCalledWith('/push/subscriptions/77'));

    const tgInput = screen.getByPlaceholderText('123456789');
    fireEvent.change(tgInput, { target: { value: '@invalid_handle' } });
    fireEvent.click(screen.getByRole('button', { name: /^Save$/ }));
    expect(await screen.findByText('telegramInvalidId')).toBeInTheDocument();
    expect(put).not.toHaveBeenCalled();
  });
});
