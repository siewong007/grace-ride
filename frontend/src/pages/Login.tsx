import { FormEvent, useEffect, useState } from 'react';
import { Link, Navigate, useLocation, useNavigate } from 'react-router-dom';
import { Fingerprint } from 'lucide-react';
import { api, post } from '../api';
import { useAuth } from '../auth';
import { useTranslation } from 'react-i18next';
import { useAction, ErrorBox, Field } from '../components/ui';
import { loginWithPasskey, passkeySupported, passkeyRegistered } from '../webauthn';

type DevUser = { name: string; email: string; roles: string };

export default function Login() {
  const { t } = useTranslation('auth');
  const { me, refresh, hasPassword, passkeyCount, mfa_ok, totp_enabled } = useAuth();
  const nav = useNavigate();
  const loc = useLocation();
  const from = (loc.state as { from?: string } | null)?.from ?? '/';
  const [email, setEmail] = useState('');
  const [who, setWho] = useState('');
  const [devUsers, setDevUsers] = useState<DevUser[] | null>(null);
  const [showPasskeySetup, setShowPasskeySetup] = useState(false);
  const [busy, error, run] = useAction();

  useEffect(() => {
    api<DevUser[]>('/auth/dev-users').then(setDevUsers).catch(() => setDevUsers(null));
  }, []);

  if (me) return <Navigate to={from} replace />;

  const done = async () => {
    await refresh();
    nav(from, { replace: true });
  };

  const onPasskey = async (e: FormEvent) => {
    e.preventDefault();
    run(async () => {
      await loginWithPasskey(email.trim());
      await done();
    });
  };

  const onEmailPassword = async (e: FormEvent) => {
    e.preventDefault();
    run(async () => {
      await post('/auth/login', { email: email.trim() });
      await done();
    });
  };

  const onSendVerification = async () => {
    run(async () => {
      await post('/auth/email/verify', { email: email.trim() });
    });
  };

  const onTestAccount = (e: FormEvent) => {
    e.preventDefault();
    run(async () => {
      await post('/auth/dev-login', { email: who });
      await done();
    });
  };

  const canUsePasskey = passkeySupported() && passkeyCount > 0;
  const canUseEmailPassword = hasPassword || totp_enabled;
  const needsVerification = !mfa_ok && passkeyCount === 0;

  return (
    <div className="auth-page">
      <div className="auth-card stack">
        <h2>{t('welcomeBack')}</h2>

        {/* Sign-in method selection }}
        <div className="method-selector" style={{ marginBottom: '1.5rem' }}>
          <h3>{t('chooseSignInMethod')}</h3>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: '1rem' }}>
            {/* Passkey option }}
            <div className="method-option" style={{ border: canUsePasskey ? '2px solid #10b981' : '2px solid #e5e7eb', borderRadius: '8px', padding: '1rem', background: canUsePasskey ? 'rgba(16, 185, 129, 0.1)' : 'transparent', cursor: canUsePasskey ? 'pointer' : 'default', transition: 'border-color 0.2s' }}>
              <div style={{ fontSize: '2rem', marginBottom: '0.5rem' }}}>{canUsePasskey ? <Fingerprint /> : '⛔'}</div>
              <h4>{t('passkey')}</h4>
              {canUsePasskey ? (
                <p className="muted tiny">{t('passkeyHelp')}</p>
              ) : (
                <p className="muted tiny" style={{ color: '#6b7280' }}>{t('noPasskey')} {passkeyCount === 0 ? '— ' + t('setupPasskeyFirst') : ''}</p>
              )}
            </div>

            {/* Email/Password option }}
            <div className="method-option" style={{ border: canUseEmailPassword ? '2px solid #3b82f6' : '2px solid #e5e7eb', borderRadius: '8px', padding: '1rem', background: canUseEmailPassword ? 'rgba(59, 130, 246, 0.1)' : 'transparent', cursor: canUseEmailPassword ? 'pointer' : 'default', transition: 'border-color 0.2s' }}>
              <div style={{ fontSize: '2rem', marginBottom: '0.5rem' }}}>{canUseEmailPassword ? '🔑' : 'placeholder'}</div>
              <h4>{t('password')}</h4>
              {canUseEmailPassword ? (
                <p className="muted tiny">{t('enterPassword')} {t('toContinue')}</p>
              ) : (
                <p className="muted tiny" style={{ color: '#6b7280' }}>{t('noPasswordSetUp')}</p>
              )}
            </div>

            {/* Email verification option }}
            <div className="method-option" style={{ border: needsVerification ? '2px solid #f59e0b' : '2px solid #e5e7eb', borderRadius: '8px', padding: '1rem', background: needsVerification ? 'rgba(245, 158, 11, 0.1)' : 'transparent', cursor: needsVerification ? 'pointer' : 'default', transition: 'border-color 0.2s' }}>
              <div style={{ fontSize: '2rem', marginBottom: '0.5rem' }}}>{needsVerification ? '✉️' : '📧'}</div>
              <h4>{t('emailVerification')}</h4>
              {needsVerification ? (
                <p className="muted tiny">
                  {t('verifyEmailToSignIn')} <a href="mailto:{email}" className="underline">{email}</a>{' '}
                </p>
              ) : (
                <p className="muted tiny">{t('emailAlreadyVerified')}</p>
              )}
            </div>
          </div>
        </div>

        {/* Passkey setup modal if triggered }}
        {showPasskeySetup && passkeySupported() && (
          <div className="modal fade-show" style={{ position: 'fixed', top: 0, left: 0, right: 0, bottom: 0, background: 'rgba(0,0,0,0.5)', display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 100 }}>
            <div className="auth-card stack" style={{ width: '90%', maxWidth: 400 }}>
              <h3>{t('setUpPasskey')}</h3>
              <p className="muted small">{t('passkeyHelp')}</p>
              <p className="muted small" style={{ margin: '1rem 0' }}>
                Please use your phone's biometric/FaceID to register a passkey.
              </p>
              <button className="btn primary block" onClick={() => {/* trigger passkey registration */}}>
                {t('registerPasskey')}
              </button>
              <button className="btn secondary block" style={{ marginTop: '0.5rem' }} onClick={() => setShowPasskeySetup(false)}>
                {t('skipForNow')}
              </button>
            </div>
          </div>
        )}

        {/* Passkey form (primary when available) }}
        {canUsePasskey && !showPasskeySetup && (
          <form className="stack" onSubmit={onPasskey}>
            <Field label={t('email')}> <input
              className="input"
              type="email"
              autoComplete="username webauthn"
              required
              value={email}
              onChange={(e) => setEmail(e.target.value)}
            /> </Field>
            <button className="btn primary block" disabled={busy}>
              <Fingerprint size={20} />{busy ? t('loggingIn') : t('loginWithPasskey')}
            </button>
          </form>
        )}

        {/* Email/Password form }}
        {canUseEmailPassword && !showPasskeySetup && (
          <form className="stack" onSubmit={onEmailPassword}>
            <Field label={t('email')}> <input
              className="input"
              type="email"
              autoComplete="username"
              required
              value={email}
              onChange={(e) => setEmail(e.target.value)}
            /> </Field>
            <Field label={t('password')}> <input
              className="input"
              type="password"
              required
            /> </Field>
            <button className="btn primary block" disabled={busy}">
              {t('login')}
            </button>
          </form>
        )}

        {/* Email verification prompt }}
        {needsVerification && !showPasskeySetup && !canUseEmailPassword && (
          <div style={{ marginTop: '1rem' }}>
            <p className="muted small">
              {t('toSignInYouNeedTo')}<br />
              {t('configureSignInMethod')}
            </p>
            <p className="muted small" style={{ marginTop: '0.5rem' }}>
              <a href="/auth/recovery-request" style={{ color: '#3b82f6', textDecoration: 'underline' }}>
                {t('useEmailToLogin')}
              </a>{' '}
              or{' '}
              <a
                onClick={() => setShowPasskeySetup(true)}
                style={{ color: '#3b82f6', textDecoration: 'underline' }}
              >
                {t('setUpPasskey')}
              </a>
            </p>
          </div>
        )}

        {/* Dev accounts (hidden from regular users) }}
        {devUsers && devUsers.length > 0 && !me && (
          <form className="stack" onSubmit={onTestAccount}>
            <Field label={t('testAccounts')} hint={t('testAccountsHint')}> <select className="input" required value={who} onChange={(e) => setWho(e.target.value)}>
              <option value="">{t('testChoose')}</option> {devUsers.map((u) => (<option key={u.email} value={u.email}>{u.name} · {u.email}</option>))} </select> </Field>
            <button className="btn block" disabled={busy || !who}>{busy ? t('loggingIn') : t('login')}</button> </form>
        )}

        <ErrorBox error={error} />
      </div>
    </div>
  );
}
