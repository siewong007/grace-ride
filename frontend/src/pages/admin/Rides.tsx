import { useNavigate, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useApi } from '../../api';
import { Ride } from '../../components/RideCard';
import {
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Page,
  PageHeader,
} from '../../components/ui';
import { RIDE_STATUS, useFormatT } from '../../format';
import { RideTable } from './RideTable';

// Every status a ride can have. The API filters on the exact status, so these must be real
// values ("requested" / "matched" were not, and always returned nothing).
const FILTER_STATUSES = Object.keys(RIDE_STATUS);

export default function AdminRides() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
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
    if (v) p.set(k, v);
    else p.delete(k);
    setParams(p, { replace: true });
  };

  const clearFilters = () => {
    setParams(new URLSearchParams(), { replace: true });
  };

  const hasFilters = Boolean(status || date || attention);

  return (
    <Page>
      <PageHeader title={t('rides.title')} subtitle={t('rides.desc')} />

      {/* Filters: status menu, date, attention toggle, clear */}
      <div className="admin-filters-bar">
          <select
            className="input admin-input-compact"
            value={status}
            onChange={(e) => set('status', e.target.value)}
            aria-label={t('rides.status')}
          >
            <option value="">{t('rides.allStatus')}</option>
            {FILTER_STATUSES.map((k) => (
              <option key={k} value={k}>{ft(RIDE_STATUS[k].label)}</option>
            ))}
          </select>
          <input
            className="input admin-input-compact"
            type="date"
            value={date}
            onChange={(e) => set('date', e.target.value)}
            aria-label={t('rides.date')}
          />
          <label className={`check ${attention ? 'on' : ''}`}>
            <input
              type="checkbox"
              checked={attention}
              onChange={(e) => set('attention', e.target.checked ? 'true' : '')}
            />
            {t('rides.onlyAttention')}
          </label>
          {hasFilters && (
            <Button variant="ghost" size="sm" onClick={clearFilters}>
              {t('rides.clearFilters')}
            </Button>
          )}
      </div>

      <ErrorBox error={error} />

      <Card>
        {loading ? (
          <CardSkeleton count={4} />
        ) : !data?.length ? (
          <EmptyState title={t('rides.none')} />
        ) : (
          <RideTable rides={data} onOpen={(id) => nav(`/admin/rides/${id}`)} />
        )}
      </Card>
    </Page>
  );
}
