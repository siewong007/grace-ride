import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';

export default function NotFound() {
  const { t } = useTranslation('notFound');
  return (
    <div className="stack">
      <div className="page-head">
        <div>
          <h1>{t('notFound')}</h1>
          <p>{t('desc')}</p>
        </div>
      </div>
      <div className="card stack-sm">
        <h3>{t('whatHappened')}</h3>
        <p className="muted">{t('pageMissing')}</p>
        <Link to="/" className="btn primary block">
          {t('goHome')}
        </Link>
      </div>
    </div>
  );
}
