import { useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import {
  Calendar,
  CarFront,
  Edit2,
  Plus,
  Route,
  Share2,
} from 'lucide-react';
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
  Page,
  PageHeader,
  Pill,
  Spinner,
  StatusPill,
  useAction,
  useConfirm,
} from '../components/ui';
import { EVENT_KIND, fmtDateTime, RIDE_STATUS, useFormatT } from '../format';
import type { EventItem } from './Events';
import { useTranslation } from 'react-i18next';

interface EventFull extends EventItem {
  group_ids: number[];
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
  const { data: signups, reload: reloadS } = useApi<Signup[]>(id ? `/events/${id}/signups` : null);

  if (error) return <ErrorBox error={error} />;
  if (!e) return <Spinner />;

  const signed = !!e.my_signup && e.my_signup.status !== 'cancelled';
  const open = e.status === 'published';
  const isFull = e.capacity > 0 && e.going >= e.capacity;

  const active = signups?.filter((s) => s.status !== 'cancelled') ?? [];
  const signupsAttended = active.filter((s) => s.attended === 1).length;
  const mark = (uid: number, attended: boolean | null) =>
    run(() => post(`/events/${e.id}/attendance`, { user_id: uid, attended }).then(reloadS));

  const handleSignup = () => {
    run(async () => {
      await post(`/events/${e.id}/signup`, {});
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

  return (
    <Page width="wide">
      <div className="stack">
        <PageHeader
          title={e.title}
          subtitle={`${fmtDateTime(e.starts_at, ft)} · ${e.location}`}
          back="/events"
          actions={
            (e.can_manage && open) || (isStaff && open) ? (
              <div className="btn-row">
                {isStaff && (
                  <Button variant="primary" to={`/admin/events/${e.id}/plan`} icon={<Route size={16} />}>
                    {t('open', { ns: 'plan' })}
                  </Button>
                )}
                {e.can_manage && (
                  <Button variant="secondary" to={`/events/${e.id}/edit`} icon={<Edit2 size={16} />}>
                    {t('edit')}
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
                {signed && (
                  <Pill tone={e.my_signup!.status === 'going' ? 'success' : 'warning'}>
                    {e.my_signup!.status === 'going' ? t('going') : t('waitlist')}
                  </Pill>
                )}
              </div>

              <p className="muted small">{t('signupNote')}</p>

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
                  {e.my_ride ? (
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
                  <span className="muted small">
                    {t('attendance', { attended: signupsAttended, total: active.length })}
                  </span>
                </div>

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
                          </div>
                        </div>
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
                      </div>
                    ))}
                  </div>
                )}

                <div className="btn-row">
                  <Button variant="ghost" size="sm" onClick={() => setWalkIn(true)} icon={<Plus size={16} />}>
                    {t('walkIn')}
                  </Button>
                </div>
                <p className="muted tiny">{t('walkInNote')}</p>

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

        {walkIn && (
          <WalkIn
            onClose={() => setWalkIn(false)}
            onPick={(uid) => mark(uid, true).then(() => setWalkIn(false))}
          />
        )}
      </div>
    </Page>
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
