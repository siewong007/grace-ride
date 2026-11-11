import { FormEvent, useEffect, useState } from 'react';
import { Link, useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { Plus } from 'lucide-react';
import { post, put, useApi } from '../api';
import { useAuth } from '../auth';
import { Empty, ErrorBox, Field, FileInput, ReceiptLink, Spinner, StatusPill, useAction } from '../components/ui';
import { fmtDateTime, fmtDay, REIMB_STATUS, rm, toCents, useFormatT } from '../format';
import { useTranslation } from 'react-i18next';

export interface Reimb {
  id: number; user: { id: number; name: string }; ride_id: number | null; ride_at: string | null; amount_cents: number;
  description: string; receipt_file_id: number; payee_name: string | null; payee_phone: string | null; status: string;
  review_note: string; reviewer: string | null; payee_verified: boolean; paid_ref: string | null; paid_at: string | null; created_at: string;
}

export default function Reimbursements() {
  const { t } = useTranslation('reimbursements');
  const ft = useFormatT();
  const { data, error, loading } = useApi<Reimb[]>('/reimbursements');
  return (
    <div className="stack">
      <div className="page-head">
        <div>
          <h1>{t('title')}</h1>
          <p>{t('desc')}</p>
        </div>
        <Link className="btn primary" to="/reimbursements/new"><Plus size={20} />{t('apply')}</Link>
      </div>
      <ol className="steps card">
        <li>{t('step1')}</li>
        <li>{t('step2')}</li>
        <li>{t('step3')}</li>
        <li>{t('step4')}</li>
      </ol>
      <ErrorBox error={error} />
      {loading ? <Spinner /> : !data?.length ? <Empty>{t('none')}</Empty> : (
        <div className="grid cols-2">
          {data.map((r) => (
            <Link key={r.id} to={`/reimbursements/${r.id}`} className="card link stack-sm">
              <div className="row between"><strong className="mono">{rm(r.amount_cents)}</strong><StatusPill map={REIMB_STATUS} status={r.status} /></div>
              <div className="small">{r.description || t('noneDesc')}</div>
              <div className="muted tiny">{fmtDay(r.created_at, ft)}{r.ride_at && ` · ${t('ridePrefix', { time: fmtDateTime(r.ride_at, ft) })}`}</div>
            </Link>
          ))}
        </div>
      )}
      <p className="muted small center">{t('note')}</p>
    </div>
  );
}

export function ReimbursementForm() {
  const { t } = useTranslation('reimbursements');
  const ft = useFormatT();
  const nav = useNavigate();
  const [params] = useSearchParams();
  const editId = params.get('edit');
  const { me } = useAuth();
  const past = useApi<any[]>('/rides?scope=past');
  const existing = useApi<Reimb>(editId ? `/reimbursements/${editId}` : null);
  const [f, setF] = useState({ ride: params.get('ride') ?? '', amount: '', desc: '', file: null as number | null, payee: me?.name ?? '', phone: me?.phone ?? '' });
  const { busy, error, run } = useAction();

  useEffect(() => {
    const r = existing.data;
    if (r) setF({ ride: r.ride_id ? String(r.ride_id) : '', amount: (r.amount_cents / 100).toFixed(2), desc: r.description, file: r.receipt_file_id, payee: r.payee_name ?? '', phone: r.payee_phone ?? '' });
  }, [existing.data]);

  const submit = (e: FormEvent) => {
    e.preventDefault();
    const body = { ride_id: f.ride ? Number(f.ride) : null, amount_cents: toCents(f.amount), description: f.desc, receipt_file_id: f.file, payee_name: f.payee, payee_phone: f.phone };
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
    <div className="stack" style={{ maxWidth: 720 }}>
      <h1>{editId ? t('editTitle') : t('applyTitle')}</h1>
      <form className="card stack" onSubmit={submit}>
        <Field label={t('ride')}>
          <select className="input" value={f.ride} onChange={(e) => setF({ ...f, ride: e.target.value })}>
            <option value="">{t('noneRide')}</option>
            {past.data?.map((r) => <option key={r.id} value={r.id}>{fmtDateTime(r.pickup_at, ft)} · {r.area.name} → {r.destination}</option>)}
          </select>
        </Field>
        <Field label={t('amount')}>
          <input className="input" inputMode="decimal" required value={f.amount} onChange={(e) => setF({ ...f, amount: e.target.value })} placeholder={t('amountPlaceholder')} />
        </Field>
        <Field label={t('desc')}>
          <input className="input" value={f.desc} onChange={(e) => setF({ ...f, desc: e.target.value })} placeholder={t('descPlaceholder')} />
        </Field>
        <FileInput label={t('receipt')} value={f.file} onChange={(id) => setF({ ...f, file: id })} />
        <div className="form-grid two">
          <Field label={t('payeeName')}>
            <input className="input" required value={f.payee} onChange={(e) => setF({ ...f, payee: e.target.value })} />
          </Field>
          <Field label={t('payeePhone')}>
            <input className="input" type="tel" required value={f.phone} onChange={(e) => setF({ ...f, phone: e.target.value })} />
          </Field>
        </div>
        <ErrorBox error={error} />
        <button className="btn primary block" disabled={busy || !f.file}>{t('submitBtn')}</button>
      </form>
    </div>
  );
}

export function ReimbursementDetail() {
  const { t } = useTranslation('reimbursements');
  const ft = useFormatT();
  const { id } = useParams();
  const { data: r, error } = useApi<Reimb>(`/reimbursements/${id}`);
  if (error) return <ErrorBox error={error} />;
  if (!r) return <Spinner />;
  return (
    <div className="stack" style={{ maxWidth: 720 }}>
      <div className="page-head">
        <h1>{t('detailTitle', { amount: rm(r.amount_cents) })}</h1>
        <StatusPill map={REIMB_STATUS} status={r.status} />
      </div>
      <div className="card stack">
        <dl className="kv">
          <dt>{t('applicant')}</dt><dd>{r.user.name}</dd>
          <dt>{t('description')}</dt><dd>{r.description || t('noneDesc')}</dd>
          {r.ride_id && <><dt>{t('rideLink')}</dt><dd><Link to={`/rides/${r.ride_id}`}>{r.ride_at && fmtDateTime(r.ride_at, ft)}</Link></dd></>}
          {r.payee_name && <><dt>{t('payee')}</dt><dd>{r.payee_name} · {r.payee_phone}</dd></>}
          <dt>{t('submittedAt')}</dt><dd>{fmtDateTime(r.created_at, ft)}</dd>
          {r.reviewer && <><dt>{t('reviewer')}</dt><dd>{r.reviewer}</dd></>}
          {r.paid_at && <><dt>{t('paid')}</dt><dd>{fmtDateTime(r.paid_at, ft)} · {r.paid_ref}</dd></>}
        </dl>
        {r.review_note && <div className="notice">{t('reviewNote')}: {r.review_note}</div>}
        <ReceiptLink id={r.receipt_file_id} />
        {r.status === 'returned' && <Link className="btn primary" to={`/reimbursements/new?edit=${r.id}`}>{t('resubmit')}</Link>}
      </div>
    </div>
  );
}
