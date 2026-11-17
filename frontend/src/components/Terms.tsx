import { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { ArrowLeft } from 'lucide-react';
import { post } from '../api';
import { useAuth } from '../auth';
import { SwipeBackGesture, useSmartBack } from './BackNavigation';
import { Button, ErrorBox, useAction } from './ui';
import { useTranslation } from 'react-i18next';
import ms from '../locales/terms.ms';

export const TERMS_SECTIONS = [
  'service', 'who', 'controller', 'info', 'sensitive', 'purpose', 'location', 'see', 'third',
  'retention', 'rights', 'drivers', 'conduct', 'ownership', 'changes', 'accept',
] as const;

/**
 * The terms and privacy notice. `heading` is h1 on a page of its own, lower when shown
 * inside another step. The Malay text is one tap away because PDPA s.7(3) requires the
 * notice in Malay and English; the app itself stays Chinese/English.
 */
export function TermsText({ heading: Heading = 'h2' }: { heading?: 'h1' | 'h2' | 'h3' }) {
  const { t: tUi } = useTranslation('terms');
  const [malay, setMalay] = useState(false);
  const t = (key: keyof typeof ms) => (malay ? ms[key] : tUi(key));
  return (
    <div className="terms-measure stack" lang={malay ? 'ms' : undefined}>
      <div>
        <Heading className="terms-title">{t('title')}</Heading>
        <p className="muted small">{t('updated')}</p>
        <button type="button" className="link-btn small" lang={malay ? undefined : 'ms'} onClick={() => setMalay(!malay)}>
          {malay ? tUi('readOwn') : tUi('readMalay')}
        </button>
      </div>
      <p className="terms-intro">{t('intro')}</p>
      {TERMS_SECTIONS.map((key) => (
        <section key={key} className="stack-sm">
          <h3>{t(`${key}Title`)}</h3>
          <p className="muted">{t(`${key}Body`)}</p>
        </section>
      ))}
      <p className="small">
        <a href="/third-party-licenses.txt" target="_blank" rel="noopener noreferrer">{t('licences')}</a>
      </p>
    </div>
  );
}

export function TermsOptIn({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
  const { t } = useTranslation('terms');
  return (
    <label className={`check ${checked ? 'on' : ''}`}>
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
      {t('optIn')}
    </label>
  );
}

/** Blocks the app after sign-in until the person opts in. */
export function TermsGate() {
  const { t } = useTranslation('terms');
  const { refresh } = useAuth();
  const nav = useNavigate();
  const [checked, setChecked] = useState(false);
  const { busy, error, run } = useAction();

  return (
    <div className="terms-gate-page">
      <div className="terms-gate-container terms-measure stack">
        <TermsText heading="h1" />
        <ErrorBox error={error} />
      </div>

      {/* Sticky accept bar at the bottom */}
      <div className="terms-sticky-bar">
        <div className="terms-sticky-inner terms-measure row between">
          <TermsOptIn checked={checked} onChange={setChecked} />
          <div className="row">
            <Button
              variant="ghost"
              disabled={busy}
              onClick={() => run(async () => { await post('/auth/logout'); await refresh(); nav('/login'); })}
            >
              {t('logout')}
            </Button>
            <Button
              variant="primary"
              disabled={busy || !checked}
              loading={busy}
              onClick={() => run(async () => { await post('/auth/terms', { accepted: true }); await refresh(); })}
            >
              {t('agree')}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}

export default function TermsPage() {
  const { t } = useTranslation('terms');
  const { t: tc } = useTranslation();
  const auth = useAuth();
  const me = auth?.me ?? null;
  const smartBack = useSmartBack(me ? '/' : '/login', { isAuthenticated: Boolean(me) });
  const backLabel = me ? tc('back') : t('back');
  return (
    <div className="auth-page">
      <div className="card auth-card terms-measure stack">
        <div>
          <Link to={smartBack.href} onClick={smartBack.onClick} className="icon-btn topbar-back" aria-label={backLabel}>
            <ArrowLeft size={22} />
          </Link>
        </div>
        <TermsText heading="h1" />
        <div>
          <Button variant="secondary" to={smartBack.href} onClick={smartBack.onClick}>{backLabel}</Button>
        </div>
      </div>
      <SwipeBackGesture enabled onBack={smartBack.goBack} />
    </div>
  );
}
