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
  Pill,
  StatusPill,
  useAction,
  useConfirm,
  useToast,
} from '../ui';
import {
  fmtDay,
  fmtTime,
  localDate,
  parseLocal,
  RIDE_STATUS,
  todayStr,
  useFormatT,
} from '../../format';
import { saveTodayRidesSnapshot } from '../../offlineSnapshot';

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
  const active = data?.some((r) => r.status === 'in_progress');

  useEffect(() => {
    if (me?.id && data) {
      saveTodayRidesSnapshot(me.id, data);
    }
  }, [me?.id, data]);

  const act = (r: Ride, path: string, msg: string) =>
    run(() => post(`/rides/${r.id}/${path}`).then(reload), msg);

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
    if (!data?.length) return null;
    const sorted = data.slice().sort((a, b) => a.pickup_at.localeCompare(b.pickup_at));
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
  }, [data]);

  const byDay = useMemo(() => {
    const map = new Map<string, Ride[]>();
    if (!data) return map;
    const sorted = data
      .filter((r) => r.id !== focusRide?.id)
      .sort((a, b) => a.pickup_at.localeCompare(b.pickup_at));
    sorted.forEach((r) => {
      const k = r.pickup_at.slice(0, 10);
      map.set(k, [...(map.get(k) ?? []), r]);
    });
    return map;
  }, [data, focusRide]);

  const renderTripCard = (r: Ride, isNextStop = false) => (
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
          <div>
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
          </div>
        </div>
        <div className="row nowrap gap-xs">
          {r.readiness_status === 'ready' && (
            <Pill tone="success">{t('passengerReady')}</Pill>
          )}
          {(r.readiness_status === 'delayed_5m' || r.readiness_status === 'delayed_10m') && (
            <Pill tone="warning">{t('passengerDelayed')}</Pill>
          )}
          <StatusPill map={RIDE_STATUS} status={r.status} />
        </div>
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
              others={(data ?? [])
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
            onClick={() => act(r, 'pickup', t('respondedPickedUp'))}
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
            onClick={() => act(r, 'complete', t('respondedCompleted'))}
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
            onClick={() => act(r, 'confirm-assignment', t('respondedCompleted'))}
          >
            {t('confirmAssignment')}
          </Button>
        )}

        {r.is_proxy && r.proxy ? (
          <div className="row wrap gap-xs">
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
          </div>
        ) : (
          r.passenger.phone && (
            <a className="btn secondary" href={`tel:${r.passenger.phone}`}>
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
            onClick={async () => {
              if (await run(() => post(`/rides/${r.id}/arrived-gate`).then(reload))) {
                toast.success(t('arrivedGateSent'));
              }
            }}
          >
            {t('arrivedGate')}
          </Button>
        )}

        {(r.status === 'confirmed' || (r.status === 'in_progress' && !r.picked_up_at)) && (
          <div className="btn-row">
            <Button
              variant="secondary"
              size="sm"
              disabled={busy}
              onClick={async () => {
                if (await run(() => post(`/rides/${r.id}/driver-delay`, { delay_minutes: 5 }).then(reload))) {
                  toast.success(t('delayToast', { mins: 5 }));
                }
              }}
            >
              {t('delayPlus5')}
            </Button>
            <Button
              variant="secondary"
              size="sm"
              disabled={busy}
              onClick={async () => {
                if (await run(() => post(`/rides/${r.id}/driver-delay`, { delay_minutes: 10 }).then(reload))) {
                  toast.success(t('delayToast', { mins: 10 }));
                }
              }}
            >
              {t('delayPlus10')}
            </Button>
          </div>
        )}

        {r.status === 'in_progress' && !r.picked_up_at && (
          <Button
            variant="danger"
            size="sm"
            disabled={busy}
            onClick={() => handleNoShow(r)}
          >
            {t('noShow')}
          </Button>
        )}

        {(r.status === 'confirmed' || r.status === 'pending_driver_confirm') && can('drive', 'delete') && (
          <Button
            variant="ghost"
            size="sm"
            className="driver-trip-withdraw"
            disabled={busy}
            onClick={() => setWithdrawRide(r)}
          >
            {t('withdrawDriver')}
          </Button>
        )}
      </div>
    </div>
  );

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
        <span className="muted small">{t('slotRides', { count: data?.length ?? 0 })}</span>
      </div>

      {active && <ShareLocation />}
      <div className="drive-schedule-body stack">
      <div className="drive-route-slot">
      <DriverRouteCard
        refreshKey={data?.map((r) => `${r.id}:${r.status}:${r.picked_up_at ?? ''}`).join(',')}
      />
      </div>
      <ErrorBox error={error ?? actErr} />

      <div className="drive-trips-slot stack">
      {loading ? (
        <CardSkeleton count={2} />
      ) : !data?.length ? (
        <EmptyState title={t('noTasks')} />
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
