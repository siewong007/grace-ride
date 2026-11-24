// The one page a visitor sees (/visit/:token). No account: the link is the invitation.
// Opening it changes nothing; "Open my ride" claims it for this phone. The page then shows
// only their ride, and closes for good when the journey ends.
import { useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Calendar, Car, Clock, MapPin, Phone, UserRound } from 'lucide-react';
import { post, useApi } from '../api';
import { downloadIcs } from '../calendar';
import { PublicDisplayBar } from '../components/AppChrome';
import { LogoMark, Wordmark } from '../components/Brand';
import CantMakeItModal from '../components/CantMakeIt';
import LiveMap from '../components/LiveMap';
import TurnstileWidget from '../components/TurnstileWidget';
import { Button, ErrorBox, Notice, Spinner, useAction } from '../components/ui';
import { useTurnstile } from '../turnstile';
import { fmtAgo, fmtDateTime, parseLocal, useFormatT } from '../format';

interface EventBrief {
  title: string;
  starts_at: string;
  location: string;
}

interface Preview {
  state: 'open' | 'not_yet' | 'used' | 'mine' | 'closed';
  closed_reason: string | null;
  opens_at: string;
  name: string;
  host: string;
  event: EventBrief;
}

interface VisitRide {
  name: string;
  host: { name: string; phone: string };
  event: EventBrief;
  ride: {
    status: string;
    pickup_at: string;
    pickup_address: string;
    area: string;
    destination: string;
    readiness_status: string;
    delay_minutes: number | null;
    arrived_at_gate_at: string | null;
    picked_up_at: string | null;
    driver: { name: string; phone: string | null; vehicle: { model: string | null; color: string | null; plate: string | null } } | null;
  };
}

function Shell({ children }: { children: React.ReactNode }) {
  return (
    <div className="auth-page">
      <div className="card auth-card stack visit-card">
        <div className="row between auth-header-row">
          <div className="row nowrap auth-brand">
            <LogoMark size={32} />
            <Wordmark />
          </div>
          <PublicDisplayBar />
        </div>
        {children}
      </div>
    </div>
  );
}

export default function Visit() {
  const { token } = useParams();
  const { t } = useTranslation('visit');
  const ft = useFormatT();
  const preview = useApi<Preview>(token ? `/visit/${token}` : null);
  const { busy, error, run } = useAction();
  const visitTurnstile = useTurnstile();

  if (preview.loading && !preview.data) return <Shell><Spinner /></Shell>;
  const p = preview.data;
  if (!p) return <Shell><ErrorBox error={preview.error ?? t('invalid')} /></Shell>;

  const eventLine = `${p.event.title} · ${fmtDateTime(p.event.starts_at, ft)} · ${p.event.location}`;

  if (p.state === 'mine') return <RideView onEnded={preview.reload} />;

  return (
    <Shell>
      {p.state === 'open' && (
        <>
          <h1 className="visit-title">{t('openTitle', { host: p.host })}</h1>
          <p className="muted">{eventLine}</p>
          <p className="small">{t('openOnePhone')}</p>
          <p className="muted small">{t('privacyNote')}</p>
          <TurnstileWidget
            siteKey={visitTurnstile.siteKey}
            action="visitor_claim"
            resetKey={visitTurnstile.resetKey}
            onToken={visitTurnstile.setToken}
          />
          <ErrorBox error={error} />
          <Button
            variant="primary"
            size="lg"
            block
            disabled={busy || (visitTurnstile.enabled && !visitTurnstile.token)}
            onClick={() => {
              if (visitTurnstile.enabled && !visitTurnstile.token) return;
              void run(async () => {
                try {
                  await (visitTurnstile.token
                    ? post(`/visit/${token}/claim`, { turnstile_token: visitTurnstile.token })
                    : post(`/visit/${token}/claim`));
                  await preview.reload();
                } catch (err) {
                  visitTurnstile.reset();
                  throw err;
                }
              });
            }}
          >
            {t('openButton')}
          </Button>
        </>
      )}
      {p.state === 'not_yet' && (
        <>
          <h1 className="visit-title">{t('notYetTitle', { when: fmtDateTime(p.opens_at, ft) })}</h1>
          <p className="muted">{eventLine}</p>
          <p>{t('notYetBody')}</p>
          <Button
            variant="secondary"
            block
            icon={<Calendar size={18} />}
            onClick={() =>
              downloadIcs(
                {
                  title: p.event.title,
                  description: `${t('openTitle', { host: p.host })}\n${typeof window !== 'undefined' ? window.location.href : ''}`,
                  location: p.event.location,
                  startsAt: p.event.starts_at,
                },
                `${p.event.title}.ics`,
              )
            }
          >
            {t('addToCalendar')}
          </Button>
        </>
      )}
      {p.state === 'used' && (
        <>
          <h1 className="visit-title">{t('usedTitle')}</h1>
          <p>{t('usedBody', { host: p.host })}</p>
        </>
      )}
      {p.state === 'closed' && (
        <h1 className="visit-title">
          {p.closed_reason === 'completed' || p.closed_reason === 'expired'
            ? t('endedThanks')
            : p.closed_reason === 'cancelled'
              ? t('endedCancelled')
              : t('endedStopped', { host: p.host })}
        </h1>
      )}
    </Shell>
  );
}

function RideView({ onEnded }: { onEnded: () => void }) {
  const { t } = useTranslation('visit');
  const ft = useFormatT();
  // Polled by hand: the member event stream needs a member session.
  const { data, error, reload } = useApi<VisitRide>('/visit/me');
  const { busy, error: actErr, run } = useAction();
  const [cant, setCant] = useState(false);
  useEffect(() => {
    const id = setInterval(() => document.visibilityState === 'visible' && reload(), 15000);
    return () => clearInterval(id);
  }, [reload]);
  // Ended (or this phone was signed out): show the link's final state instead.
  useEffect(() => {
    if (error) onEnded();
  }, [error, onEnded]);

  if (!data) return <Shell><Spinner /></Shell>;
  const r = data.ride;
  const d = r.driver;
  const aboard = !!r.picked_up_at;
  const canRespond = (r.status === 'confirmed' || r.status === 'in_progress') && !aboard;
  const stage = aboard ? t('stage.aboard', { event: data.event.title }) : t(`stage.${r.status}`, { host: data.host.name, defaultValue: '' });
  const send = (status: string, done: string) => run(() => post('/visit/me/readiness', { status }).then(reload), done);

  return (
    <Shell>
      <div className="stack-sm">
        <span className="muted small">{t('hello', { name: data.name })}</span>
        <h1 className="visit-title">{t('rideTo', { event: data.event.title })}</h1>
        {stage && <p className="bold visit-stage">{stage}</p>}
      </div>

      {r.arrived_at_gate_at && !aboard && <Notice tone="success">{t('atGate')}</Notice>}
      {!!r.delay_minutes && r.delay_minutes > 0 && !aboard && (
        <Notice tone="warning" icon={<Clock size={18} />}>{t('driverLate', { mins: r.delay_minutes })}</Notice>
      )}

      <dl className="visit-facts">
        <div><dt><Clock size={16} aria-hidden="true" /> {t('time')}</dt><dd>{fmtDateTime(r.pickup_at, ft)}</dd></div>
        <div><dt><MapPin size={16} aria-hidden="true" /> {t('pickup')}</dt><dd>{r.pickup_address || r.area}</dd></div>
        {d && (
          <>
            <div><dt><UserRound size={16} aria-hidden="true" /> {t('driver')}</dt><dd>{d.name}</dd></div>
            {d.vehicle.plate && (
              <div>
                <dt><Car size={16} aria-hidden="true" /> {t('car')}</dt>
                <dd>{[d.vehicle.color, d.vehicle.model].filter(Boolean).join(' ')} <span className="plate">{d.vehicle.plate}</span></dd>
              </div>
            )}
          </>
        )}
      </dl>

      {d?.phone && (
        <a className="btn secondary block" href={`tel:${d.phone}`}>
          <Phone size={18} aria-hidden="true" /> {t('callDriver')}
        </a>
      )}

      {r.status === 'in_progress' && !aboard && <DriverSpot driverName={d?.name} />}

      {canRespond && (
        <div className="stack-sm">
          <p className="muted small">{t('letDriverKnow')}</p>
          {r.readiness_status === 'ready' && <Notice tone="success">{t('readySent')}</Notice>}
          {r.readiness_status.startsWith('delayed') && <Notice tone="warning">{t('lateSent')}</Notice>}
          <div className="btn-row visit-ready">
            <Button variant={r.readiness_status === 'ready' ? 'secondary' : 'primary'} disabled={busy} onClick={() => send('ready', t('readySent'))}>
              {t('ready')}
            </Button>
            <Button variant="secondary" disabled={busy} onClick={() => send('delayed_5m', t('lateSent'))}>
              {t('late')}
            </Button>
          </div>
          <Button variant="ghost" className="text-danger" disabled={busy} onClick={() => setCant(true)}>
            {t('cantMakeIt')}
          </Button>
        </div>
      )}
      <ErrorBox error={actErr} />

      {data.host.phone && (
        <a className="small center" href={`tel:${data.host.phone}`}>{t('askHost', { host: data.host.name })}</a>
      )}

      {cant && <CantMakeItModal rideId={0} endpoint="/visit/me/readiness" onClose={() => setCant(false)} onDone={onEnded} />}
    </Shell>
  );
}

/** The driver's last known position while the ride is under way. */
function DriverSpot({ driverName }: { driverName?: string }) {
  const { t } = useTranslation('visit');
  const ft = useFormatT();
  const { data, reload } = useApi<{ sharing: boolean; lat?: number; lng?: number; updated_at?: string }>('/visit/me/location');
  useEffect(() => {
    const id = setInterval(() => document.visibilityState === 'visible' && reload(), 15000);
    return () => clearInterval(id);
  }, [reload]);
  if (!data?.sharing || data.lat == null || data.lng == null) return null;
  const ageLabel = data.updated_at ? fmtAgo(data.updated_at, ft) : '';
  const isStale = data.updated_at ? Date.now() - parseLocal(data.updated_at).getTime() > 5 * 60_000 : false;

  return (
    <div className="stack-xs visit-spot-wrap">
      {isStale && ageLabel && (
        <Notice tone="warning">
          <span>{t('staleLocation', { when: ageLabel })}</span>
        </Notice>
      )}
      <LiveMap
        points={[
          {
            id: 'driver',
            lat: data.lat,
            lng: data.lng,
            label: driverName ?? t('driver'),
            detail: ageLabel ? t('updated', { when: ageLabel }) : undefined,
          },
        ]}
      />
      <p className="row between small visit-spot">
        <span>{t('driverHere')}{ageLabel ? ` · ${t('updated', { when: ageLabel })}` : ''}</span>
        <a href={`https://www.google.com/maps/search/?api=1&query=${data.lat},${data.lng}`} target="_blank" rel="noopener noreferrer">
          {t('openMap')}
        </a>
      </p>
    </div>
  );
}
