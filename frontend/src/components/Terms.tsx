import { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { post } from '../api';
import { useAuth } from '../auth';
import { ErrorBox, useAction } from './ui';
import { useTranslation } from 'react-i18next';

const SECTIONS = ['service', 'who', 'info', 'location', 'see', 'choices', 'accept'] as const;

export function TermsText() {
  const { t } = useTranslation('terms');
  return (
    <div className="stack-sm">
      <h2>{t('title')}</h2>
      <p className="muted small">{t('updated')}</p>
      <p>{t('intro')}</p>
      {SECTIONS.map((key) => (
        <section key={key}>
          <h3>{t(`${key}Title`)}</h3>
          <p>{t(`${key}Body`)}</p>
        </section>
      ))}
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

/** Blocks the app after sign-in until the person opts in. The box starts clear. */
export function TermsGate() {
  const { t } = useTranslation('terms');
  const { refresh } = useAuth();
  const nav = useNavigate();
  const [checked, setChecked] = useState(false);
  const { busy, error, run } = useAction();
  return (
    <div className="auth-page">
      <div className="card auth-card stack">
        <div className="card flat" style={{ maxHeight: 320, overflow: 'auto' }}>
          <TermsText />
        </div>
        <TermsOptIn checked={checked} onChange={setChecked} />
        <ErrorBox error={error} />
        <button className="btn primary block" disabled={busy || !checked} onClick={() => run(async () => { await post('/auth/terms', { accepted: true }); await refresh(); })}>{t('agree')}</button>
        <button className="btn ghost" disabled={busy} onClick={() => run(async () => { await post('/auth/logout'); await refresh(); nav('/login'); })}>{t('logout')}</button>
      </div>
    </div>
  );
}

export default function TermsPage() {
  const { t } = useTranslation('terms');
  return (
    <div className="auth-page">
      <div className="card auth-card stack">
        <TermsText />
        <Link to="/login">{t('back')}</Link>
      </div>
    </div>
  );
}
