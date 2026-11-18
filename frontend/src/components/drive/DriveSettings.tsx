import { FormEvent, useEffect, useMemo, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { Check, Minus, Plus, Repeat, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { del, post, put, useApi } from '../../api';
import { useAuth } from '../../auth';
import {
  Button,
  Card,
  ErrorBox,
  Field,
  IconButton,
  Spinner,
  useAction,
} from '../ui';
import type { Area, DriverStanding, Profile } from './types';
import { WEEKDAYS_DRIVE } from './types';

export function DriveSettings() {
  const { t } = useTranslation('drive');
  const { can } = useAuth();
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
        gate_wait_mins: Number(profile.gate_wait_mins) || 5,
        area_ids: Array.isArray(profile.area_ids)
          ? [...profile.area_ids].sort((a, b) => a - b)
          : [],
        vehicle: {
          model: profile.vehicle?.model ?? '',
          color: profile.vehicle?.color ?? '',
          plate: profile.vehicle?.plate ?? '',
          seats: Number(profile.vehicle?.seats) || 5,
          vehicle_type:
            profile.vehicle?.vehicle_type ??
            (profile.vehicle as { type?: string } | null)?.type ??
            'standard',
          wheelchair_ok: !!profile.vehicle?.wheelchair_ok,
          low_step: !!profile.vehicle?.low_step,
          child_seat: !!profile.vehicle?.child_seat,
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
        vehicle_type: p.vehicle?.vehicle_type ?? 'standard',
        wheelchair_ok: !!p.vehicle?.wheelchair_ok,
        low_step: !!p.vehicle?.low_step,
        child_seat: !!p.vehicle?.child_seat,
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
  const setV = (k: keyof typeof v, val: string | number | boolean) =>
    setP({ ...p, vehicle: { ...v, [k]: val } });

  const save = async (e: FormEvent) => {
    e.preventDefault();
    if (!p || !p.vehicle) return;
    const ok = await run(
      () =>
        put('/driver/profile', {
          ...p,
          spare_seats: Number(p.spare_seats),
          gate_wait_mins: Number(p.gate_wait_mins || 5),
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
    <div className="stack">
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
                  {t('seatCount', { count: p.spare_seats })}
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
            <Field label={t('gateWaitMins')} hint={t('gateWaitMinsHint')}>
              <select
                className="input"
                value={p.gate_wait_mins ?? 5}
                onChange={(e) => setP({ ...p, gate_wait_mins: Number(e.target.value) })}
              >
                <option value={3}>{t('mins3')}</option>
                <option value={5}>{t('mins5')}</option>
                <option value={8}>{t('mins8')}</option>
                <option value={10}>{t('mins10')}</option>
              </select>
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
                    {t('seatCount', { count: v.seats })}
                  </span>
                  <IconButton
                    label={t('increaseSeats')}
                    icon={<Plus size={18} />}
                    disabled={v.seats >= 9}
                    onClick={() => setV('seats', Math.min(9, v.seats + 1))}
                  />
                </div>
              </Field>
              <Field label={t('vehicleType')}>
                <select
                  className="input"
                  value={v.vehicle_type}
                  onChange={(e) => setV('vehicle_type', e.target.value)}
                >
                  <option value="standard">{t('vehicleTypeStandard')}</option>
                  <option value="other">{t('vehicleTypeOther')}</option>
                </select>
              </Field>
            </div>

            <div className="stack-xs">
              <div className="bold small">{t('accessibilityTitle')}</div>
              <label className={`check ${v.wheelchair_ok ? 'on' : ''}`}>
                <input
                  type="checkbox"
                  checked={!!v.wheelchair_ok}
                  onChange={(e) => setV('wheelchair_ok', e.target.checked)}
                />
                {t('wheelchairOk')}
              </label>
              <label className={`check ${v.low_step ? 'on' : ''}`}>
                <input
                  type="checkbox"
                  checked={!!v.low_step}
                  onChange={(e) => setV('low_step', e.target.checked)}
                />
                {t('lowStep')}
              </label>
              <label className={`check ${v.child_seat ? 'on' : ''}`}>
                <input
                  type="checkbox"
                  checked={!!v.child_seat}
                  onChange={(e) => setV('child_seat', e.target.checked)}
                />
                {t('childSeat')}
              </label>
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
          {can('drive', 'update') && (
            <Button
              variant="primary"
              type="submit"
              disabled={busy || !isDirty}
            >
              {t('saveSettings')}
            </Button>
          )}
        </div>
      </div>
    </form>
      <StandingDriverSchedules areas={(areas ?? []).filter((a) => a.active)} />
    </div>
  );
}

function StandingDriverSchedules({ areas }: { areas: Area[] }) {
  const { t } = useTranslation('drive');
  const { can } = useAuth();
  const canUpdate = can('drive', 'update');
  const { data: listRaw, reload } = useApi<DriverStanding[]>('/recurring-schedules');
  const rows = (Array.isArray(listRaw) ? listRaw : []).filter((r) => r.kind === 'driver');
  const { busy, error, run } = useAction();
  const [f, setF] = useState({ weekday: 0, pickup_time: '08:30', area_id: '', destination: '', label: '' });

  const add = () => {
    if (!canUpdate) return;
    run(async () => {
      await post('/recurring-schedules', {
        kind: 'driver',
        label: f.label.trim(),
        weekday: Number(f.weekday),
        pickup_time: f.pickup_time,
        area_id: Number(f.area_id),
        destination: f.destination.trim(),
        seats: 1,
      });
      setF({ weekday: 0, pickup_time: '08:30', area_id: '', destination: '', label: '' });
      await reload();
    }, t('standingAdded'));
  };

  return (
    <Card className="stack" style={{ marginTop: 'var(--space-4)' }}>
      <div className="row nowrap gap-xs items-center">
        <Repeat size={18} aria-hidden="true" />
        <h3 className="no-margin">{t('standingTitle')}</h3>
      </div>
      <p className="muted small no-margin">{t('standingDesc')}</p>
      {rows.length > 0 && (
        <div className="stack-sm">
          {rows.map((s) => (
            <div key={s.id} className="card flat tight row between nowrap gap-sm">
              <div className="min-w-0">
                <div className="bold small">
                  {t(`weekdays.${WEEKDAYS_DRIVE[s.weekday] ?? 'sun'}`)} · {s.pickup_time}
                  {s.label ? ` · ${s.label}` : ''}
                </div>
                <div className="muted tiny">
                  {s.area_name}{s.destination ? ` → ${s.destination}` : ''}
                </div>
              </div>
              {canUpdate && (
                <IconButton
                  size="sm"
                  variant="danger"
                  label={t('standingDelete')}
                  icon={<Trash2 size={15} />}
                  onClick={() => run(() => del(`/recurring-schedules/${s.id}`).then(reload))}
                />
              )}
            </div>
          ))}
        </div>
      )}
      {canUpdate && (
        <>
          <div className="form-grid two">
            <Field label={t('standingDay')}>
              <select className="input" value={f.weekday} onChange={(e) => setF({ ...f, weekday: Number(e.target.value) })}>
                {WEEKDAYS_DRIVE.map((w, idx) => (
                  <option key={w} value={idx}>{t(`weekdays.${w}`)}</option>
                ))}
              </select>
            </Field>
            <Field label={t('standingTime')}>
              <input type="time" className="input" value={f.pickup_time} onChange={(e) => setF({ ...f, pickup_time: e.target.value })} />
            </Field>
            <Field label={t('standingArea')}>
              <select className="input" value={f.area_id} onChange={(e) => setF({ ...f, area_id: e.target.value })}>
                <option value="">—</option>
                {areas.map((a) => (
                  <option key={a.id} value={a.id}>{a.name}</option>
                ))}
              </select>
            </Field>
            <Field label={t('standingLabel')}>
              <input className="input" value={f.label} onChange={(e) => setF({ ...f, label: e.target.value })} placeholder={t('standingLabelPlaceholder')} />
            </Field>
          </div>
          <ErrorBox error={error} />
          <Button variant="secondary" disabled={busy || !f.area_id} onClick={add}>
            {t('standingAdd')}
          </Button>
        </>
      )}
    </Card>
  );
}
