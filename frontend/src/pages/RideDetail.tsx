import { useEffect, useRef, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import {
  AlertCircle,
  AlertTriangle,
  CheckCircle2,
  ChevronDown,
  ChevronUp,
  Circle,
  CircleDot,
  Clock,
  Hand,
  LifeBuoy,
  Lock,
  MapPin,
  MessageSquare,
  MoreHorizontal,
  Navigation,
  Phone,
} from 'lucide-react';
import { del, post, useApi } from '../api';
import Chat from '../components/Chat';
import LiveMap, { MapPoint } from '../components/LiveMap';
import { Ride } from '../components/RideCard';
import {
  Avatar,
  Button,
  Card,
  ErrorBox,
  Field,
  IconButton,
  KeyValue,
  KeyValueItem,
  Modal,
  Notice,
  Page,
  PageHeader,
  Spinner,
  StatusPill,
  useAction,
  useConfirm,
} from '../components/ui';
import {
  fmtAgo,
  fmtDateTime,
  RIDE_STATUS,
  useFormatT,
} from '../format';
import { Trans, useTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';

export default function RideDetail() {
  const { t } = useTranslation('rideDetail');
  const ft = useFormatT();
  const { id } = useParams();
  const chatRef = useRef<HTMLDivElement>(null);
  const { data: r, error, reload } = useApi<Ride>(`/rides/${id}`, 15000);

  if (error) return <ErrorBox error={error} />;
  if (!r) return <Spinner />;

  const chatVisible = r.trip_id != null && (r.viewer.is_passenger || r.viewer.is_driver || r.viewer.is_staff);
  const live = r.status === 'in_progress';

  const scrollToChat = () => {
    chatRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  return (
    <Page width="wide">
      <div className="stack">
        {/* Status Header */}
        <PageHeader
          title={fmtDateTime(r.pickup_at, ft)}
          subtitle={`${r.area.name} → ${r.destination}`}
          back="/rides"
          actions={<StatusPill map={RIDE_STATUS} status={r.status} />}
        />

        {r.needs_attention && r.attention_reason && (
          <Notice tone="warning" icon={<AlertTriangle size={18} />}>
            <span>{r.attention_reason}</span>
          </Notice>
        )}

        {/* Desktop Split / Mobile Stack Layout */}
        <div className="split">
          {/* Left Column: Details, Driver, Timeline */}
          <div className="stack">
            {/* Driver / Vehicle Card */}
            <DriverCard ride={r} onChatClick={chatVisible ? scrollToChat : undefined} />

            {/* Live Map (Mobile & Desktop priority when in progress) */}
            {live && <CollapsibleTrackCard ride={r} />}

            {/* Trip Details */}
            <Details ride={r} />

            {/* Vertical Status Timeline */}
            <StatusTimeline ride={r} />

            {/* Actions for Desktop / Tablet */}
            <div className="hide-mobile">
              {r.viewer.is_passenger && <PassengerActions ride={r} reload={reload} />}
              {r.viewer.is_driver && <DriverActions ride={r} reload={reload} />}
              {r.viewer.is_staff && <StaffActions ride={r} reload={reload} />}
              {!r.viewer.is_passenger && !r.viewer.is_driver && !r.viewer.is_staff && r.status === 'pending_driver' && (
                <DriverActions ride={r} reload={reload} />
              )}
            </div>
          </div>

          {/* Right Column: Chat, Help */}
          <div className="stack">
            {chatVisible && (
              <div ref={chatRef}>
                <Chat rideId={r.id} />
              </div>
            )}
            {(r.viewer.is_passenger || r.viewer.is_driver) && (
              <Button variant="ghost" to={`/help?ride=${r.id}`} icon={<LifeBuoy size={18} />}>
                {t('needHelp')}
              </Button>
            )}
          </div>
        </div>

        {/* Sticky Action Bar on Phones */}
        <div className="hide-tablet-up">
          <MobileStickyActions ride={r} reload={reload} />
        </div>
      </div>
    </Page>
  );
}

function DriverCard({
  ride: r,
  onChatClick,
}: {
  ride: Ride;
  onChatClick?: () => void;
}) {
  const { t } = useTranslation('rideDetail');
  const d = r.driver;

  if (!d) {
    return (
      <Card className="stack-sm">
        <Notice tone="info" icon={<Clock size={18} />}>
          {t('noDriverYet')}
        </Notice>
      </Card>
    );
  }

  return (
    <Card className="stack-sm">
      <div className="row between nowrap">
        <div className="row nowrap gap-md">
          <Avatar name={d.name} size="lg" />
          <div>
            <div className="muted small">
              {r.viewer.is_driver ? t('youAreDriver') : t('driver')}
            </div>
            <div className="bold font-lg">{d.name}</div>
            {d.vehicle && (
              <div className="row nowrap gap-sm">
                <span className="small muted">{d.vehicle.color} {d.vehicle.model}</span>
                <span className="plate">{d.vehicle.plate}</span>
              </div>
            )}
          </div>
        </div>

        <div className="btn-row nowrap">
          {d.phone && !r.viewer.is_driver && (
            <a className="btn sm primary" href={`tel:${d.phone}`}>
              <Phone size={16} aria-hidden="true" />
              <span>{t('call')}</span>
            </a>
          )}
          {r.trip_id != null && onChatClick && (
            <Button variant="ghost" size="sm" onClick={onChatClick}>
              <MessageSquare size={16} aria-hidden="true" />
              <span>{t('chatBtn')}</span>
            </Button>
          )}
        </div>
      </div>
    </Card>
  );
}

function StatusTimeline({ ride }: { ride: Ride }) {
  const { t } = useTranslation('rideDetail');
  const ft = useFormatT();

  const isTerminal = ['cancelled', 'no_show', 'alt_transport'].includes(ride.status);

  if (isTerminal) {
    return (
      <Card className="stack-sm">
        <h3>{t('statusTimeline')}</h3>
        <div className="status-timeline">
          <div className="timeline-step done">
            <div className="timeline-step-indicator">
              <div className="timeline-step-icon">
                <CheckCircle2 size={14} aria-hidden="true" />
              </div>
              <div className="timeline-step-line" />
            </div>
            <div className="timeline-step-content">
              <span className="timeline-step-title">{t('flow0')}</span>
              <span className="timeline-step-desc">{fmtDateTime(ride.pickup_at, ft)}</span>
            </div>
          </div>
          <div className="timeline-step cancelled">
            <div className="timeline-step-indicator">
              <div className="timeline-step-icon">
                <AlertCircle size={14} aria-hidden="true" />
              </div>
            </div>
            <div className="timeline-step-content">
              <span className="timeline-step-title">{ft(RIDE_STATUS[ride.status]?.label)}</span>
              {ride.attention_reason && (
                <span className="timeline-step-desc">{ride.attention_reason}</span>
              )}
            </div>
          </div>
        </div>
      </Card>
    );
  }

  type StepStatus = 'done' | 'current' | 'upcoming';

  const getStepState = (stepIndex: number): StepStatus => {
    let currentIdx = 0;
    if (ride.status === 'pending_driver') currentIdx = 0;
    else if (ride.status === 'pending_passenger' || ride.status === 'pending_driver_confirm') currentIdx = 1;
    else if (ride.status === 'pending_approval') currentIdx = 1;
    else if (ride.status === 'confirmed') currentIdx = 2;
    else if (ride.status === 'in_progress') currentIdx = 3;
    else if (ride.status === 'completed') currentIdx = 4;

    if (stepIndex < currentIdx) return 'done';
    if (stepIndex === currentIdx) return 'current';
    return 'upcoming';
  };

  const steps = [
    {
      title: t('flow0'),
      state: getStepState(0),
      time: fmtDateTime(ride.pickup_at, ft),
    },
    {
      title: t('flow1'),
      state: getStepState(1),
      desc: ride.driver ? ride.driver.name : undefined,
    },
    {
      title: t('flow2'),
      state: getStepState(2),
    },
    {
      title: ft('format:rideStatus.in_progress'),
      state: getStepState(3),
      time: ride.started_at ? fmtDateTime(ride.started_at, ft) : undefined,
      desc: ride.picked_up_at ? `${t('pickedUp')} (${fmtAgo(ride.picked_up_at, ft)})` : undefined,
    },
    {
      title: t('flow3'),
      state: getStepState(4),
      time: ride.completed_at ? fmtDateTime(ride.completed_at, ft) : undefined,
    },
  ];

  return (
    <Card className="stack-sm">
      <h3>{t('statusTimeline')}</h3>
      <div className="status-timeline">
        {steps.map((st, i) => (
          <div key={i} className={`timeline-step ${st.state}`}>
            <div className="timeline-step-indicator">
              <div className="timeline-step-icon">
                {st.state === 'done' ? (
                  <CheckCircle2 size={14} aria-hidden="true" />
                ) : st.state === 'current' ? (
                  <CircleDot size={14} aria-hidden="true" />
                ) : (
                  <Circle size={10} aria-hidden="true" />
                )}
              </div>
              {i < steps.length - 1 && <div className="timeline-step-line" />}
            </div>
            <div className="timeline-step-content">
              <span className="timeline-step-title">{st.title}</span>
              {st.time && <span className="timeline-step-desc">{st.time}</span>}
              {st.desc && <span className="timeline-step-desc muted">{st.desc}</span>}
            </div>
          </div>
        ))}
      </div>
    </Card>
  );
}

function Details({ ride: r }: { ride: Ride }) {
  const { t } = useTranslation('rideDetail');
  const ft = useFormatT();

  const kvItems: KeyValueItem[] = [
    {
      label: t('passenger'),
      value: (
        <span>
          {r.passenger.name}
          {r.passenger.phone && !r.viewer.is_passenger && (
            <> · <a href={`tel:${r.passenger.phone}`}>{r.passenger.phone}</a></>
          )}
        </span>
      ),
    },
    { label: t('pickupArea'), value: r.area.name },
    {
      label: t('pickupAddress'),
      value: r.pickup_address ? (
        <span>{r.pickup_address}</span>
      ) : (
        <div className="row between nowrap">
          <span className="muted">{t('pickupAddressLocked')}</span>
          <Lock size={14} className="muted" aria-hidden="true" />
        </div>
      ),
    },
    { label: t('destination'), value: r.destination },
    { label: t('people'), value: `${r.seats} ${t('seatsUnit')}` },
  ];

  if (r.assistance) {
    kvItems.push({ label: t('assistance'), value: r.assistance });
  }
  if (r.notes) {
    kvItems.push({ label: t('notes'), value: r.notes });
  }
  if (r.event) {
    kvItems.push({
      label: t('event'),
      value: <Link to={`/events/${r.event.id}`}>{r.event.title}</Link>,
    });
  }
  if (r.picked_up_at) {
    kvItems.push({
      label: t('pickedUp'),
      value: fmtAgo(r.picked_up_at, ft),
    });
  }

  return (
    <Card className="stack">
      <KeyValue items={kvItems} />

      {r.pickup_address && (r.viewer.is_driver || r.viewer.is_staff) && (
        <div className="nav-buttons-group">
          <span className="nav-group-label muted small bold">
            {t('navigateToPickup')}:
          </span>
          <div className="nav-group-buttons">
            <a
              className="btn sm secondary"
              target="_blank"
              rel="noreferrer"
              href={`https://www.google.com/maps/dir/?api=1&destination=${encodeURIComponent(r.pickup_address)}`}
            >
              <Navigation size={14} aria-hidden="true" />
              <span>{t('navigateGoogleMaps')}</span>
            </a>
            <a
              className="btn sm secondary"
              target="_blank"
              rel="noreferrer"
              href={`https://www.waze.com/ul?q=${encodeURIComponent(r.pickup_address)}`}
            >
              <Navigation size={14} aria-hidden="true" />
              <span>{t('navigateWaze')}</span>
            </a>
          </div>
        </div>
      )}
    </Card>
  );
}

interface RideLocation {
  sharing: boolean;
  lat?: number;
  lng?: number;
  updated_at?: string;
  passenger?: { lat: number; lng: number; updated_at: string };
}

function CollapsibleTrackCard({ ride }: { ride: Ride }) {
  const { t } = useTranslation('rideDetail');
  const ft = useFormatT();
  const [collapsed, setCollapsed] = useState(false);
  const { data, reload } = useApi<RideLocation>(`/rides/${ride.id}/location`, 10000);

  const points: MapPoint[] = [];
  if (data?.sharing && data.lat != null && data.lng != null) {
    points.push({
      id: 'driver',
      lat: data.lat,
      lng: data.lng,
      label: ride.driver?.name ?? t('driver'),
      detail: data.updated_at ? `${t('lastUpdated')} ${fmtAgo(data.updated_at, ft)}` : undefined,
    });
  }
  if (data?.passenger) {
    points.push({
      id: 'passenger',
      kind: 'person',
      lat: data.passenger.lat,
      lng: data.passenger.lng,
      label: ride.passenger.name,
      detail: `${t('lastUpdated')} ${fmtAgo(data.passenger.updated_at, ft)}`,
    });
  }
  const waiting = !ride.picked_up_at;

  return (
    <Card className="stack-sm collapsible-card">
      <button
        type="button"
        className="collapsible-card-header"
        onClick={() => setCollapsed(!collapsed)}
        aria-expanded={!collapsed}
      >
        <div className="row nowrap gap-sm">
          <Navigation size={18} aria-hidden="true" />
          <h3 className="no-margin">
            {ride.viewer.is_driver ? t('tripMap') : t('driverLocation')}
          </h3>
        </div>
        <div className="row nowrap gap-sm">
          {data?.updated_at && (
            <span className="muted small">
              {t('lastUpdated')} {fmtAgo(data.updated_at, ft)}
            </span>
          )}
          <IconButton
            size="sm"
            variant="ghost"
            label={collapsed ? t('tripMapExpand') : t('tripMapCollapse')}
            icon={collapsed ? <ChevronDown size={18} /> : <ChevronUp size={18} />}
          />
        </div>
      </button>

      {!collapsed && (
        <div className="stack-sm">
          {ride.viewer.is_driver && <ShareLocation />}
          {points.length > 0 ? (
            <LiveMap points={points} />
          ) : (
            !data?.sharing && <p className="muted small">{t('waitingLocation')}</p>
          )}

          {ride.viewer.is_driver && waiting && (
            data?.passenger ? (
              <p className="small row nowrap">
                <Hand size={14} aria-hidden="true" />
                <span>
                  {ride.passenger.name} {t('passengerSharing')} {fmtAgo(data.passenger.updated_at, ft)}
                </span>
              </p>
            ) : (
              <p className="muted small">{t('passengerNotSharing')}</p>
            )
          )}

          {ride.viewer.is_passenger && waiting && (
            <PassengerShareLocation
              rideId={ride.id}
              sharing={!!data?.passenger}
              onChange={reload}
            />
          )}
          <p className="muted tiny">{t('shareInfo')}</p>
        </div>
      )}
    </Card>
  );
}

/** Ask the browser for location once, so the permission prompt shows before anything starts. */
export function requestPosition(): Promise<GeolocationPosition> {
  return new Promise((resolve, reject) => {
    if (!('geolocation' in navigator)) { reject(new Error('unsupported')); return; }
    navigator.geolocation.getCurrentPosition(resolve, reject, { enableHighAccuracy: true, timeout: 15000, maximumAge: 10000 });
  });
}

const sendDriverPosition = (pos: GeolocationPosition) =>
  post('/driver/location', { lat: pos.coords.latitude, lng: pos.coords.longitude, heading: pos.coords.heading }).catch(() => {});

/**
 * "Start trip" for drivers: explains that location is shared automatically, asks the
 * browser for permission, then starts the ride and sends the first position at once.
 */
export function StartTripButton({
  ride,
  onStarted,
  className = 'btn primary',
  label,
}: {
  ride: { id: number };
  onStarted: () => void;
  className?: string;
  label: string;
}) {
  const { t } = useTranslation('rideDetail');
  const { busy, error, run } = useAction();
  const [open, setOpen] = useState(false);
  const [locErr, setLocErr] = useState<'denied' | 'unsupported' | null>(null);

  const start = async (pos: GeolocationPosition | null) => {
    const ok = await run(async () => {
      await post(`/rides/${ride.id}/start`);
      if (pos) await sendDriverPosition(pos);
    }, pos ? t('respondedStarted') : t('startedNoLocation'));
    if (ok) { setOpen(false); onStarted(); }
  };

  const allow = async () => {
    setLocErr(null);
    try {
      await start(await requestPosition());
    } catch (e) {
      setLocErr((e as Error).message === 'unsupported' ? 'unsupported' : 'denied');
    }
  };

  return (
    <>
      <button className={className} disabled={busy} onClick={() => { setLocErr(null); setOpen(true); }}>
        {label}
      </button>
      {open && (
        <Modal title={t('startPromptTitle')} onClose={() => setOpen(false)}>
          <p>{t('startPromptBody')}</p>
          <ul className="small stack-sm">
            <li>{t('startPromptWho')}</li>
            <li>{t('startPromptWhen')}</li>
            <li>{t('startPromptPassenger')}</li>
          </ul>
          {locErr && <div className="notice">{locErr === 'unsupported' ? t('sharingUnsupported') : t('sharingDenied')}</div>}
          <ErrorBox error={error} />
          <Button variant="primary" block disabled={busy} onClick={allow} icon={<MapPin size={18} />}>
            {locErr ? t('tryAgain') : t('allowAndStart')}
          </Button>
          {locErr && (
            <Button variant="secondary" block disabled={busy} onClick={() => start(null)}>
              {t('startWithoutLocation')}
            </Button>
          )}
        </Modal>
      )}
    </>
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
        sendDriverPosition(pos);
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

const shareKey = (rideId: number) => `gr-share-loc-${rideId}`;
const readShare = (rideId: number) => { try { return localStorage.getItem(shareKey(rideId)) === '1'; } catch { return false; } };
const writeShare = (rideId: number, on: boolean) => {
  try { if (on) localStorage.setItem(shareKey(rideId), '1'); else localStorage.removeItem(shareKey(rideId)); } catch { /* storage blocked */ }
};

function PassengerShareLocation({ rideId, sharing, onChange }: { rideId: number; sharing: boolean; onChange: () => void }) {
  const { t } = useTranslation('rideDetail');
  const { run } = useAction();
  const [on, setOn] = useState(() => readShare(rideId));
  const [state, setState] = useState<'idle' | 'denied' | 'unsupported' | null>(null);
  const last = useRef(0);

  useEffect(() => {
    if (!on) return;
    if (!('geolocation' in navigator)) { setState('unsupported'); setOn(false); return; }
    setState('idle');
    last.current = 0;
    const watch = navigator.geolocation.watchPosition(
      (pos) => {
        setState(null);
        if (Date.now() - last.current < 15000) return;
        last.current = Date.now();
        post<{ sharing: boolean }>(`/rides/${rideId}/passenger-location`, { lat: pos.coords.latitude, lng: pos.coords.longitude })
          .then((res) => {
            if (!res.sharing) { writeShare(rideId, false); setOn(false); }
            onChange();
          })
          .catch(() => {});
      },
      () => { setState('denied'); writeShare(rideId, false); setOn(false); },
      { enableHighAccuracy: true, maximumAge: 10000 },
    );
    return () => navigator.geolocation.clearWatch(watch);
  }, [on, rideId]); // eslint-disable-line react-hooks/exhaustive-deps

  const toggle = () => {
    const next = !on;
    writeShare(rideId, next);
    setOn(next);
    if (!next) {
      setState(null);
      run(() => del(`/rides/${rideId}/passenger-location`).then(onChange));
    }
  };

  return (
    <div className={`notice ${on && sharing ? 'ok' : 'info'} stack-sm`}>
      <div>{on ? (sharing ? t('passengerShareOn') : t('sharingIdle')) : t('passengerSharePrompt')}</div>
      {state === 'denied' && <div className="small">{t('passengerShareDenied')}</div>}
      {state === 'unsupported' && <div className="small">{t('sharingUnsupported')}</div>}
      <Button
        size="sm"
        variant={on ? 'secondary' : 'primary'}
        onClick={toggle}
        icon={<MapPin size={16} />}
      >
        {on ? t('passengerShareStop') : t('passengerShareStart')}
      </Button>
    </div>
  );
}

function PassengerActions({ ride: r, reload }: { ride: Ride; reload: () => void }) {
  const { t } = useTranslation('rideDetail');
  const confirm = useConfirm();
  const { busy, error, run } = useAction();
  const nav = useNavigate();
  const [cancel, setCancel] = useState(false);
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

  return (
    <Card className="stack">
      {r.status === 'pending_passenger' && r.driver && (
        <div className="stack">
          <h3>{t('driverInvite')}</h3>
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
      <ErrorBox error={error} />
      {cancellable && (
        <Button variant="danger" onClick={() => setCancel(true)}>
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
    </Card>
  );
}

function DriverActions({ ride: r, reload }: { ride: Ride; reload: () => void }) {
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
      <h3>{t('driverActions')}</h3>
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
            <Button variant="danger" disabled={busy} onClick={handleNoShow}>
              {t('noShowBtn')}
            </Button>
          )}
          <Button variant="secondary" className="btn accent" disabled={busy} onClick={() => act('complete', t('respondedCompleted'))}>
            {t('completedTrip')}
          </Button>
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

function StaffActions({ ride: r, reload }: { ride: Ride; reload: () => void }) {
  const { t } = useTranslation('rideDetail');
  const confirm = useConfirm();
  const { busy, error, run } = useAction();
  const [modal, setModal] = useState<'reassign' | 'return' | 'alt' | 'cancel' | null>(null);
  const drivers = useApi<DriverOpt[]>(modal === 'reassign' ? '/admin/drivers' : null);
  const [driverId, setDriverId] = useState('');
  const open = !['completed', 'cancelled', 'no_show', 'alt_transport', 'in_progress'].includes(r.status);

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
      <h3>{t('staffActions')}</h3>
      {r.driver_load && (
        <p className="small">{t('driverLoad', { used: r.driver_load.seats_used, capacity: r.driver_load.capacity, seats: r.seats })}</p>
      )}
      {!!r.needs_attention && r.attention_reason && (
        <Notice tone="warning" icon={<AlertTriangle size={16} />}>
          <span>{r.attention_reason}</span>
        </Notice>
      )}
      {r.assistance && <Notice tone="info">{t('assistanceLabel', { assistance: r.assistance })}</Notice>}
      <div className="btn-row">
        {r.status === 'pending_approval' && (
          <Button variant="primary" disabled={busy} onClick={() => run(() => post(`/rides/${r.id}/approve`).then(reload), t('respondedAccepted'))}>
            {t('approve')}
          </Button>
        )}
        {open && <Button variant="secondary" onClick={() => setModal('reassign')}>{t('reassign')}</Button>}
        {['pending_approval', 'pending_passenger', 'pending_driver_confirm'].includes(r.status) && (
          <Button variant="secondary" onClick={() => setModal('return')}>{t('retur')}</Button>
        )}
        {open && <Button variant="secondary" onClick={() => setModal('alt')}>{t('altTransport')}</Button>}
        {open && <Button variant="danger" onClick={() => setModal('cancel')}>{t('cancel')}</Button>}
        {r.status === 'in_progress' && (
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
                <option key={d.id} value={d.id}>{d.name} · {d.areas || t('noVehicle')} · {d.vehicle?.model ?? t('noVehicle')}{d.carpool_enabled ? t('carpool', { seats: d.spare_seats }) : ''}</option>
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

function MobileStickyActions({ ride: r, reload }: { ride: Ride; reload: () => void }) {
  const { t } = useTranslation('rideDetail');
  const [showMore, setShowMore] = useState(false);

  // Determine primary action based on status and role
  if (r.viewer.is_passenger && r.status === 'pending_passenger') {
    return (
      <div className="ride-detail-sticky-bar">
        <PassengerActions ride={r} reload={reload} />
      </div>
    );
  }

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
          {r.viewer.is_passenger && <PassengerActions ride={r} reload={reload} />}
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

export function ReasonModal({
  title,
  label,
  confirm,
  initial = '',
  error,
  onClose,
  onSubmit,
}: {
  title: string;
  label: string;
  confirm: string;
  initial?: string;
  error?: string | null;
  onClose: () => void;
  onSubmit: (reason: string) => Promise<boolean>;
}) {
  const [reason, setReason] = useState(initial);
  return (
    <Modal title={title} onClose={onClose}>
      <Field label={label}>
        <textarea
          className="input"
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          autoFocus
          rows={3}
        />
      </Field>
      <ErrorBox error={error} />
      <Button
        variant="primary"
        block
        onClick={async () => {
          if (await onSubmit(reason)) onClose();
        }}
      >
        {confirm}
      </Button>
    </Modal>
  );
}
