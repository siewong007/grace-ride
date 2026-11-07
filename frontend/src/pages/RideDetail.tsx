import { useEffect, useRef, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { LifeBuoy, MapPin, Phone } from 'lucide-react';
import { post, useApi } from '../api';
import Chat from '../components/Chat';
import LiveMap from '../components/LiveMap';
import { Ride } from '../components/RideCard';
import { Avatar, ErrorBox, Field, Modal, Spinner, StatusPill, useAction } from '../components/ui';
import { fmtAgo, fmtDateTime, RIDE_FLOW, RIDE_STATUS, useFormatT } from '../format';
import { useTranslation } from 'react-i18next';

export default function RideDetail() {
  const { t, ft } = useTranslationHooks();
  const { id } = useParams();
  const { data: r, error, reload } = useApi<Ride>(`/rides/${id}`, 15000);
  if (error) return <ErrorBox error={error} />;
  if (!r) return <Spinner />;
  const chatVisible = r.trip_id != null && (r.viewer.is_passenger || r.viewer.is_driver || r.viewer.is_staff);
  const live = r.status === 'in_progress';

  return (
    <div className="stack">
      <div className="page-head">
        <div>
          <h1>{fmtDateTime(r.pickup_at, ft)}</h1>
          <p>{r.area.name} → {r.destination}</p>
        </div>
        <StatusPill map={RIDE_STATUS} status={r.status} />
      </div>
      <Flow status={r.status} ft={ft} />
      {r.needs_attention && r.attention_reason && <div className="notice">⚠ {r.attention_reason}</div>}

      <div className="split">
        <div className="stack">
          {live && <TrackCard t={t} ft={ft} ride={r} />}
          <Details ride={r} t={t} ft={ft} />
          {r.viewer.is_passenger && <PassengerActions ride={r} reload={reload} t={t} />}
          {r.viewer.is_driver && <DriverActions ride={r} reload={reload} t={t} />}
          {r.viewer.is_staff && <StaffActions ride={r} reload={reload} t={t} />}
          {!r.viewer.is_passenger && !r.viewer.is_driver && !r.viewer.is_staff && r.status === 'pending_driver' && <DriverActions ride={r} reload={reload} t={t} />}
        </div>
        <div className="stack">
          {chatVisible && <Chat rideId={r.id} />}
          {(r.viewer.is_passenger || r.viewer.is_driver) && (
            <Link className="btn" to={`/help?ride=${r.id}`}><LifeBuoy size={18} />{t('needHelp')}</Link>
          )}
        </div>
      </div>
    </div>
  );
}

function useTranslationHooks() {
  const { t } = useTranslation('rideDetail');
  const ft = useFormatT();
  return { t, ft };
}

function Flow({ status, ft }: { status: string; ft: (k: string) => string }) {
  const idx = RIDE_FLOW.indexOf(status === 'pending_driver_confirm' ? 'pending_approval' : status);
  if (idx < 0) return null;
  const labels = RIDE_FLOW.map((s) => ft(`format:rideStatus.${s}`));
  return (
    <div>
      <div className="flow" aria-hidden>
        {RIDE_FLOW.map((s, i) => <div key={s} className={i < idx ? 'done' : i === idx ? 'now' : ''} />)}
      </div>
      <div className="flow-labels">{labels.map((lab, i) => <span key={i}>{lab}</span>)}</div>
    </div>
  );
}

function Details({ ride: r, t, ft }: { ride: Ride; t: (k: string, o?: any) => string; ft: (k: string) => string }) {
  const d = r.driver;
  return (
    <div className="card stack">
      {d && (
        <div className="row nowrap">
          <Avatar name={d.name} size="lg" />
          <div className="grow">
            <div className="muted small">{r.viewer.is_driver ? t('youAreDriver') : t('driver')}</div>
            <div className="bold" style={{ fontSize: '1.2rem' }}>{d.name}</div>
            {d.vehicle && (
              <div className="row" style={{ gap: 8 }}>
                <span>{d.vehicle.color} {d.vehicle.model}</span>
                <span className="plate">{d.vehicle.plate}</span>
              </div>
            )}
          </div>
          {d.phone && !r.viewer.is_driver && <a className="btn primary" href={`tel:${d.phone}`}><Phone size={18} /><span className="hide-mobile">{t('callDriver')}</span></a>}
        </div>
      )}
      <dl className="kv">
        <dt>{t('passenger')}</dt>
        <dd>
          {r.passenger.name}
          {r.passenger.phone && !r.viewer.is_passenger && <> · <a href={`tel:${r.passenger.phone}`}>{r.passenger.phone}</a></>}
        </dd>
        <dt>{t('pickupArea')}</dt><dd>{r.area.name}</dd>
        <dt>{t('pickupAddress')}</dt>
        <dd>{r.pickup_address ?? <span className="muted">{t('pickupAddressLocked')}</span>}</dd>
        <dt>{t('destination')}</dt><dd>{r.destination}</dd>
        <dt>{t('people')}</dt><dd>{r.seats} {t('seatsUnit')}</dd>
        {r.assistance && <><dt>{t('assistance')}</dt><dd>{r.assistance}</dd></>}
        {r.notes && <><dt>{t('notes')}</dt><dd>{r.notes}</dd></>}
        {r.event && <><dt>{t('event')}</dt><dd><Link to={`/events/${r.event.id}`}>{r.event.title}</Link></dd></>}
        {r.picked_up_at && <><dt>{t('pickedUp')}</dt><dd>{fmtAgo(r.picked_up_at, ft)}</dd></>}
      </dl>
      {r.pickup_address && (r.viewer.is_driver || r.viewer.is_staff) && (
        <a className="btn" target="_blank" rel="noreferrer" href={`https://www.google.com/maps/dir/?api=1&destination=${encodeURIComponent(r.pickup_address)}`}>
          <MapPin size={18} />{t('navigateToPickup')}
        </a>
      )}
    </div>
  );
}

function TrackCard({ t, ft, ride }: { t: (k: string, o?: any) => string; ft: (k: string) => string; ride: Ride }) {
  const { data } = useApi<{ sharing: boolean; lat?: number; lng?: number; updated_at?: string }>(`/rides/${ride.id}/location`, 15000);
  if (ride.viewer.is_driver) return <ShareLocation />;
  return (
    <div className="card stack-sm">
      <div className="row between"><h3>{t('driverLocation')}</h3>{data?.updated_at && <span className="muted small">{t('lastUpdated')} {fmtAgo(data.updated_at, ft)}</span>}</div>
      {data?.sharing && data.lat != null && data.lng != null ? (
        <LiveMap points={[{ id: ride.driver?.id ?? 0, lat: data.lat, lng: data.lng, label: ride.driver?.name ?? t('driver') }]} />
      ) : <p className="muted">{t('waitingLocation')}</p>}
      <p className="muted tiny">{t('shareInfo')}</p>
    </div>
  );
}

export function ShareLocation() {
  const { t } = useTranslation('rideDetail');
  const [state, setState] = useState<'idle' | 'on' | 'denied' | 'unsupported'>('idle');
  const last = useRef(0);
  useEffect(() => {
    if (!('geolocation' in navigator)) { setState('unsupported'); return; }
    const watch = navigator.geolocation.watchPosition(
      (pos) => {
        setState('on');
        if (Date.now() - last.current < 15000) return;
        last.current = Date.now();
        post('/driver/location', { lat: pos.coords.latitude, lng: pos.coords.longitude, heading: pos.coords.heading }).catch(() => {});
      },
      () => setState('denied'),
      { enableHighAccuracy: true, maximumAge: 10000 },
    );
    return () => navigator.geolocation.clearWatch(watch);
  }, []);
  return (
    <div className={`notice ${state === 'on' ? 'ok' : ''}`}>
      {state === 'on' && t('sharingOn')}
      {state === 'idle' && t('sharingIdle')}
      {state === 'denied' && t('sharingDenied')}
      {state === 'unsupported' && t('sharingUnsupported')}
    </div>
  );
}

function PassengerActions({ ride: r, reload, t }: { ride: Ride; reload: () => void; t: (k: string, o?: any) => string }) {
  const { busy, error, run } = useAction();
  const nav = useNavigate();
  const [cancel, setCancel] = useState(false);
  const cancellable = ['pending_driver', 'pending_passenger', 'pending_approval', 'pending_driver_confirm', 'confirmed'].includes(r.status);
  return (
    <div className="stack-sm">
      {r.status === 'pending_passenger' && r.driver && (
        <div className="card stack">
          <h3>{t('driverInvite')}</h3>
          <p>{t('driverInviteDesc', { name: r.driver.name })}</p>
          <div className="btn-row">
            <button className="btn primary" disabled={busy} onClick={() => run(() => post(`/rides/${r.id}/respond`, { accept: true }).then(reload), t('respondedAccepted'))}>{t('acceptInvite')}</button>
            <button className="btn" disabled={busy} onClick={() => run(() => post(`/rides/${r.id}/respond`, { accept: false }).then(reload), t('respondedDeclined'))}>{t('declineInvite')}</button>
          </div>
        </div>
      )}
      {r.status === 'pending_driver' && <div className="notice info">{t('submittedWaiting')}</div>}
      {r.status === 'pending_approval' && <div className="notice info">{t('awaitingApproval')}</div>}
      {r.status === 'alt_transport' && (
        <div className="notice info">
          {t('altTransportNotice', { link: <Link to={`/reimbursements/new?ride=${r.id}`}>上传 Grab 收据</Link> })}
        </div>
      )}
      <ErrorBox error={error} />
      {cancellable && <button className="btn danger" onClick={() => setCancel(true)}>{t('cancelRide')}</button>}
      {cancel && (
        <ReasonModal error={error} title={t('cancelTitle')} label={t('cancelReason')} confirm={t('confirmCancel')} onClose={() => setCancel(false)}
          onSubmit={(reason) => run(() => post(`/rides/${r.id}/cancel`, { reason }).then(() => nav('/rides')), t('rideCancelled'))} />
      )}
    </div>
  );
}

function DriverActions({ ride: r, reload, t }: { ride: Ride; reload: () => void; t: (k: string, o?: any) => string }) {
  const { busy, error, run } = useAction();
  const [withdraw, setWithdraw] = useState(false);
  const act = (path: string, msg: string) => run(() => post(`/rides/${r.id}/${path}`).then(reload), msg);
  return (
    <div className="card stack">
      <h3>{t('driverActions')}</h3>
      {r.status === 'pending_driver' && (
        <>
          <p className="muted">{t('driverClaimDesc')}</p>
          <button className="btn primary" disabled={busy} onClick={() => act('claim', t('respondedAccepted'))}>{t('claimRide')}</button>
        </>
      )}
      {r.status === 'pending_passenger' && <p className="muted">{t('waitingPassenger')}</p>}
      {r.status === 'pending_approval' && <p className="muted">{t('passengerAccepted')}</p>}
      {r.status === 'pending_driver_confirm' && (
        <>
          <p>{t('assignYou')}</p>
          <button className="btn primary" disabled={busy} onClick={() => act('confirm-assignment', t('respondedCompleted'))}>{t('confirmAssignment')}</button>
        </>
      )}
      {r.status === 'confirmed' && <button className="btn primary" disabled={busy} onClick={() => act('start', t('respondedStarted'))}>{t('startTrip')}</button>}
      {r.status === 'in_progress' && (
        <div className="btn-row">
          {!r.picked_up_at && <button className="btn primary" disabled={busy} onClick={() => act('pickup', t('respondedPickedUp'))}>{t('pickedUp')}</button>}
          {!r.picked_up_at && <button className="btn danger" disabled={busy} onClick={() => act('no-show', t('respondedNoShow'))}>{t('noShowBtn')}</button>}
          <button className="btn accent" disabled={busy} onClick={() => act('complete', t('respondedCompleted'))}>{t('completedTrip')}</button>
        </div>
      )}
      {['pending_passenger', 'pending_approval', 'pending_driver_confirm', 'confirmed'].includes(r.status) && r.viewer.is_driver && (
        <button className="btn danger" onClick={() => setWithdraw(true)}>{t('withdrawDriver')}</button>
      )}
      <ErrorBox error={error} />
      {withdraw && (
        <ReasonModal error={error} title={t('withdrawTitle')} label={t('withdrawLabel')} confirm={t('withdrawConfirm')} onClose={() => setWithdraw(false)}
          onSubmit={(reason) => run(() => post(`/rides/${r.id}/withdraw`, { reason }).then(reload), t('respondedWithdrew'))} />
      )}
    </div>
  );
}

interface DriverOpt { id: number; name: string; areas: string; carpool_enabled: boolean; spare_seats: number; vehicle: { model: string; plate: string } | null }

function StaffActions({ ride: r, reload, t }: { ride: Ride; reload: () => void; t: (k: string, o?: any) => string }) {
  const { busy, error, run } = useAction();
  const [modal, setModal] = useState<'reassign' | 'return' | 'alt' | 'cancel' | null>(null);
  const drivers = useApi<DriverOpt[]>(modal === 'reassign' ? '/admin/drivers' : null);
  const [driverId, setDriverId] = useState('');
  const open = !['completed', 'cancelled', 'no_show', 'alt_transport', 'in_progress'].includes(r.status);
  return (
    <div className="card stack">
      <h3>{t('staffActions')}</h3>
      {r.driver_load && (
        <p className="small">{t('driverLoad', { used: r.driver_load.seats_used, capacity: r.driver_load.capacity, seats: r.seats })}</p>
      )}
      {!!r.needs_attention && r.attention_reason && <div className="notice">⚠ {r.attention_reason}</div>}
      {r.assistance && <div className="notice info">{t('assistanceLabel', { assistance: r.assistance })}</div>}
      <div className="btn-row">
        {r.status === 'pending_approval' && (
          <button className="btn primary" disabled={busy} onClick={() => run(() => post(`/rides/${r.id}/approve`).then(reload), t('respondedAccepted'))}>{t('approve')}</button>
        )}
        {open && <button className="btn" onClick={() => setModal('reassign')}>{t('reassign')}</button>}
        {['pending_approval', 'pending_passenger', 'pending_driver_confirm'].includes(r.status) && <button className="btn" onClick={() => setModal('return')}>{t('retur')}</button>}
        {open && <button className="btn" onClick={() => setModal('alt')}>{t('altTransport')}</button>}
        {open && <button className="btn danger" onClick={() => setModal('cancel')}>{t('cancel')}</button>}
        {r.status === 'in_progress' && <button className="btn danger" disabled={busy} onClick={() => run(() => post(`/rides/${r.id}/no-show`).then(reload), t('respondedNoShow'))}>{t('noShow')}</button>}
      </div>
      <Link className="small" to={`/admin/audit?entity=ride&entity_id=${r.id}`}>{t('auditLink')}</Link>
      <ErrorBox error={error} />
      {modal === 'reassign' && (
        <Modal title={t('reassignTitle')} onClose={() => setModal(null)}>
          <Field label={t('reassignDriver')} hint={t('reassignHint')}>
            <select className="input" value={driverId} onChange={(e) => setDriverId(e.target.value)}>
              <option value="">{t('chooseDriver')}</option>
              {drivers.data?.filter((d) => d.id !== r.passenger.id).map((d) => (
                <option key={d.id} value={d.id}>{d.name} · {d.areas || t('noVehicle')} · {d.vehicle?.model ?? t('noVehicle')}{d.carpool_enabled ? t('carpool', { seats: d.spare_seats }) : ''}</option>
              ))}
            </select>
          </Field>
          <ErrorBox error={error} />
          <button className="btn primary" disabled={!driverId || busy} onClick={async () => {
            if (await run(() => post(`/rides/${r.id}/reassign`, { driver_id: Number(driverId) }).then(reload), t('respondedAccepted'))) setModal(null);
          }}>{t('confirmAssign')}</button>
        </Modal>
      )}
      {modal === 'return' && <ReasonModal error={error} title={t('returnTitle')} label={t('returnLabel')} confirm={t('returnConfirm')} onClose={() => setModal(null)} onSubmit={(reason) => run(() => post(`/rides/${r.id}/return`, { reason }).then(reload), t('returned'))} />}
      {modal === 'alt' && <ReasonModal error={error} title={t('altTitle')} label={t('altLabel')} initial={t('altInitial')} confirm={t('altConfirm')} onClose={() => setModal(null)} onSubmit={(reason) => run(() => post(`/rides/${r.id}/alt-transport`, { reason }).then(reload), t('altDone'))} />}
      {modal === 'cancel' && <ReasonModal error={error} title={t('cancelTitle')} label={t('cancelLabel')} confirm={t('cancelConfirm')} onClose={() => setModal(null)} onSubmit={(reason) => run(() => post(`/rides/${r.id}/cancel`, { reason }).then(reload), t('cancelled'))} />}
    </div>
  );
}

export function ReasonModal({ title, label, confirm, initial = '', error, onClose, onSubmit }: {
  title: string; label: string; confirm: string; initial?: string; error?: string | null; onClose: () => void; onSubmit: (reason: string) => Promise<boolean>;
}) {
  const [reason, setReason] = useState(initial);
  return (
    <Modal title={title} onClose={onClose}>
      <Field label={label}>
        <textarea className="input" value={reason} onChange={(e) => setReason(e.target.value)} autoFocus />
      </Field>
      <ErrorBox error={error} />
      <button className="btn primary" onClick={async () => { if (await onSubmit(reason)) onClose(); }}>{confirm}</button>
    </Modal>
  );
}
