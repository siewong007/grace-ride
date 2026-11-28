import { Link } from 'react-router-dom';
import { Accessibility, ChevronRight, Phone, Pin } from 'lucide-react';
import { useEffect, useState } from 'react';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { post, useApi } from '../api';
import { useAuth } from '../auth';
import CantMakeItModal from '../components/CantMakeIt';
import type { Ride } from '../components/RideCard';
import { Button, CardSkeleton, EmptyState, ErrorBox, ListSkeleton, Notice, Page, Pill, StatusPill, useAction } from '../components/ui';
import { fmtAgo, fmtDateTime, fmtDay, fmtTime, greeting, localDate, RIDE_STATUS, RIDE_STATUS_PASSENGER, rm, todayStr, useFormatT } from '../format';
import { loadTodayRidesSnapshot, saveTodayRidesSnapshot } from '../offlineSnapshot';
import { ensureLocaleNamespaces } from '../i18n';
import { canApprovePlan } from '../perms';
import { useStaffNav, type AdminCounts } from '../staffNav';
import type { EventItem } from './Events';
import { GettingStarted } from '../components/Onboarding';

interface Share {
  id: number;
  title: string;
  amount_cents: number;
  status: string;
  due_date: string | null;
}

interface Announcement {
  id: number;
  title: string;
  body: string;
  pinned: boolean;
  by: string;
  created_at: string;
}

/** Shares the member still has to pay. `pending` (receipt sent) and `confirmed` need nothing from them. */
const TO_PAY = ['unpaid', 'needs_info'];

/** A titled list on Home: heading, optional link on the right, then rows. No frame around it. */
function Section({ title, more, children }: { title: ReactNode; more?: { to: string; label: string }; children: ReactNode }) {
  return (
    <section className="home-section">
      <div className="section-head">
        <h2>{title}</h2>
        {more && <Link to={more.to} className="small">{more.label}</Link>}
      </div>
      {children}
    </section>
  );
}

/** One "Needs you" row: what is waiting, when, and a single verb. The whole row is the link. */
function NeedRow({ to, title, sub, cta }: { to: string; title: ReactNode; sub: ReactNode; cta: string }) {
  return (
    <Link to={to} className="item home-need">
      <span className="grow">
        <span className="bold">{title}</span>
        <span className="small muted">{sub}</span>
      </span>
      <span className="home-need-cta nowrap">
        {cta}
        <ChevronRight size={16} aria-hidden="true" />
      </span>
    </Link>
  );
}

/**
 * Home answers three questions, in order: what needs me, when is my next ride, what is
 * coming up. Sections with nothing in them are left out, and there is at most one blue
 * button on the screen.
 */
export default function Home() {
  const { t, i18n } = useTranslation('home');
  const ft = useFormatT();
  const { me, can } = useAuth();

  useEffect(() => {
    void ensureLocaleNamespaces(['rideDetail']);
  }, []);
  // Each block loads only when the member may open the page it belongs to.
  const rides = useApi<Ride[]>(can('rides') ? '/rides' : null, 30000);
  const events = useApi<EventItem[]>(can('events') ? '/events' : null);
  const shares = useApi<Share[]>(can('costs') ? '/me/shares' : null);
  const ann = useApi<Announcement[]>('/announcements');
  const today = todayStr();
  const nextWeek = new Date();
  nextWeek.setDate(nextWeek.getDate() + 7);
  const nextWeekStr = localDate(nextWeek);
  const driving = useApi<Ride[]>(can('drive') ? `/driver/schedule?from=${today}&to=${nextWeekStr}` : null);
  const staffNav = useStaffNav();
  const standaloneAdminCounts = useApi<AdminCounts>(
    !staffNav.fromContext && me && me.status !== 'pending' && can('admin_dashboard') ? '/admin/dashboard' : null,
  );
  const adminCountsData = staffNav.fromContext ? staffNav.counts : standaloneAdminCounts.data;

  useEffect(() => {
    if (!me?.id) return;
    const combined = [...(rides.data ?? []), ...(driving.data ?? [])];
    if (rides.data || driving.data) {
      saveTodayRidesSnapshot(me.id, combined);
    }
  }, [me?.id, rides.data, driving.data]);

  const offlineSnap =
    ((!rides.data && rides.error) || (!driving.data && driving.error)) && me?.id
      ? loadTodayRidesSnapshot(me.id)
      : null;
  const myRides = rides.data ?? (offlineSnap ? offlineSnap.rides.filter((r) => r.viewer?.is_passenger) : []);
  const allDriving = driving.data ?? (offlineSnap ? offlineSnap.rides.filter((r) => r.viewer?.is_driver) : []);
  const todaysDriving = allDriving.filter((r) => r.pickup_at.slice(0, 10) === today);
  const upcomingDriving = allDriving.filter((r) => r.pickup_at.slice(0, 10) > today);
  const nextRide = myRides[0] ?? null;
  const nextDrive = todaysDriving[0] ?? upcomingDriving[0] ?? null;

  const allEvents = events.data ?? [];
  const goingEvents = allEvents.filter((e) => e.my_signup?.status === 'going');
  const openEvents = allEvents.filter((e) => e.status === 'published' && e.my_signup?.status !== 'going');

  // Needs you: driver offers awaiting reply, assignments awaiting driver confirm, and unpaid shares.
  const awaitingReply = myRides.filter((r) => r.status === 'pending_passenger' && r.viewer?.is_passenger && r.driver);
  const awaitingDriveConfirm = allDriving.filter((r) => r.status === 'pending_driver_confirm');
  const toPay = (shares.data ?? []).filter((s) => TO_PAY.includes(s.status));
  const isTrafficLead = canApprovePlan(me?.roles);
  const pendingPlanDrafts = isTrafficLead ? (adminCountsData?.pending_plan_drafts ?? 0) : 0;
  const pendingRideDrafts = isTrafficLead ? (adminCountsData?.pending_ride_drafts ?? 0) : 0;
  const returnedDrafts = !isTrafficLead
    ? (adminCountsData?.my_returned_plan_drafts ?? 0) + (adminCountsData?.my_returned_ride_drafts ?? 0)
    : 0;

  // Single event-first hero commitment.
  const heroKind: 'ride' | 'drive' | 'goingEvent' | 'openEvent' | 'empty' =
    nextRide && (!nextDrive || nextRide.pickup_at <= nextDrive.pickup_at)
      ? 'ride'
      : nextDrive
        ? 'drive'
        : goingEvents.length > 0
          ? 'goingEvent'
          : openEvents.length > 0
            ? 'openEvent'
            : 'empty';

  const heroRideId = heroKind === 'ride' && nextRide ? nextRide.id : null;
  const heroRideEventId = heroKind === 'ride' && nextRide?.event?.id ? nextRide.event.id : null;
  const heroDriveId = heroKind === 'drive' && nextDrive ? nextDrive.id : null;
  const heroGoingEventId = heroKind === 'goingEvent' && goingEvents[0] ? goingEvents[0].id : null;

  const extraDrivingToday = todaysDriving.filter((r) => r.id !== heroRideId && r.id !== heroDriveId);
  const extraDrivingLater = upcomingDriving.filter((r) => r.id !== heroRideId && r.id !== heroDriveId);
  const showSecondaryDriveSection =
    heroKind === 'ride' && can('drive') && (extraDrivingToday.length > 0 || extraDrivingLater.length > 0);

  // Coming up: remaining commitments not already shown in the hero or secondary driving block.
  const comingUp = [
    ...myRides
      .filter((r) => r.id !== heroRideId && !awaitingReply.some((a) => a.id === r.id))
      .map((r) => ({ kind: 'ride' as const, at: r.pickup_at, ride: r })),
    ...(heroKind === 'drive' ? [...extraDrivingToday, ...extraDrivingLater] : [])
      .filter((r) => !awaitingDriveConfirm.some((a) => a.id === r.id))
      .map((r) => ({ kind: 'drive' as const, at: r.pickup_at, ride: r })),
    ...goingEvents
      .filter((e) => e.id !== heroGoingEventId && e.id !== heroRideEventId)
      .map((e) => ({ kind: 'event' as const, at: e.starts_at, event: e })),
  ]
    .sort((a, b) => a.at.localeCompare(b.at))
    .slice(0, 4);

  const canRequest = can('rides', 'create');
  const heroLoading = (can('rides') && rides.loading) || (can('events') && events.loading && !nextRide && !nextDrive);
  const heroError =
    !offlineSnap && !nextRide && !nextDrive && !allEvents.length
      ? rides.error ?? events.error ?? driving.error
      : null;
  // The whole date ("Friday 20 November"), not just "Today".
  const formattedToday = new Date().toLocaleDateString(i18n.language === 'zh' ? 'zh-CN' : 'en-GB', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  });
  // Pinned first, then newest; Home shows one line and folds the rest away.
  const announcements = [...(ann.data ?? [])].sort((a, b) => Number(b.pinned) - Number(a.pinned)).slice(0, 3);

  return (
    <Page width="medium">
      <div className="home">
        <div className="page-head home-head">
          <div>
            <h1>{greeting(ft)}{me?.name ? `, ${me.name}` : ''}</h1>
            <p className="muted">{formattedToday}</p>
          </div>
        </div>

        {offlineSnap && (
          <Notice tone="warning">
            <span>{t('offlineSnapshot', { when: fmtAgo(offlineSnap.savedAt, ft) })}</span>
          </Notice>
        )}

        <ErrorBox
          error={heroError}
          onRetry={() => {
            rides.reload();
            events.reload();
            driving.reload();
          }}
        />

        {/* 1. Needs you — only when something is waiting on the member. */}
        {(awaitingReply.length > 0 ||
          awaitingDriveConfirm.length > 0 ||
          toPay.length > 0 ||
          pendingPlanDrafts > 0 ||
          pendingRideDrafts > 0 ||
          returnedDrafts > 0) && (
          <Section title={t('needsYou')}>
            <div className="list">
              {pendingPlanDrafts > 0 && (
                <NeedRow
                  key="tpd"
                  to="/admin/traffic?tab=plans"
                  title={t('trafficPendingPlansTitle', { count: pendingPlanDrafts })}
                  sub={t('trafficPendingPlansSub')}
                  cta={t('trafficReviewCta')}
                />
              )}
              {pendingRideDrafts > 0 && (
                <NeedRow
                  key="trd"
                  to="/admin/traffic?tab=rides&view=drafts"
                  title={t('trafficPendingRidesTitle', { count: pendingRideDrafts })}
                  sub={t('trafficPendingRidesSub')}
                  cta={t('trafficReviewCta')}
                />
              )}
              {returnedDrafts > 0 && (
                <NeedRow
                  key="retd"
                  to="/admin/traffic?tab=plans"
                  title={t('trafficReturnedDraftsTitle', { count: returnedDrafts })}
                  sub={t('trafficReturnedDraftsSub')}
                  cta={t('trafficReviseCta')}
                />
              )}
              {awaitingReply.map((r) => (
                <NeedRow
                  key={`r${r.id}`}
                  to={`/rides/${r.id}`}
                  title={t('replyNeeded', { name: r.driver!.name })}
                  sub={`${fmtDay(r.pickup_at, ft)} · ${fmtTime(r.pickup_at, ft)} · ${r.event?.title ?? r.destination}`}
                  cta={t('replyCta')}
                />
              ))}
              {awaitingDriveConfirm.map((r) => (
                <NeedRow
                  key={`dc${r.id}`}
                  to={`/rides/${r.id}`}
                  title={t('confirmDriveNeeded', { place: r.event?.title ?? r.destination })}
                  sub={`${fmtDay(r.pickup_at, ft)} · ${fmtTime(r.pickup_at, ft)} · ${r.passenger.name}`}
                  cta={t('confirmAssignment')}
                />
              ))}
              {/* My costs is where a share is paid (bank details, receipt upload). */}
              {toPay.map((s) => (
                <NeedRow
                  key={`s${s.id}`}
                  to="/costs"
                  title={`${s.title} · ${rm(s.amount_cents)}`}
                  sub={s.due_date ? t('dueBy', { date: fmtDay(s.due_date, ft) }) : t('toPay')}
                  cta={t('payCta')}
                />
              ))}
            </div>
          </Section>
        )}

        {/* New members: first steps, until done or hidden. */}
        {!rides.loading && !events.loading && (
          <GettingStarted
            hasRide={myRides.length > 0}
            hasEvent={allEvents.some((e) => !!e.my_signup)}
          />
        )}

        {/* 2. Primary Focus Surface — adapts to what matters next. */}
        {heroLoading ? (
          <CardSkeleton count={1} />
        ) : heroKind === 'ride' && nextRide ? (
          <NextRide ride={nextRide} onChange={rides.reload} />
        ) : heroKind === 'drive' && nextDrive ? (
          <NextDriveFocus
            ride={nextDrive}
            todaysCount={todaysDriving.length}
            isToday={nextDrive.pickup_at.slice(0, 10) === today}
          />
        ) : heroKind === 'goingEvent' && goingEvents[0] ? (
          <NextGoingEventFocus
            event={goingEvents[0]}
            canRequest={canRequest}
            canDrive={can('drive', 'create')}
          />
        ) : heroKind === 'openEvent' && openEvents[0] ? (
          <NextOpenEventFocus event={openEvents[0]} />
        ) : (
          <EmptyState
            title={t('noEventsYet')}
            description={t('noEventsHint')}
            action={can('events') ? <Button variant="primary" to="/events">{t('browseEvents')}</Button> : undefined}
          />
        )}

        {/* Secondary driving commitments when the hero is already a passenger ride. */}
        {showSecondaryDriveSection && (
          <Section title={t('driveSchedule')} more={{ to: '/drive', label: t('seeAll') }}>
            {driving.loading ? (
              <ListSkeleton count={2} />
            ) : extraDrivingToday.length > 0 ? (
              <div className="list">
                {extraDrivingToday.map((r) => (
                  <Link key={r.id} to={`/rides/${r.id}`} className="item">
                    <span className="timeline-time">{fmtTime(r.pickup_at, ft).replace(/^.. /, '')}</span>
                    <div className="grow min-w-0">
                      <div className="bold">{r.passenger.name}</div>
                      <div className="muted small">{r.area.name} → {r.destination}</div>
                    </div>
                    <StatusPill map={RIDE_STATUS} status={r.status} />
                  </Link>
                ))}
              </div>
            ) : (
              <Link to={`/rides/${extraDrivingLater[0].id}`} className="item">
                <div className="grow min-w-0">
                  <div className="bold">{t('nextDriveUpcoming', { when: fmtDay(extraDrivingLater[0].pickup_at, ft) })}</div>
                  <div className="muted small">
                    {fmtTime(extraDrivingLater[0].pickup_at, ft)} · {extraDrivingLater[0].passenger.name} · {extraDrivingLater[0].area.name} → {extraDrivingLater[0].destination}
                  </div>
                </div>
                <ChevronRight size={16} className="muted" aria-hidden="true" />
              </Link>
            )}
          </Section>
        )}

        {/* 3. Coming up — remaining commitments, or quiet open events when user has none. */}
        {!heroLoading && comingUp.length > 0 && (
          <Section
            title={t('comingUp')}
            more={
              myRides.length > 1
                ? { to: '/rides', label: t('seeAll') }
                : can('events')
                  ? { to: '/events', label: t('browseEvents') }
                  : undefined
            }
          >
            <div className="list">
              {comingUp.map((c) =>
                c.kind === 'ride' ? (
                  <Link key={`r${c.ride.id}`} to={`/rides/${c.ride.id}`} className="item">
                    <div className="grow min-w-0">
                      <div className="bold">{c.ride.event?.title ?? t('rideTo', { place: c.ride.destination })}</div>
                      <div className="muted small">
                        {fmtDay(c.at, ft)} · {fmtTime(c.at, ft)} ·{' '}
                        {ft(RIDE_STATUS_PASSENGER[c.ride.status]?.label ?? RIDE_STATUS[c.ride.status]?.label ?? c.ride.status)}
                      </div>
                    </div>
                    <ChevronRight size={16} className="muted" aria-hidden="true" />
                  </Link>
                ) : c.kind === 'drive' ? (
                  <Link key={`d${c.ride.id}`} to={`/rides/${c.ride.id}`} className="item">
                    <div className="grow min-w-0">
                      <div className="bold">{t('driveFor', { name: c.ride.passenger.name })}</div>
                      <div className="muted small">
                        {fmtDay(c.at, ft)} · {fmtTime(c.at, ft)} · {c.ride.event?.title ?? c.ride.destination}
                      </div>
                    </div>
                    <ChevronRight size={16} className="muted" aria-hidden="true" />
                  </Link>
                ) : (
                  <Link key={`e${c.event.id}`} to={`/events/${c.event.id}`} className="item">
                    <div className="grow min-w-0">
                      <div className="bold">{c.event.title}</div>
                      <div className="muted small">
                        {fmtDay(c.at, ft)} · {fmtTime(c.at, ft)} · {t('going')}
                      </div>
                    </div>
                    <ChevronRight size={16} className="muted" aria-hidden="true" />
                  </Link>
                ),
              )}
            </div>
          </Section>
        )}

        {!heroLoading && comingUp.length === 0 && heroKind === 'openEvent' && openEvents.length > 1 && can('events') && (
          <Section title={t('otherEvents')} more={{ to: '/events', label: t('seeAll') }}>
            <div className="list">
              {openEvents.slice(1, 4).map((ev) => (
                <Link key={ev.id} to={`/events/${ev.id}`} className="item">
                  <div className="grow min-w-0">
                    <div className="bold">{ev.title}</div>
                    <div className="muted small">{fmtDateTime(ev.starts_at, ft)} · {ev.location}</div>
                  </div>
                  <ChevronRight size={16} className="muted" aria-hidden="true" />
                </Link>
              ))}
            </div>
          </Section>
        )}

        {/* Announcements: one line, the rest folded away. */}
        {announcements.length > 0 && (
          <details className="home-announce">
            <summary>
              {announcements[0].pinned && <Pin size={14} aria-hidden="true" />}
              <span className="grow home-announce-title">{announcements[0].title}</span>
              {announcements.length > 1 && <span className="small muted nowrap">{t('moreAnnouncements', { n: announcements.length - 1 })}</span>}
            </summary>
            <div className="stack">
              {announcements.map((a) => (
                <article key={a.id} className="stack-sm">
                  <div className="row between nowrap">
                    <strong>{a.title}</strong>
                    <span className="muted small nowrap">{fmtDay(a.created_at, ft)}</span>
                  </div>
                  <p className="muted">{a.body}</p>
                </article>
              ))}
            </div>
          </details>
        )}
      </div>
    </Page>
  );
}

/** The member's next ride. Tap anywhere on it to open the ride; on pickup day it also asks for readiness. */
function NextRide({ ride, onChange }: { ride: Ride; onChange: () => void }) {
  const { t } = useTranslation(['home', 'rideDetail']);
  const ft = useFormatT();
  const { can } = useAuth();
  const { busy, error, run } = useAction();
  const [unable, setUnable] = useState(false);
  const d = ride.driver;
  const proxyName = ride.is_proxy && ride.proxy ? ride.proxy.rider_name : undefined;
  const isToday = ride.pickup_at.slice(0, 10) === todayStr();
  // Same rule as the ride page: on the day, before pickup, for the passenger.
  const askReadiness =
    !!ride.viewer?.is_passenger &&
    can('rides', 'update') &&
    ride.pickup_at.slice(0, 10) <= todayStr() &&
    (ride.status === 'confirmed' || (ride.status === 'in_progress' && !ride.picked_up_at));

  const say = (status: 'ready' | 'delayed_5m', done: string) =>
    run(() => post(`/rides/${ride.id}/readiness`, { status }).then(onChange), done);

  const eventTitle = ride.event?.title;
  const whenText = isToday ? fmtTime(ride.pickup_at, ft) : `${fmtDay(ride.pickup_at, ft)} · ${fmtTime(ride.pickup_at, ft)}`;

  return (
    <section className={`focus-surface home-next${askReadiness ? ' focus-surface--today home-next--today' : ''}`} aria-labelledby="home-next-label home-next-title">
      <div className="row between nowrap">
        <span id="home-next-label" className="home-focus-eyebrow">
          {isToday ? t('today') : t('nextRide')}
          {eventTitle ? ` · ${whenText}` : ''}
        </span>
        <StatusPill map={ride.viewer?.is_passenger ? RIDE_STATUS_PASSENGER : RIDE_STATUS} status={ride.status} />
      </div>

      <h2 id="home-next-title" className="home-next-time">
        {/* Stretched over the focus surface, so tapping anywhere opens the ride. */}
        <Link to={`/rides/${ride.id}`} className="home-next-link">
          {eventTitle ?? whenText}
        </Link>
      </h2>
      <p className="home-next-route">
        <span className="sr-only">{t('pickupArea')}: </span>{ride.area.name}
        <span aria-hidden="true"> → </span>
        <span className="sr-only">, {t('destination')}: </span>{ride.destination}
      </p>

      {d && (
        <div className="home-next-driver row nowrap">
          <div className="grow">
            <div className="bold">{d.name} <span className="muted">· {t('yourDriver')}</span></div>
            {d.vehicle && (
              <div className="small muted row nowrap">
                <span>{d.vehicle.color} {d.vehicle.model}</span>
                <span className="plate">{d.vehicle.plate}</span>
              </div>
            )}
          </div>
          {d.phone && (
            <a className="icon-btn home-next-above" href={`tel:${d.phone}`} aria-label={t('callDriver')}>
              <Phone size={20} />
            </a>
          )}
        </div>
      )}

      {askReadiness && (
        <div className="stack-sm home-next-above">
          {ride.readiness_status === 'ready' ? (
            <Notice tone="success">{t('rideDetail:readyConfirmed')}</Notice>
          ) : ride.readiness_status === 'delayed_5m' || ride.readiness_status === 'delayed_10m' ? (
            <Notice tone="warning">{t('rideDetail:delayedConfirmed')}</Notice>
          ) : (
            <p className="small muted">{t('letDriverKnow')}</p>
          )}
          <div className="home-ready-row">
            <Button
              variant={ride.readiness_status === 'ready' ? 'secondary' : 'primary'}
              disabled={busy}
              onClick={() => say('ready', t('rideDetail:readyConfirmed'))}
            >
              {proxyName ? t('rideDetail:readyBtnProxy', { name: proxyName }) : t('imReady')}
            </Button>
            <Button variant="secondary" disabled={busy} onClick={() => say('delayed_5m', t('rideDetail:delayedConfirmed'))}>
              {proxyName ? t('rideDetail:delayed5mBtnProxy', { name: proxyName }) : t('runningLate')}
            </Button>
          </div>
          <Button variant="ghost" size="sm" className="quiet-danger home-cant" disabled={busy} onClick={() => setUnable(true)}>
            {proxyName ? t('rideDetail:unableToJoinBtnProxy', { name: proxyName }) : t('cantMakeIt')}
          </Button>
          <ErrorBox error={error} />
        </div>
      )}

      {unable && <CantMakeItModal rideId={ride.id} riderName={proxyName} onClose={() => setUnable(false)} onDone={onChange} />}
    </section>
  );
}

/** Volunteer driver focus surface when driving is their next commitment. */
function NextDriveFocus({ ride, todaysCount, isToday }: { ride: Ride; todaysCount: number; isToday: boolean }) {
  const { t } = useTranslation('home');
  const ft = useFormatT();
  const riderName = ride.is_proxy && ride.proxy ? ride.proxy.rider_name : ride.passenger.name;
  const multipleToday = isToday && todaysCount > 1;

  return (
    <section
      className={`focus-surface${isToday ? ' focus-surface--today' : ''}`}
      aria-labelledby="home-drive-label home-drive-title"
    >
      <div className="row between nowrap">
        <span id="home-drive-label" className="home-focus-eyebrow">
          {isToday ? t('today') : fmtDay(ride.pickup_at, ft)} · {t('drivingEyebrow')}
          {multipleToday ? ` · ${t('pickupsToday', { count: todaysCount })}` : ''}
        </span>
        <StatusPill map={RIDE_STATUS} status={ride.status} />
      </div>

      <h2 id="home-drive-title" className="home-next-time">
        <Link to={`/rides/${ride.id}`} className="home-next-link">
          {ride.event?.title ?? ride.destination}
        </Link>
      </h2>

      <p className="home-focus-meta">
        {t('pickupSummary', {
          time: fmtTime(ride.pickup_at, ft),
          name: riderName,
          area: ride.area.name,
        })}
      </p>

      {ride.assistance && (
        <div className="small muted row nowrap gap-xs">
          <Accessibility size={14} aria-hidden="true" />
          <span>{ride.assistance}</span>
        </div>
      )}

      <div className="home-next-above">
        <Button variant="primary" block to={multipleToday ? '/drive' : `/rides/${ride.id}`}>
          {multipleToday ? t('openItinerary') : t('openPickupDetails')}
        </Button>
      </div>
    </section>
  );
}

/** Signed-up event focus surface when transportation is not arranged yet. */
function NextGoingEventFocus({
  event,
  canRequest,
  canDrive,
}: {
  event: EventItem;
  canRequest: boolean;
  canDrive: boolean;
}) {
  const { t } = useTranslation('home');
  const ft = useFormatT();
  const selfTransport = Boolean(event.my_signup?.self_transport);

  return (
    <section className="focus-surface" aria-labelledby="home-event-label home-event-title">
      <div className="row between nowrap">
        <span id="home-event-label" className="home-focus-eyebrow">{t('upcomingEvent')}</span>
        <Pill tone="success">{t('going')}</Pill>
      </div>

      <h2 id="home-event-title" className="home-next-time">
        <Link to={`/events/${event.id}`} className="home-next-link">
          {event.title}
        </Link>
      </h2>

      <p className="home-focus-meta">
        {fmtDateTime(event.starts_at, ft)} · {event.location}
      </p>

      <p className="home-focus-status">{selfTransport ? t('transportSelf') : t('transportNotArranged')}</p>

      <div className="btn-row home-next-above">
        {selfTransport ? (
          <Button variant="primary" to={`/events/${event.id}`}>
            {t('viewEvent')}
          </Button>
        ) : (
          <>
            {canRequest ? (
              <Button variant="primary" to={`/rides/new?event=${event.id}`}>
                {t('needRideBtn')}
              </Button>
            ) : (
              <Button variant="primary" to={`/events/${event.id}`}>
                {t('viewEvent')}
              </Button>
            )}
            {canDrive && (
              <Button variant="secondary" to={`/events/${event.id}`}>
                {t('offerDriveBtn')}
              </Button>
            )}
          </>
        )}
      </div>
    </section>
  );
}

/** Quiet invitation to the next open event when the member has no commitments yet. */
function NextOpenEventFocus({ event }: { event: EventItem }) {
  const { t } = useTranslation('home');
  const ft = useFormatT();

  return (
    <section className="focus-surface" aria-labelledby="home-open-event-label home-open-event-title">
      <span id="home-open-event-label" className="home-focus-eyebrow">{t('nextEvent')}</span>

      <h2 id="home-open-event-title" className="home-next-time">
        <Link to={`/events/${event.id}`} className="home-next-link">
          {event.title}
        </Link>
      </h2>

      <p className="home-focus-meta">
        {fmtDateTime(event.starts_at, ft)} · {event.location}
      </p>

      <div className="btn-row home-next-above">
        <Button variant="primary" to={`/events/${event.id}`}>
          {t('viewEvent')}
        </Button>
        <Button variant="ghost" to="/events">
          {t('browseEvents')}
        </Button>
      </div>
    </section>
  );
}
