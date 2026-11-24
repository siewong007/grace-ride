import { useEffect, useRef, useState } from 'react';
import { ChevronDown, Clock, Hand, MapPin, Navigation } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { del, post, useApi } from '../../api';
import { fmtAgo, fmtDistance, fmtDuration, fmtTime, useFormatT } from '../../format';
import LiveMap, { MapPoint } from '../LiveMap';
import type { Ride } from '../RideCard';
import { useWakeLock } from '../../hooks/useWakeLock';
import { ShareLocation } from '../TripActions';
import type { RideEta } from '../TripRoute';
import { Button, Card, Notice, Pill, useAction } from '../ui';

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

export function CollapsibleTrackCard({ ride }: { ride: Ride }) {
  const { t } = useTranslation('rideDetail');
  const ft = useFormatT();
  const [collapsed, setCollapsed] = useState(false);
  const bodyRef = useRef<HTMLDivElement>(null);
  const { data, reload } = useApi<RideLocation>(`/rides/${ride.id}/location`, 10000);

  useEffect(() => {
    const el = bodyRef.current;
    if (el) el.inert = collapsed;
  }, [collapsed]);

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
          <span className="muted" aria-hidden="true">
            <ChevronDown size={18} className={`collapsible-chevron${collapsed ? '' : ' is-open'}`} />
          </span>
        </div>
      </button>

      <div
        ref={bodyRef}
        className={`collapsible-grid${collapsed ? '' : ' is-open'}`}
        aria-hidden={collapsed || undefined}
      >
        <div className="collapsible-grid-inner">
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
        </div>
      </div>
    </Card>
  );
}

const shareKey = (rideId: number) => `gr-share-loc-${rideId}`;
const readShare = (rideId: number) => {
  try {
    return localStorage.getItem(shareKey(rideId)) === '1';
  } catch {
    return false;
  }
};
const writeShare = (rideId: number, on: boolean) => {
  try {
    if (on) localStorage.setItem(shareKey(rideId), '1');
    else localStorage.removeItem(shareKey(rideId));
  } catch {
    /* storage blocked */
  }
};

export function PassengerShareLocation({
  rideId,
  sharing,
  onChange,
}: {
  rideId: number;
  sharing: boolean;
  onChange: () => void;
}) {
  const { t } = useTranslation('rideDetail');
  const { run } = useAction();
  const [on, setOn] = useState(() => readShare(rideId));
  useWakeLock(on);
  const [state, setState] = useState<'idle' | 'denied' | 'unsupported' | null>(null);
  const last = useRef(0);
  const onChangeRef = useRef(onChange);

  useEffect(() => {
    onChangeRef.current = onChange;
  }, [onChange]);

  useEffect(() => {
    if (!on) return;
    if (!('geolocation' in navigator)) {
      setState('unsupported');
      setOn(false);
      return;
    }
    setState('idle');
    last.current = 0;
    const watch = navigator.geolocation.watchPosition(
      (pos) => {
        setState(null);
        if (Date.now() - last.current < 15000) return;
        last.current = Date.now();
        post<{ sharing: boolean }>(`/rides/${rideId}/passenger-location`, {
          lat: pos.coords.latitude,
          lng: pos.coords.longitude,
        })
          .then((res) => {
            if (!res.sharing) {
              writeShare(rideId, false);
              setOn(false);
            }
            onChangeRef.current();
          })
          .catch(() => {});
      },
      () => {
        setState('denied');
        writeShare(rideId, false);
        setOn(false);
      },
      { enableHighAccuracy: true, maximumAge: 10000 },
    );
    return () => navigator.geolocation.clearWatch(watch);
  }, [on, rideId]);

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
