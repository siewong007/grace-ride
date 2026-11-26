import { FormEvent, useEffect, useMemo, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { Check, Edit2, Minus, Plus, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { del, post, put, useApi } from '../../api';
import { useAuth } from '../../auth';
import {
  Button,
  Card,
  ErrorBox,
  Field,
  IconButton,
  Modal,
  Spinner,
  useAction,
  useConfirm,
  useToast,
} from '../ui';
import type { Area, Profile, Vehicle } from './types';

interface VehicleModalProps {
  vehicle?: Vehicle | null;
  onClose: () => void;
  onSave: (v: Vehicle) => Promise<void>;
}

function VehicleModal({ vehicle, onClose, onSave }: VehicleModalProps) {
  const { t } = useTranslation('drive');
  const [form, setForm] = useState<Vehicle>({
    id: vehicle?.id,
    model: vehicle?.model ?? '',
    color: vehicle?.color ?? '',
    plate: vehicle?.plate ?? '',
    seats: vehicle?.seats ?? 5,
    vehicle_type: vehicle?.vehicle_type ?? vehicle?.type ?? 'standard',
    wheelchair_ok: !!vehicle?.wheelchair_ok,
    low_step: !!vehicle?.low_step,
    child_seat: !!vehicle?.child_seat,
    is_active: vehicle?.is_active ?? true,
  });
  const { busy, error, run } = useAction();
  const [fieldError, setFieldError] = useState<string | null>(null);

  const setF = (k: keyof Vehicle, val: string | number | boolean) => {
    setFieldError(null);
    setForm((prev) => ({ ...prev, [k]: val }));
  };

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    if (!form.model.trim() || !form.color.trim() || !form.plate.trim()) {
      setFieldError(t('vehicleFieldsRequired'));
      return;
    }
    await run(async () => {
      await onSave({
        ...form,
        model: form.model.trim(),
        color: form.color.trim(),
        plate: form.plate.trim().toUpperCase(),
        seats: Number(form.seats) || 5,
      });
      onClose();
    });
  };

  return (
    <Modal
      title={vehicle?.id || vehicle?.plate ? t('editVehicle') : t('addVehicle')}
      onClose={onClose}
    >
      <form onSubmit={handleSubmit} noValidate className="stack">
        <div className="plate-preview-box">
          <div className="plate">{form.plate.trim() || 'VXX 1234'}</div>
          <div className="plate-preview-desc">
            {form.color.trim() || t('colorPlaceholder')} ·{' '}
            {form.model.trim() || t('model')} ({form.seats} {t('seats')})
          </div>
        </div>

        <div className="form-grid two">
          <Field label={t('model')}>
            <input
              className="input"
              required
              value={form.model}
              onChange={(e) => setF('model', e.target.value)}
              placeholder="Perodua Myvi"
            />
          </Field>
          <Field label={t('color')}>
            <input
              className="input"
              required
              value={form.color}
              onChange={(e) => setF('color', e.target.value)}
              placeholder={t('colorPlaceholder')}
            />
          </Field>
          <Field label={t('plate')}>
            <input
              className="input"
              required
              value={form.plate}
              onChange={(e) => setF('plate', e.target.value)}
              placeholder="VXX 1234"
            />
          </Field>
          <Field label={t('seats')}>
            <div className="stepper" role="group" aria-label={t('seats')}>
              <IconButton
                label={t('decreaseSeats')}
                icon={<Minus size={18} />}
                disabled={form.seats <= 2}
                onClick={() => setF('seats', Math.max(2, form.seats - 1))}
              />
              <span className="stepper-value bold">
                {t('seatCount', { count: form.seats })}
              </span>
              <IconButton
                label={t('increaseSeats')}
                icon={<Plus size={18} />}
                disabled={form.seats >= 9}
                onClick={() => setF('seats', Math.min(9, form.seats + 1))}
              />
            </div>
          </Field>
          <Field label={t('vehicleType')}>
            <select
              className="input"
              value={form.vehicle_type}
              onChange={(e) => setF('vehicle_type', e.target.value)}
            >
              <option value="standard">{t('vehicleTypeStandard')}</option>
              <option value="other">{t('vehicleTypeOther')}</option>
            </select>
          </Field>
        </div>

        <div className="stack-xs">
          <div className="bold small">{t('accessibilityTitle')}</div>
          <label className={`check ${form.wheelchair_ok ? 'on' : ''}`}>
            <input
              type="checkbox"
              checked={!!form.wheelchair_ok}
              onChange={(e) => setF('wheelchair_ok', e.target.checked)}
            />
            {t('wheelchairOk')}
          </label>
          <label className={`check ${form.low_step ? 'on' : ''}`}>
            <input
              type="checkbox"
              checked={!!form.low_step}
              onChange={(e) => setF('low_step', e.target.checked)}
            />
            {t('lowStep')}
          </label>
          <label className={`check ${form.child_seat ? 'on' : ''}`}>
            <input
              type="checkbox"
              checked={!!form.child_seat}
              onChange={(e) => setF('child_seat', e.target.checked)}
            />
            {t('childSeat')}
          </label>
        </div>

        <label className={`check ${form.is_active ? 'on' : ''}`}>
          <input
            type="checkbox"
            checked={!!form.is_active}
            disabled={!!vehicle?.is_active}
            onChange={(e) => setF('is_active', e.target.checked)}
          />
          {t('setAsActive')}
        </label>

        <ErrorBox error={fieldError || error} />

        <div className="form-actions right">
          <Button type="button" variant="secondary" onClick={onClose}>
            {t('cancel', { defaultValue: 'Cancel' })}
          </Button>
          <Button type="submit" variant="primary" disabled={busy}>
            {vehicle?.id || vehicle?.plate ? t('saveSettings') : t('addVehicle')}
          </Button>
        </div>
      </form>
    </Modal>
  );
}

interface PrefsSnapshot {
  carpool_enabled: boolean;
  spare_seats: number;
  gate_wait_mins: number;
  area_ids: number[];
}

function normalizePrefs(src: Pick<Profile, 'carpool_enabled' | 'spare_seats' | 'gate_wait_mins' | 'area_ids'>): PrefsSnapshot {
  return {
    carpool_enabled: !!src.carpool_enabled,
    spare_seats: Number(src.spare_seats) || 1,
    gate_wait_mins: Number(src.gate_wait_mins) || 5,
    area_ids: [...src.area_ids].sort((a, b) => a - b),
  };
}

function clampForVehicle(spare: number, active: Vehicle | null): number {
  const max = active ? Math.min(7, Math.max(1, Number(active.seats) - 1)) : 6;
  return Math.max(1, Math.min(spare, max));
}

export function DriveSettings() {
  const { t } = useTranslation('drive');
  const { can } = useAuth();
  const confirm = useConfirm();
  const toast = useToast();
  const { data: areas } = useApi<Area[]>('/areas');
  const { data: profile, error: loadError, loading, reload } = useApi<Profile>('/driver/profile');
  const [p, setP] = useState<Profile | null>(null);
  const [vehicles, setVehicles] = useState<Vehicle[]>([]);
  const [modalVehicle, setModalVehicle] = useState<Vehicle | null | undefined>(undefined);
  const initialRef = useRef<string>('');
  const { busy, error, run } = useAction();

  useEffect(() => {
    if (profile) {
      const vList: Vehicle[] =
        profile.vehicles && profile.vehicles.length > 0
          ? profile.vehicles
          : profile.vehicle
            ? [{ ...profile.vehicle, is_active: true }]
            : [];
      const activeV =
        vList.find((v) => (profile.active_vehicle_id != null ? v.id === profile.active_vehicle_id : v.is_active)) ||
        vList.find((v) => v.is_active) ||
        vList[0] ||
        null;

      const initial: Profile = {
        carpool_enabled: !!profile.carpool_enabled,
        spare_seats: Number(profile.spare_seats) || 1,
        gate_wait_mins: Number(profile.gate_wait_mins) || 5,
        area_ids: Array.isArray(profile.area_ids)
          ? [...profile.area_ids].sort((a, b) => a - b)
          : [],
        vehicle: activeV
          ? {
              id: activeV.id,
              model: activeV.model ?? '',
              color: activeV.color ?? '',
              plate: activeV.plate ?? '',
              seats: Number(activeV.seats) || 5,
              vehicle_type: activeV.vehicle_type ?? (activeV as { type?: string }).type ?? 'standard',
              wheelchair_ok: !!activeV.wheelchair_ok,
              low_step: !!activeV.low_step,
              child_seat: !!activeV.child_seat,
              is_active: true,
            }
          : null,
        vehicles: vList,
        active_vehicle_id: activeV?.id,
      };
      setP(initial);
      setVehicles(vList);
      initialRef.current = JSON.stringify(normalizePrefs(initial));
    }
  }, [profile]);

  const isDirty = useMemo(() => {
    if (!p || !initialRef.current) return false;
    return JSON.stringify(normalizePrefs(p)) !== initialRef.current;
  }, [p]);

  if (loading && !p) return <Spinner />;
  if (!p) return <ErrorBox error={loadError} onRetry={reload} />;

  const toggleArea = (id: number) => {
    setP({
      ...p,
      area_ids: p.area_ids.includes(id)
        ? p.area_ids.filter((a) => a !== id)
        : [...p.area_ids, id],
    });
  };

  const v = p.vehicle;
  const maxSpareSeats = v ? Math.min(7, Math.max(1, Number(v.seats) - 1)) : 6;

  const syncClampedInitial = (nextSpare: number) => {
    if (!initialRef.current) return;
    try {
      const prev = JSON.parse(initialRef.current) as PrefsSnapshot;
      if (prev.spare_seats > nextSpare) {
        initialRef.current = JSON.stringify({ ...prev, spare_seats: nextSpare });
      }
    } catch {
      /* ignore */
    }
  };

  const selectVehicle = async (item: Vehicle) => {
    if (item.id != null) {
      const ok = await run(
        () => post(`/driver/vehicles/${item.id}/select`),
        t('vehicleSelected'),
      );
      if (!ok) return;
    }
    const updated = vehicles.map((vh) => ({
      ...vh,
      is_active: vh.id != null && item.id != null ? vh.id === item.id : vh.plate === item.plate,
    }));
    const chosen = updated.find((vh) => vh.is_active) || updated[0] || null;
    const nextSpare = clampForVehicle(p.spare_seats, chosen);
    syncClampedInitial(nextSpare);
    setVehicles(updated);
    if (chosen) {
      setP({ ...p, vehicle: chosen, vehicles: updated, active_vehicle_id: chosen.id, spare_seats: nextSpare });
    }
  };

  const deleteVehicle = async (item: Vehicle) => {
    const ok = await confirm({
      title: t('deleteVehicle'),
      body: t('deleteVehicleConfirm', { plate: item.plate, model: item.model }),
      confirmLabel: t('deleteVehicle'),
      tone: 'danger',
    });
    if (!ok) return;

    if (item.id != null) {
      const deleted = await run(
        () => del(`/driver/vehicles/${item.id}`),
        t('vehicleDeleted'),
      );
      if (!deleted) return;
    }
    const remaining = vehicles
      .filter((vh) => (vh.id != null && item.id != null ? vh.id !== item.id : vh.plate !== item.plate))
      .map((vh) => ({ ...vh }));
    if (item.is_active && remaining.length > 0) {
      remaining[0].is_active = true;
    }
    const chosen = remaining.find((vh) => vh.is_active) || remaining[0] || null;
    const nextSpare = clampForVehicle(p.spare_seats, chosen);
    syncClampedInitial(nextSpare);
    setVehicles(remaining);
    setP({
      ...p,
      vehicle: chosen,
      vehicles: remaining,
      active_vehicle_id: chosen?.id ?? null,
      spare_seats: nextSpare,
    });
  };

  const handleSaveModalVehicle = async (vData: Vehicle) => {
    if (vData.id != null) {
      const res = await put<Vehicle>(`/driver/vehicles/${vData.id}`, vData);
      const updated = vehicles.map((vh) =>
        vh.id === vData.id
          ? { ...vh, ...res }
          : vData.is_active
            ? { ...vh, is_active: false }
            : vh,
      );
      const activeV = updated.find((vh) => vh.is_active) || updated[0] || null;
      const nextSpare = clampForVehicle(p.spare_seats, activeV);
      syncClampedInitial(nextSpare);
      setVehicles(updated);
      setP({ ...p, vehicle: activeV, vehicles: updated, active_vehicle_id: activeV?.id ?? null, spare_seats: nextSpare });
      toast(t('vehicleSaved'));
    } else if (modalVehicle != null) {
      const prefs = normalizePrefs(p);
      await put('/driver/profile', {
        ...prefs,
        active_vehicle_id: p.active_vehicle_id ?? undefined,
        vehicle: { ...vData, seats: Number(vData.seats) },
      });
      const updated = vehicles.map((vh) =>
        vh.plate === modalVehicle.plate ? { ...vh, ...vData, is_active: true } : vh,
      );
      const activeV = updated.find((vh) => vh.is_active) || updated[0] || null;
      const nextSpare = clampForVehicle(p.spare_seats, activeV);
      syncClampedInitial(nextSpare);
      setVehicles(updated);
      setP({ ...p, vehicle: activeV, vehicles: updated, spare_seats: nextSpare });
      toast(t('vehicleSaved'));
    } else {
      const res = await post<Vehicle>('/driver/vehicles', vData);
      const exists = vehicles.some((vh) => vh.id === res.id);
      const updated: Vehicle[] = exists
        ? vehicles.map((vh) =>
            vh.id === res.id
              ? { ...vh, ...res }
              : res.is_active
                ? { ...vh, is_active: false }
                : vh,
          )
        : res.is_active
          ? [...vehicles.map((vh) => ({ ...vh, is_active: false })), res]
          : [...vehicles, res];
      const activeV = updated.find((vh) => vh.is_active) || updated[0] || null;
      const nextSpare = clampForVehicle(p.spare_seats, activeV);
      syncClampedInitial(nextSpare);
      setVehicles(updated);
      setP({ ...p, vehicle: activeV, vehicles: updated, active_vehicle_id: activeV?.id ?? null, spare_seats: nextSpare });
      toast(t('vehicleSaved'));
    }
  };

  const save = async (e: FormEvent) => {
    e.preventDefault();
    if (!p) return;
    const prefsPayload = normalizePrefs({
      ...p,
      spare_seats: clampForVehicle(Number(p.spare_seats) || 1, v),
    });
    const ok = await run(
      () =>
        put('/driver/profile', {
          ...prefsPayload,
          active_vehicle_id: v?.id ?? undefined,
        }),
      t('settingsSaved'),
    );
    if (ok) {
      setP((cur) => (cur ? { ...cur, spare_seats: prefsPayload.spare_seats } : cur));
      initialRef.current = JSON.stringify(prefsPayload);
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
              <Field
                label={t('spare')}
                hint={
                  v ? (
                    <>
                      {t('spareMaxHint', { max: maxSpareSeats, seats: v.seats })} {t('spareHint')}
                    </>
                  ) : (
                    t('spareHint')
                  )
                }
              >
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
                    disabled={p.spare_seats >= maxSpareSeats}
                    onClick={() =>
                      setP({ ...p, spare_seats: Math.min(maxSpareSeats, p.spare_seats + 1) })
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
            {/* Card 3: Vehicles configuration & selection */}
            <Card className="stack">
              <div className="section-header-row">
                <div>
                  <h3>{t('vehicles')}</h3>
                  <p className="muted small">{t('vehiclesHint')}</p>
                </div>
                {can('drive', 'update') && vehicles.length > 0 && (
                  <Button
                    type="button"
                    variant="secondary"
                    className="small"
                    onClick={() => setModalVehicle(null)}
                  >
                    <Plus size={16} /> {t('addVehicle')}
                  </Button>
                )}
              </div>

              {vehicles.length === 0 || !v ? (
                <div className="plate-preview-box">
                  <p className="muted small">{t('noVehicles')}</p>
                  {can('drive', 'update') && (
                    <Button
                      type="button"
                      variant="primary"
                      size="sm"
                      onClick={() => setModalVehicle(null)}
                    >
                      <Plus size={16} /> {t('addVehicle')}
                    </Button>
                  )}
                </div>
              ) : (
                <>
                  {/* Active vehicle live plate-badge preview */}
                  <div className="plate-preview-box">
                    <div className="vehicle-active-tag">
                      <Check size={14} /> {t('selectedForDriving')}
                    </div>
                    <div className="plate">{v.plate.trim() || 'VXX 1234'}</div>
                    <div className="plate-preview-desc">
                      {v.color.trim() || t('colorPlaceholder')} ·{' '}
                      {v.model.trim() || t('model')} ({v.seats} {t('seats')})
                    </div>
                    <div className="vehicle-badges-row">
                      {v.wheelchair_ok ? <span className="chip on small">{t('wheelchairOk')}</span> : null}
                      {v.low_step ? <span className="chip on small">{t('lowStep')}</span> : null}
                      {v.child_seat ? <span className="chip on small">{t('childSeat')}</span> : null}
                    </div>
                  </div>

                  {/* Unified vehicle list (shown for 1 or more vehicles) */}
                  <div className="vehicle-list">
                    {vehicles.map((item) => {
                      const isActive =
                        item.id != null && v.id != null
                          ? item.id === v.id
                          : item.plate === v.plate;
                      return (
                        <div
                          key={item.id ?? item.plate}
                          className={`vehicle-item-row ${isActive ? 'active' : ''}`}
                        >
                          <div className="vehicle-item-info">
                            <span className="plate small">{item.plate}</span>
                            <span className="vehicle-item-name">
                              {item.color} {item.model} ({item.seats} {t('seats')})
                            </span>
                            {isActive && (
                              <span className="pill green small">
                                <Check size={12} /> {t('selectedForDriving')}
                              </span>
                            )}
                          </div>
                          {can('drive', 'update') && (
                            <div className="vehicle-item-actions">
                              {!isActive && (
                                <Button
                                  type="button"
                                  variant="secondary"
                                  className="small"
                                  onClick={() => selectVehicle(item)}
                                >
                                  {t('selectForDriving')}
                                </Button>
                              )}
                              <IconButton
                                label={t('editVehicle')}
                                icon={<Edit2 size={15} />}
                                onClick={() => setModalVehicle(item)}
                              />
                              <IconButton
                                label={t('deleteVehicle')}
                                icon={<Trash2 size={15} />}
                                onClick={() => deleteVehicle(item)}
                              />
                            </div>
                          )}
                        </div>
                      );
                    })}
                  </div>
                  <p className="muted tiny">{t('vehiclesSaveInstant')}</p>
                </>
              )}
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

      {modalVehicle !== undefined && (
        <VehicleModal
          vehicle={modalVehicle}
          onClose={() => setModalVehicle(undefined)}
          onSave={handleSaveModalVehicle}
        />
      )}
    </div>
  );
}
