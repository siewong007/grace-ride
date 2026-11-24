import { ReactNode, useEffect, useState } from 'react';
import { useLocation, useNavigate, useSearchParams } from 'react-router-dom';
import {
  BarChart3,
  Calendar,
  Camera,
  Check,
  ChevronDown,
  ChevronRight,
  ChevronUp,
  Copy,
  ExternalLink,
  Fingerprint,
  HandCoins,
  KeyRound,
  LifeBuoy,
  Lock,
  LogOut,
  MapPin,
  Plus,
  Receipt,
  RefreshCw,
  Shield,
  Sliders,
  Smartphone,
  Sparkles,
  Users,
  Wallet,
  Car,
  CarFront,
} from 'lucide-react';
import { del, post, put, useApi } from '../api';
import { useAuth } from '../auth';
import { parseGrokBotIcon } from '../avatarStyle';
import { useAutoLock } from '../components/AutoLock';
import { DisplayPreferences } from '../components/AppChrome';
import { Avatar, Button, ErrorBox, Field, Modal, Notice, PageHeader, Pill, SheetLink, useAction, useConfirm } from '../components/ui';
import { fmtAgo, roleLabel, useFormatT } from '../format';
import { clearDevicePasskey, passkeySupported, registerPasskey, withRecentSignIn } from '../webauthn';
import { WaitingRoom } from '../components/Onboarding';
import AvatarEditor from '../components/AvatarEditor';
import { PwaInstallModal } from '../components/PwaInstallModal';
import { usePwaInstall } from '../pwaInstall';
import { disableDevicePush, enablePushNotifications, getCurrentPushEndpoint, usePushNotifications } from '../pushNotifications';
import FamilySection from './family/FamilySection';
import ProxyRidersSection from './family/ProxyRidersSection';
import SavedAddressesSection from './SavedAddressesSection';
import type { SavedPlace } from '../types/domain';
import QRCode from 'qrcode';
import { useTranslation } from 'react-i18next';

type SectionKey = 'personal' | 'preferences' | 'places' | 'calendar' | 'security';

export default function Profile() {
  const { t } = useTranslation('profile');
  const ft = useFormatT();
  const confirm = useConfirm();
  const { me, refresh, can } = useAuth();
  // A too-old sign-in can be renewed in place only with a passkey this browser can use.
  const canReauth = passkeySupported() && (me?.passkey_count ?? 0) > 0;
  const recentSignIn = <T,>(fn: () => Promise<T>) => withRecentSignIn(fn, canReauth);
  const nav = useNavigate();
  const loc = useLocation();
  const { lock, canLock } = useAutoLock();
  const passkeys = useApi<{ id: number; label: string; created_at: string; last_used: string | null }[]>('/auth/passkeys');
  // The open section lives in the URL so the header back button and swipe-back both return to the list.
  const [searchParams] = useSearchParams();
  const sectionParam = searchParams.get('section');
  const open: SectionKey | null =
    (['personal', 'preferences', 'places', 'calendar', 'security'] as const).find((k) => k === sectionParam) ?? null;
  const savedPlacesApi = useApi<SavedPlace[]>(me && me.status !== 'pending' ? '/saved-places' : null);
  const pushCfg = useApi<{
    enabled?: boolean;
    configured?: boolean;
    vapid_public_key?: string;
    subscription_count?: number;
    whatsapp_configured?: boolean;
    telegram_configured?: boolean;
    telegram_bot_username?: string;
  }>(open === 'preferences' && me?.status !== 'pending' ? '/push/config' : null);
  const channelCfg = pushCfg.data && !Array.isArray(pushCfg.data) ? pushCfg.data : undefined;
  const notifPrefsApi = useApi<{
    push_opt_in?: boolean;
    whatsapp_opt_in?: boolean;
    telegram_chat_id?: string;
    telegram_blocked?: boolean;
    readiness_prompt_mins?: number;
    driver_remind_mins?: number;
    muted_categories?: string[];
    quiet_hours_start?: string;
    quiet_hours_end?: string;
    hide_lock_details?: boolean;
  }>(open === 'preferences' && me?.status !== 'pending' ? '/notifications/preferences' : null);
  const subsApi = useApi<{
    web?: {
      id: number;
      device_label: string;
      endpoint_preview: string;
      created_at: string;
      last_delivered_at?: string | null;
    }[];
    mobile?: {
      id: number;
      platform: string;
      provider: string;
      device_label: string;
      created_at: string;
      last_delivered_at?: string | null;
    }[];
  }>(open === 'preferences' && me?.status !== 'pending' ? '/push/subscriptions' : null);
  const notifPrefs =
    notifPrefsApi.data && !Array.isArray(notifPrefsApi.data) ? notifPrefsApi.data : undefined;
  const subsData = subsApi.data && !Array.isArray(subsApi.data) ? subsApi.data : undefined;
  const telegramBot = (channelCfg?.telegram_bot_username ?? '').replace(/[^a-zA-Z0-9_]/g, '');
  const [f, setF] = useState({
    name: me?.name ?? '',
    phone: me?.phone ?? '',
    optOut: me?.badges_opt_out ?? false,
  });
  const [readinessPrompt, setReadinessPrompt] = useState(me?.readiness_prompt_mins ?? 45);
  const [autoLockMins, setAutoLockMins] = useState(me?.auto_lock_mins ?? (parseInt(localStorage.getItem('gr_autolock_mins') || '0', 10) || 0));
  const [telegramChatId, setTelegramChatId] = useState(me?.telegram_chat_id ?? '');
  const [whatsappOptIn, setWhatsappOptIn] = useState(Boolean(me?.whatsapp_opt_in));
  const [pushOptIn, setPushOptIn] = useState(me?.push_opt_in ?? true);
  const [driverRemindMins, setDriverRemindMins] = useState(60);
  const [mutedCategories, setMutedCategories] = useState<string[]>([]);
  const [quietHoursStart, setQuietHoursStart] = useState('');
  const [quietHoursEnd, setQuietHoursEnd] = useState('');
  const [hideLockDetails, setHideLockDetails] = useState(false);
  const [telegramBlocked, setTelegramBlocked] = useState(false);
  const [tgFeedback, setTgFeedback] = useState('');
  const [tgBusy, setTgBusy] = useState(false);
  const [prefsError, setPrefsError] = useState('');

  useEffect(() => {
    if (me) {
      setF({
        name: me.name,
        phone: me.phone ?? '',
        optOut: me.badges_opt_out ?? false,
      });
      setReadinessPrompt(me.readiness_prompt_mins ?? 45);
      setTelegramChatId(me.telegram_chat_id ?? '');
      setWhatsappOptIn(Boolean(me.whatsapp_opt_in));
      setPushOptIn(me.push_opt_in ?? true);
    }
  }, [me]);

  useEffect(() => {
    if (notifPrefs) {
      setDriverRemindMins(notifPrefs.driver_remind_mins ?? 60);
      setMutedCategories(Array.isArray(notifPrefs.muted_categories) ? notifPrefs.muted_categories : []);
      setQuietHoursStart(notifPrefs.quiet_hours_start ?? '');
      setQuietHoursEnd(notifPrefs.quiet_hours_end ?? '');
      setHideLockDetails(Boolean(notifPrefs.hide_lock_details));
      setTelegramBlocked(Boolean(notifPrefs.telegram_blocked));
      if (notifPrefs.telegram_chat_id !== undefined) {
        setTelegramChatId(notifPrefs.telegram_chat_id);
      }
    }
  }, [notifPrefs]);
  const [pushStatus, setPushStatus] = useState('');
  const [pushBusy, setPushBusy] = useState(false);
  const [totp, setTotp] = useState<{ secret: string; url: string } | null>(null);
  const [showVerify2fa, setShowVerify2fa] = useState(false);
  const [showBackupCodes, setShowBackupCodes] = useState<string[] | null>(null);
  const [editAvatar, setEditAvatar] = useState(false);
  const [moreActivityOpen, setMoreActivityOpen] = useState(false);
  const pwa = usePwaInstall();
  const push = usePushNotifications();
  const [showInstallModal, setShowInstallModal] = useState(false);
  const { busy, error, run } = useAction();
  if (!me) return null;

  if (loc.pathname === '/me/family') {
    return (
      <div className="stack profile-page">
        <PageHeader title={t('family')} back="/me" />
        <FamilySection />
        {me.status !== 'pending' && can('rides') && (
          <>
            <div className="divider" />
            <ProxyRidersSection />
          </>
        )}
      </div>
    );
  }

  const handleInstallClick = async () => {
    if (pwa.canPrompt) {
      const outcome = await pwa.promptInstall();
      if (outcome === 'accepted') {
        return;
      }
    }
    setShowInstallModal(true);
  };

  const openSection = (section: SectionKey) => {
    setPushStatus('');
    nav(`/me?section=${section}`);
  };

  const isPending = me.status === 'pending';
  const isAdmin = me.privileged;
  const roleText = isPending ? t('pending') : isAdmin ? t('administrator') : t('member');
  const iconSz = 18;
  const avatarEl = (
    <Avatar name={me.name} fileId={me.photo_file_id} color={me.avatar_color} icon={me.avatar_icon} size="lg" />
  );
  const readiness = me.readiness_prompt_mins ?? 45;
  const prefsHint = [
    me.push_opt_in === false ? t('pushOffShort') : t('pushOnShort'),
    readiness > 0 ? t('remindShort', { mins: readiness }) : t('remindOffShort'),
  ].join(' · ');
  const securityHint = [
    t('passkeyCount', { count: me.passkey_count }),
    me.totp_enabled ? t('twoFaOnShort') : t('twoFaOffShort'),
  ].join(' · ');
  const savedPlaces = Array.isArray(savedPlacesApi.data) ? savedPlacesApi.data : [];
  const placesHint =
    savedPlaces.length === 0
      ? t('savedAddressesHint')
      : savedPlaces.length === 1
        ? savedPlaces[0].label
        : t('savedAddressesCountHint', { label: savedPlaces[0].label, count: savedPlaces.length });

  const savePersonal = () => put('/me', { name: f.name, phone: f.phone }).then(refresh);
  const savePreferences = async () => {
    setPrefsError('');
    const cleanedTg = telegramChatId.trim();
    if (cleanedTg && !/^-?\d{5,20}$/.test(cleanedTg)) {
      setPrefsError(t('telegramInvalidId'));
      return;
    }
    await put('/me', {
      badges_opt_out: f.optOut,
      readiness_prompt_mins: readinessPrompt,
      telegram_chat_id: cleanedTg,
      whatsapp_opt_in: whatsappOptIn,
      push_opt_in: pushOptIn,
    });
    if (!isPending) {
      await put('/notifications/preferences', {
        push_opt_in: pushOptIn,
        whatsapp_opt_in: whatsappOptIn,
        telegram_chat_id: cleanedTg,
        readiness_prompt_mins: readinessPrompt,
        driver_remind_mins: driverRemindMins,
        muted_categories: mutedCategories,
        quiet_hours_start: quietHoursStart.trim(),
        quiet_hours_end: quietHoursEnd.trim(),
        hide_lock_details: hideLockDetails,
      });
    }
    await refresh();
    notifPrefsApi.reload();
  };

  const deniedPushCopy =
    push.platform === 'ios'
      ? t('pushDeniedIos')
      : push.platform === 'android'
        ? t('pushDeniedAndroid')
        : t('pushDeniedDesktop');

  const enableBrowserPush = async () => {
    setPushStatus('');
    setPushBusy(true);
    try {
      const res = await enablePushNotifications();
      if (res.ok) {
        setPushOptIn(true);
        void refresh();
        pushCfg.reload();
        subsApi.reload();
        setPushStatus(t(res.mode === 'push' ? 'pushEnabled' : 'pushEnabledLocal'));
      } else if (res.reason === 'needs_install_ios') {
        setPushStatus(t('pushNeedsInstallIos'));
      } else if (res.reason === 'unsupported') {
        setPushStatus(t('pushUnsupported'));
      } else if (res.reason === 'denied') {
        setPushStatus(deniedPushCopy);
      } else if (res.reason === 'dismissed') {
        setPushStatus(t('pushDismissed'));
      } else if (res.reason === 'subscribe_failed') {
        setPushStatus(t('pushSubscribeFailed'));
      }
    } finally {
      setPushBusy(false);
    }
  };

  const sendTestPush = async () => {
    setPushStatus('');
    setPushBusy(true);
    try {
      const res = await post<{ ok?: boolean; delivered?: number; total?: number }>('/push/test');
      if (typeof res?.delivered === 'number' && res.delivered === 0) {
        setPushStatus(t('pushTestZeroDelivered'));
      } else {
        setPushStatus(t('pushTestSent'));
      }
      subsApi.reload();
    } catch {
      setPushStatus(t('pushTestFailed'));
    } finally {
      setPushBusy(false);
    }
  };

  const disableBrowserPush = async () => {
    setPushStatus('');
    setPushBusy(true);
    try {
      await disableDevicePush();
      pushCfg.reload();
      subsApi.reload();
      setPushStatus(t('pushDisabledDevice'));
    } catch {
      setPushStatus(t('pushTestFailed'));
    } finally {
      setPushBusy(false);
    }
  };

  const removePushSub = async (subId: number) => {
    setPushStatus('');
    setPushBusy(true);
    try {
      await del(`/push/subscriptions/${subId}`);
      pushCfg.reload();
      subsApi.reload();
    } catch {
      setPushStatus(t('pushRemoveDeviceFailed'));
    } finally {
      setPushBusy(false);
    }
  };

  const handleTelegramConnect = async () => {
    setTgFeedback('');
    setTgBusy(true);
    try {
      const res = await post<{ url?: string }>('/push/telegram/link-token');
      if (res?.url) {
        window.open(res.url, '_blank', 'noopener,noreferrer');
      }
    } catch {
      setTgFeedback(t('telegramTestFailed'));
    } finally {
      setTgBusy(false);
    }
  };

  const handleTelegramTest = async () => {
    setTgFeedback('');
    setTgBusy(true);
    try {
      const res = await post<{ delivered?: boolean; error?: string }>('/push/telegram/test');
      if (res?.delivered) {
        setTelegramBlocked(false);
        setTgFeedback(t('telegramTestSent'));
      } else {
        setTgFeedback(res?.error || t('telegramTestFailed'));
      }
    } catch {
      setTgFeedback(t('telegramTestFailed'));
    } finally {
      setTgBusy(false);
    }
  };

  const toggleMutedCategory = (cat: string) => {
    setMutedCategories((prev) =>
      prev.includes(cat) ? prev.filter((c) => c !== cat) : [...prev, cat],
    );
  };

  const handleLogout = () =>
    run(async () => {
      if (push.isSubscribed) {
        const endpoint = await getCurrentPushEndpoint().catch(() => '');
        await disableDevicePush().catch(() => {});
        await (endpoint
          ? post('/auth/logout', { push_endpoint: endpoint })
          : post('/auth/logout'));
      } else {
        await post('/auth/logout');
      }
      await refresh();
      nav('/login');
    }, t('loggedOut'));

  const handleLogoutAll = async () => {
    const ok = await confirm({
      title: t('logoutAll'),
      body: t('logoutAllConfirm'),
      confirmLabel: t('logoutAll'),
      tone: 'danger',
    });
    if (ok) {
      run(async () => {
        await post('/auth/logout-all');
        await refresh();
        nav('/login');
      });
    }
  };

  const primaryActivity = [
    can('rides') && { to: '/rides', label: t('myRideHistory'), icon: <Car size={iconSz} /> },
    can('drive') && {
      to: '/drive',
      label: t('myDrivingHistory'),
      hint: t('driverSettingsHint'),
      icon: <CarFront size={iconSz} />,
    },
    can('costs') && { to: '/costs', label: t('myCosts'), icon: <Wallet size={iconSz} /> },
    can('help') && { to: '/help', label: t('myHelp'), icon: <LifeBuoy size={iconSz} /> },
  ].filter(Boolean) as { to: string; label: string; hint?: string; icon: ReactNode }[];

  const secondaryActivity = [
    (can('settlements') || can('admin_bills')) && { to: '/settlements', label: t('settlements'), icon: <HandCoins size={iconSz} /> },
    can('reimbursements') && { to: '/reimbursements', label: t('myReimb'), icon: <Receipt size={iconSz} /> },
    can('report') && { to: '/report', label: t('myReport'), icon: <BarChart3 size={iconSz} /> },
  ].filter(Boolean) as { to: string; label: string; icon: ReactNode }[];

  const installRow = (
    <SheetLink
      icon={pwa.isInstalled ? <Check size={iconSz} /> : <Smartphone size={iconSz} />}
      label={t('installApp')}
      hint={
        pwa.isInstalled
          ? t('installAppInstalledDesc')
          : pwa.canPrompt
            ? t('installAppOneClickPrompt')
            : t('installAppDesc')
      }
      badge={pwa.isInstalled ? <Pill tone="success">{t('installAppInstalled')}</Pill> : undefined}
      aria-haspopup="dialog"
      onClick={handleInstallClick}
    />
  );

  const sheetTitle: Record<SectionKey, string> = {
    personal: t('personalInfo'),
    preferences: t('preferences'),
    places: t('savedAddresses'),
    calendar: t('calendarFeed'),
    security: t('security'),
  };

  return (
    <div className="stack profile-page profile-hub">
      {!open && (
        <>
      <section className="profile-identity-header" aria-label={t('account')}>
        <button
          type="button"
          className="profile-avatar-btn"
          aria-label={t('avatarEdit')}
          onClick={() => setEditAvatar(true)}
        >
          {avatarEl}
          <span className="profile-avatar-badge" aria-hidden="true">
            <Plus size={14} strokeWidth={2.5} />
          </span>
        </button>
        <button
          type="button"
          className="profile-identity-btn"
          aria-haspopup="dialog"
          onClick={() => openSection('personal')}
        >
          <span className="sr-only">{t('personalInfo')} </span>
          <div className="grow min-w-0">
            <h1 className="ellipsis">{me.name}</h1>
            <div className="profile-identity-meta">
              {isPending ? (
                <Pill tone="warning">{roleText}</Pill>
              ) : (
                <>
                  <span>{roleText}</span>
                  {me.groups.slice(0, 3).map((g) => (
                    <span key={g.id}> · {g.name}</span>
                  ))}
                </>
              )}
            </div>
            <div className="sheet-link-hint">{me.phone || t('personalHint')}</div>
          </div>
          <ChevronRight size={18} className="muted sheet-link-chevron" aria-hidden="true" />
        </button>
      </section>

      {isPending && <WaitingRoom />}

      <div className="stack">
        {!isPending && (primaryActivity.length > 0 || secondaryActivity.length > 0) && (
          <section className="stack-xs" aria-label={t('activitySection')}>
            <div className="section-label">{t('activitySection')}</div>
            <div className="list">
              {primaryActivity.map((n) => (
                <SheetLink key={n.to} to={n.to} icon={n.icon} label={n.label} hint={n.hint} />
              ))}
              {secondaryActivity.length > 0 && (
                <>
                  <button
                    type="button"
                    className="item admin-sheet-link profile-more-toggle"
                    aria-expanded={moreActivityOpen}
                    onClick={() => setMoreActivityOpen((v) => !v)}
                  >
                    <span className="grow">
                      {moreActivityOpen
                        ? t('hideMoreActivity')
                        : t('moreActivity', { count: secondaryActivity.length })}
                    </span>
                    {moreActivityOpen ? (
                      <ChevronUp size={18} className="sheet-link-chevron" aria-hidden="true" />
                    ) : (
                      <ChevronDown size={18} className="sheet-link-chevron" aria-hidden="true" />
                    )}
                  </button>
                  {moreActivityOpen &&
                    secondaryActivity.map((n) => (
                      <SheetLink key={n.to} to={n.to} icon={n.icon} label={n.label} />
                    ))}
                </>
              )}
            </div>
          </section>
        )}

        <section className="stack-xs" aria-label={t('account')}>
          <div className="section-label">{t('account')}</div>
          <div className="list">
            <SheetLink
              icon={<Sparkles size={iconSz} />}
              label={t('openGrokStudio')}
              hint={t('openGrokStudioHint')}
              to={(() => {
                const gb = parseGrokBotIcon(me.avatar_icon);
                return gb
                  ? `/studio?shape=${gb.shape}&color=${gb.color}&face=${gb.face}`
                  : '/studio';
              })()}
            />
            <SheetLink
              icon={<Sliders size={iconSz} />}
              label={t('preferences')}
              hint={prefsHint}
              aria-haspopup="dialog"
              onClick={() => openSection('preferences')}
            />
            {!isPending && (
              <SheetLink
                icon={<MapPin size={iconSz} />}
                label={t('savedAddresses')}
                hint={placesHint}
                aria-haspopup="dialog"
                onClick={() => openSection('places')}
              />
            )}
            <SheetLink
              icon={<Users size={iconSz} />}
              label={t('family')}
              to="/me/family"
            />
            <SheetLink
              icon={<Shield size={iconSz} />}
              label={t('security')}
              hint={securityHint}
              aria-haspopup="dialog"
              onClick={() => openSection('security')}
            />
            {pwa.canPrompt && !pwa.isInstalled && installRow}
          </div>
        </section>
      </div>

      <div className="list">
        {canLock && (
          <SheetLink
            icon={<Lock size={iconSz} aria-hidden="true" />}
            label={t('lockNow')}
            chevron={false}
            onClick={lock}
          />
        )}
        <SheetLink
          icon={<LogOut size={iconSz} aria-hidden="true" />}
          label={t('logout')}
          chevron={false}
          onClick={handleLogout}
        />
      </div>
        </>
      )}

      {open === 'places' && (
        <SavedAddressesSection
          places={savedPlaces}
          loading={savedPlacesApi.loading}
          reloadPlaces={savedPlacesApi.reload}
        />
      )}

      {open && open !== 'places' && (
        <>
          <PageHeader title={sheetTitle[open]} back="/me" />
          {open === 'personal' && (
            <div className="stack">
              <div className="row align-center gap-md">
                <Avatar name={me.name} fileId={me.photo_file_id} color={me.avatar_color} icon={me.avatar_icon} size="lg" />
                <div className="stack-xs">
                  <div className="bold">{t('photo')}</div>
                  <Button variant="secondary" size="sm" onClick={() => setEditAvatar(true)}>
                    <Camera size={16} aria-hidden="true" />
                    {t('avatarEdit')}
                  </Button>
                </div>
              </div>
              <form className="stack" onSubmit={(e) => {
                e.preventDefault();
                run(savePersonal, t('saved'));
              }}>
                <Field label={t('name')}>
                  <input
                    className="input"
                    required
                    autoComplete="name"
                    value={f.name}
                    onChange={(e) => setF({ ...f, name: e.target.value })}
                  />
                </Field>
                <Field label={t('phone')} hint={t('phoneHint')}>
                  <input
                    className="input"
                    type="tel"
                    autoComplete="tel"
                    placeholder={t('phonePlaceholder')}
                    value={f.phone}
                    onChange={(e) => setF({ ...f, phone: e.target.value })}
                  />
                </Field>
                <div className="btn-row">
                  <Button variant="primary" type="submit" loading={busy}>
                    {t('save')}
                  </Button>
                </div>
              </form>

              <div className="divider" />
              <div className="stack-sm">
                <div className="row between break-all">
                  <strong>{t('email')}</strong>
                  <span className="muted">{me.email}</span>
                </div>
                <div className="bold small">{t('status')}</div>
                <div className="row wrap gap-sm">
                  <Pill tone={isPending ? 'warning' : 'success'}>
                    {isPending ? t('pending') : t('member')}
                  </Pill>
                  {me.roles.map((r) => (
                    <Pill key={r} tone="primary">
                      {roleLabel(r, ft, me.role_names)}
                    </Pill>
                  ))}
                  {me.groups.map((g) => (
                    <Pill key={g.id}>{g.name}</Pill>
                  ))}
                </div>
              </div>
              <ErrorBox error={prefsError || error} />
            </div>
          )}

          {open === 'preferences' && (
            <div className="stack">
              <div className="stack-sm">
                <div className="bold">{t('readinessPromptLabel')}</div>
                <p className="muted small">{t('readinessPromptHint')}</p>
                <select
                  className="input"
                  value={readinessPrompt}
                  onChange={(e) => setReadinessPrompt(Number(e.target.value))}
                >
                  <option value={30}>{t('mins30')}</option>
                  <option value={45}>{t('mins45')}</option>
                  <option value={60}>{t('mins60')}</option>
                  <option value={0}>{t('promptDisabled')}</option>
                </select>
              </div>
              <div className="divider" />
              <label className={`check ${f.optOut ? 'on' : ''}`}>
                <input type="checkbox" checked={f.optOut} onChange={(e) => setF({ ...f, optOut: e.target.checked })} /> {t('optOut')}
              </label>
              <div className="divider" />
              <div className="stack-sm">
                <div className="bold">{t('alertChannels')}</div>
                <label className={`check ${pushOptIn ? 'on' : ''}`}>
                  <input type="checkbox" checked={pushOptIn} onChange={(e) => setPushOptIn(e.target.checked)} /> {t('pushOptIn')}
                </label>
                {(channelCfg?.subscription_count ?? 0) > 0 && (
                  <p className="muted tiny">{t('pushConnectedCount', { count: channelCfg?.subscription_count ?? 0 })}</p>
                )}
                {push.availability === 'needs_install_ios' && (
                  <Notice tone="info">
                    <div className="stack-xs">
                      <p className="small">{t('pushNeedsInstallIos')}</p>
                      <div>
                        <Button variant="secondary" size="sm" onClick={() => setShowInstallModal(true)}>
                          {t('pushInstallGuideAction')}
                        </Button>
                      </div>
                    </div>
                  </Notice>
                )}
                <div className="row wrap gap-xs">
                  <Button variant="secondary" size="sm" disabled={pushBusy} onClick={enableBrowserPush}>
                    {t('enableBrowserPush')}
                  </Button>
                  {(channelCfg?.subscription_count ?? 0) > 0 && (
                    <>
                      <Button variant="secondary" size="sm" disabled={pushBusy} onClick={sendTestPush}>
                        {t('sendTestPush')}
                      </Button>
                      {push.isSubscribed === true && (
                        <Button variant="ghost" size="sm" disabled={pushBusy} onClick={disableBrowserPush}>
                          {t('disableBrowserPush')}
                        </Button>
                      )}
                    </>
                  )}
                </div>
                {pushStatus && <p className="muted tiny">{pushStatus}</p>}
                {!pushStatus && push.availability === 'denied' && (
                  <p className="muted tiny">{deniedPushCopy}</p>
                )}
                {!isPending &&
                  subsData &&
                  ((subsData.web?.length ?? 0) > 0 || (subsData.mobile?.length ?? 0) > 0) && (
                    <div className="stack-xs">
                      <div className="small bold">{t('pushConnectedDevices')}</div>
                      {(subsData.web ?? []).map((sub) => (
                        <div key={`web-${sub.id}`} className="row between items-center gap-xs small">
                          <div className="min-w-0">
                            <span className="bold">{sub.device_label}</span>
                            <span className="muted tiny">
                              {' '}
                              · {t('pushDeviceWeb')} · {sub.endpoint_preview}
                              {sub.last_delivered_at
                                ? ` · ${t('pushLastDelivered', { when: fmtAgo(sub.last_delivered_at, ft) })}`
                                : ''}
                            </span>
                          </div>
                          <Button
                            variant="ghost"
                            size="sm"
                            disabled={pushBusy}
                            onClick={() => void removePushSub(sub.id)}
                          >
                            {t('pushRemoveDevice')}
                          </Button>
                        </div>
                      ))}
                      {(subsData.mobile ?? []).map((sub) => (
                        <div key={`mob-${sub.id}`} className="row between items-center gap-xs small">
                          <div className="min-w-0">
                            <span className="bold">{sub.device_label}</span>
                            <span className="muted tiny">
                              {' '}
                              · {t('pushDeviceMobile')} ({sub.platform})
                              {sub.last_delivered_at
                                ? ` · ${t('pushLastDelivered', { when: fmtAgo(sub.last_delivered_at, ft) })}`
                                : ''}
                            </span>
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                {channelCfg?.whatsapp_configured === false ? (
                  <p className="muted tiny">{t('whatsappNotConfigured')}</p>
                ) : (
                  <label className={`check ${whatsappOptIn ? 'on' : ''}`}>
                    <input type="checkbox" checked={whatsappOptIn} onChange={(e) => setWhatsappOptIn(e.target.checked)} /> {t('whatsappOptIn')}
                  </label>
                )}
                {channelCfg?.telegram_configured === false ? (
                  <p className="muted tiny">{t('telegramNotConfigured')}</p>
                ) : (
                  <div className="stack-xs">
                    {telegramBlocked && (
                      <Notice tone="warning">{t('telegramBlockedBadge')}</Notice>
                    )}
                    <div className="row wrap gap-xs">
                      {!isPending && (
                        <Button
                          variant="secondary"
                          size="sm"
                          disabled={tgBusy}
                          onClick={() => void handleTelegramConnect()}
                        >
                          {t('telegramConnectOneTap')}
                        </Button>
                      )}
                      {telegramBot && (
                        <a
                          className="btn secondary sm"
                          href={`https://t.me/${telegramBot}`}
                          target="_blank"
                          rel="noopener noreferrer"
                        >
                          <ExternalLink size={15} aria-hidden="true" />
                          {t('telegramOpenBot', { bot: telegramBot })}
                        </a>
                      )}
                      {!isPending && telegramChatId.trim() && (
                        <Button
                          variant="ghost"
                          size="sm"
                          disabled={tgBusy}
                          onClick={() => void handleTelegramTest()}
                        >
                          {t('telegramTest')}
                        </Button>
                      )}
                    </div>
                    {tgFeedback && <p className="muted tiny" role="status">{tgFeedback}</p>}
                    <Field
                      label={t('telegramChatId')}
                      hint={telegramBot ? t('telegramGuidedHint') : t('telegramChatIdHint')}
                    >
                      <input
                        className="input"
                        value={telegramChatId}
                        onChange={(e) => setTelegramChatId(e.target.value)}
                        placeholder="123456789"
                      />
                    </Field>
                  </div>
                )}
              </div>
              {!isPending && (
                <>
                  <div className="divider" />
                  <div className="stack-sm">
                    <div className="bold">{t('driverRemindLabel')}</div>
                    <p className="muted small">{t('driverRemindHint')}</p>
                    <div className="notif-filter-row">
                      {([
                        { mins: 30, key: 'mins30' },
                        { mins: 60, key: 'mins60' },
                        { mins: 90, key: 'mins90' },
                        { mins: 120, key: 'mins120' },
                      ] as const).map((opt) => (
                        <button
                          key={opt.mins}
                          type="button"
                          className={`notif-chip ${driverRemindMins === opt.mins ? 'active' : ''}`}
                          onClick={() => setDriverRemindMins(opt.mins)}
                        >
                          {t(opt.key)}
                        </button>
                      ))}
                    </div>
                  </div>
                  <div className="stack-sm">
                    <div className="bold">{t('quietHoursTitle')}</div>
                    <p className="muted small">{t('quietHoursHint')}</p>
                    <div className="form-grid two">
                      <Field label={t('quietHoursStart')}>
                        <input
                          type="time"
                          className="input"
                          value={quietHoursStart}
                          onChange={(e) => setQuietHoursStart(e.target.value)}
                        />
                      </Field>
                      <Field label={t('quietHoursEnd')}>
                        <input
                          type="time"
                          className="input"
                          value={quietHoursEnd}
                          onChange={(e) => setQuietHoursEnd(e.target.value)}
                        />
                      </Field>
                    </div>
                  </div>
                  <label className={`check ${hideLockDetails ? 'on' : ''}`}>
                    <input
                      type="checkbox"
                      checked={hideLockDetails}
                      onChange={(e) => setHideLockDetails(e.target.checked)}
                    />{' '}
                    {t('hideLockDetails')}
                  </label>
                  <div className="stack-sm">
                    <div className="bold">{t('mutedCategoriesTitle')}</div>
                    <p className="muted small">{t('mutedCategoriesHint')}</p>
                    <div className="row wrap gap-xs">
                      {([
                        { key: 'ride', labelKey: 'catRide' },
                        { key: 'event', labelKey: 'catEvent' },
                        { key: 'cost', labelKey: 'catCost' },
                        { key: 'help', labelKey: 'catHelp' },
                        { key: 'family', labelKey: 'catFamily' },
                        { key: 'admin', labelKey: 'catAdmin' },
                        { key: 'announcement', labelKey: 'catAnnouncement' },
                      ] as const).map((c) => {
                        const checked = mutedCategories.includes(c.key);
                        return (
                          <label key={c.key} className={`check ${checked ? 'on' : ''}`}>
                            <input
                              type="checkbox"
                              checked={checked}
                              onChange={() => toggleMutedCategory(c.key)}
                            />{' '}
                            {t(c.labelKey)}
                          </label>
                        );
                      })}
                    </div>
                  </div>
                </>
              )}
              <div className="divider" />
              <DisplayPreferences />
              <div className="btn-row">
                <Button variant="primary" onClick={() => run(savePreferences, t('saved'))} loading={busy}>
                  {t('save')}
                </Button>
              </div>
              <ErrorBox error={prefsError || error} />
              <div className="divider" />
              <div className="list">
                {!isPending && (
                  <SheetLink
                    icon={<Calendar size={iconSz} />}
                    label={t('calendarFeed')}
                    hint={t('calendarFeedShort')}
                    aria-haspopup="dialog"
                    onClick={() => openSection('calendar')}
                  />
                )}
                {installRow}
              </div>
            </div>
          )}

          {open === 'calendar' && <CalendarFeedSection />}

          {open === 'security' && (
            <div className="stack">
              <div className="stack-sm">
                <div className="bold">{t('passkeys')}</div>
                {passkeys.data?.length ? (
                  <div className="passkey-list stack-sm">
                    {passkeys.data.map((p) => (
                      <div key={p.id} className="passkey-row row between">
                        <div>
                          <div className="bold">{p.label}</div>
                          <div className="muted tiny">
                            {t('passkeyLabel', { date: fmtAgo(p.created_at, ft) })}
                            {p.last_used && ` · ${t('lastUsed', { date: fmtAgo(p.last_used, ft) })}`}
                          </div>
                        </div>
                        <Button
                          variant="danger"
                          size="sm"
                          onClick={async () => {
                            const ok = await confirm({
                              title: t('remove'),
                              body: t('removeConfirm'),
                              confirmLabel: t('remove'),
                              tone: 'danger',
                            });
                            if (ok) run(() => recentSignIn(() => del(`/auth/passkeys/${p.id}`)).then(() => {
                              if (passkeys.data?.length === 1) clearDevicePasskey();
                            }).then(passkeys.reload).then(refresh), t('removed'));
                          }}
                        >
                          {t('remove')}
                        </Button>
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="muted">{t('none')}</p>
                )}
                <Button
                  variant="primary"
                  disabled={busy || !passkeySupported()}
                  onClick={() => run(() => recentSignIn(() => registerPasskey(navigator.platform || t('myPhone'), me.email)).then(passkeys.reload).then(refresh), t('addPasskeyDone'))}
                >
                  <Fingerprint size={18} />
                  {t('addPasskey')}
                </Button>
              </div>

              {me.status !== 'pending' && (
                <>
                  <div className="divider" />
                  <div className="stack-sm">
                    <div className="row between">
                      <strong>{t('totp')}</strong>
                      <Pill tone={me.totp_enabled ? 'success' : 'neutral'}>
                        {me.totp_enabled ? t('totpEnabled') : t('totpDisabled')}
                      </Pill>
                    </div>
                    <p className="muted small">{t('totpNote')}</p>
                    {!me.totp_enabled ? (
                      <Button
                        variant="secondary"
                        disabled={busy}
                        onClick={() => run(async () => setTotp(await recentSignIn(() => post<{ secret: string; url: string }>('/auth/totp/setup'))))}
                      >
                        {t('setupTotp')}
                      </Button>
                    ) : (
                      <div className="btn-row">
                        <Button
                          variant="secondary"
                          size="sm"
                          disabled={busy}
                          onClick={async () => {
                            const ok = await confirm({
                              title: t('regenBackupCodes'),
                              body: t('regenBackupCodesConfirm'),
                              confirmLabel: t('regenBackupCodes'),
                              tone: 'warning',
                            });
                            if (ok) {
                              run(async () => {
                                const res = await recentSignIn(() =>
                                  post<{ ok: boolean; backup_codes: string[] }>('/auth/totp/backup-codes/regenerate'),
                                );
                                setShowBackupCodes(res.backup_codes);
                                await refresh();
                              }, t('regenBackupCodesDone'));
                            }
                          }}
                        >
                          <RefreshCw size={16} />
                          {t('regenBackupCodes')}
                        </Button>
                        {me.privileged && !me.mfa_ok && (
                          <Button variant="primary" size="sm" onClick={() => setShowVerify2fa(true)}>
                            <KeyRound size={16} />
                            {t('verify2fa')}
                          </Button>
                        )}
                      </div>
                    )}
                  </div>

                  <div className="divider" />
                  <div className="stack-sm">
                    <div className="bold">{t('autoLock')}</div>
                    <p className="muted small">{t('autoLockDesc')}</p>
                    <div className="row wrap gap-sm">
                      <select
                        className="input"
                        style={{ maxWidth: 220 }}
                        value={autoLockMins}
                        onChange={async (e) => {
                          const val = Number(e.target.value);
                          setAutoLockMins(val);
                          localStorage.setItem('gr_autolock_mins', String(val));
                          await run(() => put('/me', { auto_lock_mins: val }).then(refresh), t('saved'));
                        }}
                      >
                        <option value={0}>{t('autoLockDefault', { mins: me.session_timeout_mins ?? 15 })}</option>
                        <option value={-1}>{t('autoLockNever')}</option>
                        <option value={15}>{t('autoLock15m')}</option>
                        <option value={30}>{t('autoLock30m')}</option>
                        <option value={60}>{t('autoLock60m')}</option>
                      </select>
                      {canLock && (
                        <Button variant="ghost" size="sm" onClick={lock}>
                          <Lock size={16} />
                          {t('lockNow')}
                        </Button>
                      )}
                    </div>
                    {!canLock && <p className="muted small">{t('autoLockNeedsPasskey')}</p>}
                  </div>

                  <div className="divider" />
                  <div className="stack-sm">
                    <Button variant="ghost" onClick={handleLogoutAll}>
                      <LogOut size={16} />
                      {t('logoutAll')}
                    </Button>
                  </div>
                </>
              )}

              <ErrorBox error={prefsError || error} />
            </div>
          )}
        </>
      )}

      {totp && (
        <TotpModal
          secret={totp.secret}
          url={totp.url}
          onClose={() => setTotp(null)}
          onDone={() => {
            setTotp(null);
            refresh();
          }}
        />
      )}
      {editAvatar && (
        <AvatarEditor
          name={me.name}
          current={me}
          onClose={() => setEditAvatar(false)}
          onSaved={refresh}
        />
      )}
      {showBackupCodes && (
        <BackupCodesModal
          codes={showBackupCodes}
          onClose={() => setShowBackupCodes(null)}
        />
      )}
      {showVerify2fa && (
        <Verify2faModal
          onClose={() => setShowVerify2fa(false)}
          onDone={() => {
            setShowVerify2fa(false);
            refresh();
          }}
        />
      )}
      {showInstallModal && (
        <PwaInstallModal
          onClose={() => setShowInstallModal(false)}
          onNotificationsEnabled={() => {
            setPushOptIn(true);
            pushCfg.reload();
            void refresh();
          }}
        />
      )}
    </div>
  );
}

function BackupCodesModal({
  codes,
  onClose,
}: {
  codes: string[];
  onClose: () => void;
}) {
  const { t } = useTranslation('profile');
  const [copied, setCopied] = useState(false);

  const copy = () => {
    navigator.clipboard.writeText(codes.join('\n'));
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <Modal title={t('backupCodesTitle')} onClose={onClose}>
      <p className="muted small">{t('backupCodesDesc')}</p>
      <div className="backup-codes-grid">
        {codes.map((c) => (
          <div key={c} className="backup-code-item">
            {c}
          </div>
        ))}
      </div>
      <div className="btn-row">
        <Button variant="secondary" onClick={copy}>
          <Copy size={16} />
          {copied ? t('backupCodesCopied') : t('copyBackupCodes')}
        </Button>
        <Button variant="primary" onClick={onClose}>
          {t('save')}
        </Button>
      </div>
    </Modal>
  );
}

function TotpModal({
  secret,
  url,
  onClose,
  onDone,
}: {
  secret: string;
  url: string;
  onClose: () => void;
  onDone: () => void;
}) {
  const { t } = useTranslation('profile');
  const [code, setCode] = useState('');
  const [backupCodes, setBackupCodes] = useState<string[] | null>(null);
  const { busy, error, run } = useAction();
  const [copied, setCopied] = useState(false);
  const [secretCopied, setSecretCopied] = useState(false);
  const [qr, setQr] = useState<string | null>(null);

  // The QR is drawn in the browser from the otpauth link, so the secret never goes to a third party.
  useEffect(() => {
    let live = true;
    QRCode.toDataURL(url, { margin: 1, width: 192, errorCorrectionLevel: 'M' })
      .then((d) => live && setQr(d))
      .catch(() => live && setQr(null));
    return () => {
      live = false;
    };
  }, [url]);

  if (backupCodes) {
    const copy = () => {
      navigator.clipboard.writeText(backupCodes.join('\n'));
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    };
    return (
      <Modal title={t('backupCodesTitle')} onClose={onDone}>
        <p className="muted small">{t('backupCodesDesc')}</p>
        <div className="backup-codes-grid">
          {backupCodes.map((c) => (
            <div key={c} className="backup-code-item">
              {c}
            </div>
          ))}
        </div>
        <div className="btn-row">
          <Button variant="secondary" onClick={copy}>
            <Copy size={16} />
            {copied ? t('backupCodesCopied') : t('copyBackupCodes')}
          </Button>
          <Button variant="primary" onClick={onDone}>
            {t('save')}
          </Button>
        </div>
      </Modal>
    );
  }

  return (
    <Modal title={t('totpTitle')} onClose={onClose}>
      <ol className="steps">
        <li>{t('totpStep1')}</li>
        <li>{t('totpScan')}</li>
      </ol>
      {qr && (
        <div className="totp-qr">
          <img src={qr} width={192} height={192} alt={t('totpQrAlt')} />
        </div>
      )}
      <p className="muted small">{t('totpStep2')}</p>
      <div className="row align-center gap-xs">
        <code className="card flat tight mono break-all flex-1">{secret}</code>
        <Button
          variant="secondary"
          size="sm"
          type="button"
          onClick={() => {
            navigator.clipboard?.writeText(secret);
            setSecretCopied(true);
            setTimeout(() => setSecretCopied(false), 2000);
          }}
        >
          {secretCopied ? <Check size={14} /> : <Copy size={14} />}
          {secretCopied ? t('secretCopied') : t('copySecret')}
        </Button>
      </div>
      <a className="small" href={url}>
        {t('totpJoin')}
      </a>
      <Field label={t('totpCode')}>
        <input className="input" inputMode="numeric" maxLength={6} value={code} onChange={(e) => setCode(e.target.value)} />
      </Field>
      <ErrorBox error={error} />
      <Button
        variant="primary"
        disabled={busy || code.length !== 6}
        onClick={() =>
          run(async () => {
            const res = await post<{ ok: boolean; backup_codes?: string[] }>('/auth/totp/enable', { code });
            if (res?.backup_codes && res.backup_codes.length > 0) {
              setBackupCodes(res.backup_codes);
            } else {
              onDone();
            }
          }, t('enabled'))
        }
      >
        {t('enable')}
      </Button>
    </Modal>
  );
}

function Verify2faModal({ onClose, onDone }: { onClose: () => void; onDone: () => void }) {
  const { t } = useTranslation('profile');
  const [code, setCode] = useState('');
  const { busy, error, run } = useAction();

  const handleVerify = () => {
    run(async () => {
      await post('/auth/totp/verify', { code: code.trim() });
      onDone();
    }, t('verify2faSuccess'));
  };

  return (
    <Modal title={t('verify2fa')} onClose={onClose}>
      <p className="muted small">{t('verify2faDesc')}</p>
      <Field label={t('verify2faCode')}>
        <input
          className="input"
          placeholder="123456 or ABCD-1234"
          value={code}
          onChange={(e) => setCode(e.target.value)}
        />
      </Field>
      <ErrorBox error={error} />
      <div className="btn-row">
        <Button variant="primary" disabled={busy || !code.trim()} onClick={handleVerify} loading={busy}>
          <KeyRound size={16} />
          {t('verify2fa')}
        </Button>
        <Button variant="ghost" onClick={onClose}>
          {t('remove')}
        </Button>
      </div>
    </Modal>
  );
}

function CalendarFeedSection() {
  const { t } = useTranslation('profile');
  const { data: calRaw, reload: reloadCal } = useApi<{ enabled?: boolean }>('/me/calendar-feed');
  const calEnabled = Boolean(calRaw && !Array.isArray(calRaw) && calRaw.enabled);

  const [feedUrl, setFeedUrl] = useState<string | null>(null);
  const [copiedFeed, setCopiedFeed] = useState(false);
  const { busy, error, run } = useAction();

  const rotateFeed = () => {
    run(async () => {
      const res = await post<{ token: string; ics_path: string }>('/me/calendar-feed/rotate');
      const origin = typeof window !== 'undefined' ? window.location.origin : '';
      setFeedUrl(`${origin}${res.ics_path}`);
      await reloadCal();
    }, t('calendarFeedReady'));
  };

  const revokeFeed = () => {
    run(async () => {
      await del('/me/calendar-feed');
      setFeedUrl(null);
      await reloadCal();
    }, t('calendarFeedRevoked'));
  };

  const webcalUrl = feedUrl ? feedUrl.replace(/^https?:\/\//, 'webcal://') : null;

  return (
    <div className="stack">
      <div className="stack-sm">
        <div className="row between">
          <span className="bold">{t('calendarFeedTitle')}</span>
          <Pill tone={calEnabled ? 'success' : 'neutral'}>
            {calEnabled ? t('totpEnabled') : t('totpDisabled')}
          </Pill>
        </div>
        <p className="muted small">{t('calendarFeedDesc')}</p>
        {feedUrl && (
          <div className="stack-xs">
            <input className="input mono" readOnly value={feedUrl} />
            <div className="btn-row">
              <Button
                variant="secondary"
                size="sm"
                onClick={() => {
                  navigator.clipboard?.writeText(feedUrl);
                  setCopiedFeed(true);
                  setTimeout(() => setCopiedFeed(false), 2000);
                }}
              >
                <Copy size={14} />
                {copiedFeed ? t('calendarCopied') : t('calendarCopyUrl')}
              </Button>
              {webcalUrl && (
                <a className="btn secondary sm" href={webcalUrl}>
                  {t('calendarSubscribeApp')}
                </a>
              )}
            </div>
          </div>
        )}
        <div className="btn-row">
          <Button variant="primary" size="sm" disabled={busy} onClick={rotateFeed}>
            {calEnabled ? t('calendarRotateBtn') : t('calendarEnableBtn')}
          </Button>
          {calEnabled && (
            <Button variant="danger" size="sm" disabled={busy} onClick={revokeFeed}>
              {t('calendarRevokeBtn')}
            </Button>
          )}
        </div>
      </div>

      <ErrorBox error={error} />
    </div>
  );
}
