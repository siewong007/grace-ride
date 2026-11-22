import { useState } from 'react';
import { Accessibility, Lock } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { post, useApi } from '../../api';
import { useAuth } from '../../auth';
import { Ride } from '../RideCard';
import {
  Button,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Pill,
  useAction,
  useToast,
} from '../ui';
import { fmtDay, fmtTime, useFormatT } from '../../format';
import type { Area } from './types';

/** Open ride requests. With `eventId` the list is that event's requests only: a driver
 *  volunteers per event, after signing up for it. */
export function DriveRequests({ eventId }: { eventId?: number } = {}) {
  const { t } = useTranslation('drive');
  const ft = useFormatT();
  const toast = useToast();
  const { can } = useAuth();
  const { data: areas } = useApi<Area[]>('/areas');
  const [area, setArea] = useState<string>(eventId ? '' : 'mine');
  const [date, setDate] = useState('');

  const q = new URLSearchParams();
  if (eventId) q.set('event_id', String(eventId));
  if (area === 'mine') q.set('mine_areas', 'true');
  else if (area) q.set('area_id', area);
  if (date) q.set('date', date);

  const { data, error, loading, reload } = useApi<Ride[]>(
    `/driver/requests?${q}`,
    30000,
  );
  const { busy, error: actErr } = useAction();

  const claim = async (rideId: number) => {
    try {
      await post(`/rides/${rideId}/claim`);
      toast.success(t('claimed'));
      reload();
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : t('conflictNotice');
      toast.error(message);
      reload();
    }
  };

  return (
    <div className="stack">
      {/* Filters: one area menu and an optional date, instead of a wall of chips. */}
      <div className="driver-filters">
        <select
          className="input"
          value={area}
          onChange={(e) => setArea(e.target.value)}
          aria-label={t('areas')}
        >
          <option value="mine">{t('myAreas')}</option>
          <option value="">{t('all')}</option>
          {areas
            ?.filter((a) => a.active)
            .map((a) => (
              <option key={a.id} value={String(a.id)}>{a.name}</option>
            ))}
        </select>
        {!eventId && <input
          className="input driver-date-input"
          type="date"
          value={date}
          onChange={(e) => setDate(e.target.value)}
          aria-label={t('dateLabel')}
        />}
        {!eventId && date && (
          <Button variant="ghost" size="sm" onClick={() => setDate('')}>
            {t('clearDate')}
          </Button>
        )}
      </div>

      <ErrorBox error={error ?? actErr} onRetry={reload} />

      {loading ? (
        <CardSkeleton count={3} />
      ) : !data?.length ? (
        <EmptyState title={t('noRequests')} />
      ) : (
        <div className={eventId ? 'stack' : 'driver-requests-grid'}>
          {data.map((r) => (
            <div key={r.id} className={eventId ? 'detail-section' : 'driver-request-card'}>
              <div className="driver-request-card-head">
                <span className="driver-request-area">{r.area.name}</span>
                {r.event && <span className="muted small">{r.event.title ?? t('churchEvent', { ns: 'rideCard' })}</span>}
              </div>

              <div className="driver-request-meta">
                {fmtDay(r.pickup_at, ft)} · {fmtTime(r.pickup_at, ft)} · {t('seatCount', { count: r.seats })}
              </div>

              <div className="driver-request-route">
                <span className="muted">{t('passenger')} </span>
                {r.is_proxy && r.proxy ? (
                  <>
                    <span className="bold">{r.proxy.rider_name}</span>{' '}
                    <Pill tone="info">
                      {t('proxyBadge', {
                        rel: r.proxy.rider_relationship || t('proxyDefaultRel'),
                      })}
                    </Pill>{' '}
                    <span className="small muted">
                      ({t('bookedBy', { name: r.proxy.booker_name })})
                    </span>
                  </>
                ) : (
                  <span className="bold">{r.passenger.name}</span>
                )}
                <span> → {r.destination}</span>
              </div>

              {r.assistance && (
                <div className="driver-trip-assistance">
                  <Accessibility size={14} aria-hidden="true" />
                  <span>{r.assistance}</span>
                </div>
              )}

              <div className="driver-request-locked">
                <Lock size={12} aria-hidden="true" />
                <span>{t('pickupAddressLocked')}</span>
              </div>

              {can('drive', 'create') && (
                <div className={eventId ? 'btn-row' : undefined}>
                  <Button
                    variant="primary"
                    size={eventId ? 'sm' : 'md'}
                    block={!eventId}
                    disabled={busy}
                    onClick={() => claim(r.id)}
                  >
                    {t('offerToDrive')}
                  </Button>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
