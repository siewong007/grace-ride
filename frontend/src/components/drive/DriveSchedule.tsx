import { useEffect, useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { Accessibility, Lock, Phone } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { post, useApi } from '../../api';
import { useAuth } from '../../auth';
import { Ride } from '../RideCard';
import { DriverRouteCard } from '../TripRoute';
import {
  NavButtonGroup,
  ReasonModal,
  ShareLocation,
  StartTripButton,
} from '../TripActions';
import {
  Button,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Notice,
  Pill,
  StatusPill,
  useAction,
  useConfirm,
  useToast,
} from '../ui';
import {
  fmtAgo,
  fmtDay,
  fmtTime,
  localDate,
  parseLocal,
  RIDE_STATUS,
  todayStr,
  useFormatT,
} from '../../format';
import { loadTodayRidesSnapshot, saveTodayRidesSnapshot } from '../../offlineSnapshot';
import { getQueuedActionsForRide, postOrQueue, useOfflineQueueTick } from '../../offlineQueue';

export function DriveSchedule() {
  const { t } = useTranslation('drive');
  const ft = useFormatT();
  const toast = useToast();
  const confirm = useConfirm();
  const { me, can } = useAuth();
  const [range, setRange] = useState<'day' | 'week'>('day');
  const [withdrawRide, setWithdrawRide] = useState<Ride | null>(null);

  const from = todayStr();
  const end = new Date();
  end.setDate(end.getDate() + 6);
  const to = range === 'day' ? from : localDate(end);

  const { data, error, loading, reload } = useApi<Ride[]>(
    `/driver/schedule?from=${from}&to=${to}`,
    20000,
  );
  const { busy, error: actErr, run } = useAction();
  const queueTick = useOfflineQueueTick();
  const offlineSnap = !data && error && me?.id ? loadTodayRidesSnapshot(me.id) : null;
  const scheduleRides = data ?? (offlineSnap ? offlineSnap.rides.filter((r) => r.viewer?.is_driver) : null);
  const active = scheduleRides?.some((r) => r.status === 'in_progress');

  useEffect(() => {
    if (me?.id && data) {
      saveTodayRidesSnapshot(me.id, data);
    }
  }, [me?.id, data]);

  const act = (r: Ride, path: string, msg: string) =>
    run(() => post(`/rides/${r.id}/${path}`).then(reload), msg);

  const actQueueable = async (r: Ride, path: string, label: string, successMsg: string, body?: unknown) => {
    if (!me?.id) {
      if (await run(() => post(`/rides/${r.id}/${path}`, body).then(reload))) {
        toast.success(successMsg);
      }
      return;
    }
    await run(async () => {
      const res = await postOrQueue({
        userId: me.id,
        rideId: r.id,
        path: `/rides/${r.id}/${path}`,
        body,
        label,
      });
      if (res.queued) {
        toast.info(t('common:queuedOffline'));
      } else {
        toast.success(successMsg);
        reload();
      }
    });
  };

  const handleNoShow = async (r: Ride) => {
    const ok = await confirm({
      title: t('confirmNoShowTitle'),
      body: t('confirmNoShowBody'),
      tone: 'danger',
      confirmLabel: t('confirmNoShowBtn'),
    });
    if (ok) {
      act(r, 'no-show', t('respondedNoShow'));
    }
  };

  const focusRide = useMemo(() => {
    if (!scheduleRides?.length) return null;
    const sorted = scheduleRides.slice().sort((a, b) => a.pickup_at.localeCompare(b.pickup_at));
    const inProgPickup = sorted.find((r) => r.status === 'in_progress' && !r.picked_up_at);
    if (inProgPickup) return inProgPickup;
    const inProgDrop = sorted.find((r) => r.status === 'in_progress');
    if (inProgDrop) return inProgDrop;
    const nowMs = Date.now();
    return (
      sorted.find((r) => {
        if (r.status !== 'confirmed') return false;
        const diffMins = (parseLocal(r.pickup_at).getTime() - nowMs) / 60000;
        return diffMins >= -30 && diffMins <= 45;
      }) ?? null
    );
  }, [scheduleRides]);

  const byDay = useMemo(() => {
    const map = new Map<string, Ride[]>();
    if (!scheduleRides) return map;
    const sorted = scheduleRides
      .filter((r) => r.id !== focusRide?.id)
      .sort((a, b) => a.pickup_at.localeCompare(b.pickup_at));
    sorted.forEach((r) => {
      const k = r.pickup_at.slice(0, 10);
      map.set(k, [...(map.get(k) ?? []), r]);
    });
    return map;
  }, [scheduleRides, focusRide]);

  const renderTripCard = (r: Ride, isNextStop = false) => {
    const canDelay = r.status === 'confirmed' || (r.status === 'in_progress' && !r.picked_up_at);
    const canWithdraw =
      (r.status === 'confirmed' || r.status === 'pending_driver_confirm') && can('drive', 'delete');
    const canNoShow =
      r.status === 'in_progress' && !r.picked_up_at && parseLocal(r.pickup_at).getTime() <= Date.now();
    const hasUtility =
      Boolean(r.is_proxy && r.proxy ? (r.proxy.rider_phone || r.proxy.booker_phone) : r.passenger.phone) ||
      (r.status === 'in_progress' && !r.picked_up_at) ||
      canNoShow;

    return (
      <div key={r.id} className={`driver-trip-card${isNextStop ? ' driver-next-stop' : ''}`}>
        {isNextStop && (
          <div className="row between nowrap driver-next-stop-head">
            <Pill tone={r.status === 'in_progress' ? 'primary' : 'info'}>
              {r.status === 'in_progress' ? t('nextStopActive') : t('nextStopSoon')}
            </Pill>
            <Link to={`/rides/${r.id}`} className="small">
              {t('openRideDetail')}
            </Link>
          </div>
        )}
        <div className="driver-trip-header">
          <div className="driver-trip-title-row">
            <div className="driver-trip-time">
              {fmtTime(r.pickup_at, ft).replace(/^.. /, '')}
            </div>
            <Link to={`/rides/${r.id}`} className="driver-trip-title">
              {r.is_proxy && r.proxy ? (
                <span className="row gap-xs items-center wrap">
                  <span className="bold">{r.proxy.rider_name}</span>
                  <Pill tone="info">
                    {t('proxyBadge', {
                      rel: r.proxy.rider_relationship || t('proxyDefaultRel'),
                    })}
                  </Pill>
                  <span className="muted small">
                    ({t('bookedBy', { name: r.proxy.booker_name })})
                  </span>
                </span>
              ) : (
                <span>{r.passenger.name}</span>
              )}
              <span className="muted small">· {t('seatCount', { count: r.seats })}</span>
            </Link>
          </div>
          <div className="row nowrap gap-xs">
            {me?.id && queueTick >= 0 && getQueuedActionsForRide(me.id, r.id).length > 0 && (
              <Pill tone="info">{t('common:offlineQueuedPill')}</Pill>
            )}
            {r.readiness_status === 'ready' && (
              <Pill tone="success">{t('passengerReady')}</Pill>
            )}
            {(r.readiness_status === 'delayed_5m' || r.readiness_status === 'delayed_10m') && (
              <Pill tone="warning">{t('passengerDelayed')}</Pill>
            )}
            <StatusPill map={RIDE_STATUS} status={r.status} />
          </div>
        </div>

        <div className="driver-trip-route">
          <span>
            {r.pickup_address ? (
              r.pickup_address
            ) : (
              <span className="driver-trip-route-lock">
                <Lock size={12} aria-hidden="true" />
                <span>{r.area.name}</span>
              </span>
            )}
            {' → '}
            {r.destination}
          </span>
        </div>

        {r.assistance && (
          <div className="driver-trip-assistance">
            <Accessibility size={14} aria-hidden="true" />
            <span>{r.assistance}</span>
          </div>
        )}

        {r.pickup_address && (
          <NavButtonGroup address={r.pickup_address} />
        )}

        <div className="driver-trip-actions">
          {r.status === 'confirmed' && (
            <div className="driver-trip-primary-btn">
              <StartTripButton
                ride={r}
                onStarted={reload}
                block
                label={t('startTrip')}
                others={(scheduleRides ?? [])
                  .filter(
                    (o) =>
                      o.id !== r.id &&
                      o.status === 'confirmed' &&
                      o.trip_id != null &&
                      o.trip_id === r.trip_id,
                  )
                  .map((o) => o.passenger.name)}
              />
            </div>
          )}
          {r.status === 'in_progress' && !r.picked_up_at && (
            <Button
              variant="primary"
              block
              className="driver-trip-primary-btn"
              disabled={busy}
              onClick={() => void actQueueable(r, 'pickup', t('pickedUp'), t('respondedPickedUp'))}
            >
              {t('pickedUp')}
            </Button>
          )}
          {r.status === 'in_progress' && r.picked_up_at && (
            <Button
              variant="primary"
              block
              className="driver-trip-primary-btn"
              disabled={busy}
              onClick={() => void actQueueable(r, 'complete', t('completed'), t('respondedCompleted'))}
            >
              {t('completed')}
            </Button>
          )}
          {r.status === 'pending_driver_confirm' && (
            <Button
              variant="primary"
              block
              className="driver-trip-primary-btn"
              disabled={busy}
              onClick={() => act(r, 'confirm-assignment', t('respondedConfirmed'))}
            >
              {t('confirmAssignment')}
            </Button>
          )}

          {hasUtility && (
            <div className="driver-trip-utility-row">
              {r.is_proxy && r.proxy ? (
                <>
                  {r.proxy.rider_phone && (
                    <a className="btn secondary sm" href={`tel:${r.proxy.rider_phone}`}>
                      <Phone size={14} aria-hidden="true" />
                      <span>{t('callRider', { name: r.proxy.rider_name })}</span>
                    </a>
                  )}
                  {r.proxy.booker_phone && (
                    <a className="btn secondary sm" href={`tel:${r.proxy.booker_phone}`}>
                      <Phone size={14} aria-hidden="true" />
                      <span>{t('callBooker', { name: r.proxy.booker_name })}</span>
                    </a>
                  )}
                </>
              ) : (
                r.passenger.phone && (
                  <a className="btn secondary sm" href={`tel:${r.passenger.phone}`}>
                    <Phone size={14} aria-hidden="true" />
                    <span>{t('call')}</span>
                  </a>
                )
              )}

              {r.status === 'in_progress' && !r.picked_up_at && (
                <Button
                  variant="secondary"
                  size="sm"
                  disabled={busy}
                  onClick={() => void actQueueable(r, 'arrived-gate', t('arrivedGate'), t('arrivedGateSent'))}
                >
                  {t('arrivedGate')}
                </Button>
              )}

              {/* The backend refuses a no-show before the pickup time has passed. */}
              {canNoShow && (
                <Button
                  variant="ghost"
                  className="quiet-danger"
                  size="sm"
                  disabled={busy}
                  onClick={() => handleNoShow(r)}
                >
                  {t('noShow')}
                </Button>
              )}
            </div>
          )}

          {(canDelay || canWithdraw) && (
            <details className="quiet-disclosure driver-trip-primary-btn">
              <summary>{t('moreTripOptions')}</summary>
              <div className="quiet-disclosure-body btn-row">
                {canDelay && (
                  <>
                    <Button
                      variant="secondary"
                      size="sm"
                      disabled={busy}
                      onClick={() =>
                        void actQueueable(
                          r,
                          'driver-delay',
                          t('delayPlus5'),
                          t('delayToast', { mins: 5 }),
                          { delay_minutes: 5 },
                        )
                      }
                    >
                      {t('delayPlus5')}
                    </Button>
                    <Button
                      variant="secondary"
                      size="sm"
                      disabled={busy}
                      onClick={() =>
                        void actQueueable(
                          r,
                          'driver-delay',
                          t('delayPlus10'),
                          t('delayToast', { mins: 10 }),
                          { delay_minutes: 10 },
                        )
                      }
                    >
                      {t('delayPlus10')}
                    </Button>
                  </>
                )}
                {canWithdraw && (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="quiet-danger"
                    disabled={busy}
                    onClick={() => setWithdrawRide(r)}
                  >
                    {t('withdrawDriver')}
                  </Button>
                )}
              </div>
            </details>
          )}
        </div>
      </div>
    );
  };

  return (
    <div className="stack">
      <div className="driver-schedule-controls">
        <div className="chips" role="group" aria-label={t('scheduleRange')}>
          <button
            type="button"
            className={`chip ${range === 'day' ? 'on' : ''}`}
            onClick={() => setRange('day')}
          >
            {t('today')}
          </button>
          <button
            type="button"
            className={`chip ${range === 'week' ? 'on' : ''}`}
            onClick={() => setRange('week')}
          >
            {t('week7')}
          </button>
        </div>
        <span className="muted small">{t('slotRides', { count: scheduleRides?.length ?? 0 })}</span>
      </div>

      {active && <ShareLocation />}
      {offlineSnap && (
        <Notice tone="warning">
          <span>{t('offlineSnapshot', { when: fmtAgo(offlineSnap.savedAt, ft) })}</span>
        </Notice>
      )}
      <div className="drive-schedule-body stack">
      <div className="drive-route-slot">
      <DriverRouteCard
        refreshKey={scheduleRides?.map((r) => `${r.id}:${r.status}:${r.picked_up_at ?? ''}`).join(',')}
      />
      </div>
      <ErrorBox error={offlineSnap ? null : (error ?? actErr)} onRetry={reload} />

      <div className="drive-trips-slot stack">
      {loading ? (
        <CardSkeleton count={2} />
      ) : !scheduleRides?.length ? (
        <EmptyState
          title={t('noTasks')}
          action={
            <Button variant="primary" to="/events">
              {t('browseEvents')}
            </Button>
          }
        />
      ) : (
        <>
          {focusRide && renderTripCard(focusRide, true)}
          {[...byDay.entries()].map(([day, rides]) => (
            <div key={day} className="stack">
              <div className="sticky-date-header">
                {fmtDay(`${day}T00:00`, ft)} · {t('seatCount', { count: rides.reduce((n, r) => n + r.seats, 0) })}
              </div>
              <div className="stack">
                {rides.map((r) => renderTripCard(r, false))}
              </div>
            </div>
          ))}
        </>
      )}
      </div>
      </div>

      {withdrawRide && (
        <ReasonModal
          title={t('withdrawTitle')}
          label={t('withdrawLabel')}
          confirm={t('withdrawConfirm')}
          onClose={() => setWithdrawRide(null)}
          onSubmit={(reason) =>
            run(
              () => post(`/rides/${withdrawRide.id}/withdraw`, { reason }).then(reload),
              t('respondedWithdrew'),
            )
          }
        />
      )}

    </div>
  );
}
