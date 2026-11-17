import { AlertTriangle } from 'lucide-react';
import { Ride } from '../../components/RideCard';
import { StatusPill } from '../../components/ui';
import { fmtDay, RIDE_STATUS, FormatT } from '../../format';

/**
 * One ride as a compact list row (passenger, when and where, driver, status). Shared by the
 * ride list and the dashboard so the same ride reads the same way in both places.
 */
export function rideCompactRow(r: Ride, ft: FormatT, withDestination = false) {
  return (
    <>
      <div className="grow min-w-0">
        <div className="bold ellipsis">
          {r.passenger.name}
          {r.seats > 1 ? ` (${r.seats})` : ''}
        </div>
        <div className="muted small ellipsis">
          {fmtDay(r.pickup_at, ft).split(' ')[0]} {r.pickup_at.slice(11, 16)}
          {' · '}
          {r.area.name}
          {withDestination ? ` → ${r.destination}` : ''}
        </div>
        <div className="muted tiny ellipsis">{r.driver?.name ?? '—'}</div>
      </div>
      <div className="item-status">
        <StatusPill map={RIDE_STATUS} status={r.status} />
        {r.needs_attention && <AlertTriangle size={14} aria-hidden="true" className="pill-icon warn" />}
      </div>
    </>
  );
}
