import { FormEvent, useEffect, useMemo, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { Accessibility, Check, Lock, Minus, Phone, Plus } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { post, put, useApi } from '../api';
import { useAuth } from '../auth';
import { Ride } from '../components/RideCard';
import {
  NavButtonGroup,
  ReasonModal,
  ShareLocation,
  StartTripButton,
} from '../components/TripActions';
import {
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Field,
  IconButton,
  Page,
  PageHeader,
  Pill,
  Spinner,
  StatusPill,
  Tabs,
  useAction,
  useConfirm,
  useToast,
} from '../components/ui';
import {
  fmtDay,
  fmtTime,
  localDate,
  RIDE_STATUS,
  todayStr,
  useFormatT,
} from '../format';

interface Area {
  id: number;
  name: string;
  active: boolean;
}

interface Profile {
  carpool_enabled: boolean;
  spare_seats: number;
  area_ids: number[];
  vehicle: {
    model: string;
    color: string;
    plate: string;
    seats: number;
  } | null;
}

export default function Drive() {
  const { t } = useTranslation('drive');
  const { has } = useAuth();
  const [tab, setTab] = useState<'schedule' | 'requests' | 'settings'>('schedule');

  if (!has('driver')) {
    return (
      <Page>
        <EmptyState title={t('notDriverTitle')} description={t('notDriver')} />
      </Page>
    );
  }

  return (
    <Page>
      <PageHeader title={t('title')} subtitle={t('desc')} />
      <Tabs
        value={tab}
        onChange={(v) => setTab(v as 'schedule' | 'requests' | 'settings')}
        items={[
          { value: 'schedule', label: t('schedule') },
          { value: 'requests', label: t('requests') },
          { value: 'settings', label: t('settings') },
        ]}
      />
      {tab === 'schedule' && <Schedule />}
      {tab === 'requests' && <Requests />}
      {tab === 'settings' && <Settings />}
    </Page>
  );
}

function Schedule() {
  const { t } = useTranslation('drive');
  const ft = useFormatT();
  const confirm = useConfirm();
  const [range, setRange] = useState<'day' | 'week'>('day');
  const [withdrawRide, setWithdrawRide] = useState<Ride | null>(null);

  const from = todayStr();
  const end = new Date();
  end.setDate(end.getDate() + 6);
  const to = range === 'day' ? from : localDate(end);

  const { data, error, loading, reload } = useApi<Ride[]>(
    `/driver/schedule?from=${from}&to=${to}`,
    20000,
  );
  const { busy, error: actErr, run } = useAction();
  const active = data?.some((r) => r.status === 'in_progress');

  const act = (r: Ride, path: string, msg: string) =>
    run(() => post(`/rides/${r.id}/${path}`).then(reload), msg);

  const handleNoShow = async (r: Ride) => {
    const ok = await confirm({
      title: t('confirmNoShowTitle'),
      body: t('confirmNoShowBody'),
      tone: 'danger',
      confirmLabel: t('confirmNoShowBtn'),
    });
    if (ok) {
      act(r, 'no-show', t('respondedNoShow'));
    }
  };

  const byDay = useMemo(() => {
    const map = new Map<string, Ride[]>();
    if (!data) return map;
    const sorted = data.slice().sort((a, b) => a.pickup_at.localeCompare(b.pickup_at));
    sorted.forEach((r) => {
      const k = r.pickup_at.slice(0, 10);
      map.set(k, [...(map.get(k) ?? []), r]);
    });
    return map;
  }, [data]);

  return (
    <div className="stack">
      <div className="driver-schedule-controls">
        <div className="chips" role="group" aria-label={t('scheduleRange')}>
          <button
            type="button"
            className={`chip ${range === 'day' ? 'on' : ''}`}
            onClick={() => setRange('day')}
          >
            {t('today')}
          </button>
          <button
            type="button"
            className={`chip ${range === 'week' ? 'on' : ''}`}
            onClick={() => setRange('week')}
          >
            {t('week7')}
          </button>
        </div>
        <span className="muted small">{t('slotRides', { n: data?.length ?? 0 })}</span>
      </div>

      {active && <ShareLocation />}
      <ErrorBox error={error ?? actErr} />

      {loading ? (
        <CardSkeleton count={2} />
      ) : !data?.length ? (
        <EmptyState title={t('noTasks')} />
      ) : (
        [...byDay.entries()].map(([day, rides]) => (
          <div key={day} className="stack">
            <div className="sticky-date-header">
              {fmtDay(`${day}T00:00`, ft)} · {rides.length} {t('seatsUnit')}
            </div>
            <div className="stack">
              {rides.map((r) => (
                <div key={r.id} className="driver-trip-card">
                  <div className="driver-trip-header">
                    <div className="driver-trip-title-row">
                      <div className="driver-trip-time">
                        {fmtTime(r.pickup_at, ft).replace(/^.. /, '')}
                      </div>
                      <div>
                        <Link to={`/rides/${r.id}`} className="driver-trip-title">
                          <span>{r.passenger.name}</span>
                          <span className="muted small">· {r.seats} {t('seatsUnit')}</span>
                        </Link>
                        <div className="driver-trip-route">
                          <span>
                            {r.pickup_address ? (
                              r.pickup_address
                            ) : (
                              <span className="driver-trip-route-lock">
                                <Lock size={12} aria-hidden="true" />
                                <span>{r.area.name}</span>
                              </span>
                            )}
                            {' → '}
                            {r.destination}
                          </span>
                        </div>
                      </div>
                    </div>
                    <StatusPill map={RIDE_STATUS} status={r.status} />
                  </div>

                  {r.assistance && (
                    <div className="driver-trip-assistance">
                      <Accessibility size={14} aria-hidden="true" />
                      <span>{r.assistance}</span>
                    </div>
                  )}

                  {r.pickup_address && (
                    <NavButtonGroup address={r.pickup_address} />
                  )}

                  <div className="driver-trip-actions">
                    {r.status === 'confirmed' && (
                      <div className="driver-trip-primary-btn">
                        <StartTripButton
                          ride={r}
                          onStarted={reload}
                          block
                          label={t('startTrip')}
                        />
                      </div>
                    )}
                    {r.status === 'in_progress' && !r.picked_up_at && (
                      <Button
                        variant="primary"
                        block
                        className="driver-trip-primary-btn"
                        disabled={busy}
                        onClick={() => act(r, 'pickup', t('respondedPickedUp'))}
                      >
                        {t('pickedUp')}
                      </Button>
                    )}
                    {r.status === 'in_progress' && r.picked_up_at && (
                      <Button
                        variant="primary"
                        block
                        className="driver-trip-primary-btn"
                        disabled={busy}
                        onClick={() => act(r, 'complete', t('respondedCompleted'))}
                      >
                        {t('completed')}
                      </Button>
                    )}
                    {r.status === 'pending_driver_confirm' && (
                      <Button
                        variant="primary"
                        block
                        className="driver-trip-primary-btn"
                        disabled={busy}
                        onClick={() => act(r, 'confirm-assignment', t('respondedCompleted'))}
                      >
                        {t('confirmAssignment')}
                      </Button>
                    )}

                    {r.passenger.phone && (
                      <a className="btn secondary" href={`tel:${r.passenger.phone}`}>
                        <Phone size={14} aria-hidden="true" />
                        <span>{t('call')}</span>
                      </a>
                    )}

                    {r.status === 'in_progress' && !r.picked_up_at && (
                      <Button
                        variant="danger"
                        size="sm"
                        disabled={busy}
                        onClick={() => handleNoShow(r)}
                      >
                        {t('noShow')}
                      </Button>
                    )}

                    {(r.status === 'confirmed' || r.status === 'pending_driver_confirm') && (
                      <Button
                        variant="secondary"
                        size="sm"
                        disabled={busy}
                        onClick={() => setWithdrawRide(r)}
                      >
                        {t('withdrawDriver')}
                      </Button>
                    )}

                    <Link to={`/rides/${r.id}`} className="btn sm secondary">
                      {t('viewRide')}
                    </Link>
                  </div>
                </div>
              ))}
            </div>
          </div>
        ))
      )}

      {withdrawRide && (
        <ReasonModal
          title={t('withdrawTitle')}
          label={t('withdrawLabel')}
          confirm={t('withdrawConfirm')}
          onClose={() => setWithdrawRide(null)}
          onSubmit={(reason) =>
            run(
              () => post(`/rides/${withdrawRide.id}/withdraw`, { reason }).then(reload),
              t('respondedWithdrew'),
            )
          }
        />
      )}

      <p className="muted small">{t('cannotDriveNotice')}</p>
    </div>
  );
}

function Requests() {
  const { t } = useTranslation('drive');
  const ft = useFormatT();
  const toast = useToast();
  const { data: areas } = useApi<Area[]>('/areas');
  const [area, setArea] = useState<string>('mine');
  const [date, setDate] = useState('');

  const q = new URLSearchParams();
  if (area === 'mine') q.set('mine_areas', 'true');
  else if (area) q.set('area_id', area);
  if (date) q.set('date', date);

  const { data, error, loading, reload } = useApi<Ride[]>(
    `/driver/requests?${q}`,
    30000,
  );
  const { busy, error: actErr } = useAction();

  const claim = async (rideId: number) => {
    try {
      await post(`/rides/${rideId}/claim`);
      toast.success(t('claimed'));
      reload();
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : t('conflictNotice');
      toast.error(message);
      reload();
    }
  };

  return (
    <div className="stack">
      <Card className="tight stack-sm">
        <div className="chips" role="group" aria-label={t('areas')}>
          <button
            type="button"
            className={`chip ${area === 'mine' ? 'on' : ''}`}
            onClick={() => setArea('mine')}
          >
            {t('myAreas')}
          </button>
          <button
            type="button"
            className={`chip ${area === '' ? 'on' : ''}`}
            onClick={() => setArea('')}
          >
            {t('all')}
          </button>
          {areas
            ?.filter((a) => a.active)
            .map((a) => (
              <button
                key={a.id}
                type="button"
                className={`chip ${area === String(a.id) ? 'on' : ''}`}
                onClick={() => setArea(String(a.id))}
              >
                {a.name}
              </button>
            ))}
        </div>
        <div className="row">
          <input
            className="input driver-date-input"
            type="date"
            value={date}
            onChange={(e) => setDate(e.target.value)}
            aria-label={t('dateLabel')}
          />
          {date && (
            <Button variant="ghost" size="sm" onClick={() => setDate('')}>
              {t('clearDate')}
            </Button>
          )}
        </div>
      </Card>

      <ErrorBox error={error ?? actErr} />

      {loading ? (
        <CardSkeleton count={3} />
      ) : !data?.length ? (
        <EmptyState title={t('noRequests')} />
      ) : (
        <div className="driver-requests-grid">
          {data.map((r) => (
            <div key={r.id} className="driver-request-card">
              <div className="driver-request-card-head">
                <span className="driver-request-area">{r.area.name}</span>
                {r.event && <Pill tone="blue">{r.event.title}</Pill>}
              </div>

              <div className="driver-request-meta">
                {fmtDay(r.pickup_at, ft)} · {fmtTime(r.pickup_at, ft)} · {r.seats}{' '}
                {t('seatUnit')}
              </div>

              <div className="driver-request-route">
                <span className="muted">{t('passenger')} </span>
                <span className="bold">{r.passenger.name}</span>
                <span> → {r.destination}</span>
              </div>

              {r.assistance && (
                <div className="driver-trip-assistance">
                  <Accessibility size={14} aria-hidden="true" />
                  <span>{r.assistance}</span>
                </div>
              )}

              <div className="driver-request-locked">
                <Lock size={12} aria-hidden="true" />
                <span>{t('pickupAddressLocked')}</span>
              </div>

              <Button
                variant="primary"
                block
                disabled={busy}
                onClick={() => claim(r.id)}
              >
                {t('offerToDrive')}
              </Button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function Settings() {
  const { t } = useTranslation('drive');
  const { data: areas } = useApi<Area[]>('/areas');
  const { data: profile, loading } = useApi<Profile>('/driver/profile');
  const [p, setP] = useState<Profile | null>(null);
  const initialRef = useRef<string>('');
  const { busy, error, run } = useAction();

  useEffect(() => {
    if (profile) {
      const initial: Profile = {
        carpool_enabled: !!profile.carpool_enabled,
        spare_seats: Number(profile.spare_seats) || 1,
        area_ids: Array.isArray(profile.area_ids)
          ? [...profile.area_ids].sort((a, b) => a - b)
          : [],
        vehicle: {
          model: profile.vehicle?.model ?? '',
          color: profile.vehicle?.color ?? '',
          plate: profile.vehicle?.plate ?? '',
          seats: Number(profile.vehicle?.seats) || 5,
        },
      };
      setP(initial);
      initialRef.current = JSON.stringify(initial);
    }
  }, [profile]);

  const isDirty = useMemo(() => {
    if (!p || !initialRef.current) return false;
    const currentNormalized = {
      ...p,
      area_ids: [...p.area_ids].sort((a, b) => a - b),
      vehicle: {
        model: p.vehicle?.model ?? '',
        color: p.vehicle?.color ?? '',
        plate: p.vehicle?.plate ?? '',
        seats: Number(p.vehicle?.seats) || 5,
      },
    };
    return JSON.stringify(currentNormalized) !== initialRef.current;
  }, [p]);

  if (loading || !p || !p.vehicle) return <Spinner />;

  const toggleArea = (id: number) => {
    setP({
      ...p,
      area_ids: p.area_ids.includes(id)
        ? p.area_ids.filter((a) => a !== id)
        : [...p.area_ids, id],
    });
  };

  const v = p.vehicle;
  const setV = (k: keyof typeof v, val: string | number) =>
    setP({ ...p, vehicle: { ...v, [k]: val } });

  const save = async (e: FormEvent) => {
    e.preventDefault();
    if (!p || !p.vehicle) return;
    const ok = await run(
      () =>
        put('/driver/profile', {
          ...p,
          spare_seats: Number(p.spare_seats),
          vehicle: { ...v, seats: Number(v.seats) },
        }),
      t('settingsSaved'),
    );
    if (ok) {
      const savedNormalized = {
        ...p,
        area_ids: [...p.area_ids].sort((a, b) => a - b),
        vehicle: { ...v, seats: Number(v.seats) },
      };
      initialRef.current = JSON.stringify(savedNormalized);
    }
  };

  return (
    <form onSubmit={save}>
      <div className="driver-settings-grid">
        <div className="stack">
          {/* Card 1: Service areas */}
          <Card className="stack">
            <div>
              <h3>{t('areas')}</h3>
              <p className="muted small">{t('areasHint')}</p>
            </div>
            <div className="chips" role="group" aria-label={t('areas')}>
              {areas
                ?.filter((a) => a.active)
                .map((a) => {
                  const on = p.area_ids.includes(a.id);
                  return (
                    <button
                      key={a.id}
                      type="button"
                      role="checkbox"
                      aria-checked={on}
                      className={`chip ${on ? 'on' : ''}`}
                      onClick={() => toggleArea(a.id)}
                    >
                      {a.name}
                    </button>
                  );
                })}
            </div>
          </Card>

          {/* Card 2: Carpool mode */}
          <Card className="stack">
            <h3>{t('carpool')}</h3>
            <label className={`check ${p.carpool_enabled ? 'on' : ''}`}>
              <input
                type="checkbox"
                checked={p.carpool_enabled}
                onChange={(e) => setP({ ...p, carpool_enabled: e.target.checked })}
              />
              {t('carpoolLabel')}
            </label>
            <Field label={t('spare')} hint={t('spareHint')}>
              <div className="stepper" role="group" aria-label={t('spare')}>
                <IconButton
                  label={t('decreaseSeats')}
                  icon={<Minus size={18} />}
                  disabled={p.spare_seats <= 1}
                  onClick={() =>
                    setP({ ...p, spare_seats: Math.max(1, p.spare_seats - 1) })
                  }
                />
                <span className="stepper-value bold">
                  {p.spare_seats} {t('seatUnit')}
                </span>
                <IconButton
                  label={t('increaseSeats')}
                  icon={<Plus size={18} />}
                  disabled={p.spare_seats >= 6}
                  onClick={() =>
                    setP({ ...p, spare_seats: Math.min(6, p.spare_seats + 1) })
                  }
                />
              </div>
            </Field>
          </Card>
        </div>

        <div className="stack">
          {/* Card 3: Vehicle with live plate-badge preview */}
          <Card className="stack">
            <div>
              <h3>{t('vehicle')}</h3>
              <p className="muted small">{t('vehicleHint')}</p>
            </div>

            <div className="plate-preview-box">
              <div className="plate">{v.plate.trim() || 'VXX 1234'}</div>
              <div className="plate-preview-desc">
                {v.color.trim() || t('colorPlaceholder')} ·{' '}
                {v.model.trim() || t('model')} ({v.seats} {t('seats')})
              </div>
            </div>

            <div className="form-grid two">
              <Field label={t('model')}>
                <input
                  className="input"
                  required
                  value={v.model}
                  onChange={(e) => setV('model', e.target.value)}
                  placeholder="Perodua Myvi"
                />
              </Field>
              <Field label={t('color')}>
                <input
                  className="input"
                  required
                  value={v.color}
                  onChange={(e) => setV('color', e.target.value)}
                  placeholder={t('colorPlaceholder')}
                />
              </Field>
              <Field label={t('plate')}>
                <input
                  className="input"
                  required
                  value={v.plate}
                  onChange={(e) => setV('plate', e.target.value)}
                  placeholder="VXX 1234"
                />
              </Field>
              <Field label={t('seats')}>
                <div className="stepper" role="group" aria-label={t('seats')}>
                  <IconButton
                    label={t('decreaseSeats')}
                    icon={<Minus size={18} />}
                    disabled={v.seats <= 2}
                    onClick={() => setV('seats', Math.max(2, v.seats - 1))}
                  />
                  <span className="stepper-value bold">
                    {v.seats} {t('seatUnit')}
                  </span>
                  <IconButton
                    label={t('increaseSeats')}
                    icon={<Plus size={18} />}
                    disabled={v.seats >= 9}
                    onClick={() => setV('seats', Math.min(9, v.seats + 1))}
                  />
                </div>
              </Field>
            </div>
          </Card>
        </div>
      </div>

      <ErrorBox error={error} />

      <div className="driver-settings-submit-bar">
        <div className="dirty-indicator-box">
          {isDirty ? (
            <span className="dirty-indicator dirty">
              <span className="dirty-dot" aria-hidden="true" />
              <span>{t('unsavedChanges')}</span>
            </span>
          ) : (
            <span className="dirty-indicator clean">
              <Check size={16} aria-hidden="true" />
              <span>{t('allChangesSaved')}</span>
            </span>
          )}
        </div>
        <div className="driver-settings-actions">
          <Link to="/report" className="btn secondary">
            {t('badges')}
          </Link>
          <Button
            variant="primary"
            type="submit"
            disabled={busy || !isDirty}
          >
            {t('saveSettings')}
          </Button>
        </div>
      </div>
    </form>
  );
}
