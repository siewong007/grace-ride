import { useNavigate, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Download } from 'lucide-react';
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
  Pagination,
} from '../../components/ui';
import { exportToCsv, RIDE_STATUS, todayStr, useFormatT } from '../../format';
import { RideTable } from './RideTable';

// Every status a ride can have. The API filters on the exact status, so these must be real
// values ("requested" / "matched" were not, and always returned nothing).
const FILTER_STATUSES = Object.keys(RIDE_STATUS);

interface AdminRidesData {
  items: Ride[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export default function AdminRides() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const [params, setParams] = useSearchParams();
  const nav = useNavigate();

  const status = params.get('status') ?? '';
  const date = params.get('date') ?? '';
  const attention = params.get('attention') === 'true';
  const page = Math.max(1, Number(params.get('page') ?? '1') || 1);
  const pageSize = Number(params.get('limit') ?? '50') || 50;

  const q = new URLSearchParams();
  if (status) q.set('status', status);
  if (date) q.set('date', date);
  if (attention) q.set('attention', 'true');
  if (page > 1 || pageSize !== 50) {
    q.set('page', String(page));
    q.set('limit', String(pageSize));
  }

  const { data, error, loading } = useApi<AdminRidesData | Ride[]>(`/admin/rides?${q}`, 20000);

  const rides = Array.isArray(data) ? data : data?.items ?? [];
  const total = Array.isArray(data) ? data.length : data?.total ?? 0;
  const totalPages = Array.isArray(data) ? 1 : data?.total_pages ?? 1;

  const set = (k: string, v: string) => {
    const p = new URLSearchParams(params);
    if (v) p.set(k, v);
    else p.delete(k);
    if (k !== 'page' && k !== 'limit') {
      p.delete('page');
    }
    setParams(p, { replace: true });
  };

  const clearFilters = () => {
    setParams(new URLSearchParams(), { replace: true });
  };

  const hasFilters = Boolean(status || date || attention);

  const handleExport = () => {
    const headers = ['ID', 'Date/Time', 'Area', 'Destination', 'Passenger', 'Driver', 'Seats', 'Status'];
    const rows = rides.map((r) => [
      r.id,
      r.pickup_at,
      r.area.name,
      r.destination,
      r.passenger.name,
      r.driver?.name ?? '',
      r.seats,
      r.status,
    ]);
    exportToCsv(`grace-ride-admin-rides-${todayStr()}`, headers, rows);
  };

  return (
    <Page>
      <PageHeader
        title={t('rides.title')}
        subtitle={t('rides.desc')}
        actions={
          rides.length > 0 ? (
            <Button variant="secondary" size="sm" onClick={handleExport} icon={<Download size={16} />}>
              {t('common:exportCsv', 'Export CSV')}
            </Button>
          ) : undefined
        }
      />

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
        ) : !rides.length ? (
          <EmptyState title={t('rides.none')} />
        ) : (
          <>
            <RideTable rides={rides} onOpen={(id) => nav(`/admin/rides/${id}`)} />
            <Pagination
              page={page}
              totalPages={totalPages}
              total={total}
              pageSize={pageSize}
              onPageChange={(p) => set('page', String(p))}
              onPageSizeChange={(sz) => {
                const p = new URLSearchParams(params);
                p.set('limit', String(sz));
                p.delete('page');
                setParams(p, { replace: true });
              }}
            />
          </>
        )}
      </Card>
    </Page>
  );
}
