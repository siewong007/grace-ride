import { Link } from 'react-router-dom';
import { CalendarPlus, Phone, Pin } from 'lucide-react';
import { useApi } from '../api';
import { useAuth } from '../auth';
import RideCard, { Ride } from '../components/RideCard';
import { Button, CardSkeleton, EmptyState, ListSkeleton, Page, Pill, StatusPill } from '../components/ui';
import {
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
import type { ReactNode } from 'react';

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

/** A titled list on Home: heading, optional "All →" link, then rows. No frame around it. */
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
  const otherRides = (rides.data ?? []).slice(next ? 1 : 0, 4);
  const unpaid = (shares.data ?? []).filter((s) => s.status !== 'confirmed');
  const formattedToday = fmtDay(`${todayStr()}T00:00`, ft);

  return (
    <Page width="medium">
      <div className="home">
        {/* Greeting, today's date, and the one main action */}
        <div className="page-head home-head">
          <div>
            <h1>{greeting(ft)}{me?.name ? `, ${me.name}` : ''}</h1>
            <p className="muted">{formattedToday}</p>
          </div>
          <div className="btn-row page-head-actions">
            <Button variant="primary" to="/rides/new" icon={<CalendarPlus size={20} />}>
              {t('signupRide')}
            </Button>
            {isStaff && <Link className="btn hide-mobile" to="/admin">{t('dashboard')}</Link>}
          </div>
        </div>

        {/* Next ride */}
        {rides.loading ? (
          <CardSkeleton count={1} />
        ) : next ? (
          <NextRide ride={next} />
        ) : (
          <EmptyState icon={<CalendarPlus size={32} />} title={t('noRide')} />
        )}

        {/* Today's driving (drivers only) */}
        {has('driver') && (
          <Section title={t('driveSchedule')} more={{ to: '/drive', label: t('all') }}>
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

        {/* Other upcoming rides — only when there are some */}
        {otherRides.length > 0 && (
          <Section title={t('otherRides')} more={{ to: '/rides', label: t('all') }}>
            <div className="list">
              {otherRides.map((r) => (
                <RideCard key={r.id} ride={r} />
              ))}
            </div>
          </Section>
        )}

        {/* Unpaid shares — only when something is owed */}
        {unpaid.length > 0 && (
          <Section title={t('unpaid')} more={{ to: '/settlements', label: t('all') }}>
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
          </Section>
        )}

        {/* Events */}
        {(events.loading || (events.data?.length ?? 0) > 0) && (
          <Section title={t('events')} more={{ to: '/events', label: t('allEvents') }}>
            {events.loading ? (
              <ListSkeleton count={2} />
            ) : (
              <div className="list">
                {events.data!.slice(0, 4).map((e) => (
                  <Link key={e.id} to={`/events/${e.id}`} className="item">
                    <div className="grow">
                      <div className="bold">{e.title}</div>
                      <div className="muted small">
                        {fmtDay(e.starts_at, ft)} · {fmtTime(e.starts_at, ft)} · {e.location}
                      </div>
                    </div>
                    {e.my_signup?.status === 'going' ? (
                      <Pill tone="green">{t('going')}</Pill>
                    ) : e.spots_left != null ? (
                      <span className="muted small nowrap">{t('spotsLeft', { n: e.spots_left })}</span>
                    ) : null}
                  </Link>
                ))}
              </div>
            )}
          </Section>
        )}

        {/* Announcements */}
        {(ann.loading || (ann.data?.length ?? 0) > 0) && (
          <Section title={t('announcements')}>
            {ann.loading ? (
              <ListSkeleton count={2} />
            ) : (
              <div className="list">
                {ann.data!.slice(0, 3).map((a) => (
                  <article key={a.id} className="item home-announcement">
                    <div className="grow stack-sm">
                      <div className="row between nowrap">
                        <strong className="row nowrap">
                          {a.pinned && <Pin size={14} aria-hidden="true" />}
                          <span>{a.title}</span>
                        </strong>
                        <span className="muted small nowrap">{fmtDay(a.created_at, ft)}</span>
                      </div>
                      <p className="muted">{a.body}</p>
                    </div>
                  </article>
                ))}
              </div>
            )}
          </Section>
        )}
      </div>
    </Page>
  );
}

function NextRide({ ride }: { ride: Ride }) {
  const { t } = useTranslation('home');
  const ft = useFormatT();
  const d = ride.driver;

  return (
    <section className="card home-next" aria-labelledby="home-next-title">
      <div className="row between nowrap">
        <span id="home-next-title" className="muted small">{t('nextRide')}</span>
        <StatusPill map={RIDE_STATUS} status={ride.status} />
      </div>

      <p className="home-next-time">
        {fmtDay(ride.pickup_at, ft)} · {fmtTime(ride.pickup_at, ft)}
      </p>
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
            <a className="icon-btn" href={`tel:${d.phone}`} aria-label={t('callDriver')}>
              <Phone size={20} />
            </a>
          )}
        </div>
      )}

      <Button variant="secondary" block to={`/rides/${ride.id}`}>
        {t('viewRide')}
      </Button>
    </section>
  );
}
