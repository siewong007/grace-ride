import { useEffect } from 'react';
import { MapContainer, Marker, Popup, TileLayer, useMap } from 'react-leaflet';
import L from 'leaflet';
import 'leaflet/dist/leaflet.css';

// Car marker as an inline SVG (avoids Leaflet's default image paths under Vite).
// Passengers get a blue pin so the driver can tell them apart from the car.
const getMarkerStyles = (kind: 'car' | 'person') => {
  if (typeof window === 'undefined') {
    return {
      bg: kind === 'person' ? 'var(--info)' : 'var(--primary)',
      ink: 'var(--primary-ink)',
    };
  }
  const cs = getComputedStyle(document.documentElement);
  const personColor = cs.getPropertyValue('--info').trim();
  const carColor = cs.getPropertyValue('--primary').trim();
  const inkColor = cs.getPropertyValue('--primary-ink').trim();
  return {
    bg: (kind === 'person' ? personColor : carColor) || (kind === 'person' ? 'var(--info)' : 'var(--primary)'),
    ink: inkColor || 'var(--primary-ink)',
  };
};

const markerIcon = (label: string | undefined, kind: 'car' | 'person') => {
  const { bg, ink } = getMarkerStyles(kind);
  return L.divIcon({
    className: '',
    iconSize: [44, 44],
    iconAnchor: [22, 22],
    html: `<div style="display:grid;place-items:center;width:44px;height:44px;border-radius:50%;background:${bg};color:${ink};border:3px solid ${ink};box-shadow:0 2px 8px rgba(0,0,0,.35);font-weight:700;font-size:15px;font-family:sans-serif">${
      label ? escapeHtml(label.slice(0, 1)) : kind === 'person' ? '🙋' : '🚗'
    }</div>`,
  });
};

const escapeHtml = (s: string) => s.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);

export interface MapPoint { id: number | string; lat: number; lng: number; label: string; detail?: string; kind?: 'car' | 'person' }

function Fit({ points }: { points: MapPoint[] }) {
  const map = useMap();
  useEffect(() => {
    if (points.length === 1) map.setView([points[0].lat, points[0].lng], 15);
    else if (points.length > 1) map.fitBounds(L.latLngBounds(points.map((p) => [p.lat, p.lng])), { padding: [40, 40] });
  }, [map, points.map((p) => `${p.lat},${p.lng}`).join('|')]); // eslint-disable-line react-hooks/exhaustive-deps
  return null;
}

// Free OpenStreetMap tiles — no Google Maps bill. Swap the TileLayer URL for another provider if needed.
export default function LiveMap({ points, tall }: { points: MapPoint[]; tall?: boolean }) {
  const center: [number, number] = points[0] ? [points[0].lat, points[0].lng] : [3.1073, 101.6067]; // Petaling Jaya
  return (
    <MapContainer center={center} zoom={13} className={`map ${tall ? 'tall' : ''}`} scrollWheelZoom={false}>
      <TileLayer attribution='&copy; OpenStreetMap contributors' url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png" />
      {points.map((p) => (
        <Marker key={p.id} position={[p.lat, p.lng]} icon={markerIcon(p.label, p.kind ?? 'car')}>
          <Popup>
            <strong>{p.label}</strong>
            {p.detail && <div>{p.detail}</div>}
          </Popup>
        </Marker>
      ))}
      <Fit points={points} />
    </MapContainer>
  );
}
