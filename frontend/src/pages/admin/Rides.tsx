import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Download, AlertTriangle, CheckCircle2, X } from 'lucide-react';
import { post, useApi } from '../../api';
import { useAuth } from '../../auth';
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
  Pill,
  UniversalTable,
  TableColumn,
  TableFilter,
  StatusPill,
  useToast,
} from '../../components/ui';
import { useTableParams } from '../../components/tableState';
import { useExportAll } from '../../components/useExportAll';
import { exportToCsv, fmtDay, todayStr, upcomingSundayStr, RIDE_STATUS, useFormatT } from '../../format';
import { ADMIN_HOME } from '../../staffNav';
import { rideCompactRow } from './rideRows';

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

export function AdminRidesContent({ showExportTop = true }: { showExportTop?: boolean }) {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const nav = useNavigate();
  const auth = useAuth();
  const can = auth?.can ?? (() => true);
  const toast = useToast();
  const canUpdate = can('admin_rides', 'update');

  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  const [batchApproving, setBatchApproving] = useState(false);

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
    // No column sort: the server lists upcoming rides first, then recent past rides.
    defaultSort: null,
    filterKeys: ['q', 'status', 'date', 'attention'],
  });

  const apiQuery = new URLSearchParams(queryString);
  apiQuery.set('page', String(page));
  apiQuery.set('limit', String(pageSize));
  const attention = filters.attention === 'true' ? 'true' : '';
  if (!attention) apiQuery.delete('attention');

  const { data, error, loading, reload } = useApi<AdminRidesData | Ride[]>(
    `/admin/rides?${apiQuery}`,
    20000,
  );

  const [initialLoadComplete, setInitialLoadComplete] = useState(false);
  if (!loading && !initialLoadComplete) setInitialLoadComplete(true);

  const rides = Array.isArray(data) ? data : data?.items ?? [];
  const total = Array.isArray(data) ? data.length : data?.total ?? 0;
  const totalPages = Array.isArray(data) ? 1 : data?.total_pages ?? 1;

  const pendingSelected = rides.filter(
    (r) => selectedIds.has(r.id) && r.status === 'pending_approval',
  );

  const handleBatchApprove = async () => {
    if (pendingSelected.length === 0 || batchApproving) return;
    setBatchApproving(true);
    let success = 0;
    try {
      for (const ride of pendingSelected) {
        await post(`/rides/${ride.id}/approve`);
        success++;
      }
      toast.success(t('rides.approvedBatch', { count: success }));
      setSelectedIds(new Set());
      reload();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
      if (success > 0) reload();
    } finally {
      setBatchApproving(false);
    }
  };

  const handleExportSelected = () => {
    const selectedRows = rides.filter((r) => selectedIds.has(r.id));
    if (selectedRows.length === 0) return;
    exportToCsv(
      `grace-ride-admin-rides-selected-${todayStr()}`,
      ['ID', 'Date/Time', 'Area', 'Destination', 'Passenger', 'Driver', 'Seats', 'Status'],
      selectedRows.map((r) => [
        r.id,
        r.pickup_at,
        r.area.name,
        r.destination,
        r.passenger.name,
        r.driver?.name ?? '',
        r.seats,
        r.status,
      ]),
    );
  };

  const thisSunday = upcomingSundayStr();
  const today = todayStr();
  const currentDate = filters.date ?? '';

  const quickDates = [
    { key: 'all', label: t('rides.dateAll'), date: '' },
    { key: 'today', label: t('rides.dateToday'), date: today },
    { key: 'sunday', label: t('rides.dateThisSunday'), date: thisSunday },
  ];

  const filterDefs: TableFilter[] = [
    { type: 'text', key: 'q', label: t('rides.search'), placeholder: t('rides.search') },
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

  const exporter = useExportAll<Ride>();
  const handleExport = () =>
    exporter.run({
      path: '/admin/rides',
      query: apiQuery,
      filename: `grace-ride-admin-rides-${todayStr()}`,
      headers: ['ID', 'Date/Time', 'Area', 'Destination', 'Passenger', 'Driver', 'Seats', 'Status'],
      row: (r) => [r.id, r.pickup_at, r.area.name, r.destination, r.passenger.name, r.driver?.name ?? '', r.seats, r.status],
    });

  const allOnPageSelected = rides.length > 0 && rides.every((r) => selectedIds.has(r.id));
  const someOnPageSelected = rides.some((r) => selectedIds.has(r.id));

  const columns: TableColumn<Ride>[] = [
    {
      key: 'select',
      header: (
        <input
          type="checkbox"
          aria-label={t('rides.selectAll')}
          checked={allOnPageSelected}
          ref={(input) => {
            if (input) {
              input.indeterminate = someOnPageSelected && !allOnPageSelected;
            }
          }}
          onChange={(e) => {
            if (e.target.checked) {
              setSelectedIds(new Set([...selectedIds, ...rides.map((r) => r.id)]));
            } else {
              const next = new Set(selectedIds);
              for (const r of rides) next.delete(r.id);
              setSelectedIds(next);
            }
          }}
        />
      ),
      render: (r) => (
        <input
          type="checkbox"
          aria-label={t('rides.selectRide', { id: r.id })}
          checked={selectedIds.has(r.id)}
          onClick={(e) => e.stopPropagation()}
          onChange={(e) => {
            const next = new Set(selectedIds);
            if (e.target.checked) next.add(r.id);
            else next.delete(r.id);
            setSelectedIds(next);
          }}
        />
      ),
    },
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
    <div className="stack">
      <ErrorBox error={error} />

      {/* Quick Date Presets */}
      <div className="row between wrap gap-sm">
        <div className="quick-date-chips" role="group" aria-label={t('rides.quickDateLabel')}>
          <span className="quick-date-label">{t('rides.quickDateLabel')}:</span>
          {quickDates.map((d) => (
            <button
              key={d.key}
              type="button"
              className={`chip ${currentDate === d.date ? 'on' : ''}`}
              aria-pressed={currentDate === d.date}
              onClick={() => setFilter('date', d.date)}
            >
              {d.label}
              {d.date && <span className="muted tiny"> · {d.date}</span>}
            </button>
          ))}
        </div>
        {showExportTop && total > 0 && (
          <Button variant="secondary" size="sm" onClick={handleExport} loading={exporter.busy} icon={<Download size={16} />}>
            {t('common:exportAll', { count: total })}
          </Button>
        )}
      </div>

      {/* Batch Selection Action Bar */}
      {selectedIds.size > 0 && (
        <div className="table-batch-bar" role="region" aria-label="Batch actions">
          <div className="table-batch-info">
            <Pill tone="info">{t('rides.selectedCount', { count: selectedIds.size })}</Pill>
          </div>
          <div className="table-batch-actions">
            {canUpdate && pendingSelected.length > 0 && (
              <Button
                variant="primary"
                size="sm"
                icon={<CheckCircle2 size={16} />}
                disabled={batchApproving}
                loading={batchApproving}
                onClick={handleBatchApprove}
              >
                {t('rides.approveSelected', { count: pendingSelected.length })}
              </Button>
            )}
            <Button
              variant="secondary"
              size="sm"
              icon={<Download size={16} />}
              onClick={handleExportSelected}
            >
              {t('rides.exportSelected', { count: selectedIds.size })}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              icon={<X size={16} />}
              onClick={() => setSelectedIds(new Set())}
            >
              {t('rides.clearSelection')}
            </Button>
          </div>
        </div>
      )}

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
              compactRowLead={(r) => (
                <input
                  type="checkbox"
                  className="ride-row-check"
                  aria-label={t('rides.selectRide', { id: r.id })}
                  checked={selectedIds.has(r.id)}
                  onChange={(e) => {
                    const next = new Set(selectedIds);
                    if (e.target.checked) next.add(r.id);
                    else next.delete(r.id);
                    setSelectedIds(next);
                  }}
                />
              )}
              compactRow={(r) => rideCompactRow(r, ft, true)}
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
    </div>
  );
}

export default function AdminRides() {
  const { t } = useTranslation('admin');
  return (
    <Page>
      <PageHeader
        title={t('rides.title')}
        subtitle={t('rides.desc')}
        back={ADMIN_HOME}
      />
      <AdminRidesContent />
    </Page>
  );
}
