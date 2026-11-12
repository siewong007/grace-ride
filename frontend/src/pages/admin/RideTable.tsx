import { Link } from 'react-router-dom';
import { AlertTriangle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Ride } from '../../components/RideCard';
import { StatusPill } from '../../components/ui';
import { fmtDay, RIDE_STATUS, useFormatT } from '../../format';

export function RideTable({
  rides,
  onOpen,
}: {
  rides: Ride[];
  onOpen?: (id: number) => void;
}) {
  const { t } = useTranslation('admin');
  const ft = useFormatT();

  return (
    <div className="stack-sm">
      <div className="admin-table-header">
        <span className="muted small bold">
          {t('rides.count', { count: rides.length })}
        </span>
      </div>
      <div className="table-wrap">
        <table className="table responsive sticky-header row-clickable">
          <thead>
            <tr>
              <th>{t('dashboard.pickupTime')}</th>
              <th>{t('dashboard.passenger')}</th>
              <th>{t('dashboard.area')}</th>
              <th>{t('dashboard.driver')}</th>
              <th>{t('dashboard.status')}</th>
            </tr>
          </thead>
          <tbody>
            {rides.map((r) => (
              <tr
                key={r.id}
                tabIndex={0}
                onClick={() => onOpen?.(r.id)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    onOpen?.(r.id);
                  }
                }}
              >
                <td data-label={t('dashboard.pickupTime')} className="mono bold">
                  <Link to={`/admin/rides/${r.id}`} className="row-link">
                    {fmtDay(r.pickup_at, ft).split(' ')[0]} {r.pickup_at.slice(11, 16)}
                  </Link>
                </td>
                <td data-label={t('dashboard.passenger')}>
                  {r.passenger.name}
                  {r.seats > 1 && ` (${r.seats})`}
                </td>
                <td data-label={t('dashboard.area')}>{r.area.name}</td>
                <td data-label={t('dashboard.driver')}>
                  {r.driver?.name ?? <span className="muted">—</span>}
                </td>
                <td data-label={t('dashboard.status')}>
                  <StatusPill map={RIDE_STATUS} status={r.status} />
                  {r.needs_attention && (
                    <AlertTriangle
                      size={14}
                      aria-hidden="true"
                      className="pill-icon warn"
                    />
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

export default RideTable;
