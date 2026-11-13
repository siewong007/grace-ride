import { useEffect, useRef } from 'react';
import { Link } from 'react-router-dom';
import { AlertTriangle, Flag, MapPin, Navigation, Route } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useApi } from '../api';
import { fmtDistance, fmtDuration, fmtTime, useFormatT } from '../format';
import LiveMap, { MapPoint, RouteLine } from './LiveMap';
import { Card, Pill } from './ui';

/** One pickup in the driver's suggested route (see backend `routing::Stop`). */
export interface RouteStop {
  ride_id: number;
  passenger: string;
  address: string;
  pickup_at: string;
  seats: number;
  started: boolean;
  lat: number | null;
  lng: number | null;
  /** live = passenger's shared GPS; address / approx_address = geocoded; null = not found. */
  location: 'live' | 'address' | 'approx_address' | null;
  leg_distance_m: number | null;
  leg_duration_s: number | null;
  distance_m: number | null;
  duration_s: number | null;
  eta: string | null;
}

export interface TripRoute {
  trip_id: number;
  live: boolean;
  origin: { lat: number; lng: number; updated_at: string } | null;
  stops: RouteStop[];
  onboard: number;
  destination: {
    name: string;
    lat: number | null;
    lng: number | null;
    distance_m: number | null;
    duration_s: number | null;
    eta: string | null;
  };
  distance_m: number;
  duration_s: number;
  approximate: boolean;
  computed_at: string;
  geometry: RouteLine;
}

/** A rider's own distance/ETA, returned with `/rides/{id}/location`. */
export interface RideEta {
  phase: 'pickup' | 'dropoff';
  located: boolean;
  stops_before: number;
  approximate: boolean;
  computed_at: string;
  distance_m?: number | null;
  duration_s?: number | null;
  eta?: string | null;
  destination?: string;
  location?: RouteStop['location'];
  target?: [number, number];
  path?: RouteLine;
}

type Place = Pick<RouteStop, 'lat' | 'lng' | 'location' | 'address'>;

/**
 * What to hand to Google Maps / Waze for a stop: exact GPS when the passenger is sharing
 * it, otherwise the address text (their geocoders know Malaysian addresses better than ours).
 */
const placeParam = (s: Place) =>
  s.location === 'live' && s.lat != null && s.lng != null ? `${s.lat},${s.lng}` : s.address;

/** Google Maps directions from the phone's current position through every stop, in order. */
export function googleRouteUrl(stops: Place[], destination: string) {
  const p = new URLSearchParams({ api: '1', travelmode: 'driving', destination });
  if (stops.length) p.set('waypoints', stops.map(placeParam).join('|'));
  return `https://www.google.com/maps/dir/?${p}`;
}

export function googleStopUrl(s: Place) {
  return `https://www.google.com/maps/dir/?${new URLSearchParams({ api: '1', travelmode: 'driving', destination: placeParam(s) })}`;
}

/** Waze links take a single destination, so navigation goes one stop at a time. */
export function wazeStopUrl(s: Place) {
  const where =
    s.location === 'live' && s.lat != null && s.lng != null
      ? `ll=${s.lat},${s.lng}`
      : `q=${encodeURIComponent(s.address)}`;
  return `https://www.waze.com/ul?${where}&navigate=yes`;
}

/** Google Maps in a phone browser accepts only 3 waypoints in a directions link. */
const MOBILE_WAYPOINT_LIMIT = 3;

/**
 * The driver's current (or next) trip: pickups in the best order, the route on the map,
 * and one-tap hand-off to Google Maps (all stops) or Waze (next stop). Renders nothing
 * when there's no trip to drive.
 */
export function DriverRouteCard({ refreshKey }: { refreshKey?: string }) {
  const { t } = useTranslation('drive');
  const ft = useFormatT();
  const { data, reload } = useApi<{ route: TripRoute | null }>('/driver/route', 20000);

  // Refresh when the schedule changes (pickup recorded, trip started, …).
  const first = useRef(true);
  useEffect(() => {
    if (first.current) {
      first.current = false;
      return;
    }
    reload();
  }, [refreshKey]); // eslint-disable-line react-hooks/exhaustive-deps

  const route = data?.route;
  if (!route) return null;

  const located = route.stops.filter((s) => s.location != null);
  const next = route.stops[0];
  const dest = route.destination;
  const destPlace: Place = { lat: dest.lat, lng: dest.lng, location: null, address: dest.name };

  const points: MapPoint[] = [];
  if (route.origin) {
    points.push({ id: 'driver', kind: 'car', lat: route.origin.lat, lng: route.origin.lng, label: '' });
  }
  route.stops.forEach((s, i) => {
    if (s.lat != null && s.lng != null) {
      points.push({
        id: `stop-${s.ride_id}`,
        kind: 'stop',
        badge: String(i + 1),
        lat: s.lat,
        lng: s.lng,
        label: `${i + 1}. ${s.passenger}`,
        detail: s.eta ? fmtTime(s.eta, ft) : undefined,
      });
    }
  });
  if (dest.lat != null && dest.lng != null) {
    points.push({
      id: 'destination',
      kind: 'dest',
      lat: dest.lat,
      lng: dest.lng,
      label: dest.name,
      detail: dest.eta ? fmtTime(dest.eta, ft) : undefined,
    });
  }

  const distance = fmtDistance(route.distance_m, ft);
  const duration = fmtDuration(route.duration_s, ft);

  return (
    <Card className="stack trip-route-card" aria-label={t('routeTitle')}>
      <div className="row between nowrap trip-route-head">
        <div className="trip-route-col">
          <h3 className="row nowrap trip-route-title">
            <Route size={18} aria-hidden="true" />
            <span>{t('routeTitle')}</span>
          </h3>
          <span className="muted small">
            {route.stops.length > 0
              ? t('routeSummary', { count: route.stops.length, distance, duration })
              : t('routeAllAboard', { distance, duration })}
          </span>
          {dest.eta && (
            <span className="small bold">
              {t('routeArrive', { place: dest.name, time: fmtTime(dest.eta, ft) })}
            </span>
          )}
        </div>
        <Pill tone={route.live ? 'success' : 'neutral'}>{route.live ? t('routeLive') : t('routePlanned')}</Pill>
      </div>

      <div className="trip-route-nav">
        <a
          className="btn primary block"
          target="_blank"
          rel="noreferrer"
          href={googleRouteUrl([...located, ...route.stops.filter((s) => s.location == null)], dest.name)}
        >
          <Navigation size={16} aria-hidden="true" />
          <span>{t('routeOpenGoogle')}</span>
        </a>
        <a className="btn secondary block" target="_blank" rel="noreferrer" href={wazeStopUrl(next ?? destPlace)}>
          <Navigation size={16} aria-hidden="true" />
          <span>{next ? t('routeNextWaze', { name: next.passenger }) : t('routeDestWaze')}</span>
        </a>
      </div>
      {route.stops.length > MOBILE_WAYPOINT_LIMIT && <p className="muted tiny">{t('routeGoogleLimit')}</p>}

      {points.length > 0 && <LiveMap points={points} route={route.geometry} />}

      <ol className="trip-route-stops" aria-label={t('routeStopsLabel')}>
        {route.stops.map((s, i) => (
          <li key={s.ride_id} className="trip-route-stop">
            <span className={`trip-route-badge ${s.location == null ? 'unknown' : ''}`} aria-hidden="true">
              {i + 1}
            </span>
            <div className="grow trip-route-col trip-route-body">
              <div className="row between nowrap">
                <Link to={`/rides/${s.ride_id}`} className="bold">
                  {s.passenger}
                  {s.seats > 1 && (
                    <span className="muted small">
                      {' '}
                      · {t('seatCount', { count: s.seats })}
                    </span>
                  )}
                </Link>
                {s.eta && <span className="trip-route-eta">{fmtTime(s.eta, ft)}</span>}
              </div>
              <span className="muted small">{s.address}</span>
              <div className="trip-route-meta small">
                {s.leg_distance_m != null && s.leg_duration_s != null && (
                  <span>
                    {fmtDistance(s.leg_distance_m, ft)} · {fmtDuration(s.leg_duration_s, ft)}{' '}
                    <span className="muted">{i === 0 && route.origin ? t('routeFromYou') : t('routeFromPrev')}</span>
                  </span>
                )}
                {s.location === 'live' && (
                  <Pill tone="info" icon={<MapPin size={12} />}>
                    {t('routeLiveSpot')}
                  </Pill>
                )}
                {s.location === 'approx_address' && <Pill tone="warning">{t('routeApproxSpot')}</Pill>}
                {route.live && !s.started && <Pill tone="neutral">{t('routeNotStarted')}</Pill>}
              </div>
              {s.location == null && (
                <p className="small trip-route-warn">
                  <AlertTriangle size={14} aria-hidden="true" />
                  <span>{t('routeUnlocated')}</span>
                </p>
              )}
              <div className="nav-group-buttons">
                <a className="btn sm secondary" target="_blank" rel="noreferrer" href={googleStopUrl(s)}>
                  <Navigation size={14} aria-hidden="true" />
                  <span>{t('navigateGoogleMaps')}</span>
                </a>
                <a className="btn sm secondary" target="_blank" rel="noreferrer" href={wazeStopUrl(s)}>
                  <Navigation size={14} aria-hidden="true" />
                  <span>{t('navigateWaze')}</span>
                </a>
              </div>
            </div>
          </li>
        ))}
        <li className="trip-route-stop">
          <span className="trip-route-badge dest" aria-hidden="true">
            <Flag size={14} />
          </span>
          <div className="grow trip-route-col trip-route-body">
            <div className="row between nowrap">
              <span className="bold">{dest.name}</span>
              {dest.eta && <span className="trip-route-eta">{fmtTime(dest.eta, ft)}</span>}
            </div>
            <span className="muted small">
              {t('routeDestination')}
              {dest.distance_m != null && ` · ${fmtDistance(dest.distance_m, ft)}`}
            </span>
          </div>
        </li>
      </ol>

      {route.onboard > 0 && <p className="small muted">{t('routeOnboard', { n: route.onboard })}</p>}
      <p className="muted tiny">{route.approximate ? t('routeApproximate') : t('routeNoTraffic')}</p>
    </Card>
  );
}
