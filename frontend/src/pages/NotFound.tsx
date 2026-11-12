import { MapPinOff } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button, Page } from '../components/ui';

export default function NotFound() {
  const { t } = useTranslation('notFound');

  return (
    <Page width="narrow" className="not-found-page">
      <div className="card auth-card not-found-card stack">
        <div className="not-found-icon-wrap" aria-hidden="true">
          <MapPinOff size={72} className="not-found-icon" />
        </div>
        <div className="stack-sm not-found-text">
          <h1>{t('notFound')}</h1>
          <p className="muted">{t('desc')}</p>
          <p className="muted small">{t('pageMissing')}</p>
        </div>
        <div className="btn-row not-found-actions">
          <Button variant="primary" to="/" size="lg">
            {t('goHome')}
          </Button>
          <Button variant="secondary" onClick={() => window.history.back()} size="lg">
            {t('back')}
          </Button>
        </div>
      </div>
    </Page>
  );
}
