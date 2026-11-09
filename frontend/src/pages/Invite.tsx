import { FormEvent, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { Fingerprint } from 'lucide-react';
import { post, useApi } from '../api';
import { useAuth } from '../auth';
import { TermsOptIn, TermsText } from '../components/Terms';
import { ErrorBox, Field, Spinner, useAction } from '../components/ui';
import { passkeySupported, registerPasskey } from '../webauthn';
import { useTranslation } from 'react-i18next';

export default function Invite() {
  const { t } = useTranslation('invite');
  const { token } = useParams();
  const nav = useNavigate();
  const { refresh } = useAuth();
  const { data, error, loading } = useApi<{ kind: string; email: string; name: string }>(`/invites/${token}`);
  const [name, setName] = useState('');
  const [email, setEmail] = useState('');
  const [phone, setPhone] = useState('');
  const [terms, setTerms] = useState(false);
  const [step, setStep] = useState<'details' | 'passkey'>('details');
  const { busy, error: actErr, run } = useAction();

  if (loading) return <div className="auth-page"><Spinner /></div>;
  if (error || !data)
    return (
      <div className="auth-page">
        <div className="card auth-card"><ErrorBox error={error ?? t('invalidLink')} /></div>
      </div>
    );

  const recovery = data.kind === 'recovery';

  const accept = (e: FormEvent) => {
    e.preventDefault();
    run(async () => {
      await post(`/invites/${token}/accept`, recovery ? { terms: true } : { name: name.trim(), email: email.trim(), phone, terms: true });
      await refresh();
      setStep('passkey');
    });
  };

  const addPasskey = () =>
    run(async () => {
      await registerPasskey(t('myPhone'));
      await refresh();
      nav('/', { replace: true });
    }, t('passkeySet'));

  return (
    <div className="auth-page">
      <div className="card auth-card stack">
        <img src="/icon.svg" width={48} height={48} alt="" />
        {step === 'details' ? (
          <form className="stack" onSubmit={accept}>
            <div>
              <h2>{recovery ? t('recover') : t('join')}</h2>
              {recovery && data.email && <p className="muted">{data.email}</p>}
            </div>
            {recovery ? (
              <p>{t('recoverDesc')}</p>
            ) : (
              <>
                <Field label={t('name')}>
                  <input className="input" required autoComplete="name" value={name} onChange={(e) => setName(e.target.value)} />
                </Field>
                <Field label={t('email')}>
                  <input className="input" type="email" required autoComplete="email" value={email} onChange={(e) => setEmail(e.target.value)} />
                </Field>
                <Field label={t('phone')} hint={t('phoneHint')}>
                  <input className="input" type="tel" required value={phone} onChange={(e) => setPhone(e.target.value)} />
                </Field>
              </>
            )}
            <div className="card flat" style={{ maxHeight: 220, overflow: 'auto' }}>
              <TermsText />
            </div>
            <TermsOptIn checked={terms} onChange={setTerms} />
            <ErrorBox error={actErr} />
            <button className="btn primary block" disabled={busy || !terms}>{t('continue')}</button>
          </form>
        ) : (
          <div className="stack">
            <h2>{t('setupPasskey')}</h2>
            <p>{t('setupPasskeyDesc')}</p>
            {!passkeySupported() && <div className="notice">{t('noPasskey')}</div>}
            <ErrorBox error={actErr} />
            <button className="btn primary block" onClick={addPasskey} disabled={busy || !passkeySupported()}>
              <Fingerprint size={20} /> {t('setNow')}
            </button>
            <button className="btn ghost" onClick={() => nav('/', { replace: true })}>{t('setLater')}</button>
          </div>
        )}
      </div>
    </div>
  );
}
