import { FormEvent, useEffect, useState } from 'react';
import { Link, Navigate, useLocation, useNavigate } from 'react-router-dom';
import { Check, CheckCircle2, Copy, ExternalLink, Fingerprint } from 'lucide-react';
import { api, post } from '../api';
import { useAuth } from '../auth';
import { useTranslation } from 'react-i18next';
import { LogoMark, Wordmark } from '../components/Brand';
import { PublicDisplayBar } from '../components/AppChrome';
import TurnstileWidget from '../components/TurnstileWidget';
import { Button, ErrorBox, Field, Modal, Notice, useAction } from '../components/ui';
import { useTurnstile } from '../turnstile';
import { getDevicePasskey, loginWithPasskey, passkeySupported, type DevicePasskeyInfo } from '../webauthn';
import { clearAutoLock } from '../components/AutoLock';
import { detectInAppBrowser, inAppBrowserName } from '../inAppBrowser';

type DevUser = { name: string; email: string; roles: string };

export default function Login() {
  const { t } = useTranslation('auth');
  const { me, refresh } = useAuth();
  const nav = useNavigate();
  const loc = useLocation();
  const rawFrom = (loc.state as { from?: string } | null)?.from ?? '/';
  const from = rawFrom.startsWith('/') && !rawFrom.startsWith('//') && !rawFrom.startsWith('/api/') ? rawFrom : '/';
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
  const passkeyTurnstile = useTurnstile();
  const recoveryTurnstile = useTurnstile();

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
    if (passkeyTurnstile.enabled && !passkeyTurnstile.token) return;
    const targetEmail = (hasRecognizedDevicePasskey && devicePasskey ? devicePasskey.email : email).trim();
    run(async () => {
      try {
        await (passkeyTurnstile.token
          ? loginWithPasskey(targetEmail, passkeyTurnstile.token)
          : loginWithPasskey(targetEmail));
        await done();
      } catch (err) {
        passkeyTurnstile.reset();
        throw err;
      }
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
    recoveryTurnstile.reset();
    setRecoveryOpen(true);
  };

  const closeRecoveryModal = () => {
    recoveryTurnstile.reset();
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
    if (recoveryTurnstile.enabled && !recoveryTurnstile.token) return;
    setRecoveryEmailErr('');
    await recoveryAct.run(async () => {
      try {
        await post(
          '/auth/recovery-request',
          recoveryTurnstile.token
            ? { email: trimmed, reason: recoveryReason.trim(), turnstile_token: recoveryTurnstile.token }
            : { email: trimmed, reason: recoveryReason.trim() },
        );
        setRecoverySent(true);
      } catch (err) {
        recoveryTurnstile.reset();
        throw err;
      }
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
              <Wordmark />
            </div>
            <PublicDisplayBar />
          </div>

          <h1 className="auth-title">{t('welcomeBack')}</h1>

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
              <div className="passkey-device-line">
                <Fingerprint size={18} aria-hidden="true" />
                <span className="passkey-device-email">{devicePasskey.email}</span>
                <span className="sr-only">{t('passkeyDetected')}</span>
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

            {passkeySupported() && (
              <TurnstileWidget
                siteKey={passkeyTurnstile.siteKey}
                action="passkey_login"
                resetKey={passkeyTurnstile.resetKey}
                onToken={passkeyTurnstile.setToken}
              />
            )}

            {passkeySupported() ? (
              <Button
                variant="primary"
                size="lg"
                type="submit"
                block
                loading={busy}
                disabled={busy || (passkeyTurnstile.enabled && !passkeyTurnstile.token)}
              >
                <Fingerprint size={20} />
                {busy ? t('loggingIn') : t('loginWithPasskey')}
              </Button>
            ) : (
              <div className="notice warning">{t('noPasskey')}</div>
            )}

            <div className="auth-alt-actions">
              {hasRecognizedDevicePasskey ? (
                <button
                  type="button"
                  className="link-btn"
                  disabled={busy}
                  onClick={() => {
                    setSwitchingAccount(true);
                    setEmail('');
                  }}
                >
                  {t('switchAccount')}
                </button>
              ) : devicePasskey && switchingAccount ? (
                <button
                  type="button"
                  className="link-btn"
                  disabled={busy}
                  onClick={() => {
                    setSwitchingAccount(false);
                    setEmail(devicePasskey.email);
                  }}
                >
                  {t('useSavedPasskey', { email: devicePasskey.email })}
                </button>
              ) : null}
              {passkeySupported() && (
                <button type="button" className="link-btn" disabled={busy} onClick={openRecoveryModal}>
                  {t('emailRecovery')}
                </button>
              )}
            </div>

            {/* Without passkeys, recovery is the only way in, so it gets a full button. */}
            {!passkeySupported() && (
              <Button variant="secondary" type="button" block disabled={busy} onClick={openRecoveryModal}>
                {t('emailRecovery')}
              </Button>
            )}

            <div className="auth-footnote">
              <p className="muted small no-margin">{t('passkeyHelp')}</p>
              <Link to="/terms" className="small">{t('title', { ns: 'terms' })}</Link>
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

              <TurnstileWidget
                siteKey={recoveryTurnstile.siteKey}
                action="recovery_request"
                resetKey={recoveryTurnstile.resetKey}
                onToken={recoveryTurnstile.setToken}
              />
              <ErrorBox error={recoveryAct.error} />
              <Button
                variant="primary"
                type="submit"
                block
                loading={recoveryAct.busy}
                disabled={recoveryAct.busy || (recoveryTurnstile.enabled && !recoveryTurnstile.token)}
              >
                {t('recoverySubmit')}
              </Button>
            </form>
          )}
        </Modal>
      )}
    </div>
  );
}
