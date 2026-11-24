import { FormEvent, useEffect, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { Fingerprint } from 'lucide-react';
import { ApiError, post, useApi } from '../api';
import { useAuth } from '../auth';
import { clearAutoLock } from '../components/AutoLock';
import { clearOfflineSnapshots } from '../offlineSnapshot';
import { LogoMark, Wordmark } from '../components/Brand';
import { SwipeBackGesture } from '../components/BackNavigation';
import { PublicDisplayBar } from '../components/AppChrome';
import { TermsOptIn, TermsText } from '../components/Terms';
import TurnstileWidget from '../components/TurnstileWidget';
import { Button, ErrorBox, Field, Spinner, useAction, useConfirm } from '../components/ui';
import { useTurnstile } from '../turnstile';
import { passkeySupported, registerPasskey } from '../webauthn';
import { useTranslation } from 'react-i18next';

type InvitePreview = {
  kind: string;
  email: string;
  name: string;
  family_name?: string;
  phone?: string;
};

type Step = 'details' | 'terms' | 'passkey';
type DetailField = 'name' | 'email' | 'phone';

/** Server error codes that belong to a field on the details step. */
const FIELD_FOR_CODE: Record<string, DetailField> = {
  NAME_REQUIRED: 'name',
  NAME_TOO_LONG: 'name',
  EMAIL_INVALID: 'email',
  EMAIL_ALREADY_REGISTERED: 'email',
  PHONE_INVALID: 'phone',
};

/** Same limits as `check_name` / `check_phone` on the server. */
const MAX_NAME_CHARS = 60;
const PHONE_CHARS = { min: 10, max: 20 };

export default function Invite() {
  const { t } = useTranslation('invite');
  const { token } = useParams();
  const nav = useNavigate();
  const { refresh } = useAuth();
  const confirm = useConfirm();
  const { data, error, loading } = useApi<InvitePreview>(`/invites/${token}`);
  const [name, setName] = useState('');
  const [email, setEmail] = useState('');
  const [phone, setPhone] = useState('');
  const [terms, setTerms] = useState(false);
  const [stepState, setStep] = useState<Step>('details');
  const [fieldErrors, setFieldErrors] = useState<{ name?: string; email?: string; phone?: string; terms?: string }>({});
  const [emailTaken, setEmailTaken] = useState(false);
  const { busy, error: actErr, run } = useAction();
  const inviteTurnstile = useTurnstile();

  // An invite made for a named person arrives with their details filled in.
  useEffect(() => {
    if (!data || data.kind === 'recovery') return;
    if (data.name) setName((v) => v || data.name);
    if (data.email) setEmail((v) => v || data.email);
    if (data.kind === 'family' && data.phone) setPhone(data.phone);
  }, [data]);

  if (loading) return <div className="auth-page"><Spinner /></div>;
  if (error || !data)
    return (
      <div className="auth-page">
        <div className="card auth-card"><ErrorBox error={error ?? t('invalidLink')} /></div>
      </div>
    );

  const recovery = data.kind === 'recovery';
  // Recovery keeps the member's existing details, so it starts at the terms.
  const step: Step = recovery && stepState === 'details' ? 'terms' : stepState;
  const steps: Step[] = recovery ? ['terms', 'passkey'] : ['details', 'terms', 'passkey'];
  const stepIdx = steps.indexOf(step) + 1;

  const validateDetails = (): boolean => {
    const errs: { name?: string; email?: string; phone?: string } = {};
    if (!name.trim()) {
      errs.name = t('nameRequired');
    } else if ([...name.trim()].length > MAX_NAME_CHARS) {
      errs.name = t('nameTooLong', { max: MAX_NAME_CHARS });
    }
    const emailTrimmed = email.trim();
    if (!emailTrimmed || !/\S+@\S+\.\S+/.test(emailTrimmed)) {
      errs.email = t('emailInvalid');
    }
    const familyPhone = data.kind === 'family';
    const phoneChars = [...phone.trim()].length;
    if (
      !familyPhone &&
      (phoneChars < PHONE_CHARS.min || phoneChars > PHONE_CHARS.max || phone.replace(/\D/g, '').length < 8)
    ) {
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
    if (inviteTurnstile.enabled && !inviteTurnstile.token) return;
    run(async () => {
      const baseBody = recovery
        ? { terms: true }
        : { name: name.trim(), email: email.trim(), phone: phone.trim(), terms: true };
      const body = inviteTurnstile.token
        ? { ...baseBody, turnstile_token: inviteTurnstile.token }
        : baseBody;
      try {
        await post(`/invites/${token}/accept`, body);
      } catch (e) {
        inviteTurnstile.reset();
        // A detail the server refused goes back to its field on step 1, not a banner on step 2.
        const field = e instanceof ApiError && e.code ? FIELD_FOR_CODE[e.code] : undefined;
        if (!field || recovery) throw e;
        setFieldErrors({ [field]: e instanceof Error ? e.message : String(e) });
        setEmailTaken(e instanceof ApiError && e.code === 'EMAIL_ALREADY_REGISTERED');
        setStep('details');
        return;
      }
      clearAutoLock();
      clearOfflineSnapshots();
      await refresh();
      setStep('passkey');
    });
  };

  const goBackToDetails = () => {
    inviteTurnstile.reset();
    setStep('details');
  };

  const addPasskey = () =>
    run(async () => {
      await registerPasskey(t('myPhone'), email);
      await refresh();
      nav('/', { replace: true });
    }, t('passkeySet'));

  // There is no password: skipping the passkey means no way back in after logging out.
  const skipPasskey = async () => {
    const ok = await confirm({
      title: t('skipTitle'),
      body: t('skipBody'),
      confirmLabel: t('skipConfirm'),
      cancelLabel: t('skipCancel'),
      tone: 'danger',
    });
    if (ok) nav('/', { replace: true });
  };

  const stepTitle =
    step === 'details'
      ? t('stepDetails')
      : step === 'terms'
      ? t('stepTerms')
      : t('stepPasskey');

  return (
    <div className="auth-page">
      <div className="card auth-card stack">
        <div className="row between auth-header-row">
          <div className="row nowrap auth-brand">
            <LogoMark size={36} />
            <Wordmark />
          </div>
          <PublicDisplayBar />
        </div>

        {/* 3-step progress stepper */}
        <div className="stepper-wrap">
          <div className="row between stepper-info">
            <span className="stepper-counter">{t('stepOf', { current: stepIdx, total: steps.length })}</span>
            <span className="stepper-name bold">{stepTitle}</span>
          </div>
          <div
            className="progress-bar"
            role="progressbar"
            aria-label={stepTitle}
            aria-valuenow={stepIdx}
            aria-valuemin={1}
            aria-valuemax={steps.length}
          >
            <div className={`progress-fill step-${stepIdx}`} style={{ width: `${(stepIdx / steps.length) * 100}%` }} />
          </div>
        </div>

        {/* Step 1: Details */}
        {step === 'details' && (
          <form className="stack" onSubmit={handleDetailsNext}>
            <div>
              <h1 className="auth-title">{t('join')}</h1>
              {data.kind === 'family' && data.family_name ? <p>{data.family_name}</p> : null}
            </div>

            <Field label={t('name')}>
              <input
                className={`input ${fieldErrors.name ? 'invalid' : ''}`}
                required
                maxLength={MAX_NAME_CHARS}
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
                  setEmailTaken(false);
                  if (fieldErrors.email) setFieldErrors({ ...fieldErrors, email: undefined });
                }}
              />
              {fieldErrors.email && <span className="field-error" role="alert">{fieldErrors.email}</span>}
              {emailTaken && (
                <span className="small">
                  {t('emailTakenHint')} <Link to="/login">{t('signInInstead')}</Link>
                </span>
              )}
            </Field>

            <Field label={t('phone')} hint={data.kind === 'family' ? undefined : t('phoneHint')}>
              <input
                className={`input ${fieldErrors.phone ? 'invalid' : ''}`}
                type="tel"
                required
                autoComplete="tel"
                maxLength={PHONE_CHARS.max}
                readOnly={data.kind === 'family'}
                value={phone}
                onChange={(e) => {
                  if (data.kind === 'family') return;
                  setPhone(e.target.value);
                  if (fieldErrors.phone) setFieldErrors({ ...fieldErrors, phone: undefined });
                }}
              />
              {fieldErrors.phone && <span className="field-error" role="alert">{fieldErrors.phone}</span>}
            </Field>

            <Button variant="primary" type="submit" block>
              {t('continue')}
            </Button>
          </form>
        )}

        {/* Step 2: Terms */}
        {step === 'terms' && (
          <form className="stack" onSubmit={handleAcceptTerms}>
            <div>
              <h1 className="auth-title">{recovery ? t('recover') : t('stepTerms')}</h1>
              {recovery && data.email && <p className="muted">{data.email}</p>}
            </div>
            {recovery && <p>{t('recoverDesc')}</p>}
            <div className="card flat terms-scroll">
              <TermsText heading="h3" />
            </div>
            <TermsOptIn checked={terms} onChange={(v) => { setTerms(v); if (v) setFieldErrors({ ...fieldErrors, terms: undefined }); }} />
            {fieldErrors.terms && <span className="field-error" role="alert">{fieldErrors.terms}</span>}
            <TurnstileWidget
              siteKey={inviteTurnstile.siteKey}
              action="invite_accept"
              resetKey={inviteTurnstile.resetKey}
              onToken={inviteTurnstile.setToken}
            />
            <ErrorBox error={actErr} />
            <div className="btn-row">
              {!recovery && (
                <Button variant="secondary" type="button" onClick={goBackToDetails}>
                  {t('back')}
                </Button>
              )}
              <Button
                variant="primary"
                type="submit"
                loading={busy}
                disabled={busy || !terms || (inviteTurnstile.enabled && !inviteTurnstile.token)}
                block={recovery}
              >
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
            <Button variant="ghost" block onClick={skipPasskey} disabled={busy}>
              {t('setLater')}
            </Button>
          </div>
        )}
      </div>
      <SwipeBackGesture enabled={step === 'terms' && !recovery} onBack={goBackToDetails} />
    </div>
  );
}
