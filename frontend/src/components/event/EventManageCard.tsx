import { Link } from 'react-router-dom';
import { Edit2, EyeOff, QrCode, Route, Send } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { EventIcon } from '../../eventIcon';
import { fmtDateTime, useFormatT } from '../../format';
import type { EventItem } from '../../pages/Events';
import { Button, Pill } from '../ui';

export function EventManageCard({
  event,
  canPlan,
  canReimb,
  busy = false,
  onOpenQr,
  onOpenReimb,
  onPublish,
  onUnpublish,
}: {
  event: EventItem;
  canPlan: boolean;
  canReimb: boolean;
  busy?: boolean;
  onOpenQr: (e: EventItem) => void;
  onOpenReimb: (e: EventItem) => void;
  onPublish?: (e: EventItem) => void;
  onUnpublish?: (e: EventItem) => void;
}) {
  const { t } = useTranslation('events');
  const ft = useFormatT();

  const isPublished = event.status === 'published';
  const isDraft = event.status === 'draft';
  const showQr = isPublished && event.attendance_enabled !== false;
  const pct =
    event.capacity > 0 ? Math.min(100, Math.round((event.going / Math.max(1, event.capacity)) * 100)) : 0;
  const unassigned = event.unassigned_rides ?? 0;

  return (
    <article className="event-manage-card stack-sm">
      <div className="row between nowrap">
        <div className="grow min-w-0">
          <div className="muted small">{fmtDateTime(event.starts_at, ft)}</div>
          <Link to={`/events/${event.id}`} className="bold event-manage-title clamp-2">
            <span className="event-title-with-icon">
              <EventIcon icon={event.icon} kind={event.kind} size={22} />
              <span>{event.title}</span>
            </span>
          </Link>
          <div className="muted small ellipsis">{event.location}</div>
        </div>
        <Pill tone={isPublished ? 'success' : isDraft ? 'warning' : 'neutral'}>
          {isPublished
            ? t('managePublished')
            : isDraft
              ? t('manageDraft')
              : event.status === 'cancelled'
                ? t('cancelled')
                : t('finished')}
        </Pill>
      </div>

      {!isDraft && (
        <div className="stack-xs">
          <div className="row between wrap gap-xs small">
            <span className="bold">
              {event.capacity > 0
                ? t('people', { going: event.going, capacity: event.capacity })
                : `${event.going} ${t('peopleAttend')}`}
              {event.waitlist > 0 ? ` · ${t('waitlistCount', { count: event.waitlist })}` : ''}
            </span>
            {event.attendance_enabled !== false &&
              event.attended_count != null &&
              (!isPublished || event.attended_count > 0) && (
                <span className="event-manage-attended">{t('manageAttended', { count: event.attended_count })}</span>
              )}
            {isPublished && unassigned > 0 && (
              <span className="event-deadline-urgent">{t('manageUnassignedRides', { count: unassigned })}</span>
            )}
          </div>
          {event.capacity > 0 && (
            <div className="event-progress" aria-hidden="true">
              <div className="event-progress-fill" style={{ width: `${pct}%` }} />
            </div>
          )}
        </div>
      )}

      <div className="btn-row">
        {isDraft ? (
          <>
            {event.can_manage && onPublish && (
              <Button
                variant="primary"
                size="sm"
                icon={<Send size={15} />}
                disabled={busy}
                onClick={() => onPublish(event)}
              >
                {t('publishBtn')}
              </Button>
            )}
            {event.can_manage && (
              <Button
                variant="secondary"
                size="sm"
                icon={<Edit2 size={16} />}
                to={`/events/${event.id}/edit`}
              >
                {t('manageEdit')}
              </Button>
            )}
            <Button variant="ghost" size="sm" to={`/events/${event.id}`}>
              {t('viewDetails')}
            </Button>
          </>
        ) : isPublished ? (
          <>
            {showQr && (
              <Button variant="primary" size="sm" icon={<QrCode size={16} />} onClick={() => onOpenQr(event)}>
                {t('attendanceQr', { ns: 'eventDetail' })}
              </Button>
            )}
            {canPlan && (
              <Button
                variant={showQr ? 'secondary' : 'primary'}
                size="sm"
                icon={<Route size={16} />}
                to={`/admin/events/${event.id}/plan`}
              >
                {t('manageTransportPlan')}
              </Button>
            )}
            {event.can_manage && (
              <Button
                variant={!showQr && !canPlan ? 'primary' : 'secondary'}
                size="sm"
                icon={<Edit2 size={16} />}
                to={`/events/${event.id}/edit`}
              >
                {t('manageEdit')}
              </Button>
            )}
            {event.can_manage && event.can_unpublish && onUnpublish && (
              <Button
                variant="ghost"
                size="sm"
                icon={<EyeOff size={15} />}
                disabled={busy}
                onClick={() => onUnpublish(event)}
              >
                {t('hideToDraftBtn')}
              </Button>
            )}
            <Button variant="ghost" size="sm" to={`/events/${event.id}`}>
              {t('viewDetails')}
            </Button>
          </>
        ) : (
          <>
            {event.status === 'finished' && canReimb && (
              <Button variant="secondary" size="sm" onClick={() => onOpenReimb(event)}>
                {t('postEventReimbBtn')}
              </Button>
            )}
            <Button variant="ghost" size="sm" to={`/events/${event.id}`}>
              {t('viewDetails')}
            </Button>
          </>
        )}
      </div>
    </article>
  );
}
