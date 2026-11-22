import { Link } from 'react-router-dom';
import { CalendarPlus, ChevronRight, Phone, Pin } from 'lucide-react';
import { useEffect, useState } from 'react';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { post, useApi } from '../api';
import { useAuth } from '../auth';
import CantMakeItModal from '../components/CantMakeIt';
import type { Ride } from '../components/RideCard';
import { Button, CardSkeleton, ErrorBox, ListSkeleton, Notice, Page, StatusPill, useAction } from '../components/ui';
import { fmtAgo, fmtDay, fmtTime, greeting, localDate, RIDE_STATUS, RIDE_STATUS_PASSENGER, rm, todayStr, useFormatT } from '../format';
import { loadTodayRidesSnapshot, saveTodayRidesSnapshot } from '../offlineSnapshot';
import { loadPrefs, savePrefs, TextSize } from '../prefs';
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

/** Shares the member still has to pay. */
const TO_PAY = ['unpaid', 'needs_info'];

/**
 * A titled section on Home — heading + optional "see all" link, then content.
 * Uses uppercase label style (not a heavy card container).
 */
function Section({
  title,
  more,
  children,
}: {
  title: ReactNode;
  more?: { to: string; label: string };
  children: ReactNode;
}) {
  return (
    <section>
      <div className="home-section-head">
        <h2>{title}</h2>
        {more && (
          <Link to={more.to} className="small muted">
            {more.label}
          </Link>
        )}
      </div>
      {children}
    </section>
  );
}

/**
 * Home answers three questions in order:
 *   1. What needs me right now?
 *   2. What is my next ride?
 *   3. What else is coming up?
 *
 * Sections with nothing in them are omitted. Only one primary action is shown at a time.
 */
export default function Home() {
  const { t, i18n } = useTranslation('home');
  const ft = useFormatT();
  const { me, can } = useAuth();

  const rides = useApi<Ride[]>(can('rides') ? '/rides' : null, 30000);
  const events = useApi<EventItem[]>(can('events') ? '/events' : null);
  const shares = useApi<Share[]>(can('costs') ? '/me/shares' : null);
  const ann = useApi<Announcement[]>('/announcements');

  const today = todayStr();
  const nextWeek = new Date();
  nextWeek.setDate(nextWeek.getDate() + 7);
  const nextWeekStr = localDate(nextWeek);
  const driving = useApi<Ride[]>(
    can('drive') ? `/driver/schedule?from=${today}&to=${nextWeekStr}` : null,
  );

  const [showComfort, setShowComfort] = useState(() => {
    try {
      return !localStorage.getItem('gr-prefs') && !localStorage.getItem('gr-comfort-dismissed');
    } catch {
      return false;
    }
  });

  const chooseComfort = (size: TextSize) => {
    const p = loadPrefs();
    savePrefs({ ...p, textSize: size });
    try {
      localStorage.setItem('gr-comfort-dismissed', '1');
    } catch {
      /* ignore */
    }
    setShowComfort(false);
  };

  useEffect(() => {
    if (!me?.id) return;
    const combined = [...(rides.data ?? []), ...(driving.data ?? [])];
    if (rides.data || driving.data) {
      saveTodayRidesSnapshot(me.id, combined);
    }
  }, [me?.id, rides.data, driving.data]);

  const offlineSnap =
    !rides.data && rides.error && me?.id ? loadTodayRidesSnapshot(me.id) : null;

  const myRides = rides.data ?? (offlineSnap ? offlineSnap.rides.filter((r) => r.viewer?.is_passenger) : []);
  const allDriving =
    driving.data ?? (offlineSnap ? offlineSnap.rides.filter((r) => r.viewer?.is_driver) : []);
  const todaysDriving = allDriving.filter((r) => r.pickup_at.slice(0, 10) === today);
  const upcomingDriving = allDriving.filter((r) => r.pickup_at.slice(0, 10) > today);
  const next = myRides[0];

  const awaitingReply = myRides.filter(
    (r) => r.status === 'pending_passenger' && r.viewer?.is_passenger && r.driver,
  );
  const toPay = (shares.data ?? []).filter((s) => TO_PAY.includes(s.status));

  // Coming up: member's other rides, upcoming driving, events they are going to.
  const comingUp = [
    ...myRides
      .slice(1)
      .filter((r) => !awaitingReply.includes(r))
      .map((r) => ({ kind: 'ride' as const, at: r.pickup_at, ride: r })),
    ...upcomingDriving.map((r) => ({ kind: 'drive' as const, at: r.pickup_at, ride: r })),
    ...(events.data ?? [])
      .filter((e) => e.my_signup?.status === 'going')
      .map((e) => ({ kind: 'event' as const, at: e.starts_at, event: e })),
  ]
    .sort((a, b) => a.at.localeCompare(b.at))
    .slice(0, 4);

  const isDriverToday = can('drive') && (driving.loading || todaysDriving.length > 0);
  const canRequest = can('rides', 'create');

  const formattedToday = new Date().toLocaleDateString(
    i18n.language === 'zh' ? 'zh-CN' : 'en-GB',
    { weekday: 'long', day: 'numeric', month: 'long' },
  );

  const announcements = [...(ann.data ?? [])]
    .sort((a, b) => Number(b.pinned) - Number(a.pinned))
    .slice(0, 3);

  const hasNeeds = awaitingReply.length > 0 || toPay.length > 0;

  return (
    <Page width="medium">
      <div className="home">
        {/* Greeting */}
        <div className="home-greeting">
          <div className="row between nowrap">
            <div>
              <h1>
                {greeting(ft)}
                {me?.name ? `, ${me.name}` : ''}
              </h1>
              <p className="home-date muted">{formattedToday}</p>
            </div>
            {can('admin_dashboard') && (
              <Link className="btn sm hide-mobile" to="/admin">
                {t('dashboard')}
              </Link>
            )}
          </div>
        </div>

        {offlineSnap && (
          <Notice tone="warning">
            <span>{t('offlineSnapshot', { when: fmtAgo(offlineSnap.savedAt, ft) })}</span>
          </Notice>
        )}

        {/* 1. Needs you — only when something awaits the member */}
        {hasNeeds && (
          <Section title={t('needsYou')}>
            <div>
              {awaitingReply.map((r) => (
                <Link
                  key={`r${r.id}`}
                  to={`/rides/${r.id}`}
                  className="home-need-row"
                >
                  <div className="home-need-row-body">
                    <span className="home-need-row-title">
                      {t('replyNeeded', { name: r.driver!.name })}
                    </span>
                    <span className="home-need-row-sub">
                      {fmtDay(r.pickup_at, ft)} · {fmtTime(r.pickup_at, ft)} · {r.destination}
                    </span>
                  </div>
                  <span className="home-need-row-cta">
                    {t('replyCta')}
                    <ChevronRight size={14} aria-hidden="true" />
                  </span>
                </Link>
              ))}
              {toPay.map((s) => (
                <Link key={`s${s.id}`} to="/costs" className="home-need-row">
                  <div className="home-need-row-body">
                    <span className="home-need-row-title">
                      {s.title} · {rm(s.amount_cents)}
                    </span>
                    <span className="home-need-row-sub">
                      {s.due_date ? t('dueBy', { date: fmtDay(s.due_date, ft) }) : t('toPay')}
                    </span>
                  </div>
                  <span className="home-need-row-cta">
                    {t('payCta')}
                    <ChevronRight size={14} aria-hidden="true" />
                  </span>
                </Link>
              ))}
            </div>
          </Section>
        )}

        {/* Onboarding steps for new members */}
        {!rides.loading && !events.loading && (
          <GettingStarted
            hasRide={myRides.length > 0}
            hasEvent={(events.data ?? []).some((e) => !!e.my_signup)}
          />
        )}

        {/* 2. Next ride — or the single primary action when nothing is booked */}
        {rides.loading ? (
          <CardSkeleton count={1} />
        ) : next ? (
          <NextRide ride={next} onChange={rides.reload} />
        ) : !can('rides') || isDriverToday ? null : (
          <div className="home-empty-state">
            <h2>{t('noRide')}</h2>
            {canRequest && (
              <>
                <p className="muted">{t('noRideHint')}</p>
                <Button
                  variant="primary"
                  to="/events"
                  icon={<CalendarPlus size={18} />}
                >
                  {t('signupRide')}
                </Button>
              </>
            )}
          </div>
        )}

        {/* Text-size comfort chooser (new members only, once) */}
        {!rides.loading && !next && !isDriverToday && showComfort && (
          <div className="home-comfort-card">
            <div>
              <div className="bold">{t('comfortTitle')}</div>
              <p className="muted small">{t('comfortBody')}</p>
            </div>
            <div className="btn-row">
              <Button variant="secondary" size="sm" onClick={() => chooseComfort('large')}>
                {t('comfortLarge')}
              </Button>
              <Button variant="secondary" size="sm" onClick={() => chooseComfort('xlarge')}>
                {t('comfortXlarge')}
              </Button>
              <Button variant="ghost" size="sm" onClick={() => chooseComfort('normal')}>
                {t('comfortKeep')}
              </Button>
            </div>
          </div>
        )}

        {/* 3. Driving today (volunteer drivers) */}
        {can('drive') && (
          <Section
            title={t('driveSchedule')}
            more={{ to: '/drive', label: t('seeAll') }}
          >
            {driving.loading ? (
              <ListSkeleton count={2} />
            ) : todaysDriving.length ? (
              <div className="home-coming-list">
                {todaysDriving.map((r) => (
                  <Link key={r.id} to={`/rides/${r.id}`} className="home-coming-row">
                    <div className="home-coming-row-body">
                      <span className="home-coming-row-title">{r.passenger.name}</span>
                      <span className="home-coming-row-meta">
                        {fmtTime(r.pickup_at, ft)} · {r.area.name} → {r.destination}
                      </span>
                    </div>
                    <StatusPill map={RIDE_STATUS} status={r.status} />
                  </Link>
                ))}
              </div>
            ) : upcomingDriving.length ? (
              <div className="home-coming-list">
                <p className="home-drive-empty">{t('noDriveToday')}</p>
                <Link to={`/rides/${upcomingDriving[0].id}`} className="home-coming-row">
                  <div className="home-coming-row-body">
                    <span className="home-coming-row-title">
                      {t('nextDriveUpcoming', { when: fmtDay(upcomingDriving[0].pickup_at, ft) })}
                    </span>
                    <span className="home-coming-row-meta">
                      {fmtTime(upcomingDriving[0].pickup_at, ft)} · {upcomingDriving[0].passenger.name} ·{' '}
                      {upcomingDriving[0].destination}
                    </span>
                  </div>
                  <ChevronRight size={16} className="home-coming-row-side" aria-hidden="true" />
                </Link>
              </div>
            ) : (
              <p className="home-drive-empty">{t('noDriveToday')}</p>
            )}
          </Section>
        )}

        {/* 4. Coming up */}
        {(can('rides') || can('events')) && !rides.loading && (
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
            {events.loading ? (
              <ListSkeleton count={2} />
            ) : comingUp.length ? (
              <div className="home-coming-list">
                {comingUp.map((c) =>
                  c.kind === 'ride' ? (
                    <Link key={`r${c.ride.id}`} to={`/rides/${c.ride.id}`} className="home-coming-row">
                      <div className="home-coming-row-body">
                        <span className="home-coming-row-title">
                          {t('rideTo', { place: c.ride.destination })}
                        </span>
                        <span className="home-coming-row-meta">
                          {fmtDay(c.at, ft)} · {fmtTime(c.at, ft)} ·{' '}
                          {ft(
                            RIDE_STATUS_PASSENGER[c.ride.status]?.label ??
                              RIDE_STATUS[c.ride.status]?.label ??
                              c.ride.status,
                          )}
                        </span>
                      </div>
                      <ChevronRight size={16} className="home-coming-row-side" aria-hidden="true" />
                    </Link>
                  ) : c.kind === 'drive' ? (
                    <Link key={`d${c.ride.id}`} to={`/rides/${c.ride.id}`} className="home-coming-row">
                      <div className="home-coming-row-body">
                        <span className="home-coming-row-title">
                          {t('driveFor', { name: c.ride.passenger.name })}
                        </span>
                        <span className="home-coming-row-meta">
                          {fmtDay(c.at, ft)} · {fmtTime(c.at, ft)} · {c.ride.destination}
                        </span>
                      </div>
                      <ChevronRight size={16} className="home-coming-row-side" aria-hidden="true" />
                    </Link>
                  ) : (
                    <Link key={`e${c.event.id}`} to={`/events/${c.event.id}`} className="home-coming-row">
                      <div className="home-coming-row-body">
                        <span className="home-coming-row-title">{c.event.title}</span>
                        <span className="home-coming-row-meta">
                          {fmtDay(c.at, ft)} · {fmtTime(c.at, ft)} · {t('going')}
                        </span>
                      </div>
                      <ChevronRight size={16} className="home-coming-row-side" aria-hidden="true" />
                    </Link>
                  ),
                )}
              </div>
            ) : (
              <p className="home-drive-empty">{t('nothingComingUp')}</p>
            )}
          </Section>
        )}

        {/* Quiet secondary action once a ride is booked */}
        {next && canRequest && (
          <div className="home-secondary-action">
            <Button variant="ghost" size="sm" to="/events" icon={<CalendarPlus size={16} />}>
              {t('signupRide')}
            </Button>
          </div>
        )}

        {/* Announcements: one folded line */}
        {announcements.length > 0 && (
          <details className="home-ann">
            <summary>
              {announcements[0].pinned && (
                <Pin size={12} aria-hidden="true" className="muted" />
              )}
              <span className="home-ann-title">{announcements[0].title}</span>
              {announcements.length > 1 && (
                <span className="small muted">
                  {t('moreAnnouncements', { n: announcements.length - 1 })}
                </span>
              )}
            </summary>
            <div className="home-ann-body">
              {announcements.map((a) => (
                <article key={a.id} className="home-ann-article">
                  <div className="home-ann-article-head">
                    <strong>{a.title}</strong>
                    <span className="muted small nowrap">{fmtDay(a.created_at, ft)}</span>
                  </div>
                  <p className="muted small">{a.body}</p>
                </article>
              ))}
            </div>
          </details>
        )}
      </div>
    </Page>
  );
}

/**
 * The member's next ride. The card is the primary focus of Home on ride day.
 * On pickup day it also shows readiness controls.
 */
function NextRide({ ride, onChange }: { ride: Ride; onChange: () => void }) {
  const { t } = useTranslation(['home', 'rideDetail']);
  const ft = useFormatT();
  const { can } = useAuth();
  const { busy, error, run } = useAction();
  const [unable, setUnable] = useState(false);
  const d = ride.driver;
  const proxyName = ride.is_proxy && ride.proxy ? ride.proxy.rider_name : undefined;
  const isToday = ride.pickup_at.slice(0, 10) === todayStr();

  const askReadiness =
    !!ride.viewer?.is_passenger &&
    can('rides', 'update') &&
    ride.pickup_at.slice(0, 10) <= todayStr() &&
    (ride.status === 'confirmed' ||
      (ride.status === 'in_progress' && !ride.picked_up_at));

  const say = (status: 'ready' | 'delayed_5m', done: string) =>
    run(() => post(`/rides/${ride.id}/readiness`, { status }).then(onChange), done);

  return (
    <section
      className={`home-next-hero${isToday ? ' is-today' : ''}`}
      aria-labelledby="home-next-label home-next-time"
    >
      <div className="row between nowrap">
        <span
          id="home-next-label"
          className="home-next-hero-label"
        >
          {isToday ? t('today') : t('nextRide')}
        </span>
        <StatusPill
          map={ride.viewer?.is_passenger ? RIDE_STATUS_PASSENGER : RIDE_STATUS}
          status={ride.status}
        />
      </div>

      <h2 id="home-next-time" className="home-next-hero-time">
        <Link to={`/rides/${ride.id}`} className="home-next-hero-time-link" aria-label={t('nextRide')}>
          {isToday
            ? fmtTime(ride.pickup_at, ft)
            : `${fmtDay(ride.pickup_at, ft)} · ${fmtTime(ride.pickup_at, ft)}`}
        </Link>
      </h2>

      <p className="home-next-hero-route">
        <span className="sr-only">{t('pickupArea')}: </span>
        {ride.area.name}
        <span className="home-next-hero-route-sep" aria-hidden="true"> → </span>
        <span className="sr-only">, {t('destination')}: </span>
        {ride.destination}
      </p>

      {d && (
        <div className="home-next-hero-driver">
          <div className="home-next-hero-driver-info">
            <div className="home-next-hero-driver-name">
              {d.name}
              <span className="muted"> · {t('yourDriver')}</span>
            </div>
            {d.vehicle && (
              <div className="home-next-hero-driver-vehicle">
                <span>
                  {d.vehicle.color} {d.vehicle.model}
                </span>
                <span className="plate">{d.vehicle.plate}</span>
              </div>
            )}
          </div>
          {d.phone && (
            <a
              className="icon-btn home-next-hero-above"
              href={`tel:${d.phone}`}
              aria-label={t('callDriver')}
              style={{ position: 'relative', zIndex: 1 }}
            >
              <Phone size={18} />
            </a>
          )}
        </div>
      )}

      {askReadiness && (
        <div className="home-next-hero-actions home-next-hero-above">
          {ride.readiness_status === 'ready' ? (
            <Notice tone="success">{t('rideDetail:readyConfirmed')}</Notice>
          ) : ride.readiness_status === 'delayed_5m' ||
            ride.readiness_status === 'delayed_10m' ? (
            <Notice tone="warning">{t('rideDetail:delayedConfirmed')}</Notice>
          ) : (
            <p className="small muted">{t('letDriverKnow')}</p>
          )}
          <div className="home-ready-actions">
            <Button
              variant={ride.readiness_status === 'ready' ? 'secondary' : 'primary'}
              disabled={busy}
              onClick={() => say('ready', t('rideDetail:readyConfirmed'))}
            >
              {proxyName
                ? t('rideDetail:readyBtnProxy', { name: proxyName })
                : t('imReady')}
            </Button>
            <Button
              variant="secondary"
              disabled={busy}
              onClick={() => say('delayed_5m', t('rideDetail:delayedConfirmed'))}
            >
              {proxyName
                ? t('rideDetail:delayed5mBtnProxy', { name: proxyName })
                : t('runningLate')}
            </Button>
          </div>
          <Button
            variant="ghost"
            size="sm"
            className="text-danger home-cant"
            disabled={busy}
            onClick={() => setUnable(true)}
            style={{ alignSelf: 'flex-start' }}
          >
            {proxyName
              ? t('rideDetail:unableToJoinBtnProxy', { name: proxyName })
              : t('cantMakeIt')}
          </Button>
          <ErrorBox error={error} />
        </div>
      )}

      {unable && (
        <CantMakeItModal
          rideId={ride.id}
          riderName={proxyName}
          onClose={() => setUnable(false)}
          onDone={onChange}
        />
      )}
    </section>
  );
}
