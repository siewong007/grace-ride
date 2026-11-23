import { Link } from 'react-router-dom';
import { CarFront, QrCode } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { EventIcon } from '../../eventIcon';
import { fmtTime, useFormatT } from '../../format';
import type { EventItem } from '../../pages/Events';
import { Button, Pill } from '../ui';

export function EventTodayBanner({
  event,
  transportSummary,
  outboundRideId,
  drivingCount,
  canRequestRide,
  onShowPass,
  onScan,
}: {
  event: EventItem;
  transportSummary: string | null;
  outboundRideId?: number;
  drivingCount: number;
  canRequestRide: boolean;
  onShowPass: () => void;
  onScan: () => void;
}) {
  const { t } = useTranslation('events');
  const ft = useFormatT();

  const attendanceEnabled = event.attendance_enabled !== false;
  const attended = attendanceEnabled && event.my_signup?.attended === 1;
  const canShowPass = attendanceEnabled && !attended && Boolean(event.my_signup?.pass_code);
  const canScan = attendanceEnabled && !attended;
  const hasAttendanceActions = canShowPass || canScan;
  const transportVariant = hasAttendanceActions ? 'secondary' : 'primary';

  return (
    <section className="focus-surface focus-surface--today" aria-label={t('todayEyebrow', { time: fmtTime(event.starts_at, ft) })}>
      <div className="row between nowrap">
        <span className="home-focus-eyebrow">{t('todayEyebrow', { time: fmtTime(event.starts_at, ft) })}</span>
        <Pill tone="success">
          {attended ? t('statusAttended', { ns: 'eventDetail' }) : t('signedUp')}
        </Pill>
      </div>

      <div className="stack-xs">
        <Link to={`/events/${event.id}`} className="bold event-today-title">
          <span className="event-title-with-icon">
            <EventIcon icon={event.icon} kind={event.kind} size={18} />
            <span>{event.title}</span>
          </span>
        </Link>
        <div className="muted small">{event.location}</div>
        {transportSummary && (
          <div className="small event-today-transport">
            <CarFront size={15} className="event-today-transport-icon" aria-hidden="true" />
            <span>{transportSummary}</span>
          </div>
        )}
      </div>

      <div className="btn-row">
        {canShowPass && (
          <Button variant="primary" size="sm" icon={<QrCode size={16} />} onClick={onShowPass}>
            {t('showMyPass')}
          </Button>
        )}
        {canScan && (
          <Button
            variant={canShowPass ? 'secondary' : 'primary'}
            size="sm"
            icon={<QrCode size={16} />}
            onClick={onScan}
          >
            {t('scanToAttend', { ns: 'eventDetail' })}
          </Button>
        )}
        {outboundRideId ? (
          <Button variant={transportVariant} size="sm" to={`/rides/${outboundRideId}`}>
            {t('openRide')}
          </Button>
        ) : drivingCount > 0 ? (
          <Button variant={transportVariant} size="sm" to="/drive">
            {t('openItinerary')}
          </Button>
        ) : canRequestRide ? (
          <Button variant={transportVariant} size="sm" to={`/rides/new?event=${event.id}`}>
            {t('needRideBtn')}
          </Button>
        ) : (
          <Button variant={hasAttendanceActions ? 'ghost' : 'primary'} size="sm" to={`/events/${event.id}`}>
            {t('viewDetails')}
          </Button>
        )}
      </div>
    </section>
  );
}
