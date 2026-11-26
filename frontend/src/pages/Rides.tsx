import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { CalendarPlus } from 'lucide-react';
import { api, useApi } from '../api';
import { useAuth } from '../auth';
import RideCard, { Ride } from '../components/RideCard';
import {
  BackToTop,
  Button,
  EmptyState,
  ErrorBox,
  InfiniteSentinel,
  ListSkeleton,
  Notice,
  Page,
  PageHeader,
  Tabs,
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

export default function Rides() {
  const { t } = useTranslation('rides');
  const ft = useFormatT();
  const { me, can } = useAuth();
  const [scope, setScope] = useState<'upcoming' | 'past'>('upcoming');

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
    const activeRides =
      scope === 'upcoming'
        ? upcomingList ?? offlineSnap?.rides.filter((r) => r.viewer?.is_passenger) ?? []
        : pastItems;
    if (activeRides.length === 0) return [];
    return groupRidesByDate(activeRides, ft, t);
  }, [scope, upcomingList, offlineSnap?.rides, pastItems, ft, t]);

  return (
    <Page width="medium">
      <div className="stack">
        <PageHeader
          title={t('title')}
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

        <ErrorBox error={error} onRetry={() => (scope === 'past' ? loadPast(null) : upcoming.reload())} />

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
                <Button variant="primary" to="/events" icon={<CalendarPlus size={18} />}>
                  {t('findEvent')}
                </Button>
              ) : undefined
            }
          />
        )}
      </div>
      <BackToTop />
    </Page>
  );
}
