import { Link } from 'react-router-dom';
import { AlertTriangle, Lock } from 'lucide-react';
import { fmtDay, fmtTime, RIDE_STATUS, useFormatT } from '../format';
import { Avatar, StatusPill } from './ui';
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

export default function RideCard({
  ride,
  to,
  perspective = 'passenger',
}: {
  ride: Ride;
  to?: string;
  perspective?: 'passenger' | 'driver' | 'staff';
}) {
  const ft = useFormatT();
  const { t } = useTranslation('rideCard');

  return (
    <Link to={to ?? `/rides/${ride.id}`} className="card link ride-card">
      <div className="ride-card-time">
        <span className="ride-card-time-val">{fmtTime(ride.pickup_at, ft)}</span>
        <span className="ride-card-date-val muted">{fmtDay(ride.pickup_at, ft)}</span>
      </div>

      <div className="ride-card-main grow">
        <div className="ride-card-dest">{ride.destination}</div>
        <div className="ride-card-pickup muted small row nowrap">
          {ride.pickup_address === null && (
            <Lock size={12} className="muted" aria-hidden="true" />
          )}
          <span>{ride.area.name}</span>
          {ride.seats > 1 && <span>· {ride.seats} {t('seats')}</span>}
          {ride.event && <span>· {ride.event.title}</span>}
        </div>
        {ride.needs_attention && (
          <div className="small row nowrap ride-card-attention">
            <AlertTriangle size={14} aria-hidden="true" />
            <span>{ride.attention_reason || t('needsAttention')}</span>
          </div>
        )}
      </div>

      <div className="ride-card-side">
        <div className="ride-card-status">
          <StatusPill map={RIDE_STATUS} status={ride.status} />
        </div>
        <div className="ride-card-person">
          {perspective === 'passenger' ? (
            ride.driver ? (
              <div className="row nowrap ride-card-driver" title={ride.driver.name}>
                <span className="small muted hide-mobile">{ride.driver.name}</span>
                <Avatar name={ride.driver.name} size="sm" />
              </div>
            ) : null
          ) : perspective === 'driver' ? (
            <div className="row nowrap ride-card-driver" title={ride.passenger.name}>
              <span className="small muted hide-mobile">{ride.passenger.name}</span>
              <Avatar name={ride.passenger.name} size="sm" />
            </div>
          ) : (
            <span className="small muted">{ride.passenger.name} → {ride.driver?.name ?? t('noDriver')}</span>
          )}
        </div>
      </div>
    </Link>
  );
}
