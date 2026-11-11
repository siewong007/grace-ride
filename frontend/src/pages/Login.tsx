import { FormEvent, useEffect, useState } from 'react';
import { Link, Navigate, useLocation, useNavigate } from 'react-router-dom';
import { Fingerprint, Globe } from 'lucide-react';
import { api, post } from '../api';
import { useAuth } from '../auth';
import { useTranslation } from 'react-i18next';
import { setLanguage, SupportedLanguage } from '../i18n';
import { useAction, ErrorBox, Field } from '../components/ui';
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

  return (
    <div className="auth-page">
      <div className="auth-card stack">
        <div className="auth-header-row row between">
          <img src="/icon.svg" width={36} height={36} alt="" />
          <button
            type="button"
            className="btn sm ghost lang-toggle"
            onClick={() => setLanguage(currentLang === 'zh' ? 'en' : 'zh')}
            aria-label={t('lang')}
          >
            <Globe size={16} aria-hidden="true" />
            <span>{currentLang === 'zh' ? t('en', { ns: 'chrome' }) : t('zh', { ns: 'chrome' })}</span>
          </button>
        </div>
        <h2>{t('welcomeBack')}</h2>

        {/* Passkey as default primary login */}
        <form className="stack" onSubmit={onPasskey}>
          <Field label={t('email')}> <input
              className="input"
              type="email"
              autoComplete="username webauthn"
              required
              value={email}
              onChange={(e) => setEmail(e.target.value)}
            /> </Field>
          {passkeySupported() ? (
            <button className="btn primary block" disabled={busy}>
              <Fingerprint size={20} />{busy ? t('loggingIn') : t('loginWithPasskey')}
            </button>
          ) : (
            <div className="notice">{t('noPasskey')}</div>
          )}
          <p className="muted small">{t('passkeyHelp')}</p>
          <Link to="/terms">{t('title', { ns: 'terms' })}</Link>
        </form>

        {/* Link to other login methods */}
        <div className="other-methods">
          <span>{t('or')}</span>
          <a
            href="/api/auth/recovery-request"
            onClick={(e) => {
              e.preventDefault();
              run(async () => {
                await post('/auth/recovery-request', { email, reason: '' });
              });
            }}
          >{t('useEmailToLogin')}</a>
        </div>

        {devUsers && devUsers.length > 0 && (
          <form className="stack" onSubmit={onTestAccount}>
            <Field label={t('testAccounts')} hint={t('testAccountsHint')}> <select className="input" required value={who} onChange={(e) => setWho(e.target.value)}> <option value="">{t('testChoose')}</option> {devUsers.map((u) => (<option key={u.email} value={u.email}>{u.name} · {u.email}</option>))} </select> </Field> <button className="btn block" disabled={busy || !who}>{busy ? t('loggingIn') : t('login')}</button> </form>
        )}

        <ErrorBox error={error} />
      </div>
    </div>
  );
}
