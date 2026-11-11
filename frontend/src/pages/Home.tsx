import { Link } from 'react-router-dom';
import { CalendarPlus, CarFront, LifeBuoy, Phone } from 'lucide-react';
import { useApi } from '../api';
import { useAuth } from '../auth';
import RideCard, { Ride } from '../components/RideCard';
import { Avatar, Empty, Pill, StatusPill } from '../components/ui';
import { EVENT_KIND, fmtDay, fmtTime, greeting, RIDE_STATUS, rm, SHARE_STATUS, todayStr, useFormatT } from '../format';
import type { EventItem } from './Events';
import { useTranslation } from 'react-i18next';

interface Share { id: number; title: string; amount_cents: number; status: string; due_date: string | null }
interface Announcement { id: number; title: string; body: string; pinned: boolean; by: string; created_at: string }

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

  return (
    <div className="stack">
      <div className="page-head">
        <div>
          <h1>{greeting(ft)}{me?.name ? ` ${me.name}` : ''}</h1>
          <p>
            <Pill tone="green">{t('verified')}</Pill>{' '}
            {me?.groups.map((g) => <Pill key={g.id}>{g.name}</Pill>)}
          </p>
        </div>
        <div className="btn-row hide-mobile">
          <Link className="btn primary" to="/rides/new"><CalendarPlus size={20} />{t('signupRide')}</Link>
          {isStaff && <Link className="btn" to="/admin">{t('dashboard')}</Link>}
        </div>
      </div>

      <div className="split">
        <div className="stack">
          {next ? <NextRide ride={next} t={t} ft={ft} /> : (
            <div className="card stack">
              <h3>{t('nextRide')}</h3>
              <p className="muted">{t('noRide')}</p>
              <Link className="btn primary" to="/rides/new"><CalendarPlus size={20} />{t('signupRide')}</Link>
            </div>
          )}
          {rides.data && rides.data.length > 1 && (
            <div className="stack-sm">
              <h3>{t('otherRides')}</h3>
              {rides.data.slice(1, 4).map((r) => <RideCard key={r.id} ride={r} />)}
            </div>
          )}

          {has('driver') && (
            <div className="card">
              <div className="card-title">
                <h3><CarFront size={18} /> {t('driveSchedule')}</h3>
                <Link to="/drive" className="small">{t('all')}</Link>
              </div>
              {driving.data?.length ? (
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
              ) : <p className="muted">{t('noDriveToday')}</p>}
            </div>
          )}

          <div className="card stack">
            <div className="card-title">
              <h3>{t('unpaid')}</h3>
            </div>
            {unpaid.length ? (
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
            ) : <p className="muted">{t('noUnpaid')}</p>}
          </div>
        </div>

        <div className="stack">
          <div className="card">
            <div className="card-title">
              <h3>{t('events')}</h3>
              <Link to="/events" className="small">{t('allEvents')}</Link>
            </div>
            {events.data?.length ? (
              <div className="list">
                {events.data.slice(0, 4).map((e) => (
                  <Link key={e.id} to={`/events/${e.id}`} className="item">
                    <span className="event-emoji" aria-hidden>{EVENT_KIND[e.kind]?.emoji}</span>
                    <div className="grow">
                      <div className="bold">{e.title}</div>
                      <div className="muted small">{ft(EVENT_KIND[e.kind]?.label)} · {e.location}</div>
                    </div>
                    {e.my_signup?.status === 'going' ? <Pill tone="green">{t('going')}</Pill> : e.spots_left != null ? <Pill tone="amber">余 {e.spots_left}</Pill> : null}
                  </Link>
                ))}
              </div>
            ) : <Empty>{t('noEvents')}</Empty>}
          </div>

          <div className="card">
            <h3 style={{ marginBottom: 10 }}>{t('announcements')}</h3>
            {ann.data?.length ? (
              <div className="stack-sm">
                {ann.data.slice(0, 3).map((a) => (
                  <div key={a.id} className="card flat tight">
                    <div className="row between"><strong>{a.pinned && '📌 '}{a.title}</strong><span className="muted tiny">{fmtDay(a.created_at, ft)}</span></div>
                    <p className="small" style={{ whiteSpace: 'pre-wrap' }}>{a.body}</p>
                  </div>
                ))}
              </div>
            ) : <p className="muted">{t('noAnnouncements')}</p>}
          </div>

          <div className="btn-row">
            <Link className="btn" to="/report">{t('viewReport')}</Link>
            <Link className="btn" to="/help"><LifeBuoy size={18} />{t('helpCentre')}</Link>
          </div>
        </div>
      </div>
      <Link className="btn primary block hide-tablet-up" to="/rides/new"><CalendarPlus size={20} />{t('signupRide')}</Link>
    </div>
  );
}

interface NextRideProps { ride: Ride; t: (k: string, opts?: any) => string; ft: (k: string) => string }
function NextRide({ ride, t, ft }: NextRideProps) {
  const d = ride.driver;
  return (
    <div className="card hero stack">
      <div className="row between">
        <span className="bold">{t('nextRide')}</span>
        <span className="pill" style={{ background: 'rgba(255,255,255,.18)', color: 'inherit' }}>{ft(RIDE_STATUS[ride.status]?.label)}</span>
      </div>
      <div>
        <div style={{ fontSize: '1.9rem', fontWeight: 800 }}>{fmtDay(ride.pickup_at, ft)} {fmtTime(ride.pickup_at, ft)}</div>
        <div className="muted">{ride.area.name} → {ride.destination}</div>
      </div>
      {d ? (
        <div className="row nowrap" style={{ background: 'rgba(255,255,255,.1)', padding: 12, borderRadius: 12 }}>
          <Avatar name={d.name} />
          <div className="grow">
            <div className="bold">{d.name} · {t('yourDriver')}</div>
            {d.vehicle && <div className="small">{d.vehicle.color} {d.vehicle.model} · <span className="plate" style={{ borderColor: 'currentColor', padding: '0 6px' }}>{d.vehicle.plate}</span></div>}
          </div>
          {d.phone && <a className="btn sm" href={`tel:${d.phone}`} aria-label={t('callDriver')}><Phone size={18} /></a>}
        </div>
      ) : <div className="muted">{t('waitingDriver')}</div>}
      <Link className="btn block" to={`/rides/${ride.id}`} style={{ color: 'var(--text)' }}>{t('viewDetail')}</Link>
    </div>
  );
}
