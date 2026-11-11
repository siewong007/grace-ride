import { FormEvent, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { post, put, useApi } from '../api';
import { useAuth } from '../auth';
import { Ride } from '../components/RideCard';
import { Empty, ErrorBox, Field, Pill, Spinner, StatusPill, useAction } from '../components/ui';
import { fmtDay, fmtTime, localDate, RIDE_STATUS, todayStr, useFormatT } from '../format';
import { ShareLocation, StartTripButton } from './RideDetail';
import { useTranslation } from 'react-i18next';

interface Area { id: number; name: string; active: boolean }
interface Profile { carpool_enabled: boolean; spare_seats: number; area_ids: number[]; vehicle: { model: string; color: string; plate: string; seats: number } | null }

export default function Drive() {
  const { t, ft } = useTranslationHooks();
  const { has } = useAuth();
  const [tab, setTab] = useState<'schedule' | 'requests' | 'settings'>('schedule');
  if (!has('driver')) return <Empty>{t('notDriver')}</Empty>;
  return (
    <div className="stack">
      <div className="page-head">
        <div>
          <h1>{t('title')}</h1>
          <p>{t('desc')}</p>
        </div>
      </div>
      <div className="tabs">
        <button className={tab === 'schedule' ? 'on' : ''} onClick={() => setTab('schedule')}>{t('schedule')}</button>
        <button className={tab === 'requests' ? 'on' : ''} onClick={() => setTab('requests')}>{t('requests')}</button>
        <button className={tab === 'settings' ? 'on' : ''} onClick={() => setTab('settings')}>{t('settings')}</button>
      </div>
      {tab === 'schedule' && <Schedule ft={ft} />}
      {tab === 'requests' && <Requests t={t} ft={ft} />}
      {tab === 'settings' && <Settings t={t} />}
    </div>
  );
}

function useTranslationHooks() {
  const { t } = useTranslation('drive');
  const ft = useFormatT();
  return { t, ft };
}

function Schedule({ ft }: { ft: (k: string) => string }) {
  const { t } = useTranslation('drive');
  const [range, setRange] = useState<'day' | 'week'>('day');
  const from = todayStr();
  const end = new Date();
  end.setDate(end.getDate() + 6);
  const to = range === 'day' ? from : localDate(end);
  const { data, error, loading, reload } = useApi<Ride[]>(`/driver/schedule?from=${from}&to=${to}`, 20000);
  const { busy, error: actErr, run } = useAction();
  const active = data?.some((r) => r.status === 'in_progress');
  const act = (r: Ride, path: string, msg: string) => run(() => post(`/rides/${r.id}/${path}`).then(reload), msg);

  const byDay = new Map<string, Ride[]>();
  data?.forEach((r) => {
    const k = r.pickup_at.slice(0, 10);
    byDay.set(k, [...(byDay.get(k) ?? []), r]);
  });

  return (
    <div className="stack">
      <div className="row between">
        <div className="chips">
          <button className={`chip ${range === 'day' ? 'on' : ''}`} onClick={() => setRange('day')}>{t('today')}</button>
          <button className={`chip ${range === 'week' ? 'on' : ''}`} onClick={() => setRange('week')}>{t('week7')}</button>
        </div>
        <span className="muted small">{t('slotRides', { n: data?.length ?? 0 })}</span>
      </div>
      {active && <ShareLocation />}
      <ErrorBox error={error ?? actErr} />
      {loading ? <Spinner /> : !data?.length ? <Empty>{t('noTasks')}</Empty> : (
        [...byDay.entries()].map(([day, rides]) => (
          <div key={day} className="card">
            <h3 style={{ marginBottom: 6 }}>{fmtDay(`${day}T00:00`, ft)} · {rides.length} {t('seatsUnit')}</h3>
            <div className="list">
              {rides.map((r) => (
                <div key={r.id} className="item" style={{ alignItems: 'flex-start' }}>
                  <span className="timeline-time">{fmtTime(r.pickup_at, ft).replace(/^.. /, '')}</span>
                  <div className="grow stack-sm">
                    <Link to={`/rides/${r.id}`} className="row between" style={{ color: 'inherit' }}>
                      <span className="bold">{r.passenger.name} · {r.seats} {t('seatsUnit')}</span>
                      <StatusPill map={RIDE_STATUS} status={r.status} />
                    </Link>
                    <div className="muted small">{r.pickup_address ?? r.area.name} → {r.destination}</div>
                    {r.assistance && <div className="small">♿ {r.assistance}</div>}
                    <div className="btn-row">
                      {r.status === 'confirmed' && <StartTripButton ride={r} onStarted={reload} className="btn sm primary" label={t('startTrip')} />}
                      {r.status === 'in_progress' && !r.picked_up_at && (
                        <>
                          <button className="btn sm primary" disabled={busy} onClick={() => act(r, 'pickup', t('respondedPickedUp'))}>{t('pickedUp')}</button>
                          <button className="btn sm danger" disabled={busy} onClick={() => act(r, 'no-show', t('respondedNoShow'))}>{t('noShow')}</button>
                        </>
                      )}
                      {r.status === 'in_progress' && <button className="btn sm accent" disabled={busy} onClick={() => act(r, 'complete', t('respondedCompleted'))}>{t('completed')}</button>}
                      {r.status === 'pending_driver_confirm' && <button className="btn sm primary" disabled={busy} onClick={() => act(r, 'confirm-assignment', t('respondedCompleted'))}>{t('confirmAssignment')}</button>}
                      {r.passenger.phone && <a className="btn sm" href={`tel:${r.passenger.phone}`}>{t('call')}</a>}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        ))
      )}
      <p className="muted small">{t('cannotDriveNotice')}</p>
    </div>
  );
}

function Requests({ t, ft }: { t: (k: string, o?: any) => string; ft: (k: string) => string }) {
  const { data: areas } = useApi<Area[]>('/areas');
  const [area, setArea] = useState<string>('mine');
  const [date, setDate] = useState('');
  const q = new URLSearchParams();
  if (area === 'mine') q.set('mine_areas', 'true');
  else if (area) q.set('area_id', area);
  if (date) q.set('date', date);
  const { data, error, loading, reload } = useApi<Ride[]>(`/driver/requests?${q}`, 30000);
  const { busy, error: actErr, run } = useAction();

  return (
    <div className="stack">
      <div className="card tight stack-sm">
        <div className="chips" role="group" aria-label={t('areas')}>
          <button className={`chip ${area === 'mine' ? 'on' : ''}`} onClick={() => setArea('mine')}>{t('myAreas')}</button>
          <button className={`chip ${area === '' ? 'on' : ''}`} onClick={() => setArea('')}>{t('all')}</button>
          {areas?.filter((a) => a.active).map((a) => (
            <button key={a.id} className={`chip ${area === String(a.id) ? 'on' : ''}`} onClick={() => setArea(String(a.id))}>{a.name}</button>
          ))}
        </div>
        <div className="row">
          <input className="input" type="date" value={date} onChange={(e) => setDate(e.target.value)} style={{ maxWidth: 220 }} aria-label={t('dateLabel')} />
          {date && <button className="btn ghost sm" onClick={() => setDate('')}>{t('clearDate')}</button>}
        </div>
      </div>
      <ErrorBox error={error ?? actErr} />
      {loading ? <Spinner /> : !data?.length ? <Empty>{t('noRequests')}</Empty> : (
        <div className="grid cols-3">
          {data.map((r) => (
            <div key={r.id} className="card stack-sm">
              <div className="row between">
                <span className="bold" style={{ fontSize: '1.15rem' }}>{r.area.name}</span>
                {r.event && <Pill tone="blue">{r.event.title}</Pill>}
              </div>
              <div>{fmtDay(r.pickup_at, ft)} · {fmtTime(r.pickup_at, ft)} · {r.seats} {t('seatUnit')}</div>
              <div className="muted small">{t('passenger')} {r.passenger.name} → {r.destination}</div>
              {r.assistance && <div className="small">♿ {r.assistance}</div>}
              <div className="muted tiny">🔒 {t('pickupAddressLocked')}</div>
              <button className="btn primary" disabled={busy} onClick={() => run(() => post(`/rides/${r.id}/claim`).then(reload), t('claimed'))}>{t('selectRide')}</button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function Settings({ t }: { t: (k: string, o?: any) => string }) {
  const { data: areas } = useApi<Area[]>('/areas');
  const { data: profile, loading } = useApi<Profile>('/driver/profile');
  const [p, setP] = useState<Profile | null>(null);
  const { busy, error, run } = useAction();
  useEffect(() => {
    if (profile) setP({ ...profile, vehicle: profile.vehicle ?? { model: '', color: '', plate: '', seats: 5 } });
  }, [profile]);
  if (loading || !p) return <Spinner />;

  const toggleArea = (id: number) =>
    setP({ ...p, area_ids: p.area_ids.includes(id) ? p.area_ids.filter((a) => a !== id) : [...p.area_ids, id] });
  const v = p.vehicle!;
  const setV = (k: keyof typeof v, val: string | number) => setP({ ...p, vehicle: { ...v, [k]: val } });

  const save = (e: FormEvent) => {
    e.preventDefault();
    run(() => put('/driver/profile', { ...p, spare_seats: Number(p.spare_seats), vehicle: { ...v, seats: Number(v.seats) } }), t('settingsSaved'));
  };

  return (
    <form className="split" onSubmit={save}>
      <div className="stack">
        <div className="card stack">
          <div>
            <h3>{t('areas')}</h3>
            <p className="muted small">{t('areasHint')}</p>
          </div>
          <div className="grid cols-2">
            {areas?.filter((a) => a.active).map((a) => (
              <label key={a.id} className={`check ${p.area_ids.includes(a.id) ? 'on' : ''}`}>
                <input type="checkbox" checked={p.area_ids.includes(a.id)} onChange={() => toggleArea(a.id)} />
                {a.name}
              </label>
            ))}
          </div>
        </div>
        <div className="card stack">
          <h3>{t('carpool')}</h3>
          <label className={`check ${p.carpool_enabled ? 'on' : ''}`}>
            <input type="checkbox" checked={p.carpool_enabled} onChange={(e) => setP({ ...p, carpool_enabled: e.target.checked })} />
            {t('carpoolLabel')}
          </label>
          <Field label={t('spare')} hint={t('spareHint')}>
            <select className="input" value={p.spare_seats} onChange={(e) => setP({ ...p, spare_seats: Number(e.target.value) })}>
              {[1, 2, 3, 4, 5, 6].map((n) => <option key={n} value={n}>{n}</option>)}
            </select>
          </Field>
        </div>
      </div>
      <div className="stack">
        <div className="card stack">
          <div>
            <h3>{t('vehicle')}</h3>
            <p className="muted small">{t('vehicleHint')}</p>
          </div>
          <div className="form-grid two">
            <Field label={t('model')}>
              <input className="input" required value={v.model} onChange={(e) => setV('model', e.target.value)} placeholder="Perodua Myvi" />
            </Field>
            <Field label={t('color')}>
              <input className="input" required value={v.color} onChange={(e) => setV('color', e.target.value)} placeholder="白色" />
            </Field>
            <Field label={t('plate')}>
              <input className="input" required value={v.plate} onChange={(e) => setV('plate', e.target.value)} placeholder="VXX 1234" />
            </Field>
            <Field label={t('seats')}>
              <select className="input" value={v.seats} onChange={(e) => setV('seats', Number(e.target.value))}>
                {[2, 4, 5, 7, 8, 9].map((n) => <option key={n} value={n}>{n}</option>)}
              </select>
            </Field>
          </div>
        </div>
        <ErrorBox error={error} />
        <button className="btn primary block" disabled={busy} type="submit">{t('saveSettings')}</button>
        <Link to="/report" className="btn block">{t('badges')}</Link>
      </div>
    </form>
  );
}
