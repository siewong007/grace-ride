import { useState } from 'react';
import { Link } from 'react-router-dom';
import { Copy, Upload } from 'lucide-react';
import { post, useApi } from '../api';
import {
  Button,
  Card,
  EmptyState,
  ErrorBox,
  Field,
  FileInput,
  Modal,
  Page,
  PageHeader,
  Spinner,
  StatusPill,
  useAction,
  useToast,
} from '../components/ui';
import { rm, SHARE_STATUS, todayStr } from '../format';
import { useTranslation } from 'react-i18next';

interface Share {
  id: number;
  settlement_id: number;
  title: string;
  event_title: string;
  amount_cents: number;
  status: string;
  payee_name: string;
  payee_account: string;
  due_date: string | null;
  note: string;
  txn_ref: string;
  proof_file_id: number | null;
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
    toast.success(t('copied'));
  };

  return (
    <Page width="medium">
      <div className="stack">
        <PageHeader title={t('title')} subtitle={t('desc')} />

        <ErrorBox error={error} />

        {loading ? (
          <Spinner />
        ) : !data?.length ? (
          <EmptyState title={t('none')} />
        ) : (
          <>
            <div className="grid cols-2">
              {owing.map((s) => (
                <Card key={s.id} className="stack">
                  <div className="row between nowrap">
                    <div className="min-w-0">
                      <div className="bold ellipsis">{s.title}</div>
                      <div className="muted small ellipsis">{s.event_title}</div>
                    </div>
                    <StatusPill map={SHARE_STATUS} status={s.status} />
                  </div>

                  <div className="costs-amount">{rm(s.amount_cents)}</div>

                  <div className="card flat tight stack-sm">
                    <div className="small">{t('payee', { name: s.payee_name })}</div>
                    <div className="row between nowrap">
                      <span className="mono bold">{s.payee_account}</span>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => copyAccount(s)}
                        icon={<Copy size={16} />}
                      >
                        {t('copy')}
                      </Button>
                    </div>
                    {s.due_date && <div className="small muted">{t('due', { date: s.due_date })}</div>}
                  </div>

                  {s.note && <div className="notice warning small">{s.note}</div>}

                  {s.status === 'pending' ? (
                    <div className="notice info small">{t('pending', { ref: s.txn_ref })}</div>
                  ) : (
                    <Button
                      variant="primary"
                      block
                      onClick={() => setPaying(s)}
                      icon={<Upload size={18} />}
                    >
                      {t('uploadReceipt')}
                    </Button>
                  )}

                  <Link className="small muted text-center" to={`/settlements/${s.settlement_id}`}>
                    {t('viewBill')}
                  </Link>
                </Card>
              ))}

              {owing.length === 0 && <div className="notice ok">{t('allPaid')}</div>}
            </div>

            {done.length > 0 && (
              <Card className="stack">
                <div className="card-title">
                  <h3>{t('paid')}</h3>
                </div>
                <div className="list">
                  {done.map((s) => (
                    <Link key={s.id} to={`/settlements/${s.settlement_id}`} className="item">
                      <div className="grow min-w-0">
                        <div className="bold ellipsis">{s.title}</div>
                        <div className="muted tiny ellipsis">{s.event_title}</div>
                      </div>
                      <span className="mono bold">{rm(s.amount_cents)}</span>
                      <StatusPill map={SHARE_STATUS} status={s.status} />
                    </Link>
                  ))}
                </div>
              </Card>
            )}
          </>
        )}

        {paying && (
          <ProofModal
            share={paying}
            onClose={() => setPaying(null)}
            onDone={() => {
              setPaying(null);
              reload();
            }}
          />
        )}
      </div>
    </Page>
  );
}

function ProofModal({
  share,
  onClose,
  onDone,
}: {
  share: Share;
  onClose: () => void;
  onDone: () => void;
}) {
  const { t } = useTranslation('costs');
  const [file, setFile] = useState<number | null>(null);
  const [txn, setTxn] = useState('');
  const [date, setDate] = useState(todayStr());
  const { busy, error, run } = useAction();

  const submit = () => {
    run(async () => {
      await post(`/me/shares/${share.id}/pay`, { proof_file_id: file, txn_ref: txn, paid_date: date });
      onDone();
    }, t('submitted'));
  };

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
      <Button variant="primary" block disabled={busy || !file || !txn} onClick={submit}>
        {t('submit')}
      </Button>
    </Modal>
  );
}
