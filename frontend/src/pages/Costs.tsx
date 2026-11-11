import { useState } from 'react';
import { Link } from 'react-router-dom';
import { Copy } from 'lucide-react';
import { post, useApi } from '../api';
import { Empty, ErrorBox, Field, FileInput, Modal, Spinner, StatusPill, useAction, useToast } from '../components/ui';
import { rm, SHARE_STATUS, todayStr } from '../format';
import { useTranslation } from 'react-i18next';

interface Share {
  id: number; settlement_id: number; title: string; event_title: string; amount_cents: number; status: string;
  payee_name: string; payee_account: string; due_date: string | null; note: string; txn_ref: string; proof_file_id: number | null;
}

export default function Costs() {
  const { t } = useTranslation('costs');
  const { data, error, loading, reload } = useApi<Share[]>('/me/shares');
  const toast = useToast();
  const [paying, setPaying] = useState<Share | null>(null);
  const owing = data?.filter((s) => s.status !== 'confirmed') ?? [];
  const done = data?.filter((s) => s.status === 'confirmed') ?? [];

  const copyAccount = (share: Share) => {
    navigator.clipboard?.writeText(share.payee_account);
    toast(t('copied'));
  };

  return (
    <div className="stack">
      <div className="page-head">
        <div>
          <h1>{t('title')}</h1>
          <p>{t('desc')}</p>
        </div>
      </div>
      <ErrorBox error={error} />
      {loading ? <Spinner /> : !data?.length ? <Empty>{t('none')}</Empty> : (
        <>
          <div className="grid cols-2">
            {owing.map((s) => (
              <div key={s.id} className="card stack">
                <div className="row between">
                  <div>
                    <div className="bold">{s.title}</div>
                    <div className="muted small">{s.event_title}</div>
                  </div>
                  <StatusPill map={SHARE_STATUS} status={s.status} />
                </div>
                <div className="center" style={{ fontSize: '2rem', fontWeight: 800 }}>{rm(s.amount_cents)}</div>
                <div className="card flat tight stack-sm">
                  <div className="small">{t('payee', { name: s.payee_name })}</div>
                  <div className="row between">
                    <span className="mono">{s.payee_account}</span>
                    <button className="btn sm" onClick={() => copyAccount(s)}><Copy size={16} />{t('copy')}</button>
                  </div>
                  {s.due_date && <div className="small muted">{t('due', { date: s.due_date })}</div>}
                </div>
                {s.note && <div className="notice">{s.note}</div>}
                {s.status === 'pending' ? (
                  <div className="notice info">{t('pending', { ref: s.txn_ref })}</div>
                ) : (
                  <button className="btn primary" onClick={() => setPaying(s)}>{t('uploadReceipt')}</button>
                )}
                <Link className="small" to={`/settlements/${s.settlement_id}`}>{t('viewBill')} →</Link>
              </div>
            ))}
            {owing.length === 0 && <div className="notice ok">{t('allPaid')}</div>}
          </div>
          {done.length > 0 && (
            <div className="card">
              <h3 style={{ marginBottom: 8 }}>{t('paid')}</h3>
              <div className="list">
                {done.map((s) => (
                  <Link key={s.id} to={`/settlements/${s.settlement_id}`} className="item">
                    <div className="grow">{s.title}<div className="muted tiny">{s.event_title}</div></div>
                    <span className="mono">{rm(s.amount_cents)}</span>
                    <StatusPill map={SHARE_STATUS} status={s.status} />
                  </Link>
                ))}
              </div>
            </div>
          )}
        </>
      )}
      {paying && <ProofModal share={paying} onClose={() => setPaying(null)} onDone={() => { setPaying(null); reload(); }} />}
    </div>
  );
}

function ProofModal({ share, onClose, onDone }: { share: Share; onClose: () => void; onDone: () => void }) {
  const { t } = useTranslation('costs');
  const [file, setFile] = useState<number | null>(null);
  const [txn, setTxn] = useState('');
  const [date, setDate] = useState(todayStr());
  const { busy, error, run } = useAction();
  return (
    <Modal title={t('receiptModal', { amount: rm(share.amount_cents) })} onClose={onClose}>
      <FileInput label={t('receiptLabel')} value={file} onChange={setFile} />
      <Field label={t('txnLabel')} hint={t('txnHint')}>
        <input className="input" required value={txn} onChange={(e) => setTxn(e.target.value)} />
      </Field>
      <Field label={t('paidDate')}>
        <input className="input" type="date" value={date} onChange={(e) => setDate(e.target.value)} />
      </Field>
      <p className="muted tiny">{t('privacy')}</p>
      <ErrorBox error={error} />
      <button className="btn primary" disabled={busy || !file || txn.length < 4}
        onClick={() => run(() => post(`/shares/${share.id}/proof`, { proof_file_id: file, txn_ref: txn, paid_date: date }).then(onDone), t('submitted'))}>
        {t('submit')}
      </button>
    </Modal>
  );
}
