import { FormEvent, useState } from 'react';
import { Navigate, useLocation, useNavigate } from 'react-router-dom';
import { Fingerprint, KeyRound } from 'lucide-react';
import { post } from '../api';
import { useAuth } from '../auth';
import { ErrorBox, Field, Modal, useAction } from '../components/ui';
import { loginWithPasskey, passkeySupported } from '../webauthn';
import { useTranslation } from 'react-i18next';

export default function Login() {
  const { t } = useTranslation('auth');
  const { me, refresh } = useAuth();
  const nav = useNavigate();
  const loc = useLocation();
  const from = (loc.state as { from?: string } | null)?.from ?? '/';
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [code, setCode] = useState('');
  const [needCode, setNeedCode] = useState(false);
  const [mode, setMode] = useState<'passkey' | 'password'>(passkeySupported() ? 'passkey' : 'password');
  const [recover, setRecover] = useState(false);
  const { busy, error, run } = useAction();

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

  const onPassword = (e: FormEvent) => {
    e.preventDefault();
    run(async () => {
      const r = await post<{ mfa_required?: boolean }>('/auth/login', { email: email.trim(), password, code: needCode ? code : undefined });
      if (r.mfa_required) {
        setNeedCode(true);
        return;
      }
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
            {(t('steps') as unknown as string[]).map((s: string) => <li key={s}>{s}</li>)}
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

          <div className="tabs" role="tablist">
            <button className={mode === 'passkey' ? 'on' : ''} onClick={() => setMode('passkey')} role="tab">
              {t('passkey')}
            </button>
            <button className={mode === 'password' ? 'on' : ''} onClick={() => setMode('password')} role="tab">
              {t('password')}
            </button>
          </div>

          {mode === 'passkey' ? (
            <form className="stack" onSubmit={onPasskey}>
              <Field label={t('email')}>
                <input className="input" type="email" autoComplete="username webauthn" required value={email} onChange={(e) => setEmail(e.target.value)} />
              </Field>
              <button className="btn primary block" disabled={busy}>
                <Fingerprint size={20} /> {busy ? t('loggingIn') : t('loginWithPasskey')}
              </button>
              <p className="muted small">{t('passkeyHelp')}</p>
            </form>
          ) : (
            <form className="stack" onSubmit={onPassword}>
              <Field label={t('email')}>
                <input className="input" type="email" autoComplete="username" required value={email} onChange={(e) => setEmail(e.target.value)} />
              </Field>
              <Field label={t('passwordField')}>
                <input className="input" type="password" autoComplete="current-password" required value={password} onChange={(e) => setPassword(e.target.value)} />
              </Field>
              {needCode && (
                <Field label={t('mfaField')} hint={t('mfaHint')}>
                  <input className="input" inputMode="numeric" autoComplete="one-time-code" maxLength={6} autoFocus value={code} onChange={(e) => setCode(e.target.value)} />
                </Field>
              )}
              <button className="btn primary block" disabled={busy}>
                <KeyRound size={20} /> {busy ? t('loggingIn') : t('login')}
              </button>
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
