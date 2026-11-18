import { useEffect } from 'react';
import { useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Car, CheckCircle2, Clock, MapPin, ShieldCheck, UserRound } from 'lucide-react';
import { post, useApi } from '../api';
import { PublicDisplayBar } from '../components/AppChrome';
import { LogoMark, Wordmark } from '../components/Brand';
import LiveMap from '../components/LiveMap';
import { Button, ErrorBox, Notice, Pill, Spinner, StatusPill, useAction } from '../components/ui';
import { fmtAgo, fmtDateTime, parseLocal, RIDE_STATUS_PASSENGER, todayStr, useFormatT } from '../format';

interface CareStatus {
  ride_id: number;
  label: string;
  rider_name: string;
  booker_name: string;
  status: string;
  leg?: 'outbound' | 'return';
  pickup_at: string;
  area: { id: number; name: string };
  destination: string;
  readiness_status: string;
  delay_minutes: number;
  arrived_at_gate_at: string | null;
  started_at: string | null;
  picked_up_at: string | null;
  completed_at: string | null;
  driver: {
    name: string;
    vehicle: { model: string | null; color: string | null; plate: string | null } | null;
  } | null;
  location: {
    sharing: boolean;
    lat?: number;
    lng?: number;
    updated_at?: string;
    eta?: { duration_s?: number; distance_m?: number; eta?: string };
  };
}

function CareShell({ children }: { children: React.ReactNode }) {
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

export default function CarePass() {
  const { token } = useParams();
  const { t } = useTranslation('visit');
  const ft = useFormatT();
  const { data, error, loading, reload } = useApi<CareStatus>(token ? `/care/${token}` : null);
  const { busy, error: actErr, run } = useAction();

  const terminal = Boolean(
    data && ['completed', 'cancelled', 'no_show', 'alt_transport'].includes(data.status),
  );
  const stopped = Boolean(error) || terminal;
  useEffect(() => {
    if (!token || stopped) return;
    const id = setInterval(() => {
      if (document.visibilityState === 'visible') void reload();
    }, 15000);
    return () => clearInterval(id);
  }, [token, stopped, reload]);

  if (loading && !data) return <CareShell><Spinner /></CareShell>;
  if (error || !data) {
    return (
      <CareShell>
        <div className="stack-sm">
          <h1 className="visit-title">{t('care.closedTitle')}</h1>
          <ErrorBox error={error || t('invalid')} />
        </div>
      </CareShell>
    );
  }

  const aboard = !!data.picked_up_at;
  const completed = data.status === 'completed';
  const canRespond =
    !terminal &&
    !aboard &&
    (data.status === 'in_progress' ||
      (data.status === 'confirmed' && data.pickup_at.slice(0, 10) <= todayStr()));
  const send = (status: string, done: string) =>
    run(() => post(`/care/${token}/readiness`, { status }).then(reload), done);

  const loc = data.location;
  const ageLabel = loc.updated_at ? fmtAgo(loc.updated_at, ft) : '';
  const isStale = loc.updated_at ? Date.now() - parseLocal(loc.updated_at).getTime() > 5 * 60_000 : false;

  return (
    <CareShell>
      <div className="stack-sm">
        <div className="row between nowrap gap-xs">
          <span className="pill info row nowrap gap-xs">
            <ShieldCheck size={14} aria-hidden="true" />
            <span>{data.label || t('care.badge')}</span>
          </span>
          <StatusPill map={RIDE_STATUS_PASSENGER} status={data.status} />
        </div>
        <h1 className="visit-title">{t('care.title', { name: data.rider_name })}</h1>
        <p className="muted small">
          {t('care.privacyHint')}
          {data.leg === 'return' ? ` · ${t('care.returnLeg')}` : ''}
        </p>
      </div>

      {completed && (
        <Notice tone="success" icon={<CheckCircle2 size={18} />}>
          {t('care.arrivedSafely', { dest: data.destination })}
        </Notice>
      )}
      {!terminal && aboard && (
        <Notice tone="info" icon={<Car size={18} />}>
          {t('care.pickedUpOnWay', { dest: data.destination })}
        </Notice>
      )}
      {!terminal && data.arrived_at_gate_at && !aboard && (
        <Notice tone="success">{t('atGate')}</Notice>
      )}
      {!terminal && data.delay_minutes > 0 && !aboard && (
        <Notice tone="warning" icon={<Clock size={18} />}>
          {t('driverLate', { mins: data.delay_minutes })}
        </Notice>
      )}

      <dl className="visit-facts">
        <div>
          <dt><Clock size={16} aria-hidden="true" /> {t('time')}</dt>
          <dd>{fmtDateTime(data.pickup_at, ft)}</dd>
        </div>
        <div>
          <dt><MapPin size={16} aria-hidden="true" /> {t('care.route')}</dt>
          <dd>{data.area.name} → {data.destination}</dd>
        </div>
        {data.driver && (
          <>
            <div>
              <dt><UserRound size={16} aria-hidden="true" /> {t('driver')}</dt>
              <dd>{data.driver.name}</dd>
            </div>
            {data.driver.vehicle?.plate && (
              <div>
                <dt><Car size={16} aria-hidden="true" /> {t('car')}</dt>
                <dd>
                  {[data.driver.vehicle.color, data.driver.vehicle.model].filter(Boolean).join(' ')}{' '}
                  <span className="plate">{data.driver.vehicle.plate}</span>
                </dd>
              </div>
            )}
          </>
        )}
      </dl>

      {loc.sharing && loc.lat != null && loc.lng != null && !terminal && (
        <div className="stack-xs visit-spot-wrap">
          {isStale && ageLabel && (
            <Notice tone="warning">{t('staleLocation', { when: ageLabel })}</Notice>
          )}
          <LiveMap
            points={[
              {
                id: 'driver',
                lat: loc.lat,
                lng: loc.lng,
                label: data.driver?.name ?? t('driver'),
                detail: ageLabel ? t('updated', { when: ageLabel }) : undefined,
              },
            ]}
          />
          <p className="row between small visit-spot">
            <span>{t('driverHere')}{ageLabel ? ` · ${t('updated', { when: ageLabel })}` : ''}</span>
            <Pill tone="info">{data.area.name}</Pill>
          </p>
        </div>
      )}

      {canRespond && (
        <div className="stack-sm">
          <p className="muted small">{t('care.readinessLabel')}</p>
          {data.readiness_status === 'ready' && <Notice tone="success">{t('readySent')}</Notice>}
          {data.readiness_status.startsWith('delayed') && <Notice tone="warning">{t('lateSent')}</Notice>}
          <div className="btn-row visit-ready">
            <Button
              variant={data.readiness_status === 'ready' ? 'secondary' : 'primary'}
              disabled={busy}
              onClick={() => send('ready', t('readySent'))}
            >
              {t('care.riderReady', { name: data.rider_name })}
            </Button>
            <Button
              variant="secondary"
              disabled={busy}
              onClick={() => send('delayed_5m', t('lateSent'))}
            >
              {t('late')}
            </Button>
          </div>
          <ErrorBox error={actErr} />
        </div>
      )}
    </CareShell>
  );
}
