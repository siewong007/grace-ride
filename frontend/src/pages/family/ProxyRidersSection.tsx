import { FormEvent, useRef, useState } from 'react';
import { Pencil, Plus, Trash2, Users } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { del, post, put, useApi } from '../../api';
import { inferAreaId } from '../../areaMatch';
import { AddressInput } from '../../components/AddressInput';
import {
  Button,
  EmptyState,
  ErrorBox,
  Field,
  IconButton,
  ListSkeleton,
  Modal,
  Pill,
  useAction,
  useConfirm,
} from '../../components/ui';
import type { ProxyRider } from '../../types/domain';

interface AreaOpt {
  id: number;
  name: string;
  active: boolean;
}

const MAX_PROXY_RIDERS = 50;

const REL_OPTIONS = [
  { value: 'parent', key: 'relParent' },
  { value: 'cell_member', key: 'relCellMember' },
  { value: 'spouse', key: 'relSpouse' },
  { value: 'friend', key: 'relFriend' },
  { value: 'other', key: 'relOther' },
] as const;

export default function ProxyRidersSection({
  embedded = false,
  onChanged,
}: {
  embedded?: boolean;
  onChanged?: () => void | Promise<unknown>;
}) {
  const { t } = useTranslation('profile');
  const { data: ridersRaw, loading, reload } = useApi<ProxyRider[]>('/proxy-riders');
  const { data: areasRaw } = useApi<AreaOpt[]>('/areas');
  const riders = Array.isArray(ridersRaw) ? ridersRaw : [];
  const areas = (Array.isArray(areasRaw) ? areasRaw : []).filter((a) => a.active);
  const areaNames = new Map(areas.map((a) => [a.id, a.name]));

  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<ProxyRider | null>(null);

  const openCreate = () => {
    setEditing(null);
    setModalOpen(true);
  };

  const openEdit = (rider: ProxyRider) => {
    setEditing(rider);
    setModalOpen(true);
  };

  const handleChanged = async () => {
    await reload();
    await onChanged?.();
  };

  const relLabel = (value: string) => {
    const opt = REL_OPTIONS.find((r) => r.value === value);
    return opt ? t(opt.key) : value;
  };

  const canAdd = riders.length < MAX_PROXY_RIDERS;

  return (
    <div className="stack">
      <div className="row between items-center gap-sm wrap">
        <div>
          {!embedded && <h2>{t('proxyRidersTitle')}</h2>}
          <p className="muted small">{t('proxyRidersDesc')}</p>
        </div>
        {riders.length > 0 && canAdd && (
          <Button variant="primary" size="sm" icon={<Plus size={16} aria-hidden="true" />} onClick={openCreate}>
            {t('addProxyRider')}
          </Button>
        )}
      </div>

      {loading && riders.length === 0 ? (
        <ListSkeleton count={2} />
      ) : riders.length === 0 ? (
        <EmptyState
          icon={<Users size={24} aria-hidden="true" />}
          title={t('proxyRidersEmptyTitle')}
          description={t('proxyRidersEmptyDesc')}
          action={
            <Button variant="primary" icon={<Plus size={16} aria-hidden="true" />} onClick={openCreate}>
              {t('addProxyRider')}
            </Button>
          }
        />
      ) : (
        <div className="list saved-addr-list" role="list">
          {riders.map((r) => (
            <div key={r.id} className="saved-addr-row" role="listitem">
              <div className="saved-addr-info">
                <div className="saved-addr-head">
                  <span className="saved-addr-label">{r.name}</span>
                  {r.relationship && <Pill tone="neutral">{relLabel(r.relationship)}</Pill>}
                  {r.area_id && areaNames.get(r.area_id) && (
                    <span className="saved-addr-area">· {areaNames.get(r.area_id)}</span>
                  )}
                </div>
                {r.phone && <span className="saved-addr-notes">{r.phone}</span>}
                {r.pickup_address && <span className="saved-addr-street">{r.pickup_address}</span>}
                {(r.gate_notes || r.notes) && (
                  <span className="saved-addr-notes">{[r.gate_notes, r.notes].filter(Boolean).join(' · ')}</span>
                )}
              </div>
              <div className="saved-addr-actions">
                <IconButton
                  variant="ghost"
                  size="sm"
                  label={t('editAddressAria', { label: r.name })}
                  onClick={() => openEdit(r)}
                  icon={<Pencil size={16} aria-hidden="true" />}
                />
              </div>
            </div>
          ))}
        </div>
      )}

      {modalOpen && (
        <ProxyRiderModal
          key={editing ? `proxy-${editing.id}` : 'proxy-new'}
          editing={editing}
          areas={areas}
          onClose={() => {
            setModalOpen(false);
            setEditing(null);
          }}
          onChanged={handleChanged}
        />
      )}
    </div>
  );
}

function ProxyRiderModal({
  editing,
  areas,
  onClose,
  onChanged,
}: {
  editing: ProxyRider | null;
  areas: AreaOpt[];
  onClose: () => void;
  onChanged: () => void | Promise<unknown>;
}) {
  const { t } = useTranslation('profile');
  const confirm = useConfirm();
  const nameRef = useRef<HTMLInputElement>(null);
  const { busy, error, run } = useAction();

  const [name, setName] = useState(editing?.name ?? '');
  const [relationship, setRelationship] = useState(editing?.relationship || 'parent');
  const [phone, setPhone] = useState(editing?.phone ?? '');
  const [pickupAddress, setPickupAddress] = useState(editing?.pickup_address ?? '');
  const [areaId, setAreaId] = useState(editing?.area_id ? String(editing.area_id) : '');
  const [gateNotes, setGateNotes] = useState(editing?.gate_notes ?? '');
  const [notes, setNotes] = useState(editing?.notes ?? '');

  const canSave = !busy && name.trim().length > 0;

  const saveRider = () => {
    if (!canSave) return;
    void run(async () => {
      const payload = {
        name: name.trim(),
        phone: phone.trim(),
        relationship: relationship.trim(),
        pickup_address: pickupAddress.trim(),
        area_id: areaId ? Number(areaId) : null,
        gate_notes: gateNotes.trim(),
        notes: notes.trim(),
      };
      if (editing) {
        await put(`/proxy-riders/${editing.id}`, payload);
      } else {
        await post('/proxy-riders', payload);
      }
      await onChanged();
      onClose();
    }, t('proxyRiderSaved'));
  };

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    saveRider();
  };

  const handleDelete = async () => {
    if (!editing || busy) return;
    const ok = await confirm({
      title: t('remove'),
      body: t('removeProxyConfirm'),
      confirmLabel: t('remove'),
      tone: 'danger',
    });
    if (ok) {
      await run(async () => {
        await del(`/proxy-riders/${editing.id}`);
        await onChanged();
        onClose();
      }, t('removed'));
    }
  };

  return (
    <Modal
      title={editing ? t('editProxyRider') : t('addProxyRider')}
      onClose={onClose}
      initialFocusRef={nameRef}
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
            <Button variant="primary" loading={busy} disabled={!canSave} onClick={saveRider}>
              {t('save')}
            </Button>
          </div>
        </div>
      }
    >
      <form className="stack" onSubmit={handleSubmit}>
        <Field label={t('proxyName')}>
          <input
            ref={nameRef}
            className="input"
            required
            maxLength={80}
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={t('proxyNamePlaceholder')}
          />
        </Field>

        <div className="form-grid two">
          <div className="field">
            <label htmlFor="proxy-rel-select" className="field-label">{t('proxyRelationship')}</label>
            <select
              id="proxy-rel-select"
              className="input"
              value={relationship}
              onChange={(e) => setRelationship(e.target.value)}
            >
              {REL_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>{t(opt.key)}</option>
              ))}
            </select>
          </div>

          <Field label={t('proxyPhone')}>
            <input
              className="input"
              type="tel"
              value={phone}
              onChange={(e) => setPhone(e.target.value)}
              placeholder={t('phonePlaceholder')}
            />
          </Field>
        </div>

        <AddressInput
          id="proxy-pickup-address"
          label={t('placeAddress')}
          maxLength={200}
          value={pickupAddress}
          onChange={(v) => {
            setPickupAddress(v);
            if (!areaId && v.trim().length >= 3) {
              const matched = inferAreaId(v, areas);
              if (matched) setAreaId(String(matched));
            }
          }}
          onSelectPlace={(place) => {
            setPickupAddress(place.label);
            if (!areaId) {
              const matched = inferAreaId(place.label, areas);
              if (matched) setAreaId(String(matched));
            }
          }}
          placeholder={t('placeAddressPlaceholder')}
        />

        <div className="field">
          <label htmlFor="proxy-area-select" className="field-label">{t('placeArea')}</label>
          <select
            id="proxy-area-select"
            className="input"
            value={areaId}
            onChange={(e) => setAreaId(e.target.value)}
          >
            <option value="">—</option>
            {areas.map((a) => (
              <option key={a.id} value={a.id}>
                {a.name}
              </option>
            ))}
          </select>
        </div>

        <Field label={t('placeGateNotes')}>
          <input
            className="input"
            maxLength={300}
            value={gateNotes}
            onChange={(e) => setGateNotes(e.target.value)}
            placeholder={t('placeGatePlaceholder')}
          />
        </Field>

        <Field label={t('proxyNotes')}>
          <input
            className="input"
            maxLength={500}
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            placeholder={t('proxyNotesPlaceholder')}
          />
        </Field>

        <ErrorBox error={error} />
      </form>
    </Modal>
  );
}
