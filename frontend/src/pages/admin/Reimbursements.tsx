import { useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Download } from 'lucide-react';
import { post, useApi } from '../../api';
import { useAuth } from '../../auth';
import {
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Field,
  Modal,
  Page,
  PageHeader,
  Pagination,
  ReceiptLink,
  StatusPill,
  TableColumn,
  TableFilter,
  Tabs,
  UniversalTable,
  useAction,
} from '../../components/ui';
import { useTableParams } from '../../components/tableState';
import { exportToCsv, fmtDateTime, fmtDay, REIMB_STATUS, rm, todayStr, useFormatT } from '../../format';
import type { Reimb } from '../Reimbursements';

interface PaginatedReimbursements {
  items: Reimb[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export default function AdminReimbursements() {
  const { t } = useTranslation('admin');
  const { can } = useAuth();
  const [params, setParams] = useSearchParams();
  const canReview = can('admin_reimb_review');
  const canPay = can('admin_reimb_pay');
  const tabParam = params.get('tab') as 'review' | 'payout';
  const tab = (tabParam === 'review' && canReview) || (tabParam === 'payout' && canPay)
    ? tabParam
    : canReview ? 'review' : 'payout';

  if (!canReview && !canPay) {
    return (
      <Page>
        <EmptyState title={t('reimb.noPermission')} />
      </Page>
    );
  }

  const handleTabChange = (nextTab: 'review' | 'payout') => {
    const p = new URLSearchParams(params);
    p.set('tab', nextTab);
    p.delete('page');
    p.delete('limit');
    p.delete('q');
    p.delete('status');
    p.delete('sort');
    p.delete('dir');
    setParams(p, { replace: true });
  };

  return (
    <Page>
      <PageHeader title={t('reimb.title')} subtitle={t('reimb.desc')} />
      {canReview && canPay && (
        <Tabs
          value={tab}
          onChange={(v) => handleTabChange(v as 'review' | 'payout')}
          items={[
            { value: 'review', label: t('reimb.review') },
            { value: 'payout', label: t('reimb.payout') },
          ]}
        />
      )}
      <List scope={tab} />
    </Page>
  );
}

function List({ scope }: { scope: 'review' | 'payout' }) {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const { can } = useAuth();
  const canAct = can(scope === 'review' ? 'admin_reimb_review' : 'admin_reimb_pay', 'update');
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
    queryString,
  } = useTableParams({
    defaultPageSize: 25,
    filterKeys: ['q', 'status'],
  });
  const apiQuery = new URLSearchParams(`scope=${scope}${queryString ? `&${queryString}` : ''}`);
  apiQuery.set('page', String(page));
  apiQuery.set('limit', String(pageSize));
  const { data, error, loading, reload } = useApi<PaginatedReimbursements | Reimb[]>(
    `/reimbursements?${apiQuery}`,
    undefined,
    { latestOnly: true },
  );
  const [initialLoadComplete, setInitialLoadComplete] = useState(false);
  if (!loading && !initialLoadComplete) setInitialLoadComplete(true);

  const [open, setOpen] = useState<Reimb | null>(null);

  const items = Array.isArray(data) ? data : data?.items ?? [];
  const total = Array.isArray(data) ? data.length : data?.total ?? 0;
  const totalPages = Array.isArray(data) ? 1 : data?.total_pages ?? 1;

  const statusOptions = Object.keys(REIMB_STATUS)
    .filter((k) => scope === 'review' || k === 'approved' || k === 'paid')
    .map((k) => ({ value: k, label: ft(REIMB_STATUS[k].label) }));

  const filterDefs: TableFilter[] = [
    {
      type: 'text',
      key: 'q',
      label: t(can('admin_reimb_pay') ? 'reimb.search' : 'reimb.searchWithoutPayee'),
      placeholder: t(can('admin_reimb_pay') ? 'reimb.search' : 'reimb.searchWithoutPayee'),
    },
    {
      type: 'select',
      key: 'status',
      label: t('reimb.status'),
      allLabel: t('reimb.allStatus'),
      options: statusOptions,
    },
  ];

  const handleExport = () => {
    if (scope === 'payout') {
      const headers = ['Claim ID', 'Payee Name', 'DuitNow / Account', 'Amount (MYR)', 'Applicant', 'Description', 'Status'];
      const rows = items.map((r) => [
        r.id,
        r.payee_name || r.user.name,
        r.payee_phone || '',
        (r.amount_cents / 100).toFixed(2),
        r.user.name,
        r.description,
        r.status,
      ]);
      exportToCsv(`grace-ride-duitnow-batch-${todayStr()}`, headers, rows);
      return;
    }
    const headers = ['ID', 'Applicant', 'Amount (cents)', 'Description', 'Ride ID', 'Status', 'Submitted At'];
    const rows = items.map((r) => [
      r.id,
      r.user.name,
      r.amount_cents,
      r.description,
      r.ride_id ?? '',
      r.status,
      r.created_at,
    ]);
    exportToCsv(`grace-ride-reimbursements-${scope}-${todayStr()}`, headers, rows);
  };

  const columns: TableColumn<Reimb>[] = [
    {
      key: 'applicant',
      header: t('reimb.applicant'),
      sortable: true,
      sortKey: 'applicant',
      render: (r) => <div className="bold">{r.user.name}</div>,
    },
    {
      key: 'amount',
      header: t('reimb.amount'),
      sortable: true,
      sortKey: 'amount_cents',
      render: (r) => <div className="mono bold">{rm(r.amount_cents)}</div>,
    },
    {
      key: 'description',
      header: t('reimb.description'),
      render: (r) => (
        <div className="small">
          {r.description}
          {r.ride_id && (
            <>
              {' · '}
              <Link to={`/admin/rides/${r.ride_id}`}>
                {t('reimb.rideLink')}
              </Link>
            </>
          )}
        </div>
      ),
    },
    {
      key: 'created_at',
      header: t('reimb.submitted'),
      sortable: true,
      sortKey: 'created_at',
      render: (r) => fmtDay(r.created_at, ft),
    },
    {
      key: 'status',
      header: t('reimb.status'),
      sortable: true,
      sortKey: 'status',
      render: (r) => <StatusPill map={REIMB_STATUS} status={r.status} />,
    },
    {
      key: 'action',
      header: t('actions'),
      render: (r) => (
        <>
          {canAct &&
          ((scope === 'review' && r.status === 'pending') ||
            (scope === 'payout' && r.status === 'approved')) ? (
            <Button
              variant="primary"
              size="sm"
              onClick={() => setOpen(r)}
            >
              {scope === 'review'
                ? t('reimb.reviewBtn')
                : t('reimb.payBtn')}
            </Button>
          ) : (
            <ReceiptLink
              id={r.receipt_file_id}
              label={t('reimb.receipt')}
            />
          )}
        </>
      ),
    },
  ];

  return (
    <div className="stack">
      {/* Clear section notices for role separation */}
      <div className="row between nowrap gap-sm">
        <div className="notice info grow">
          {scope === 'payout' ? t('reimb.financeNotice') : t('reimb.transportNotice')}
        </div>
        {items.length > 0 && (
          <Button variant="secondary" size="sm" onClick={handleExport} icon={<Download size={16} />}>
            {scope === 'payout' ? t('reimb.exportBatchDuitNow') : t('common:exportCsv', 'Export CSV')}
          </Button>
        )}
      </div>

      <ErrorBox error={error} />

      <Card>
        {loading && !initialLoadComplete ? (
          <CardSkeleton count={3} />
        ) : (
          <>
            <UniversalTable
              columns={columns}
              items={loading ? [] : items}
              itemKey={(r) => r.id}
              sort={sort}
              onSortChange={setSort}
              filters={filterDefs}
              filterValues={filters}
              onFilterChange={setFilter}
              onClearFilters={clearFilters}
              emptyContent={loading ? <CardSkeleton count={3} /> : <EmptyState title={t('reimb.none')} />}
            />
            {!loading && items.length > 0 && (
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

      {open && scope === 'review' && (
        <ReviewModal
          r={open}
          onClose={() => setOpen(null)}
          onDone={() => {
            setOpen(null);
            reload();
          }}
        />
      )}
      {open && scope === 'payout' && (
        <PayModal
          r={open}
          onClose={() => setOpen(null)}
          onDone={() => {
            setOpen(null);
            reload();
          }}
        />
      )}
    </div>
  );
}

function ReviewModal({
  r,
  onClose,
  onDone,
}: {
  r: Reimb;
  onClose: () => void;
  onDone: () => void;
}) {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const [note, setNote] = useState('');
  const { busy, error, run } = useAction();

  const act = (action: string, msg: string) =>
    run(
      () => post(`/reimbursements/${r.id}/review`, { action, note }).then(onDone),
      msg,
    );

  return (
    <Modal
      title={t('reimb.reviewTitle', {
        name: r.user.name,
        amount: rm(r.amount_cents),
      })}
      onClose={onClose}
    >
      <div className="stack-sm">
        <img
          className="receipt-thumb"
          src={`/api/files/${r.receipt_file_id}`}
          alt={t('reimb.receiptAlt')}
        />
        <ReceiptLink id={r.receipt_file_id} label={t('reimb.openReceipt')} />
        <p className="small">{r.description}</p>
        {r.ride_at && (
          <p className="small muted">
            {t('reimb.ride', { time: fmtDateTime(r.ride_at, ft) })}
          </p>
        )}
        <Field label={t('reimb.noteLabel')}>
          <input
            className="input"
            value={note}
            onChange={(e) => setNote(e.target.value)}
          />
        </Field>
        <ErrorBox error={error} />
        <div className="btn-row">
          <Button
            variant="primary"
            disabled={busy}
            onClick={() => act('approve', t('reimb.approved'))}
          >
            {t('reimb.approve')}
          </Button>
          <Button
            variant="secondary"
            disabled={busy}
            onClick={() => act('return', t('reimb.returned'))}
          >
            {t('reimb.return')}
          </Button>
          <Button
            variant="danger"
            disabled={busy}
            onClick={() => act('reject', t('reimb.rejected'))}
          >
            {t('reimb.reject')}
          </Button>
        </div>
      </div>
    </Modal>
  );
}

function PayModal({
  r,
  onClose,
  onDone,
}: {
  r: Reimb;
  onClose: () => void;
  onDone: () => void;
}) {
  const { t } = useTranslation('admin');
  const [verified, setVerified] = useState(false);
  const [ref, setRef] = useState('');
  const { busy, error, run } = useAction();

  return (
    <Modal
      title={t('reimb.payTitle', { amount: rm(r.amount_cents) })}
      onClose={onClose}
    >
      <div className="stack-sm">
        <dl className="kv">
          <dt>{t('reimb.payee')}</dt>
          <dd>{r.payee_name}</dd>
          <dt>{t('reimb.phone')}</dt>
          <dd className="mono">{r.payee_phone}</dd>
          <dt>{t('reimb.applicant2')}</dt>
          <dd>{r.user.name}</dd>
        </dl>
        <label className={`check ${verified ? 'on' : ''}`}>
          <input
            type="checkbox"
            checked={verified}
            onChange={(e) => setVerified(e.target.checked)}
          />
          {t('reimb.verified')}
        </label>
        <Field label={t('reimb.txn')} hint={t('reimb.txnHint')}>
          <input
            className="input"
            value={ref}
            onChange={(e) => setRef(e.target.value)}
          />
        </Field>
        <ErrorBox error={error} />
        <Button
          variant="primary"
          block
          disabled={busy || !verified || ref.length < 4}
          onClick={() =>
            run(
              () =>
                post(`/reimbursements/${r.id}/pay`, {
                  payee_verified: verified,
                  paid_ref: ref,
                }).then(onDone),
              t('reimb.recorded'),
            )
          }
        >
          {t('reimb.record')}
        </Button>
      </div>
    </Modal>
  );
}
