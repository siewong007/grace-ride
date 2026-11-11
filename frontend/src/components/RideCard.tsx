import { Link } from 'react-router-dom';
import { AlertTriangle } from 'lucide-react';
import { fmtDay, fmtTime, RIDE_STATUS, useFormatT } from '../format';
import { StatusPill } from './ui';
import { useTranslation } from 'react-i18next';

export interface Ride {
  id: number;
  status: string;
  pickup_at: string;
  area: { id: number; name: string };
  pickup_address: string | null;
  destination: string;
  seats: number;
  assistance: string;
  notes: string;
  event: { id: number; title: string } | null;
  trip_id: number | null;
  passenger: { id: number; name: string; phone: string | null };
  driver: { id: number; name: string; phone: string | null; vehicle: { model: string; color: string; plate: string } | null } | null;
  needs_attention: boolean;
  attention_reason: string;
  started_at: string | null;
  picked_up_at: string | null;
  completed_at: string | null;
  updated_at: string;
  viewer: { is_passenger: boolean; is_driver: boolean; is_staff: boolean };
  driver_load?: { seats_used: number; capacity: number };
  passenger_conflicts?: { id: number; pickup_at: string }[];
}

export default function RideCard({ ride, to, perspective = 'passenger' }: { ride: Ride; to?: string; perspective?: 'passenger' | 'driver' | 'staff' }) {
  const ft = useFormatT();
  const { t } = useTranslation('rideCard');
  const who =
    perspective === 'passenger'
      ? ride.driver ? t('driver', { name: ride.driver.name }) : t('waitingDriver')
      : perspective === 'driver'
        ? t('passenger', { name: ride.passenger.name })
        : `${ride.passenger.name} → ${ride.driver?.name ?? t('noDriver')}`;
  return (
    <Link to={to ?? `/rides/${ride.id}`} className="card link tight">
      <div className="row nowrap">
        <div className="stack-sm grow">
          <div className="row between">
            <span className="bold">{fmtDay(ride.pickup_at, ft)} · {fmtTime(ride.pickup_at, ft)}</span>
            <StatusPill map={RIDE_STATUS} status={ride.status} />
          </div>
          <div className="muted small">
            {ride.area.name} → {ride.destination} · {ride.seats} {t('seats')}{ride.event ? ` · ${ride.event.title}` : ''}
          </div>
          <div className="small">{who}</div>
          {ride.needs_attention && (
            <div className="small row nowrap" style={{ color: 'var(--accent)', gap: 'var(--space-1)' }}>
              <AlertTriangle size={14} aria-hidden="true" />
              <span>{ride.attention_reason || t('needsAttention')}</span>
            </div>
          )}
        </div>
      </div>
    </Link>
  );
}
