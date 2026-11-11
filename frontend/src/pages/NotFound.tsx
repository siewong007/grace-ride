import { Link } from 'react-router-dom';
import { ErrorBox } from '../components/ui';
import { useTranslation } from 'react-i18next';

function useTranslationHooks() {
  const { t } = useTranslation('notFound');
  return { t };
}

export default function NotFound() {
  const { t } = useTranslationHooks();
  return (
    <div className="stack">
      <div className="page-head">
        <div>
          <h1>{t('notFound')}</h1>
          <p>{t('desc')}</p>
        </div>
      </div>
      <ErrorBox error={null} />
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
