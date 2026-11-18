import { useTranslation } from 'react-i18next';
import { useApi } from '../../api';

interface VolunteerStats {
  total_trips: number;
  total_hours: number;
  active_since: string;
}

export default function VolunteerImpactCard() {
  const { t } = useTranslation('volunteer');
  const { data } = useApi<VolunteerStats>('/community/volunteer/stats');

  if (!data) return null;

  return (
    <div className="card">
      <h3>{t('impact.title')}</h3>
      <div className="grid grid-3">
        <div>
          <div className="label">{t('impact.trips')}</div>
          <div className="value">{data.total_trips}</div>
        </div>
        <div>
          <div className="label">{t('impact.hours')}</div>
          <div className="value">{data.total_hours}</div>
        </div>
        <div>
          <div className="label">{t('impact.activeSince')}</div>
          <div className="value">{data.active_since}</div>
        </div>
      </div>
    </div>
  );
}
