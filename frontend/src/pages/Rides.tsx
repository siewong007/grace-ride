import { useState } from 'react';
import { Link } from 'react-router-dom';
import { CalendarPlus } from 'lucide-react';
import { useApi } from '../api';
import RideCard, { Ride } from '../components/RideCard';
import { Empty, ErrorBox, Spinner } from '../components/ui';
import { useTranslation } from 'react-i18next';

export default function Rides() {
  const { t } = useTranslation('rides');
  const [scope, setScope] = useState<'upcoming' | 'past'>('upcoming');
  const { data, error, loading } = useApi<Ride[]>(`/rides?scope=${scope}`, 30000);
  return (
    <div className="stack">
      <div className="page-head">
        <div>
          <h1>{t('title')}</h1>
          <p>{t('desc')}</p>
        </div>
        <Link className="btn primary" to="/rides/new"><CalendarPlus size={20} />{t('signupRide')}</Link>
      </div>
      <div className="tabs">
        <button className={scope === 'upcoming' ? 'on' : ''} onClick={() => setScope('upcoming')}>{t('upcoming')}</button>
        <button className={scope === 'past' ? 'on' : ''} onClick={() => setScope('past')}>{t('past')}</button>
      </div>
      <ErrorBox error={error} />
      {loading ? <Spinner /> : data?.length ? (
        <div className="grid cols-2">{data.map((r) => <RideCard key={r.id} ride={r} />)}</div>
      ) : <Empty>{scope === 'upcoming' ? t('noUpcoming') : t('noPast')}</Empty>}
    </div>
  );
}
