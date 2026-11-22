import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { ChevronDown, Plus, QrCode } from 'lucide-react';
import { api, post, useApi } from '../api';
import { useAuth } from '../auth';
import type { Ride } from '../components/RideCard';
import {
  DayStrip,
  ScheduleTimeline,
  type DayOption,
  type TimelineNode,
} from '../components/event/EventScheduleTimeline';
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
  useAction,
  useInfiniteScroll,
} from '../components/ui';
import {
  EVENT_KIND,
  fmtDateTime,
  fmtDay,
  fmtTime,
  localDate,
  parseLocal,
  RIDE_STATUS,
  RIDE_STATUS_PASSENGER,
  todayStr,
  useFormatT,
} from '../format';
import { useTranslation } from 'react-i18next';
import { UniversalCheckinScannerModal } from '../components/event/EventScannerModals';

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
  my_signup: { status: string; need_ride: boolean; offer_drive: boolean; attended: number | null; pass_code?: string | null } | null;
  groups: string;
  can_manage: boolean;
  /** May cancel it (the events page's delete permission). */
  can_cancel?: boolean;
  /** Optional cover photo. */
  photo_file_id?: number | null;
  /** Whether the optional attendance module is enabled for this event. */
  attendance_enabled?: boolean;
  /** Whether volunteer rides for this event are claimable for mileage reimbursement. */
  mileage_claimable?: boolean;
  /** Check-in code for QR attendance (returned only to organiser). */
  checkin_code?: string | null;
}

interface EventsPage {
  items: EventItem[];
  next_before: string | null;
}

interface DatedTimelineNode {
  sortAt: string;
  dayKey: string;
  node: TimelineNode;
}

function monthParts(ym: string, locale: string) {
  const [y, m] = ym.split('-').map(Number);
  const d = new Date(y || 2026, (m || 1) - 1, 1);
  return {
    short: d.toLocaleDateString(locale, { month: 'short' }),
    long: d.toLocaleDateString(locale, { month: 'long', year: 'numeric' }),
  };
}

function dayChipParts(dayKey: string, locale: string, ft: (k: string) => string) {
  const today = todayStr();
  const tmr = new Date();
  tmr.setDate(tmr.getDate() + 1);
  const tmrStr = localDate(tmr);
  const d = parseLocal(`${dayKey}T00:00`);
  const weekday =
    dayKey === today
      ? ft('format:today')
      : dayKey === tmrStr
        ? ft('format:tomorrow')
        : d.toLocaleDateString(locale, { weekday: 'short' });
  const dateLabel = d.toLocaleDateString(locale, { day: 'numeric', month: 'short' });
  return { weekday, dateLabel };
}

export default function Events() {
  const { t, i18n } = useTranslation('events');
  const ft = useFormatT();
  const nav = useNavigate();
  const { can } = useAuth();
  const { busy: actionBusy, run } = useAction();

  const [scope, setScope] = useState<'upcoming' | 'my' | 'past' | 'managing'>('upcoming');
  const [selectedMonth, setSelectedMonth] = useState<string>('all');
  const [selectedDay, setSelectedDay] = useState<string>('all');
  const [expandedId, setExpandedId] = useState<number | null>(null);
  const [showScanModal, setShowScanModal] = useState(false);

  const listScope = scope === 'my' ? 'upcoming' : scope;
  const upcomingOrManaging = useApi<EventItem[]>(listScope !== 'past' ? `/events?scope=${listScope}` : null);
  const loadTransport = scope === 'upcoming' || scope === 'my';
  const myRidesRes = useApi<Ride[]>(can('rides') && loadTransport ? '/rides?scope=upcoming' : null);

  const today = todayStr();
  const horizonDate = new Date();
  horizonDate.setDate(horizonDate.getDate() + 60);
  const horizon = localDate(horizonDate);
  const myDrivingRes = useApi<Ride[]>(
    can('drive') && loadTransport ? `/driver/schedule?from=${today}&to=${horizon}` : null,
  );
  const openRequests = useApi<Ride[]>(can('drive') && scope === 'upcoming' ? '/driver/requests?mine_areas=true' : null);

  const myRides = useMemo(
    () =>
      (Array.isArray(myRidesRes.data) ? myRidesRes.data : []).filter(
        (r) => r.viewer?.is_passenger !== false && !['cancelled', 'no_show', 'alt_transport'].includes(r.status),
      ),
    [myRidesRes.data],
  );
  const myDrives = useMemo(
    () =>
      (Array.isArray(myDrivingRes.data) ? myDrivingRes.data : []).filter(
        (r) => !['cancelled', 'no_show'].includes(r.status),
      ),
    [myDrivingRes.data],
  );

  const ridesByEvent = useMemo(() => {
    const map = new Map<number, { outbound?: Ride; returnRide?: Ride }>();
    for (const r of myRides) {
      const eid = r.event?.id;
      if (!eid) continue;
      const entry = map.get(eid) ?? {};
      if (r.leg === 'return') {
        if (!entry.returnRide) entry.returnRide = r;
      } else if (!entry.outbound) {
        entry.outbound = r;
      }
      map.set(eid, entry);
    }
    return map;
  }, [myRides]);

  const drivesByEvent = useMemo(() => {
    const map = new Map<number, Ride[]>();
    for (const r of myDrives) {
      const eid = r.event?.id;
      if (!eid) continue;
      map.set(eid, [...(map.get(eid) ?? []), r]);
    }
    return map;
  }, [myDrives]);

  const openRequestsCountByEvent = useMemo(() => {
    const counts: Record<number, number> = {};
    for (const r of Array.isArray(openRequests.data) ? openRequests.data : []) {
      if (r.event?.id) {
        counts[r.event.id] = (counts[r.event.id] || 0) + 1;
      }
    }
    return counts;
  }, [openRequests.data]);

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
      const qs = before != null ? `&before=${encodeURIComponent(before)}&limit=25` : '&limit=25';
      const res = await api<EventsPage | EventItem[]>(`/events?scope=past${qs}`);
      if (mine !== latestPast.current) return;
      const newItems = Array.isArray(res) ? res : res.items;
      const nb = Array.isArray(res) ? null : res.next_before;
      setPastEvents((prev) => {
        if (before == null) return newItems;
        const existing = new Set(prev.map((e) => e.id));
        return [...prev, ...newItems.filter((e) => !existing.has(e.id))];
      });
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

  const activeEvents = useMemo(
    () => (scope === 'past' ? pastEvents : Array.isArray(upcomingOrManaging.data) ? upcomingOrManaging.data : []),
    [scope, pastEvents, upcomingOrManaging.data],
  );
  const loading = scope === 'past' ? loadingPast : upcomingOrManaging.loading;
  const error = scope === 'past' ? pastError : upcomingOrManaging.error;
  const dateLocale = i18n.language?.startsWith('zh') ? 'zh-CN' : 'en-GB';

  const months = useMemo(() => {
    if (scope !== 'upcoming') return [];
    const seen = new Set<string>();
    const list: string[] = [];
    for (const e of activeEvents) {
      const ym = e.starts_at.slice(0, 7);
      if (ym && !seen.has(ym)) {
        seen.add(ym);
        list.push(ym);
      }
    }
    return list;
  }, [scope, activeEvents]);

  const effectiveMonth = selectedMonth !== 'all' && months.includes(selectedMonth) ? selectedMonth : 'all';

  const monthGroups = useMemo(() => {
    const filtered =
      scope === 'upcoming' && effectiveMonth !== 'all'
        ? activeEvents.filter((e) => e.starts_at.startsWith(effectiveMonth))
        : activeEvents;
    if (scope !== 'upcoming') {
      return [{ monthKey: 'all', items: filtered }];
    }
    const groups: { monthKey: string; items: EventItem[] }[] = [];
    for (const e of filtered) {
      const ym = e.starts_at.slice(0, 7) || 'other';
      const last = groups[groups.length - 1];
      if (!last || last.monthKey !== ym) {
        groups.push({ monthKey: ym, items: [e] });
      } else {
        last.items.push(e);
      }
    }
    return groups;
  }, [scope, effectiveMonth, activeEvents]);

  const myScheduleGroups = useMemo(() => {
    if (scope !== 'my') return [];
    const nodes: DatedTimelineNode[] = [];
    const seenRideIds = new Set<number>();

    const myUpcomingEvents = activeEvents.filter(
      (e) => e.my_signup && e.my_signup.status !== 'cancelled',
    );
    for (const e of myUpcomingEvents) {
      const ridePair = ridesByEvent.get(e.id);
      const drives = drivesByEvent.get(e.id) ?? [];
      const hasTransport = Boolean(ridePair?.outbound || ridePair?.returnRide || drives.length > 0);
      const going = e.my_signup?.status === 'going';
      const action =
        e.status === 'published' && going && !hasTransport && (can('rides', 'create') || can('drive', 'create')) ? (
          <div className="btn-row">
            {can('rides', 'create') && (
              <Button variant="secondary" size="sm" to={`/rides/new?event=${e.id}`}>
                {t('needRideBtn')}
              </Button>
            )}
            {can('drive', 'create') && (
              <Button variant="ghost" size="sm" to={`/events/${e.id}`}>
                {t('offerDriveBtn')}
              </Button>
            )}
          </div>
        ) : undefined;

      nodes.push({
        sortAt: e.starts_at,
        dayKey: e.starts_at.slice(0, 10),
        node: {
          id: `event-${e.id}`,
          timeLabel: fmtTime(e.starts_at, ft),
          title: e.title,
          subtitle: `${e.location}${going && !hasTransport ? ` · ${t('transportNotArranged')}` : ''}`,
          to: `/events/${e.id}`,
          kind: 'event',
          state: going ? 'active' : 'upcoming',
          badge: going ? (
            <Pill tone="success">{t('signedUp')}</Pill>
          ) : (
            <Pill tone="warning">{t('waitlist')}</Pill>
          ),
          action,
        },
      });
    }

    for (const r of myRides) {
      if (seenRideIds.has(r.id)) continue;
      seenRideIds.add(r.id);
      const isReturn = r.leg === 'return';
      const statusText = ft(RIDE_STATUS_PASSENGER[r.status]?.label ?? r.status);
      const subtitle = r.driver?.name ? `${r.driver.name} · ${r.area.name}` : r.area.name;
      nodes.push({
        sortAt: r.pickup_at,
        dayKey: r.pickup_at.slice(0, 10),
        node: {
          id: `ride-${r.id}`,
          timeLabel: fmtTime(r.pickup_at, ft),
          title: isReturn ? t('timelineReturnRide') : t('timelineRideTo', { dest: r.destination }),
          subtitle,
          to: `/rides/${r.id}`,
          kind: isReturn ? 'return' : 'ride',
          state: r.status === 'completed' ? 'done' : r.status === 'confirmed' || r.status === 'in_progress' ? 'active' : 'upcoming',
          badge: <Pill tone={RIDE_STATUS_PASSENGER[r.status]?.tone ?? 'neutral'}>{statusText}</Pill>,
        },
      });
    }

    for (const r of myDrives) {
      const riderName = r.is_proxy && r.proxy ? r.proxy.rider_name : r.passenger.name;
      const statusText = ft(RIDE_STATUS[r.status]?.label ?? r.status);
      nodes.push({
        sortAt: r.pickup_at,
        dayKey: r.pickup_at.slice(0, 10),
        node: {
          id: `drive-${r.id}`,
          timeLabel: fmtTime(r.pickup_at, ft),
          title: t('timelineDriveFor', { name: riderName }),
          subtitle: `${r.area.name} → ${r.destination}`,
          to: `/rides/${r.id}`,
          kind: 'drive',
          state: r.status === 'completed' ? 'done' : r.status === 'confirmed' || r.status === 'in_progress' ? 'active' : 'upcoming',
          badge: <Pill tone={RIDE_STATUS[r.status]?.tone ?? 'neutral'}>{statusText}</Pill>,
        },
      });
    }

    nodes.sort((a, b) => a.sortAt.localeCompare(b.sortAt));

    const byDay = new Map<string, TimelineNode[]>();
    for (const item of nodes) {
      if (!item.dayKey) continue;
      byDay.set(item.dayKey, [...(byDay.get(item.dayKey) ?? []), item.node]);
    }
    return [...byDay.entries()].map(([dayKey, items]) => ({ dayKey, items }));
  }, [scope, activeEvents, myRides, myDrives, ridesByEvent, drivesByEvent, can, t, ft]);

  const scheduleDays: DayOption[] = useMemo(
    () =>
      myScheduleGroups.map((g) => {
        const parts = dayChipParts(g.dayKey, dateLocale, ft);
        return { key: g.dayKey, weekday: parts.weekday, dateLabel: parts.dateLabel, count: g.items.length };
      }),
    [myScheduleGroups, dateLocale, ft],
  );

  const effectiveDay =
    selectedDay !== 'all' && myScheduleGroups.some((g) => g.dayKey === selectedDay) ? selectedDay : 'all';

  const visibleScheduleGroups =
    effectiveDay === 'all' ? myScheduleGroups : myScheduleGroups.filter((g) => g.dayKey === effectiveDay);

  const handleQuickSignUp = async (eventId: number) => {
    const ok = await run(
      () => post(`/events/${eventId}/signup`, {}).then(() => upcomingOrManaging.reload()),
      t('signedUpSuccess'),
    );
    if (!ok) {
      nav(`/events/${eventId}`);
    }
  };

  const renderEventRow = (e: EventItem) => {
    const d = parseLocal(e.starts_at);
    const monthName = d.toLocaleDateString(dateLocale, { month: 'short' });
    const dayNum = d.getDate();
    const isExpanded = expandedId === e.id;
    const going = e.my_signup?.status === 'going';
    const signed = Boolean(e.my_signup && e.my_signup.status !== 'cancelled');
    const ridePair = ridesByEvent.get(e.id);
    const drives = drivesByEvent.get(e.id) ?? [];
    const deadlinePassed = e.signup_deadline ? parseLocal(e.signup_deadline).getTime() < Date.now() : false;
    const isFull = e.capacity > 0 && e.going >= e.capacity;
    const canQuickSignUp =
      scope === 'upcoming' &&
      e.status === 'published' &&
      !signed &&
      !deadlinePassed &&
      !isFull &&
      can('events', 'view');

    const transportSummary = (() => {
      if (!going) return null;
      if (ridePair?.outbound) {
        const statusText = ft(RIDE_STATUS_PASSENGER[ridePair.outbound.status]?.label ?? ridePair.outbound.status);
        return ridePair.outbound.driver?.name
          ? t('transportRideWithDriver', { driver: ridePair.outbound.driver.name, status: statusText })
          : t('transportRideSummary', { status: statusText });
      }
      if (drives.length > 0) {
        return t('transportDrivingCount', { count: drives.length });
      }
      return t('transportNotArranged');
    })();

    return (
      <div key={e.id} className="event-row-wrap">
        <div className="event-row">
          <Link to={`/events/${e.id}`} className="event-card-link">
            {e.photo_file_id ? (
              <img src={`/api/files/${e.photo_file_id}`} alt="" className="event-card-photo" loading="lazy" />
            ) : (
              <div className="event-date-col" aria-hidden="true">
                <span className="event-date-day">{dayNum}</span>
                <span className="event-date-month">{monthName}</span>
              </div>
            )}
            <div className="grow min-w-0 event-card-main">
              <div className="bold clamp-2">{e.title}</div>
              <div className="muted small clamp-2">
                {fmtDateTime(e.starts_at, ft)} · {e.location}
              </div>
              <div className="muted small ellipsis">
                {ft(EVENT_KIND[e.kind]?.label ?? e.kind)}{e.groups ? ` · ${e.groups}` : ''}
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
              {e.my_signup?.need_ride && !ridePair?.outbound && drives.length === 0 ? (
                <span className="muted small">{t('needRide')}</span>
              ) : scope === 'upcoming' && can('drive') && openRequestsCountByEvent[e.id] ? (
                <span className="small text-primary bold">{t('needRideCount', { count: openRequestsCountByEvent[e.id] })}</span>
              ) : e.status === 'published' && !e.my_signup ? (
                e.capacity > 0 && e.spots_left != null ? (
                  <span className="muted small nowrap">{e.spots_left > 0 ? t('spotsLeft', { n: e.spots_left }) : t('full')}</span>
                ) : (
                  <span className="muted small nowrap">{e.going} {t('peopleAttend')}</span>
                )
              ) : null}
            </div>
          </Link>

          {scope === 'upcoming' && (
            <button
              type="button"
              className="icon-btn sm event-expand-btn"
              aria-expanded={isExpanded}
              aria-label={isExpanded ? t('collapseDetails') : t('expandDetails')}
              onClick={() => setExpandedId((cur) => (cur === e.id ? null : e.id))}
            >
              <ChevronDown size={18} aria-hidden="true" />
            </button>
          )}
        </div>

        {scope === 'upcoming' && isExpanded && (
          <div className="event-row-drawer">
            {(e.description || e.cost_note) && (
              <p className="muted small clamp-2">
                {[e.description, e.cost_note].filter(Boolean).join(' · ')}
              </p>
            )}
            {transportSummary && <p className="small">{transportSummary}</p>}
            <div className="btn-row">
              {canQuickSignUp ? (
                <Button variant="primary" size="sm" disabled={actionBusy} onClick={() => handleQuickSignUp(e.id)}>
                  {t('quickSignUp')}
                </Button>
              ) : going && ridePair?.outbound ? (
                <>
                  <Button variant="primary" size="sm" to={`/rides/${ridePair.outbound.id}`}>
                    {t('openRide')}
                  </Button>
                  {ridePair.returnRide && ridePair.returnRide.id !== ridePair.outbound.id && (
                    <Button variant="secondary" size="sm" to={`/rides/${ridePair.returnRide.id}`}>
                      {t('openReturnRide')}
                    </Button>
                  )}
                </>
              ) : going && drives.length > 0 ? (
                <Button variant="primary" size="sm" to="/drive">
                  {t('openItinerary')}
                </Button>
              ) : going && e.status === 'published' ? (
                <>
                  {can('rides', 'create') && (
                    <Button variant="primary" size="sm" to={`/rides/new?event=${e.id}`}>
                      {t('needRideBtn')}
                    </Button>
                  )}
                  {can('drive', 'create') && (
                    <Button variant="secondary" size="sm" to={`/events/${e.id}`}>
                      {t('offerDriveBtn')}
                    </Button>
                  )}
                </>
              ) : null}
              <Button variant="ghost" size="sm" to={`/events/${e.id}`}>
                {t('viewDetails')}
              </Button>
            </div>
          </div>
        )}
      </div>
    );
  };

  return (
    <Page width="medium">
      <div className="stack">
        <PageHeader
          title={t('title')}
          subtitle={t('desc')}
          actions={
            <div className="page-head-action-pair">
              <Button
                variant="ghost"
                size="sm"
                icon={<QrCode size={16} />}
                onClick={() => setShowScanModal(true)}
              >
                <span className="page-head-action-label">{t('scanToAttend', { ns: 'eventDetail' })}</span>
              </Button>
              {can('events', 'create') && (
                <Button variant="primary" to="/events/new" icon={<Plus size={18} />}>
                  <span className="page-head-action-label">{t('create')}</span>
                </Button>
              )}
            </div>
          }
        />

        <Tabs
          value={scope}
          onChange={(v) => setScope(v as 'upcoming' | 'my' | 'past' | 'managing')}
          items={[
            { value: 'upcoming', label: t('upcoming') },
            { value: 'my', label: t('mySchedule') },
            { value: 'past', label: t('past') },
            ...(can('events', 'create') ? [{ value: 'managing', label: t('managing') }] : []),
          ]}
        />

        {scope === 'upcoming' && months.length >= 2 && (
          <div className="events-month-bar" data-no-swipe-back data-no-swipe-next>
            <button
              type="button"
              className={`chip ${effectiveMonth === 'all' ? 'on' : ''}`}
              aria-pressed={effectiveMonth === 'all'}
              onClick={() => setSelectedMonth('all')}
            >
              {t('allMonths')}
            </button>
            {months.map((ym) => (
              <button
                key={ym}
                type="button"
                className={`chip ${effectiveMonth === ym ? 'on' : ''}`}
                aria-pressed={effectiveMonth === ym}
                onClick={() => setSelectedMonth(ym)}
              >
                {monthParts(ym, dateLocale).short}
              </button>
            ))}
          </div>
        )}

        <ErrorBox
          error={error}
          onRetry={() => (scope === 'past' ? loadPast(null) : upcomingOrManaging.reload())}
        />

        {loading ? (
          <CardSkeleton count={3} />
        ) : scope === 'my' ? (
          myScheduleGroups.length === 0 ? (
            <EmptyState
              title={t('emptyScheduleTitle')}
              description={t('emptyScheduleDesc')}
              action={
                <Button variant="primary" onClick={() => setScope('upcoming')}>
                  {t('browseUpcoming')}
                </Button>
              }
            />
          ) : (
            <div className="stack">
              {scheduleDays.length >= 2 && (
                <DayStrip
                  days={scheduleDays}
                  selectedDay={effectiveDay}
                  onSelect={setSelectedDay}
                  allLabel={t('allDays')}
                  ariaLabel={t('mySchedule')}
                />
              )}
              {visibleScheduleGroups.map((g) => (
                <section key={g.dayKey} className="stack-xs">
                  {effectiveDay === 'all' && (
                    <div className="events-month-head">{fmtDay(`${g.dayKey}T00:00`, ft)}</div>
                  )}
                  <ScheduleTimeline items={g.items} />
                </section>
              ))}
            </div>
          )
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
          <div className="stack">
            {monthGroups.map((grp) => (
              <div key={grp.monthKey} className="stack-xs">
                {scope === 'upcoming' && monthGroups.length >= 2 && grp.monthKey !== 'all' && (
                  <div className="events-month-head">{monthParts(grp.monthKey, dateLocale).long}</div>
                )}
                <div className="list">{grp.items.map(renderEventRow)}</div>
              </div>
            ))}
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
      {showScanModal && (
        <UniversalCheckinScannerModal onClose={() => setShowScanModal(false)} />
      )}
      <BackToTop />
    </Page>
  );
}
