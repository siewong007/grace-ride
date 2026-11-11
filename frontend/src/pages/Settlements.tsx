import { FormEvent, useEffect, useMemo, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { del, post, put, useApi } from '../api';
import { useAuth } from '../auth';
import { Empty, ErrorBox, Field, FileInput, Modal, Spinner, StatusPill, useAction } from '../components/ui';
import { fmtAgo, rm, SHARE_STATUS, toCents, useFormatT } from '../format';
import { useTranslation } from 'react-i18next';

interface ShareRow { id: number; user_id: number; name: string; amount_cents: number; status: string; proof_file_id: number | null; txn_ref: string; paid_date: string | null; note: string; updated_at: string }
interface Settlement {
  id: number; event: { id: number; title: string; starts_at: string }; created_by: { id: number; name: string };
  title: string; split_mode: 'equal' | 'custom'; subsidy_cents: number; payee_name: string; payee_account: string; due_date: string | null;
  status: string; version: number; gross_cents: number; total_cents: number;
  items: { id: number; label: string; amount_cents: number; receipt_file_id: number | null }[];
  shares: ShareRow[]; can_manage: boolean; participant_count?: number;
}

export default function Settlements() {
  const { t } = useTranslation('settlements');
  const { data, error, loading } = useApi<{ id: number; title: string; event_title: string; status: string; total_cents: number; confirmed: number; count: number }[]>('/settlements');
  return (
    <div className="stack">
      <div className="page-head">
        <div>
          <h1>{t('title')}</h1>
          <p>{t('desc')}</p>
        </div>
        <Link className="btn" to="/events?scope=managing">{t('fromEvent')}</Link>
      </div>
      <ErrorBox error={error} />
      {loading ? <Spinner /> : !data?.length ? <Empty>{t('none')}</Empty> : (
        <div className="grid cols-3">
          {data.map((s) => (
            <Link key={s.id} to={`/settlements/${s.id}`} className="card link stack-sm">
              <div className="row between"><strong>{s.title}</strong><StatusPill map={STATUS_MAP(t)} status={s.status} /></div>
              <div className="muted small">{s.event_title}</div>
              <div className="row between"><span className="bold">{rm(s.total_cents)}</span><span className="small">{t('confirmedCount', { confirmed: s.confirmed, count: s.count })}</span></div>
              <div className="bar"><div style={{ width: `${s.count ? (s.confirmed / s.count) * 100 : 0}%` }} /></div>
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}

function STATUS_MAP(t: (k: string, o?: any) => string) {
  return {
    draft: { label: t('statusDraft'), tone: 'gray' as const },
    published: { label: t('statusPublished'), tone: 'blue' as const },
    closed: { label: t('statusClosed'), tone: 'green' as const },
  };
}

interface Signup { user_id: number; name: string; status: string; attended: number | null }

export function SettlementForm() {
  const { t } = useTranslation('settlements');
  const ft = useFormatT();
  const { eventId, id } = useParams();
  const nav = useNavigate();
  const { data: existing, reload } = useApi<Settlement>(id ? `/settlements/${id}` : null);
  const evId = eventId ?? existing?.event.id;
  const { data: signups } = useApi<Signup[]>(evId ? `/events/${evId}/signups` : null);
  const { data: event } = useApi<{ title: string }>(evId ? `/events/${evId}` : null);
  const { data: revisions } = useApi<any[]>(id ? `/settlements/${id}/revisions` : null);

  const [title, setTitle] = useState('');
  const [items, setItems] = useState([{ label: t('itemLabel'), amount: '', receipt: null as number | null }]);
  const [subsidy, setSubsidy] = useState('0');
  const [mode, setMode] = useState<'equal' | 'custom'>('equal');
  const [selected, setSelected] = useState<number[]>([]);
  const [custom, setCustom] = useState<Record<number, string>>({});
  const [payee, setPayee] = useState({ name: '', account: '' });
  const [review, setReview] = useState<ShareRow | null>(null);

  const { busy, error, run } = useAction();

  useEffect(() => {
    if (existing) {
      setTitle(existing.title);
      setSubsidy(String(existing.subsidy_cents / 100));
      setMode(existing.split_mode);
      setPayee({ name: existing.payee_name, account: existing.payee_account });
      setItems(existing.items.map((it) => ({ label: it.label, amount: String(it.amount_cents / 100), receipt: it.receipt_file_id })));
      if (existing.shares) {
        setSelected(existing.shares.map((s) => s.user_id));
        const c: Record<number, string> = {};
        existing.shares.forEach((s) => {
          c[s.user_id] = String(s.amount_cents / 100);
        });
        setCustom(c);
      }
    }
  }, [existing]);

  const addItem = () => setItems([...items, { label: '', amount: '', receipt: null }]);
  const removeItem = (i: number) => setItems(items.filter((_, ix) => ix !== i));
  const total = useMemo(() => items.reduce((s, it) => s + (toCents(it.amount) || 0), 0) - (toCents(subsidy) || 0), [items, subsidy]);
  const perPerson = selected.length ? Math.round(total / selected.length) : 0;

  const customSum = useMemo(() => {
    return selected.reduce((sum, uid) => {
      const v = custom[uid];
      const cents = v !== undefined && v !== '' ? toCents(v) : null;
      return sum + (cents !== null && cents >= 0 ? cents : 0);
    }, 0);
  }, [selected, custom]);

  const customValid = useMemo(() => {
    if (mode !== 'custom') return true;
    if (selected.length === 0) return false;
    for (const uid of selected) {
      const v = custom[uid];
      const cents = v !== undefined && v !== '' ? toCents(v) : null;
      if (cents === null || cents < 0) return false;
    }
    return customSum === total;
  }, [mode, selected, custom, customSum, total]);

  const submit = (e: FormEvent) => {
    e.preventDefault();
    if (!customValid) return;
    const body = {
      title, event_id: evId ? Number(evId) : undefined,
      items: items.map((it) => ({ label: it.label, amount_cents: toCents(it.amount), receipt_file_id: it.receipt })),
      subsidy_cents: toCents(subsidy), split_mode: mode, payee_name: payee.name, payee_account: payee.account,
      shares: mode === 'equal' ? selected.map((uid) => ({ user_id: uid, amount_cents: perPerson })) : selected.map((uid) => ({ user_id: uid, amount_cents: toCents(custom[uid] || '0') })),
    };
    run(async () => {
      if (id) { await put(`/settlements/${id}`, body); nav(`/settlements/${id}`); }
      else { const r = await post<{ id: number }>('/settlements', body); nav(`/settlements/${r.id}`); }
    }, id ? t('updated') : t('billPublished'));
  };

  const confirmedShares = existing ? existing.shares.filter((x) => x.status === 'confirmed').length : 0;

  return (
    <div className="stack" style={{ maxWidth: 900 }}>
      <h1>{id ? t('edit') : t('settlementTitle')}</h1>
      <form className="card stack" onSubmit={submit}>
        <div className="form-grid two">
          <Field label={t('titleLabel')}>
            <input className="input" required value={title} onChange={(e) => setTitle(e.target.value)} placeholder={t('titlePlaceholder')} />
          </Field>
          <div className="full">
            <Field label={t('eventLabel')}>{event ? <span className="input">{event.title}</span> : <span className="input muted">—</span>}</Field>
          </div>
          <Field label={t('subsidyLabel')}>
            <input className="input" value={subsidy} onChange={(e) => setSubsidy(e.target.value)} placeholder="0" />
          </Field>
          <Field label={t('splitMode')}>
            <select className="input" value={mode} onChange={(e) => setMode(e.target.value as any)}>
              <option value="equal">{t('splitEqual')}</option>
              <option value="custom">{t('splitCustom')}</option>
            </select>
          </Field>
          <Field label={t('payeeLabel')}>
            <input className="input" required value={payee.name} onChange={(e) => setPayee({ ...payee, name: e.target.value })} />
          </Field>
          <Field label={t('payeeAccountLabel')}>
            <input className="input" required value={payee.account} onChange={(e) => setPayee({ ...payee, account: e.target.value })} />
          </Field>
        </div>
        <div className="stack-sm">
          <strong>{t('item')}</strong>
          <div className="list">
            {items.map((it, i) => (
              <div key={i} className="item row between">
                <input className="input" value={it.label} onChange={(e) => setItems(items.map((x, ix) => ix === i ? { ...x, label: e.target.value } : x))} placeholder={t('itemLabel')} />
                <input className="input" type="number" step="0.01" value={it.amount} onChange={(e) => setItems(items.map((x, ix) => ix === i ? { ...x, amount: e.target.value } : x))} placeholder={t('amount')} />
                <FileInput label={t('receipt')} value={it.receipt} onChange={(rid) => setItems(items.map((x, ix) => ix === i ? { ...x, receipt: rid } : x))} />
                {items.length > 1 && <button type="button" className="btn sm danger" onClick={() => removeItem(i)}>{t('delete')}</button>}
              </div>
            ))}
            <button type="button" className="btn ghost sm" onClick={addItem}>+ {t('item')}</button>
          </div>
        </div>
        <div className="stack-sm">
          <strong>{t('participantsLabel')}</strong>
          {signups ? (
            <div className="grid cols-3">
              {signups.map((s) => (
                <div key={s.user_id} className="stack-xs">
                  <label className={`check ${selected.includes(s.user_id) ? 'on' : ''}`}>
                    <input type="checkbox" checked={selected.includes(s.user_id)} onChange={() => setSelected(selected.includes(s.user_id) ? selected.filter((x) => x !== s.user_id) : [...selected, s.user_id])} />
                    {s.name} {mode === 'equal' && ` · ${rm(perPerson)}`}
                  </label>
                  {mode === 'custom' && selected.includes(s.user_id) && (
                    <input
                      type="number"
                      step="0.01"
                      min="0"
                      className="input"
                      placeholder="0.00"
                      value={custom[s.user_id] ?? ''}
                      onChange={(e) => setCustom({ ...custom, [s.user_id]: e.target.value })}
                    />
                  )}
                </div>
              ))}
            </div>
          ) : <div className="muted">加载中…</div>}
          {mode === 'custom' && selected.length > 0 && customSum !== total && (
            <div className="small muted" style={{ color: 'var(--danger)' }}>
              {t('customMismatch', { sum: rm(customSum), total: rm(Math.max(total, 0)) })}
            </div>
          )}
        </div>
        <div className="row between"><strong>{t('totalLabel')}</strong><strong>{rm(Math.max(total, 0))}</strong></div>
        <ErrorBox error={error} />
        <button className="btn primary block" disabled={busy || !title || !payee.name || !payee.account || selected.length === 0 || total <= 0 || !customValid}>
          {busy ? '保存中…' : (id ? t('saveDraft') : t('publishBill'))}
        </button>
      </form>

      {id && existing && (
        <>
          {existing.status === 'draft' && (
            <button className="btn danger" disabled={busy} onClick={() => run(() => del(`/settlements/${existing.id}`).then(() => nav('/settlements')), t('deletedBill'))}>{t('deleteDraft')}</button>
          )}
          {existing.status === 'published' && (
            <button className="btn primary" disabled={busy || confirmedShares < existing.shares.length} onClick={() => run(() => post(`/settlements/${existing.id}/close`).then(reload), t('closed'))}>{t('close')}</button>
          )}
        </>
      )}

      {id && revisions?.length && (
        <div className="card stack-sm">
          <h3>{t('revisions')}</h3>
          {revisions.map((r: any) => <div key={r.version} className="small">v{r.version} → v{r.version + 1} · {r.by} · {fmtAgo(r.created_at, ft)} · {r.reason}</div>)}
        </div>
      )}

      {id && existing && (
        <div className="card stack">
          <div className="row between">
            <h3>{existing.can_manage ? t('verifyReceipts') : t('myPayable')}</h3>
            {existing.can_manage && <span className="muted small">{t('confirmedCount', { confirmed: confirmedShares, count: existing.shares.length })}</span>}
          </div>
          {existing.can_manage && <div className="notice info small">{t('receiptHint')}</div>}
          <div className="list">
            {existing.shares.map((x) => (
              <div key={x.id} className="item">
                <div className="grow">
                  <div className="bold">{x.name} · <span className="mono">{rm(x.amount_cents)}</span></div>
                  {x.txn_ref && <div className="muted tiny">交易编号 {x.txn_ref}{x.paid_date && ` · ${x.paid_date}`}</div>}
                  {x.note && <div className="tiny" style={{ color: 'var(--accent)' }}>{x.note}</div>}
                </div>
                <StatusPill map={SHARE_STATUS} status={x.status} />
                {existing.can_manage && existing.status === 'published' && <button className="btn sm" onClick={() => setReview(x)}>核对</button>}
              </div>
            ))}
          </div>
          {!existing.can_manage && <Link className="btn primary" to="/costs">{t('payLink')}</Link>}
        </div>
      )}

      {review && <VerifyModal share={review} onClose={() => setReview(null)} onDone={() => { setReview(null); if (existing) reload(); }} />}
    </div>
  );
}

export function SettlementDetail() {
  const { t } = useTranslation('settlements');
  const { has, me } = useAuth();
  const { id } = useParams();
  const { data: s, error } = useApi<Settlement>(id ? `/settlements/${id}` : null);
  if (error) return <ErrorBox error={error} />;
  if (!s) return <Spinner />;

  const canEdit = s.can_manage || has('admin') || me?.id === s.created_by.id;
  const paidCents = s.shares.filter((x) => x.status === 'confirmed').reduce((sum, x) => sum + x.amount_cents, 0);
  const paidPercent = s.total_cents > 0 ? Math.min(100, Math.round((paidCents / s.total_cents) * 100)) : 0;

  return (
    <div className="stack" style={{ maxWidth: 820 }}>
      <div className="page-head">
        <div>
          <h1>{t('settlementDetail.title', { title: s.title })}</h1>
          <StatusPill map={getStatusMap(t)} status={s.status} />
        </div>
        {canEdit && <Link className="btn" to={`/settlements/${s.id}/edit`}>{t('settlementDetail.edit')}</Link>}
      </div>
      <div className="card stack">
        <dl className="kv">
          <dt>{t('settlementDetail.event')}</dt><dd>{s.event.title}</dd>
          <dt>{t('settlementDetail.split')}</dt><dd>{s.split_mode === 'equal' ? t('settlementDetail.equal') : t('settlementDetail.custom')}</dd>
          <dt>{t('settlementDetail.payee')}</dt><dd>{s.payee_name} · {s.payee_account}</dd>
          <dt>{t('settlementDetail.subsidy')}</dt><dd>{rm(s.subsidy_cents)}</dd>
          <dt>{t('settlementDetail.total')}</dt><dd><strong>{rm(s.total_cents)}</strong></dd>
        </dl>
        <div className="card stack-sm">
          <h3>{t('settlementDetail.items')}</h3>
          <div className="list">
            {s.items.map((it) => <div key={it.id} className="item"><div className="grow">{it.label}</div><div className="mono">{rm(it.amount_cents)}</div></div>)}
          </div>
        </div>
        <div className="card stack-sm">
          <h3>{t('settlementDetail.shares')}</h3>
          <div className="list">
            {s.shares.map((x) => (
              <div key={x.id} className="item">
                <div className="grow">{x.name} · <span className="mono">{rm(x.amount_cents)}</span></div>
                <StatusPill map={SHARE_STATUS} status={x.status} />
              </div>
            ))}
          </div>
        </div>
        <div className="card stack-sm">
          <div className="row between">
            <h3>{t('settlementDetail.history')}</h3>
            {s.version > 1 && <span className="small muted">v{s.version}</span>}
          </div>
          {s.shares.filter((x) => x.status === 'confirmed').length > 0 && (
            <div className="bar"><div style={{ width: `${paidPercent}%` }} /></div>
          )}
        </div>
      </div>
    </div>
  );
}

function getStatusMap(t: (k: string) => string) {
  return {
    draft: { label: t('statusDraft'), tone: 'gray' as const },
    published: { label: t('statusPublished'), tone: 'blue' as const },
    closed: { label: t('statusClosed'), tone: 'green' as const },
  };
}


function VerifyModal({ share, onClose, onDone }: { share: ShareRow; onClose: () => void; onDone: () => void }) {
  const { t } = useTranslation('settlements');
  const [note, setNote] = useState('');
  const { busy, error, run } = useAction();
  const act = (action: string, msg: string) => run(() => post(`/shares/${share.id}/verify`, { action, note }).then(onDone), msg);
  return (
    <Modal title={`${share.name} · ${rm(share.amount_cents)}`} onClose={onClose}>
      {share.proof_file_id ? (
        <a href={`/api/files/${share.proof_file_id}`} target="_blank" rel="noreferrer"><img className="receipt-thumb" src={`/api/files/${share.proof_file_id}`} alt="付款凭证（PDF 请点击打开）" /></a>
      ) : <p className="muted">尚未上传凭证。</p>}
      {share.txn_ref && <p>交易编号：<strong>{share.txn_ref}</strong>{share.paid_date && ` · ${share.paid_date}`}</p>}
      <Field label="备注（退回时必填）"><input className="input" value={note} onChange={(e) => setNote(e.target.value)} placeholder="例如：金额不符 / 凭证不清" /></Field>
      <ErrorBox error={error} />
      <div className="btn-row">
        <button className="btn primary" disabled={busy} onClick={() => act('confirm', t('confirmReceivedDone'))}>{t('confirmReceived')}</button>
        <button className="btn" disabled={busy} onClick={() => act('needs_info', t('returnInfoDone'))}>{t('returnInfo')}</button>
        {share.status !== 'unpaid' && <button className="btn ghost" disabled={busy} onClick={() => act('reset', t('resetDone'))}>{t('resetStatus')}</button>}
      </div>
    </Modal>
  );
}
