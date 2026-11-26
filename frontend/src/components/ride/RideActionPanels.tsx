import { useEffect, useState, type ReactNode } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { AlertCircle, AlertTriangle, Check, CheckCircle2, Clock, FileEdit, Lock, MoreHorizontal } from 'lucide-react';
import type { TFunction } from 'i18next';
import { Trans, useTranslation } from 'react-i18next';
import { api, post, useApi } from '../../api';
import { useAuth } from '../../auth';
import { canApprovePlan } from '../../perms';
import CantMakeItModal from '../CantMakeIt';
import type { Ride } from '../RideCard';
import { ReasonModal, StartTripButton } from '../TripActions';
import { parseLocal } from '../../format';
import { Button, ErrorBox, Field, Modal, Notice, Pill, useAction, useConfirm, useToast, type PillTone } from '../ui';
import { getQueuedActionsForRide, postOrQueue, useOfflineQueueTick } from '../../offlineQueue';

/** `part` splits the card when the passenger has something to do now: "respond" is the
 *  answer/readiness block shown near the top, "cancel" the quieter cancel control below. */
export function PassengerActions({
  ride: r,
  reload,
  part = 'all',
}: {
  ride: Ride;
  reload: () => void;
  part?: 'all' | 'respond' | 'cancel';
}) {
  const { t } = useTranslation('rideDetail');
  const auth = useAuth();
  const meId = auth?.me?.id;
  const can = auth?.can ?? (() => true);
  const confirm = useConfirm();
  const toast = useToast();
  const queueTick = useOfflineQueueTick();
  const { busy, error, run } = useAction();
  const nav = useNavigate();
  const [cancel, setCancel] = useState(false);
  const [unable, setUnable] = useState(false);

  const postReadiness = (status: 'ready' | 'delayed_5m', label: string, successMsg: string) =>
    run(async () => {
      if (!meId) {
        await post(`/rides/${r.id}/readiness`, { status });
        toast.success(successMsg);
        reload();
        return;
      }
      const res = await postOrQueue({
        userId: meId,
        rideId: r.id,
        path: `/rides/${r.id}/readiness`,
        body: { status },
        label,
      });
      if (res.queued) {
        toast.info(t('common:queuedOffline'));
      } else {
        toast.success(successMsg);
        reload();
      }
    });
  const cancellable = ['pending_driver', 'pending_passenger', 'pending_approval', 'pending_driver_confirm', 'confirmed'].includes(r.status);

  const handleDecline = async () => {
    const ok = await confirm({
      title: t('confirmDeclineTitle'),
      body: t('confirmDeclineBody'),
      confirmLabel: t('confirmDeclineBtn'),
      tone: 'danger',
    });
    if (ok) {
      await run(() => post(`/rides/${r.id}/respond`, { accept: false }).then(reload), t('respondedDeclined'));
    }
  };

  const cancelControls = (
    <>
      {cancellable && can('rides', 'delete') && (
        <Button variant="ghost" className="quiet-danger" onClick={() => setCancel(true)}>
          {t('cancelRide')}
        </Button>
      )}
      {cancel && (
        <ReasonModal
          error={error}
          title={t('cancelTitle')}
          label={t('cancelReason')}
          confirm={t('confirmCancel')}
          onClose={() => setCancel(false)}
          onSubmit={(reason) => run(() => post(`/rides/${r.id}/cancel`, { reason }).then(() => nav('/rides')), t('rideCancelled'))}
        />
      )}
    </>
  );

  if (part === 'cancel') {
    if (!cancellable || !can('rides', 'delete')) return null;
    return <div className="ride-cancel-row">{cancelControls}</div>;
  }

  const hasReadiness =
    (r.status === 'confirmed' || (r.status === 'in_progress' && !r.picked_up_at)) && can('rides', 'update');
  const hasSupper =
    r.leg === 'return' &&
    !['completed', 'cancelled', 'no_show', 'alt_transport'].includes(r.status) &&
    can('rides', 'update');
  const hasContent =
    (r.status === 'pending_passenger' && !!r.driver && can('rides', 'update')) ||
    r.status === 'pending_driver' ||
    r.status === 'pending_approval' ||
    r.status === 'alt_transport' ||
    hasReadiness ||
    hasSupper ||
    (part === 'all' && cancellable && can('rides', 'delete'));

  if (!hasContent) return null;

  return (
    <section className="focus-surface stack">
      {r.status === 'pending_passenger' && r.driver && can('rides', 'update') && (
        <div className="stack">
          <h2>{t('driverInvite')}</h2>
          <p>{t('driverInviteDesc', { name: r.driver.name })}</p>
          <div className="btn-row">
            <Button
              variant="primary"
              disabled={busy}
              onClick={() => run(() => post(`/rides/${r.id}/respond`, { accept: true }).then(reload), t('respondedAccepted'))}
            >
              {t('acceptInvite')}
            </Button>
            <Button
              variant="ghost"
              disabled={busy}
              onClick={handleDecline}
            >
              {t('declineInvite')}
            </Button>
          </div>
        </div>
      )}
      {r.status === 'pending_driver' && <Notice tone="info">{t('submittedWaiting')}</Notice>}
      {r.status === 'pending_approval' && <Notice tone="info">{t('awaitingApproval')}</Notice>}
      {r.status === 'alt_transport' && (
        <Notice tone="info">
          <Trans t={t as TFunction} i18nKey="altTransportNotice" components={{ receipt: <Link to={`/reimbursements/new?ride=${r.id}`} /> }} />
        </Notice>
      )}

      {/* Passenger Readiness & Delay Notification Block */}
      {(r.status === 'confirmed' || (r.status === 'in_progress' && !r.picked_up_at)) && can('rides', 'update') && (
        <div className="stack-sm">
          {r.delay_minutes && r.delay_minutes > 0 ? (
            <Notice tone="warning" icon={<Clock size={18} />}>
              <span>{t('driverDelayNotice', { mins: r.delay_minutes })}</span>
            </Notice>
          ) : null}

          {r.arrived_at_gate_at ? (
            <Notice tone="success" icon={<AlertCircle size={18} />}>
              <span className="bold">
                {r.is_proxy && r.proxy
                  ? t('driverArrivedGateNoticeProxy', { name: r.proxy.rider_name })
                  : t('driverArrivedGateNotice')}
              </span>
            </Notice>
          ) : null}

          <div className="stack-sm">
            <h3 className="no-margin">
              {r.is_proxy && r.proxy ? t('readinessTitleProxy') : t('readinessTitle')}
            </h3>
            <p className="small muted no-margin">
              {r.is_proxy && r.proxy ? t('readinessPromptProxy') : t('readinessPrompt')}
            </p>

            {r.readiness_status === 'ready' && (
              <Notice tone="success">{t('readyConfirmed')}</Notice>
            )}
            {(r.readiness_status === 'delayed_5m' || r.readiness_status === 'delayed_10m') && (
              <Notice tone="warning">{t('delayedConfirmed')}</Notice>
            )}
            {meId && queueTick >= 0 && getQueuedActionsForRide(meId, r.id).length > 0 && (
              <div>
                <Pill tone="info">{t('common:offlineQueuedPill')}</Pill>
              </div>
            )}

            <div className="btn-row">
              <Button
                variant={r.readiness_status === 'ready' ? 'secondary' : 'primary'}
                size="sm"
                disabled={busy}
                onClick={() =>
                  void postReadiness(
                    'ready',
                    r.is_proxy && r.proxy
                      ? t('readyBtnProxy', { name: r.proxy.rider_name })
                      : t('readyBtn'),
                    t('readyConfirmed'),
                  )
                }
              >
                {r.is_proxy && r.proxy
                  ? t('readyBtnProxy', { name: r.proxy.rider_name })
                  : t('readyBtn')}
              </Button>
              <Button
                variant="secondary"
                size="sm"
                disabled={busy}
                onClick={() =>
                  void postReadiness(
                    'delayed_5m',
                    r.is_proxy && r.proxy
                      ? t('delayed5mBtnProxy', { name: r.proxy.rider_name })
                      : t('delayed5mBtn'),
                    t('delayedConfirmed'),
                  )
                }
              >
                {r.is_proxy && r.proxy
                  ? t('delayed5mBtnProxy', { name: r.proxy.rider_name })
                  : t('delayed5mBtn')}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                className="text-danger"
                disabled={busy}
                onClick={() => setUnable(true)}
              >
                {r.is_proxy && r.proxy
                  ? t('unableToJoinBtnProxy', { name: r.proxy.rider_name })
                  : t('unableToJoinBtn')}
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Return trip supper preference */}
      {hasSupper && (
        <div className="detail-section stack-sm">
          <div className="bold small">
            {t('supperChoiceTitle')}
          </div>
          <p className="tiny muted no-margin">
            {t('supperChoicePrompt')}
          </p>
          <div className="btn-row">
            <Button
              variant={r.supper_choice === 'supper' ? 'primary' : 'secondary'}
              size="sm"
              disabled={busy}
              onClick={() => run(() => post(`/rides/${r.id}/supper-choice`, { choice: 'supper' }).then(reload), t('supperChoiceConfirmed'))}
            >
              {t('supperChoiceJoin')}
            </Button>
            <Button
              variant={r.supper_choice === 'direct_home' ? 'primary' : 'secondary'}
              size="sm"
              disabled={busy}
              onClick={() => run(() => post(`/rides/${r.id}/supper-choice`, { choice: 'direct_home' }).then(reload), t('supperChoiceHomeConfirmed'))}
            >
              {t('supperChoiceDirectHome')}
            </Button>
          </div>
          {r.supper_choice === 'supper' && (
            <span className="tiny text-success row items-center gap-xs">
              <Check size={14} aria-hidden="true" />
              <span>{t('supperChoiceJoinNote')}</span>
            </span>
          )}
          {r.supper_choice === 'direct_home' && (
            <span className="tiny muted row items-center gap-xs">
              <Check size={14} aria-hidden="true" />
              <span>{t('supperChoiceDirectHomeNote')}</span>
            </span>
          )}
        </div>
      )}
      <ErrorBox error={error} />
      {part === 'all' && cancelControls}
      {unable && (
        <CantMakeItModal
          rideId={r.id}
          riderName={r.is_proxy && r.proxy ? r.proxy.rider_name : undefined}
          onClose={() => setUnable(false)}
          onDone={reload}
        />
      )}
    </section>
  );
}

export function DriverActions({ ride: r, reload }: { ride: Ride; reload: () => void }) {
  const { t } = useTranslation('rideDetail');
  const auth = useAuth();
  const meId = auth?.me?.id;
  const confirm = useConfirm();
  const toast = useToast();
  const queueTick = useOfflineQueueTick();
  const { busy, error, run } = useAction();
  const [withdraw, setWithdraw] = useState(false);
  const act = (path: string, msg: string) => run(() => post(`/rides/${r.id}/${path}`).then(reload), msg);
  const actQueueable = (path: string, label: string, successMsg: string, body?: unknown) =>
    run(async () => {
      if (!meId) {
        if (body !== undefined) {
          await post(`/rides/${r.id}/${path}`, body);
        } else {
          await post(`/rides/${r.id}/${path}`);
        }
        toast.success(successMsg);
        reload();
        return;
      }
      const res = await postOrQueue({
        userId: meId,
        rideId: r.id,
        path: `/rides/${r.id}/${path}`,
        body,
        label,
      });
      if (res.queued) {
        toast.info(t('common:queuedOffline'));
      } else {
        toast.success(successMsg);
        reload();
      }
    });

  const handleNoShow = async () => {
    const ok = await confirm({
      title: t('confirmNoShowTitle'),
      body: t('confirmNoShowBody'),
      confirmLabel: t('confirmNoShowBtn'),
      tone: 'danger',
    });
    if (ok) {
      await act('no-show', t('respondedNoShow'));
    }
  };

  const hasSupperNotice =
    r.leg === 'return' && (r.supper_choice === 'supper' || r.supper_choice === 'direct_home');
  const hasReadinessNotice =
    r.readiness_status === 'ready' ||
    r.readiness_status === 'delayed_5m' ||
    r.readiness_status === 'delayed_10m';
  const hasStatusContent =
    ['pending_driver', 'pending_passenger', 'pending_approval', 'pending_driver_confirm', 'confirmed', 'in_progress'].includes(r.status) ||
    (r.status === 'completed' && !!r.mileage_receipt_needed);
  const canWithdraw =
    ['pending_passenger', 'pending_approval', 'pending_driver_confirm', 'confirmed'].includes(r.status) &&
    r.viewer.is_driver;

  if (!hasSupperNotice && !hasReadinessNotice && !hasStatusContent && !canWithdraw) {
    return null;
  }

  const delayButtons = (
    <div className="btn-row">
      <Button
        variant="secondary"
        size="sm"
        disabled={busy}
        onClick={() => void actQueueable('driver-delay', t('driverDelay5m'), t('driverDelaySent', { mins: 5 }), { delay_minutes: 5 })}
      >
        {t('driverDelay5m')}
      </Button>
      <Button
        variant="secondary"
        size="sm"
        disabled={busy}
        onClick={() => void actQueueable('driver-delay', t('driverDelay10m'), t('driverDelaySent', { mins: 10 }), { delay_minutes: 10 })}
      >
        {t('driverDelay10m')}
      </Button>
      <Button
        variant="secondary"
        size="sm"
        disabled={busy}
        onClick={() => void actQueueable('driver-delay', t('driverDelay15m'), t('driverDelaySent', { mins: 15 }), { delay_minutes: 15 })}
      >
        {t('driverDelay15m')}
      </Button>
    </div>
  );

  return (
    <section className="focus-surface stack">
      <div className="row between items-center wrap gap-xs">
        <h2>{t('driverActions')}</h2>
        {meId && queueTick >= 0 && getQueuedActionsForRide(meId, r.id).length > 0 && (
          <Pill tone="info">{t('common:offlineQueuedPill')}</Pill>
        )}
      </div>

      {/* Passenger readiness status notice for driver */}
      {r.leg === 'return' && r.supper_choice === 'supper' && (
        <Notice tone="info">{t('driverSupperNotice')}</Notice>
      )}
      {r.leg === 'return' && r.supper_choice === 'direct_home' && (
        <Notice tone="info">{t('driverDirectHomeNotice')}</Notice>
      )}
      {r.readiness_status === 'ready' && (
        <Notice tone="success">{t('readyConfirmed')}</Notice>
      )}
      {(r.readiness_status === 'delayed_5m' || r.readiness_status === 'delayed_10m') && (
        <Notice tone="warning">{t('delayedConfirmed')}</Notice>
      )}

      {r.status === 'completed' && r.mileage_receipt_needed && (
        <Notice tone="warning">
          <Trans t={t as TFunction} i18nKey="mileageReceiptNeeded" components={{ receipt: <Link to={`/reimbursements/new?kind=mileage&ride=${r.id}`} /> }} />
        </Notice>
      )}

      {r.status === 'pending_driver' && (
        <>
          <p className="muted">{t('driverClaimDesc')}</p>
          <Button variant="primary" disabled={busy} onClick={() => act('claim', t('respondedAccepted'))}>
            {t('claimRide')}
          </Button>
        </>
      )}
      {r.status === 'pending_passenger' && <p className="muted">{t('waitingPassenger')}</p>}
      {r.status === 'pending_approval' && <p className="muted">{t('passengerAccepted')}</p>}
      {r.status === 'pending_driver_confirm' && (
        <>
          <p>{t('assignYou')}</p>
          <Button variant="primary" disabled={busy} onClick={() => act('confirm-assignment', t('respondedConfirmed'))}>
            {t('confirmAssignment')}
          </Button>
        </>
      )}
      {r.status === 'confirmed' && (
        <>
          <StartTripButton ride={r} onStarted={reload} label={t('startTrip')} />
          {r.viewer.is_driver && (
            <details className="quiet-disclosure">
              <summary>{t('runningLatePrompt')}</summary>
              <div className="quiet-disclosure-body">{delayButtons}</div>
            </details>
          )}
        </>
      )}
      {r.status === 'in_progress' && !r.picked_up_at && (
        <>
          <div className="btn-row">
            <Button variant="primary" disabled={busy} onClick={() => void actQueueable('pickup', t('pickedUp'), t('respondedPickedUp'))}>
              {t('pickedUp')}
            </Button>
            <Button
              variant="secondary"
              disabled={busy}
              onClick={() => void actQueueable('arrived-gate', t('arrivedGateBtn'), t('arrivedGateSent'))}
            >
              {t('arrivedGateBtn')}
            </Button>
          </div>
          <details className="quiet-disclosure">
            <summary>{t('delayOrIssue')}</summary>
            <div className="quiet-disclosure-body stack-sm">
              {r.viewer.is_driver && delayButtons}
              <div className="btn-row">
                {/* The backend refuses a no-show before the pickup time has passed. */}
                {parseLocal(r.pickup_at).getTime() <= Date.now() && (
                  <Button variant="ghost" className="quiet-danger" size="sm" disabled={busy} onClick={handleNoShow}>
                    {t('noShowBtn')}
                  </Button>
                )}
                <Button variant="ghost" size="sm" disabled={busy} onClick={() => void actQueueable('complete', t('completedTrip'), t('respondedCompleted'))}>
                  {t('completedTrip')}
                </Button>
              </div>
            </div>
          </details>
        </>
      )}
      {r.status === 'in_progress' && r.picked_up_at && (
        <Button variant="primary" disabled={busy} onClick={() => void actQueueable('complete', t('completedTrip'), t('respondedCompleted'))}>
          {t('completedTrip')}
        </Button>
      )}
      {canWithdraw && (
        <Button variant="ghost" className="quiet-danger" onClick={() => setWithdraw(true)}>
          {t('withdrawDriver')}
        </Button>
      )}
      <ErrorBox error={error} />
      {withdraw && (
        <ReasonModal
          error={error}
          title={t('withdrawTitle')}
          label={t('withdrawLabel')}
          confirm={t('withdrawConfirm')}
          onClose={() => setWithdraw(false)}
          onSubmit={(reason) => run(() => post(`/rides/${r.id}/withdraw`, { reason }).then(reload), t('respondedWithdrew'))}
        />
      )}
    </section>
  );
}

interface DriverOpt {
  id: number;
  name: string;
  areas: string;
  carpool_enabled: boolean;
  spare_seats: number;
  vehicle: { model: string; plate: string } | null;
}

type RideDraftAction = 'approve' | 'reassign' | 'return' | 'alt_transport' | 'reimbursement_excluded';

interface RideDraftItem {
  id: number;
  ride_id: number;
  version: number;
  action: RideDraftAction;
  driver_id: number | null;
  driver_name: string | null;
  reason: string;
  reimbursement_excluded: boolean | null;
  status: 'draft' | 'pending_approval' | 'approved' | 'returned' | 'superseded';
  created_by: { id: number; name: string };
  updated_by: { id: number; name: string };
  submitted_by?: { id: number; name: string } | null;
  review_notes: string;
  locked_by?: { id: number; name: string } | null;
  lock_is_mine?: boolean;
}

function rideDraftTone(status: string): PillTone {
  if (status === 'pending_approval') return 'warning';
  if (status === 'returned') return 'danger';
  if (status === 'approved') return 'success';
  if (status === 'superseded') return 'neutral';
  return 'info';
}

export function StaffActions({ ride: r, reload }: { ride: Ride; reload: () => void }) {
  const { t } = useTranslation('rideDetail');
  const auth = useAuth();
  const can = auth?.can ?? (() => true);
  // Staff who may only view rides get the summary without the buttons.
  const edit = can('admin_rides', 'update');
  const cancel = can('admin_rides', 'delete');
  const isLead = canApprovePlan(auth?.me?.roles);
  const canProposeDraft = can('admin_plan', 'update') || edit;
  const confirm = useConfirm();
  const { busy, error, run } = useAction();
  const [modal, setModal] = useState<'reassign' | 'return' | 'alt' | 'cancel' | null>(null);
  const [proposeOpen, setProposeOpen] = useState(false);
  const [editingDraft, setEditingDraft] = useState<RideDraftItem | null>(null);
  const [draftAction, setDraftAction] = useState<RideDraftAction>('reassign');
  const [draftDriverId, setDraftDriverId] = useState('');
  const [draftReimbExcl, setDraftReimbExcl] = useState(true);
  const [draftReason, setDraftReason] = useState('');
  const [returningDraft, setReturningDraft] = useState<RideDraftItem | null>(null);
  const [returnNotes, setReturnNotes] = useState('');

  const drivers = useApi<DriverOpt[]>(modal === 'reassign' || proposeOpen ? '/admin/drivers' : null);
  const rideDrafts = useApi<RideDraftItem[]>(r.viewer.is_staff ? `/rides/${r.id}/drafts` : null);
  const draftList = Array.isArray(rideDrafts.data) ? rideDrafts.data : [];

  const [driverId, setDriverId] = useState('');
  const open = !['completed', 'cancelled', 'no_show', 'alt_transport', 'in_progress'].includes(r.status);
  // After an approval: the next ride still waiting (null when the queue is empty), so staff can keep going.
  const [nextApproval, setNextApproval] = useState<number | null | undefined>(undefined);
  useEffect(() => setNextApproval(undefined), [r.id]);
  const approve = () =>
    run(async () => {
      await post(`/rides/${r.id}/approve`);
      const waiting = await api<Ride[]>('/admin/rides?status=pending_approval').catch(() => null);
      if (waiting) setNextApproval(waiting.find((x) => x.id !== r.id)?.id ?? null);
      reload();
    }, t('respondedAccepted'));

  const handleStaffNoShow = async () => {
    const ok = await confirm({
      title: t('confirmNoShowTitle'),
      body: t('confirmNoShowBody'),
      confirmLabel: t('confirmNoShowBtn'),
      tone: 'danger',
    });
    if (ok) {
      await run(() => post(`/rides/${r.id}/no-show`).then(reload), t('respondedNoShow'));
    }
  };

  const toggleReimbExcluded = () =>
    run(
      () =>
        api(`/rides/${r.id}/reimbursement-excluded`, {
          method: 'PUT',
          body: { excluded: !r.reimbursement_excluded },
        }).then(reload),
      t('reimbFlagUpdated'),
    );

  const openNewDraftModal = () => {
    setEditingDraft(null);
    setDraftAction(r.status === 'pending_approval' ? 'approve' : 'reassign');
    setDraftDriverId('');
    setDraftReimbExcl(!r.reimbursement_excluded);
    setDraftReason('');
    setProposeOpen(true);
  };

  const openEditDraftModal = async (d: RideDraftItem) => {
    if (!d.lock_is_mine) {
      const ok = await run(() => post(`/rides/${r.id}/drafts/${d.id}/lock`, {}).then(() => rideDrafts.reload()));
      if (!ok) return;
    }
    setEditingDraft(d);
    setDraftAction(d.action);
    setDraftDriverId(d.driver_id != null ? String(d.driver_id) : '');
    setDraftReimbExcl(d.reimbursement_excluded ?? !r.reimbursement_excluded);
    setDraftReason(d.reason ?? '');
    setProposeOpen(true);
  };

  const submitDraftModal = async (andSubmit: boolean) => {
    const payload = {
      action: draftAction,
      driver_id: draftAction === 'reassign' && draftDriverId ? Number(draftDriverId) : null,
      reason: draftReason.trim(),
      reimbursement_excluded: draftAction === 'reimbursement_excluded' ? draftReimbExcl : null,
      submit: andSubmit,
    };
    const res = await run(async () => {
      if (editingDraft) {
        await api(`/rides/${r.id}/drafts/${editingDraft.id}`, {
          method: 'PUT',
          body: payload,
        });
        if (andSubmit) {
          await post(`/rides/${r.id}/drafts/${editingDraft.id}/request-approval`, {});
        }
      } else {
        await post(`/rides/${r.id}/drafts`, payload);
      }
      rideDrafts.reload();
    }, andSubmit ? t('proposeSubmitted') : editingDraft ? t('proposeUpdated') : t('proposeCreated'));
    if (res) {
      setProposeOpen(false);
      setEditingDraft(null);
    }
  };

  const draftStatusText = (status: RideDraftItem['status']) => {
    if (status === 'pending_approval') return t('draftStatusPending');
    if (status === 'returned') return t('draftStatusReturned');
    if (status === 'approved') return t('draftStatusApproved');
    if (status === 'superseded') return t('draftStatusSuperseded');
    return t('draftStatusDraft');
  };

  const draftActionText = (d: RideDraftItem) => {
    if (d.action === 'approve') return t('actionApprove');
    if (d.action === 'reassign') return `${t('actionReassign')}: ${d.driver_name ?? '—'}`;
    if (d.action === 'return') return t('actionReturn');
    if (d.action === 'alt_transport') return t('actionAltTransport');
    return d.reimbursement_excluded ? t('actionReimbExclude') : t('actionReimbInclude');
  };

  return (
    <section className="detail-section stack">
      <h2>{t('staffActions')}</h2>
      {r.driver_load && (
        <p className="small">{t('driverLoad', { used: r.driver_load.seats_used, capacity: r.driver_load.capacity, seats: r.seats })}</p>
      )}
      {r.reimbursement_excluded && (
        <Notice tone="warning" icon={<AlertTriangle size={16} />}>
          <span>{t('noReimbDesc')}</span>
        </Notice>
      )}
      {!!r.needs_attention && r.attention_reason && (
        <Notice tone="warning" icon={<AlertTriangle size={16} />}>
          <span>{r.attention_reason}</span>
        </Notice>
      )}
      {r.assistance && <Notice tone="info">{t('assistanceLabel', { assistance: r.assistance })}</Notice>}
      {nextApproval !== undefined && (
        <Notice tone="success" icon={<CheckCircle2 size={16} />}>
          {nextApproval ? (
            <Link to={`/admin/rides/${nextApproval}`}>{t('nextApproval')}</Link>
          ) : (
            <span>
              {t('noMoreApprovals')} <Link to="/admin">{t('backToQueue')}</Link>
            </span>
          )}
        </Notice>
      )}
      <div className="btn-row">
        {edit && r.status === 'pending_approval' && (
          <Button variant="primary" disabled={busy} onClick={approve}>
            {t('approve')}
          </Button>
        )}
        {edit && open && <Button variant="secondary" onClick={() => setModal('reassign')}>{t('reassign')}</Button>}
        {edit && ['pending_approval', 'pending_passenger', 'pending_driver_confirm'].includes(r.status) && (
          <Button variant="secondary" onClick={() => setModal('return')}>{t('retur')}</Button>
        )}
        {edit && open && <Button variant="secondary" onClick={() => setModal('alt')}>{t('altTransport')}</Button>}
        {(edit || isLead) && (
          <Button variant="secondary" disabled={busy} onClick={toggleReimbExcluded}>
            {r.reimbursement_excluded ? t('restoreReimb') : t('markNoReimb')}
          </Button>
        )}
        {canProposeDraft && (
          <Button variant="secondary" icon={<FileEdit size={16} />} disabled={busy} onClick={openNewDraftModal}>
            {t('proposeChangeBtn')}
          </Button>
        )}
        {cancel && open && <Button variant="danger" onClick={() => setModal('cancel')}>{t('cancel')}</Button>}
        {edit && r.status === 'in_progress' && !r.picked_up_at && (
          <Button variant="danger" disabled={busy} onClick={handleStaffNoShow}>{t('noShow')}</Button>
        )}
      </div>

      {draftList.length > 0 && (
        <div className="stack-xs">
          <h3 className="no-margin small muted">{t('proposedChangesTitle')}</h3>
          {draftList.map((d) => {
            const lockedByOther = Boolean(d.locked_by && !d.lock_is_mine);
            const isDraftEditable = (d.status === 'draft' || d.status === 'returned') && !lockedByOther;
            const canDelete = (d.created_by.id === auth?.me?.id || isLead) && d.status !== 'approved' && !lockedByOther;
            return (
              <div key={d.id} className="card flat tight stack-xs">
                <div className="row between wrap gap-xs">
                  <div className="row wrap items-center gap-xs">
                    <strong>v{d.version}</strong>
                    <Pill tone="info">{draftActionText(d)}</Pill>
                    <Pill tone={rideDraftTone(d.status)}>{draftStatusText(d.status)}</Pill>
                    {d.locked_by && (
                      <Pill tone={d.lock_is_mine ? 'info' : 'warning'} icon={<Lock size={12} />}>
                        {t('proposeLockedBy', { name: d.locked_by.name })}
                      </Pill>
                    )}
                  </div>
                  <span className="tiny muted">{d.submitted_by?.name ?? d.created_by.name}</span>
                </div>
                {d.reason && <div className="small">{d.reason}</div>}
                {d.status === 'returned' && d.review_notes && (
                  <div className="small text-danger">{d.review_notes}</div>
                )}
                <div className="btn-row">
                  {lockedByOther && isLead && d.status !== 'approved' && d.status !== 'superseded' && (
                    <Button
                      size="sm"
                      variant="secondary"
                      disabled={busy}
                      onClick={() => run(() => post(`/rides/${r.id}/drafts/${d.id}/lock`, { force: true }).then(() => rideDrafts.reload()))}
                    >
                      {t('proposeTakeOverBtn')}
                    </Button>
                  )}
                  {d.lock_is_mine && (
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={busy}
                      onClick={() => run(() => api(`/rides/${r.id}/drafts/${d.id}/lock`, { method: 'DELETE' }).then(() => rideDrafts.reload()))}
                    >
                      {t('proposeReleaseLockBtn')}
                    </Button>
                  )}
                  {canProposeDraft && isDraftEditable && (
                    <>
                      <Button size="sm" variant="secondary" disabled={busy} onClick={() => openEditDraftModal(d)}>
                        {t('proposeEditBtn')}
                      </Button>
                      <Button
                        size="sm"
                        variant="primary"
                        disabled={busy}
                        onClick={() =>
                          run(
                            () => post(`/rides/${r.id}/drafts/${d.id}/request-approval`, {}).then(() => rideDrafts.reload()),
                            t('proposeSubmitted'),
                          )
                        }
                      >
                        {t('proposeSubmitLead')}
                      </Button>
                    </>
                  )}
                  {canDelete && (
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={busy}
                      onClick={() =>
                        run(
                          () => api(`/rides/${r.id}/drafts/${d.id}`, { method: 'DELETE' }).then(() => rideDrafts.reload()),
                          t('proposeDeleted'),
                        )
                      }
                    >
                      {t('proposeDeleteBtn')}
                    </Button>
                  )}
                  {isLead && d.status === 'pending_approval' && (
                    <>
                      <Button
                        size="sm"
                        variant="danger"
                        disabled={busy}
                        onClick={() => {
                          setReturningDraft(d);
                          setReturnNotes('');
                        }}
                      >
                        {t('proposeReturn')}
                      </Button>
                      <Button
                        size="sm"
                        variant="primary"
                        disabled={busy}
                        onClick={() =>
                          run(async () => {
                            await post(`/rides/${r.id}/drafts/${d.id}/approve`, {});
                            rideDrafts.reload();
                            reload();
                          }, t('proposeApproved'))
                        }
                      >
                        {t('proposeApprove')}
                      </Button>
                    </>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}

      <Link className="small" to={`/admin/audit?entity=ride&entity_id=${r.id}`}>{t('auditLink')}</Link>
      <ErrorBox error={error} />
      {proposeOpen && (
        <Modal
          title={t('proposeModalTitle')}
          onClose={() => {
            setProposeOpen(false);
            setEditingDraft(null);
          }}
        >
          <div className="stack">
            <Field label={t('proposeActionLabel')}>
              <select
                className="input"
                aria-label={t('proposeActionLabel')}
                value={draftAction}
                onChange={(e) => setDraftAction(e.target.value as RideDraftAction)}
              >
                {r.status === 'pending_approval' && <option value="approve">{t('actionApprove')}</option>}
                <option value="reassign">{t('actionReassign')}</option>
                <option value="return">{t('actionReturn')}</option>
                <option value="alt_transport">{t('actionAltTransport')}</option>
                <option value="reimbursement_excluded">
                  {draftReimbExcl ? t('actionReimbExclude') : t('actionReimbInclude')}
                </option>
              </select>
            </Field>
            {draftAction === 'reassign' && (
              <Field label={t('proposeDriverLabel')}>
                <select
                  className="input"
                  aria-label={t('proposeDriverLabel')}
                  value={draftDriverId}
                  onChange={(e) => setDraftDriverId(e.target.value)}
                >
                  <option value="">{t('chooseDriver')}</option>
                  {(drivers.data ?? []).filter((d) => d.id !== r.passenger.id).map((d) => (
                    <option key={d.id} value={d.id}>
                      {d.name} · {d.areas || t('noAreas')} · {d.vehicle?.model ?? t('noVehicle')}
                    </option>
                  ))}
                </select>
              </Field>
            )}
            {draftAction === 'reimbursement_excluded' && (
              <label className="row items-center gap-xs small">
                <input
                  type="checkbox"
                  checked={draftReimbExcl}
                  onChange={(e) => setDraftReimbExcl(e.target.checked)}
                />
                <span>{t('proposeExcludeLabel')}</span>
              </label>
            )}
            <Field label={t('proposeReasonLabel')}>
              <textarea
                className="input"
                aria-label={t('proposeReasonLabel')}
                rows={3}
                value={draftReason}
                placeholder={t('proposeReasonPlaceholder')}
                onChange={(e) => setDraftReason(e.target.value)}
              />
            </Field>
            <ErrorBox error={error} />
            <div className="btn-row end">
              <Button
                variant="secondary"
                disabled={busy || (draftAction === 'reassign' && !draftDriverId)}
                onClick={() => submitDraftModal(false)}
              >
                {t('proposeSaveDraft')}
              </Button>
              <Button
                variant="primary"
                disabled={busy || (draftAction === 'reassign' && !draftDriverId)}
                onClick={() => submitDraftModal(true)}
              >
                {t('proposeSubmitLead')}
              </Button>
            </div>
          </div>
        </Modal>
      )}
      {returningDraft && (
        <Modal
          title={t('proposeReturnTitle')}
          onClose={() => {
            setReturningDraft(null);
            setReturnNotes('');
          }}
        >
          <div className="stack">
            <Field label={t('proposeReturnNotesLabel')}>
              <textarea
                className="input"
                aria-label={t('proposeReturnNotesLabel')}
                rows={3}
                value={returnNotes}
                placeholder={t('proposeReturnNotesPlaceholder')}
                onChange={(e) => setReturnNotes(e.target.value)}
              />
            </Field>
            <div className="btn-row end">
              <Button
                variant="ghost"
                onClick={() => {
                  setReturningDraft(null);
                  setReturnNotes('');
                }}
              >
                {t('cancel', { ns: 'common' })}
              </Button>
              <Button
                variant="danger"
                disabled={busy || !returnNotes.trim()}
                onClick={async () => {
                  const res = await run(
                    () =>
                      post(`/rides/${r.id}/drafts/${returningDraft.id}/return`, { notes: returnNotes.trim() }).then(() =>
                        rideDrafts.reload(),
                      ),
                    t('proposeReturned'),
                  );
                  if (res) {
                    setReturningDraft(null);
                    setReturnNotes('');
                  }
                }}
              >
                {t('proposeReturn')}
              </Button>
            </div>
          </div>
        </Modal>
      )}
      {modal === 'reassign' && (
        <Modal title={t('reassignTitle')} onClose={() => setModal(null)}>
          <Field label={t('reassignDriver')} hint={t('reassignHint')}>
            <select className="input" value={driverId} onChange={(e) => setDriverId(e.target.value)}>
              <option value="">{t('chooseDriver')}</option>
              {(drivers.data ?? []).filter((d) => d.id !== r.passenger.id).map((d) => (
                <option key={d.id} value={d.id}>{d.name} · {d.areas || t('noAreas')} · {d.vehicle?.model ?? t('noVehicle')}{d.carpool_enabled ? t('carpool', { seats: d.spare_seats }) : ''}</option>
              ))}
            </select>
          </Field>
          <ErrorBox error={error} />
          <Button variant="primary" block disabled={!driverId || busy} onClick={async () => {
            if (await run(() => post(`/rides/${r.id}/reassign`, { driver_id: Number(driverId) }).then(reload), t('respondedAccepted'))) setModal(null);
          }}>{t('confirmAssign')}</Button>
        </Modal>
      )}
      {modal === 'return' && <ReasonModal error={error} title={t('returnTitle')} label={t('returnLabel')} confirm={t('returnConfirm')} onClose={() => setModal(null)} onSubmit={(reason) => run(() => post(`/rides/${r.id}/return`, { reason }).then(reload), t('returned'))} />}
      {modal === 'alt' && <ReasonModal error={error} title={t('altTitle')} label={t('altLabel')} initial={t('altInitial')} confirm={t('altConfirm')} onClose={() => setModal(null)} onSubmit={(reason) => run(() => post(`/rides/${r.id}/alt-transport`, { reason }).then(reload), t('altDone'))} />}
      {modal === 'cancel' && <ReasonModal error={error} title={t('cancelTitle')} label={t('cancelLabel')} confirm={t('cancelConfirm')} onClose={() => setModal(null)} onSubmit={(reason) => run(() => post(`/rides/${r.id}/cancel`, { reason }).then(reload), t('cancelled'))} />}
    </section>
  );
}

export function MobileStickyActions({
  ride: r,
  reload,
  passengerTurn,
}: {
  ride: Ride;
  reload: () => void;
  passengerTurn: boolean;
}) {
  const { t } = useTranslation('rideDetail');
  const { t: tc } = useTranslation();
  const toast = useToast();
  const auth = useAuth();
  const meId = auth?.me?.id;
  const can = auth?.can ?? (() => true);
  const { busy, error, run } = useAction();
  const [showMore, setShowMore] = useState(false);

  const act = (path: string, msg: string) => run(() => post(`/rides/${r.id}/${path}`).then(reload), msg);

  const actQueueable = async (path: string, label: string, successMsg: string) => {
    if (!meId) {
      if (await run(() => post(`/rides/${r.id}/${path}`).then(reload))) {
        toast.success(successMsg);
      }
      return;
    }
    await run(async () => {
      const res = await postOrQueue({
        userId: meId,
        rideId: r.id,
        path: `/rides/${r.id}/${path}`,
        label,
      });
      if (res.queued) {
        toast.info(tc('queuedOffline'));
      } else {
        toast.success(successMsg);
        reload();
      }
    });
  };

  const isVolunteerClaimable =
    !r.viewer.is_passenger && !r.viewer.is_driver && !r.viewer.is_staff && r.status === 'pending_driver';
  const showDriverSticky = r.viewer.is_driver || isVolunteerClaimable;
  const showStaffApprove =
    r.viewer.is_staff && can('admin_rides', 'update') && r.status === 'pending_approval';

  const modalContent = (
    <Modal title={t('moreActions')} onClose={() => setShowMore(false)}>
      {r.viewer.is_passenger && !passengerTurn && <PassengerActions ride={r} reload={reload} />}
      {(r.viewer.is_driver || isVolunteerClaimable) && <DriverActions ride={r} reload={reload} />}
      {r.viewer.is_staff && <StaffActions ride={r} reload={reload} />}
    </Modal>
  );

  if (showDriverSticky) {
    let primaryButton: ReactNode = null;
    if (r.status === 'pending_driver') {
      primaryButton = (
        <Button variant="primary" block disabled={busy} onClick={() => act('claim', t('respondedAccepted'))}>
          {t('claimRide')}
        </Button>
      );
    } else if (r.status === 'pending_driver_confirm') {
      primaryButton = (
        <Button variant="primary" block disabled={busy} onClick={() => act('confirm-assignment', t('respondedConfirmed'))}>
          {t('confirmAssignment')}
        </Button>
      );
    } else if (r.status === 'confirmed') {
      primaryButton = <StartTripButton ride={r} onStarted={reload} block label={t('startTrip')} />;
    } else if (r.status === 'in_progress' && !r.picked_up_at) {
      primaryButton = (
        <Button variant="primary" block disabled={busy} onClick={() => actQueueable('pickup', t('pickedUp'), t('respondedPickedUp'))}>
          {t('pickedUp')}
        </Button>
      );
    } else if (r.status === 'in_progress' && r.picked_up_at) {
      primaryButton = (
        <Button variant="primary" block disabled={busy} onClick={() => actQueueable('complete', t('completedTrip'), t('respondedCompleted'))}>
          {t('completedTrip')}
        </Button>
      );
    }

    if (primaryButton) {
      return (
        <>
          {error && <ErrorBox error={error} />}
          <div className="ride-detail-sticky-bar">
            <div className="grow">{primaryButton}</div>
            {r.viewer.is_driver && (
              <Button variant="secondary" onClick={() => setShowMore(true)} icon={<MoreHorizontal size={18} />}>
                {t('moreActions')}
              </Button>
            )}
          </div>
          {showMore && modalContent}
        </>
      );
    }
  }

  if (showStaffApprove) {
    return (
      <>
        {error && <ErrorBox error={error} />}
        <div className="ride-detail-sticky-bar">
          <div className="grow">
            <Button variant="primary" block disabled={busy} onClick={() => act('approve', t('respondedAccepted'))}>
              {t('approve')}
            </Button>
          </div>
          <Button variant="secondary" onClick={() => setShowMore(true)} icon={<MoreHorizontal size={18} />}>
            {t('moreActions')}
          </Button>
        </div>
        {showMore && modalContent}
      </>
    );
  }

  // A passenger whose controls are already on the page or on a completed/terminal ride has nothing more to open.
  if (passengerTurn && !r.viewer.is_driver && !r.viewer.is_staff) return null;
  if (r.viewer.is_passenger && !r.viewer.is_driver && !r.viewer.is_staff && ['completed', 'cancelled', 'no_show'].includes(r.status)) {
    return null;
  }

  return (
    <>
      <div className="ride-detail-sticky-bar">
        <Button variant="secondary" block onClick={() => setShowMore(true)} icon={<MoreHorizontal size={18} />}>
          {t('moreActions')}
        </Button>
      </div>
      {showMore && modalContent}
    </>
  );
}
