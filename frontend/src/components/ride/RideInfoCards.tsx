import { Link } from 'react-router-dom';
import {
  AlertCircle,
  Calendar,
  CheckCircle2,
  Circle,
  CircleDot,
  Clock,
  Lock,
  MessageSquare,
  Phone,
  Share2,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { downloadIcs } from '../../calendar';
import { fmtAgo, fmtDateTime, RIDE_STATUS, useFormatT } from '../../format';
import type { Ride } from '../RideCard';
import { NavButtonGroup } from '../TripActions';
import { AreaPhotoStrip } from '../AreaPhotos';
import { Avatar, Button, KeyValue, KeyValueItem, Notice, Pill } from '../ui';

export function DriverCard({
  ride: r,
  onChatClick,
}: {
  ride: Ride;
  onChatClick?: () => void;
}) {
  const { t } = useTranslation('rideDetail');
  const d = r.driver;

  if (!d) {
    return (
      <Notice tone="info" icon={<Clock size={18} />}>
        {t('noDriverYet')}
      </Notice>
    );
  }

  return (
    <section className="detail-section ride-driver">
      <div className="row nowrap gap-md min-w-0">
        <Avatar name={d.name} fileId={d.photo_file_id} color={d.avatar_color} icon={d.avatar_icon} size="lg" />
        <div className="min-w-0">
          <div className="muted small">
            {r.viewer.is_driver ? t('youAreDriver') : t('driver')}
          </div>
          <div className="bold font-lg">{d.name}</div>
          {d.vehicle && (
            <div className="small muted ride-driver-car">
              {d.vehicle.color} {d.vehicle.model} <span className="plate">{d.vehicle.plate}</span>
            </div>
          )}
        </div>
      </div>

      <div className="btn-row ride-driver-actions">
        {d.phone && !r.viewer.is_driver && (
          <a className="btn sm secondary" href={`tel:${d.phone}`}>
            <Phone size={16} aria-hidden="true" />
            <span>{t('call')}</span>
          </a>
        )}
        {r.trip_id != null && onChatClick && (
          <Button variant="ghost" size="sm" onClick={onChatClick}>
            <MessageSquare size={16} aria-hidden="true" />
            <span>{t('chatBtn')}</span>
          </Button>
        )}
      </div>
    </section>
  );
}

export function StatusTimeline({ ride }: { ride: Ride }) {
  const { t } = useTranslation('rideDetail');
  const ft = useFormatT();

  const isTerminal = ['cancelled', 'no_show', 'alt_transport'].includes(ride.status);

  if (isTerminal) {
    return (
      <section className="detail-section stack-sm">
        <h2>{t('statusTimeline')}</h2>
        <div className="status-timeline">
          <div className="timeline-step done">
            <div className="timeline-step-indicator">
              <div className="timeline-step-icon">
                <CheckCircle2 size={14} aria-hidden="true" />
              </div>
              <div className="timeline-step-line" />
            </div>
            <div className="timeline-step-content">
              <span className="timeline-step-title">{t('flow0')}</span>
              <span className="timeline-step-desc">{fmtDateTime(ride.pickup_at, ft)}</span>
            </div>
          </div>
          <div className="timeline-step cancelled">
            <div className="timeline-step-indicator">
              <div className="timeline-step-icon">
                <AlertCircle size={14} aria-hidden="true" />
              </div>
            </div>
            <div className="timeline-step-content">
              <span className="timeline-step-title">{ft(RIDE_STATUS[ride.status]?.label)}</span>
              {ride.attention_reason && (
                <span className="timeline-step-desc">{ride.attention_reason}</span>
              )}
            </div>
          </div>
        </div>
      </section>
    );
  }

  type StepStatus = 'done' | 'current' | 'upcoming';

  const getStepState = (stepIndex: number): StepStatus => {
    let currentIdx = 0;
    if (ride.status === 'pending_driver') currentIdx = 0;
    else if (ride.status === 'pending_passenger' || ride.status === 'pending_driver_confirm') currentIdx = 1;
    else if (ride.status === 'pending_approval') currentIdx = 1;
    else if (ride.status === 'confirmed') currentIdx = 2;
    else if (ride.status === 'in_progress') currentIdx = 3;
    else if (ride.status === 'completed') currentIdx = 4;

    if (stepIndex < currentIdx) return 'done';
    if (stepIndex === currentIdx) return 'current';
    return 'upcoming';
  };

  const steps = [
    {
      title: t('flow0'),
      state: getStepState(0),
      time: fmtDateTime(ride.pickup_at, ft),
    },
    {
      title: t('flow1'),
      state: getStepState(1),
      desc: ride.driver ? ride.driver.name : undefined,
    },
    {
      title: t('flow2'),
      state: getStepState(2),
    },
    {
      title: ft('format:rideStatus.in_progress'),
      state: getStepState(3),
      time: ride.started_at ? fmtDateTime(ride.started_at, ft) : undefined,
      desc: ride.picked_up_at ? `${t('pickedUp')} (${fmtAgo(ride.picked_up_at, ft)})` : undefined,
    },
    {
      title: t('flow3'),
      state: getStepState(4),
      time: ride.completed_at ? fmtDateTime(ride.completed_at, ft) : undefined,
    },
  ];

  const isLive = ride.status === 'in_progress';

  return (
    <section className="detail-section stack-sm">
      <h2>{t('statusTimeline')}</h2>
      <div className="status-timeline">
        {steps.map((st, i) => (
          <div key={i} className={`timeline-step ${st.state}${st.state === 'current' && isLive ? ' is-live' : ''}`}>
            <div className="timeline-step-indicator">
              <div className="timeline-step-icon">
                {st.state === 'done' ? (
                  <CheckCircle2 size={14} aria-hidden="true" />
                ) : st.state === 'current' ? (
                  <CircleDot size={14} aria-hidden="true" />
                ) : (
                  <Circle size={10} aria-hidden="true" />
                )}
              </div>
              {i < steps.length - 1 && <div className="timeline-step-line" />}
            </div>
            <div className="timeline-step-content">
              <span className="timeline-step-title">{st.title}</span>
              {st.time && <span className="timeline-step-desc">{st.time}</span>}
              {st.desc && <span className="timeline-step-desc muted">{st.desc}</span>}
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}

export function Details({ ride: r }: { ride: Ride }) {
  const { t } = useTranslation('rideDetail');
  const ft = useFormatT();

  const kvItems: KeyValueItem[] = [];

  if (r.is_proxy && r.proxy) {
    kvItems.push({
      label: t('proxyRider'),
      value: (
        <span>
          <strong className="bold">{r.proxy.rider_name}</strong>
          {r.proxy.rider_phone ? (
            <>
              {' '}
              · <a href={`tel:${r.proxy.rider_phone}`}>{r.proxy.rider_phone}</a>
            </>
          ) : (
            <span className="small muted"> ({t('proxyNoPhone')})</span>
          )}
        </span>
      ),
    });
    if (r.proxy.rider_relationship) {
      kvItems.push({
        label: t('proxyRelationship'),
        value: r.proxy.rider_relationship,
      });
    }
    kvItems.push({
      label: t('proxyBooker'),
      value: (
        <span>
          {r.proxy.booker_name}
          {r.proxy.booker_phone && (
            <>
              {' '}
              · <a href={`tel:${r.proxy.booker_phone}`}>{r.proxy.booker_phone}</a>
            </>
          )}
        </span>
      ),
    });
  } else {
    kvItems.push({
      label: t('passenger'),
      value: (
        <span>
          {r.passenger.name}
          {r.passenger.phone && !r.viewer.is_passenger && (
            <>
              {' '}
              · <a href={`tel:${r.passenger.phone}`}>{r.passenger.phone}</a>
            </>
          )}
        </span>
      ),
    });
  }

  kvItems.push(
    { label: t('pickupArea'), value: r.area.name },
    {
      label: t('pickupAddress'),
      value:
        r.pickup_address ? (
          <span>{r.pickup_address}</span>
        ) : (
          <div className="row between nowrap">
            <span className="muted">{t('pickupAddressLocked')}</span>
            <Lock size={14} className="muted" aria-hidden="true" />
          </div>
        ),
    },
    { label: t('destination'), value: r.destination },
    { label: t('people'), value: t('seatCount', { count: r.seats }) },
  );

  if (r.assistance) {
    kvItems.push({ label: t('assistance'), value: r.assistance });
  }
  if (r.notes) {
    kvItems.push({ label: t('notes'), value: r.notes });
  }
  if (r.event) {
    kvItems.push({
      label: t('event'),
      // Someone outside the event's groups learns only that it is a church event.
      value: r.event.id != null ? <Link to={`/events/${r.event.id}`}>{r.event.title}</Link> : t('churchEvent', { ns: 'rideCard' }),
    });
  }
  if (r.route_fare) {
    const f = r.route_fare;
    kvItems.push({
      label: t('routeFare'),
      value: (
        <span className="row wrap gap-sm">
          {f.amount_cents != null ? (
            <strong>RM {(f.amount_cents / 100).toFixed(2)}</strong>
          ) : (
            <Pill tone="warning">{t('routeFareNotSet')}</Pill>
          )}
          <span className="muted small">{f.from} ↔ {f.to}</span>
        </span>
      ),
    });
  }
  if (r.picked_up_at) {
    kvItems.push({
      label: t('pickedUp'),
      value: fmtAgo(r.picked_up_at, ft),
    });
  }

  return (
    <section className="detail-section stack">
      <KeyValue items={kvItems} />

      <AreaPhotoStrip areaId={r.area.id} areaName={r.area.name} />

      {r.pickup_address && (r.viewer.is_driver || r.viewer.is_staff) && (
        <NavButtonGroup address={r.pickup_address} />
      )}

      {['confirmed', 'in_progress', 'completed'].includes(r.status) && (
        <div className="btn-row">
          <Button
            variant="ghost"
            size="sm"
            icon={<Calendar size={16} />}
            onClick={() => {
              const d = r.driver;
              downloadIcs(
                {
                  title: `Grace Ride: ${r.destination}`,
                  description: `Driver: ${d?.name ?? ''} (${[d?.vehicle?.color, d?.vehicle?.model, d?.vehicle?.plate].filter(Boolean).join(' ')})\nPickup: ${r.pickup_address || r.area.name}`,
                  location: r.pickup_address || r.area.name,
                  startsAt: r.pickup_at,
                  durationMinutes: 60,
                },
                `GraceRide-${r.id}.ics`,
              );
            }}
          >
            {t('addToCalendar')}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            icon={<Share2 size={16} />}
            onClick={() => {
              const d = r.driver;
              const text = t('shareRideText', {
                time: fmtDateTime(r.pickup_at, ft),
                pickup: r.pickup_address || r.area.name,
                dest: r.destination,
                driver: d
                  ? `${d.name} (${[d.vehicle?.color, d.vehicle?.model, d.vehicle?.plate].filter(Boolean).join(' ')})`
                  : '—',
              });
              window.open(`https://wa.me/?text=${encodeURIComponent(text)}`, '_blank', 'noopener,noreferrer');
            }}
          >
            {t('shareWhatsApp')}
          </Button>
        </div>
      )}
    </section>
  );
}
