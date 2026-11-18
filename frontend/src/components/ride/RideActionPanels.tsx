import { useEffect, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { AlertCircle, AlertTriangle, CheckCircle2, Clock, MoreHorizontal } from 'lucide-react';
import type { TFunction } from 'i18next';
import { Trans, useTranslation } from 'react-i18next';
import { api, post, useApi } from '../../api';
import { useAuth } from '../../auth';
import CantMakeItModal from '../CantMakeIt';
import type { Ride } from '../RideCard';
import { ReasonModal, StartTripButton } from '../TripActions';
import { Button, Card, ErrorBox, Field, Modal, Notice, useAction, useConfirm } from '../ui';

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
  const can = auth?.can ?? (() => true);
  const confirm = useConfirm();
  const { busy, error, run } = useAction();
  const nav = useNavigate();
  const [cancel, setCancel] = useState(false);
  const [unable, setUnable] = useState(false);
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
        <Button variant={part === 'cancel' ? 'ghost' : 'danger'} className={part === 'cancel' ? 'text-danger' : undefined} onClick={() => setCancel(true)}>
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

  return (
    <Card className="stack">
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

            <div className="btn-row">
              <Button
                variant={r.readiness_status === 'ready' ? 'secondary' : 'primary'}
                size="sm"
                disabled={busy}
                onClick={() => run(() => post(`/rides/${r.id}/readiness`, { status: 'ready' }).then(reload), t('readyConfirmed'))}
              >
                {r.is_proxy && r.proxy
                  ? t('readyBtnProxy', { name: r.proxy.rider_name })
                  : t('readyBtn')}
              </Button>
              <Button
                variant="secondary"
                size="sm"
                disabled={busy}
                onClick={() => run(() => post(`/rides/${r.id}/readiness`, { status: 'delayed_5m' }).then(reload), t('delayedConfirmed'))}
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
    </Card>
  );
}

export function DriverActions({ ride: r, reload }: { ride: Ride; reload: () => void }) {
  const { t } = useTranslation('rideDetail');
  const confirm = useConfirm();
  const { busy, error, run } = useAction();
  const [withdraw, setWithdraw] = useState(false);
  const act = (path: string, msg: string) => run(() => post(`/rides/${r.id}/${path}`).then(reload), msg);

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

  return (
    <Card className="stack">
      <h2>{t('driverActions')}</h2>

      {/* Passenger readiness status notice for driver */}
      {r.readiness_status === 'ready' && (
        <Notice tone="success">{t('readyConfirmed')}</Notice>
      )}
      {(r.readiness_status === 'delayed_5m' || r.readiness_status === 'delayed_10m') && (
        <Notice tone="warning">{t('delayedConfirmed')}</Notice>
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
          <Button variant="primary" disabled={busy} onClick={() => act('confirm-assignment', t('respondedCompleted'))}>
            {t('confirmAssignment')}
          </Button>
        </>
      )}
      {r.status === 'confirmed' && <StartTripButton ride={r} onStarted={reload} label={t('startTrip')} />}
      {r.status === 'in_progress' && (
        <div className="btn-row">
          {!r.picked_up_at && (
            <Button variant="primary" disabled={busy} onClick={() => act('pickup', t('respondedPickedUp'))}>
              {t('pickedUp')}
            </Button>
          )}
          {!r.picked_up_at && (
            <Button
              variant="secondary"
              disabled={busy}
              onClick={() => run(() => post(`/rides/${r.id}/arrived-gate`).then(reload), t('arrivedGateSent'))}
            >
              {t('arrivedGateBtn')}
            </Button>
          )}
          {!r.picked_up_at && (
            <Button variant="danger" disabled={busy} onClick={handleNoShow}>
              {t('noShowBtn')}
            </Button>
          )}
          <Button variant="secondary" className="btn accent" disabled={busy} onClick={() => act('complete', t('respondedCompleted'))}>
            {t('completedTrip')}
          </Button>
        </div>
      )}

      {/* Driver Delay Broadcast Buttons */}
      {(r.status === 'confirmed' || (r.status === 'in_progress' && !r.picked_up_at)) && r.viewer.is_driver && (
        <div className="stack-sm">
          <span className="small muted">{t('driverDelayBtn')}:</span>
          <div className="btn-row">
            <Button
              variant="secondary"
              size="sm"
              disabled={busy}
              onClick={() => run(() => post(`/rides/${r.id}/driver-delay`, { delay_minutes: 5 }).then(reload), t('driverDelaySent', { mins: 5 }))}
            >
              {t('driverDelay5m')}
            </Button>
            <Button
              variant="secondary"
              size="sm"
              disabled={busy}
              onClick={() => run(() => post(`/rides/${r.id}/driver-delay`, { delay_minutes: 10 }).then(reload), t('driverDelaySent', { mins: 10 }))}
            >
              {t('driverDelay10m')}
            </Button>
            <Button
              variant="secondary"
              size="sm"
              disabled={busy}
              onClick={() => run(() => post(`/rides/${r.id}/driver-delay`, { delay_minutes: 15 }).then(reload), t('driverDelaySent', { mins: 15 }))}
            >
              {t('driverDelay15m')}
            </Button>
          </div>
        </div>
      )}
      {['pending_passenger', 'pending_approval', 'pending_driver_confirm', 'confirmed'].includes(r.status) && r.viewer.is_driver && (
        <Button variant="danger" onClick={() => setWithdraw(true)}>
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
    </Card>
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

export function StaffActions({ ride: r, reload }: { ride: Ride; reload: () => void }) {
  const { t } = useTranslation('rideDetail');
  const auth = useAuth();
  const can = auth?.can ?? (() => true);
  // Staff who may only view rides get the summary without the buttons.
  const edit = can('admin_rides', 'update');
  const cancel = can('admin_rides', 'delete');
  const confirm = useConfirm();
  const { busy, error, run } = useAction();
  const [modal, setModal] = useState<'reassign' | 'return' | 'alt' | 'cancel' | null>(null);
  const drivers = useApi<DriverOpt[]>(modal === 'reassign' ? '/admin/drivers' : null);
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

  return (
    <Card className="stack">
      <h2>{t('staffActions')}</h2>
      {r.driver_load && (
        <p className="small">{t('driverLoad', { used: r.driver_load.seats_used, capacity: r.driver_load.capacity, seats: r.seats })}</p>
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
        {cancel && open && <Button variant="danger" onClick={() => setModal('cancel')}>{t('cancel')}</Button>}
        {edit && r.status === 'in_progress' && !r.picked_up_at && (
          <Button variant="danger" disabled={busy} onClick={handleStaffNoShow}>{t('noShow')}</Button>
        )}
      </div>
      <Link className="small" to={`/admin/audit?entity=ride&entity_id=${r.id}`}>{t('auditLink')}</Link>
      <ErrorBox error={error} />
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
    </Card>
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
  const [showMore, setShowMore] = useState(false);

  if (r.viewer.is_driver) {
    if (r.status === 'pending_driver') {
      return (
        <div className="ride-detail-sticky-bar">
          <DriverActions ride={r} reload={reload} />
        </div>
      );
    }
    if (r.status === 'confirmed' || r.status === 'in_progress') {
      return (
        <div className="ride-detail-sticky-bar">
          <div className="grow">
            <DriverActions ride={r} reload={reload} />
          </div>
        </div>
      );
    }
  }

  if (r.viewer.is_staff && r.status === 'pending_approval') {
    return (
      <div className="ride-detail-sticky-bar">
        <StaffActions ride={r} reload={reload} />
      </div>
    );
  }

  // A passenger whose controls are already on the page has nothing more to open.
  if (passengerTurn && !r.viewer.is_driver && !r.viewer.is_staff) return null;

  // If there's an action drawer / menu
  return (
    <>
      <div className="ride-detail-sticky-bar">
        <Button variant="secondary" block onClick={() => setShowMore(true)} icon={<MoreHorizontal size={18} />}>
          {t('moreActions')}
        </Button>
      </div>
      {showMore && (
        <Modal title={t('moreActions')} onClose={() => setShowMore(false)}>
          {r.viewer.is_passenger && !passengerTurn && <PassengerActions ride={r} reload={reload} />}
          {r.viewer.is_driver && <DriverActions ride={r} reload={reload} />}
          {r.viewer.is_staff && <StaffActions ride={r} reload={reload} />}
          {!r.viewer.is_passenger && !r.viewer.is_driver && !r.viewer.is_staff && r.status === 'pending_driver' && (
            <DriverActions ride={r} reload={reload} />
          )}
        </Modal>
      )}
    </>
  );
}
