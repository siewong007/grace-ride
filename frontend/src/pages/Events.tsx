import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { AlertCircle, Calendar, CarFront, Check, Eye, EyeOff, Plus, QrCode, Search } from 'lucide-react';
import { api, post, useApi } from '../api';
import { useAuth } from '../auth';
import { downloadIcs } from '../calendar';
import type { Ride } from '../components/RideCard';
import { EventManageCard } from '../components/event/EventManageCard';
import { AttendanceQrModal, MemberPassQrModal } from '../components/event/EventQrModals';
import {
  DayStrip,
  ScheduleTimeline,
  type DayOption,
  type TimelineNode,
} from '../components/event/EventScheduleTimeline';
import { EventScannerModal, UniversalCheckinScannerModal } from '../components/event/EventScannerModals';
import { EventTodayBanner } from '../components/event/EventTodayBanner';
import { QuickRsvpModal } from '../components/event/QuickRsvpModal';
import type { EventFull } from '../components/event/types';
import PostEventReimbursementModal from '../components/events/PostEventReimbursementModal';
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
  useConfirm,
  useInfiniteScroll,
  useToast,
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
import { EventIcon } from '../eventIcon';
import { useTranslation } from 'react-i18next';

export interface EventItem {
  id: number;
  title: string;
  kind: string;
  icon?: string | null;
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
  my_signup: {
    status: string;
    need_ride: boolean;
    offer_drive: boolean;
    self_transport?: boolean;
    attended: number | null;
    pass_code?: string | null;
  } | null;
  groups: string;
  can_manage: boolean;
  /** Organiser may publish this draft event. */
  can_publish?: boolean;
  /** Organiser may hide this published event back to draft (no signups/rides). */
  can_unpublish?: boolean;
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
  /** Number of attended signups (returned only to authorised event organisers/staff). */
  attended_count?: number | null;
  /** Number of pending_driver rides (returned only to authorised event organisers/staff). */
  unassigned_rides?: number | null;
}

interface EventsPage {
  items: EventItem[];
  next_before: string | null;
}

type ScopeTab = 'upcoming' | 'my' | 'past' | 'managing';
type UpcomingFilter = 'all' | 'open' | 'going' | 'needs_transport' | 'rides_needed';

interface DatedTimelineNode {
  sortAt: string;
  node: TimelineNode;
}

interface OutingBlock {
  id: string;
  sortAt: string;
  event?: EventItem;
  items: TimelineNode[];
}

interface QuickRsvpState {
  event: EventItem;
  step: 'group' | 'transport';
  loadingChoices: boolean;
  joinChoices?: { id: number; name: string }[];
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
  const { can, me } = useAuth();
  const toast = useToast();
  const confirm = useConfirm();
  const [searchParams, setSearchParams] = useSearchParams();

  const canCreateEvent = can('events', 'create');
  const canDrive = can('drive');
  const canRides = can('rides');

  const rawTab = searchParams.get('tab');
  const scope: ScopeTab =
    rawTab === 'my' || rawTab === 'past' || (rawTab === 'managing' && canCreateEvent)
      ? rawTab
      : 'upcoming';

  const rawFilter = searchParams.get('filter');
  const statusFilter: UpcomingFilter =
    rawFilter === 'open' ||
    rawFilter === 'going' ||
    rawFilter === 'needs_transport' ||
    (rawFilter === 'rides_needed' && canDrive)
      ? rawFilter
      : 'all';

  const selectedMonth = searchParams.get('month') || 'all';
  const selectedDay = searchParams.get('day') || 'all';
  const pastQuery = searchParams.get('q') ?? '';
  const pastAttendedOnly = searchParams.get('attended') === '1';
  const hideDrafts = searchParams.get('hideDrafts') === '1';

  const updateParams = useCallback(
    (patch: Record<string, string | null>) => {
      setSearchParams(
        (prev) => {
          const next = new URLSearchParams(prev);
          for (const [k, v] of Object.entries(patch)) {
            if (v == null || v === '') next.delete(k);
            else next.set(k, v);
          }
          return next;
        },
        { replace: true },
      );
    },
    [setSearchParams],
  );

  const setScope = (next: ScopeTab) => updateParams({ tab: next === 'upcoming' ? null : next });

  const [actionBusy, setActionBusy] = useState(false);
  const [showUniversalScan, setShowUniversalScan] = useState(false);
  const [scanEvent, setScanEvent] = useState<EventFull | null>(null);
  const [passModalEvent, setPassModalEvent] = useState<EventItem | null>(null);
  const [qrManageEvent, setQrManageEvent] = useState<EventItem | null>(null);
  const [reimbEventId, setReimbEventId] = useState<number | null>(null);
  const [quickRsvp, setQuickRsvp] = useState<QuickRsvpState | null>(null);

  const upcomingRes = useApi<EventItem[]>('/events?scope=upcoming');
  const managingRes = useApi<EventItem[]>(canCreateEvent ? '/events?scope=managing' : null);

  const loadTransport = scope === 'upcoming' || scope === 'my';
  const myRidesRes = useApi<Ride[]>(canRides && loadTransport ? '/rides?scope=upcoming' : null);

  const today = todayStr();
  const horizonDate = new Date();
  horizonDate.setDate(horizonDate.getDate() + 60);
  const horizon = localDate(horizonDate);
  const myDrivingRes = useApi<Ride[]>(
    canDrive && loadTransport ? `/driver/schedule?from=${today}&to=${horizon}` : null,
  );
  const openRequests = useApi<Ride[]>(
    canDrive && scope === 'upcoming' ? '/driver/requests?mine_areas=true' : null,
  );

  const myRides = useMemo(
    () =>
      (Array.isArray(myRidesRes.data) ? myRidesRes.data : []).filter(
        (r) =>
          r.viewer?.is_passenger !== false &&
          !['cancelled', 'no_show', 'alt_transport'].includes(r.status),
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
  const [pastLoaded, setPastLoaded] = useState(false);
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
      setPastLoaded(true);
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

  const upcomingLoaded = Array.isArray(upcomingRes.data);
  const upcomingEvents = useMemo(
    () => (Array.isArray(upcomingRes.data) ? upcomingRes.data : []),
    [upcomingRes.data],
  );
  const managingEvents = useMemo(
    () => (Array.isArray(managingRes.data) ? managingRes.data : []),
    [managingRes.data],
  );
  const draftManagingCount = useMemo(
    () => managingEvents.filter((e) => e.status === 'draft').length,
    [managingEvents],
  );
  const visibleManagingEvents = useMemo(
    () => (hideDrafts ? managingEvents.filter((e) => e.status !== 'draft') : managingEvents),
    [managingEvents, hideDrafts],
  );

  const myUpcomingEvents = useMemo(
    () => upcomingEvents.filter((e) => e.my_signup && e.my_signup.status !== 'cancelled'),
    [upcomingEvents],
  );

  const loading =
    scope === 'past'
      ? loadingPast && !pastLoaded
      : scope === 'managing'
        ? managingRes.loading && !Array.isArray(managingRes.data)
        : upcomingRes.loading && !upcomingLoaded;

  const error =
    scope === 'past' ? pastError : scope === 'managing' ? managingRes.error : upcomingRes.error;

  const dateLocale = i18n.language?.startsWith('zh') ? 'zh-CN' : 'en-GB';

  const getTransportSummary = useCallback(
    (e: EventItem): string | null => {
      if (e.my_signup?.status !== 'going') return null;
      const ridePair = ridesByEvent.get(e.id);
      const drives = drivesByEvent.get(e.id) ?? [];
      if (ridePair?.outbound) {
        const statusText = ft(
          RIDE_STATUS_PASSENGER[ridePair.outbound.status]?.label ?? ridePair.outbound.status,
        );
        return ridePair.outbound.driver?.name
          ? t('transportRideWithDriver', { driver: ridePair.outbound.driver.name, status: statusText })
          : t('transportRideSummary', { status: statusText });
      }
      if (drives.length > 0) {
        return t('transportDrivingCount', { count: drives.length });
      }
      return e.my_signup?.self_transport ? t('transportSelf') : t('transportNotArranged');
    },
    [ridesByEvent, drivesByEvent, ft, t],
  );

  const todayEvent = useMemo(
    () =>
      upcomingEvents.find(
        (e) =>
          e.status === 'published' &&
          e.my_signup?.status === 'going' &&
          e.starts_at.slice(0, 10) === today,
      ) ?? null,
    [upcomingEvents, today],
  );

  const todayBannerVisible = Boolean(todayEvent) && (scope === 'upcoming' || scope === 'my');
  const todayBannerHasScan = Boolean(
    todayBannerVisible &&
      todayEvent &&
      todayEvent.attendance_enabled !== false &&
      todayEvent.my_signup?.attended !== 1,
  );
  const hasAnyAttendanceEvent =
    !upcomingLoaded || upcomingEvents.some((e) => e.attendance_enabled !== false);
  const showHeaderScan = hasAnyAttendanceEvent && !todayBannerHasScan;

  const ridesNeededEventCount = useMemo(() => {
    if (!canDrive) return 0;
    return upcomingEvents.filter((e) => (openRequestsCountByEvent[e.id] ?? 0) > 0).length;
  }, [canDrive, upcomingEvents, openRequestsCountByEvent]);

  const months = useMemo(() => {
    if (scope !== 'upcoming') return [];
    const seen = new Set<string>();
    const list: string[] = [];
    for (const e of upcomingEvents) {
      const ym = e.starts_at.slice(0, 7);
      if (ym && !seen.has(ym)) {
        seen.add(ym);
        list.push(ym);
      }
    }
    return list;
  }, [scope, upcomingEvents]);

  const effectiveMonth =
    selectedMonth !== 'all' && months.includes(selectedMonth) ? selectedMonth : 'all';

  const hasTransportForEvent = useCallback(
    (e: EventItem) => {
      const pair = ridesByEvent.get(e.id);
      const drives = drivesByEvent.get(e.id) ?? [];
      return Boolean(pair?.outbound || pair?.returnRide || drives.length > 0);
    },
    [ridesByEvent, drivesByEvent],
  );

  const filteredUpcoming = useMemo(() => {
    return upcomingEvents.filter((e) => {
      if (effectiveMonth !== 'all' && !e.starts_at.startsWith(effectiveMonth)) return false;
      if (statusFilter === 'open') {
        const signed = Boolean(e.my_signup && e.my_signup.status !== 'cancelled');
        const deadlinePassed = e.signup_deadline
          ? parseLocal(e.signup_deadline).getTime() < Date.now()
          : false;
        const isFull = e.capacity > 0 && e.going >= e.capacity;
        return e.status === 'published' && !signed && !deadlinePassed && !isFull;
      }
      if (statusFilter === 'going') {
        return e.my_signup?.status === 'going';
      }
      if (statusFilter === 'needs_transport') {
        return (
          e.status === 'published' &&
          e.my_signup?.status === 'going' &&
          !hasTransportForEvent(e) &&
          !e.my_signup?.self_transport
        );
      }
      if (statusFilter === 'rides_needed') {
        return (openRequestsCountByEvent[e.id] ?? 0) > 0;
      }
      return true;
    });
  }, [
    upcomingEvents,
    effectiveMonth,
    statusFilter,
    hasTransportForEvent,
    openRequestsCountByEvent,
  ]);

  const monthGroups = useMemo(() => {
    if (scope !== 'upcoming') return [];
    const groups: { monthKey: string; items: EventItem[] }[] = [];
    for (const e of filteredUpcoming) {
      const ym = e.starts_at.slice(0, 7) || 'other';
      const last = groups[groups.length - 1];
      if (!last || last.monthKey !== ym) {
        groups.push({ monthKey: ym, items: [e] });
      } else {
        last.items.push(e);
      }
    }
    return groups;
  }, [scope, filteredUpcoming]);

  const myScheduleDays = useMemo(() => {
    if (scope !== 'my') return [];

    const consumedRideIds = new Set<number>();
    const consumedDriveIds = new Set<number>();
    const blocksByDay = new Map<string, OutingBlock[]>();

    const toRideNode = (r: Ride): DatedTimelineNode => {
      const isReturn = r.leg === 'return';
      const statusText = ft(RIDE_STATUS_PASSENGER[r.status]?.label ?? r.status);
      const subtitle = r.driver?.name ? `${r.driver.name} · ${r.area.name}` : r.area.name;
      return {
        sortAt: r.pickup_at,
        node: {
          id: `ride-${r.id}`,
          timeLabel: fmtTime(r.pickup_at, ft),
          title: isReturn ? t('timelineReturnRide') : t('timelineRideTo', { dest: r.destination }),
          subtitle,
          to: `/rides/${r.id}`,
          kind: isReturn ? 'return' : 'ride',
          state:
            r.status === 'completed'
              ? 'done'
              : r.status === 'confirmed' || r.status === 'in_progress'
                ? 'active'
                : 'upcoming',
          badge: <Pill tone={RIDE_STATUS_PASSENGER[r.status]?.tone ?? 'neutral'}>{statusText}</Pill>,
        },
      };
    };

    const toDriveNode = (r: Ride): DatedTimelineNode => {
      const riderName = r.is_proxy && r.proxy ? r.proxy.rider_name : r.passenger.name;
      const statusText = ft(RIDE_STATUS[r.status]?.label ?? r.status);
      return {
        sortAt: r.pickup_at,
        node: {
          id: `drive-${r.id}`,
          timeLabel: fmtTime(r.pickup_at, ft),
          title: t('timelineDriveFor', { name: riderName }),
          subtitle: `${r.area.name} → ${r.destination}`,
          to: `/rides/${r.id}`,
          kind: 'drive',
          state:
            r.status === 'completed'
              ? 'done'
              : r.status === 'confirmed' || r.status === 'in_progress'
                ? 'active'
                : 'upcoming',
          badge: <Pill tone={RIDE_STATUS[r.status]?.tone ?? 'neutral'}>{statusText}</Pill>,
        },
      };
    };

    for (const e of myUpcomingEvents) {
      const dayKey = e.starts_at.slice(0, 10);
      if (!dayKey) continue;

      const linkedRides = myRides.filter((r) => r.event?.id === e.id);
      const linkedDrives = myDrives.filter((r) => r.event?.id === e.id);
      for (const r of linkedRides) consumedRideIds.add(r.id);
      for (const r of linkedDrives) consumedDriveIds.add(r.id);

      const hasTransport = linkedRides.length > 0 || linkedDrives.length > 0;
      const selfTransport = !hasTransport && Boolean(e.my_signup?.self_transport);
      const going = e.my_signup?.status === 'going';
      const attendanceEnabled = e.attendance_enabled !== false;
      const attended = attendanceEnabled && e.my_signup?.attended === 1;
      const canShowPass = attendanceEnabled && going && !attended && Boolean(e.my_signup?.pass_code);

      const action =
        canShowPass ||
        (e.status === 'published' && going && !hasTransport && !selfTransport && (can('rides', 'create') || can('drive', 'create'))) ? (
          <div className="btn-row">
            {canShowPass && (
              <Button
                variant="secondary"
                size="sm"
                icon={<QrCode size={14} />}
                onClick={() => setPassModalEvent(e)}
              >
                {t('showMyPass')}
              </Button>
            )}
            {e.status === 'published' && going && !hasTransport && !selfTransport && can('rides', 'create') && (
              <Button variant="primary" size="sm" to={`/rides/new?event=${e.id}`}>
                {t('needRideBtn')}
              </Button>
            )}
            {e.status === 'published' && going && !hasTransport && !selfTransport && can('drive', 'create') && (
              <Button variant="ghost" size="sm" to={`/events/${e.id}?drive=1`}>
                {t('offerDriveBtn')}
              </Button>
            )}
          </div>
        ) : undefined;

      const datedNodes: DatedTimelineNode[] = [
        ...linkedRides.map(toRideNode),
        ...linkedDrives.map(toDriveNode),
        {
          sortAt: e.starts_at,
          node: {
            id: `event-${e.id}`,
            timeLabel: fmtTime(e.starts_at, ft),
            title: e.title,
            subtitle: `${e.location}${going && !hasTransport ? ` · ${selfTransport ? t('transportSelf') : t('transportNotArranged')}` : ''}`,
            to: `/events/${e.id}`,
            kind: 'event',
            icon: <EventIcon icon={e.icon} kind={e.kind} size={20} />,
            state: attended ? 'done' : going ? 'active' : 'upcoming',
            badge: attended ? (
              <Pill tone="success">{t('statusAttended', { ns: 'eventDetail' })}</Pill>
            ) : going ? (
              <Pill tone="success">{t('signedUp')}</Pill>
            ) : (
              <Pill tone="warning">{t('waitlist')}</Pill>
            ),
            action,
          },
        },
      ];

      datedNodes.sort((a, b) => a.sortAt.localeCompare(b.sortAt));

      const block: OutingBlock = {
        id: `outing-${e.id}`,
        sortAt: datedNodes[0]?.sortAt ?? e.starts_at,
        event: e,
        items: datedNodes.map((d) => d.node),
      };
      blocksByDay.set(dayKey, [...(blocksByDay.get(dayKey) ?? []), block]);
    }

    const standaloneByDay = new Map<string, DatedTimelineNode[]>();
    for (const r of myRides) {
      if (consumedRideIds.has(r.id)) continue;
      consumedRideIds.add(r.id);
      const dk = r.pickup_at.slice(0, 10);
      if (!dk) continue;
      standaloneByDay.set(dk, [...(standaloneByDay.get(dk) ?? []), toRideNode(r)]);
    }
    for (const r of myDrives) {
      if (consumedDriveIds.has(r.id)) continue;
      consumedDriveIds.add(r.id);
      const dk = r.pickup_at.slice(0, 10);
      if (!dk) continue;
      standaloneByDay.set(dk, [...(standaloneByDay.get(dk) ?? []), toDriveNode(r)]);
    }
    for (const [dk, nodes] of standaloneByDay.entries()) {
      nodes.sort((a, b) => a.sortAt.localeCompare(b.sortAt));
      const block: OutingBlock = {
        id: `standalone-${dk}`,
        sortAt: nodes[0]?.sortAt ?? dk,
        items: nodes.map((n) => n.node),
      };
      blocksByDay.set(dk, [...(blocksByDay.get(dk) ?? []), block]);
    }

    return [...blocksByDay.entries()]
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([dayKey, blocks]) => {
        const sortedBlocks = [...blocks].sort((a, b) => a.sortAt.localeCompare(b.sortAt));
        const count = sortedBlocks.reduce((sum, blk) => sum + blk.items.length, 0);
        return { dayKey, blocks: sortedBlocks, count };
      });
  }, [scope, myUpcomingEvents, myRides, myDrives, can, t, ft]);

  const scheduleDays: DayOption[] = useMemo(
    () =>
      myScheduleDays.map((g) => {
        const parts = dayChipParts(g.dayKey, dateLocale, ft);
        return { key: g.dayKey, weekday: parts.weekday, dateLabel: parts.dateLabel, count: g.count };
      }),
    [myScheduleDays, dateLocale, ft],
  );

  const effectiveDay =
    selectedDay !== 'all' && myScheduleDays.some((g) => g.dayKey === selectedDay)
      ? selectedDay
      : 'all';

  const visibleScheduleDays =
    effectiveDay === 'all'
      ? myScheduleDays
      : myScheduleDays.filter((g) => g.dayKey === effectiveDay);

  const filteredPast = useMemo(() => {
    const q = pastQuery.trim().toLowerCase();
    return pastEvents.filter((e) => {
      if (pastAttendedOnly && (e.attendance_enabled === false || e.my_signup?.attended !== 1)) return false;
      if (q) {
        const hay = `${e.title} ${e.location} ${e.groups}`.toLowerCase();
        if (!hay.includes(q)) return false;
      }
      return true;
    });
  }, [pastEvents, pastQuery, pastAttendedOnly]);

  const handleQuickSignUp = async (e: EventItem) => {
    setActionBusy(true);
    try {
      const res = await post<{ ok?: boolean; status?: string }>(`/events/${e.id}/signup`, {});
      upcomingRes.reload();
      if (res?.status === 'waitlist') {
        toast.success(t('joinedWaitlistSuccess'));
      } else {
        toast.success(t('signedUpSuccess'));
        if (can('rides', 'create') || can('drive', 'create')) {
          setQuickRsvp({ event: e, step: 'transport', loadingChoices: false });
        }
      }
    } catch (err: unknown) {
      const code =
        typeof err === 'object' && err !== null && 'code' in err
          ? (err as { code?: string }).code
          : undefined;
      if (code === 'JOIN_AS_REQUIRED') {
        setQuickRsvp({ event: e, step: 'group', loadingChoices: true });
        try {
          const detail = await api<EventFull>(`/events/${e.id}`);
          setQuickRsvp({
            event: e,
            step: 'group',
            loadingChoices: false,
            joinChoices: detail.join_as ?? [],
          });
        } catch (fetchErr) {
          setQuickRsvp(null);
          toast.error((fetchErr as Error).message);
        }
      } else {
        toast.error((err as Error).message);
      }
    } finally {
      setActionBusy(false);
    }
  };

  const handleSetSelfTransport = async (e: EventItem) => {
    setActionBusy(true);
    try {
      await post(`/events/${e.id}/transport`, { self_transport: true });
      upcomingRes.reload();
      toast.success(t('goingOwnWaySaved', { ns: 'eventDetail' }));
    } catch (err) {
      toast.error((err as Error).message);
    } finally {
      setActionBusy(false);
    }
  };

  const handlePublishEvent = async (ev: EventItem) => {
    setActionBusy(true);
    try {
      await post(`/events/${ev.id}/publish`, {});
      managingRes.reload();
      upcomingRes.reload();
      toast.success(t('publishSuccess'));
    } catch (err) {
      toast.error((err as Error).message);
    } finally {
      setActionBusy(false);
    }
  };

  const handleUnpublishEvent = async (ev: EventItem) => {
    const ok = await confirm({
      title: t('unpublishConfirmTitle'),
      body: t('unpublishConfirmBody'),
      confirmLabel: t('hideToDraftBtn'),
    });
    if (!ok) return;
    setActionBusy(true);
    try {
      await post(`/events/${ev.id}/unpublish`, {});
      managingRes.reload();
      upcomingRes.reload();
      toast.success(t('unpublishSuccess'));
    } catch (err) {
      toast.error((err as Error).message);
    } finally {
      setActionBusy(false);
    }
  };

  const renderEventRow = (e: EventItem) => {
    const d = parseLocal(e.starts_at);
    const monthName = d.toLocaleDateString(dateLocale, { month: 'short' });
    const dayNum = d.getDate();
    const going = e.my_signup?.status === 'going';
    const signed = Boolean(e.my_signup && e.my_signup.status !== 'cancelled');
    const ridePair = ridesByEvent.get(e.id);
    const drives = drivesByEvent.get(e.id) ?? [];
    const hasTransport = Boolean(ridePair?.outbound || ridePair?.returnRide || drives.length > 0);
    const selfTransport = !hasTransport && Boolean(e.my_signup?.self_transport);
    const deadlineMs = e.signup_deadline ? parseLocal(e.signup_deadline).getTime() : null;
    const nowMs = Date.now();
    const deadlinePassed = deadlineMs != null ? deadlineMs < nowMs : false;
    const isFull = e.capacity > 0 && e.going >= e.capacity;
    const canQuickSignUp =
      scope === 'upcoming' &&
      e.status === 'published' &&
      !signed &&
      !deadlinePassed &&
      can('events', 'view');

    const isClosingSoon =
      scope === 'upcoming' &&
      !signed &&
      !deadlinePassed &&
      deadlineMs != null &&
      deadlineMs - nowMs <= 24 * 60 * 60 * 1000;
    const urgencyLabel =
      isClosingSoon && e.signup_deadline
        ? e.signup_deadline.slice(0, 10) === today
          ? t('closesToday')
          : t('closesSoon', { time: fmtTime(e.signup_deadline, ft) })
        : null;

    const transportSummary = getTransportSummary(e);
    const openReqCount = scope === 'upcoming' && canDrive ? openRequestsCountByEvent[e.id] ?? 0 : 0;

    const spotsCue =
      !signed && e.status === 'published' && e.capacity > 0 && e.spots_left != null
        ? e.spots_left > 0
          ? t('spotsLeft', { n: e.spots_left })
          : t('full')
        : null;
    const spotsUrgent =
      !signed &&
      e.status === 'published' &&
      e.capacity > 0 &&
      e.spots_left != null &&
      e.spots_left > 0 &&
      e.spots_left <= 5;
    const waitlistCue =
      !signed && e.status === 'published' && isFull && e.waitlist > 0
        ? t('waitlistCount', { count: e.waitlist })
        : null;
    const showCues = Boolean(urgencyLabel || spotsCue || waitlistCue || e.cost_note);
    const pastAttended =
      scope === 'past' &&
      e.status !== 'published' &&
      e.attendance_enabled !== false &&
      e.my_signup?.attended === 1;
    const outboundConfirmed = Boolean(
      ridePair?.outbound &&
        ['confirmed', 'in_progress', 'completed'].includes(ridePair.outbound.status),
    );

    return (
      <article key={e.id} className="event-row-wrap">
        <div className="event-row">
          <Link to={`/events/${e.id}`} className="event-date-link" tabIndex={-1} aria-hidden="true">
            <div className="event-date-col">
              <span className="event-date-day">{dayNum}</span>
              <span className="event-date-month">{monthName}</span>
            </div>
          </Link>
          <div className="event-row-body">
            <div className="event-card-top">
              <Link to={`/events/${e.id}`} className="event-card-info">
                <div className="bold clamp-2 event-card-title">{e.title}</div>
                <div className="muted small clamp-2">
                  {scope === 'past' ? fmtDateTime(e.starts_at, ft) : fmtTime(e.starts_at, ft)} ·{' '}
                  {e.location}
                </div>
                <div className="muted small ellipsis">
                  <span className="event-kind-inline">
                    <EventIcon icon={e.icon} kind={e.kind} size={20} />
                    <span>{ft(EVENT_KIND[e.kind]?.label ?? e.kind)}</span>
                  </span>
                  {e.groups ? ` · ${e.groups}` : ''}
                </div>
                {showCues && (
                  <div className="small muted event-card-cues">
                    {urgencyLabel && <span className="event-deadline-urgent">{urgencyLabel}</span>}
                    {spotsCue && (
                      <span className={spotsUrgent ? 'event-deadline-urgent' : undefined}>
                        {spotsCue}
                      </span>
                    )}
                    {waitlistCue && <span>{waitlistCue}</span>}
                    {e.cost_note && <span>{e.cost_note}</span>}
                  </div>
                )}
              </Link>
              <div className="event-card-side">
                {e.status !== 'published' ? (
                  pastAttended ? (
                    <Pill tone="success">{t('statusAttended', { ns: 'eventDetail' })}</Pill>
                  ) : (
                    <Pill tone="neutral">{e.status === 'cancelled' ? t('cancelled') : t('finished')}</Pill>
                  )
                ) : e.my_signup?.status === 'going' ? (
                  <Pill tone="success">{t('signedUp')}</Pill>
                ) : e.my_signup?.status === 'waitlist' ? (
                  <Pill tone="warning">{t('waitlist')}</Pill>
                ) : null}
                {canQuickSignUp && (
                  <Button
                    variant={isFull ? 'secondary' : 'primary'}
                    size="sm"
                    className="event-row-rsvp-btn"
                    disabled={actionBusy}
                    onClick={() => handleQuickSignUp(e)}
                  >
                    {isFull ? t('joinWaitlist', { ns: 'eventDetail' }) : t('quickSignUp')}
                  </Button>
                )}
                {!canQuickSignUp && !signed && e.status === 'published' && e.capacity === 0 && (
                  <span className="muted small nowrap">
                    {e.going} {t('peopleAttend')}
                  </span>
                )}
                {e.photo_file_id && (
                  <Link to={`/events/${e.id}`} tabIndex={-1} aria-hidden="true">
                    <img
                      src={`/api/files/${e.photo_file_id}`}
                      alt=""
                      className="event-card-photo"
                      loading="lazy"
                    />
                  </Link>
                )}
              </div>
            </div>

            {scope === 'upcoming' && openReqCount > 0 && (
              <Link to={`/events/${e.id}?drive=1`} className="event-driver-chip">
                <CarFront size={14} aria-hidden="true" />
                <span>{t('needRideCount', { count: openReqCount })}</span>
              </Link>
            )}

            {scope === 'upcoming' && going && ridePair?.outbound && (
              <div className="event-transport-strip">
                <span
                  className={`small event-transport-text ${
                    outboundConfirmed ? 'is-confirmed' : 'is-pending'
                  }`}
                >
                  {outboundConfirmed && <Check size={14} aria-hidden="true" />}
                  <span>{transportSummary}</span>
                </span>
                <div className="btn-row event-transport-links">
                  <Button variant="secondary" size="sm" to={`/rides/${ridePair.outbound.id}`}>
                    {t('openRide')}
                  </Button>
                  {ridePair.returnRide && ridePair.returnRide.id !== ridePair.outbound.id && (
                    <Button variant="ghost" size="sm" to={`/rides/${ridePair.returnRide.id}`}>
                      {t('openReturnRide')}
                    </Button>
                  )}
                </div>
              </div>
            )}

            {scope === 'upcoming' && going && !ridePair?.outbound && drives.length > 0 && (
              <div className="event-transport-strip">
                <span className="small event-transport-text is-driving">
                  <CarFront size={14} aria-hidden="true" />
                  <span>{transportSummary}</span>
                </span>
                <Button variant="secondary" size="sm" to="/drive">
                  {t('openItinerary')}
                </Button>
              </div>
            )}

            {scope === 'upcoming' && going && !hasTransport && !selfTransport && e.status === 'published' && (
              <div className="event-transport-alert">
                <span className="small bold event-transport-alert-label">
                  <AlertCircle size={15} aria-hidden="true" />
                  <span>{t('transportNotArranged')}</span>
                </span>
                <div className="btn-row event-transport-alert-actions">
                  {can('rides', 'create') && (
                    <Button variant="primary" size="sm" to={`/rides/new?event=${e.id}`}>
                      {t('needRideBtn')}
                    </Button>
                  )}
                  {can('drive', 'create') && (
                    <Button variant="secondary" size="sm" to={`/events/${e.id}?drive=1`}>
                      {t('offerDriveBtn')}
                    </Button>
                  )}
                  <Button
                    variant="ghost"
                    size="sm"
                    disabled={actionBusy}
                    onClick={() => handleSetSelfTransport(e)}
                  >
                    {t('goingOwnWay', { ns: 'eventDetail' })}
                  </Button>
                </div>
              </div>
            )}

            {scope === 'upcoming' && going && !hasTransport && selfTransport && e.status === 'published' && (
              <div className="event-transport-strip">
                <span className="small event-transport-text">
                  <Check size={14} aria-hidden="true" />
                  <span>{t('transportSelf')}</span>
                </span>
              </div>
            )}
          </div>
        </div>
      </article>
    );
  };

  const tabItems = [
    {
      value: 'upcoming',
      label: t('upcoming'),
      count: upcomingLoaded ? upcomingEvents.length : undefined,
    },
    {
      value: 'my',
      label: t('mySchedule'),
      count: upcomingLoaded ? myUpcomingEvents.length : undefined,
    },
    {
      value: 'past',
      label: t('past'),
      count: pastLoaded ? pastEvents.length : undefined,
    },
    ...(canCreateEvent
      ? [
          {
            value: 'managing',
            label: t('managing'),
            count: Array.isArray(managingRes.data) ? managingRes.data.length : undefined,
          },
        ]
      : []),
  ];

  const todayBannerEl = todayBannerVisible && todayEvent ? (
    <EventTodayBanner
      event={todayEvent}
      transportSummary={getTransportSummary(todayEvent)}
      outboundRideId={ridesByEvent.get(todayEvent.id)?.outbound?.id}
      drivingCount={(drivesByEvent.get(todayEvent.id) ?? []).length}
      canRequestRide={can('rides', 'create')}
      onShowPass={() => setPassModalEvent(todayEvent)}
      onScan={() => setScanEvent(todayEvent as EventFull)}
    />
  ) : null;

  const emptyCalendarIcon = (
    <span className="events-empty-icon">
      <Calendar size={22} />
    </span>
  );

  return (
    <Page width="medium">
      <div className="stack">
        <PageHeader
          className="events-page-head"
          title={t('title')}
          subtitle={t('desc')}
          actions={
            showHeaderScan || canCreateEvent ? (
              <div className="events-head-actions">
                {showHeaderScan && (
                  <Button
                    variant="secondary"
                    size="sm"
                    className="events-scan-icon-btn"
                    icon={<QrCode size={17} />}
                    aria-label={t('scanToAttend', { ns: 'eventDetail' })}
                    title={t('scanToAttend', { ns: 'eventDetail' })}
                    onClick={() => setShowUniversalScan(true)}
                  />
                )}
                {canCreateEvent && (
                  <Button
                    variant="primary"
                    size="sm"
                    className="events-create-btn"
                    to="/events/new"
                    icon={<Plus size={16} />}
                    aria-label={t('create')}
                  >
                    {t('createShort')}
                  </Button>
                )}
              </div>
            ) : undefined
          }
        />

        <Tabs
          className="events-tabs"
          value={scope}
          onChange={(v) => setScope(v as ScopeTab)}
          items={tabItems}
        />

        {todayBannerEl}

        {scope === 'upcoming' && upcomingEvents.length > 0 && (
          <div className="events-month-bar" data-no-swipe-back data-no-swipe-next>
            <button
              type="button"
              className={`chip ${statusFilter === 'all' && effectiveMonth === 'all' ? 'on' : ''}`}
              aria-pressed={statusFilter === 'all' && effectiveMonth === 'all'}
              onClick={() => updateParams({ filter: null, month: null })}
            >
              {t('filterAll')}
            </button>
            <button
              type="button"
              className={`chip ${statusFilter === 'open' ? 'on' : ''}`}
              aria-pressed={statusFilter === 'open'}
              onClick={() => updateParams({ filter: statusFilter === 'open' ? null : 'open' })}
            >
              {t('filterOpen')}
            </button>
            <button
              type="button"
              className={`chip ${statusFilter === 'going' ? 'on' : ''}`}
              aria-pressed={statusFilter === 'going'}
              onClick={() => updateParams({ filter: statusFilter === 'going' ? null : 'going' })}
            >
              {t('filterGoing')}
            </button>
            <button
              type="button"
              className={`chip ${statusFilter === 'needs_transport' ? 'on' : ''}`}
              aria-pressed={statusFilter === 'needs_transport'}
              onClick={() =>
                updateParams({ filter: statusFilter === 'needs_transport' ? null : 'needs_transport' })
              }
            >
              {t('filterNeedsTransport')}
            </button>
            {canDrive && ridesNeededEventCount > 0 && (
              <button
                type="button"
                className={`chip ${statusFilter === 'rides_needed' ? 'on' : ''}`}
                aria-pressed={statusFilter === 'rides_needed'}
                onClick={() =>
                  updateParams({ filter: statusFilter === 'rides_needed' ? null : 'rides_needed' })
                }
              >
                {t('filterRidesNeeded', { count: ridesNeededEventCount })}
              </button>
            )}
            {months.length >= 2 && (
              <>
                <span className="events-filter-sep" aria-hidden="true" />
                {months.map((ym) => (
                  <button
                    key={ym}
                    type="button"
                    className={`chip chip-month ${effectiveMonth === ym ? 'on' : ''}`}
                    aria-pressed={effectiveMonth === ym}
                    onClick={() => updateParams({ month: effectiveMonth === ym ? null : ym })}
                  >
                    {monthParts(ym, dateLocale).short}
                  </button>
                ))}
              </>
            )}
          </div>
        )}

        {scope === 'past' && (
          <div className="events-past-tools">
            <div className="events-past-search-wrap">
              <Search size={15} className="events-past-search-icon" aria-hidden="true" />
              <input
                type="search"
                className="input events-past-search"
                placeholder={t('searchPastPlaceholder')}
                aria-label={t('searchPastPlaceholder')}
                value={pastQuery}
                onChange={(ev) => updateParams({ q: ev.target.value || null })}
              />
            </div>
            <button
              type="button"
              className={`chip ${pastAttendedOnly ? 'on' : ''}`}
              aria-pressed={pastAttendedOnly}
              onClick={() => updateParams({ attended: pastAttendedOnly ? null : '1' })}
            >
              {t('attendedOnly')}
            </button>
          </div>
        )}

        <ErrorBox
          error={error}
          onRetry={() =>
            scope === 'past'
              ? loadPast(null)
              : scope === 'managing'
                ? managingRes.reload()
                : upcomingRes.reload()
          }
        />

        {loading ? (
          <CardSkeleton count={3} />
        ) : scope === 'my' ? (
          myScheduleDays.length === 0 ? (
            <EmptyState
              className="events-empty-card"
              icon={emptyCalendarIcon}
              title={t('emptyScheduleTitle')}
              description={t('emptyScheduleDesc')}
              action={
                <Button variant="primary" size="sm" onClick={() => setScope('upcoming')}>
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
                  onSelect={(day) => updateParams({ day: day === 'all' ? null : day })}
                  allLabel={t('allDays')}
                  ariaLabel={t('mySchedule')}
                />
              )}
              {visibleScheduleDays.map((day) => (
                <section key={day.dayKey} className="stack-sm">
                  {effectiveDay === 'all' && (
                    <div className="events-month-head">{fmtDay(`${day.dayKey}T00:00`, ft)}</div>
                  )}
                  {day.blocks.map((block) => (
                    <div key={block.id} className="outing-block">
                      {block.event && (
                        <div className="outing-block-head">
                          <span className="small muted event-kind-inline">
                            <EventIcon icon={block.event.icon} kind={block.event.kind} size={20} />
                            <span>
                              {ft(EVENT_KIND[block.event.kind]?.label ?? block.event.kind)}
                              {block.event.groups ? ` · ${block.event.groups}` : ''}
                            </span>
                          </span>
                          <Button
                            variant="ghost"
                            size="sm"
                            icon={<Calendar size={14} />}
                            onClick={() =>
                              downloadIcs(
                                {
                                  title: block.event!.title,
                                  description: block.event!.description,
                                  location: block.event!.location,
                                  startsAt: block.event!.starts_at,
                                },
                                `${block.event!.title}.ics`,
                              )
                            }
                          >
                            {t('addToCalendar', { ns: 'eventDetail' })}
                          </Button>
                        </div>
                      )}
                      <ScheduleTimeline items={block.items} />
                    </div>
                  ))}
                </section>
              ))}
            </div>
          )
        ) : scope === 'managing' ? (
          !managingEvents.length ? (
            <EmptyState
              className="events-empty-card"
              icon={emptyCalendarIcon}
              title={t('none')}
              description={t('emptyManagingDesc')}
            />
          ) : (
            <div className="stack-sm">
              {(draftManagingCount > 0 || hideDrafts) && (
                <div className="events-month-bar" data-no-swipe-back data-no-swipe-next>
                  <button
                    type="button"
                    className={`chip ${hideDrafts ? 'on' : ''}`}
                    aria-pressed={hideDrafts}
                    onClick={() => updateParams({ hideDrafts: hideDrafts ? null : '1' })}
                  >
                    {hideDrafts ? <Eye size={14} aria-hidden="true" /> : <EyeOff size={14} aria-hidden="true" />}
                    <span>
                      {hideDrafts
                        ? draftManagingCount > 0
                          ? t('draftsHiddenCount', { count: draftManagingCount })
                          : t('showDrafts')
                        : draftManagingCount > 0
                          ? t('hideDraftsCount', { count: draftManagingCount })
                          : t('hideDrafts')}
                    </span>
                  </button>
                </div>
              )}
              {!visibleManagingEvents.length ? (
                <EmptyState
                  className="events-empty-card"
                  icon={emptyCalendarIcon}
                  title={t('draftsHiddenEmptyTitle')}
                  description={t('draftsHiddenEmptyDesc')}
                  action={
                    <Button variant="secondary" size="sm" onClick={() => updateParams({ hideDrafts: null })}>
                      {t('showDrafts')}
                    </Button>
                  }
                />
              ) : (
                visibleManagingEvents.map((e) => {
                  const canPlan = can('admin_plan') && (can('admin_dashboard') || e.can_manage);
                  const canReimb =
                    (e.created_by?.id === me?.id && can('events', 'update')) ||
                    can('admin_events') ||
                    can('admin_plan') ||
                    can('admin_rides');
                  return (
                    <EventManageCard
                      key={e.id}
                      event={e}
                      canPlan={canPlan}
                      canReimb={canReimb}
                      busy={actionBusy}
                      onOpenQr={setQrManageEvent}
                      onOpenReimb={(ev) => setReimbEventId(ev.id)}
                      onPublish={handlePublishEvent}
                      onUnpublish={handleUnpublishEvent}
                    />
                  );
                })
              )}
            </div>
          )
        ) : scope === 'past' ? (
          !pastEvents.length ? (
            <EmptyState
              className="events-empty-card"
              icon={emptyCalendarIcon}
              title={t('none')}
              description={t('emptyPastDesc')}
            />
          ) : !filteredPast.length ? (
            <EmptyState
              className="events-empty-card"
              title={t('pastEmptySearch')}
              action={
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => updateParams({ q: null, attended: null })}
                >
                  {t('showAllEvents')}
                </Button>
              }
            />
          ) : (
            <div className="stack">
              <div className="list">{filteredPast.map(renderEventRow)}</div>
              <InfiniteSentinel
                hasMore={nextPast != null}
                loading={loadingMore}
                sentinelRef={sentinelRef}
                empty={!pastEvents.length}
              />
            </div>
          )
        ) : !upcomingEvents.length ? (
          <EmptyState
            className="events-empty-card"
            icon={emptyCalendarIcon}
            title={t('none')}
            description={t('emptyUpcomingDesc')}
            action={
              <Button variant="secondary" size="sm" onClick={() => setScope('past')}>
                {t('browsePast')}
              </Button>
            }
          />
        ) : !filteredUpcoming.length ? (
          <EmptyState
            className="events-empty-card"
            title={t('filterEmptyTitle')}
            description={t('filterEmptyDesc')}
            action={
              <Button
                variant="primary"
                size="sm"
                onClick={() => updateParams({ filter: null, month: null })}
              >
                {t('showAllEvents')}
              </Button>
            }
          />
        ) : (
          <div className="stack">
            {monthGroups.map((grp) => (
              <div key={grp.monthKey} className="stack-xs">
                {monthGroups.length >= 2 && grp.monthKey !== 'other' && (
                  <div className="events-month-head">{monthParts(grp.monthKey, dateLocale).long}</div>
                )}
                <div className="list">{grp.items.map(renderEventRow)}</div>
              </div>
            ))}
          </div>
        )}
      </div>

      {showUniversalScan && (
        <UniversalCheckinScannerModal onClose={() => setShowUniversalScan(false)} />
      )}
      {scanEvent && (
        <EventScannerModal
          event={scanEvent}
          onClose={() => setScanEvent(null)}
          onSuccess={() => {
            setScanEvent(null);
            upcomingRes.reload();
          }}
        />
      )}
      {passModalEvent && passModalEvent.my_signup?.pass_code && (
        <MemberPassQrModal
          event={passModalEvent}
          passCode={passModalEvent.my_signup.pass_code}
          onClose={() => setPassModalEvent(null)}
        />
      )}
      {qrManageEvent && (
        <AttendanceQrModal
          event={qrManageEvent}
          onClose={() => setQrManageEvent(null)}
          onResetCode={() => managingRes.reload()}
        />
      )}
      {reimbEventId != null && (
        <PostEventReimbursementModal
          eventId={reimbEventId}
          onClose={() => setReimbEventId(null)}
          onSuccess={() => managingRes.reload()}
        />
      )}
      {quickRsvp && (
        <QuickRsvpModal
          event={quickRsvp.event}
          initialStep={quickRsvp.step}
          joinChoices={quickRsvp.joinChoices}
          loadingChoices={quickRsvp.loadingChoices}
          onCompleteSignup={() => upcomingRes.reload()}
          onClose={() => setQuickRsvp(null)}
        />
      )}
      <BackToTop />
    </Page>
  );
}
