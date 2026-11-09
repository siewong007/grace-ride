import { FormEvent, useEffect, useState } from 'react';
import { Navigate, useLocation, useNavigate } from 'react-router-dom';
import { Fingerprint } from 'lucide-react';
import { api, post } from '../api';
import { useAuth } from '../auth';
import { ErrorBox, Field, Modal, useAction } from '../components/ui';
import { loginWithPasskey, passkeySupported } from '../webauthn';
import { useTranslation } from 'react-i18next';

type DevUser = { name: string; email: string; roles: string };

export default function Login() {
  const { t } = useTranslation('auth');
  const { me, refresh } = useAuth();
  const nav = useNavigate();
  const loc = useLocation();
  const from = (loc.state as { from?: string } | null)?.from ?? '/';
  const steps = t('steps', { returnObjects: true });
  const stepList = Array.isArray(steps) ? steps : [];
  const [email, setEmail] = useState('');
  const [who, setWho] = useState('');
  const [devUsers, setDevUsers] = useState<DevUser[] | null>(null);
  const [recover, setRecover] = useState(false);
  const { busy, error, run } = useAction();

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
      <div className="auth-split">
        <div className="auth-aside stack">
          <img src="/icon.svg" width={64} height={64} alt="" />
          <h1 style={{ fontSize: '2.4rem' }}>{t('tagline')}</h1>
          <p className="muted" style={{ fontSize: '1.15rem' }}>
            {t('desc')}
          </p>
          <ul className="steps">
            {stepList.map((s) => <li key={s}>{s}</li>)}
          </ul>
        </div>

        <div className="card auth-card stack">
          <div className="row nowrap">
            <img src="/icon.svg" width={44} height={44} alt="" />
            <div>
              <h2>{t('welcomeBack')}</h2>
              <p className="muted small">{t('membersOnly')}</p>
            </div>
          </div>

          <form className="stack" onSubmit={onPasskey}>
            <Field label={t('email')}>
              <input className="input" type="email" autoComplete="username webauthn" required value={email} onChange={(e) => setEmail(e.target.value)} />
            </Field>
            {passkeySupported() ? (
              <button className="btn primary block" disabled={busy}>
                <Fingerprint size={20} /> {busy ? t('loggingIn') : t('loginWithPasskey')}
              </button>
            ) : (
              <div className="notice">{t('noPasskey')}</div>
            )}
            <p className="muted small">{t('passkeyHelp')}</p>
          </form>
          {devUsers && devUsers.length > 0 && (
            <form className="stack" onSubmit={onTestAccount}>
              <Field label={t('testAccounts')} hint={t('testAccountsHint')}>
                <select className="input" required value={who} onChange={(e) => setWho(e.target.value)}>
                  <option value="">{t('testChoose')}</option>
                  {devUsers.map((u) => (
                    <option key={u.email} value={u.email}>{u.name} · {u.email}</option>
                  ))}
                </select>
              </Field>
              <button className="btn block" disabled={busy || !who}>{busy ? t('loggingIn') : t('login')}</button>
            </form>
          )}
          <ErrorBox error={error} />
          <button className="btn ghost" onClick={() => setRecover(true)}>{t('recover')}</button>
        </div>
      </div>
      {recover && <RecoveryModal onClose={() => setRecover(false)} />}
    </div>
  );
}

function RecoveryModal({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation('auth');
  const [email, setEmail] = useState('');
  const [reason, setReason] = useState('');
  const [sent, setSent] = useState(false);
  const { busy, error, run } = useAction();
  return (
    <Modal title={t('recoveryTitle')} onClose={onClose}>
      {sent ? (
        <div className="notice ok">{t('recoveryReceived')}</div>
      ) : (
        <form className="stack" onSubmit={(e) => { e.preventDefault(); run(async () => { await post('/auth/recovery-request', { email, reason }); setSent(true); }); }}>
          <Field label={t('recoveryEmail')}>
            <input className="input" type="email" required value={email} onChange={(e) => setEmail(e.target.value)} />
          </Field>
          <Field label={t('recoveryReason')} hint={t('recoveryReasonHint')}>
            <textarea className="input" required value={reason} onChange={(e) => setReason(e.target.value)} />
          </Field>
          <ErrorBox error={error} />
          <button className="btn primary" disabled={busy}>{t('submitRecovery')}</button>
        </form>
      )}
    </Modal>
  );
}
