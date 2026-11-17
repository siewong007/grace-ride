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
import { fmtAgo, fmtDay, fmtTime, greeting, RIDE_STATUS, RIDE_STATUS_PASSENGER, rm, todayStr, useFormatT } from '../format';
import { loadTodayRidesSnapshot, saveTodayRidesSnapshot } from '../offlineSnapshot';
import { loadPrefs, savePrefs, TextSize } from '../prefs';
import type { EventItem } from './Events';

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
  // Each block loads only when the member may open the page it belongs to.
  const rides = useApi<Ride[]>(can('rides') ? '/rides' : null, 30000);
  const events = useApi<EventItem[]>(can('events') ? '/events' : null);
  const shares = useApi<Share[]>(can('costs') ? '/me/shares' : null);
  const ann = useApi<Announcement[]>('/announcements');
  const today = todayStr();
  const driving = useApi<Ride[]>(can('drive') ? `/driver/schedule?from=${today}&to=${today}` : null);

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
      // ignore
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

  const offlineSnap = !rides.data && rides.error && me?.id ? loadTodayRidesSnapshot(me.id) : null;
  const myRides = rides.data ?? (offlineSnap ? offlineSnap.rides.filter((r) => r.viewer?.is_passenger) : []);
  const next = myRides[0];
  // Rides where a driver has offered and only the member's answer is missing.
  const awaitingReply = myRides.filter((r) => r.status === 'pending_passenger' && r.viewer?.is_passenger && r.driver);
  const toPay = (shares.data ?? []).filter((s) => TO_PAY.includes(s.status));

  // Coming up: the member's other rides and the events they are going to, in date order.
  const comingUp = [
    // Rides already listed under "Needs you" are not repeated here.
    ...myRides
      .slice(1)
      .filter((r) => !awaitingReply.includes(r))
      .map((r) => ({ kind: 'ride' as const, at: r.pickup_at, ride: r })),
    ...(events.data ?? [])
      .filter((e) => e.my_signup?.status === 'going')
      .map((e) => ({ kind: 'event' as const, at: e.starts_at, event: e })),
  ]
    .sort((a, b) => a.at.localeCompare(b.at))
    .slice(0, 3);

  const isDriverToday = can('drive') && (driving.loading || (driving.data?.length ?? 0) > 0);
  const canRequest = can('rides', 'create');
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
          {can('admin_dashboard') && (
            <div className="btn-row page-head-actions hide-mobile">
              <Link className="btn" to="/admin">{t('dashboard')}</Link>
            </div>
          )}
        </div>

        {offlineSnap && (
          <Notice tone="warning">
            <span>{t('offlineSnapshot', { when: fmtAgo(offlineSnap.savedAt, ft) })}</span>
          </Notice>
        )}

        {/* 1. Needs you — only when something is waiting on the member. */}
        {(awaitingReply.length > 0 || toPay.length > 0) && (
          <Section title={t('needsYou')}>
            <div className="list">
              {awaitingReply.map((r) => (
                <NeedRow
                  key={`r${r.id}`}
                  to={`/rides/${r.id}`}
                  title={t('replyNeeded', { name: r.driver!.name })}
                  sub={`${fmtDay(r.pickup_at, ft)} · ${fmtTime(r.pickup_at, ft)} · ${r.destination}`}
                  cta={t('replyCta')}
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

        {/* 2. Next ride — or, with nothing booked, the one blue button. */}
        {rides.loading ? (
          <CardSkeleton count={1} />
        ) : next ? (
          <NextRide ride={next} onChange={rides.reload} />
        ) : !can('rides') || isDriverToday ? null : (
          <section className="card home-empty">
            <h2>{t('noRide')}</h2>
            {canRequest && (
              <>
                <p className="muted">{t('noRideHint')}</p>
                <Button variant="primary" block to="/rides/new" icon={<CalendarPlus size={20} />}>
                  {t('signupRide')}
                </Button>
              </>
            )}
          </section>
        )}

        {!rides.loading && !next && !isDriverToday && showComfort && (
          <section className="card stack-sm home-comfort" aria-label={t('comfortTitle')}>
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
          </section>
        )}

        {/* Today's driving (drivers only). */}
        {can('drive') && (isDriverToday || !!next) && (
          <Section title={t('driveSchedule')} more={{ to: '/drive', label: t('seeAll') }}>
            {driving.loading ? (
              <ListSkeleton count={2} />
            ) : driving.data?.length ? (
              <div className="list">
                {driving.data.map((r) => (
                  <Link key={r.id} to={`/rides/${r.id}`} className="item">
                    <span className="timeline-time">{fmtTime(r.pickup_at, ft).replace(/^.. /, '')}</span>
                    <div className="grow">
                      <div className="bold">{r.passenger.name}</div>
                      <div className="muted small">{r.area.name} → {r.destination}</div>
                    </div>
                    <StatusPill map={RIDE_STATUS} status={r.status} />
                  </Link>
                ))}
              </div>
            ) : (
              <p className="muted">{t('noDriveToday')}</p>
            )}
          </Section>
        )}

        {/* 3. Coming up — the member's own plans only; other events stay on the Events tab. */}
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
              <div className="list">
                {comingUp.map((c) =>
                  c.kind === 'ride' ? (
                    <Link key={`r${c.ride.id}`} to={`/rides/${c.ride.id}`} className="item">
                      <div className="grow">
                        <div className="bold">{t('rideTo', { place: c.ride.destination })}</div>
                        <div className="muted small">
                          {fmtDay(c.at, ft)} · {fmtTime(c.at, ft)} ·{' '}
                          {ft(RIDE_STATUS_PASSENGER[c.ride.status]?.label ?? RIDE_STATUS[c.ride.status]?.label ?? c.ride.status)}
                        </div>
                      </div>
                      <ChevronRight size={16} className="muted" aria-hidden="true" />
                    </Link>
                  ) : (
                    <Link key={`e${c.event.id}`} to={`/events/${c.event.id}`} className="item">
                      <div className="grow">
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
            ) : (
              <p className="muted">{t('nothingComingUp')}</p>
            )}
          </Section>
        )}

        {/* With a ride already booked, requesting another is a quiet secondary action. */}
        {next && canRequest && (
          <Button variant="secondary" block to="/rides/new" icon={<CalendarPlus size={20} />}>
            {t('signupRide')}
          </Button>
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

  return (
    <section className={`card home-next${askReadiness ? ' home-next--today' : ''}`} aria-labelledby="home-next-label home-next-title">
      <div className="row between nowrap">
        <span id="home-next-label" className="muted small">{isToday ? t('today') : t('nextRide')}</span>
        <StatusPill map={ride.viewer?.is_passenger ? RIDE_STATUS_PASSENGER : RIDE_STATUS} status={ride.status} />
      </div>

      <h2 id="home-next-title" className="home-next-time">
        {/* Stretched over the card, so the whole card opens the ride. */}
        <Link to={`/rides/${ride.id}`} className="home-next-link">
          {isToday ? fmtTime(ride.pickup_at, ft) : `${fmtDay(ride.pickup_at, ft)} · ${fmtTime(ride.pickup_at, ft)}`}
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
          <Button variant="ghost" size="sm" className="text-danger home-cant" disabled={busy} onClick={() => setUnable(true)}>
            {proxyName ? t('rideDetail:unableToJoinBtnProxy', { name: proxyName }) : t('cantMakeIt')}
          </Button>
          <ErrorBox error={error} />
        </div>
      )}

      {unable && <CantMakeItModal rideId={ride.id} riderName={proxyName} onClose={() => setUnable(false)} onDone={onChange} />}
    </section>
  );
}
