import { ReactNode, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  BarChart3,
  Copy,
  Fingerprint,
  KeyRound,
  LifeBuoy,
  Lock,
  LogOut,
  Receipt,
  RefreshCw,
  Shield,
  Sliders,
  User,
  Wallet,
} from 'lucide-react';
import { del, post, put, useApi } from '../api';
import { useAuth } from '../auth';
import { useAutoLock } from '../components/AutoLock';
import { DisplayPreferences } from '../components/AppChrome';
import { Avatar, Button, ErrorBox, Field, FileInput, Modal, Pill, SheetLink, useAction, useConfirm } from '../components/ui';
import { fmtAgo, roleLabel, useFormatT } from '../format';
import { passkeySupported, registerPasskey, withRecentSignIn } from '../webauthn';
import QRCode from 'qrcode';
import { useTranslation } from 'react-i18next';

type SectionKey = 'personal' | 'preferences' | 'security';

export default function Profile() {
  const { t } = useTranslation('profile');
  const ft = useFormatT();
  const confirm = useConfirm();
  const { me, refresh, can } = useAuth();
  // A too-old sign-in can be renewed in place only with a passkey this browser can use.
  const canReauth = passkeySupported() && (me?.passkey_count ?? 0) > 0;
  const recentSignIn = <T,>(fn: () => Promise<T>) => withRecentSignIn(fn, canReauth);
  const nav = useNavigate();
  const { lock } = useAutoLock();
  const passkeys = useApi<{ id: number; label: string; created_at: string; last_used: string | null }[]>('/auth/passkeys');
  const [open, setOpen] = useState<SectionKey | null>(null);
  const [f, setF] = useState({
    name: me?.name ?? '',
    phone: me?.phone ?? '',
    photo: me?.photo_file_id ?? null,
    optOut: me?.badges_opt_out ?? false,
  });
  const [readinessPrompt, setReadinessPrompt] = useState(me?.readiness_prompt_mins ?? 45);
  const [autoLockMins, setAutoLockMins] = useState(me?.auto_lock_mins ?? (parseInt(localStorage.getItem('gr_autolock_mins') || '0', 10) || 0));
  const [totp, setTotp] = useState<{ secret: string; url: string } | null>(null);
  const [showVerify2fa, setShowVerify2fa] = useState(false);
  const [showBackupCodes, setShowBackupCodes] = useState<string[] | null>(null);
  const { busy, error, run } = useAction();
  if (!me) return null;

  const openSection = (section: SectionKey) => {
    setF({
      name: me.name ?? '',
      phone: me.phone ?? '',
      photo: me.photo_file_id ?? null,
      optOut: me.badges_opt_out ?? false,
    });
    setReadinessPrompt(me.readiness_prompt_mins ?? 45);
    setAutoLockMins(
      me.auto_lock_mins ?? (parseInt(localStorage.getItem('gr_autolock_mins') || '0', 10) || 0),
    );
    setOpen(section);
  };

  const isPending = me.status === 'pending';
  const isAdmin = me.privileged;
  const roleText = isPending ? t('pending') : isAdmin ? t('administrator') : t('member');
  const iconSz = 20;

  const savePersonal = () => put('/me', { name: f.name, phone: f.phone, photo_file_id: f.photo }).then(refresh);
  const savePreferences = () =>
    put('/me', { badges_opt_out: f.optOut, readiness_prompt_mins: readinessPrompt }).then(refresh);

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
    can('costs') && { to: '/costs', label: t('myCosts'), icon: <Wallet size={iconSz} /> },
    can('reimbursements') && { to: '/reimbursements', label: t('myReimb'), icon: <Receipt size={iconSz} /> },
    can('report') && { to: '/report', label: t('myReport'), icon: <BarChart3 size={iconSz} /> },
    can('help') && { to: '/help', label: t('myHelp'), icon: <LifeBuoy size={iconSz} /> },
  ].filter(Boolean) as { to: string; label: string; icon: ReactNode }[];

  const sheetTitle: Record<SectionKey, string> = {
    personal: t('personalInfo'),
    preferences: t('preferences'),
    security: t('security'),
  };

  return (
    <div className="stack profile-page">
      <div className="profile-identity">
        <div className="row align-center gap-md">
          <Avatar name={me.name} fileId={me.photo_file_id} size="lg" />
          <div className="stack-sm">
            <h1>{me.name}</h1>
            <Pill tone={isPending ? 'warning' : 'primary'}>{roleText}</Pill>
          </div>
        </div>
      </div>

      <div className="stack">
        <section className="stack-xs" aria-label={t('account')}>
          <div className="section-label">{t('account')}</div>
          <div className="list">
            <SheetLink icon={<User size={iconSz} />} label={t('personalInfo')} aria-haspopup="dialog" onClick={() => openSection('personal')} />
            <SheetLink icon={<Sliders size={iconSz} />} label={t('preferences')} aria-haspopup="dialog" onClick={() => openSection('preferences')} />
            <SheetLink icon={<Shield size={iconSz} />} label={t('security')} aria-haspopup="dialog" onClick={() => openSection('security')} />
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

      {open && (
        <Modal title={sheetTitle[open]} onClose={() => setOpen(null)} size="sm">
          {open === 'personal' && (
            <div className="stack">
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
                <FileInput label={t('photo')} accept="image/*" value={f.photo} onChange={(id) => setF({ ...f, photo: id })} />
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
              <DisplayPreferences />
              <div className="btn-row">
                <Button variant="primary" onClick={() => run(savePreferences, t('saved'))} loading={busy}>
                  {t('save')}
                </Button>
              </div>
              <ErrorBox error={error} />
            </div>
          )}

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
                            if (ok) run(() => recentSignIn(() => del(`/auth/passkeys/${p.id}`)).then(passkeys.reload).then(refresh), t('removed'));
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
                  onClick={() => run(() => recentSignIn(() => registerPasskey(navigator.platform || t('myPhone'))).then(passkeys.reload).then(refresh), t('addPasskeyDone'))}
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
                      <Button variant="ghost" size="sm" onClick={lock}>
                        <Lock size={16} />
                        {t('lockNow')}
                      </Button>
                    </div>
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
        </Modal>
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
          autoFocus
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
