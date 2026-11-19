import { ReactNode, useEffect, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import {
  BarChart3,
  Camera,
  Check,
  Copy,
  Download,
  Fingerprint,
  KeyRound,
  LifeBuoy,
  Lock,
  LogOut,
  MapPin,
  Receipt,
  RefreshCw,
  Shield,
  Sliders,
  Smartphone,
  Trash2,
  User,
  Wallet,
  Car,
  CarFront,
} from 'lucide-react';
import { api, del, post, put, useApi } from '../api';
import { useAuth } from '../auth';
import { useAutoLock } from '../components/AutoLock';
import { DisplayPreferences } from '../components/AppChrome';
import { Avatar, Button, ErrorBox, Field, Modal, Notice, PageHeader, Pill, SheetLink, useAction, useConfirm } from '../components/ui';
import { fmtAgo, roleLabel, useFormatT } from '../format';
import { clearDevicePasskey, passkeySupported, registerPasskey, withRecentSignIn } from '../webauthn';
import { WaitingRoom } from '../components/Onboarding';
import AvatarEditor from '../components/AvatarEditor';
import { PwaInstallModal } from '../components/PwaInstallModal';
import { usePwaInstall } from '../pwaInstall';
import QRCode from 'qrcode';
import { useTranslation } from 'react-i18next';

type SectionKey = 'personal' | 'preferences' | 'places' | 'security';

export default function Profile() {
  const { t } = useTranslation('profile');
  const ft = useFormatT();
  const confirm = useConfirm();
  const { me, refresh, can } = useAuth();
  // A too-old sign-in can be renewed in place only with a passkey this browser can use.
  const canReauth = passkeySupported() && (me?.passkey_count ?? 0) > 0;
  const recentSignIn = <T,>(fn: () => Promise<T>) => withRecentSignIn(fn, canReauth);
  const nav = useNavigate();
  const { lock, canLock } = useAutoLock();
  const passkeys = useApi<{ id: number; label: string; created_at: string; last_used: string | null }[]>('/auth/passkeys');
  // The open section lives in the URL so the header back button and swipe-back both return to the list.
  const [searchParams] = useSearchParams();
  const sectionParam = searchParams.get('section');
  const open: SectionKey | null = (['personal', 'preferences', 'places', 'security'] as const).find((k) => k === sectionParam) ?? null;
  const pushCfg = useApi<{
    enabled?: boolean;
    configured?: boolean;
    vapid_public_key?: string;
    subscription_count?: number;
  }>(open === 'preferences' && me?.status !== 'pending' ? '/push/config' : null);
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
  const [pushStatus, setPushStatus] = useState('');
  const [pushBusy, setPushBusy] = useState(false);
  const [totp, setTotp] = useState<{ secret: string; url: string } | null>(null);
  const [showVerify2fa, setShowVerify2fa] = useState(false);
  const [showBackupCodes, setShowBackupCodes] = useState<string[] | null>(null);
  const [editAvatar, setEditAvatar] = useState(false);
  const pwa = usePwaInstall();
  const [showInstallModal, setShowInstallModal] = useState(false);
  const { busy, error, run } = useAction();
  if (!me) return null;

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

  const savePersonal = () => put('/me', { name: f.name, phone: f.phone }).then(refresh);
  const savePreferences = () =>
    put('/me', {
      badges_opt_out: f.optOut,
      readiness_prompt_mins: readinessPrompt,
      telegram_chat_id: telegramChatId.trim(),
      whatsapp_opt_in: whatsappOptIn,
      push_opt_in: pushOptIn,
    }).then(refresh);

  const enableBrowserPush = async () => {
    setPushStatus('');
    if (typeof Notification === 'undefined') {
      setPushStatus(t('pushUnsupported'));
      return;
    }
    const perm = await Notification.requestPermission();
    if (perm !== 'granted') {
      setPushStatus(t('pushDenied'));
      return;
    }
    setPushOptIn(true);
    setPushBusy(true);
    try {
      await put('/me', { push_opt_in: true }).catch(() => {});
      void refresh();
      const cfg = await api<{ enabled?: boolean; vapid_public_key?: string }>('/push/config');
      const reg = 'serviceWorker' in navigator ? await navigator.serviceWorker.getRegistration() : undefined;
      if (cfg?.vapid_public_key && reg?.pushManager) {
        const pad = '='.repeat((4 - (cfg.vapid_public_key.length % 4)) % 4);
        const b64 = (cfg.vapid_public_key + pad).replace(/-/g, '+').replace(/_/g, '/');
        const raw = atob(b64);
        const keyBytes = Uint8Array.from(raw, (c) => c.charCodeAt(0));
        let sub: PushSubscription;
        try {
          sub = await reg.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: keyBytes });
        } catch (err) {
          const existing = await reg.pushManager.getSubscription();
          if (existing) {
            await existing.unsubscribe();
            sub = await reg.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: keyBytes });
          } else {
            throw err;
          }
        }
        const json = sub.toJSON();
        if (json.endpoint) {
          await post('/push/subscribe', {
            endpoint: json.endpoint,
            p256dh: json.keys?.p256dh ?? '',
            auth: json.keys?.auth ?? '',
          });
          pushCfg.reload();
        }
        setPushStatus(t('pushEnabled'));
      } else {
        setPushStatus(t('pushEnabledLocal'));
      }
    } catch {
      setPushStatus(t('pushEnabledLocal'));
    } finally {
      setPushBusy(false);
    }
  };

  const sendTestPush = async () => {
    setPushStatus('');
    setPushBusy(true);
    try {
      await post('/push/test');
      setPushStatus(t('pushTestSent'));
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
      let endpoint = '';
      if ('serviceWorker' in navigator) {
        const reg = await navigator.serviceWorker.ready;
        const sub = await reg.pushManager?.getSubscription?.();
        if (sub) {
          endpoint = sub.endpoint;
          await sub.unsubscribe();
        }
      }
      if (endpoint) {
        await api('/push/subscribe', { method: 'DELETE', body: { endpoint } });
        pushCfg.reload();
      }
      setPushStatus(t('pushDisabledDevice'));
    } catch {
      setPushStatus(t('pushTestFailed'));
    } finally {
      setPushBusy(false);
    }
  };

  const handleLogout = () =>
    run(async () => {
      await post('/auth/logout');
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

  const related = [
    can('rides') && { to: '/rides', label: t('myRideHistory'), icon: <Car size={iconSz} /> },
    can('drive') && { to: '/drive', label: t('myDrivingHistory'), icon: <CarFront size={iconSz} /> },
    can('costs') && { to: '/costs', label: t('myCosts'), icon: <Wallet size={iconSz} /> },
    can('reimbursements') && { to: '/reimbursements', label: t('myReimb'), icon: <Receipt size={iconSz} /> },
    can('report') && { to: '/report', label: t('myReport'), icon: <BarChart3 size={iconSz} /> },
    can('help') && { to: '/help', label: t('myHelp'), icon: <LifeBuoy size={iconSz} /> },
  ].filter(Boolean) as { to: string; label: string; icon: ReactNode }[];

  const sheetTitle: Record<SectionKey, string> = {
    personal: t('personalInfo'),
    preferences: t('preferences'),
    places: t('placesAndCalendar'),
    security: t('security'),
  };

  return (
    <div className="stack profile-page profile-hub">
      {!open && (
        <>
      <section className="profile-hero" aria-label={t('account')}>
        <button
          type="button"
          className="profile-avatar-btn"
          aria-label={t('avatarEdit')}
          onClick={() => setEditAvatar(true)}
        >
          {avatarEl}
          <span className="profile-avatar-badge" aria-hidden="true">
            <Camera size={16} />
          </span>
        </button>
        <h1>{me.name}</h1>
        <div className="profile-email">{me.email}</div>
        <div className="profile-tags">
          <Pill tone={isPending ? 'warning' : 'primary'}>{roleText}</Pill>
          {!isPending && me.groups.slice(0, 3).map((g) => <Pill key={g.id}>{g.name}</Pill>)}
        </div>
      </section>

      {isPending && <WaitingRoom />}

      <div className="stack">
        {!pwa.isInstalled && (
          <Notice tone="info" icon={<Smartphone size={18} />}>
            <div className="row between align-center wrap gap-xs">
              <div className="stack-xs">
                <strong>{t('installAppBannerTitle')}</strong>
                <span className="muted small">{t('installAppBannerDesc')}</span>
              </div>
              <Button variant="primary" size="sm" onClick={handleInstallClick}>
                {pwa.canPrompt ? (
                  <>
                    <Download size={14} aria-hidden="true" />
                    {t('installAppOneClick')}
                  </>
                ) : (
                  t('installAppGuide')
                )}
              </Button>
            </div>
          </Notice>
        )}

        <section className="stack-xs" aria-label={t('account')}>
          <div className="section-label">{t('account')}</div>
          <div className="list">
            <SheetLink
              tone="blue"
              icon={<User size={iconSz} />}
              label={t('personalInfo')}
              hint={me.phone || t('personalHint')}
              aria-haspopup="dialog"
              onClick={() => openSection('personal')}
            />
            <SheetLink
              tone="teal"
              icon={<Sliders size={iconSz} />}
              label={t('preferences')}
              hint={prefsHint}
              aria-haspopup="dialog"
              onClick={() => openSection('preferences')}
            />
            {!isPending && (
              <SheetLink
                tone="amber"
                icon={<MapPin size={iconSz} />}
                label={t('placesAndCalendar')}
                aria-haspopup="dialog"
                onClick={() => openSection('places')}
              />
            )}
            <SheetLink
              tone="green"
              icon={<Shield size={iconSz} />}
              label={t('security')}
              hint={securityHint}
              aria-haspopup="dialog"
              onClick={() => openSection('security')}
            />
            <SheetLink
              tone={pwa.isInstalled ? 'green' : 'purple'}
              icon={pwa.isInstalled ? <Check size={iconSz} /> : <Smartphone size={iconSz} />}
              label={t('installApp')}
              hint={
                pwa.isInstalled
                  ? t('installAppInstalledDesc')
                  : pwa.canPrompt
                    ? t('installAppOneClickPrompt')
                    : t('installAppDesc')
              }
              badge={
                pwa.isInstalled ? (
                  <Pill tone="success">{t('installAppInstalled')}</Pill>
                ) : (
                  <Pill tone={pwa.canPrompt ? 'primary' : 'neutral'}>
                    {pwa.canPrompt ? t('installAppOneClick') : t('installAppRecommended')}
                  </Pill>
                )
              }
              aria-haspopup="dialog"
              onClick={handleInstallClick}
            />
          </div>
        </section>

        {!isPending && related.length > 0 && (
          <section className="stack-xs" aria-label={t('relatedPages')}>
            <div className="section-label">{t('relatedPages')}</div>
            <div className="list">
              {related.map((n) => (
                <SheetLink key={n.to} to={n.to} icon={n.icon} label={n.label} />
              ))}
            </div>
          </section>
        )}
      </div>

      <div className="list">
        <SheetLink
          icon={<LogOut size={iconSz} aria-hidden="true" />}
          label={t('logout')}
          chevron={false}
          onClick={handleLogout}
        />
      </div>
        </>
      )}

      {open && (
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
                  <input className="input" required value={f.name} onChange={(e) => setF({ ...f, name: e.target.value })} />
                </Field>
                <Field label={t('phone')} hint={t('phoneHint')}>
                  <input className="input" type="tel" value={f.phone} onChange={(e) => setF({ ...f, phone: e.target.value })} />
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
              <ErrorBox error={error} />
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
                {(pushCfg.data?.subscription_count ?? 0) > 0 && (
                  <p className="muted tiny">{t('pushConnectedCount', { count: pushCfg.data?.subscription_count ?? 0 })}</p>
                )}
                <div className="row wrap gap-xs">
                  <Button variant="secondary" size="sm" disabled={pushBusy} onClick={enableBrowserPush}>
                    {t('enableBrowserPush')}
                  </Button>
                  {(pushCfg.data?.subscription_count ?? 0) > 0 && (
                    <>
                      <Button variant="secondary" size="sm" disabled={pushBusy} onClick={sendTestPush}>
                        {t('sendTestPush')}
                      </Button>
                      <Button variant="ghost" size="sm" disabled={pushBusy} onClick={disableBrowserPush}>
                        {t('disableBrowserPush')}
                      </Button>
                    </>
                  )}
                </div>
                {pushStatus && <p className="muted tiny">{pushStatus}</p>}
                <label className={`check ${whatsappOptIn ? 'on' : ''}`}>
                  <input type="checkbox" checked={whatsappOptIn} onChange={(e) => setWhatsappOptIn(e.target.checked)} /> {t('whatsappOptIn')}
                </label>
                <Field label={t('telegramChatId')} hint={t('telegramChatIdHint')}>
                  <input
                    className="input"
                    value={telegramChatId}
                    onChange={(e) => setTelegramChatId(e.target.value)}
                    placeholder="123456789"
                  />
                </Field>
              </div>
              <div className="divider" />
              <DisplayPreferences />
              <div className="btn-row">
                <Button variant="primary" onClick={() => run(savePreferences, t('saved'))} loading={busy}>
                  {t('save')}
                </Button>
              </div>
              <ErrorBox error={error} />
            </div>
          )}

          {open === 'places' && <PlacesAndCalendarSection />}

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

              <ErrorBox error={error} />
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
        <PwaInstallModal onClose={() => setShowInstallModal(false)} />
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
      <code className="card flat tight mono break-all">{secret}</code>
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

interface SavedPlaceItem {
  id: number;
  label: string;
  area_id: number | null;
  area_name: string | null;
  address: string;
  gate_notes: string;
  is_default: number;
}

interface AreaOpt {
  id: number;
  name: string;
  active: boolean;
}

function PlacesAndCalendarSection() {
  const { t } = useTranslation('profile');
  const { data: placesRaw, reload: reloadPlaces } = useApi<SavedPlaceItem[]>('/saved-places');
  const { data: areasRaw } = useApi<AreaOpt[]>('/areas');
  const { data: calRaw, reload: reloadCal } = useApi<{ enabled?: boolean }>('/me/calendar-feed');
  const places = Array.isArray(placesRaw) ? placesRaw : [];
  const areas = (Array.isArray(areasRaw) ? areasRaw : []).filter((a) => a.active);
  const calEnabled = Boolean(calRaw && !Array.isArray(calRaw) && calRaw.enabled);

  const [newPlace, setNewPlace] = useState({ label: '', area_id: '', address: '', gate_notes: '' });
  const [feedUrl, setFeedUrl] = useState<string | null>(null);
  const [copiedFeed, setCopiedFeed] = useState(false);
  const { busy, error, run } = useAction();

  const addPlace = () => {
    run(async () => {
      await post('/saved-places', {
        label: newPlace.label.trim(),
        area_id: newPlace.area_id ? Number(newPlace.area_id) : null,
        address: newPlace.address.trim(),
        gate_notes: newPlace.gate_notes.trim(),
      });
      setNewPlace({ label: '', area_id: '', address: '', gate_notes: '' });
      await reloadPlaces();
    }, t('saved'));
  };

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
        <div className="bold">{t('savedPlacesTitle')}</div>
        <p className="muted small">{t('savedPlacesHint')}</p>
        {places.length > 0 && (
          <div className="stack-sm">
            {places.map((sp) => (
              <div key={sp.id} className="card flat tight row between nowrap gap-sm">
                <div className="min-w-0">
                  <div className="bold small">{sp.label}{sp.area_name ? ` · ${sp.area_name}` : ''}</div>
                  <div className="muted tiny ellipsis">{sp.address}</div>
                  {sp.gate_notes && <div className="tiny muted">{sp.gate_notes}</div>}
                </div>
                <Button
                  variant="danger"
                  size="sm"
                  disabled={busy}
                  onClick={() => run(() => del(`/saved-places/${sp.id}`).then(reloadPlaces))}
                >
                  <Trash2 size={14} />
                  {t('remove')}
                </Button>
              </div>
            ))}
          </div>
        )}
        <div className="form-grid two">
          <Field label={t('placeLabel')}>
            <input
              className="input"
              value={newPlace.label}
              onChange={(e) => setNewPlace({ ...newPlace, label: e.target.value })}
              placeholder={t('placeLabelPlaceholder')}
            />
          </Field>
          <Field label={t('placeArea')}>
            <select
              className="input"
              value={newPlace.area_id}
              onChange={(e) => setNewPlace({ ...newPlace, area_id: e.target.value })}
            >
              <option value="">—</option>
              {areas.map((a) => (
                <option key={a.id} value={a.id}>{a.name}</option>
              ))}
            </select>
          </Field>
        </div>
        <Field label={t('placeAddress')}>
          <input
            className="input"
            value={newPlace.address}
            onChange={(e) => setNewPlace({ ...newPlace, address: e.target.value })}
          />
        </Field>
        <Field label={t('placeGateNotes')}>
          <input
            className="input"
            value={newPlace.gate_notes}
            onChange={(e) => setNewPlace({ ...newPlace, gate_notes: e.target.value })}
            placeholder={t('placeGatePlaceholder')}
          />
        </Field>
        <Button
          variant="secondary"
          disabled={busy || !newPlace.label.trim() || !newPlace.address.trim()}
          onClick={addPlace}
        >
          {t('addSavedPlace')}
        </Button>
      </div>

      <div className="divider" />

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
