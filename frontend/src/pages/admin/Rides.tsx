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

const FILTER_STATUSES = [
  'requested',
  'matched',
  'confirmed',
  'in_progress',
  'completed',
  'cancelled',
];

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

      {/* Filter bar with status chips, date, attention toggle, and clear link */}
      <Card className="tight stack-sm">
        <div className="chips" role="group" aria-label={t('rides.status')}>
          <button
            type="button"
            className={`chip ${status === '' ? 'on' : ''}`}
            onClick={() => set('status', '')}
          >
            {t('rides.allStatus')}
          </button>
          {FILTER_STATUSES.map((k) => (
            <button
              key={k}
              type="button"
              className={`chip ${status === k ? 'on' : ''}`}
              onClick={() => set('status', k)}
            >
              {ft(RIDE_STATUS[k]?.label ?? k)}
            </button>
          ))}
        </div>

        <div className="admin-filters-bar">
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
      </Card>

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
