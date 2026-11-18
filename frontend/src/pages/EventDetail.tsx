import { useEffect, useRef, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import {
  Calendar,
  CarFront,
  Copy,
  Edit2,
  Maximize2,
  Minimize2,
  Plus,
  QrCode,
  RefreshCw,
  Route,
  ScanLine,
  Share2,
} from 'lucide-react';
import QRCode from 'qrcode';
import { downloadIcs } from '../calendar';
import { post, useApi } from '../api';
import { useAuth } from '../auth';
import {
  Button,
  Card,
  EmptyState,
  ErrorBox,
  KeyValue,
  KeyValueItem,
  Modal,
  Notice,
  Page,
  PageHeader,
  Pill,
  Spinner,
  StatusPill,
  useAction,
  useConfirm,
  useToast,
} from '../components/ui';
import { EVENT_KIND, fmtDateTime, RIDE_STATUS, useFormatT } from '../format';
import type { EventItem } from './Events';
import { useTranslation } from 'react-i18next';
import VisitorInvites from '../components/VisitorInvites';

interface EventFull extends EventItem {
  group_ids: number[];
  /** Groups this member may join as (best match first). */
  join_as?: { id: number; name: string }[];
  my_ride: { id: number; status: string } | null;
  settlement: { id: number; status: string } | null;
}

interface Signup {
  id: number;
  user_id: number;
  name: string;
  status: string;
  need_ride: boolean;
  offer_drive: boolean;
  offer_seats: number;
  attended: number | null;
  ride_status: string | null;
  as_group?: string | null;
}

export default function EventDetail() {
  const { t } = useTranslation('eventDetail');
  const ft = useFormatT();
  const { id } = useParams();
  const { can } = useAuth();
  // Planning the whole event's transport needs its own permission (the server checks too).
  const isStaff = can('admin_plan');
  const { data: e, error, reload } = useApi<EventFull>(id ? `/events/${id}` : null);
  const { busy, error: actErr, run } = useAction();
  const confirm = useConfirm();
  const [walkIn, setWalkIn] = useState(false);
  const [showQrModal, setShowQrModal] = useState(false);
  const [showScanModal, setShowScanModal] = useState(false);
  const [showMyPass, setShowMyPass] = useState(false);
  const [showScanMember, setShowScanMember] = useState(false);
  const [joinAs, setJoinAs] = useState<number | null>(null);
  const [joinAsMissing, setJoinAsMissing] = useState(false);
  const [showTransportPrompt, setShowTransportPrompt] = useState(false);
  const { data: signups, reload: reloadS } = useApi<Signup[]>(id ? `/events/${id}/signups` : null);

  if (error) return <ErrorBox error={error} />;
  if (!e) return <Spinner />;

  const signed = !!e.my_signup && e.my_signup.status !== 'cancelled';
  const open = e.status === 'published';
  const isFull = e.capacity > 0 && e.going >= e.capacity;
  const attendanceEnabled = e.attendance_enabled !== false;

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
    { label: t('cost'), value: e.cost_note || t('costNone') },
    { label: t('organiser'), value: e.created_by.name },
  ];

  if (e.signup_deadline) {
    kvItems.push({ label: t('deadline'), value: fmtDateTime(e.signup_deadline, ft) });
  }

  // Global traffic staff (the dashboard holders) plan any event; a cell group leader
  // plans only the events they created, matching the server's scope.
  const canPlan = isStaff && (can('admin_dashboard') || e.can_manage);

  return (
    <Page width="wide">
      <div className="stack">
        <PageHeader
          title={e.title}
          subtitle={`${fmtDateTime(e.starts_at, ft)} · ${e.location}`}
          back="/events"
          actions={
            (e.can_manage && open) || (canPlan && open) || can('events', 'create') ? (
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
                {can('events', 'create') && (
                  <Button variant="ghost" to={`/events/new?duplicate=${e.id}`} icon={<Copy size={16} />}>
                    {t('duplicate')}
                  </Button>
                )}
              </div>
            ) : undefined
          }
        />

        {e.status === 'cancelled' && (
          <div className="notice danger">{t('cancelled')}</div>
        )}

        <div className="split">
          {/* Left Column: Event Hero Details & Signup Card */}
          <div className="stack">
            {/* Event Hero Card */}
            <Card className="stack">
              {e.photo_file_id && (
                <a href={`/api/files/${e.photo_file_id}`} target="_blank" rel="noreferrer" className="event-photo-link">
                  <img src={`/api/files/${e.photo_file_id}`} alt={t('photoAlt')} className="event-photo" />
                </a>
              )}
              <span className="muted small">{ft(EVENT_KIND[e.kind]?.label ?? e.kind)}</span>

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
            </Card>

            {/* Signup Card */}
            <Card className="stack">
              <div className="row between nowrap">
                <h3>{t('mySignup')}</h3>
                <div className="row nowrap gap-xs">
                  {signed && (
                    <Pill tone={e.my_signup!.status === 'going' ? 'success' : 'warning'}>
                      {e.my_signup!.status === 'going' ? t('going') : t('waitlist')}
                    </Pill>
                  )}
                  {attendanceEnabled && signed && e.my_signup?.attended === 1 && (
                    <Pill tone="success">{t('statusAttended')}</Pill>
                  )}
                </div>
              </div>

              {signed && myGroup && <p className="small">{t('joiningAsName', { name: myGroup })}</p>}
              <p className="muted small">{t('signupNote')}</p>

              {attendanceEnabled && open && (
                <div className="card flat tight stack-xs">
                  <div className="row between nowrap">
                    <span className="bold small">{t('attendanceModule')}</span>
                    <span className="muted tiny">
                      {e.my_signup?.attended === 1 ? `✓ ${t('statusAttended')}` : t('statusNotAttended')}
                    </span>
                  </div>
                  <p className="muted tiny">{t('participateNoRide')}</p>
                  {e.my_signup?.attended !== 1 && (
                    <Button
                      variant="secondary"
                      size="sm"
                      block
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
                      block
                      icon={<QrCode size={16} />}
                      onClick={() => setShowMyPass(true)}
                    >
                      {t('myCheckinPass')}
                    </Button>
                  )}
                </div>
              )}

              {open && !signed && mustChoose && (
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

              {/* Single Clear CTA */}
              {open && !signed && (
                <Button
                  variant="primary"
                  block
                  size="lg"
                  disabled={busy}
                  onClick={handleSignup}
                >
                  {isFull ? t('joinWaitlist') : t('signUp')}
                </Button>
              )}

              {open && showTransportPrompt && going && !e.my_ride && (
                <Notice tone="info" className="stack-sm">
                  <div>
                    <strong>{t('transportPromptTitle')}</strong>
                    <div className="small muted">{t('transportPromptBody')}</div>
                  </div>
                  <div className="btn-row">
                    {can('rides', 'create') && (
                      <Button variant="primary" size="sm" to={`/rides/new?event=${e.id}`} icon={<CarFront size={16} />}>
                        {t('needRide')}
                      </Button>
                    )}
                    {can('drive', 'create') && (
                      <Button variant="secondary" size="sm" to="/drive">
                        {t('offerDrive')}
                      </Button>
                    )}
                    <Button variant="ghost" size="sm" onClick={() => setShowTransportPrompt(false)}>
                      {t('goingOwnWay')}
                    </Button>
                  </div>
                </Notice>
              )}

              {open && signed && (
                <Button
                  variant="danger"
                  block
                  disabled={busy}
                  onClick={handleWithdraw}
                >
                  {t('withdraw')}
                </Button>
              )}

              {/* Need a ride & Offer to drive triggers */}
              {open && (
                <div className="stack-sm">
                  {!e.my_ride && !going ? (
                    <p className="muted small">{t('signUpForRide')}</p>
                  ) : e.my_ride ? (
                    <Link to={`/rides/${e.my_ride.id}`} className="row between card flat tight link">
                      <span className="row nowrap gap-sm">
                        <CarFront size={16} aria-hidden="true" />
                        <span>{t('myRide')}</span>
                      </span>
                      <StatusPill map={RIDE_STATUS} status={e.my_ride.status} />
                    </Link>
                  ) : (
                    <Button variant="secondary" block to={`/rides/new?event=${e.id}`} icon={<CarFront size={18} />}>
                      {t('needRide')}
                    </Button>
                  )}

                  {can('drive', 'create') && (
                    <Button variant="ghost" block to="/drive">
                      {t('offerDrive')}
                    </Button>
                  )}
                </div>
              )}

              <ErrorBox error={actErr} />
            </Card>

            {/* Members who are going, and the organiser, can bring a visitor. */}
            {open && (going || e.can_manage) && can('rides', 'create') && <VisitorInvites event={e} />}
          </div>

          {/* Right Column: Organiser Tools / Manage Card */}
          <div className="stack">
            {e.can_manage && (
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
            )}
          </div>
        </div>

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
      </div>
    </Page>
  );
}

function AttendanceQrModal({
  event,
  onClose,
  onResetCode,
}: {
  event: EventFull;
  onClose: () => void;
  onResetCode: (newCode: string) => void;
}) {
  const { t } = useTranslation('eventDetail');
  const ft = useFormatT();
  const confirm = useConfirm();
  const { busy, run } = useAction();
  const [qrUrl, setQrUrl] = useState<string | null>(null);
  const [fullscreen, setFullscreen] = useState(false);
  const [copied, setCopied] = useState(false);

  const checkinUrl = `${window.location.origin}/events/${event.id}/checkin${
    event.checkin_code ? `?code=${encodeURIComponent(event.checkin_code)}` : ''
  }`;

  useEffect(() => {
    QRCode.toDataURL(checkinUrl, {
      margin: 1,
      width: 280,
      errorCorrectionLevel: 'M',
    })
      .then(setQrUrl)
      .catch(() => setQrUrl(null));
  }, [checkinUrl]);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(checkinUrl);
      setCopied(true);
      setTimeout(() => setCopied(false), 2500);
    } catch {
      // fallback
    }
  };

  const handleReset = async () => {
    const ok = await confirm({
      title: t('qrResetCode'),
      body: t('qrResetConfirm'),
      confirmLabel: t('qrResetCode'),
      tone: 'danger',
    });
    if (ok) {
      run(async () => {
        const res = await post<{ checkin_code: string }>(`/events/${event.id}/checkin-code/reset`);
        onResetCode(res.checkin_code);
      }, t('qrResetDone'));
    }
  };

  return (
    <Modal title={t('qrModalTitle')} onClose={onClose}>
      <div
        className="stack items-center"
        style={
          fullscreen
            ? {
                position: 'fixed',
                inset: 0,
                zIndex: 1000,
                background: 'var(--bg)',
                padding: '2rem',
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
              }
            : undefined
        }
      >
        <div className="text-center stack-sm">
          <h3>{event.title}</h3>
          <p className="muted small">
            {fmtDateTime(event.starts_at, ft)} · {event.location}
          </p>
        </div>

        {qrUrl ? (
          <div
            className="totp-qr"
            style={{
              padding: '1rem',
              borderRadius: '12px',
              border: '1px solid var(--border)',
            }}
          >
            <img src={qrUrl} width={280} height={280} alt={t('qrModalTitle')} />
          </div>
        ) : (
          <Spinner />
        )}

        <div className="text-center stack-sm">
          {event.checkin_code && (
            <p className="small">
              <span className="muted">{t('enterCodePlaceholder')}: </span>
              <strong style={{ letterSpacing: '2px', fontFamily: 'monospace', fontSize: '1.25rem' }}>
                {event.checkin_code}
              </strong>
            </p>
          )}
          <p className="muted tiny max-w-sm">{t('qrModalDesc')}</p>
        </div>

        <div className="btn-row wrap justify-center">
          <Button variant="secondary" size="sm" icon={<Copy size={16} />} onClick={handleCopy}>
            {copied ? t('qrLinkCopied') : t('qrCopyLink')}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            icon={fullscreen ? <Minimize2 size={16} /> : <Maximize2 size={16} />}
            onClick={() => setFullscreen(!fullscreen)}
          >
            {fullscreen ? t('qrExitFullscreen') : t('qrFullscreen')}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            icon={<RefreshCw size={16} />}
            disabled={busy}
            onClick={handleReset}
          >
            {t('qrResetCode')}
          </Button>
        </div>
      </div>
    </Modal>
  );
}

function EventScannerModal({
  event,
  onClose,
  onSuccess,
}: {
  event: EventFull;
  onClose: () => void;
  onSuccess: () => void;
}) {
  const { t } = useTranslation('eventDetail');
  const [code, setCode] = useState('');
  const { busy, error, run } = useAction();
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const [cameraActive, setCameraActive] = useState(false);

  const extractCode = (input: string) => {
    try {
      if (input.includes('code=')) {
        const u = new URL(input, window.location.origin);
        return u.searchParams.get('code') || input;
      }
    } catch {
      // not a full url
    }
    return input.trim();
  };

  const handleCheckin = (codeToUse: string) => {
    const clean = extractCode(codeToUse);
    run(async () => {
      await post(`/events/${event.id}/checkin`, { code: clean });
      onSuccess();
    }, t('checkinSuccess'));
  };
  const checkinRef = useRef(handleCheckin);
  checkinRef.current = handleCheckin;

  useEffect(() => {
    let cancelled = false;
    let stream: MediaStream | null = null;
    let timer: number | null = null;

    if (navigator.mediaDevices && navigator.mediaDevices.getUserMedia) {
      navigator.mediaDevices
        .getUserMedia({ video: { facingMode: 'environment' } })
        .then((s) => {
          if (cancelled) {
            s.getTracks().forEach((track) => track.stop());
            return;
          }
          stream = s;
          if (videoRef.current) {
            videoRef.current.srcObject = s;
            videoRef.current.play().catch(() => {});
            setCameraActive(true);
          }
          if ('BarcodeDetector' in window) {
            // @ts-expect-error BarcodeDetector browser standard
            const detector = new window.BarcodeDetector({ formats: ['qr_code'] });
            const scanLoop = async () => {
              if (cancelled) return;
              if (
                videoRef.current &&
                videoRef.current.readyState === videoRef.current.HAVE_ENOUGH_DATA
              ) {
                try {
                  const barcodes = await detector.detect(videoRef.current);
                  if (barcodes.length > 0) {
                    const raw = barcodes[0].rawValue;
                    checkinRef.current(raw);
                    return;
                  }
                } catch {
                  // ignore
                }
              }
              if (!cancelled) timer = window.setTimeout(scanLoop, 250);
            };
            timer = window.setTimeout(scanLoop, 500);
          }
        })
        .catch(() => {
          if (!cancelled) setCameraActive(false);
        });
    }

    return () => {
      cancelled = true;
      if (timer) clearTimeout(timer);
      if (stream) {
        stream.getTracks().forEach((track) => track.stop());
      }
    };
  }, []);

  return (
    <Modal title={t('scanQrModalTitle')} onClose={onClose}>
      <div className="stack">
        <p className="muted small">{t('scanQrModalDesc')}</p>

        {cameraActive && (
          <div
            style={{
              position: 'relative',
              width: '100%',
              maxWidth: '320px',
              margin: '0 auto',
              overflow: 'hidden',
              borderRadius: '12px',
              background: 'var(--surface-2)',
            }}
          >
            <video
              ref={videoRef}
              style={{ width: '100%', height: '240px', objectFit: 'cover' }}
              muted
              playsInline
            />
          </div>
        )}

        <form
          className="stack-sm"
          onSubmit={(e) => {
            e.preventDefault();
            if (code.trim()) handleCheckin(code);
          }}
        >
          <input
            className="input"
            type="text"
            placeholder={t('enterCodePlaceholder')}
            value={code}
            onChange={(e) => setCode(e.target.value)}
            disabled={busy}
            autoFocus={!cameraActive}
          />
          <Button variant="primary" block type="submit" loading={busy} disabled={!code.trim()}>
            {t('submitCheckin')}
          </Button>
        </form>

        <p className="muted tiny">{t('participateNoRide')}</p>
        <ErrorBox error={error} />
      </div>
    </Modal>
  );
}

function WalkIn({ onClose, onPick }: { onClose: () => void; onPick: (uid: number) => void }) {
  const { t } = useTranslation('eventDetail');
  const [q, setQ] = useState('');
  const { data } = useApi<{ id: number; name: string }[]>(
    q.length >= 1 ? `/admin/users?q=${encodeURIComponent(q)}` : null,
  );

  return (
    <Modal title={t('walkInTitle')} onClose={onClose}>
      <input
        className="input"
        placeholder={t('walkInPlaceholder')}
        value={q}
        onChange={(e) => setQ(e.target.value)}
      />
      <div className="list">
        {data?.slice(0, 10).map((u) => (
          <button
            key={u.id}
            type="button"
            className="item btn ghost text-left"
            onClick={() => onPick(u.id)}
          >
            {u.name}
          </button>
        ))}
      </div>
    </Modal>
  );
}

function MemberPassQrModal({
  event,
  passCode,
  onClose,
}: {
  event: EventFull;
  passCode: string;
  onClose: () => void;
}) {
  const { t } = useTranslation('eventDetail');
  const [qrUrl, setQrUrl] = useState<string | null>(null);
  const payload = `GRPASS:${event.id}:${passCode}`;

  useEffect(() => {
    QRCode.toDataURL(payload, { margin: 1, width: 240, errorCorrectionLevel: 'M' })
      .then(setQrUrl)
      .catch(() => setQrUrl(null));
  }, [payload]);

  return (
    <Modal title={t('myCheckinPass')} onClose={onClose}>
      <div className="stack items-center" style={{ padding: 'var(--space-2) 0' }}>
        {qrUrl && (
          <div className="totp-qr" style={{ padding: 'var(--space-3)', background: 'var(--surface)', borderRadius: 'var(--radius)' }}>
            <img src={qrUrl} width={240} height={240} alt={t('myCheckinPass')} />
          </div>
        )}
        <code className="card flat tight mono">{passCode}</code>
        <p className="muted tiny center max-w-sm">{t('myCheckinPassDesc')}</p>
      </div>
    </Modal>
  );
}

function ScanMemberPassModal({
  eventId,
  onClose,
  onSuccess,
}: {
  eventId: number;
  onClose: () => void;
  onSuccess: () => void;
}) {
  const { t } = useTranslation('eventDetail');
  const toast = useToast();
  const [rawCode, setRawCode] = useState('');
  const [scanError, setScanError] = useState<string | null>(null);
  const { busy, error, run } = useAction();
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const [cameraOn, setCameraOn] = useState(false);
  const hasBarcodeDetector = typeof window !== 'undefined' && 'BarcodeDetector' in window;

  const stopCamera = () => {
    if (streamRef.current) {
      streamRef.current.getTracks().forEach((tr) => tr.stop());
      streamRef.current = null;
    }
    setCameraOn(false);
  };

  useEffect(() => () => stopCamera(), []);

  const submitCode = async (input: string): Promise<boolean> => {
    const trimmed = input.trim();
    const parts = trimmed.split(':');
    if (parts[0] === 'GRPASS') {
      if (parts.length !== 3 || parts[1] !== String(eventId) || !parts[2].trim()) {
        setScanError(t('events.INVALID_PASS_CODE', { ns: 'errors' }));
        return false;
      }
    }
    const code = parts.length === 3 && parts[0] === 'GRPASS' ? parts[2].trim() : trimmed;
    if (!code) return false;
    setScanError(null);
    return run(async () => {
      const res = await post<{ ok: boolean; user: { id: number; name: string } }>(
        `/events/${eventId}/checkin/scan-member`,
        { pass_code: code },
      );
      stopCamera();
      toast.success(t('memberCheckedIn', { name: res.user?.name ?? '' }));
      onSuccess();
      onClose();
    });
  };
  const submitCodeRef = useRef(submitCode);
  submitCodeRef.current = submitCode;

  useEffect(() => {
    if (!cameraOn) return;
    let cancelled = false;
    let raf = 0;
    let retryTimer = 0;
    const start = async () => {
      try {
        const stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: 'environment' } });
        if (cancelled) {
          stream.getTracks().forEach((tr) => tr.stop());
          return;
        }
        streamRef.current = stream;
        if (videoRef.current) {
          videoRef.current.srcObject = stream;
          await videoRef.current.play();
        }
        if (cancelled) {
          stream.getTracks().forEach((tr) => tr.stop());
          streamRef.current = null;
          return;
        }
        const Detector = (
          window as Window & {
            BarcodeDetector?: new (opts?: { formats?: string[] }) => {
              detect(source: HTMLVideoElement): Promise<{ rawValue?: string }[]>;
            };
          }
        ).BarcodeDetector;
        if (Detector) {
          const detector = new Detector({ formats: ['qr_code'] });
          const scan = async () => {
            if (cancelled || !videoRef.current) return;
            try {
              const codes = await detector.detect(videoRef.current);
              if (codes.length > 0 && codes[0].rawValue) {
                const ok = await submitCodeRef.current(codes[0].rawValue);
                if (ok || cancelled) return;
                retryTimer = window.setTimeout(() => {
                  if (!cancelled) raf = requestAnimationFrame(scan);
                }, 1500);
                return;
              }
            } catch {
              // skip frame
            }
            if (!cancelled) raf = requestAnimationFrame(scan);
          };
          raf = requestAnimationFrame(scan);
        }
      } catch {
        if (!cancelled) setCameraOn(false);
      }
    };
    void start();
    return () => {
      cancelled = true;
      cancelAnimationFrame(raf);
      if (retryTimer) window.clearTimeout(retryTimer);
      stopCamera();
    };
  }, [cameraOn]);

  return (
    <Modal title={t('scanMemberPass')} onClose={onClose}>
      <div className="stack">
        <p className="muted small">{t('scanMemberPassDesc')}</p>
        {cameraOn && (
          <div style={{ borderRadius: 'var(--radius)', overflow: 'hidden', background: 'var(--surface-2)' }}>
            <video ref={videoRef} playsInline muted style={{ width: '100%', maxHeight: 240, display: 'block', objectFit: 'cover' }} />
          </div>
        )}
        {cameraOn && !hasBarcodeDetector && (
          <Notice tone="info">{t('scanMemberPassDesc')}</Notice>
        )}
        <div className="btn-row">
          <Button variant="secondary" size="sm" onClick={() => setCameraOn((v) => !v)} icon={<QrCode size={16} />}>
            {t('scanQrCheckin')}
          </Button>
        </div>
        <div className="row nowrap gap-sm">
          <input
            className="input grow mono"
            value={rawCode}
            onChange={(e) => setRawCode(e.target.value)}
            placeholder={t('memberPassPlaceholder')}
            onKeyDown={(e) => e.key === 'Enter' && void submitCode(rawCode)}
          />
          <Button variant="primary" disabled={busy || !rawCode.trim()} onClick={() => void submitCode(rawCode)}>
            {t('submitCheckin')}
          </Button>
        </div>
        <ErrorBox error={scanError || error} />
      </div>
    </Modal>
  );
}
