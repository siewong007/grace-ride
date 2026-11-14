import { useEffect, useRef, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import {
  AlertCircle,
  AlertTriangle,
  Calendar,
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
  Share2,
} from 'lucide-react';
import { downloadIcs } from '../calendar';
import { api, del, post, useApi } from '../api';
import { useAuth } from '../auth';
import Chat from '../components/Chat';
import LiveMap, { MapPoint } from '../components/LiveMap';
import { Ride } from '../components/RideCard';
import type { RideEta } from '../components/TripRoute';
import {
  NavButtonGroup,
  ReasonModal,
  ShareLocation,
  StartTripButton,
  requestPosition,
  sendDriverPosition,
} from '../components/TripActions';
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
  Pill,
  Spinner,
  StatusPill,
  useAction,
  useConfirm,
} from '../components/ui';
import {
  fmtAgo,
  fmtDateTime,
  fmtDistance,
  fmtDuration,
  fmtTime,
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
    <Card className="ride-driver">
      <div className="row nowrap gap-md min-w-0">
        <Avatar name={d.name} size="lg" />
        <div className="min-w-0">
          <div className="muted small">
            {r.viewer.is_driver ? t('youAreDriver') : t('driver')}
          </div>
          <div className="bold font-lg">{d.name}</div>
          {d.vehicle && (
            <div className="small muted ride-driver-car">
              {d.vehicle.color} {d.vehicle.model} <span className="plate">{d.vehicle.plate}</span>
            </div>
          )}
        </div>
      </div>

      <div className="btn-row ride-driver-actions">
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
        <h2>{t('statusTimeline')}</h2>
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
      <h2>{t('statusTimeline')}</h2>
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

  const kvItems: KeyValueItem[] = [];

  if (r.is_proxy && r.proxy) {
    kvItems.push({
      label: t('proxyRider'),
      value: (
        <span>
          <strong className="bold">{r.proxy.rider_name}</strong>
          {r.proxy.rider_phone ? (
            <>
              {' '}
              · <a href={`tel:${r.proxy.rider_phone}`}>{r.proxy.rider_phone}</a>
            </>
          ) : (
            <span className="small muted"> ({t('proxyNoPhone')})</span>
          )}
        </span>
      ),
    });
    if (r.proxy.rider_relationship) {
      kvItems.push({
        label: t('proxyRelationship'),
        value: r.proxy.rider_relationship,
      });
    }
    kvItems.push({
      label: t('proxyBooker'),
      value: (
        <span>
          {r.proxy.booker_name}
          {r.proxy.booker_phone && (
            <>
              {' '}
              · <a href={`tel:${r.proxy.booker_phone}`}>{r.proxy.booker_phone}</a>
            </>
          )}
        </span>
      ),
    });
  } else {
    kvItems.push({
      label: t('passenger'),
      value: (
        <span>
          {r.passenger.name}
          {r.passenger.phone && !r.viewer.is_passenger && (
            <>
              {' '}
              · <a href={`tel:${r.passenger.phone}`}>{r.passenger.phone}</a>
            </>
          )}
        </span>
      ),
    });
  }

  kvItems.push(
    { label: t('pickupArea'), value: r.area.name },
    {
      label: t('pickupAddress'),
      value:
        r.pickup_address ? (
          <span>{r.pickup_address}</span>
        ) : (
          <div className="row between nowrap">
            <span className="muted">{t('pickupAddressLocked')}</span>
            <Lock size={14} className="muted" aria-hidden="true" />
          </div>
        ),
    },
    { label: t('destination'), value: r.destination },
    { label: t('people'), value: t('seatCount', { count: r.seats }) },
  );

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
        <NavButtonGroup address={r.pickup_address} />
      )}

      {['confirmed', 'in_progress', 'completed'].includes(r.status) && (
        <div className="btn-row">
          <Button
            variant="ghost"
            size="sm"
            icon={<Calendar size={16} />}
            onClick={() => {
              const d = r.driver;
              downloadIcs(
                {
                  title: `Grace Ride: ${r.destination}`,
                  description: `Driver: ${d?.name ?? ''} (${[d?.vehicle?.color, d?.vehicle?.model, d?.vehicle?.plate].filter(Boolean).join(' ')})\nPickup: ${r.pickup_address || r.area.name}`,
                  location: r.pickup_address || r.area.name,
                  startsAt: r.pickup_at,
                  durationMinutes: 60,
                },
                `GraceRide-${r.id}.ics`,
              );
            }}
          >
            {t('addToCalendar')}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            icon={<Share2 size={16} />}
            onClick={() => {
              const d = r.driver;
              const text = t('shareRideText', {
                time: fmtDateTime(r.pickup_at, ft),
                pickup: r.pickup_address || r.area.name,
                dest: r.destination,
                driver: d
                  ? `${d.name} (${[d.vehicle?.color, d.vehicle?.model, d.vehicle?.plate].filter(Boolean).join(' ')})`
                  : '—',
              });
              window.open(`https://wa.me/?text=${encodeURIComponent(text)}`, '_blank', 'noopener,noreferrer');
            }}
          >
            {t('shareWhatsApp')}
          </Button>
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
  /** Distance/ETA to this passenger's pickup, or to the destination once aboard. */
  eta?: RideEta;
}

/** "Your driver is about 8 min away · 3.2 km · arriving around 10:12". */
function EtaSummary({ ride, eta }: { ride: Ride; eta: RideEta }) {
  const { t } = useTranslation('rideDetail');
  const ft = useFormatT();
  const driverView = ride.viewer.is_driver || (ride.viewer.is_staff && !ride.viewer.is_passenger);
  if (!eta.located) {
    if (eta.phase === 'dropoff') return null;
    return (
      <Notice tone="info" icon={<MapPin size={18} />}>
        {driverView ? t('etaNoAddressDriver') : t('etaNoAddress')}
      </Notice>
    );
  }
  if (eta.duration_s == null || eta.distance_m == null) return null;
  const duration = fmtDuration(eta.duration_s, ft);
  const title =
    eta.phase === 'dropoff'
      ? t('etaArriving', { duration, place: eta.destination ?? ride.destination })
      : driverView
        ? t('etaToPassenger', { duration, name: ride.passenger.name })
        : t('etaDriverAway', { duration });
  return (
    <div className="eta-summary" role="status">
      <Clock size={20} aria-hidden="true" />
      <div className="stack-sm">
        <p className="eta-summary-title">{title}</p>
        <p className="small">
          {t('etaDetail', { distance: fmtDistance(eta.distance_m, ft), time: eta.eta ? fmtTime(eta.eta, ft) : '–' })}
        </p>
        {eta.stops_before > 0 && (
          <p className="small">
            {eta.phase === 'dropoff'
              ? t('etaStopsOnWay', { n: eta.stops_before })
              : t('etaStopsBefore', { n: eta.stops_before })}
          </p>
        )}
        <p className="muted tiny">{eta.approximate ? t('etaApproximate') : t('etaNoTraffic')}</p>
      </div>
    </div>
  );
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
  const eta = data?.eta;
  // Where the driver is heading: the pickup point (unless the passenger's live spot is
  // already on the map) or, once aboard, the destination.
  if (eta?.located && eta.target && (eta.phase === 'dropoff' || !data?.passenger)) {
    const dropoff = eta.phase === 'dropoff';
    points.push({
      id: 'target',
      kind: dropoff ? 'dest' : 'stop',
      lat: eta.target[0],
      lng: eta.target[1],
      label: dropoff ? eta.destination ?? ride.destination : ride.passenger.name,
      detail: eta.eta ? fmtTime(eta.eta, ft) : undefined,
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
          <h2 className="no-margin">
            {ride.viewer.is_driver ? t('tripMap') : t('driverLocation')}
          </h2>
        </div>
        <div className="row nowrap gap-sm">
          {eta?.located && eta.duration_s != null && (
            <Pill tone="info" icon={<Clock size={12} />}>
              {fmtDuration(eta.duration_s, ft)}
            </Pill>
          )}
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
          {eta && <EtaSummary ride={ride} eta={eta} />}
          {points.length > 0 ? (
            <LiveMap points={points} route={eta?.path} />
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

export { StartTripButton, ShareLocation, requestPosition, sendDriverPosition };

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
  const auth = useAuth();
  const can = auth?.can ?? (() => true);
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
                onClick={async () => {
                  const ok = await confirm({
                    title: t('confirmUnableTitle'),
                    body: t('confirmUnableBody'),
                    confirmLabel: t('confirmUnableBtn'),
                    tone: 'danger',
                  });
                  if (ok) {
                    await run(() => post(`/rides/${r.id}/readiness`, { status: 'unable_to_join' }).then(reload));
                  }
                }}
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
      {cancellable && can('rides', 'delete') && (
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
              variant="ghost"
              size="sm"
              disabled={busy}
              onClick={() => run(() => post(`/rides/${r.id}/driver-delay`, { delay_minutes: 5 }).then(reload), t('driverDelaySent', { mins: 5 }))}
            >
              {t('driverDelay5m')}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              disabled={busy}
              onClick={() => run(() => post(`/rides/${r.id}/driver-delay`, { delay_minutes: 10 }).then(reload), t('driverDelaySent', { mins: 10 }))}
            >
              {t('driverDelay10m')}
            </Button>
            <Button
              variant="ghost"
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

function StaffActions({ ride: r, reload }: { ride: Ride; reload: () => void }) {
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
        {edit && r.status === 'in_progress' && (
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

export { ReasonModal };
