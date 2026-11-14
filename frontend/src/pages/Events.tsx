import { useCallback, useEffect, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { Plus } from 'lucide-react';
import { api, useApi } from '../api';
import { useAuth } from '../auth';
import {
  BackToTop,
  Button,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  InfiniteSentinel,
  Page,
  PageHeader,
  Pill,
  Tabs,
  useInfiniteScroll,
} from '../components/ui';
import { EVENT_KIND, fmtDateTime, useFormatT } from '../format';
import { useTranslation } from 'react-i18next';

export interface EventItem {
  id: number;
  title: string;
  kind: string;
  description: string;
  starts_at: string;
  location: string;
  capacity: number;
  cost_note: string;
  signup_deadline: string | null;
  status: string;
  created_by: { id: number; name: string };
  going: number;
  waitlist: number;
  spots_left: number | null;
  my_signup: { status: string; need_ride: boolean; offer_drive: boolean; attended: number | null } | null;
  groups: string;
  can_manage: boolean;
  /** May cancel it (the events page's delete permission). */
  can_cancel?: boolean;
}

interface EventsPage {
  items: EventItem[];
  next_before: string | null;
}

export default function Events() {
  const { t } = useTranslation('events');
  const ft = useFormatT();
  const { can } = useAuth();
  const [scope, setScope] = useState<'upcoming' | 'past' | 'managing'>('upcoming');
  const upcomingOrManaging = useApi<EventItem[]>(scope !== 'past' ? `/events?scope=${scope}` : null);

  const [pastEvents, setPastEvents] = useState<EventItem[]>([]);
  const [nextPast, setNextPast] = useState<string | null>(null);
  const [loadingPast, setLoadingPast] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [pastError, setPastError] = useState<string | null>(null);
  const latestPast = useRef(0);

  const loadPast = useCallback(async (before: string | null) => {
    const mine = ++latestPast.current;
    if (before == null) setLoadingPast(true);
    else setLoadingMore(true);
    try {
      const qs = before != null ? `&before=${encodeURIComponent(before)}&limit=25` : '';
      const res = await api<EventsPage | EventItem[]>(`/events?scope=past${qs}`);
      if (mine !== latestPast.current) return;
      const newItems = Array.isArray(res) ? res : res.items;
      const nb = Array.isArray(res) ? null : res.next_before;
      setPastEvents((prev) => (before == null ? newItems : [...prev, ...newItems]));
      setNextPast(nb);
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
    hasMore: nextPast != null,
    loading: loadingMore || loadingPast,
    onLoadMore: () => {
      if (nextPast != null && !loadingMore && !loadingPast) {
        loadPast(nextPast);
      }
    },
    disabled: scope !== 'past',
  });

  const activeEvents = scope === 'past' ? pastEvents : upcomingOrManaging.data ?? [];
  const loading = scope === 'past' ? loadingPast : upcomingOrManaging.loading;
  const error = scope === 'past' ? pastError : upcomingOrManaging.error;

  return (
    <Page width="medium">
      <div className="stack">
        <PageHeader
          title={t('title')}
          subtitle={t('desc')}
          actions={
            can('events', 'create') ? (
              <Button variant="primary" to="/events/new" icon={<Plus size={20} />}>
                {t('create')}
              </Button>
            ) : undefined
          }
        />

        <Tabs
          value={scope}
          onChange={(v) => setScope(v as 'upcoming' | 'past' | 'managing')}
          items={[
            { value: 'upcoming', label: t('upcoming') },
            { value: 'past', label: t('past') },
            ...(can('events', 'create') ? [{ value: 'managing', label: t('managing') }] : []),
          ]}
        />

        <ErrorBox error={error} />

        {loading ? (
          <CardSkeleton count={3} />
        ) : !activeEvents.length ? (
          <EmptyState
            title={t('none')}
            action={
              can('events', 'create') ? (
                <Button variant="primary" to="/events/new" icon={<Plus size={18} />}>
                  {t('create')}
                </Button>
              ) : undefined
            }
          />
        ) : (
          <div className="list event-list">
            {activeEvents.map((e) => {
              const d = new Date(e.starts_at);
              const monthName = d.toLocaleDateString(undefined, { month: 'short' });
              const dayNum = d.getDate();

              return (
                <Link key={e.id} to={`/events/${e.id}`} className="item event-card">
                  <div className="event-date-badge" aria-hidden="true">
                    <span className="event-date-month">{monthName}</span>
                    <span className="event-date-day">{dayNum}</span>
                  </div>
                  <div className="grow min-w-0 event-card-main">
                    <div className="bold ellipsis">{e.title}</div>
                    <div className="muted small ellipsis">
                      {fmtDateTime(e.starts_at, ft)} · {e.location}
                    </div>
                    <div className="muted small ellipsis">
                      {ft(EVENT_KIND[e.kind]?.label ?? e.kind)} · {e.groups}
                    </div>
                  </div>
                  <div className="event-card-side">
                    {e.status !== 'published' ? (
                      <Pill tone="neutral">{e.status === 'cancelled' ? t('cancelled') : t('finished')}</Pill>
                    ) : e.my_signup?.status === 'going' ? (
                      <Pill tone="success">{t('signedUp')}</Pill>
                    ) : e.my_signup?.status === 'waitlist' ? (
                      <Pill tone="warning">{t('waitlist')}</Pill>
                    ) : null}
                    {e.my_signup?.need_ride && <span className="muted small">{t('needRide')}</span>}
                    {e.status === 'published' && !e.my_signup && (
                      e.capacity > 0 && e.spots_left != null ? (
                        <span className="muted small nowrap">{e.spots_left > 0 ? t('spotsLeft', { n: e.spots_left }) : t('full')}</span>
                      ) : (
                        <span className="muted small nowrap">{e.going} {t('peopleAttend')}</span>
                      )
                    )}
                  </div>
                </Link>
              );
            })}
            {scope === 'past' && (
              <InfiniteSentinel
                hasMore={nextPast != null}
                loading={loadingMore}
                sentinelRef={sentinelRef}
                empty={!pastEvents.length}
              />
            )}
          </div>
        )}
      </div>
      <BackToTop />
    </Page>
  );
}
