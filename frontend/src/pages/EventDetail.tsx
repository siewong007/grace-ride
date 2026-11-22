import { useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import {
  Calendar,
  Car,
  CarFront,
  Check,
  ChevronRight,
  Copy,
  Edit2,
  Plus,
  QrCode,
  Route,
  ScanLine,
  Share2,
  Users,
} from 'lucide-react';
import { downloadIcs } from '../calendar';
import { post, useApi } from '../api';
import { useAuth } from '../auth';
import type { Ride } from '../components/RideCard';
import {
  Button,
  Card,
  EmptyState,
  ErrorBox,
  KeyValue,
  KeyValueItem,
  Notice,
  Page,
  PageHeader,
  Pill,
  Spinner,
  StatusPill,
  useAction,
  useConfirm,
} from '../components/ui';
import { EVENT_KIND, fmtDateTime, fmtTime, RIDE_STATUS, RIDE_STATUS_PASSENGER, useFormatT } from '../format';
import { useTranslation } from 'react-i18next';
import VisitorInvites from '../components/VisitorInvites';
import { DriveRequests } from '../components/drive/DriveRequests';
import type { EventFull, Signup } from '../components/event/types';
import {
  AttendanceQrModal,
  MemberPassQrModal,
  WalkIn,
} from '../components/event/EventQrModals';
import {
  EventScannerModal,
  ScanMemberPassModal,
} from '../components/event/EventScannerModals';
import PostEventReimbursementModal from '../components/events/PostEventReimbursementModal';

export default function EventDetail() {
  const { t } = useTranslation('eventDetail');
  const ft = useFormatT();
  const { id } = useParams();
  const { can, me } = useAuth();
  // Planning the whole event's transport needs its own permission (the server checks too).
  const isStaff = can('admin_plan');
  const { data: e, error, reload } = useApi<EventFull>(id ? `/events/${id}` : null);
  const { busy, error: actErr, run } = useAction();
  const confirm = useConfirm();
  const [showPostEventReimb, setShowPostEventReimb] = useState(false);
  const [walkIn, setWalkIn] = useState(false);
  const [showQrModal, setShowQrModal] = useState(false);
  const [showScanModal, setShowScanModal] = useState(false);
  const [showMyPass, setShowMyPass] = useState(false);
  const [showScanMember, setShowScanMember] = useState(false);
  const [joinAs, setJoinAs] = useState<number | null>(null);
  const [joinAsMissing, setJoinAsMissing] = useState(false);
  const [showTransportPrompt, setShowTransportPrompt] = useState(false);
  const [showDriveRequests, setShowDriveRequests] = useState(false);
  const { data: signups, reload: reloadS } = useApi<Signup[]>(id ? `/events/${id}/signups` : null);
  const eventDay = e?.starts_at ? e.starts_at.slice(0, 10) : '';
  const shouldLoadDriverDay = Boolean(
    e && can('drive') && e.my_signup?.status === 'going' && eventDay,
  );
  const myDrivingDay = useApi<Ride[]>(
    shouldLoadDriverDay ? `/driver/schedule?from=${eventDay}&to=${eventDay}` : null,
  );

  if (error) return <Page width="medium"><ErrorBox error={error} onRetry={reload} /></Page>;
  if (!e) return <Spinner />;

  const signed = !!e.my_signup && e.my_signup.status !== 'cancelled';
  const open = e.status === 'published';
  const isFull = e.capacity > 0 && e.going >= e.capacity;
  const attendanceEnabled = e.attendance_enabled !== false;
  const myDrivingForEvent = (myDrivingDay.data ?? []).filter((r) => r.event?.id === e.id);

  const active = Array.isArray(signups) ? signups.filter((s) => s.status !== 'cancelled') : [];
  const signupsAttended = active.filter((s) => s.attended === 1).length;
  const mark = (uid: number, attended: boolean | null) =>
    run(() => post(`/events/${e.id}/attendance`, { user_id: uid, attended }).then(reloadS));

  // Members in more than one of the event's groups say which one they come as.
  const choices = e.join_as ?? [];
  const mustChoose = choices.length > 1;
  const myGroup = (e.my_signup as { as_group?: { name: string } | null } | null)?.as_group?.name;
  const going = signed && e.my_signup!.status === 'going';

  const handleSignup = () => {
    if (mustChoose && joinAs == null) {
      setJoinAsMissing(true);
      return;
    }
    run(async () => {
      await post(`/events/${e.id}/signup`, mustChoose ? { as_group_id: joinAs } : {});
      setShowTransportPrompt(true);
      reload();
    }, t('signedUpSuccess'));
  };

  const handleWithdraw = async () => {
    const ok = await confirm({
      title: t('withdraw'),
      body: t('cancelConfirm'),
      confirmLabel: t('withdraw'),
      tone: 'danger',
    });
    if (ok) {
      run(() => post(`/events/${e.id}/withdraw`).then(reload), t('withdrawn'));
    }
  };

  const kvItems: KeyValueItem[] = [
    { label: t('groups'), value: e.groups },
    {
      label: t('quota'),
      value: e.capacity
        ? `${e.going}/${e.capacity}${e.waitlist > 0 ? ` · ${t('waitlist')}: ${e.waitlist}` : ''}`
        : t('quotaUnlimited', { going: e.going }),
    },
    ...(e.cost_note ? [{ label: t('cost'), value: e.cost_note }] : []),
    ...(e.mileage_claimable || can('drive') || e.can_manage
      ? [{
          label: t('mileage'),
          value: e.mileage_claimable ? t('mileageClaimable') : t('mileageNotClaimable'),
        }]
      : []),
    { label: t('organiser'), value: e.created_by.name },
  ];

  if (e.signup_deadline) {
    kvItems.push({ label: t('deadline'), value: fmtDateTime(e.signup_deadline, ft) });
  }

  // Global traffic staff (the dashboard holders) plan any event; a cell group leader
  // plans only the events they created, matching the server's scope.
  const canPlan = isStaff && (can('admin_dashboard') || e.can_manage);

  const primaryAndEditActions =
    (e.can_manage && open) || (canPlan && open) ? (
      <div className="btn-row">
        {canPlan && open && (
          <Button variant="primary" to={`/admin/events/${e.id}/plan`} icon={<Route size={16} />}>
            {t('open', { ns: 'plan' })}
          </Button>
        )}
        {e.can_manage && open && (
          <Button variant="secondary" to={`/events/${e.id}/edit`} icon={<Edit2 size={16} />}>
            {t('edit')}
          </Button>
        )}
      </div>
    ) : undefined;

  const mainContent = (
    <div className="stack">
      {e.photo_file_id && (
        <a href={`/api/files/${e.photo_file_id}`} target="_blank" rel="noreferrer" className="event-photo-link">
          <img src={`/api/files/${e.photo_file_id}`} alt={t('photoAlt')} className="event-photo" />
        </a>
      )}

      {/* 1. Primary Participation & Transportation Focus Surface */}
      <section className="focus-surface" aria-label={t('mySignup')}>
        <div className="row between nowrap">
          <div className="row wrap gap-xs items-center">
            <h2 className="home-focus-eyebrow">{t('mySignup')}</h2>
            {signed && (
              <Pill tone={e.my_signup!.status === 'going' ? 'success' : 'warning'}>
                {e.my_signup!.status === 'going' ? t('going') : t('waitlist')}
              </Pill>
            )}
            {attendanceEnabled && signed && e.my_signup?.attended === 1 && (
              <Pill tone="success">{t('statusAttended')}</Pill>
            )}
          </div>
          {open && signed && (
            <Button
              variant="ghost"
              size="sm"
              className="quiet-danger"
              disabled={busy}
              onClick={handleWithdraw}
            >
              {t('withdraw')}
            </Button>
          )}
        </div>

        {signed && myGroup && <p className="small muted">{t('joiningAsName', { name: myGroup })}</p>}

        {open && !signed && (
          <>
            {mustChoose && (
              <fieldset className="join-as stack-sm">
                <legend className="bold small">{t('joinAsLegend')}</legend>
                <div className="chips">
                  {choices.map((g) => (
                    <label key={g.id} className={`chip ${joinAs === g.id ? 'on' : ''}`}>
                      <input
                        type="radio"
                        name="join-as"
                        className="sr-only"
                        checked={joinAs === g.id}
                        onChange={() => {
                          setJoinAs(g.id);
                          setJoinAsMissing(false);
                        }}
                      />
                      {g.name}
                    </label>
                  ))}
                </div>
                {joinAsMissing && <span className="field-error" role="alert">{t('joinAsRequired')}</span>}
              </fieldset>
            )}
            <Button
              variant="primary"
              block
              size="lg"
              disabled={busy}
              onClick={handleSignup}
            >
              {isFull ? t('joinWaitlist') : t('signUp')}
            </Button>
            <p className="muted small">{t('signUpForRide')}</p>
          </>
        )}

        {/* Contextual Transportation (only once going) */}
        {open && going && (
          <div className="detail-section">
            <div className="row between nowrap">
              <span className="small bold">{t('transportation')}</span>
            </div>

            {showTransportPrompt && !e.my_ride && myDrivingForEvent.length === 0 && (
              <Notice tone="info" className="stack-sm">
                <div>
                  <strong>{t('transportPromptTitle')}</strong>
                  <div className="small muted">{t('transportPromptBody')}</div>
                </div>
              </Notice>
            )}

            {e.my_ride ? (
              <div className="stack-xs">
                <div className="list">
                  <Link to={`/rides/${e.my_ride.id}`} className="item">
                    <CarFront size={16} aria-hidden="true" className="muted" />
                    <span className="grow bold">{t('myRide')}</span>
                    <StatusPill map={RIDE_STATUS_PASSENGER} status={e.my_ride.status} />
                    <ChevronRight size={16} className="muted" aria-hidden="true" />
                  </Link>
                  {e.my_return_ride && e.my_return_ride.id !== e.my_ride.id && (
                    <Link to={`/rides/${e.my_return_ride.id}`} className="item">
                      <CarFront size={16} aria-hidden="true" className="muted" />
                      <span className="grow bold">{t('yourReturnRide')}</span>
                      <StatusPill map={RIDE_STATUS_PASSENGER} status={e.my_return_ride.status} />
                      <ChevronRight size={16} className="muted" aria-hidden="true" />
                    </Link>
                  )}
                </div>
                {can('rides', 'create') && (
                  <div className="btn-row">
                    <Button variant="ghost" size="sm" to={`/rides/new?event=${e.id}&proxy=1`} icon={<Users size={16} />}>
                      {t('bookForFamily')}
                    </Button>
                  </div>
                )}
              </div>
            ) : myDrivingForEvent.length > 0 ? (
              <div className="stack-xs">
                <div className="small muted">{t('youAreDriving')}</div>
                <div className="list">
                  {myDrivingForEvent.map((r) => (
                    <Link key={r.id} to={`/rides/${r.id}`} className="item">
                      <span className="timeline-time">{fmtTime(r.pickup_at, ft).replace(/^.. /, '')}</span>
                      <div className="grow min-w-0">
                        <div className="bold">{r.is_proxy && r.proxy ? r.proxy.rider_name : r.passenger.name}</div>
                        <div className="muted small">{r.area.name} → {r.destination}</div>
                      </div>
                      <StatusPill map={RIDE_STATUS} status={r.status} />
                      <ChevronRight size={16} className="muted" aria-hidden="true" />
                    </Link>
                  ))}
                </div>
                <div>
                  <Button variant="secondary" size="sm" to="/drive">
                    {t('openItinerary')}
                  </Button>
                </div>
              </div>
            ) : (
              <div className="btn-row">
                {can('rides', 'create') && (
                  <Button variant="primary" to={`/rides/new?event=${e.id}`} icon={<CarFront size={18} />}>
                    {t('needRide')}
                  </Button>
                )}
                {can('drive', 'create') && (
                  <Button
                    variant="secondary"
                    onClick={() => {
                      setShowDriveRequests((v) => !v);
                      setShowTransportPrompt(false);
                    }}
                  >
                    {showDriveRequests ? t('hideDriveRequests') : t('offerDrive')}
                  </Button>
                )}
                {showTransportPrompt && (
                  <Button variant="ghost" size="sm" onClick={() => setShowTransportPrompt(false)}>
                    {t('goingOwnWay')}
                  </Button>
                )}
              </div>
            )}

            {(e.my_ride || myDrivingForEvent.length > 0) && can('drive', 'create') && (
              <div>
                <Button variant="ghost" size="sm" onClick={() => setShowDriveRequests((v) => !v)}>
                  {showDriveRequests ? t('hideDriveRequests') : t('offerDrive')}
                </Button>
              </div>
            )}

            {!e.my_ride && myDrivingForEvent.length === 0 && (
              <p className="muted small">{t('signupNote')}</p>
            )}
          </div>
        )}

        {/* Attendance Check-in (hairline-divided inside the same surface — no nested card) */}
        {attendanceEnabled && open && (
          <div className="detail-section stack-xs">
            <div className="row between nowrap">
              <span className="bold small">{t('attendanceModule')}</span>
              <span className="muted small row nowrap gap-xs">
                {e.my_signup?.attended === 1 && <Check size={14} aria-hidden="true" />}
                <span>{e.my_signup?.attended === 1 ? t('statusAttended') : t('statusNotAttended')}</span>
              </span>
            </div>
            <p className="muted small">{t('participateNoRide')}</p>
            {(e.my_signup?.attended !== 1 || (signed && e.my_signup?.pass_code)) && (
              <div className="btn-row">
                {e.my_signup?.attended !== 1 && (
                  <Button
                    variant="secondary"
                    size="sm"
                    icon={<QrCode size={16} />}
                    onClick={() => setShowScanModal(true)}
                  >
                    {t('scanToAttend')}
                  </Button>
                )}
                {signed && e.my_signup?.pass_code && (
                  <Button
                    variant="ghost"
                    size="sm"
                    icon={<QrCode size={16} />}
                    onClick={() => setShowMyPass(true)}
                  >
                    {t('myCheckinPass')}
                  </Button>
                )}
              </div>
            )}
          </div>
        )}

        <ErrorBox error={actErr} />
      </section>

      {/* Inline Ride Requests for Volunteer Drivers */}
      {open && going && can('drive', 'create') && showDriveRequests && (
        <section className="detail-section stack">
          <div className="row between">
            <h3>{t('driveRequestsTitle')}</h3>
            <Link to="/drive?tab=settings" className="small">{t('driverSetup')}</Link>
          </div>
          <p className="muted small">{t('driveRequestsHint')}</p>
          <DriveRequests eventId={e.id} />
        </section>
      )}

      {/* 2. Event Details (Open typography & hairline rules — no outer card box) */}
      <section className="detail-section stack">
        <div className="row between nowrap">
          <h3>{t('aboutEvent')}</h3>
          <span className="muted small">{ft(EVENT_KIND[e.kind]?.label ?? e.kind)}</span>
        </div>

        {e.description && (
          <p className="pre-wrap">{e.description}</p>
        )}

        <KeyValue items={kvItems} />

        <div className="btn-row">
          <Button
            variant="ghost"
            size="sm"
            icon={<Calendar size={16} />}
            onClick={() =>
              downloadIcs(
                {
                  title: e.title,
                  description: e.description,
                  location: e.location,
                  startsAt: e.starts_at,
                },
                `${e.title}.ics`,
              )
            }
          >
            {t('addToCalendar')}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            icon={<Share2 size={16} />}
            onClick={() => {
              const text = t('shareEventText', {
                title: e.title,
                time: fmtDateTime(e.starts_at, ft),
                location: e.location,
              });
              window.open(`https://wa.me/?text=${encodeURIComponent(text)}`, '_blank', 'noopener,noreferrer');
            }}
          >
            {t('shareWhatsApp')}
          </Button>
        </div>
      </section>

      {/* 3. Bringing a visitor */}
      {open && (going || e.can_manage) && can('rides', 'create') && <VisitorInvites event={e} />}
    </div>
  );

  const managePanel = e.can_manage ? (
    <Card className="stack">
      <div className="row between nowrap">
        <div>
          <h3>{t('manageTitle')}</h3>
          <p className="muted tiny">{t('manageDesc')}</p>
        </div>
        {attendanceEnabled && (
          <span className="muted small">
            {t('attendance', { attended: signupsAttended, total: active.length })}
          </span>
        )}
      </div>

      {attendanceEnabled && (
        <div className="btn-row">
          <Button
            variant="secondary"
            size="sm"
            icon={<QrCode size={16} />}
            onClick={() => setShowQrModal(true)}
          >
            {t('attendanceQr')}
          </Button>
        </div>
      )}

      {!active.length ? (
        <EmptyState title={t('noSignups')} />
      ) : (
        <div className="list">
          {active.map((s) => (
            <div key={s.id} className="item">
              <div className="grow min-w-0">
                <div className="bold">
                  {s.name}{' '}
                  {s.status === 'waitlist' && <Pill tone="warning">{t('waitlist')}</Pill>}
                </div>
                <div className="muted tiny">
                  {s.need_ride && (
                    s.ride_status
                      ? t('needsRideWithStatus', { status: ft(`format:rideStatus.${s.ride_status}`) })
                      : t('needsRide')
                  )}
                  {s.need_ride && s.offer_drive ? ' ' : ''}
                  {s.offer_drive && t('offersSeats', { n: s.offer_seats })}
                  {s.as_group && <>{(s.need_ride || s.offer_drive) ? ' · ' : ''}{t('joiningAsName', { name: s.as_group })}</>}
                </div>
              </div>
              {attendanceEnabled && (
                <div className="chips">
                  <button
                    type="button"
                    className={`chip ${s.attended === 1 ? 'on' : ''}`}
                    disabled={busy}
                    onClick={() => mark(s.user_id, s.attended === 1 ? null : true)}
                  >
                    {t('attending')}
                  </button>
                  <button
                    type="button"
                    className={`chip ${s.attended === 0 ? 'on' : ''}`}
                    disabled={busy}
                    onClick={() => mark(s.user_id, s.attended === 0 ? null : false)}
                  >
                    {t('absent')}
                  </button>
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {attendanceEnabled && (
        <>
          <div className="btn-row">
            <Button variant="ghost" size="sm" onClick={() => setWalkIn(true)} icon={<Plus size={16} />}>
              {t('walkIn')}
            </Button>
            <Button variant="secondary" size="sm" onClick={() => setShowScanMember(true)} icon={<ScanLine size={16} />}>
              {t('scanMemberPass')}
            </Button>
          </div>
          <p className="muted tiny">{t('walkInNote')}</p>
        </>
      )}

      {/* Event Actions & Settlements */}
      <div className="divider" />
      <h4>{t('afterEvent')}</h4>
      {((e.created_by?.id === me?.id && can('events', 'update')) || can('admin_events') || can('admin_plan') || can('admin_rides')) && (
        <Button
          variant="secondary"
          block
          icon={<Car size={16} />}
          onClick={() => setShowPostEventReimb(true)}
        >
          {t('postEventReimbBtn')}
        </Button>
      )}
      {e.settlement ? (
        <Button variant="secondary" block to={`/settlements/${e.settlement.id}`}>
          {t('viewSettlement')}
        </Button>
      ) : (
        can('settlements', 'create') && (
          <Button variant="primary" block to={`/events/${e.id}/settle`}>
            {t('createSettlement')}
          </Button>
        )
      )}

      {can('events', 'create') && (
        <Button variant="ghost" block to={`/events/new?duplicate=${e.id}`} icon={<Copy size={16} />}>
          {t('duplicate')}
        </Button>
      )}

      {open && (
        <div className="btn-row">
          <Button
            variant="secondary"
            disabled={busy}
            onClick={() => run(() => post(`/events/${e.id}/finish`).then(reload), t('finishedDone'))}
          >
            {t('finish')}
          </Button>
          {(e.can_cancel ?? true) && <Button
            variant="danger"
            disabled={busy}
            onClick={async () => {
              const ok = await confirm({
                title: t('cancelEvent'),
                body: t('cancelConfirm'),
                confirmLabel: t('cancelEvent'),
                tone: 'danger',
              });
              if (ok) run(() => post(`/events/${e.id}/cancel`).then(reload), t('cancelledDone'));
            }}
          >
            {t('cancelEvent')}
          </Button>}
        </div>
      )}
    </Card>
  ) : null;

  return (
    <Page width={e.can_manage ? 'wide' : 'medium'}>
      <div className="stack">
        <PageHeader
          title={e.title}
          subtitle={`${fmtDateTime(e.starts_at, ft)} · ${e.location}`}
          back="/events"
          actions={primaryAndEditActions}
        />

        {e.status === 'cancelled' && (
          <div className="notice danger">{t('cancelled')}</div>
        )}

        {e.can_manage ? (
          <div className="split">
            {mainContent}
            <div className="stack">{managePanel}</div>
          </div>
        ) : (
          mainContent
        )}

        {showQrModal && (
          <AttendanceQrModal
            event={e}
            onClose={() => setShowQrModal(false)}
            onResetCode={() => reload()}
          />
        )}

        {showScanModal && (
          <EventScannerModal
            event={e}
            onClose={() => setShowScanModal(false)}
            onSuccess={() => {
              setShowScanModal(false);
              reload();
            }}
          />
        )}

        {walkIn && (
          <WalkIn
            onClose={() => setWalkIn(false)}
            onPick={(uid) => mark(uid, true).then(() => setWalkIn(false))}
          />
        )}

        {showMyPass && e.my_signup?.pass_code && (
          <MemberPassQrModal
            event={e}
            passCode={e.my_signup.pass_code}
            onClose={() => setShowMyPass(false)}
          />
        )}

        {showScanMember && (
          <ScanMemberPassModal
            eventId={e.id}
            onClose={() => setShowScanMember(false)}
            onSuccess={() => reloadS()}
          />
        )}

        {showPostEventReimb && (
          <PostEventReimbursementModal
            eventId={e.id}
            onClose={() => setShowPostEventReimb(false)}
            onSuccess={() => reload()}
          />
        )}
      </div>
    </Page>
  );
}
