import { FormEvent, useEffect, useState } from 'react';
import { Link, Navigate, useLocation, useNavigate, useSearchParams } from 'react-router-dom';
import { Check, CheckCircle2, Copy, ExternalLink, Fingerprint, KeyRound, Lock } from 'lucide-react';
import { api, ApiError, apiMessage, post } from '../api';
import { useAuth } from '../auth';
import { useTranslation } from 'react-i18next';
import { LogoMark, Wordmark } from '../components/Brand';
import { PublicDisplayBar } from '../components/AppChrome';
import TurnstileWidget from '../components/TurnstileWidget';
import { Button, ErrorBox, Field, Modal, Notice, useAction } from '../components/ui';
import { useTurnstile } from '../turnstile';
import { getDevicePasskey, loginWithPasskey, passkeySupported, type DevicePasskeyInfo } from '../webauthn';
import { clearAutoLock } from '../components/AutoLock';
import { safeInternalPath } from '../guideLinks';
import { clearClientSessionStorage } from '../offlineSnapshot';
import { detectInAppBrowser, inAppBrowserName } from '../inAppBrowser';

type DevUser = { name: string; email: string; roles: string };

function GoogleMark() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden="true">
      <path
        fill="var(--google-blue)"
        d="M23.745 12.27c0-.7-.06-1.4-.19-2.07H12v4.51h6.6c-.29 1.52-1.14 2.82-2.4 3.68v3.05h3.88c2.27-2.09 3.665-5.17 3.665-9.17Z"
      />
      <path
        fill="var(--google-green)"
        d="M12 24c3.24 0 5.95-1.08 7.93-2.91l-3.88-3.05c-1.08.72-2.45 1.16-4.05 1.16-3.12 0-5.77-2.11-6.72-4.96H1.29v3.14C3.26 21.3 7.31 24 12 24Z"
      />
      <path
        fill="var(--google-yellow)"
        d="M5.28 14.24c-.24-.72-.38-1.49-.38-2.24s.14-1.52.38-2.24V6.62H1.29C.47 8.24 0 10.06 0 12s.47 3.76 1.29 5.38l3.99-3.14Z"
      />
      <path
        fill="var(--google-red)"
        d="M12 4.75c1.77 0 3.35.61 4.6 1.8l3.42-3.42C17.95 1.19 15.24 0 12 0 7.31 0 3.26 2.7 1.29 6.62l3.99 3.14c.95-2.85 3.6-4.96 6.72-4.96Z"
      />
    </svg>
  );
}

export default function Login() {
  const { t } = useTranslation('auth');
  const { me, refresh } = useAuth();
  const nav = useNavigate();
  const loc = useLocation();
  const [searchParams, setSearchParams] = useSearchParams();
  const rawFrom = (loc.state as { from?: string } | null)?.from ?? '/';
  const from = safeInternalPath(rawFrom, '/');
  const [devicePasskey] = useState<DevicePasskeyInfo | null>(() => getDevicePasskey());
  const [switchingAccount, setSwitchingAccount] = useState(false);
  const [mode, setMode] = useState<'passkey' | 'password'>('passkey');
  const [email, setEmail] = useState(() => getDevicePasskey()?.email ?? '');
  const [password, setPassword] = useState('');
  const [totpCode, setTotpCode] = useState('');
  const [needsTotp, setNeedsTotp] = useState(false);
  const [who, setWho] = useState('');
  const [devUsers, setDevUsers] = useState<DevUser[] | null>(null);
  const [googleEnabled, setGoogleEnabled] = useState(false);
  const [oauthError, setOauthError] = useState<string | null>(null);
  const [resetNotice, setResetNotice] = useState<string | null>(null);
  const { busy, error, setError, run } = useAction();
  const googleAct = useAction();
  const [linkCopied, setLinkCopied] = useState(false);
  const [recoveryOpen, setRecoveryOpen] = useState(false);
  const [recoveryEmail, setRecoveryEmail] = useState('');
  const [recoveryEmailErr, setRecoveryEmailErr] = useState('');
  const [recoveryReason, setRecoveryReason] = useState('');
  const [recoverySent, setRecoverySent] = useState(false);
  const recoveryAct = useAction();
  const [forgotOpen, setForgotOpen] = useState(false);
  const [forgotEmail, setForgotEmail] = useState('');
  const [forgotEmailErr, setForgotEmailErr] = useState('');
  const forgotAct = useAction();
  const [resetOpen, setResetOpen] = useState(false);
  const [resetEmail, setResetEmail] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [resetValidationErr, setResetValidationErr] = useState('');
  const resetAct = useAction();
  const { setError: setResetError } = resetAct;
  const [googleTotpOpen, setGoogleTotpOpen] = useState(false);
  const [googleTotpEmail, setGoogleTotpEmail] = useState('');
  const [googleTotpCode, setGoogleTotpCode] = useState('');
  const googleTotpAct = useAction();
  const { setError: setGoogleTotpError } = googleTotpAct;
  const passkeyTurnstile = useTurnstile();
  const passwordTurnstile = useTurnstile();
  const recoveryTurnstile = useTurnstile();
  const forgotTurnstile = useTurnstile();

  useEffect(() => {
    api<DevUser[]>('/auth/dev-users').then(setDevUsers).catch(() => setDevUsers(null));
    api<{ enabled?: boolean }>('/auth/google/config')
      .then((res) => setGoogleEnabled(Boolean(res?.enabled)))
      .catch(() => setGoogleEnabled(false));
  }, []);

  useEffect(() => {
    const gErr = searchParams.get('google_error');
    const gReset = searchParams.get('google_reset');
    const gTotp = searchParams.get('google_totp');
    if (!gErr && !gReset && !gTotp) return;
    const next = new URLSearchParams(searchParams);
    next.delete('google_error');
    next.delete('google_reset');
    next.delete('google_totp');
    setSearchParams(next, { replace: true });
    if (gErr) {
      setOauthError(apiMessage({ code: gErr }, 'network'));
    }
    if (gReset === '1') {
      void api<{ valid?: boolean; email?: string }>('/auth/password-reset/status')
        .then((res) => {
          if (res?.valid) {
            setResetEmail(res.email ?? '');
            setNewPassword('');
            setConfirmPassword('');
            setResetValidationErr('');
            setResetError(null);
            setResetOpen(true);
          } else {
            setOauthError(apiMessage({ code: 'PASSWORD_RESET_INVALID' }, 'network'));
          }
        })
        .catch(() => {
          setOauthError(apiMessage({ code: 'PASSWORD_RESET_INVALID' }, 'network'));
        });
    }
    if (gTotp === '1') {
      void api<{ valid?: boolean; email?: string }>('/auth/google/totp/status')
        .then((res) => {
          if (res?.valid) {
            setGoogleTotpEmail(res.email ?? '');
            setGoogleTotpCode('');
            setGoogleTotpError(null);
            setGoogleTotpOpen(true);
          } else {
            setOauthError(apiMessage({ code: 'VERIFICATION_EXPIRED' }, 'network'));
          }
        })
        .catch(() => {
          setOauthError(apiMessage({ code: 'VERIFICATION_EXPIRED' }, 'network'));
        });
    }
  }, [searchParams, setSearchParams, setResetError, setGoogleTotpError]);

  if (me) return <Navigate to={from} replace />;

  const done = async () => {
    clearAutoLock();
    await refresh();
    nav(from, { replace: true });
  };

  const hasRecognizedDevicePasskey = passkeySupported() && Boolean(devicePasskey) && !switchingAccount;
  const inApp = detectInAppBrowser();

  const switchMode = (next: 'passkey' | 'password') => {
    setMode(next);
    setError(null);
    setOauthError(null);
    setNeedsTotp(false);
    setTotpCode('');
    if (next === 'password' && hasRecognizedDevicePasskey && devicePasskey && !email) {
      setEmail(devicePasskey.email);
    }
  };

  const onPasskey = (e: FormEvent) => {
    e.preventDefault();
    if (passkeyTurnstile.enabled && !passkeyTurnstile.token) return;
    setOauthError(null);
    setResetNotice(null);
    const targetEmail = (hasRecognizedDevicePasskey && devicePasskey ? devicePasskey.email : email).trim();
    run(async () => {
      clearClientSessionStorage();
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

  const onPasswordLogin = (e: FormEvent) => {
    e.preventDefault();
    if (passwordTurnstile.enabled && !passwordTurnstile.token) return;
    setOauthError(null);
    setResetNotice(null);
    run(async () => {
      clearClientSessionStorage();
      try {
        const body: Record<string, string> = {
          email: email.trim(),
          password,
        };
        if (totpCode.trim()) body.code = totpCode.trim();
        if (passwordTurnstile.token) body.turnstile_token = passwordTurnstile.token;
        await post('/auth/login', body);
        await done();
      } catch (err) {
        passwordTurnstile.reset();
        if (err instanceof ApiError && err.code === 'TOTP_REQUIRED') {
          setNeedsTotp(true);
        }
        throw err;
      }
    });
  };

  const onGoogleSignIn = () => {
    setError(null);
    setOauthError(null);
    setResetNotice(null);
    void googleAct.run(async () => {
      clearClientSessionStorage();
      const res = await post<{ url: string }>('/auth/google/start', {
        purpose: 'login',
        redirect: from,
      });
      if (res?.url) {
        window.location.assign(res.url);
      }
    });
  };

  const onTestAccount = (e: FormEvent) => {
    e.preventDefault();
    run(async () => {
      clearClientSessionStorage();
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

  const openForgotModal = () => {
    const prefill = (hasRecognizedDevicePasskey && devicePasskey ? devicePasskey.email : email).trim();
    setForgotEmail(prefill);
    setForgotEmailErr('');
    forgotAct.setError(null);
    forgotTurnstile.reset();
    setForgotOpen(true);
  };

  const closeForgotModal = () => {
    forgotTurnstile.reset();
    setForgotOpen(false);
    setForgotEmailErr('');
  };

  const submitForgotGoogle = async (e: FormEvent) => {
    e.preventDefault();
    const trimmed = forgotEmail.trim();
    if (trimmed && !trimmed.includes('@')) {
      setForgotEmailErr(t('recoveryEmailRequired'));
      return;
    }
    if (forgotTurnstile.enabled && !forgotTurnstile.token) return;
    setForgotEmailErr('');
    await forgotAct.run(async () => {
      try {
        const body: Record<string, string> = { purpose: 'password_reset' };
        if (trimmed) body.email = trimmed;
        if (forgotTurnstile.token) body.turnstile_token = forgotTurnstile.token;
        const res = await post<{ url: string }>('/auth/google/start', body);
        if (res?.url) {
          window.location.assign(res.url);
        }
      } catch (err) {
        forgotTurnstile.reset();
        throw err;
      }
    });
  };

  const submitPasswordReset = async (e: FormEvent) => {
    e.preventDefault();
    if (newPassword.length < 10) {
      setResetValidationErr(apiMessage({ code: 'PASSWORD_TOO_SHORT' }, 'network'));
      return;
    }
    if (newPassword !== confirmPassword) {
      setResetValidationErr(t('passwordsDoNotMatch'));
      return;
    }
    setResetValidationErr('');
    const ok = await resetAct.run(async () => {
      await post('/auth/password-reset/complete', { new_password: newPassword });
    });
    if (ok) {
      setResetOpen(false);
      setNewPassword('');
      setConfirmPassword('');
      setPassword('');
      setMode('password');
      if (resetEmail) setEmail(resetEmail);
      setResetNotice(t('passwordResetDone'));
    }
  };

  const submitGoogleTotp = async (e: FormEvent) => {
    e.preventDefault();
    const trimmed = googleTotpCode.trim();
    if (!trimmed) return;
    let redirectTarget = from;
    const ok = await googleTotpAct.run(async () => {
      const res = await post<{ ok?: boolean; redirect?: string }>('/auth/google/totp/complete', {
        code: trimmed,
      });
      if (res?.redirect) {
        redirectTarget = safeInternalPath(res.redirect, from);
      }
    });
    if (ok) {
      setGoogleTotpOpen(false);
      setGoogleTotpCode('');
      clearAutoLock();
      clearClientSessionStorage();
      await refresh();
      nav(redirectTarget, { replace: true });
    }
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

  const anyBusy = busy || googleAct.busy;
  const combinedError = error ?? googleAct.error ?? oauthError;

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

          {resetNotice && (
            <Notice tone="success" icon={<CheckCircle2 size={20} />}>
              {resetNotice}
            </Notice>
          )}

          {mode === 'passkey' ? (
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
                  disabled={anyBusy || (passkeyTurnstile.enabled && !passkeyTurnstile.token)}
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
                    disabled={anyBusy}
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
                    disabled={anyBusy}
                    onClick={() => {
                      setSwitchingAccount(false);
                      setEmail(devicePasskey.email);
                    }}
                  >
                    {t('useSavedPasskey', { email: devicePasskey.email })}
                  </button>
                ) : null}
                <button
                  type="button"
                  className="link-btn"
                  disabled={anyBusy}
                  onClick={() => switchMode('password')}
                >
                  {t('loginWithPassword')}
                </button>
                {passkeySupported() && (
                  <button type="button" className="link-btn" disabled={anyBusy} onClick={openRecoveryModal}>
                    {t('emailRecovery')}
                  </button>
                )}
              </div>

              {/* Without passkeys, recovery is the only way in, so it gets a full button. */}
              {!passkeySupported() && (
                <Button variant="secondary" type="button" block disabled={anyBusy} onClick={openRecoveryModal}>
                  {t('emailRecovery')}
                </Button>
              )}

              <div className="auth-footnote">
                <p className="muted small no-margin">{t('passkeyHelp')}</p>
                <Link to="/terms" className="small">{t('title', { ns: 'terms' })}</Link>
              </div>
            </form>
          ) : (
            <form className="stack" onSubmit={onPasswordLogin}>
              <Field label={t('email')}>
                <input
                  className="input"
                  type="email"
                  autoComplete="username"
                  required
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                />
              </Field>
              <Field label={t('passwordLabel')}>
                <input
                  className="input"
                  type="password"
                  autoComplete="current-password"
                  placeholder={t('passwordPlaceholder')}
                  required
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                />
              </Field>
              {needsTotp && (
                <Field label={t('totpCodeLabel')} hint={t('totpCodeHint')}>
                  <input
                    className="input"
                    type="text"
                    autoComplete="one-time-code"
                    required
                    value={totpCode}
                    onChange={(e) => setTotpCode(e.target.value)}
                  />
                </Field>
              )}

              <TurnstileWidget
                siteKey={passwordTurnstile.siteKey}
                action="password_login"
                resetKey={passwordTurnstile.resetKey}
                onToken={passwordTurnstile.setToken}
              />

              <Button
                variant="primary"
                size="lg"
                type="submit"
                block
                loading={busy}
                disabled={anyBusy || (passwordTurnstile.enabled && !passwordTurnstile.token)}
              >
                <Lock size={18} />
                {busy ? t('loggingIn') : t('login')}
              </Button>

              <div className="auth-alt-actions">
                <button
                  type="button"
                  className="link-btn"
                  disabled={anyBusy}
                  onClick={() => switchMode('passkey')}
                >
                  {t('usePasskey')}
                </button>
                <button
                  type="button"
                  className="link-btn"
                  disabled={anyBusy}
                  onClick={openForgotModal}
                >
                  {t('forgotPassword')}
                </button>
                <button type="button" className="link-btn" disabled={anyBusy} onClick={openRecoveryModal}>
                  {t('emailRecovery')}
                </button>
              </div>

              <div className="auth-footnote">
                <Link to="/terms" className="small">{t('title', { ns: 'terms' })}</Link>
              </div>
            </form>
          )}

          {googleEnabled && (
            <>
              <div className="auth-divider">{t('orContinueWith')}</div>
              <Button
                variant="secondary"
                size="lg"
                type="button"
                block
                className="btn-google"
                loading={googleAct.busy}
                disabled={anyBusy}
                onClick={onGoogleSignIn}
              >
                <GoogleMark />
                {t('loginWithGoogle')}
              </Button>
            </>
          )}

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
                <Button variant="secondary" type="submit" block disabled={anyBusy || !who}>
                  {busy ? t('loggingIn') : t('login')}
                </Button>
              </form>
            </div>
          )}

          <ErrorBox error={combinedError} />
        </div>
      </div>

      {forgotOpen && (
        <Modal title={t('forgotPasswordTitle')} onClose={closeForgotModal}>
          <div className="stack">
            <p className="muted small no-margin">{t('forgotPasswordHint')}</p>
            {googleEnabled ? (
              <form className="stack" onSubmit={submitForgotGoogle} noValidate>
                <div>
                  <Field label={t('email')}>
                    <input
                      className="input"
                      type="email"
                      autoComplete="email"
                      value={forgotEmail}
                      onChange={(e) => {
                        setForgotEmail(e.target.value);
                        if (forgotEmailErr) setForgotEmailErr('');
                      }}
                    />
                  </Field>
                  {forgotEmailErr && (
                    <div className="field-error-msg" role="alert">
                      {forgotEmailErr}
                    </div>
                  )}
                </div>
                <TurnstileWidget
                  siteKey={forgotTurnstile.siteKey}
                  action="password_reset"
                  resetKey={forgotTurnstile.resetKey}
                  onToken={forgotTurnstile.setToken}
                />
                <ErrorBox error={forgotAct.error} />
                <Button
                  variant="primary"
                  type="submit"
                  block
                  className="btn-google"
                  loading={forgotAct.busy}
                  disabled={forgotAct.busy || (forgotTurnstile.enabled && !forgotTurnstile.token)}
                >
                  <GoogleMark />
                  {t('forgotPasswordGoogleBtn')}
                </Button>
              </form>
            ) : (
              <Notice tone="info">{t('forgotPasswordGoogleUnavailable')}</Notice>
            )}
            <button
              type="button"
              className="link-btn"
              onClick={() => {
                closeForgotModal();
                openRecoveryModal();
              }}
            >
              {t('forgotPasswordUseRecovery')}
            </button>
          </div>
        </Modal>
      )}

      {resetOpen && (
        <Modal title={t('resetPasswordTitle')} onClose={() => setResetOpen(false)}>
          <form className="stack" onSubmit={submitPasswordReset}>
            <p className="muted small no-margin">{t('resetPasswordHint', { email: resetEmail })}</p>
            <Field label={t('newPassword')} hint={t('passwordMinHint')}>
              <input
                className="input"
                type="password"
                autoComplete="new-password"
                minLength={10}
                required
                value={newPassword}
                onChange={(e) => {
                  setNewPassword(e.target.value);
                  if (resetValidationErr) setResetValidationErr('');
                }}
              />
            </Field>
            <Field label={t('confirmPassword')}>
              <input
                className="input"
                type="password"
                autoComplete="new-password"
                minLength={10}
                required
                value={confirmPassword}
                onChange={(e) => {
                  setConfirmPassword(e.target.value);
                  if (resetValidationErr) setResetValidationErr('');
                }}
              />
            </Field>
            <ErrorBox error={resetValidationErr || resetAct.error} />
            <Button variant="primary" type="submit" block loading={resetAct.busy} disabled={resetAct.busy}>
              <KeyRound size={18} />
              {t('resetPasswordSubmit')}
            </Button>
          </form>
        </Modal>
      )}

      {googleTotpOpen && (
        <Modal title={t('googleTotpTitle')} onClose={() => setGoogleTotpOpen(false)}>
          <form className="stack" onSubmit={submitGoogleTotp}>
            <p className="muted small no-margin">{t('googleTotpHint', { email: googleTotpEmail })}</p>
            <Field label={t('totpCodeLabel')} hint={t('totpCodeHint')}>
              <input
                className="input"
                type="text"
                autoComplete="one-time-code"
                required
                value={googleTotpCode}
                onChange={(e) => setGoogleTotpCode(e.target.value)}
              />
            </Field>
            <ErrorBox error={googleTotpAct.error} />
            <Button variant="primary" type="submit" block loading={googleTotpAct.busy} disabled={googleTotpAct.busy}>
              <Lock size={18} />
              {t('googleTotpSubmit')}
            </Button>
          </form>
        </Modal>
      )}

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
