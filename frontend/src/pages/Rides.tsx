import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { CalendarPlus, Repeat, Trash2 } from 'lucide-react';
import { api, del, post, put, useApi } from '../api';
import { useAuth } from '../auth';
import RideCard, { Ride } from '../components/RideCard';
import {
  BackToTop,
  Button,
  EmptyState,
  ErrorBox,
  Field,
  IconButton,
  InfiniteSentinel,
  ListSkeleton,
  Modal,
  Notice,
  Page,
  PageHeader,
  Pill,
  Tabs,
  useAction,
  useInfiniteScroll,
} from '../components/ui';
import { fmtAgo, fmtDay, localDate, todayStr, useFormatT } from '../format';
import { loadTodayRidesSnapshot, saveTodayRidesSnapshot } from '../offlineSnapshot';
import { useTranslation } from 'react-i18next';

interface DateGroup {
  dateKey: string;
  label: string;
  rides: Ride[];
}

interface RidesPage {
  items: Ride[];
  next_before: string | null;
}

function groupRidesByDate(
  rides: Ride[],
  ft: (key: string, opts?: Record<string, unknown>) => string,
  t: (key: string) => string,
): DateGroup[] {
  const map = new Map<string, Ride[]>();
  const today = todayStr();
  const tmr = new Date();
  tmr.setDate(tmr.getDate() + 1);
  const tmrStr = localDate(tmr);

  for (const ride of rides) {
    const day = ride.pickup_at.slice(0, 10);
    if (!map.has(day)) {
      map.set(day, []);
    }
    map.get(day)!.push(ride);
  }

  const groups: DateGroup[] = [];
  for (const [day, dayRides] of map.entries()) {
    let label = fmtDay(`${day}T00:00:00`, ft);
    if (day === today) {
      label = t('dateToday');
    } else if (day === tmrStr) {
      label = t('dateTomorrow');
    }
    groups.push({ dateKey: day, label, rides: dayRides });
  }

  return groups;
}

interface AreaOption {
  id: number;
  name: string;
  active: boolean;
}

interface RecurringSchedule {
  id: number;
  kind: 'passenger' | 'driver';
  label: string;
  weekday: number;
  pickup_time: string;
  area_id: number;
  area_name: string;
  pickup_address: string;
  destination: string;
  seats: number;
  active: number;
  last_generated_date: string | null;
}

const WEEKDAYS = ['sun', 'mon', 'tue', 'wed', 'thu', 'fri', 'sat'] as const;

export default function Rides() {
  const { t } = useTranslation('rides');
  const ft = useFormatT();
  const { me, can } = useAuth();
  const [scope, setScope] = useState<'upcoming' | 'past'>('upcoming');
  const [showRecurring, setShowRecurring] = useState(false);

  // Upcoming rides (typically bounded and refreshed)
  const upcoming = useApi<Ride[]>('/rides?scope=upcoming', 30000);
  const upcomingList = Array.isArray(upcoming.data) ? upcoming.data : null;

  useEffect(() => {
    if (me?.id && upcomingList) {
      saveTodayRidesSnapshot(me.id, upcomingList);
    }
  }, [me?.id, upcomingList]);

  const offlineSnap =
    scope === 'upcoming' && !upcomingList && upcoming.error && me?.id
      ? loadTodayRidesSnapshot(me.id)
      : null;

  // Past rides with infinite scroll
  const [pastItems, setPastItems] = useState<Ride[]>([]);
  const [nextBefore, setNextBefore] = useState<string | null>(null);
  const [loadingPast, setLoadingPast] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [pastError, setPastError] = useState<string | null>(null);
  const latestPast = useRef(0);

  const loadPast = useCallback(async (before: string | null) => {
    const mine = ++latestPast.current;
    if (before == null) setLoadingPast(true);
    else setLoadingMore(true);
    try {
      const qs = before != null ? `&before=${encodeURIComponent(before)}&limit=30` : '&limit=30';
      const res = await api<RidesPage | Ride[]>(`/rides?scope=past${qs}`);
      if (mine !== latestPast.current) return;
      const newItems = Array.isArray(res) ? res : res.items;
      const nb = Array.isArray(res) ? null : res.next_before;
      setPastItems((prev) => {
        if (before == null) return newItems;
        const existing = new Set(prev.map((r) => r.id));
        return [...prev, ...newItems.filter((r) => !existing.has(r.id))];
      });
      setNextBefore(nb);
      setPastError(null);
    } catch (e) {
      if (mine === latestPast.current) setPastError((e as Error).message);
    } finally {
      if (mine === latestPast.current) {
        setLoadingPast(false);
        setLoadingMore(false);
      }
    }
  }, []);

  useEffect(() => {
    if (scope === 'past') {
      loadPast(null);
    }
  }, [scope, loadPast]);

  const { sentinelRef } = useInfiniteScroll({
    hasMore: nextBefore != null,
    loading: loadingMore || loadingPast,
    onLoadMore: () => {
      if (nextBefore != null && !loadingMore && !loadingPast) {
        loadPast(nextBefore);
      }
    },
    disabled: scope !== 'past',
  });

  const loading = scope === 'upcoming' ? upcoming.loading : loadingPast;
  const error = scope === 'upcoming' ? (offlineSnap ? null : upcoming.error) : pastError;

  const dateGroups = useMemo(() => {
    const activeRides = scope === 'upcoming' ? upcomingList ?? offlineSnap?.rides ?? [] : pastItems;
    if (activeRides.length === 0) return [];
    return groupRidesByDate(activeRides, ft, t);
  }, [scope, upcomingList, offlineSnap?.rides, pastItems, ft, t]);

  return (
    <Page width="medium">
      <div className="stack">
        <PageHeader
          title={t('title')}
          actions={
            can('rides', 'create') ? (
              <div className="row nowrap gap-xs">
                <Button variant="secondary" size="sm" icon={<Repeat size={16} />} onClick={() => setShowRecurring(true)}>
                  {t('recurringBtn')}
                </Button>
                <Button variant="primary" to="/rides/new" icon={<CalendarPlus size={20} />}>
                  {t('signupRide')}
                </Button>
              </div>
            ) : undefined
          }
        />

        {offlineSnap && (
          <Notice tone="warning">
            <span>{t('offlineSnapshot', { when: fmtAgo(offlineSnap.savedAt, ft) })}</span>
          </Notice>
        )}

        <Tabs
          value={scope}
          onChange={(v) => setScope(v as 'upcoming' | 'past')}
          items={[
            { value: 'upcoming', label: t('upcoming') },
            { value: 'past', label: t('past') },
          ]}
        />

        <ErrorBox error={error} />

        {loading ? (
          <ListSkeleton count={4} />
        ) : dateGroups.length > 0 ? (
          <div className="rides-grouped-list">
            {dateGroups.map((group) => (
              <section key={group.dateKey} className="rides-date-section" aria-label={group.label}>
                <h2 className="sticky-date-header">{group.label}</h2>
                <div className="list">
                  {group.rides.map((r) => (
                    <RideCard key={r.id} ride={r} showDate={false} />
                  ))}
                </div>
              </section>
            ))}
            {scope === 'past' && (
              <InfiniteSentinel
                hasMore={nextBefore != null}
                loading={loadingMore}
                sentinelRef={sentinelRef}
                empty={!pastItems.length}
              />
            )}
          </div>
        ) : (
          <EmptyState
            icon={<CalendarPlus size={36} />}
            title={scope === 'upcoming' ? t('noUpcoming') : t('noPast')}
            description={scope === 'upcoming' ? t('desc') : undefined}
            action={
              scope === 'upcoming' && can('rides', 'create') ? (
                <Button variant="primary" to="/rides/new" icon={<CalendarPlus size={18} />}>
                  {t('signupRide')}
                </Button>
              ) : undefined
            }
          />
        )}
      </div>
      {showRecurring && (
        <RecurringPassengerModal
          onClose={() => setShowRecurring(false)}
          onGenerated={upcoming.reload}
        />
      )}
      <BackToTop />
    </Page>
  );
}

function RecurringPassengerModal({
  onClose,
  onGenerated,
}: {
  onClose: () => void;
  onGenerated: () => void;
}) {
  const { t } = useTranslation('rides');
  const { data: listRaw, reload } = useApi<RecurringSchedule[]>('/recurring-schedules');
  const { data: areasRaw } = useApi<AreaOption[]>('/areas');
  const schedules = (Array.isArray(listRaw) ? listRaw : []).filter((s) => s.kind === 'passenger');
  const areas = (Array.isArray(areasRaw) ? areasRaw : []).filter((a) => a.active);
  const { busy, error, run } = useAction();
  const [f, setF] = useState({
    label: '',
    weekday: 0,
    pickup_time: '08:30',
    area_id: '',
    pickup_address: '',
    destination: '',
    seats: 1,
  });

  const add = () => {
    run(async () => {
      await post('/recurring-schedules', {
        kind: 'passenger',
        label: f.label.trim(),
        weekday: Number(f.weekday),
        pickup_time: f.pickup_time,
        area_id: Number(f.area_id),
        pickup_address: f.pickup_address.trim(),
        destination: f.destination.trim(),
        seats: Number(f.seats) || 1,
      });
      setF({ label: '', weekday: 0, pickup_time: '08:30', area_id: '', pickup_address: '', destination: '', seats: 1 });
      await reload();
    }, t('recurringAdded'));
  };

  return (
    <Modal title={t('recurringTitle')} onClose={onClose}>
      <p className="muted small">{t('recurringDesc')}</p>
      {schedules.length === 0 ? (
        <p className="muted small">{t('recurringEmpty')}</p>
      ) : (
        <div className="stack-sm">
          {schedules.map((s) => (
            <div key={s.id} className="card flat tight row between nowrap gap-sm">
              <div className="min-w-0">
                <div className="bold small">
                  {t(`weekdays.${WEEKDAYS[s.weekday] ?? 'sun'}`)} · {s.pickup_time}
                  {s.label ? ` · ${s.label}` : ''}
                </div>
                <div className="muted tiny ellipsis">
                  {s.area_name} · {s.pickup_address} → {s.destination}
                </div>
                {s.active ? null : <Pill tone="neutral">{t('recurringPaused')}</Pill>}
              </div>
              <div className="row nowrap gap-xs">
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={busy}
                  onClick={() =>
                    run(
                      () =>
                        put(`/recurring-schedules/${s.id}`, {
                          kind: s.kind,
                          label: s.label,
                          weekday: s.weekday,
                          pickup_time: s.pickup_time,
                          area_id: s.area_id,
                          pickup_address: s.pickup_address,
                          destination: s.destination,
                          seats: s.seats,
                          active: !s.active,
                        }).then(reload),
                    )
                  }
                >
                  {s.active ? t('recurringPauseBtn') : t('recurringResumeBtn')}
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={busy}
                  onClick={() =>
                    run(async () => {
                      await post(`/recurring-schedules/${s.id}/generate`);
                      await reload();
                      onGenerated();
                    }, t('recurringGenerated'))
                  }
                >
                  {t('recurringGenerateNow')}
                </Button>
                <IconButton
                  size="sm"
                  variant="danger"
                  label={t('recurringDelete')}
                  icon={<Trash2 size={15} />}
                  onClick={() => run(() => del(`/recurring-schedules/${s.id}`).then(reload))}
                />
              </div>
            </div>
          ))}
        </div>
      )}

      <div className="divider" />
      <div className="stack-sm">
        <div className="form-grid two">
          <Field label={t('recurringDay')}>
            <select
              className="input"
              value={f.weekday}
              onChange={(e) => setF({ ...f, weekday: Number(e.target.value) })}
            >
              {WEEKDAYS.map((w, idx) => (
                <option key={w} value={idx}>{t(`weekdays.${w}`)}</option>
              ))}
            </select>
          </Field>
          <Field label={t('recurringTime')}>
            <input
              type="time"
              className="input"
              value={f.pickup_time}
              onChange={(e) => setF({ ...f, pickup_time: e.target.value })}
            />
          </Field>
          <Field label={t('recurringArea')}>
            <select
              className="input"
              value={f.area_id}
              onChange={(e) => setF({ ...f, area_id: e.target.value })}
            >
              <option value="">—</option>
              {areas.map((a) => (
                <option key={a.id} value={a.id}>{a.name}</option>
              ))}
            </select>
          </Field>
          <Field label={t('recurringLabel')}>
            <input
              className="input"
              value={f.label}
              onChange={(e) => setF({ ...f, label: e.target.value })}
              placeholder={t('recurringLabelPlaceholder')}
            />
          </Field>
        </div>
        <Field label={t('recurringPickup')}>
          <input
            className="input"
            value={f.pickup_address}
            onChange={(e) => setF({ ...f, pickup_address: e.target.value })}
          />
        </Field>
        <Field label={t('recurringDest')}>
          <input
            className="input"
            value={f.destination}
            onChange={(e) => setF({ ...f, destination: e.target.value })}
          />
        </Field>
        <ErrorBox error={error} />
        <Button
          variant="primary"
          block
          disabled={busy || !f.area_id || !f.pickup_address.trim() || !f.destination.trim()}
          onClick={add}
        >
          {t('recurringAdd')}
        </Button>
      </div>
    </Modal>
  );
}
