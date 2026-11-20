import { Link } from 'react-router-dom';
import { AlertTriangle, Lock } from 'lucide-react';
import { fmtDay, fmtTime, RIDE_STATUS, RIDE_STATUS_PASSENGER, useFormatT } from '../format';
import { Pill, StatusPill } from './ui';
import { useTranslation } from 'react-i18next';

export interface Ride {
  id: number;
  status: string;
  pickup_at: string;
  leg?: 'outbound' | 'return';
  paired_ride_id?: number | null;
  area: { id: number; name: string };
  pickup_address: string | null;
  destination: string;
  seats: number;
  assistance: string;
  notes: string;
  /** `private`: the viewer may not see this event, so it has no id or title. */
  event: { id: number | null; title: string | null; private?: boolean } | null;
  trip_id: number | null;
  passenger: { id: number; name: string; phone: string | null };
  driver: {
    id: number;
    name: string;
    phone: string | null;
    photo_file_id?: number | null;
    avatar_color?: string | null;
    avatar_icon?: string | null;
    vehicle: {
      model: string;
      color: string;
      plate: string;
      wheelchair_ok?: boolean;
      low_step?: boolean;
      child_seat?: boolean;
    } | null;
  } | null;
  needs_attention: boolean;
  /** Completed for a claimable event on a route with no fare: the driver hands in a mileage receipt. */
  mileage_receipt_needed?: boolean;
  /**
   * Fixed driver fare for this route (pickup area ↔ event area) on a mileage-claimable
   * event. Shown to drivers and staff only; `amount_cents` is null until it is priced.
   */
  route_fare?: { from: string; to: string; amount_cents: number | null } | null;
  attention_reason: string;
  started_at: string | null;
  picked_up_at: string | null;
  completed_at: string | null;
  updated_at: string;
  readiness_status?: 'unconfirmed' | 'ready' | 'delayed_5m' | 'delayed_10m' | 'unable_to_join';
  readiness_updated_at?: string | null;
  delay_minutes?: number;
  arrived_at_gate_at?: string | null;
  supper_choice?: 'unspecified' | 'supper' | 'direct_home';
  is_proxy?: boolean;
  proxy?: {
    is_proxy: boolean;
    rider_name: string;
    rider_phone: string | null;
    rider_relationship: string;
    booker_name: string;
    booker_phone: string | null;
  } | null;
  viewer: { is_passenger: boolean; is_driver: boolean; is_staff: boolean };
  driver_load?: { seats_used: number; capacity: number };
  passenger_conflicts?: { id: number; pickup_at: string }[];
}

/**
 * One ride as a list row: time, destination and pickup area, status. Put rows inside a
 * `.list` so they get dividers. `showDate` is off when a date heading already says the day.
 */
export default function RideCard({
  ride,
  to,
  perspective = 'passenger',
  showDate = true,
}: {
  ride: Ride;
  to?: string;
  perspective?: 'passenger' | 'driver' | 'staff';
  showDate?: boolean;
}) {
  const ft = useFormatT();
  const { t } = useTranslation('rideCard');
  const person =
    perspective === 'passenger'
      ? ride.is_proxy && ride.proxy
        ? `${ride.proxy.rider_name} (${t('proxyLabel')}) · ${ride.driver?.name ?? t('noDriver')}`
        : ride.driver?.name
      : perspective === 'driver'
        ? ride.is_proxy && ride.proxy
          ? `${ride.proxy.rider_name} (${t('proxyBookedBy', { name: ride.proxy.booker_name })})`
          : ride.passenger.name
        : `${ride.passenger.name} → ${ride.driver?.name ?? t('noDriver')}`;

  return (
    <Link to={to ?? `/rides/${ride.id}`} className="item ride-card">
      <div className="ride-card-time">
        <span className="ride-card-time-val">{fmtTime(ride.pickup_at, ft)}</span>
        {showDate && <span className="ride-card-date-val muted small">{fmtDay(ride.pickup_at, ft)}</span>}
      </div>

      <div className="ride-card-main grow">
        <div className="ride-card-dest">{ride.destination}</div>
        <div className="ride-card-pickup muted small row nowrap">
          {ride.leg === 'return' && <Pill tone="info">{t('returnLeg')}</Pill>}
          {ride.pickup_address === null && <Lock size={14} aria-hidden="true" />}
          <span>{ride.area.name}</span>
          {ride.seats > 1 && <span>· {ride.seats} {t('seats')}</span>}
          {ride.event && <span>· {ride.event.title ?? t('churchEvent')}</span>}
          {perspective !== 'passenger' && ride.route_fare?.amount_cents != null && (
            <span>· RM {(ride.route_fare.amount_cents / 100).toFixed(2)}</span>
          )}
        </div>
        {ride.needs_attention && (
          <div className="small row nowrap ride-card-attention">
            <AlertTriangle size={14} aria-hidden="true" />
            <span>{ride.attention_reason || t('needsAttention')}</span>
          </div>
        )}
      </div>

      <div className="ride-card-side">
        <StatusPill map={perspective === 'passenger' && ride.viewer?.is_passenger ? RIDE_STATUS_PASSENGER : RIDE_STATUS} status={ride.status} />
        {person && <span className="ride-card-person muted small">{person}</span>}
      </div>
    </Link>
  );
}
