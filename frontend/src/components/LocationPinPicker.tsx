import { useEffect, useMemo } from 'react';
import { MapContainer, Marker, TileLayer, useMap, useMapEvents } from 'react-leaflet';
import L from 'leaflet';
import 'leaflet/dist/leaflet.css';

const DEFAULT_CENTER: [number, number] = [3.1073, 101.6067];

function makePinIcon() {
  return L.divIcon({
    className: '',
    iconSize: [32, 32],
    iconAnchor: [16, 16],
    html: '<div style="width:28px;height:28px;border-radius:50%;background:var(--primary);color:var(--primary-ink);border:3px solid var(--primary-ink);box-shadow:0 2px 8px rgba(0,0,0,.35);display:grid;place-items:center;font-weight:700;font-size:14px">●</div>',
  });
}

function MapEventsAndCenter({
  lat,
  lng,
  onPick,
}: {
  lat: number | null;
  lng: number | null;
  onPick: (lat: number, lng: number) => void;
}) {
  const map = useMap();

  useMapEvents({
    click(e) {
      onPick(Number(e.latlng.lat.toFixed(6)), Number(e.latlng.lng.toFixed(6)));
    },
  });

  useEffect(() => {
    if (lat != null && lng != null) {
      map.setView([lat, lng], Math.max(map.getZoom(), 15), { animate: true });
    }
  }, [map, lat, lng]);

  return null;
}

export default function LocationPinPicker({
  lat,
  lng,
  onPick,
}: {
  lat: number | null;
  lng: number | null;
  onPick: (lat: number, lng: number) => void;
}) {
  const icon = useMemo(() => makePinIcon(), []);
  const hasPin = lat != null && lng != null;
  const center: [number, number] = hasPin ? [lat, lng] : DEFAULT_CENTER;

  return (
    <div className="pin-picker-map">
      <MapContainer
        center={center}
        zoom={hasPin ? 15 : 13}
        style={{ height: '100%', width: '100%' }}
      >
        <TileLayer
          attribution="&copy; OpenStreetMap contributors"
          url="https://tile.openstreetmap.org/{z}/{x}/{y}.png"
          referrerPolicy="no-referrer"
        />
        <MapEventsAndCenter lat={lat} lng={lng} onPick={onPick} />
        {hasPin && (
          <Marker
            position={[lat, lng]}
            icon={icon}
            draggable
            eventHandlers={{
              dragend(e) {
                const pos = e.target.getLatLng();
                onPick(Number(pos.lat.toFixed(6)), Number(pos.lng.toFixed(6)));
              },
            }}
          />
        )}
      </MapContainer>
    </div>
  );
}
