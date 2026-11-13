import { Link } from 'react-router-dom';
import {
  Calendar,
  CalendarPlus,
  CarFront,
  CreditCard,
  LifeBuoy,
  Megaphone,
  Phone,
  Pin,
} from 'lucide-react';
import { useApi } from '../api';
import { useAuth } from '../auth';
import RideCard, { Ride } from '../components/RideCard';
import {
  Avatar,
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ListSkeleton,
  Page,
  Pill,
  StatusPill,
} from '../components/ui';
import {
  EVENT_KIND,
  fmtDay,
  fmtTime,
  greeting,
  RIDE_STATUS,
  rm,
  SHARE_STATUS,
  todayStr,
  useFormatT,
} from '../format';
import type { EventItem } from './Events';
import { useTranslation } from 'react-i18next';

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

export default function Home() {
  const { t } = useTranslation(['home', 'events']);
  const ft = useFormatT();
  const { me, has, isStaff } = useAuth();
  const rides = useApi<Ride[]>('/rides', 30000);
  const events = useApi<EventItem[]>('/events');
  const shares = useApi<Share[]>('/me/shares');
  const ann = useApi<Announcement[]>('/announcements');
  const today = todayStr();
  const driving = useApi<Ride[]>(has('driver') ? `/driver/schedule?from=${today}&to=${today}` : null);

  const next = rides.data?.[0];
  const unpaid = (shares.data ?? []).filter((s) => s.status !== 'confirmed');
  const formattedToday = fmtDay(`${todayStr()}T00:00`, ft);

  return (
    <Page width="medium">
      <div className="stack">
        {/* Header Greeting with Date */}
        <div className="page-head">
          <div>
            <h1>{greeting(ft)}{me?.name ? `, ${me.name}` : ''}</h1>
            <p className="muted row nowrap">
              <span>{formattedToday}</span>
              <span>·</span>
              <Pill tone="green">{t('verified')}</Pill>
              {me?.groups.map((g) => <Pill key={g.id}>{g.name}</Pill>)}
            </p>
          </div>
          <div className="btn-row hide-mobile">
            <Button variant="primary" to="/rides/new" icon={<CalendarPlus size={20} />}>
              {t('signupRide')}
            </Button>
            {isStaff && <Link className="btn" to="/admin">{t('dashboard')}</Link>}
          </div>
        </div>

        {/* Next Ride Hero Card */}
        {rides.loading ? (
          <CardSkeleton count={1} />
        ) : next ? (
          <NextRide ride={next} />
        ) : (
          <Card className="stack">
            <EmptyState
              icon={<CalendarPlus size={36} />}
              title={t('noRide')}
              action={
                <Button variant="primary" to="/rides/new" icon={<CalendarPlus size={18} />}>
                  {t('signupRide')}
                </Button>
              }
            />
          </Card>
        )}

        {/* Quick Action Tiles */}
        <div className="home-quick-actions" role="region" aria-label="Quick actions">
          <Link to="/rides/new" className="card link home-action-tile">
            <div className="home-action-icon primary" aria-hidden="true">
              <CalendarPlus size={22} />
            </div>
            <span className="home-action-label">{t('signupRide')}</span>
          </Link>
          <Link to="/events" className="card link home-action-tile">
            <div className="home-action-icon secondary" aria-hidden="true">
              <Calendar size={22} />
            </div>
            <span className="home-action-label">{t('quickEvents')}</span>
          </Link>
          <Link to="/help" className="card link home-action-tile">
            <div className="home-action-icon neutral" aria-hidden="true">
              <LifeBuoy size={22} />
            </div>
            <span className="home-action-label">{t('helpCentre')}</span>
          </Link>
        </div>

        {/* Section 1: Today's driving (drivers only) */}
        {has('driver') && (
          <Card className="stack">
            <div className="card-title">
              <h3 className="row nowrap">
                <CarFront size={18} aria-hidden="true" />
                <span>{t('driveSchedule')}</span>
              </h3>
              <Link to="/drive" className="small">{t('all')}</Link>
            </div>
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
              <EmptyState title={t('noDriveToday')} />
            )}
          </Card>
        )}

        {/* Section 2: Upcoming rides */}
        <Card className="stack">
          <div className="card-title">
            <h3>{t('otherRides')}</h3>
            <Link to="/rides" className="small">{t('all')}</Link>
          </div>
          {rides.loading ? (
            <ListSkeleton count={2} />
          ) : rides.data && rides.data.length > (next ? 1 : 0) ? (
            <div className="grid cols-2">
              {rides.data.slice(next ? 1 : 0, 4).map((r) => (
                <RideCard key={r.id} ride={r} />
              ))}
            </div>
          ) : (
            <EmptyState
              title={t('noUpcomingRides')}
              action={
                <Button to="/rides/new" variant="secondary" size="sm">
                  {t('signupRide')}
                </Button>
              }
            />
          )}
        </Card>

        {/* Section 3: Unpaid shares */}
        <Card className="stack">
          <div className="card-title">
            <h3 className="row nowrap">
              <CreditCard size={18} aria-hidden="true" />
              <span>{t('unpaid')}</span>
              {unpaid.length > 0 && <Pill tone="warning">{unpaid.length}</Pill>}
            </h3>
            <Link to="/settlements" className="small">{t('all')}</Link>
          </div>
          {shares.loading ? (
            <ListSkeleton count={2} />
          ) : unpaid.length ? (
            <div className="list">
              {unpaid.map((s) => (
                <Link key={s.id} to={`/settlements/${s.id}`} className="item">
                  <div className="grow">
                    <div className="bold">{s.title}</div>
                    <div className="muted small">{rm(s.amount_cents)}</div>
                  </div>
                  <StatusPill map={SHARE_STATUS} status={s.status} />
                </Link>
              ))}
            </div>
          ) : (
            <EmptyState title={t('noUnpaid')} />
          )}
        </Card>

        {/* Section 4: Events */}
        <Card className="stack">
          <div className="card-title">
            <h3 className="row nowrap">
              <Calendar size={18} aria-hidden="true" />
              <span>{t('events')}</span>
            </h3>
            <Link to="/events" className="small">{t('allEvents')}</Link>
          </div>
          {events.loading ? (
            <ListSkeleton count={2} />
          ) : events.data?.length ? (
            <div className="list">
              {events.data.slice(0, 4).map((e) => (
                <Link key={e.id} to={`/events/${e.id}`} className="item">
                  <span className="event-emoji" aria-hidden>{EVENT_KIND[e.kind]?.emoji}</span>
                  <div className="grow">
                    <div className="bold">{e.title}</div>
                    <div className="muted small">{ft(EVENT_KIND[e.kind]?.label)} · {e.location}</div>
                  </div>
                  {e.my_signup?.status === 'going' ? (
                    <Pill tone="green">{t('going')}</Pill>
                  ) : e.spots_left != null ? (
                    <Pill tone="amber">{t('spotsLeft', { n: e.spots_left })}</Pill>
                  ) : null}
                </Link>
              ))}
            </div>
          ) : (
            <EmptyState title={t('noEvents')} />
          )}
        </Card>

        {/* Section 5: Announcements */}
        <Card className="stack">
          <div className="card-title">
            <h3 className="row nowrap">
              <Megaphone size={18} aria-hidden="true" />
              <span>{t('announcements')}</span>
            </h3>
          </div>
          {ann.loading ? (
            <ListSkeleton count={2} />
          ) : ann.data?.length ? (
            <div className="stack-sm">
              {ann.data.slice(0, 3).map((a) => (
                <div key={a.id} className="card flat tight">
                  <div className="row between">
                    <strong className="row nowrap">
                      {a.pinned && <Pin size={14} aria-hidden="true" />}
                      <span>{a.title}</span>
                    </strong>
                    <span className="muted tiny">{fmtDay(a.created_at, ft)}</span>
                  </div>
                  <p className="small">{a.body}</p>
                </div>
              ))}
            </div>
          ) : (
            <EmptyState title={t('noAnnouncements')} />
          )}
        </Card>

        {/* Mobile sticky or bottom button */}
        <div className="hide-tablet-up">
          <Button variant="primary" block to="/rides/new" icon={<CalendarPlus size={20} />}>
            {t('signupRide')}
          </Button>
        </div>
      </div>
    </Page>
  );
}

function NextRide({ ride }: { ride: Ride }) {
  const { t } = useTranslation('home');
  const ft = useFormatT();
  const d = ride.driver;

  return (
    <Card variant="hero" className="stack home-hero-card">
      <div className="row between nowrap">
        <span className="bold">{t('nextRide')}</span>
        <span className="pill hero-pill">{ft(RIDE_STATUS[ride.status]?.label)}</span>
      </div>

      <div className="home-hero-time-block">
        <div className="home-hero-time">
          {fmtDay(ride.pickup_at, ft)} · {fmtTime(ride.pickup_at, ft)}
        </div>
      </div>

      <div className="route-display">
        <div className="route-track">
          <span className="route-dot pickup" aria-hidden="true" />
          <span className="route-line" aria-hidden="true" />
          <span className="route-dot dropoff" aria-hidden="true" />
        </div>
        <div className="route-stops">
          <div className="route-stop">
            <span className="muted tiny">{t('pickupArea')}</span>
            <span className="route-location bold">{ride.area.name}</span>
          </div>
          <div className="route-stop">
            <span className="muted tiny">{t('destination')}</span>
            <span className="route-location bold">{ride.destination}</span>
          </div>
        </div>
      </div>

      {d ? (
        <div className="row nowrap hero-row">
          <Avatar name={d.name} size="md" />
          <div className="grow">
            <div className="bold">{d.name} · {t('yourDriver')}</div>
            {d.vehicle && (
              <div className="small muted row nowrap">
                <span>{d.vehicle.color} {d.vehicle.model}</span>
                <span className="plate hero-plate">{d.vehicle.plate}</span>
              </div>
            )}
          </div>
          {d.phone && (
            <a className="btn sm ghost" href={`tel:${d.phone}`} aria-label={t('callDriver')}>
              <Phone size={18} />
            </a>
          )}
        </div>
      ) : null}

      <Button variant="primary" block to={`/rides/${ride.id}`} className="hero-btn">
        {t('viewRide')}
      </Button>
    </Card>
  );
}
