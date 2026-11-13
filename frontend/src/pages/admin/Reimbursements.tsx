import { useState } from 'react';
import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
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
  ReceiptLink,
  StatusPill,
  Tabs,
  useAction,
} from '../../components/ui';
import { fmtDateTime, fmtDay, REIMB_STATUS, rm, useFormatT } from '../../format';
import type { Reimb } from '../Reimbursements';

export default function AdminReimbursements() {
  const { t } = useTranslation('admin');
  const { has, me } = useAuth();
  const canReview = has('transport') && me?.mfa_ok;
  const canPay = has('finance') && me?.mfa_ok;
  const [tab, setTab] = useState<'review' | 'payout'>(
    canReview ? 'review' : 'payout',
  );

  if (!canReview && !canPay) {
    return (
      <Page>
        <EmptyState title={t('reimb.noPermission')} />
      </Page>
    );
  }

  return (
    <Page>
      <PageHeader title={t('reimb.title')} subtitle={t('reimb.desc')} />
      {canReview && canPay && (
        <Tabs
          value={tab}
          onChange={(v) => setTab(v as 'review' | 'payout')}
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
  const { data, error, loading, reload } = useApi<Reimb[]>(
    `/reimbursements?scope=${scope}`,
  );
  const [open, setOpen] = useState<Reimb | null>(null);

  return (
    <div className="stack">
      {/* Clear section notices for role separation */}
      <div className="notice info">
        {scope === 'payout' ? t('reimb.financeNotice') : t('reimb.transportNotice')}
      </div>

      <ErrorBox error={error} />

      {loading ? (
        <CardSkeleton count={3} />
      ) : !data?.length ? (
        <EmptyState title={t('reimb.none')} />
      ) : (
        <Card>
          <div className="table-wrap">
            <table className="table responsive sticky-header">
              <thead>
                <tr>
                  <th>{t('reimb.applicant')}</th>
                  <th>{t('reimb.amount')}</th>
                  <th>{t('reimb.description')}</th>
                  <th>{t('reimb.submitted')}</th>
                  <th>{t('reimb.status')}</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {data.map((r) => (
                  <tr key={r.id}>
                    <td data-label={t('reimb.applicant')} className="bold">
                      {r.user.name}
                    </td>
                    <td data-label={t('reimb.amount')} className="mono bold">
                      {rm(r.amount_cents)}
                    </td>
                    <td data-label={t('reimb.description')} className="small">
                      {r.description}
                      {r.ride_id && (
                        <>
                          {' · '}
                          <Link to={`/admin/rides/${r.ride_id}`}>
                            {t('reimb.rideLink')}
                          </Link>
                        </>
                      )}
                    </td>
                    <td data-label={t('reimb.submitted')}>
                      {fmtDay(r.created_at, ft)}
                    </td>
                    <td data-label={t('reimb.status')}>
                      <StatusPill map={REIMB_STATUS} status={r.status} />
                    </td>
                    <td>
                      {(scope === 'review' && r.status === 'pending') ||
                      (scope === 'payout' && r.status === 'approved') ? (
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
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>
      )}

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
