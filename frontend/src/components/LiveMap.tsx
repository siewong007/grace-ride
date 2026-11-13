import { useEffect } from 'react';
import { MapContainer, Marker, Polyline, Popup, TileLayer, useMap } from 'react-leaflet';
import L from 'leaflet';
import 'leaflet/dist/leaflet.css';

type MarkerKind = 'car' | 'person' | 'stop' | 'dest';

const cssVar = (name: string) =>
  (typeof window !== 'undefined' && getComputedStyle(document.documentElement).getPropertyValue(name).trim()) ||
  `var(${name})`;

// Car marker as an inline SVG (avoids Leaflet's default image paths under Vite).
// The car is the app's blue; passengers and pickup stops get a green pin and the
// destination an amber flag, so the three never look alike (status colours, not --accent,
// which is the same blue as the car).
// A car whose location is out of date turns amber too (admin live map legend).
const getMarkerStyles = (kind: MarkerKind, stale = false) => ({
  bg: cssVar(kind === 'car' ? (stale ? '--warning' : '--primary') : kind === 'dest' ? '--warning' : '--success'),
  ink: cssVar('--primary-ink'),
});

const CAR_SVG = '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 17h2c.6 0 1-.4 1-1v-3c0-.9-.7-1.7-1.5-1.9C18.7 10.6 16 10 16 10s-1.3-1.4-2.2-2.3c-.5-.4-1.1-.7-1.8-.7H5c-.6 0-1.1.4-1.4.9l-1.5 2.8C2.1 11 2 11.5 2 12v4c0 .6.4 1 1 1h2"/><circle cx="7" cy="17" r="2"/><path d="M9 17h6"/><circle cx="17" cy="17" r="2"/></svg>';
const HAND_SVG = '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 11V6a2 2 0 0 0-2-2v0a2 2 0 0 0-2 2v0"/><path d="M14 10V4a2 2 0 0 0-2-2v0a2 2 0 0 0-2 2v2"/><path d="M10 10.5V6a2 2 0 0 0-2-2v0a2 2 0 0 0-2 2v8"/><path d="M18 8a2 2 0 1 1 4 0v6a8 8 0 0 1-8 8h-2c-2.8 0-4.5-.86-5.99-2.34l-3.6-3.6a2 2 0 0 1 2.83-2.82L7 15"/></svg>';

const FLAG_SVG = '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 15s1-1 4-1 5 2 8 2 4-1 4-1V3s-1 1-4 1-5-2-8-2-4 1-4 1z"/><line x1="4" x2="4" y1="22" y2="15"/></svg>';

const markerIcon = (label: string | undefined, kind: MarkerKind, badge?: string, stale?: boolean) => {
  const { bg, ink } = getMarkerStyles(kind, stale);
  const iconContent = badge
    ? escapeHtml(badge)
    : kind === 'dest'
      ? FLAG_SVG
      : label
        ? escapeHtml(label.slice(0, 1))
        : kind === 'person' || kind === 'stop'
          ? HAND_SVG
          : CAR_SVG;
  return L.divIcon({
    className: '',
    iconSize: [44, 44],
    iconAnchor: [22, 22],
    html: `<div style="display:grid;place-items:center;width:44px;height:44px;border-radius:50%;background:${bg};color:${ink};border:3px solid ${ink};box-shadow:0 2px 8px rgba(0,0,0,.35);font-weight:700;font-size:15px;font-family:sans-serif">${iconContent}</div>`,
  });
};

const escapeHtml = (s: string) => s.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);

export interface MapPoint {
  id: number | string;
  lat: number;
  lng: number;
  label: string;
  detail?: string;
  kind?: MarkerKind;
  /** Short text shown inside the pin instead of the label's first letter (e.g. stop number). */
  badge?: string;
  /** Location not updated recently: the car pin is drawn amber. */
  stale?: boolean;
}

/** Route line as [lat, lng] pairs. */
export type RouteLine = [number, number][];

function Fit({
  points,
  route,
  focusPoint,
}: {
  points: MapPoint[];
  route?: RouteLine;
  focusPoint?: { lat: number; lng: number } | null;
}) {
  const map = useMap();
  const all: [number, number][] = [...points.map((p): [number, number] => [p.lat, p.lng]), ...(route ?? [])];
  useEffect(() => {
    if (focusPoint) {
      map.setView([focusPoint.lat, focusPoint.lng], 16, { animate: true });
      return;
    }
    if (all.length === 1) map.setView(all[0], 15);
    else if (all.length > 1) map.fitBounds(L.latLngBounds(all), { padding: [40, 40] });
  }, [map, focusPoint?.lat, focusPoint?.lng, all.map((p) => p.join(',')).join('|')]); // eslint-disable-line react-hooks/exhaustive-deps
  return null;
}

// Free OpenStreetMap tiles — no Google Maps bill. Swap the TileLayer URL for another provider if needed.
export default function LiveMap({
  points,
  tall,
  focusPoint,
  route,
}: {
  points: MapPoint[];
  tall?: boolean;
  focusPoint?: { lat: number; lng: number } | null;
  /** Suggested driving route, drawn under the markers. */
  route?: RouteLine;
}) {
  const center: [number, number] = points[0] ? [points[0].lat, points[0].lng] : [3.1073, 101.6067]; // Petaling Jaya
  return (
    <MapContainer center={center} zoom={13} className={`map ${tall ? 'tall' : ''}`} scrollWheelZoom={false}>
      <TileLayer attribution='&copy; OpenStreetMap contributors' url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png" />
      {route && route.length > 1 && (
        <Polyline positions={route} pathOptions={{ color: cssVar('--primary'), weight: 5, opacity: 0.75 }} />
      )}
      {points.map((p) => (
        <Marker key={p.id} position={[p.lat, p.lng]} icon={markerIcon(p.label, p.kind ?? 'car', p.badge, p.stale)}>
          <Popup>
            <strong>{p.label}</strong>
            {p.detail && <div>{p.detail}</div>}
          </Popup>
        </Marker>
      ))}
      <Fit points={points} route={route} focusPoint={focusPoint} />
    </MapContainer>
  );
}
