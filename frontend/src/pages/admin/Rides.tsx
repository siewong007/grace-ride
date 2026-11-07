import { useNavigate, useSearchParams } from 'react-router-dom';
import { useApi } from '../../api';
import { Ride } from '../../components/RideCard';
import { Empty, ErrorBox, Spinner } from '../../components/ui';
import { RIDE_STATUS, useFormatT } from '../../format';
import { RideTable } from './Dashboard';
import { useTranslation } from 'react-i18next';

export default function AdminRides() {
  const { t, ft } = useTranslationHooks();
  const [params, setParams] = useSearchParams();
  const nav = useNavigate();
  const status = params.get('status') ?? '';
  const date = params.get('date') ?? '';
  const attention = params.get('attention') === 'true';
  const q = new URLSearchParams();
  if (status) q.set('status', status);
  if (date) q.set('date', date);
  if (attention) q.set('attention', 'true');
  const { data, error, loading } = useApi<Ride[]>(`/admin/rides?${q}`, 20000);
  const set = (k: string, v: string) => {
    const p = new URLSearchParams(params);
    if (v) p.set(k, v); else p.delete(k);
    setParams(p, { replace: true });
  };

  return (
    <div className="stack">
      <div className="page-head"><div><h1>{t('rides.title')}</h1><p>{t('rides.desc')}</p></div></div>
      <div className="card tight row">
        <select className="input" style={{ maxWidth: 220 }} value={status} onChange={(e) => set('status', e.target.value)} aria-label={t('rides.status')}>
          <option value="">{t('rides.allStatus')}</option>
          {Object.entries(RIDE_STATUS).map(([k, v]) => <option key={k} value={k}>{ft(v.label)}</option>)}
        </select>
        <input className="input" style={{ maxWidth: 200 }} type="date" value={date} onChange={(e) => set('date', e.target.value)} aria-label={t('rides.date')} />
        <label className={`check ${attention ? 'on' : ''}`}>
          <input type="checkbox" checked={attention} onChange={(e) => set('attention', e.target.checked ? 'true' : '')} />{t('rides.onlyAttention')}
        </label>
      </div>
      <ErrorBox error={error} />
      <div className="card">
        {loading ? <Spinner /> : !data?.length ? <Empty>{t('rides.none')}</Empty> : <RideTable rides={data} onOpen={(id) => nav(`/admin/rides/${id}`)} t={t} ft={ft} />}
      </div>
    </div>
  );

  function useTranslationHooks() {
    const { t } = useTranslation('admin');
    const ft = useFormatT();
    return { t, ft };
  }
}
