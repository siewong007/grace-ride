import { useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Download, ZoomIn, ZoomOut, RotateCw, RotateCcw } from 'lucide-react';
import { del, post, useApi } from '../../api';
import { useAuth } from '../../auth';
import {
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  NoAccess,
  ErrorBox,
  Field,
  Modal,
  Page,
  PageHeader,
  Pagination,
  Pill,
  ReceiptLink,
  StatusPill,
  TableColumn,
  TableFilter,
  Tabs,
  UniversalTable,
  useAction,
} from '../../components/ui';
import { useTableParams } from '../../components/tableState';
import { useExportAll } from '../../components/useExportAll';
import { fmtDateTime, fmtDay, REIMB_STATUS, rm, todayStr, useFormatT } from '../../format';
import type { Reimb } from '../Reimbursements';
import { ADMIN_HOME, useStaffNav } from '../../staffNav';

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
  const { counts } = useStaffNav();
  const canReview = can('admin_reimb_review');
  const canPay = can('admin_reimb_pay');
  const tabParam = params.get('tab') as 'review' | 'payout';
  const tab = (tabParam === 'review' && canReview) || (tabParam === 'payout' && canPay)
    ? tabParam
    : canReview ? 'review' : 'payout';

  if (!canReview && !canPay) {
    return (
      <Page>
        <NoAccess />
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
      <PageHeader title={t('reimb.title')} subtitle={t('reimb.desc')} back={ADMIN_HOME} />
      {canReview && canPay && (
        <Tabs
          value={tab}
          onChange={(v) => handleTabChange(v as 'review' | 'payout')}
          items={[
            { value: 'review', label: t('reimb.review'), count: counts?.pending_reimbursements || undefined },
            { value: 'payout', label: t('reimb.payout'), count: counts?.awaiting_payment || undefined },
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
  const { can, me } = useAuth();
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
    hasFilters,
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

  const exporter = useExportAll<Reimb>();
  const handleExport = () =>
    scope === 'payout'
      ? exporter.run({
          path: '/reimbursements',
          query: apiQuery,
          filename: `grace-ride-duitnow-batch-${todayStr()}`,
          headers: ['Claim ID', 'Payee Name', 'DuitNow / Account', 'Amount (MYR)', 'Applicant', 'Description', 'Status'],
          row: (r) => [r.id, r.payee_name || r.user.name, r.payee_phone || '', (r.amount_cents / 100).toFixed(2), r.user.name, r.description, r.status],
        })
      : exporter.run({
          path: '/reimbursements',
          query: apiQuery,
          filename: `grace-ride-reimbursements-${scope}-${todayStr()}`,
          // Amounts in ringgit here too, so the review and payout files agree.
          headers: ['ID', 'Applicant', 'Amount (MYR)', 'Description', 'Ride ID', 'Status', 'Submitted At'],
          row: (r) => [r.id, r.user.name, (r.amount_cents / 100).toFixed(2), r.description, r.ride_id ?? '', r.status, r.created_at],
        });

  /** Review or Pay when this member may act on the claim now; otherwise the receipt. */
  const actionCell = (r: Reimb) => {
    const isSelf = r.user.id === me?.id;
    const isOwnRequest = Boolean(r.requested_by && r.requested_by.id === me?.id);
    if (scope === 'review' && r.status === 'pending' && (isSelf || isOwnRequest)) {
      return <span className="muted tiny">{t('reimb.ownRequest')}</span>;
    }
    if (scope === 'payout' && r.status === 'approved' && isSelf) {
      return <span className="muted tiny">{t('reimb.ownRequest')}</span>;
    }
    if (
      canAct &&
      ((scope === 'review' && r.status === 'pending') ||
        (scope === 'payout' && r.status === 'approved'))
    ) {
      return (
        <Button variant="primary" size="sm" onClick={() => setOpen(r)}>
          {scope === 'review' ? t('reimb.reviewBtn') : t('reimb.payBtn')}
        </Button>
      );
    }
    return <ReceiptLink id={r.receipt_file_id} label={t('reimb.receipt')} />;
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
          {r.kind === 'visitor_special' ? <SpecialSummary r={r} /> : <div>{r.description}</div>}
          {r.event_title && (
            <div className="muted tiny">
              {r.event_title}
            </div>
          )}
          {r.ride_id && (
            <div className="tiny">
              <Link to={`/admin/rides/${r.ride_id}`}>
                {t('reimb.rideLink')}
              </Link>
            </div>
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
      headerClassName: 'col-sticky-end',
      cellClassName: 'col-sticky-end',
      render: (r) => actionCell(r),
    },
  ];

  return (
    <div className="stack">
      {/* Clear section notices for role separation */}
      <div className="row between nowrap gap-sm">
        <div className="notice info grow">
          {scope === 'payout' ? t('reimb.financeNotice') : t('reimb.transportNotice')}
        </div>
        {total > 0 && (
          <Button variant="secondary" size="sm" onClick={handleExport} loading={exporter.busy} icon={<Download size={16} />}>
            {scope === 'payout' ? t('reimb.exportBatchDuitNow') : t('common:exportAll', { count: total })}
          </Button>
        )}
      </div>

      <ErrorBox error={error} />

      {scope === 'payout' && can('admin_reimb_pay') && (
        <MonthlyPayoutBatches onChanged={reload} />
      )}

      <Card>
        {loading && !initialLoadComplete ? (
          <CardSkeleton count={3} />
        ) : (
          <>
            <UniversalTable
              columns={columns}
              items={loading ? [] : items}
              itemKey={(r) => r.id}
              compactRow={(r) => (
                <>
                  <div className="grow min-w-0 stack-xs">
                    <div className="row between nowrap">
                      <span className="bold ellipsis">{r.user.name}</span>
                      <span className="mono bold no-wrap">{rm(r.amount_cents)}</span>
                    </div>
                    {r.kind === 'visitor_special' ? (
                      <div className="small"><SpecialSummary r={r} /></div>
                    ) : (
                      <div className="small clamp-2">{r.description}</div>
                    )}
                    <div className="row tiny muted">
                      <StatusPill map={REIMB_STATUS} status={r.status} />
                      <span>{fmtDay(r.created_at, ft)}</span>
                      {r.kind === 'mileage' && <span className="bold">{t('reimb.mileageKind')}</span>}
                      {r.ride_id && <Link to={`/admin/rides/${r.ride_id}`}>{t('reimb.rideLink')}</Link>}
                    </div>
                  </div>
                  <div className="item-actions">{actionCell(r)}</div>
                </>
              )}
              sort={sort}
              onSortChange={setSort}
              filters={filterDefs}
              filterValues={filters}
              onFilterChange={setFilter}
              onClearFilters={clearFilters}
              emptyContent={loading ? <CardSkeleton count={3} /> : (
                <EmptyState
                  title={t('reimb.none')}
                  description={hasFilters ? undefined : t('reimb.noneDesc')}
                  action={hasFilters ? <Button variant="secondary" onClick={clearFilters}>{t('common:clearFilters')}</Button> : undefined}
                />
              )}
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

function getDuitNowBadge(phone: string | null | undefined, t: (k: string) => string) {
  if (!phone) return null;
  const cleaned = phone.replace(/[\s-]/g, '');
  if (/^(\+?601|01)\d{7,9}$/.test(cleaned)) {
    return <Pill tone="teal">{t('reimb.duitnowTypePhone')}</Pill>;
  }
  if (/^\d{12}$/.test(cleaned)) {
    return <Pill tone="blue">{t('reimb.duitnowTypeNric')}</Pill>;
  }
  if (/^\d{8,16}$/.test(cleaned)) {
    return <Pill tone="gray">{t('reimb.duitnowTypeAccount')}</Pill>;
  }
  return null;
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
  const [zoom, setZoom] = useState(1);
  const [rotation, setRotation] = useState(0);
  const [imgFailed, setImgFailed] = useState(false);
  const needsNote = !note.trim();
  const { busy, error, run } = useAction();

  const handleZoomIn = () => setZoom((z) => Math.min(z + 0.3, 3));
  const handleZoomOut = () => setZoom((z) => Math.max(z - 0.3, 0.7));
  const handleRotate = () => setRotation((rot) => (rot + 90) % 360);
  const handleResetZoom = () => {
    setZoom(1);
    setRotation(0);
  };

  const act = (action: string, msg: string) =>
    run(
      () =>
        post(
          `/reimbursements/${r.id}/review`,
          r.updated_at ? { action, note, expected_updated_at: r.updated_at } : { action, note },
        ).then(onDone),
      msg,
    );

  const hasReceipt = Boolean(r.receipt_file_id);

  return (
    <Modal
      size={hasReceipt ? 'lg' : 'md'}
      title={t('reimb.reviewTitle', {
        name: r.user.name,
        amount: rm(r.amount_cents),
      })}
      onClose={onClose}
    >
      <div className={hasReceipt ? 'reimb-review-split' : 'stack-sm'}>
        {hasReceipt && (
          <div className="reimb-receipt-pane">
            <div className="reimb-receipt-toolbar">
              <span className="small bold">{t('reimb.receiptViewer')}</span>
              <div className="btn-row nowrap">
                <Button
                  variant="ghost"
                  size="sm"
                  icon={<ZoomIn size={15} />}
                  onClick={handleZoomIn}
                  aria-label={t('reimb.zoomIn')}
                  title={t('reimb.zoomIn')}
                />
                <Button
                  variant="ghost"
                  size="sm"
                  icon={<ZoomOut size={15} />}
                  onClick={handleZoomOut}
                  aria-label={t('reimb.zoomOut')}
                  title={t('reimb.zoomOut')}
                />
                <Button
                  variant="ghost"
                  size="sm"
                  icon={<RotateCw size={15} />}
                  onClick={handleRotate}
                  aria-label={t('reimb.rotate')}
                  title={t('reimb.rotate')}
                />
                {(zoom !== 1 || rotation !== 0) && (
                  <Button
                    variant="ghost"
                    size="sm"
                    icon={<RotateCcw size={15} />}
                    onClick={handleResetZoom}
                    aria-label={t('reimb.resetZoom')}
                    title={t('reimb.resetZoom')}
                  />
                )}
              </div>
            </div>
            <div className="receipt-zoom-container">
              {!imgFailed ? (
                <img
                  className="receipt-zoom-img"
                  style={{
                    transform: `scale(${zoom}) rotate(${rotation}deg)`,
                  }}
                  src={`/api/files/${r.receipt_file_id}`}
                  alt={t('reimb.receiptAlt')}
                  onError={() => setImgFailed(true)}
                />
              ) : (
                <div className="stack-xs center pad-md">
                  <ReceiptLink id={r.receipt_file_id} label={t('reimb.openReceipt')} />
                </div>
              )}
            </div>
            <ReceiptLink id={r.receipt_file_id} label={t('reimb.openReceipt')} />
          </div>
        )}

        <div className={hasReceipt ? 'reimb-details-pane' : 'stack-sm'}>
          {r.kind === 'visitor_special' ? (
            <>
              <p className="small muted">{t('reimb.noReceiptNeeded')}</p>
              <div className="small">
                <SpecialSummary r={r} />
              </div>
            </>
          ) : (
            <p className="small">{r.description}</p>
          )}
          {r.high_amount && (
            <div className="notice warning small" role="status">
              {t('reimb.highAmountNote')}
            </div>
          )}
          {r.event_title && (
            <p className="small">
              <strong>{t('reimb.event')}:</strong> {r.event_title}
              {r.event_starts_at && (
                <span className="muted"> ({fmtDateTime(r.event_starts_at, ft)})</span>
              )}
            </p>
          )}
          {r.kind === 'mileage' && (
            <p className="small">
              <strong>{t('reimb.mileageKind')}</strong>
              {r.ride_route ? ` · ${r.ride_route}` : ''}
            </p>
          )}
          {r.ride_at && (
            <p className="small muted">
              {t('reimb.ride', { time: fmtDateTime(r.ride_at, ft) })}
            </p>
          )}
          {r.payee_phone && (
            <div className="row wrap gap-xs small">
              <span className="muted">{t('reimb.phone')}:</span>
              <span className="mono">{r.payee_phone}</span>
              {getDuitNowBadge(r.payee_phone, t)}
            </div>
          )}
          <Field label={t('reimb.noteLabel')} hint={needsNote ? t('reimb.noteNeeded') : undefined}>
            <input
              className="input"
              value={note}
              onChange={(e) => setNote(e.target.value)}
            />
          </Field>
          <ErrorBox error={error} />
          <Button
            variant="primary"
            block
            disabled={busy}
            onClick={() => act('approve', t('reimb.approved'))}
          >
            {t('reimb.approve')}
          </Button>
          {/* Sending it back or turning it down needs a reason the member can read. */}
          <div className="btn-row">
            <Button
              variant="secondary"
              disabled={busy || !note.trim()}
              onClick={() => act('return', t('reimb.returned'))}
            >
              {t('reimb.return')}
            </Button>
            <Button
              variant="danger"
              disabled={busy || !note.trim()}
              onClick={() => act('reject', t('reimb.rejected'))}
            >
              {t('reimb.reject')}
            </Button>
          </div>
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
          <dd className="mono">
            {r.payee_phone} {getDuitNowBadge(r.payee_phone, t)}
          </dd>
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
          disabled={busy || !verified || ref.trim().length < 4}
          onClick={() =>
            run(
              () =>
                post(`/reimbursements/${r.id}/pay`, {
                  payee_verified: verified,
                  paid_ref: ref.trim(),
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

interface PayoutBatch {
  id: number;
  month: string;
  title: string;
  total_cents: number;
  claim_count: number;
  status: 'open' | 'paid';
  paid_ref: string;
  created_by: string;
  paid_by: string | null;
  paid_at: string | null;
}

function MonthlyPayoutBatches({ onChanged }: { onChanged: () => void }) {
  const { t } = useTranslation('admin');
  const { can } = useAuth();
  const canPay = can('admin_reimb_pay', 'update');
  const { data: raw, reload } = useApi<PayoutBatch[]>('/reimbursements/batches');
  const batches = Array.isArray(raw) ? raw : [];
  const [month, setMonth] = useState(() => todayStr().slice(0, 7));
  const [payBatch, setPayBatch] = useState<PayoutBatch | null>(null);
  const [verified, setVerified] = useState(false);
  const [paidRef, setPaidRef] = useState('');
  const { busy, error, run } = useAction();

  const createBatch = () => {
    run(async () => {
      await post('/reimbursements/batches', { month });
      await reload();
      onChanged();
    }, t('reimb.batchCreated'));
  };

  const cancelBatch = (id: number) => {
    run(async () => {
      await del(`/reimbursements/batches/${id}`);
      await reload();
      onChanged();
    }, t('reimb.batchCancelled'));
  };

  const downloadCsv = (b: PayoutBatch) => {
    run(async () => {
      const res = await fetch(`/api/reimbursements/batches/${b.id}/csv`, { credentials: 'same-origin' });
      if (!res.ok) {
        let err: { error?: string } | null = null;
        try {
          err = await res.json();
        } catch {
          /* fall through to statusText */
        }
        throw new Error(err?.error || res.statusText);
      }
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `payout-batch-${b.month}-${b.id}.csv`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    });
  };

  const confirmPay = () => {
    if (!payBatch) return;
    run(async () => {
      await post(`/reimbursements/batches/${payBatch.id}/pay`, {
        payee_verified: verified,
        paid_ref: paidRef.trim(),
      });
      setPayBatch(null);
      setVerified(false);
      setPaidRef('');
      await reload();
      onChanged();
    }, t('reimb.batchPaidDone'));
  };

  return (
    <Card className="stack-sm">
      <div className="row between wrap gap-sm">
        <div>
          <h3 className="no-margin">{t('reimb.batchTitle')}</h3>
          <p className="muted tiny no-margin">{t('reimb.batchDesc')}</p>
        </div>
        {canPay && (
          <div className="row nowrap gap-xs">
            <input
              type="month"
              className="input"
              aria-label={t('reimb.batchMonth')}
              value={month}
              onChange={(e) => setMonth(e.target.value)}
            />
            <Button variant="primary" size="sm" disabled={busy || !month} onClick={createBatch}>
              {t('reimb.batchCreate')}
            </Button>
          </div>
        )}
      </div>

      <ErrorBox error={error} />

      {batches.length > 0 && (
        <div className="stack-sm">
          {batches.map((b) => (
            <div key={b.id} className="card flat tight stack-xs">
              <div className="row between wrap gap-sm">
                <div className="min-w-0">
                  <div className="row nowrap gap-xs align-center">
                    <span className="bold small">{b.title}</span>
                    <span className="mono bold">{rm(b.total_cents)}</span>
                    <Pill tone={b.status === 'paid' ? 'success' : 'warning'}>
                      {b.status === 'paid' ? t('reimb.batchStatusPaid') : t('reimb.batchStatusOpen')}
                    </Pill>
                  </div>
                  <div className="muted tiny">
                    {b.month} · {b.created_by} ({t('reimb.batchCount', { count: b.claim_count })})
                    {b.status === 'paid' && b.paid_ref ? ` · ${b.paid_ref}` : ''}
                    {b.paid_by ? ` · ${b.paid_by}` : ''}
                  </div>
                </div>
                <div className="row nowrap gap-xs">
                  <Button
                    variant="secondary"
                    size="sm"
                    disabled={busy}
                    onClick={() => downloadCsv(b)}
                    icon={<Download size={14} />}
                  >
                    {t('reimb.batchCsv')}
                  </Button>
                  {b.status === 'open' && canPay && (
                    <>
                      <Button
                        variant="ghost"
                        size="sm"
                        disabled={busy}
                        onClick={() => cancelBatch(b.id)}
                      >
                        {t('reimb.batchCancel')}
                      </Button>
                      <Button
                        variant="primary"
                        size="sm"
                        onClick={() => {
                          setPayBatch(b);
                          setVerified(false);
                          setPaidRef('');
                        }}
                      >
                        {t('reimb.batchPayBtn')}
                      </Button>
                    </>
                  )}
                </div>
              </div>
              <BatchFlowStepper batch={b} />
            </div>
          ))}
        </div>
      )}

      {payBatch && (
        <Modal
          title={t('reimb.batchPayTitle', { title: payBatch.title, amount: rm(payBatch.total_cents) })}
          onClose={() => setPayBatch(null)}
        >
          <div className="stack-sm">
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
                value={paidRef}
                onChange={(e) => setPaidRef(e.target.value)}
              />
            </Field>
            <ErrorBox error={error} />
            <Button
              variant="primary"
              block
              disabled={busy || !verified || paidRef.trim().length < 4}
              onClick={confirmPay}
            >
              {t('reimb.record')}
            </Button>
          </div>
        </Modal>
      )}
    </Card>
  );
}

/** Visual 4-step stepper illustrating the Maker-Checker payout lifecycle */
function BatchFlowStepper({ batch }: { batch: PayoutBatch }) {
  const { t } = useTranslation('admin');
  const isPaid = batch.status === 'paid';
  const steps = [
    { key: 'created', label: t('reimb.stepCreated'), done: true, sub: batch.created_by },
    { key: 'csv', label: t('reimb.stepCsv'), done: true, sub: 'DuitNow CSV' },
    { key: 'disburse', label: t('reimb.stepDisburse'), done: isPaid, sub: isPaid ? 'Done' : t('reimb.batchStatusOpen') },
    { key: 'finalized', label: t('reimb.stepFinalized'), done: isPaid, sub: isPaid ? (batch.paid_ref || 'Verified') : 'Pending' },
  ];
  return (
    <div className="payout-stepper" role="region" aria-label="Batch progress">
      {steps.map((s, idx) => (
        <div key={s.key} className={`payout-step-item ${s.done ? 'step-done' : 'step-pending'}`}>
          <div className="step-circle">{s.done ? '✓' : idx + 1}</div>
          <div className="step-content">
            <span className="step-label">{s.label}</span>
            <span className="step-sub muted tiny">{s.sub}</span>
          </div>
          {idx < steps.length - 1 && <div className={`step-connector ${isPaid || idx === 0 ? 'connector-done' : ''}`} />}
        </div>
      ))}
    </div>
  );
}

/** A visitor special reimbursement, marked as such with its visitor, reason and requester. */
function SpecialSummary({ r }: { r: Reimb }) {
  const { t } = useTranslation('admin');
  return (
    <div className="stack-xs">
      <div className="row wrap gap-xs">
        <Pill tone="info">{t('reimb.visitorSpecialKind')}</Pill>
        {r.high_amount && <Pill tone="amber">{t('reimb.highAmount')}</Pill>}
        {r.visitor?.name && <span className="bold">{t('reimb.visitorSpecialFor', { name: r.visitor.name })}</span>}
      </div>
      <div>
        <strong>{t('reimb.specialReason')}:</strong> {r.special_reason}
      </div>
      {r.requested_by?.name && <div className="muted tiny">{t('reimb.requestedBy', { name: r.requested_by.name })}</div>}
    </div>
  );
}
