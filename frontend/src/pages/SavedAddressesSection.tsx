import { FormEvent, lazy, Suspense, useRef, useState } from 'react';
import { LocateFixed, MapPin, Pencil, Plus, Star, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { del, post, put, useApi } from '../api';
import { inferAreaId } from '../areaMatch';
import { AddressInput, type Place } from '../components/AddressInput';

const LocationPinPicker = lazy(() => import('../components/LocationPinPicker'));
import {
  Button,
  EmptyState,
  ErrorBox,
  Field,
  IconButton,
  ListSkeleton,
  Modal,
  PageHeader,
  Pill,
  useAction,
  useConfirm,
} from '../components/ui';
import type { SavedPlace } from '../types/domain';

interface AreaOpt {
  id: number;
  name: string;
  active: boolean;
}

interface PlaceFormState {
  label: string;
  area_id: string;
  address: string;
  gate_notes: string;
  is_default: boolean;
  lat: number | null;
  lng: number | null;
}

const MAX_SAVED_PLACES = 50;

export default function SavedAddressesSection({
  places,
  loading,
  reloadPlaces,
  embedded = false,
}: {
  places: SavedPlace[];
  loading: boolean;
  reloadPlaces: () => void | Promise<unknown>;
  embedded?: boolean;
}) {
  const { t } = useTranslation('profile');
  const { data: areasRaw } = useApi<AreaOpt[]>('/areas');
  const areas = (Array.isArray(areasRaw) ? areasRaw : []).filter((a) => a.active);
  const { busy, error, run } = useAction();

  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<SavedPlace | null>(null);

  const openCreate = () => {
    setEditing(null);
    setModalOpen(true);
  };

  const openEdit = (sp: SavedPlace) => {
    setEditing(sp);
    setModalOpen(true);
  };

  const closeModal = () => {
    setModalOpen(false);
    setEditing(null);
  };

  const setDefault = (sp: SavedPlace) => {
    if (sp.is_default === 1 || busy) return;
    void run(async () => {
      await put(`/saved-places/${sp.id}`, {
        label: sp.label,
        area_id: sp.area_id,
        address: sp.address,
        gate_notes: sp.gate_notes ?? '',
        is_default: true,
      });
      await reloadPlaces();
    }, t('defaultAddressSet'));
  };

  const canAdd = places.length < MAX_SAVED_PLACES;

  return (
    <div className="stack">
      {!embedded ? (
        <PageHeader
          title={t('savedAddresses')}
          subtitle={places.length > 0 ? t('savedAddressCount', { count: places.length }) : undefined}
          back="/me"
          actions={
            places.length > 0 && canAdd ? (
              <Button variant="primary" size="sm" icon={<Plus size={16} aria-hidden="true" />} onClick={openCreate}>
                {t('addAddress')}
              </Button>
            ) : undefined
          }
        />
      ) : (
        places.length > 0 && (
          <div className="row between items-center">
            <span className="muted small">{t('savedAddressCount', { count: places.length })}</span>
            {canAdd && (
              <Button variant="primary" size="sm" icon={<Plus size={16} aria-hidden="true" />} onClick={openCreate}>
                {t('addAddress')}
              </Button>
            )}
          </div>
        )
      )}

      {loading && places.length === 0 ? (
        <ListSkeleton count={2} />
      ) : places.length === 0 ? (
        <EmptyState
          icon={<MapPin size={24} aria-hidden="true" />}
          title={t('savedAddressesEmptyTitle')}
          description={t('savedAddressesEmptyDesc')}
          action={
            <Button variant="primary" icon={<Plus size={16} aria-hidden="true" />} onClick={openCreate}>
              {t('addAddress')}
            </Button>
          }
        />
      ) : (
        <div className="list saved-addr-list" role="list">
          {places.map((sp) => {
            const isDef = sp.is_default === 1;
            return (
              <div key={sp.id} className="saved-addr-row" role="listitem">
                <MapPin
                  size={18}
                  className={isDef ? 'saved-addr-pin is-default' : 'saved-addr-pin'}
                  aria-hidden="true"
                />
                <div className="saved-addr-info">
                  <div className="saved-addr-head">
                    <span className="saved-addr-label">{sp.label}</span>
                    {isDef && <Pill tone="primary">{t('defaultAddress')}</Pill>}
                    {sp.area_name && <span className="saved-addr-area">· {sp.area_name}</span>}
                  </div>
                  <span className="saved-addr-street">{sp.address}</span>
                  {sp.gate_notes && <span className="saved-addr-notes">{sp.gate_notes}</span>}
                  <div className="saved-addr-meta">
                    {!sp.area_id && <Pill tone="warning">{t('placeAreaMissing')}</Pill>}
                    <span className={sp.lat != null && sp.lng != null ? 'place-pin-status is-pinned' : 'place-pin-status'}>
                      {sp.lat != null && sp.lng != null ? t('placePinned') : t('placeNoPin')}
                    </span>
                  </div>
                </div>
                <div className="saved-addr-actions">
                  <IconButton
                    variant="ghost"
                    size="sm"
                    className={isDef ? 'saved-addr-star is-default' : 'saved-addr-star'}
                    label={isDef ? t('defaultAddress') : t('setDefaultAddress', { label: sp.label })}
                    aria-pressed={isDef}
                    disabled={busy || isDef}
                    onClick={() => setDefault(sp)}
                    icon={<Star size={16} fill={isDef ? 'currentColor' : 'none'} aria-hidden="true" />}
                  />
                  <IconButton
                    variant="ghost"
                    size="sm"
                    label={t('editAddressAria', { label: sp.label })}
                    disabled={busy}
                    onClick={() => openEdit(sp)}
                    icon={<Pencil size={16} aria-hidden="true" />}
                  />
                </div>
              </div>
            );
          })}
        </div>
      )}

      {places.length >= MAX_SAVED_PLACES && (
        <p className="muted small">{t('savedAddressLimitReached')}</p>
      )}

      <ErrorBox error={error} />

      {modalOpen && (
        <SavedAddressModal
          key={editing ? `edit-${editing.id}` : 'new'}
          editing={editing}
          areas={areas}
          defaultCheckedInitial={editing ? editing.is_default === 1 : places.length === 0}
          onClose={closeModal}
          onChanged={reloadPlaces}
        />
      )}
    </div>
  );
}

function SavedAddressModal({
  editing,
  areas,
  defaultCheckedInitial,
  onClose,
  onChanged,
}: {
  editing: SavedPlace | null;
  areas: AreaOpt[];
  defaultCheckedInitial: boolean;
  onClose: () => void;
  onChanged: () => void | Promise<unknown>;
}) {
  const { t } = useTranslation('profile');
  const confirm = useConfirm();
  const labelRef = useRef<HTMLInputElement>(null);
  const { busy, error, run } = useAction();

  const [form, setForm] = useState<PlaceFormState>(() => ({
    label: editing?.label ?? '',
    area_id: editing?.area_id ? String(editing.area_id) : '',
    address: editing?.address ?? '',
    gate_notes: editing?.gate_notes ?? '',
    is_default: defaultCheckedInitial,
    lat: editing?.lat ?? null,
    lng: editing?.lng ?? null,
  }));
  const [gpsStatus, setGpsStatus] = useState<'idle' | 'locating' | 'captured' | 'denied'>('idle');
  const [showMap, setShowMap] = useState(false);
  const hasGeolocation =
    typeof navigator !== 'undefined' &&
    'geolocation' in navigator &&
    typeof navigator.geolocation?.getCurrentPosition === 'function';

  const presets = [t('placePresetHome'), t('placePresetWork'), t('placePresetParents')];
  const canSave = !busy && form.label.trim().length > 0 && form.address.trim().length > 0;

  const captureGps = () => {
    if (!hasGeolocation) return;
    setGpsStatus('locating');
    navigator.geolocation.getCurrentPosition(
      (pos) => {
        setForm((cur) => ({
          ...cur,
          lat: Number(pos.coords.latitude.toFixed(6)),
          lng: Number(pos.coords.longitude.toFixed(6)),
        }));
        setGpsStatus('captured');
      },
      () => setGpsStatus('denied'),
      { enableHighAccuracy: true, timeout: 10000 },
    );
  };

  const savePlace = () => {
    if (!canSave) return;
    void run(async () => {
      const payload = {
        label: form.label.trim(),
        area_id: form.area_id ? Number(form.area_id) : null,
        address: form.address.trim(),
        gate_notes: form.gate_notes.trim(),
        is_default: form.is_default,
        ...(form.lat != null && form.lng != null ? { lat: form.lat, lng: form.lng } : {}),
      };
      if (editing) {
        await put(`/saved-places/${editing.id}`, payload);
      } else {
        await post('/saved-places', payload);
      }
      await onChanged();
      onClose();
    }, t('addressSaved'));
  };

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    savePlace();
  };

  const handleDelete = async () => {
    if (!editing || busy) return;
    const ok = await confirm({
      title: t('remove'),
      body: t('removePlaceConfirm'),
      confirmLabel: t('remove'),
      tone: 'danger',
    });
    if (ok) {
      await run(async () => {
        await del(`/saved-places/${editing.id}`);
        await onChanged();
        onClose();
      }, t('removed'));
    }
  };

  return (
    <Modal
      title={editing ? t('editAddress') : t('addAddress')}
      onClose={onClose}
      initialFocusRef={labelRef}
      footer={
        <div className="saved-addr-modal-footer">
          {editing && (
            <Button variant="danger" size="sm" disabled={busy} onClick={handleDelete}>
              <Trash2 size={14} aria-hidden="true" />
              {t('remove')}
            </Button>
          )}
          <div className="saved-addr-modal-actions">
            <Button variant="ghost" disabled={busy} onClick={onClose}>
              {t('cancel')}
            </Button>
            <Button variant="primary" loading={busy} disabled={!canSave} onClick={savePlace}>
              {t('save')}
            </Button>
          </div>
        </div>
      }
    >
      <form id="saved-place-form" className="stack" onSubmit={handleSubmit}>
        <div className="chips">
          {presets.map((preset) => (
            <button
              key={preset}
              type="button"
              className={`chip ${form.label.trim() === preset ? 'on' : ''}`}
              onClick={() => setForm((cur) => ({ ...cur, label: preset }))}
            >
              {preset}
            </button>
          ))}
        </div>

        <Field label={t('placeLabel')}>
          <input
            ref={labelRef}
            className="input"
            required
            maxLength={40}
            value={form.label}
            onChange={(e) => setForm((cur) => ({ ...cur, label: e.target.value }))}
            placeholder={t('placeLabelPlaceholder')}
          />
        </Field>

        <AddressInput
          id="saved-place-address"
          label={t('placeAddress')}
          required
          maxLength={200}
          value={form.address}
          onChange={(v) => {
            setGpsStatus('idle');
            setForm((cur) => {
              const addrUnchanged = editing != null && v.trim() === editing.address.trim();
              const inferred = !cur.area_id && v.trim().length >= 3 ? inferAreaId(v, areas) : null;
              return {
                ...cur,
                address: v,
                lat: addrUnchanged ? (editing.lat ?? null) : null,
                lng: addrUnchanged ? (editing.lng ?? null) : null,
                area_id: cur.area_id || (inferred ? String(inferred) : ''),
              };
            });
          }}
          onSelectPlace={(place: Place) => {
            setGpsStatus('idle');
            setForm((cur) => ({
              ...cur,
              address: place.label,
              lat: place.lat,
              lng: place.lng,
              area_id: cur.area_id || String(inferAreaId(place.label, areas) ?? ''),
            }));
          }}
          placeholder={t('placeAddressPlaceholder')}
        />

        <div className="stack-xs">
          <div className="row wrap items-center gap-xs">
            {hasGeolocation && (
              <Button
                type="button"
                variant="secondary"
                size="sm"
                loading={gpsStatus === 'locating'}
                onClick={captureGps}
              >
                <LocateFixed size={14} aria-hidden="true" />
                {gpsStatus === 'locating' ? t('locatingGps') : t('useCurrentLocation')}
              </Button>
            )}
            <Button
              type="button"
              variant="secondary"
              size="sm"
              onClick={() => setShowMap((v) => !v)}
            >
              <MapPin size={14} aria-hidden="true" />
              {t(showMap ? 'hideMapPicker' : 'pickOnMap')}
            </Button>
            {gpsStatus === 'captured' && (
              <span className="place-pin-status is-pinned">{t('placeGpsCaptured')}</span>
            )}
            {gpsStatus === 'denied' && (
              <span className="muted tiny">{t('placeGpsDenied')}</span>
            )}
            {gpsStatus === 'idle' && form.lat != null && form.lng != null && (
              <span className="place-pin-status is-pinned">{t('placePinned')}</span>
            )}
          </div>
          {showMap && (
            <div className="stack-xs">
              <Suspense fallback={<ListSkeleton count={1} />}>
                <LocationPinPicker
                  lat={form.lat}
                  lng={form.lng}
                  onPick={(lat, lng) => {
                    setGpsStatus('idle');
                    setForm((cur) => ({ ...cur, lat, lng }));
                  }}
                />
              </Suspense>
              <small className="hint">{t('mapPickerHint')}</small>
            </div>
          )}
        </div>

        <div className="field">
          <label htmlFor="saved-place-area" className="field-label">{t('placeArea')}</label>
          <select
            id="saved-place-area"
            className="input"
            value={form.area_id}
            onChange={(e) => setForm((cur) => ({ ...cur, area_id: e.target.value }))}
          >
            <option value="">—</option>
            {areas.map((a) => (
              <option key={a.id} value={a.id}>
                {a.name}
              </option>
            ))}
          </select>
          <small className="hint">{t('placeAreaHint')}</small>
        </div>

        <Field label={t('placeGateNotes')}>
          <input
            className="input"
            maxLength={300}
            value={form.gate_notes}
            onChange={(e) => setForm((cur) => ({ ...cur, gate_notes: e.target.value }))}
            placeholder={t('placeGatePlaceholder')}
          />
        </Field>

        <label className={`check ${form.is_default ? 'on' : ''}`}>
          <input
            type="checkbox"
            checked={form.is_default}
            onChange={(e) => setForm((cur) => ({ ...cur, is_default: e.target.checked }))}
          />
          <span>{t('setAsDefault')}</span>
        </label>

        <ErrorBox error={error} />
      </form>
    </Modal>
  );
}
