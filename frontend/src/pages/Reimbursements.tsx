import { FormEvent, useEffect, useState } from 'react';
import { Link, useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { AlertCircle, CheckCircle2, Circle, CircleDot, Plus } from 'lucide-react';
import { post, put, useApi } from '../api';
import { useAuth } from '../auth';
import {
  Button,
  Card,
  EmptyState,
  ErrorBox,
  Field,
  FileInput,
  Page,
  PageHeader,
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

export default function Reimbursements() {
  const { t } = useTranslation('reimbursements');
  const ft = useFormatT();
  const { data, error, loading } = useApi<Reimb[]>('/reimbursements');

  return (
    <Page width="medium">
      <div className="stack">
        <PageHeader
          title={t('title')}
          subtitle={t('desc')}
          actions={
            <Button variant="primary" to="/reimbursements/new" icon={<Plus size={20} />}>
              {t('apply')}
            </Button>
          }
        />

        <ol className="steps card">
          <li>{t('step1')}</li>
          <li>{t('step2')}</li>
          <li>{t('step3')}</li>
          <li>{t('step4')}</li>
        </ol>

        <ErrorBox error={error} />

        {loading ? (
          <Spinner />
        ) : !data?.length ? (
          <EmptyState
            title={t('none')}
            action={
              <Button variant="primary" to="/reimbursements/new" icon={<Plus size={18} />}>
                {t('apply')}
              </Button>
            }
          />
        ) : (
          <div className="grid cols-2">
            {data.map((r) => (
              <Link key={r.id} to={`/reimbursements/${r.id}`} className="card link stack-sm">
                <div className="row between nowrap">
                  <strong className="mono">{rm(r.amount_cents)}</strong>
                  <StatusPill map={REIMB_STATUS} status={r.status} />
                </div>
                <div className="small ellipsis">{r.description || t('noneDesc')}</div>
                <div className="muted tiny">
                  {fmtDay(r.created_at, ft)}
                  {r.ride_at && ` · ${t('ridePrefix', { time: fmtDateTime(r.ride_at, ft) })}`}
                </div>
              </Link>
            ))}
          </div>
        )}
        <p className="muted small center">{t('note')}</p>
      </div>
    </Page>
  );
}

interface PastRide {
  id: number;
  pickup_at: string;
  area: { name: string };
  destination: string;
}

export function ReimbursementForm() {
  const { t } = useTranslation('reimbursements');
  const ft = useFormatT();
  const nav = useNavigate();
  const [params] = useSearchParams();
  const editId = params.get('edit');
  const { me } = useAuth();
  const past = useApi<PastRide[]>('/rides?scope=past');
  const existing = useApi<Reimb>(editId ? `/reimbursements/${editId}` : null);
  const [f, setF] = useState({
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
        ride: r.ride_id ? String(r.ride_id) : '',
        amount: (r.amount_cents / 100).toFixed(2),
        desc: r.description,
        file: r.receipt_file_id,
        payee: r.payee_name ?? '',
        phone: r.payee_phone ?? '',
      });
    }
  }, [existing.data]);

  const submit = (e: FormEvent) => {
    e.preventDefault();
    const body = {
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
          title={editId ? t('editTitle') : t('applyTitle')}
          back="/reimbursements"
        />

        <form className="card stack" onSubmit={submit}>
          <Field label={t('ride')}>
            <select
              className="input"
              value={f.ride}
              onChange={(e) => setF({ ...f, ride: e.target.value })}
            >
              <option value="">{t('noneRide')}</option>
              {past.data?.map((r) => (
                <option key={r.id} value={r.id}>
                  {fmtDateTime(r.pickup_at, ft)} · {r.area.name} → {r.destination}
                </option>
              ))}
            </select>
          </Field>

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
            <FileInput label={t('receipt')} value={f.file} onChange={(id) => setF({ ...f, file: id })} />
            {f.file && (
              <div className="receipt-drop-preview">
                <img
                  className="receipt-drop-thumb"
                  src={`/api/files/${f.file}`}
                  alt={t('receipt')}
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
            disabled={busy || !f.file}
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
          {r.status === 'returned' && (
            <Button variant="primary" block to={`/reimbursements/new?edit=${r.id}`}>
              {t('resubmit')}
            </Button>
          )}
        </Card>
      </div>
    </Page>
  );
}
