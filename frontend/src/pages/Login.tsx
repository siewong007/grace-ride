import { FormEvent, useEffect, useState } from 'react';
import { Link, Navigate, useLocation, useNavigate } from 'react-router-dom';
import { Fingerprint } from 'lucide-react';
import { api, post } from '../api';
import { useAuth } from '../auth';
import { useTranslation } from 'react-i18next';
import { LogoMark, Wordmark } from '../components/Brand';
import { PublicDisplayBar } from '../components/AppChrome';
import { Button, ErrorBox, Field, useAction } from '../components/ui';
import { loginWithPasskey, passkeySupported } from '../webauthn';
import { clearAutoLock } from '../components/AutoLock';

type DevUser = { name: string; email: string; roles: string };

export default function Login() {
  const { t } = useTranslation('auth');
  const { me, refresh } = useAuth();
  const nav = useNavigate();
  const loc = useLocation();
  const from = (loc.state as { from?: string } | null)?.from ?? '/';
  const [email, setEmail] = useState('');
  const [who, setWho] = useState('');
  const [devUsers, setDevUsers] = useState<DevUser[] | null>(null);
  const { busy, error, run } = useAction();

  useEffect(() => {
    api<DevUser[]>('/auth/dev-users').then(setDevUsers).catch(() => setDevUsers(null));
  }, []);

  if (me) return <Navigate to={from} replace />;

  const done = async () => {
    clearAutoLock();
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
