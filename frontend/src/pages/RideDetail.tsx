import { useEffect, useRef, useState } from 'react';
import { Link, useLocation, useParams, useSearchParams } from 'react-router-dom';
import { AlertTriangle, LifeBuoy, Repeat, ShieldCheck } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { post, useApi } from '../api';
import { useAuth } from '../auth';
import Chat from '../components/Chat';
import type { Ride } from '../components/RideCard';
import {
  ReasonModal,
  ShareLocation,
  StartTripButton,
  requestPosition,
  sendDriverPosition,
} from '../components/TripActions';
import { CareLinkModal } from '../components/ride/CareLinkModal';
import {
  DriverActions,
  MobileStickyActions,
  PassengerActions,
  StaffActions,
} from '../components/ride/RideActionPanels';
import { Details, DriverCard, StatusTimeline } from '../components/ride/RideInfoCards';
import { CollapsibleTrackCard } from '../components/ride/RideTracking';
import {
  Button,
  ErrorBox,
  Notice,
  Page,
  PageHeader,
  Spinner,
  StatusPill,
} from '../components/ui';
import {
  fmtAgo,
  fmtDateTime,
  RIDE_STATUS,
  RIDE_STATUS_PASSENGER,
  todayStr,
  useFormatT,
} from '../format';
import { loadTodayRideById, saveTodayRidesSnapshot } from '../offlineSnapshot';

export { StartTripButton, ShareLocation, requestPosition, sendDriverPosition, ReasonModal };

export default function RideDetail() {
  const { t } = useTranslation('rideDetail');
  const loc = useLocation();
  const ft = useFormatT();
  const { id } = useParams();
  const [searchParams, setSearchParams] = useSearchParams();
  const quickParam = searchParams.get('quick');
  const chatRef = useRef<HTMLDivElement>(null);
  const handledQuickRef = useRef<string | null>(null);
  const { data: liveRide, error, reload } = useApi<Ride>(`/rides/${id}`, 15000);
  const auth = useAuth();
  const canAct = auth?.can ?? (() => true);
  const canUpdateRides = canAct('rides', 'update');
  const [staleRefreshed, setStaleRefreshed] = useState(false);
  const [careModal, setCareModal] = useState(false);

  useEffect(() => {
    if (
      !liveRide ||
      String(liveRide.id) !== id ||
      (quickParam !== 'ready' && quickParam !== 'delayed_5m')
    ) {
      return;
    }
    const key = `${liveRide.id}:${quickParam}`;
    if (handledQuickRef.current === key) return;
    if (
      !liveRide.viewer.is_passenger ||
      !canUpdateRides ||
      (liveRide.status !== 'confirmed' && liveRide.status !== 'in_progress') ||
      liveRide.picked_up_at
    ) {
      return;
    }
    handledQuickRef.current = key;
    setSearchParams(
      (prev) => {
        const next = new URLSearchParams(prev);
        next.delete('quick');
        return next;
      },
      { replace: true },
    );
    void post(`/rides/${liveRide.id}/readiness`, { status: quickParam })
      .then(() => reload())
      .catch(() => {});
  }, [liveRide, id, quickParam, canUpdateRides, setSearchParams, reload]);

  useEffect(() => {
    if (auth?.me?.id && liveRide && String(liveRide.id) === id) {
      saveTodayRidesSnapshot(auth.me.id, [liveRide]);
    }
  }, [auth?.me?.id, liveRide, id]);

  useEffect(() => {
    const onStale = () => {
      setStaleRefreshed(true);
      void reload();
    };
    window.addEventListener('gr:ride-stale', onStale);
    return () => window.removeEventListener('gr:ride-stale', onStale);
  }, [reload]);

  const offlineFallback =
    !liveRide && error && auth?.me?.id && id ? loadTodayRideById(auth.me.id, Number(id)) : null;
  const r = liveRide ?? offlineFallback?.ride ?? null;

  if (error && !offlineFallback) {
    return (
      <Page width="wide">
        <ErrorBox error={error} onRetry={reload} />
      </Page>
    );
  }
  // Never act on a ride other than the one in the URL, even for a moment.
  if (!r || String(r.id) !== id) return <Spinner />;

  const chatVisible = r.trip_id != null && (r.viewer.is_passenger || r.viewer.is_driver || r.viewer.is_staff);
  const live = r.status === 'in_progress';
  const showHelpLink = r.viewer.is_passenger || r.viewer.is_driver;
  const showCareLink =
    (r.viewer.is_passenger || (r.viewer.is_staff && canAct('admin_rides', 'update'))) &&
    !['completed', 'cancelled', 'no_show', 'alt_transport'].includes(r.status);
  // The passenger has something to do now (answer a driver, say they are ready or late):
  // show those controls beside the driver card on every screen, not at the bottom or
  // behind "More actions".
  const passengerTurn =
    r.viewer.is_passenger &&
    canAct('rides', 'update') &&
    (r.status === 'pending_passenger' ||
      // "Ready / running late" matters on the day itself, not days ahead.
      (r.status === 'confirmed' && r.pickup_at.slice(0, 10) <= todayStr()) ||
      (r.status === 'in_progress' && !r.picked_up_at));

  const scrollToChat = () => {
    chatRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  return (
    <Page width="wide">
      <div className="stack">
        {r.event && (
          <div className="muted small">
            {r.event.id != null ? (
              <Link to={`/events/${r.event.id}`}>{r.event.title ?? t('churchEvent', { ns: 'rideCard' })}</Link>
            ) : (
              <span>{t('churchEvent', { ns: 'rideCard' })}</span>
            )}
          </div>
        )}
        {/* Status Header */}
        <PageHeader
          title={fmtDateTime(r.pickup_at, ft)}
          subtitle={`${r.area.name} → ${r.destination}`}
          back={loc.pathname.startsWith('/admin/') ? '/admin/rides' : '/rides'}
          actions={<StatusPill map={r.viewer.is_passenger ? RIDE_STATUS_PASSENGER : RIDE_STATUS} status={r.status} />}
        />

        {offlineFallback && (
          <Notice tone="warning">
            <span>{t('offlineSnapshot', { when: fmtAgo(offlineFallback.savedAt, ft) })}</span>
          </Notice>
        )}

        {staleRefreshed && (
          <Notice tone="info">
            <span>{t('staleRefreshed')}</span>
          </Notice>
        )}

        {r.paired_ride_id && (
          <Notice tone="info" icon={<Repeat size={18} />}>
            <div className="row between wrap gap-sm">
              <span>{r.leg === 'return' ? t('pairedOutboundInfo') : t('pairedReturnInfo')}</span>
              <Link
                to={`${loc.pathname.startsWith('/admin/') ? '/admin/rides' : '/rides'}/${r.paired_ride_id}`}
                className="bold"
              >
                {r.leg === 'return' ? t('openOutboundRide') : t('openReturnRide')}
              </Link>
            </div>
          </Notice>
        )}

        {r.needs_attention && r.attention_reason && (
          <Notice tone="warning" icon={<AlertTriangle size={18} />}>
            <span>{r.attention_reason}</span>
          </Notice>
        )}

        {/* Desktop Split / Mobile Stack Layout */}
        <div className="split">
          {/* Left Column: Details, Driver, Timeline */}
          <div className="stack">
            {/* Driver / Vehicle Card */}
            <DriverCard ride={r} onChatClick={chatVisible ? scrollToChat : undefined} />

            {passengerTurn && <PassengerActions ride={r} reload={reload} part="respond" />}

            {/* Live Map (Mobile & Desktop priority when in progress) */}
            {live && <CollapsibleTrackCard ride={r} />}

            {/* Trip Details */}
            <Details ride={r} />

            {/* Vertical Status Timeline */}
            <StatusTimeline ride={r} />

            {/* Cancelling stays available, but below the details rather than beside "I'm ready". */}
            {passengerTurn && <PassengerActions ride={r} reload={reload} part="cancel" />}

            {/* Actions for Desktop / Tablet */}
            <div className="hide-mobile">
              {r.viewer.is_passenger && !passengerTurn && <PassengerActions ride={r} reload={reload} />}
              {r.viewer.is_driver && <DriverActions ride={r} reload={reload} />}
              {r.viewer.is_staff && <StaffActions ride={r} reload={reload} />}
              {!r.viewer.is_passenger && !r.viewer.is_driver && !r.viewer.is_staff && r.status === 'pending_driver' && (
                <DriverActions ride={r} reload={reload} />
              )}
            </div>
          </div>

          {/* Right Column: Chat, Help */}
          <div className="stack">
            {chatVisible && !offlineFallback && (
              <div ref={chatRef}>
                <Chat
                  rideId={r.id}
                  viewerRole={r.viewer.is_driver ? 'driver' : r.viewer.is_passenger ? 'passenger' : 'staff'}
                />
              </div>
            )}
            {(showHelpLink || showCareLink) && (
              <div className="detail-section">
                <div className="btn-row">
                  {showHelpLink && (
                    <Button variant="ghost" size="sm" to={`/help?ride=${r.id}`} icon={<LifeBuoy size={16} />}>
                      {t('needHelp')}
                    </Button>
                  )}
                  {showCareLink && (
                    <Button variant="ghost" size="sm" onClick={() => setCareModal(true)} icon={<ShieldCheck size={16} />}>
                      {t('careLinkBtn')}
                    </Button>
                  )}
                </div>
              </div>
            )}
            {careModal && <CareLinkModal ride={r} onClose={() => setCareModal(false)} />}
          </div>
        </div>

        {/* Sticky Action Bar on Phones & Landscape Phones (<=767px) */}
        <MobileStickyActions ride={r} reload={reload} passengerTurn={passengerTurn} />
      </div>
    </Page>
  );
}
