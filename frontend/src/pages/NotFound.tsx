import { MapPinOff } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { SwipeBackGesture, useSmartBack } from '../components/BackNavigation';
import { Button, Page } from '../components/ui';

export default function NotFound() {
  const { t } = useTranslation('notFound');
  const smartBack = useSmartBack('/');

  return (
    <Page width="narrow" className="not-found-page">
      <div className="card auth-card not-found-card stack">
        <div className="not-found-icon-wrap" aria-hidden="true">
          <MapPinOff size={40} className="not-found-icon" />
        </div>
        <div className="stack-sm not-found-text">
          <h1>{t('notFound')}</h1>
          <p className="muted">{t('desc')}</p>
          <p className="muted small">{t('pageMissing')}</p>
        </div>
        <div className="btn-row not-found-actions">
          <Button variant="primary" to="/">
            {t('goHome')}
          </Button>
          <Button variant="secondary" onClick={smartBack.goBack}>
            {t('back')}
          </Button>
        </div>
      </div>
      <SwipeBackGesture enabled onBack={smartBack.goBack} />
    </Page>
  );
}
