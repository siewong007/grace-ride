import { FormEvent, useEffect, useState } from 'react';
import { Link, useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { AlertCircle, CheckCircle2, Circle, CircleDot, Plus } from 'lucide-react';
import { post, put, useApi } from '../api';
import { useAuth } from '../auth';
import {
  Button,
  Card,
  ErrorBox,
  Field,
  FileInput,
  Notice,
  Page,
  PageHeader,
  Pagination,
  ReceiptLink,
  Spinner,
  StatusPill,
  useAction,
} from '../components/ui';
import { fmtDateTime, fmtDay, REIMB_STATUS, rm, toCents, useFormatT } from '../format';
import { useTranslation } from 'react-i18next';

export interface Reimb {
  id: number;
  user: { id: number; name: string };
  ride_id: number | null;
  ride_at: string | null;
  kind?: 'grab' | 'mileage';
  ride_route?: string | null;
  event_id?: number | null;
  event_title?: string | null;
  event_starts_at?: string | null;
  amount_cents: number;
  description: string;
  receipt_file_id: number;
  payee_name: string | null;
  payee_phone: string | null;
  status: string;
  review_note: string;
  reviewer: string | null;
  payee_verified: boolean;
  paid_ref: string | null;
  paid_at: string | null;
  created_at: string;
}

export interface PaginatedReimbursements {
  items: Reimb[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export default function Reimbursements() {
  const { t } = useTranslation('reimbursements');
  const ft = useFormatT();
  const { can } = useAuth();
  const [params, setParams] = useSearchParams();
  const page = Math.max(1, Number(params.get('page') ?? '1') || 1);
  const pageSize = Number(params.get('limit') ?? '25') || 25;
  const pagingQs = page > 1 || pageSize !== 25 ? `?page=${page}&limit=${pageSize}` : '';
  const { data, error, loading } = useApi<PaginatedReimbursements | Reimb[]>(`/reimbursements${pagingQs}`);

  const items = Array.isArray(data) ? data : data?.items ?? [];
  const total = Array.isArray(data) ? data.length : data?.total ?? 0;
  const totalPages = Array.isArray(data) ? 1 : data?.total_pages ?? 1;

  return (
    <Page width="medium">
      <div className="stack">
        <PageHeader
          title={t('title')}
          subtitle={t('note')}
          back="/me"
          actions={
            can('reimbursements', 'create') ? (
              <Button variant="primary" to="/reimbursements/new" icon={<Plus size={20} />}>
                {t('apply')}
              </Button>
            ) : undefined
          }
        />

        <ErrorBox error={error} />

        {loading ? (
          <Spinner />
        ) : !items.length ? (
          <div className="stack">
            <p className="muted">{t('none')}</p>
            {/* How it works: only useful before the first request. */}
            <ol className="steps">
              <li>{t('step1')}</li>
              <li>{t('step2')}</li>
              <li>{t('step3')}</li>
              <li>{t('step4')}</li>
            </ol>
          </div>
        ) : (
          <>
            <div className="list">
              {items.map((r) => (
                <Link key={r.id} to={`/reimbursements/${r.id}`} className="item money-row">
                  <div className="grow min-w-0">
                    <div className="bold ellipsis">{r.description || t('noneDesc')}</div>
                    <div className="muted small">
                      {fmtDay(r.created_at, ft)}
                      {r.event_title && ` · ${r.event_title}`}
                      {r.ride_at && ` · ${t('ridePrefix', { time: fmtDateTime(r.ride_at, ft) })}`}
                    </div>
                  </div>
                  <div className="money-row-side">
                    <span className="money">{rm(r.amount_cents)}</span>
                    <StatusPill map={REIMB_STATUS} status={r.status} />
                  </div>
                </Link>
              ))}
            </div>
            <Pagination
              page={page}
              totalPages={totalPages}
              total={total}
              pageSize={pageSize}
              onPageChange={(p) => {
                const next = new URLSearchParams(params);
                if (p > 1) next.set('page', String(p));
                else next.delete('page');
                setParams(next, { replace: true });
              }}
              onPageSizeChange={(sz) => {
                const next = new URLSearchParams(params);
                if (sz !== 25) next.set('limit', String(sz));
                else next.delete('limit');
                next.delete('page');
                setParams(next, { replace: true });
              }}
            />
          </>
        )}
      </div>
    </Page>
  );
}

interface PastRide {
  id: number;
  pickup_at: string;
  area: { name: string };
  destination: string;
  event?: { id: number; title: string } | null;
}

interface RegisteredEvent {
  id: number;
  title: string;
  starts_at: string;
}

export function ReimbursementForm() {
  const { t } = useTranslation('reimbursements');
  const ft = useFormatT();
  const nav = useNavigate();
  const [params] = useSearchParams();
  const editId = params.get('edit');
  const { me } = useAuth();
  const past = useApi<PastRide[] | { items: PastRide[] }>('/rides?scope=past');
  const pastRides = Array.isArray(past.data) ? past.data : past.data?.items ?? [];
  const registeredEventsApi = useApi<RegisteredEvent[]>('/events?scope=registered');
  const registeredEvents = Array.isArray(registeredEventsApi.data) ? registeredEventsApi.data : [];
  const existing = useApi<Reimb>(editId ? `/reimbursements/${editId}` : null);
  const mileage = params.get('kind') === 'mileage' || existing.data?.kind === 'mileage';
  const [f, setF] = useState({
    event_id: params.get('event') ?? '',
    ride: params.get('ride') ?? '',
    amount: '',
    desc: '',
    file: null as number | null,
    payee: me?.name ?? '',
    phone: me?.phone ?? '',
  });
  const { busy, error, run } = useAction();

  useEffect(() => {
    const r = existing.data;
    if (r) {
      setF({
        event_id: r.event_id ? String(r.event_id) : '',
        ride: r.ride_id ? String(r.ride_id) : '',
        amount: (r.amount_cents / 100).toFixed(2),
        desc: r.description,
        file: r.receipt_file_id,
        payee: r.payee_name ?? '',
        phone: r.payee_phone ?? '',
      });
    }
  }, [existing.data]);

  useEffect(() => {
    if (!f.event_id && f.ride && pastRides.length > 0) {
      const matched = pastRides.find((r) => String(r.id) === f.ride);
      if (matched?.event?.id) {
        setF((prev) => ({ ...prev, event_id: String(matched.event!.id) }));
      }
    }
  }, [f.ride, f.event_id, pastRides]);

  const submit = (e: FormEvent) => {
    e.preventDefault();
    const body = {
      kind: mileage ? 'mileage' : 'grab',
      event_id: !mileage && f.event_id ? Number(f.event_id) : undefined,
      ride_id: f.ride ? Number(f.ride) : null,
      amount_cents: toCents(f.amount),
      description: f.desc,
      receipt_file_id: f.file,
      payee_name: f.payee,
      payee_phone: f.phone,
    };
    run(async () => {
      if (editId) {
        await put(`/reimbursements/${editId}`, body);
        nav(`/reimbursements/${editId}`);
      } else {
        const r = await post<{ id: number }>('/reimbursements', body);
        nav(`/reimbursements/${r.id}`);
      }
    }, t('submitted'));
  };

  return (
    <Page width="narrow">
      <div className="stack">
        <PageHeader
          title={editId ? t('editTitle') : mileage ? t('mileageTitle') : t('applyTitle')}
          back="/reimbursements"
        />

        <form className="card stack" onSubmit={submit}>
          {mileage && <Notice tone="info">{t('mileageNote')}</Notice>}
          {!mileage && (
            <Field label={t('event')}>
              <select
                className="input"
                required
                value={f.event_id}
                onChange={(e) => {
                  const eid = e.target.value;
                  setF((prev) => ({
                    ...prev,
                    event_id: eid,
                    ride:
                      prev.ride &&
                      pastRides.find((r) => String(r.id) === prev.ride)?.event?.id !== Number(eid)
                        ? ''
                        : prev.ride,
                  }));
                }}
              >
                <option value="">{t('selectEvent')}</option>
                {registeredEvents.map((ev) => (
                  <option key={ev.id} value={ev.id}>
                    {fmtDateTime(ev.starts_at, ft)} · {ev.title}
                  </option>
                ))}
              </select>
              {registeredEvents.length === 0 && !registeredEventsApi.loading && (
                <div className="small muted text-accent" style={{ marginTop: '4px' }}>
                  {t('noRegisteredEventsNotice')}
                </div>
              )}
            </Field>
          )}

          {mileage ? (
            <Field label={t('ride')}>
              <div className="muted">
                {existing.data?.ride_route ?? `#${f.ride}`}
                {' · '}
                <Link to={`/rides/${f.ride}`}>{t('viewRide')}</Link>
              </div>
            </Field>
          ) : (
            <Field label={t('ride')} hint={t('rideOptional')}>
              <select
                className="input"
                value={f.ride}
                onChange={(e) => {
                  const rid = e.target.value;
                  const matched = pastRides.find((r) => String(r.id) === rid);
                  setF((prev) => ({
                    ...prev,
                    ride: rid,
                    event_id: matched?.event?.id ? String(matched.event.id) : prev.event_id,
                  }));
                }}
              >
                <option value="">{t('noneRide')}</option>
                {(f.event_id
                  ? pastRides.filter((r) => !r.event || r.event.id === Number(f.event_id))
                  : pastRides
                ).map((r) => (
                  <option key={r.id} value={r.id}>
                    {fmtDateTime(r.pickup_at, ft)} · {r.area.name} → {r.destination}
                  </option>
                ))}
              </select>
            </Field>
          )}

          <Field label={t('amount')}>
            <input
              className="input"
              inputMode="decimal"
              required
              value={f.amount}
              onChange={(e) => setF({ ...f, amount: e.target.value })}
              placeholder={t('amountPlaceholder')}
            />
          </Field>

          <Field label={t('desc')}>
            <input
              className="input"
              value={f.desc}
              onChange={(e) => setF({ ...f, desc: e.target.value })}
              placeholder={t('descPlaceholder')}
            />
          </Field>

          <div>
            <FileInput label={mileage ? t('mileageReceipt') : t('receipt')} value={f.file} onChange={(id) => setF({ ...f, file: id })} />
            {f.file && (
              <div className="receipt-drop-preview">
                <img
                  key={f.file}
                  className="receipt-drop-thumb"
                  src={`/api/files/${f.file}`}
                  alt={t('receipt')}
                  onError={(e) => {
                    e.currentTarget.style.display = 'none';
                  }}
                />
                <span className="small muted">{t('receipt')} #{f.file}</span>
              </div>
            )}
          </div>

          <div className="form-grid two">
            <Field label={t('payeeName')}>
              <input
                className="input"
                required
                value={f.payee}
                onChange={(e) => setF({ ...f, payee: e.target.value })}
              />
            </Field>
            <Field label={t('payeePhone')}>
              <input
                className="input"
                type="tel"
                required
                value={f.phone}
                onChange={(e) => setF({ ...f, phone: e.target.value })}
              />
            </Field>
          </div>

          <ErrorBox error={error} />
          <Button
            variant="primary"
            block
            type="submit"
            disabled={busy || !f.file || (!mileage && !f.event_id)}
            loading={busy}
            size="lg"
          >
            {t('submitBtn')}
          </Button>
        </form>
      </div>
    </Page>
  );
}

export function ReimbursementDetail() {
  const { t } = useTranslation('reimbursements');
  const ft = useFormatT();
  const { can } = useAuth();
  const { id } = useParams();
  const { data: r, error } = useApi<Reimb>(`/reimbursements/${id}`);

  if (error) return <ErrorBox error={error} />;
  if (!r) return <Spinner />;

  // Status timeline steps: submitted -> reviewed -> paid
  const isPaid = r.status === 'paid';
  const isReviewed = r.status === 'approved' || isPaid;
  const isRejected = r.status === 'rejected' || r.status === 'returned';

  return (
    <Page width="narrow">
      <div className="stack">
        <PageHeader
          title={t('detailTitle', { amount: rm(r.amount_cents) })}
          back="/reimbursements"
          actions={<StatusPill map={REIMB_STATUS} status={r.status} />}
        />

        {/* Status Timeline */}
        <Card className="stack-sm">
          <h3>{t('statusTimeline')}</h3>
          <div className="status-timeline">
            {/* Step 1: Submitted */}
            <div className="timeline-step done">
              <div className="timeline-step-indicator">
                <div className="timeline-step-icon">
                  <CheckCircle2 size={14} aria-hidden="true" />
                </div>
                <div className="timeline-step-line" />
              </div>
              <div className="timeline-step-content">
                <span className="timeline-step-title">{t('stepSubmitted')}</span>
                <span className="timeline-step-desc">{fmtDateTime(r.created_at, ft)}</span>
              </div>
            </div>

            {/* Step 2: Reviewed */}
            <div className={`timeline-step ${isReviewed ? 'done' : isRejected ? 'cancelled' : 'current'}`}>
              <div className="timeline-step-indicator">
                <div className="timeline-step-icon">
                  {isReviewed ? (
                    <CheckCircle2 size={14} aria-hidden="true" />
                  ) : isRejected ? (
                    <AlertCircle size={14} aria-hidden="true" />
                  ) : (
                    <CircleDot size={14} aria-hidden="true" />
                  )}
                </div>
                <div className="timeline-step-line" />
              </div>
              <div className="timeline-step-content">
                <span className="timeline-step-title">{t('stepReviewed')}</span>
                {r.reviewer && (
                  <span className="timeline-step-desc">{r.reviewer}</span>
                )}
                {r.review_note && (
                  <span className="timeline-step-desc text-accent">{r.review_note}</span>
                )}
              </div>
            </div>

            {/* Step 3: Paid */}
            <div className={`timeline-step ${isPaid ? 'done' : 'upcoming'}`}>
              <div className="timeline-step-indicator">
                <div className="timeline-step-icon">
                  {isPaid ? (
                    <CheckCircle2 size={14} aria-hidden="true" />
                  ) : (
                    <Circle size={10} aria-hidden="true" />
                  )}
                </div>
              </div>
              <div className="timeline-step-content">
                <span className="timeline-step-title">{t('stepPaid')}</span>
                {r.paid_at && (
                  <span className="timeline-step-desc">
                    {fmtDateTime(r.paid_at, ft)} {r.paid_ref ? `· ${r.paid_ref}` : ''}
                  </span>
                )}
              </div>
            </div>
          </div>
        </Card>

        <Card className="stack">
          <dl className="kv">
            <dt>{t('applicant')}</dt>
            <dd>{r.user.name}</dd>
            <dt>{t('description')}</dt>
            <dd>{r.description || t('noneDesc')}</dd>
            {r.event_title && (
              <>
                <dt>{t('event')}</dt>
                <dd>
                  <Link to={`/events/${r.event_id}`}>
                    {r.event_title}
                    {r.event_starts_at && ` · ${fmtDateTime(r.event_starts_at, ft)}`}
                  </Link>
                </dd>
              </>
            )}
            {r.kind === 'mileage' && (
              <>
                <dt>{t('route')}</dt>
                <dd>{t('kindMileage')}{r.ride_route ? ` · ${r.ride_route}` : ''}</dd>
              </>
            )}
            {r.ride_id && (
              <>
                <dt>{t('rideLink')}</dt>
                <dd>
                  <Link to={`/rides/${r.ride_id}`}>
                    {r.ride_at ? fmtDateTime(r.ride_at, ft) : t('viewRide')}
                  </Link>
                </dd>
              </>
            )}
            {r.payee_name && (
              <>
                <dt>{t('payee')}</dt>
                <dd>{r.payee_name} · {r.payee_phone}</dd>
              </>
            )}
            <dt>{t('submittedAt')}</dt>
            <dd>{fmtDateTime(r.created_at, ft)}</dd>
            {r.reviewer && (
              <>
                <dt>{t('reviewer')}</dt>
                <dd>{r.reviewer}</dd>
              </>
            )}
            {r.paid_at && (
              <>
                <dt>{t('paid')}</dt>
                <dd>{fmtDateTime(r.paid_at, ft)} · {r.paid_ref}</dd>
              </>
            )}
          </dl>

          <ReceiptLink id={r.receipt_file_id} />
          {r.status === 'returned' && can('reimbursements', 'update') && (
            <Button variant="primary" block to={`/reimbursements/new?edit=${r.id}`}>
              {t('resubmit')}
            </Button>
          )}
        </Card>
      </div>
    </Page>
  );
}
