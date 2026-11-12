import { FormEvent, useEffect, useState } from 'react';
import { Link, Navigate, useLocation, useNavigate } from 'react-router-dom';
import { Fingerprint, Globe } from 'lucide-react';
import { api, post } from '../api';
import { useAuth } from '../auth';
import { useTranslation } from 'react-i18next';
import { setLanguage, SupportedLanguage } from '../i18n';
import { LogoMark, Wordmark } from '../components/Brand';
import { Button, ErrorBox, Field, useAction } from '../components/ui';
import { loginWithPasskey, passkeySupported } from '../webauthn';

type DevUser = { name: string; email: string; roles: string };

export default function Login() {
  const { t, i18n } = useTranslation('auth');
  const { me, refresh } = useAuth();
  const nav = useNavigate();
  const loc = useLocation();
  const from = (loc.state as { from?: string } | null)?.from ?? '/';
  const [email, setEmail] = useState('');
  const [who, setWho] = useState('');
  const [devUsers, setDevUsers] = useState<DevUser[] | null>(null);
  const { busy, error, run } = useAction();
  const currentLang: SupportedLanguage = i18n.language?.startsWith('zh') ? 'zh' : 'en';

  useEffect(() => {
    api<DevUser[]>('/auth/dev-users').then(setDevUsers).catch(() => setDevUsers(null));
  }, []);

  if (me) return <Navigate to={from} replace />;

  const done = async () => {
    await refresh();
    nav(from, { replace: true });
  };

  const onPasskey = (e: FormEvent) => {
    e.preventDefault();
    run(async () => {
      await loginWithPasskey(email.trim());
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

  const onEmailRecovery = () => {
    run(async () => {
      await post('/auth/recovery-request', { email: email.trim(), reason: '' });
    }, t('recoveryReceived'));
  };

  return (
    <div className="login-page-wrap">
      <div className="login-split">
        {/* Brand panel on ≥900px */}
        <div className="login-brand-panel">
          <div className="login-brand-content stack">
            <div className="row nowrap login-brand-header">
              <LogoMark size={44} mono />
              <div>
                <h1 className="login-brand-title"><Wordmark mono /></h1>
                <p className="login-brand-sub">{t('brandSub', { ns: 'layout' })}</p>
              </div>
            </div>
            <p className="login-welcome-tagline">{t('welcomeBilingual')}</p>
            <div className="login-illustration-wrap" aria-hidden="true">
              <svg className="login-road-svg" viewBox="0 0 320 180" fill="none" xmlns="http://www.w3.org/2000/svg">
                <path d="M 20 160 Q 160 20 300 160" stroke="currentColor" strokeWidth="20" strokeLinecap="round" className="road-path" />
                <path d="M 20 160 Q 160 20 300 160" stroke="currentColor" strokeWidth="3" strokeDasharray="10 10" className="road-stripes" />
                <circle cx="160" cy="65" r="14" fill="currentColor" className="road-sun" />
              </svg>
            </div>
          </div>
        </div>

        {/* Right card form panel */}
        <div className="card auth-card stack">
          <div className="auth-header-row row between">
            <div className="row nowrap hide-desktop auth-brand">
              <LogoMark size={36} />
              <Wordmark />
            </div>
            <button
              type="button"
              className="btn sm ghost lang-toggle"
              onClick={() => setLanguage(currentLang === 'zh' ? 'en' : 'zh')}
            >
              <Globe size={16} aria-hidden="true" />
              <span>{currentLang === 'zh' ? t('en', { ns: 'chrome' }) : t('zh', { ns: 'chrome' })}</span>
            </button>
          </div>

          <h2>{t('welcomeBack')}</h2>

          {/* Passkey primary login form */}
          <form className="stack" onSubmit={onPasskey}>
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

            {passkeySupported() ? (
              <Button variant="primary" size="lg" type="submit" block loading={busy} disabled={busy}>
                <Fingerprint size={20} />
                {busy ? t('loggingIn') : t('loginWithPasskey')}
              </Button>
            ) : (
              <div className="notice warning">{t('noPasskey')}</div>
            )}

            <Button
              variant="ghost"
              type="button"
              block
              disabled={busy}
              onClick={onEmailRecovery}
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
    </div>
  );
}
