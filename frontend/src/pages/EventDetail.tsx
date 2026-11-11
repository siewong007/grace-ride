import { useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import { post, useApi } from '../api';
import { useAuth } from '../auth';
import { Empty, ErrorBox, Modal, Pill, Spinner, StatusPill, useAction, useConfirm } from '../components/ui';
import { EVENT_KIND, fmtDateTime, RIDE_STATUS, useFormatT } from '../format';
import type { EventItem } from './Events';
import { useTranslation } from 'react-i18next';

interface EventFull extends EventItem {
  group_ids: number[];
  my_ride: { id: number; status: string } | null;
  settlement: { id: number; status: string } | null;
}
interface Signup { id: number; user_id: number; name: string; status: string; need_ride: boolean; offer_drive: boolean; offer_seats: number; attended: number | null; ride_status: string | null }

export default function EventDetail() {
  const { t } = useTranslation('eventDetail');
  const ft = useFormatT();
  const { id } = useParams();
  const { has } = useAuth();
  const { data: e, error, reload } = useApi<EventFull>(id ? `/events/${id}` : null);
  const { busy, error: actErr, run } = useAction();
  const confirm = useConfirm();
  const [walkIn, setWalkIn] = useState(false);
  const { data: signups, reload: reloadS } = useApi<Signup[]>(id ? `/events/${id}/signups` : null);

  if (error) return <ErrorBox error={error} />;
  if (!e) return <Spinner />;
  const signed = e.my_signup && e.my_signup.status !== 'cancelled';
  const open = e.status === 'published';

  const active = signups?.filter((s) => s.status !== 'cancelled') ?? [];
  const signupsAttended = active.filter((s) => s.attended === 1).length;
  const mark = (uid: number, attended: boolean | null) => run(() => post(`/events/${e.id}/attendance`, { user_id: uid, attended }).then(reloadS));

  return (
    <div className="stack">
      <div className="page-head">
        <div className="row nowrap">
          <span className="event-emoji" aria-hidden>{EVENT_KIND[e.kind]?.emoji}</span>
          <div>
            <h1>{e.title}</h1>
            <p>{fmtDateTime(e.starts_at, ft)} · {e.location}</p>
          </div>
        </div>
        {e.can_manage && open && <Link className="btn" to={`/events/${e.id}/edit`}>{t('edit')}</Link>}
      </div>

      <div className="split">
        <div className="stack">
          <div className="card stack-sm">
            {e.description && <p style={{ whiteSpace: 'pre-wrap' }}>{e.description}</p>}
            <dl className="kv">
              <dt>{t('groups')}</dt><dd>{e.groups}</dd>
              <dt>{t('quota')}</dt><dd>{e.capacity ? `${e.going}/${e.capacity}` : t('quotaUnlimited', { going: e.going })}{e.waitlist > 0 && ` · ${t('waitlist', { waitlist: e.waitlist })}`}</dd>
              <dt>{t('cost')}</dt><dd>{e.cost_note || t('costNone')}</dd>
              {e.signup_deadline && <><dt>{t('deadline')}</dt><dd>{fmtDateTime(e.signup_deadline, ft)}</dd></>}
              <dt>{t('organiser')}</dt><dd>{e.created_by.name}</dd>
            </dl>
            {e.status === 'cancelled' && <div className="notice">{t('cancelled')}</div>}
          </div>

          <div className="card stack">
            {signed ? (
              <>
                <div className="row between">
                  <h3>{t('mySignup')}</h3>
                  <Pill tone={e.my_signup!.status === 'going' ? 'green' : 'amber'}>{e.my_signup!.status === 'going' ? t('going') : t('waitlist')}</Pill>
                </div>
                <p className="muted small">{t('signupNote')}</p>
                {e.my_ride ? (
                  <Link to={`/rides/${e.my_ride.id}`} className="row between card flat tight link">
                    <span>{t('myRide')}</span><StatusPill map={RIDE_STATUS} status={e.my_ride.status} />
                  </Link>
                ) : open && <Link className="btn primary" to={`/rides/new?event=${e.id}`}>{t('needRide')}</Link>}
                {has('driver') && open && <Link className="btn" to="/drive">{t('offerDrive')}</Link>}
                {open && <button className="btn danger" disabled={busy} onClick={() => run(() => post(`/events/${e.id}/withdraw`).then(reload), t('withdrawn'))}>{t('withdraw')}</button>}
              </>
            ) : (
              <div className="notice info">{t('signupNote')}</div>
            )}
            <ErrorBox error={actErr} />
          </div>
        </div>

        <div className="stack">
          {e.can_manage && (
            <div className="card stack">
              <div className="row between">
                <h3>{t('signups')}</h3>
                <span className="muted small">{t('attendance', { attended: signupsAttended, total: active.length })}</span>
              </div>
              {!active.length ? <Empty>{t('noSignups')}</Empty> : (
                <div className="list">
                  {active.map((s) => (
                    <div key={s.id} className="item">
                      <div className="grow">
                        <div className="bold">{s.name} {s.status === 'waitlist' && <Pill tone="amber">{t('waitlist')}</Pill>}</div>
                        <div className="muted tiny">
                          {s.need_ride && (s.ride_status ? t('needsRideWithStatus', { status: ft(`format:rideStatus.${s.ride_status}`) }) : t('needsRide'))}
                          {s.need_ride && s.offer_drive ? ' ' : ''}
                          {s.offer_drive && t('offersSeats', { n: s.offer_seats })}
                        </div>
                      </div>
                      <div className="chips">
                        <button className={`chip ${s.attended === 1 ? 'on' : ''}`} disabled={busy} onClick={() => mark(s.user_id, s.attended === 1 ? null : true)}>{t('attending')}</button>
                        <button className={`chip ${s.attended === 0 ? 'on' : ''}`} disabled={busy} onClick={() => mark(s.user_id, s.attended === 0 ? null : false)}>{t('absent')}</button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
              <button className="btn sm ghost" onClick={() => setWalkIn(true)}>{t('walkIn')}</button>
              <p className="muted tiny">{t('walkInNote')}</p>
            </div>
          )}

          <div className="card stack-sm">
            <h3>{t('afterEvent')}</h3>
            {e.settlement ? (
              <Link className="btn primary" to={`/settlements/${e.settlement.id}`}>{t('viewSettlement')}</Link>
            ) : (
              <Link className="btn primary" to={`/events/${e.id}/settle`}>{t('createSettlement')}</Link>
            )}
            {e.status === 'published' && (
              <div className="btn-row">
                <button className="btn" disabled={busy} onClick={() => run(() => post(`/events/${e.id}/finish`).then(reload), t('finishedDone'))}>{t('finish')}</button>
                <button
                  className="btn danger"
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
                </button>
              </div>
            )}
          </div>
          <ErrorBox error={error} />
          {walkIn && <WalkIn onClose={() => setWalkIn(false)} onPick={(uid) => mark(uid, true).then(() => setWalkIn(false))} />}
        </div>
      </div>
    </div>
  );
}

function WalkIn({ onClose, onPick }: { onClose: () => void; onPick: (uid: number) => void }) {
  const { t } = useTranslation('eventDetail');
  const [q, setQ] = useState('');
  const { data } = useApi<{ id: number; name: string }[]>(q.length >= 1 ? `/admin/users?q=${encodeURIComponent(q)}` : null);
  return (
    <Modal title={t('walkInTitle')} onClose={onClose}>
      <input className="input" placeholder={t('walkInPlaceholder')} value={q} onChange={(e) => setQ(e.target.value)} autoFocus />
      <div className="list">
        {data?.slice(0, 10).map((u) => (
          <button key={u.id} className="item btn ghost" style={{ justifyContent: 'flex-start' }} onClick={() => onPick(u.id)}>{u.name}</button>
        ))}
      </div>
    </Modal>
  );
}
