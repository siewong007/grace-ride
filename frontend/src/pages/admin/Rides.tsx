import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Download, AlertTriangle } from 'lucide-react';
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
  UniversalTable,
  TableColumn,
  TableFilter,
  StatusPill,
} from '../../components/ui';
import { useTableParams } from '../../components/tableState';
import { exportToCsv, fmtDay, RIDE_STATUS, todayStr, useFormatT } from '../../format';

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
  const nav = useNavigate();

  const {
    page,
    pageSize,
    sort,
    filters,
    setPage,
    setPageSize,
    setSort,
    setFilter,
    clearFilters,
    hasFilters,
    queryString,
  } = useTableParams({
    defaultPageSize: 50,
    defaultSort: { key: 'pickup_at', dir: 'asc' },
    filterKeys: ['status', 'date', 'attention'],
  });

  const apiQuery = new URLSearchParams(queryString);
  apiQuery.set('page', String(page));
  apiQuery.set('limit', String(pageSize));
  const attention = filters.attention === 'true' ? 'true' : '';
  if (!attention) apiQuery.delete('attention');

  const { data, error, loading } = useApi<AdminRidesData | Ride[]>(
    `/admin/rides?${apiQuery}`,
    20000,
    { latestOnly: true },
  );

  const [initialLoadComplete, setInitialLoadComplete] = useState(false);
  if (!loading && !initialLoadComplete) setInitialLoadComplete(true);

  const rides = Array.isArray(data) ? data : data?.items ?? [];
  const total = Array.isArray(data) ? data.length : data?.total ?? 0;
  const totalPages = Array.isArray(data) ? 1 : data?.total_pages ?? 1;

  const filterDefs: TableFilter[] = [
    {
      type: 'select',
      key: 'status',
      label: t('rides.status'),
      allLabel: t('rides.allStatus'),
      options: FILTER_STATUSES.map((k) => ({
        value: k,
        label: ft(RIDE_STATUS[k].label),
      })),
    },
    { type: 'date', key: 'date', label: t('rides.date') },
    { type: 'checkbox', key: 'attention', label: t('rides.onlyAttention') },
  ];

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

  const columns: TableColumn<Ride>[] = [
    {
      key: 'pickup_at',
      header: t('dashboard.pickupTime'),
      sortable: true,
      sortKey: 'pickup_at',
      render: (r) => (
        <>
          {fmtDay(r.pickup_at, ft).split(' ')[0]} {r.pickup_at.slice(11, 16)}
        </>
      ),
    },
    {
      key: 'passenger',
      header: t('dashboard.passenger'),
      sortable: true,
      sortKey: 'passenger',
      render: (r) => `${r.passenger.name}${r.seats > 1 ? ` (${r.seats})` : ''}`,
    },
    {
      key: 'area',
      header: t('dashboard.area'),
      sortable: true,
      sortKey: 'area',
      render: (r) => r.area.name,
    },
    {
      key: 'driver',
      header: t('dashboard.driver'),
      sortable: true,
      sortKey: 'driver',
      render: (r) => r.driver?.name ?? <span className="muted">—</span>,
    },
    {
      key: 'status',
      header: t('dashboard.status'),
      sortable: true,
      sortKey: 'status',
      render: (r) => (
        <>
          <StatusPill map={RIDE_STATUS} status={r.status} />
          {r.needs_attention && (
            <AlertTriangle size={14} aria-hidden="true" className="pill-icon warn" />
          )}
        </>
      ),
    },
  ];

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

      <ErrorBox error={error} />

      <Card>
        {loading && !initialLoadComplete ? (
          <CardSkeleton count={4} />
        ) : (
          <>
            <UniversalTable
              columns={columns}
              items={loading ? [] : rides}
              onRowClick={(r) => nav(`/admin/rides/${r.id}`)}
              itemKey={(r) => r.id}
              sort={sort}
              onSortChange={setSort}
              filters={filterDefs}
              filterValues={{ ...filters, attention }}
              onFilterChange={setFilter}
              onClearFilters={clearFilters}
              emptyContent={loading ? <CardSkeleton count={4} /> : (
                <EmptyState
                  title={t('rides.none')}
                  action={hasFilters ? <Button variant="secondary" onClick={clearFilters}>{t('rides.clearFilters')}</Button> : undefined}
                />
              )}
            />
            {!loading && rides.length > 0 && (
              <Pagination
                page={page}
                totalPages={totalPages}
                total={total}
                pageSize={pageSize}
                onPageChange={setPage}
                onPageSizeChange={setPageSize}
              />
            )}
          </>
        )}
      </Card>
    </Page>
  );
}
