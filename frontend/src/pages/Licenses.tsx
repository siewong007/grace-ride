import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { ArrowLeft } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../auth';
import { SwipeBackGesture, useSmartBack } from '../components/BackNavigation';
import { Button, Spinner } from '../components/ui';

type Package = { name: string; version: string; license: string; text: string };

/**
 * The open-source notices that /terms links to, laid out like the terms themselves:
 * one readable column, a section per package, folded shut until you want the text.
 * The data comes from public/third-party-licenses.json, written by the prebuild script.
 */
export default function LicensesPage() {
  const { t } = useTranslation('terms');
  const { t: tc } = useTranslation();
  const me = useAuth()?.me ?? null;
  const smartBack = useSmartBack(me ? '/' : '/login', { isAuthenticated: Boolean(me) });
  const backLabel = me ? tc('back') : t('back');

  const [packages, setPackages] = useState<Package[] | null>(null);
  const [failed, setFailed] = useState(false);
  const [filter, setFilter] = useState('');

  useEffect(() => {
    let alive = true;
    fetch('/third-party-licenses.json')
      .then((res) => {
        if (!res.ok) throw new Error(String(res.status));
        return res.json() as Promise<Package[]>;
      })
      .then((data) => alive && setPackages(data))
      .catch(() => alive && setFailed(true));
    return () => {
      alive = false;
    };
  }, []);

  const query = filter.trim().toLowerCase();
  const shown = (packages ?? []).filter(
    (p) => !query || `${p.name} ${p.version} ${p.license}`.toLowerCase().includes(query),
  );

  return (
    <div className="auth-page">
      <div className="card auth-card terms-measure stack">
        <div>
          <Link to={smartBack.href} onClick={smartBack.onClick} className="icon-btn topbar-back" aria-label={backLabel}>
            <ArrowLeft size={22} />
          </Link>
        </div>
        <div>
          <h1 className="terms-title">{t('licences')}</h1>
          {packages && <p className="muted small">{t('licencesCount', { n: packages.length })}</p>}
        </div>
        <p className="terms-intro">{t('licencesIntro')}</p>

        {packages && packages.length > 0 && (
          <label className="field">
            <span>{t('licencesFilter')}</span>
            <input
              className="input"
              type="search"
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              placeholder={t('licencesFilterHint')}
            />
          </label>
        )}

        {failed && <p className="error">{t('licencesError')}</p>}
        {!packages && !failed && <Spinner />}

        <div className="stack-sm">
          {shown.map((p) => (
            <details className="licence-item" key={`${p.name}@${p.version}`}>
              <summary>
                <span className="grow licence-name">
                  <strong>{p.name}</strong> <span className="muted small">{p.version}</span>
                </span>
                <span className="pill neutral">{p.license}</span>
              </summary>
              <pre className="licence-text" lang="en">
                {p.text}
              </pre>
            </details>
          ))}
        </div>

        {packages && shown.length === 0 && <p className="muted">{t('licencesEmpty')}</p>}

        <div>
          <Button variant="secondary" to={smartBack.href} onClick={smartBack.onClick}>{backLabel}</Button>
        </div>
      </div>
      <SwipeBackGesture enabled onBack={smartBack.goBack} />
    </div>
  );
}
