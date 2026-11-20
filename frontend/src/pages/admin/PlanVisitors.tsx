// Visitors on the transport plan, for the record. The traffic lead notes people without
// an account who travel to the event; nothing is sent to them. A traffic lead may also ask
// for a special reimbursement for a visitor, paid to their driver via the usual review.
import { FormEvent, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { HandCoins, Pencil, Plus, Trash2, UserRoundPlus } from 'lucide-react';
import { del, post, put, useApi } from '../../api';
import { Button, Card, ErrorBox, Field, IconButton, Modal, Pill, StatusPill, useAction, useConfirm } from '../../components/ui';
import { REIMB_STATUS, rm, SPECIAL_HIGH_CENTS, toCents } from '../../format';
import type { PlanDriver, PlanVisitor } from './planLogic';

interface AreaOpt {
  id: number;
  name: string;
  active: boolean;
}

export function visitorTotals(visitors: PlanVisitor[]) {
  return { count: visitors.length, seats: visitors.reduce((n, v) => n + v.seats, 0) };
}

/** One visitor line, as shown in the list and inside a driver's car on the board. */
export function VisitorLine({ v, compact = false }: { v: PlanVisitor; compact?: boolean }) {
  const { t } = useTranslation('plan');
  return (
    <div className={`plan-visitor-line${compact ? ' is-compact' : ''}`}>
      <div className="row wrap gap-sm">
        <strong>{v.name}</strong>
        <Pill tone="info">{t('visitorTag')}</Pill>
        {v.leg === 'return' && <Pill tone="neutral">{t('returnTag')}</Pill>}
      </div>
      <div className="muted small">
        {[
          v.area?.name,
          t('passengers', { count: v.seats }),
          !compact && (v.driver_name ? t('visitorInCar', { name: v.driver_name }) : t('visitorNoCar')),
        ]
          .filter(Boolean)
          .join(' · ')}
      </div>
      {!compact && (v.phone || v.pickup_address) && (
        <div className="muted small">{[v.phone, v.pickup_address].filter(Boolean).join(' · ')}</div>
      )}
      {!compact && v.notes && <div className="small">{v.notes}</div>}
      {!compact && v.special_reimbursement && (
        <div className="plan-visitor-reimb small">
          <div className="row wrap gap-xs">
            <StatusPill map={REIMB_STATUS} status={v.special_reimbursement.status} />
            {v.special_reimbursement.amount_cents > SPECIAL_HIGH_CENTS && <Pill tone="amber">{t('visitorReimbHigh')}</Pill>}
            <span className="bold">
              {t('visitorReimbLine', {
                amount: rm(v.special_reimbursement.amount_cents),
                payee: v.special_reimbursement.payee_name ?? '—',
              })}
            </span>
          </div>
          <div className="muted">{v.special_reimbursement.reason}</div>
          {v.special_reimbursement.status === 'returned' && v.special_reimbursement.review_note && (
            <div className="text-warn">{t('visitorReimbReturnedNote', { note: v.special_reimbursement.review_note })}</div>
          )}
        </div>
      )}
    </div>
  );
}

export default function PlanVisitors({
  eventId,
  visitors,
  drivers,
  canUpdate,
  onChanged,
}: {
  eventId: number;
  visitors: PlanVisitor[];
  drivers: PlanDriver[];
  canUpdate: boolean;
  onChanged: () => void;
}) {
  const { t } = useTranslation('plan');
  const confirm = useConfirm();
  const { run } = useAction();
  const [editing, setEditing] = useState<PlanVisitor | 'new' | null>(null);
  const [claiming, setClaiming] = useState<PlanVisitor | null>(null);
  const { count, seats } = visitorTotals(visitors);

  const remove = async (v: PlanVisitor) => {
    if (!(await confirm({ title: t('visitorRemoveTitle', { name: v.name }), body: t('visitorRemoveBody') }))) return;
    run(() => del(`/admin/events/${eventId}/plan/visitors/${v.id}`).then(onChanged), t('visitorRemoved'));
  };

  return (
    <Card className="stack plan-visitors" aria-labelledby="plan-visitors-title">
      <div className="row between wrap gap-sm">
        <div>
          <h2 id="plan-visitors-title" className="plan-col-title">
            {t('visitorsTitle')} <span className="plan-count">{count}</span>
          </h2>
          <p className="muted small">{t('visitorsDesc')}</p>
          {!canUpdate && <p className="muted small">{t('visitorsLeadOnly')}</p>}
        </div>
        {canUpdate && (
          <Button variant="secondary" icon={<UserRoundPlus size={16} />} onClick={() => setEditing('new')}>
            {t('visitorAdd')}
          </Button>
        )}
      </div>
      {count === 0 ? (
        <p className="muted">{t('visitorsEmpty')}</p>
      ) : (
        <>
          <p className="small bold">{t('visitorSummary', { count, seats })}</p>
          <ul className="plan-visitor-list">
            {visitors.map((v) => (
              <li key={v.id} className="plan-visitor-item">
                <VisitorLine v={v} />
                {canUpdate && v.can_edit && (
                  <div className="row nowrap gap-xs">
                    <IconButton icon={<Pencil size={16} />} label={t('visitorEdit', { name: v.name })} onClick={() => setEditing(v)} />
                    <IconButton icon={<Trash2 size={16} />} label={t('visitorRemove', { name: v.name })} onClick={() => remove(v)} />
                  </div>
                )}
                {canUpdate && v.can_edit && (
                  <div className="plan-visitor-reimb-action">
                    <ReimbAction v={v} onOpen={() => setClaiming(v)} />
                  </div>
                )}
              </li>
            ))}
          </ul>
        </>
      )}
      {claiming && (
        <SpecialReimbForm
          eventId={eventId}
          visitor={claiming}
          onClose={() => setClaiming(null)}
          onSaved={() => {
            setClaiming(null);
            onChanged();
          }}
        />
      )}
      {editing && (
        <VisitorForm
          eventId={eventId}
          visitor={editing === 'new' ? null : editing}
          drivers={drivers}
          onClose={() => setEditing(null)}
          onSaved={() => {
            setEditing(null);
            onChanged();
          }}
        />
      )}
    </Card>
  );
}

function VisitorForm({
  eventId,
  visitor,
  drivers,
  onClose,
  onSaved,
}: {
  eventId: number;
  visitor: PlanVisitor | null;
  drivers: PlanDriver[];
  onClose: () => void;
  onSaved: () => void;
}) {
  const { t } = useTranslation('plan');
  const { data: areas } = useApi<AreaOpt[]>('/areas');
  const { busy, error, run } = useAction();
  const [name, setName] = useState(visitor?.name ?? '');
  const [phone, setPhone] = useState(visitor?.phone ?? '');
  const [areaId, setAreaId] = useState(visitor?.area ? String(visitor.area.id) : '');
  const [pickup, setPickup] = useState(visitor?.pickup_address ?? '');
  const [seats, setSeats] = useState(String(visitor?.seats ?? 1));
  const [leg, setLeg] = useState<'outbound' | 'return'>(visitor?.leg ?? 'outbound');
  const [driverId, setDriverId] = useState(visitor?.driver_id ? String(visitor.driver_id) : '');
  const [notes, setNotes] = useState(visitor?.notes ?? '');
  const nameMissing = name.trim() === '';
  const areaChoices = (Array.isArray(areas) ? areas : []).filter((a) => a.active || String(a.id) === areaId);

  const submit = (e: FormEvent) => {
    e.preventDefault();
    if (nameMissing) return;
    const body = {
      name: name.trim(),
      phone: phone.trim(),
      area_id: areaId ? Number(areaId) : null,
      pickup_address: pickup.trim(),
      seats: Number(seats),
      leg,
      driver_id: driverId ? Number(driverId) : null,
      notes: notes.trim(),
    };
    const base = `/admin/events/${eventId}/plan/visitors`;
    run(
      () => (visitor ? put(`${base}/${visitor.id}`, body) : post(base, body)).then(onSaved),
      visitor ? t('visitorSaved') : t('visitorAdded'),
    );
  };

  return (
    <Modal title={visitor ? t('visitorEditTitle') : t('visitorAddTitle')} description={t('visitorRecordOnly')} onClose={onClose}>
      <form className="stack" onSubmit={submit}>
        <Field label={t('visitorName')}>
          <input aria-label={t('visitorName')} className="input" value={name} maxLength={80} required onChange={(e) => setName(e.target.value)} />
        </Field>
        <div className="grid cols-2">
          <Field label={t('visitorPhone')} hint={t('optional')}>
            <input aria-label={t('visitorPhone')} className="input" type="tel" inputMode="tel" value={phone} maxLength={20} onChange={(e) => setPhone(e.target.value)} />
          </Field>
          <Field label={t('visitorSeats')}>
            <select aria-label={t('visitorSeats')} className="input" value={seats} onChange={(e) => setSeats(e.target.value)}>
              {[1, 2, 3, 4, 5, 6, 7, 8].map((n) => (
                <option key={n} value={n}>{n}</option>
              ))}
            </select>
          </Field>
        </div>
        <div className="grid cols-2">
          <Field label={t('visitorArea')} hint={t('optional')}>
            <select aria-label={t('visitorArea')} className="input" value={areaId} onChange={(e) => setAreaId(e.target.value)}>
              <option value="">—</option>
              {areaChoices.map((a) => (
                <option key={a.id} value={a.id}>{a.name}</option>
              ))}
            </select>
          </Field>
          <Field label={t('visitorLeg')}>
            <select aria-label={t('visitorLeg')} className="input" value={leg} onChange={(e) => setLeg(e.target.value as 'outbound' | 'return')}>
              <option value="outbound">{t('visitorLegOutbound')}</option>
              <option value="return">{t('visitorLegReturn')}</option>
            </select>
          </Field>
        </div>
        <Field label={t('visitorPickup')} hint={t('optional')}>
          <input aria-label={t('visitorPickup')} className="input" value={pickup} maxLength={500} onChange={(e) => setPickup(e.target.value)} />
        </Field>
        <Field label={t('visitorCar')} hint={t('visitorCarHint')}>
          <select aria-label={t('visitorCar')} className="input" value={driverId} onChange={(e) => setDriverId(e.target.value)}>
            <option value="">{t('visitorNoCar')}</option>
            {drivers.map((d) => (
              <option key={d.id} value={d.id}>{d.name}</option>
            ))}
          </select>
        </Field>
        <Field label={t('visitorNotes')} hint={t('optional')}>
          <textarea aria-label={t('visitorNotes')} className="input" rows={2} value={notes} maxLength={500} onChange={(e) => setNotes(e.target.value)} />
        </Field>
        <ErrorBox error={error} />
        <div className="btn-row">
          <Button type="submit" variant="primary" icon={visitor ? undefined : <Plus size={16} />} disabled={nameMissing || busy} loading={busy}>
            {visitor ? t('visitorSave') : t('visitorAdd')}
          </Button>
          <Button type="button" variant="ghost" onClick={onClose}>
            {t('cancel', { ns: 'common' })}
          </Button>
        </div>
      </form>
    </Modal>
  );
}

/** What a traffic lead can do about a visitor's special reimbursement right now. */
function ReimbAction({ v, onOpen }: { v: PlanVisitor; onOpen: () => void }) {
  const { t } = useTranslation('plan');
  const sr = v.special_reimbursement;
  if (sr?.status === 'returned') {
    return (
      <Button variant="secondary" size="sm" icon={<HandCoins size={16} />} onClick={onOpen} aria-label={t('visitorReimbResendFor', { name: v.name })}>
        {t('visitorReimbResend')}
      </Button>
    );
  }
  // Pending, approved or paid: one live request per visitor.
  if (sr && sr.status !== 'rejected') return null;
  if (!v.driver_id) return <p className="muted tiny">{t('visitorReimbNeedsCar')}</p>;
  return (
    <Button variant="ghost" size="sm" icon={<HandCoins size={16} />} onClick={onOpen} aria-label={t('visitorReimbFor', { name: v.name })}>
      {t('visitorReimb')}
    </Button>
  );
}

function SpecialReimbForm({
  eventId,
  visitor,
  onClose,
  onSaved,
}: {
  eventId: number;
  visitor: PlanVisitor;
  onClose: () => void;
  onSaved: () => void;
}) {
  const { t } = useTranslation('plan');
  const { busy, error, run } = useAction();
  const resend = visitor.special_reimbursement?.status === 'returned' ? visitor.special_reimbursement : null;
  const [amount, setAmount] = useState(resend ? (resend.amount_cents / 100).toFixed(2) : '');
  const [reason, setReason] = useState(resend?.reason ?? '');
  const cents = toCents(amount);
  // Any amount above RM 0; there is no maximum. Over RM 100 only warns.
  const amountBad = amount.trim() !== '' && (!Number.isFinite(cents) || cents <= 0);
  const high = Number.isFinite(cents) && cents > SPECIAL_HIGH_CENTS;
  const ready = Number.isFinite(cents) && cents > 0 && reason.trim() !== '';
  const driver = (resend?.payee_name ?? visitor.driver_name) || '—';

  const submit = (e: FormEvent) => {
    e.preventDefault();
    if (!ready) return;
    const base = `/admin/events/${eventId}/plan/visitors/${visitor.id}/reimbursement`;
    const body = { amount_cents: cents, reason: reason.trim() };
    run(() => (resend ? put(`${base}/${resend.id}`, body) : post(base, body)).then(onSaved), t('visitorReimbSent'));
  };

  return (
    <Modal
      title={resend ? t('visitorReimbResendFor', { name: visitor.name }) : t('visitorReimbFor', { name: visitor.name })}
      description={t('visitorReimbDesc', { driver })}
      onClose={onClose}
    >
      <form className="stack" onSubmit={submit}>
        {resend?.review_note && <p className="small text-warn">{t('visitorReimbReturnedNote', { note: resend.review_note })}</p>}
        <Field label={t('visitorReimbAmount')} hint={amountBad ? t('visitorReimbAmountInvalid') : undefined}>
          <input
            aria-label={t('visitorReimbAmount')}
            className="input"
            inputMode="decimal"
            placeholder="0.00"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            aria-invalid={amountBad || undefined}
          />
        </Field>
        {high && (
          <div className="notice warning small" role="status">
            {t('visitorReimbHighWarn', { amount: rm(cents) })}
          </div>
        )}
        <Field label={t('visitorReimbReason')} hint={t('visitorReimbReasonHint')}>
          <textarea
            aria-label={t('visitorReimbReason')}
            className="input"
            rows={3}
            maxLength={300}
            required
            value={reason}
            onChange={(e) => setReason(e.target.value)}
          />
        </Field>
        <ErrorBox error={error} />
        <div className="btn-row">
          <Button type="submit" variant="primary" disabled={!ready || busy} loading={busy}>
            {t('visitorReimbSubmit')}
          </Button>
          <Button type="button" variant="ghost" onClick={onClose}>
            {t('cancel', { ns: 'common' })}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
