import { FormEvent, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { Fingerprint } from 'lucide-react';
import { post, useApi } from '../api';
import { useAuth } from '../auth';
import { LogoMark, Wordmark } from '../components/Brand';
import { TermsOptIn, TermsText } from '../components/Terms';
import { Button, ErrorBox, Field, Spinner, useAction } from '../components/ui';
import { passkeySupported, registerPasskey } from '../webauthn';
import { useTranslation } from 'react-i18next';

type Step = 'details' | 'terms' | 'passkey';

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
  const [step, setStep] = useState<Step>('details');
  const [fieldErrors, setFieldErrors] = useState<{ name?: string; email?: string; phone?: string; terms?: string }>({});
  const { busy, error: actErr, run } = useAction();

  if (loading) return <div className="auth-page"><Spinner /></div>;
  if (error || !data)
    return (
      <div className="auth-page">
        <div className="card auth-card"><ErrorBox error={error ?? t('invalidLink')} /></div>
      </div>
    );

  const recovery = data.kind === 'recovery';
  const stepIdx = step === 'details' ? 1 : step === 'terms' ? 2 : 3;

  const validateDetails = (): boolean => {
    if (recovery) return true;
    const errs: { name?: string; email?: string; phone?: string } = {};
    if (!name.trim()) {
      errs.name = t('nameRequired');
    }
    const emailTrimmed = email.trim();
    if (!emailTrimmed || !/\S+@\S+\.\S+/.test(emailTrimmed)) {
      errs.email = t('emailInvalid');
    }
    const cleanPhone = phone.replace(/\D/g, '');
    if (!phone.trim() || cleanPhone.length < 8) {
      errs.phone = t('phoneInvalid');
    }
    setFieldErrors(errs);
    return Object.keys(errs).length === 0;
  };

  const handleDetailsNext = (e: FormEvent) => {
    e.preventDefault();
    if (validateDetails()) {
      setStep('terms');
    }
  };

  const handleAcceptTerms = (e: FormEvent) => {
    e.preventDefault();
    if (!terms) {
      setFieldErrors({ terms: t('termsRequired') });
      return;
    }
    run(async () => {
      await post(
        `/invites/${token}/accept`,
        recovery ? { terms: true } : { name: name.trim(), email: email.trim(), phone: phone.trim(), terms: true }
      );
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

  const stepTitle =
    step === 'details'
      ? (recovery ? t('recover') : t('stepDetails'))
      : step === 'terms'
      ? t('stepTerms')
      : t('stepPasskey');

  return (
    <div className="auth-page">
      <div className="card auth-card stack">
        <div className="row nowrap auth-brand">
          <LogoMark size={36} />
          <Wordmark />
        </div>

        {/* 3-step progress stepper */}
        <div className="stepper-wrap">
          <div className="row between stepper-info">
            <span className="stepper-counter">{t('stepOf', { current: stepIdx, total: 3 })}</span>
            <span className="stepper-name bold">{stepTitle}</span>
          </div>
          <div className="progress-bar" role="progressbar" aria-valuenow={stepIdx} aria-valuemin={1} aria-valuemax={3}>
            <div className={`progress-fill step-${stepIdx}`} />
          </div>
        </div>

        {/* Step 1: Details */}
        {step === 'details' && (
          <form className="stack" onSubmit={handleDetailsNext}>
            <div>
              <h1 className="auth-title">{recovery ? t('recover') : t('join')}</h1>
              {recovery && data.email && <p className="muted">{data.email}</p>}
            </div>

            {recovery ? (
              <p>{t('recoverDesc')}</p>
            ) : (
              <>
                <Field label={t('name')}>
                  <input
                    className={`input ${fieldErrors.name ? 'invalid' : ''}`}
                    required
                    autoComplete="name"
                    value={name}
                    onChange={(e) => {
                      setName(e.target.value);
                      if (fieldErrors.name) setFieldErrors({ ...fieldErrors, name: undefined });
                    }}
                  />
                  {fieldErrors.name && <span className="field-error" role="alert">{fieldErrors.name}</span>}
                </Field>

                <Field label={t('email')}>
                  <input
                    className={`input ${fieldErrors.email ? 'invalid' : ''}`}
                    type="email"
                    required
                    autoComplete="email"
                    value={email}
                    onChange={(e) => {
                      setEmail(e.target.value);
                      if (fieldErrors.email) setFieldErrors({ ...fieldErrors, email: undefined });
                    }}
                  />
                  {fieldErrors.email && <span className="field-error" role="alert">{fieldErrors.email}</span>}
                </Field>

                <Field label={t('phone')} hint={t('phoneHint')}>
                  <input
                    className={`input ${fieldErrors.phone ? 'invalid' : ''}`}
                    type="tel"
                    required
                    value={phone}
                    onChange={(e) => {
                      setPhone(e.target.value);
                      if (fieldErrors.phone) setFieldErrors({ ...fieldErrors, phone: undefined });
                    }}
                  />
                  {fieldErrors.phone && <span className="field-error" role="alert">{fieldErrors.phone}</span>}
                </Field>
              </>
            )}

            <Button variant="primary" type="submit" block>
              {t('continue')}
            </Button>
          </form>
        )}

        {/* Step 2: Terms */}
        {step === 'terms' && (
          <form className="stack" onSubmit={handleAcceptTerms}>
            <div>
              <h1 className="auth-title">{t('stepTerms')}</h1>
            </div>
            <div className="card flat terms-scroll">
              <TermsText heading="h3" />
            </div>
            <TermsOptIn checked={terms} onChange={(v) => { setTerms(v); if (v) setFieldErrors({ ...fieldErrors, terms: undefined }); }} />
            {fieldErrors.terms && <span className="field-error" role="alert">{fieldErrors.terms}</span>}
            <ErrorBox error={actErr} />
            <div className="btn-row">
              <Button variant="secondary" type="button" onClick={() => setStep('details')}>
                {t('back')}
              </Button>
              <Button variant="primary" type="submit" loading={busy} disabled={busy || !terms}>
                {t('agreeAndContinue')}
              </Button>
            </div>
          </form>
        )}

        {/* Step 3: Passkey */}
        {step === 'passkey' && (
          <div className="stack">
            <h1 className="auth-title">{t('setupPasskey')}</h1>
            <div className="passkey-explainer-card row nowrap">
              <Fingerprint size={28} className="passkey-icon" aria-hidden="true" />
              <p className="passkey-explainer-text">{t('passkeyExplainer')}</p>
            </div>
            <p className="muted small">{t('setupPasskeyDesc')}</p>
            {!passkeySupported() && <div className="notice warning">{t('noPasskey')}</div>}
            <ErrorBox error={actErr} />
            <Button
              variant="primary"
              size="lg"
              block
              onClick={addPasskey}
              disabled={busy || !passkeySupported()}
              loading={busy}
            >
              <Fingerprint size={20} />
              {t('setNow')}
            </Button>
            <Button variant="ghost" block onClick={() => nav('/', { replace: true })}>
              {t('setLater')}
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
