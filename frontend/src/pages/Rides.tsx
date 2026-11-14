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
  Page,
  PageHeader,
  Tabs,
  useInfiniteScroll,
} from '../components/ui';
import { fmtDay, localDate, todayStr, useFormatT } from '../format';
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
  const { can } = useAuth();
  const [scope, setScope] = useState<'upcoming' | 'past'>('upcoming');

  // Upcoming rides (typically bounded and refreshed)
  const upcoming = useApi<Ride[]>('/rides?scope=upcoming', 30000);

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
      const qs = before != null ? `&before=${encodeURIComponent(before)}&limit=30` : '';
      const res = await api<RidesPage | Ride[]>(`/rides?scope=past${qs}`);
      if (mine !== latestPast.current) return;
      const newItems = Array.isArray(res) ? res : res.items;
      const nb = Array.isArray(res) ? null : res.next_before;
      setPastItems((prev) => (before == null ? newItems : [...prev, ...newItems]));
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

  const activeRides = scope === 'upcoming' ? upcoming.data ?? [] : pastItems;
  const loading = scope === 'upcoming' ? upcoming.loading : loadingPast;
  const error = scope === 'upcoming' ? upcoming.error : pastError;

  const dateGroups = useMemo(() => {
    if (!activeRides || activeRides.length === 0) return [];
    return groupRidesByDate(activeRides, ft, t);
  }, [activeRides, ft, t]);

  return (
    <Page width="medium">
      <div className="stack">
        <PageHeader
          title={t('title')}
          actions={
            can('rides', 'create') ? (
              <Button variant="primary" to="/rides/new" icon={<CalendarPlus size={20} />}>
                {t('signupRide')}
              </Button>
            ) : undefined
          }
        />

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
      <BackToTop />
    </Page>
  );
}
