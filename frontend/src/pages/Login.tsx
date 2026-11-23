import { FormEvent, useEffect, useState } from 'react';
import { Link, Navigate, useLocation, useNavigate } from 'react-router-dom';
import { Check, CheckCircle2, Copy, ExternalLink, Fingerprint } from 'lucide-react';
import { api, post } from '../api';
import { useAuth } from '../auth';
import { useTranslation } from 'react-i18next';
import { LogoMark, Wordmark } from '../components/Brand';
import { PublicDisplayBar } from '../components/AppChrome';
import { Button, ErrorBox, Field, Modal, Notice, useAction } from '../components/ui';
import { getDevicePasskey, loginWithPasskey, passkeySupported, type DevicePasskeyInfo } from '../webauthn';
import { clearAutoLock } from '../components/AutoLock';
import { detectInAppBrowser, inAppBrowserName } from '../inAppBrowser';

type DevUser = { name: string; email: string; roles: string };

export default function Login() {
  const { t } = useTranslation('auth');
  const { me, refresh } = useAuth();
  const nav = useNavigate();
  const loc = useLocation();
  const from = (loc.state as { from?: string } | null)?.from ?? '/';
  const [devicePasskey] = useState<DevicePasskeyInfo | null>(() => getDevicePasskey());
  const [switchingAccount, setSwitchingAccount] = useState(false);
  const [email, setEmail] = useState(() => getDevicePasskey()?.email ?? '');
  const [who, setWho] = useState('');
  const [devUsers, setDevUsers] = useState<DevUser[] | null>(null);
  const { busy, error, run } = useAction();
  const [linkCopied, setLinkCopied] = useState(false);
  const [recoveryOpen, setRecoveryOpen] = useState(false);
  const [recoveryEmail, setRecoveryEmail] = useState('');
  const [recoveryEmailErr, setRecoveryEmailErr] = useState('');
  const [recoveryReason, setRecoveryReason] = useState('');
  const [recoverySent, setRecoverySent] = useState(false);
  const recoveryAct = useAction();

  useEffect(() => {
    api<DevUser[]>('/auth/dev-users').then(setDevUsers).catch(() => setDevUsers(null));
  }, []);

  if (me) return <Navigate to={from} replace />;

  const done = async () => {
    clearAutoLock();
    await refresh();
    nav(from, { replace: true });
  };

  const hasRecognizedDevicePasskey = passkeySupported() && Boolean(devicePasskey) && !switchingAccount;
  const inApp = detectInAppBrowser();

  const onPasskey = (e: FormEvent) => {
    e.preventDefault();
    const targetEmail = (hasRecognizedDevicePasskey && devicePasskey ? devicePasskey.email : email).trim();
    run(async () => {
      await loginWithPasskey(targetEmail);
      await done();
    });
  };

  const onTestAccount = (e: FormEvent) => {
    e.preventDefault();
    run(async () => {
      await post('/auth/dev-login', { email: who });
      await done();
    });
  };

  const openRecoveryModal = () => {
    const prefill = (hasRecognizedDevicePasskey && devicePasskey ? devicePasskey.email : email).trim();
    setRecoveryEmail(prefill);
    setRecoveryEmailErr('');
    setRecoveryReason('');
    setRecoverySent(false);
    setRecoveryOpen(true);
  };

  const closeRecoveryModal = () => {
    setRecoveryOpen(false);
    setRecoverySent(false);
    setRecoveryEmailErr('');
  };

  const submitRecovery = async (e: FormEvent) => {
    e.preventDefault();
    const trimmed = recoveryEmail.trim();
    if (!trimmed || !trimmed.includes('@')) {
      setRecoveryEmailErr(t('recoveryEmailRequired'));
      return;
    }
    setRecoveryEmailErr('');
    await recoveryAct.run(async () => {
      await post('/auth/recovery-request', { email: trimmed, reason: recoveryReason.trim() });
      setRecoverySent(true);
    });
  };

  const copyCurrentLink = async () => {
    try {
      await navigator.clipboard?.writeText(window.location.href);
      setLinkCopied(true);
      window.setTimeout(() => setLinkCopied(false), 2500);
    } catch {
      /* clipboard may be blocked in embedded webviews */
    }
  };

  const recoveryPresets = [
    t('recoveryPresetNewPhone'),
    t('recoveryPresetLost'),
    t('recoveryPresetBrowser'),
  ];

  return (
    <div className="login-page-wrap">
      <div className="login-split">
        <div className="card auth-card stack">
          <div className="auth-header-row row between">
            <div className="row nowrap auth-brand">
              <LogoMark size={36} />
              <div>
                <Wordmark />
                <p className="muted small">{t('brandSub', { ns: 'layout' })}</p>
              </div>
            </div>
            <PublicDisplayBar />
          </div>

          <div>
            <h1 className="auth-title">{t('welcomeBack')}</h1>
            <p className="muted">{t('welcomeBilingual')}</p>
          </div>

          {inApp && (
            <Notice tone="warning" icon={<ExternalLink size={20} />}>
              <div className="stack-xs">
                <div className="bold">{t('inAppTitle')}</div>
                <p className="small no-margin">{t('inAppBody', { app: inAppBrowserName(inApp) })}</p>
                <div>
                  <Button variant="secondary" size="sm" type="button" onClick={copyCurrentLink}>
                    {linkCopied ? <Check size={16} /> : <Copy size={16} />}
                    {linkCopied ? t('linkCopied') : t('copyLoginLink')}
                  </Button>
                </div>
              </div>
            </Notice>
          )}

          {/* Passkey primary login form */}
          <form className="stack" onSubmit={onPasskey}>
            {hasRecognizedDevicePasskey && devicePasskey ? (
              <div className="passkey-device-card stack-sm">
                <div className="row items-center gap-sm">
                  <span className="notice-icon text-primary">
                    <Fingerprint size={24} />
                  </span>
                  <div className="stack-none" style={{ minWidth: 0, flex: 1 }}>
                    <div className="bold small row items-center gap-xs">
                      <span>{t('passkeyDetected')}</span>
                      <span className="pill success tiny">{t('thisDevice')}</span>
                    </div>
                    <div className="text-ellipsis bold" style={{ fontSize: 'var(--fs-md)' }}>
                      {devicePasskey.email}
                    </div>
                  </div>
                </div>
                <p className="muted tiny">{t('passkeyReadyHint')}</p>
              </div>
            ) : (
              <Field label={t('email')}>
                <input
                  className="input"
                  type="email"
                  autoComplete="username webauthn"
                  required
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                />
              </Field>
            )}

            {passkeySupported() ? (
              <Button variant="primary" size="lg" type="submit" block loading={busy} disabled={busy}>
                <Fingerprint size={20} />
                {busy ? t('loggingIn') : t('loginWithPasskey')}
              </Button>
            ) : (
              <div className="notice warning">{t('noPasskey')}</div>
            )}

            {hasRecognizedDevicePasskey ? (
              <Button
                variant="ghost"
                type="button"
                block
                disabled={busy}
                onClick={() => {
                  setSwitchingAccount(true);
                  setEmail('');
                }}
              >
                {t('switchAccount')}
              </Button>
            ) : devicePasskey && switchingAccount ? (
              <Button
                variant="ghost"
                type="button"
                block
                disabled={busy}
                onClick={() => {
                  setSwitchingAccount(false);
                  setEmail(devicePasskey.email);
                }}
              >
                {t('useSavedPasskey', { email: devicePasskey.email })}
              </Button>
            ) : null}

            <Button
              variant={passkeySupported() ? 'ghost' : 'secondary'}
              type="button"
              block
              disabled={busy}
              onClick={openRecoveryModal}
            >
              {t('emailRecovery')}
            </Button>

            <p className="muted small">{t('passkeyHelp')}</p>
            <div>
              <Link to="/terms">{t('title', { ns: 'terms' })}</Link>
            </div>
          </form>

          {/* Dev account picker in a dashed frame */}
          {devUsers && devUsers.length > 0 && (
            <div className="dev-account-frame stack-sm">
              <div className="row between dev-frame-header">
                <span className="dev-badge">{t('devBadge')}</span>
              </div>
              <form className="stack-sm" onSubmit={onTestAccount}>
                <Field label={t('testAccounts')} hint={t('testAccountsHint')}>
                  <select
                    className="input"
                    required
                    value={who}
                    onChange={(e) => setWho(e.target.value)}
                  >
                    <option value="">{t('testChoose')}</option>
                    {devUsers.map((u) => (
                      <option key={u.email} value={u.email}>
                        {u.name} · {u.email}
                      </option>
                    ))}
                  </select>
                </Field>
                <Button variant="secondary" type="submit" block disabled={busy || !who}>
                  {busy ? t('loggingIn') : t('login')}
                </Button>
              </form>
            </div>
          )}

          <ErrorBox error={error} />
        </div>
      </div>

      {recoveryOpen && (
        <Modal title={t('recoveryModalTitle')} onClose={closeRecoveryModal}>
          {recoverySent ? (
            <div className="stack">
              <div className="row items-center gap-sm text-success">
                <CheckCircle2 size={22} aria-hidden="true" />
                <span className="bold">{t('recoverySentTitle')}</span>
              </div>
              <p className="no-margin">{t('recoveryReceived')}</p>
              <Button variant="primary" type="button" block onClick={closeRecoveryModal}>
                {t('recoveryDone')}
              </Button>
            </div>
          ) : (
            <form className="stack" onSubmit={submitRecovery} noValidate>
              <p className="muted small no-margin">{t('recoveryModalHint')}</p>
              <div>
                <Field label={t('email')}>
                  <input
                    className="input"
                    type="email"
                    autoComplete="email"
                    required
                    value={recoveryEmail}
                    onChange={(e) => {
                      setRecoveryEmail(e.target.value);
                      if (recoveryEmailErr) setRecoveryEmailErr('');
                    }}
                  />
                </Field>
                {recoveryEmailErr && (
                  <div className="field-error-msg" role="alert">
                    {recoveryEmailErr}
                  </div>
                )}
              </div>

              <div className="stack-xs">
                <div className="bold small">{t('recoveryReasonLabel')}</div>
                <div className="row wrap gap-xs">
                  {recoveryPresets.map((preset) => (
                    <button
                      key={preset}
                      type="button"
                      className={`chip ${recoveryReason === preset ? 'active' : ''}`}
                      onClick={() => setRecoveryReason((cur) => (cur === preset ? '' : preset))}
                    >
                      {preset}
                    </button>
                  ))}
                </div>
                <textarea
                  className="input"
                  rows={2}
                  value={recoveryReason}
                  placeholder={t('recoveryReasonPlaceholder')}
                  onChange={(e) => setRecoveryReason(e.target.value)}
                />
              </div>

              <ErrorBox error={recoveryAct.error} />
              <Button variant="primary" type="submit" block loading={recoveryAct.busy} disabled={recoveryAct.busy}>
                {t('recoverySubmit')}
              </Button>
            </form>
          )}
        </Modal>
      )}
    </div>
  );
}
