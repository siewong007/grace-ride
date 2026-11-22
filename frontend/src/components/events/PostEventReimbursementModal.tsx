import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Check, UtensilsCrossed } from 'lucide-react';
import { post, useApi } from '../../api';
import { Button, ErrorBox, Modal, Spinner, useAction, useToast } from '../ui';
import { rm } from '../../format';

export interface AreaFareOption {
  id: number;
  name: string;
  fare_cents: number | null;
}

export interface DriverReturnPassengerStats {
  total: number;
  supper: number;
  direct_home: number;
  unspecified: number;
}

export interface PostEventDriverItem {
  id: number;
  name: string;
  phone: string;
  vehicle: string | null;
  default_area: AreaFareOption | null;
  passenger_stats: DriverReturnPassengerStats;
  suggested_trip_type: 'supper' | 'direct_return';
  already_claimed: boolean;
  claimed_amount_cents: number | null;
  claimed_status: string | null;
}

export interface PostEventDriversResponse {
  event_id: number;
  event_title: string;
  event_area: AreaFareOption | null;
  available_areas: AreaFareOption[];
  drivers: PostEventDriverItem[];
}

interface DriverRowState {
  selected: boolean;
  tripType: 'supper' | 'direct_return';
  destAreaId: number | '';
  amountCents: number;
  note: string;
}

export default function PostEventReimbursementModal({
  eventId,
  onClose,
  onSuccess,
}: {
  eventId: number;
  onClose: () => void;
  onSuccess?: () => void;
}) {
  const { t } = useTranslation('events');
  const toast = useToast();
  const { busy, error, run } = useAction();
  const { data, loading } = useApi<PostEventDriversResponse>(
    `/events/${eventId}/post-event-drivers`
  );

  const [supperAreaId, setSupperAreaId] = useState<number | ''>('');
  const [supperNote, setSupperNote] = useState('');
  const [rows, setRows] = useState<Record<number, DriverRowState>>({});

  // Initialize driver rows once data loads
  useEffect(() => {
    if (!data?.drivers) return;
    const initialRows: Record<number, DriverRowState> = {};
    for (const d of data.drivers) {
      const isSupper = d.suggested_trip_type === 'supper';
      const destAreaId = d.default_area?.id || '';

      const matchedArea = data.available_areas.find((a) => a.id === destAreaId);
      const amountCents = matchedArea?.fare_cents ?? 0;

      initialRows[d.id] = {
        selected: !d.already_claimed,
        tripType: isSupper ? 'supper' : 'direct_return',
        destAreaId,
        amountCents,
        note: '',
      };
    }
    setRows(initialRows);
  }, [data]);

  // When global supper area is changed, update drivers with tripType === 'supper'
  const handleSupperAreaChange = (newAreaId: number | '') => {
    setSupperAreaId(newAreaId);
    if (!data) return;

    setRows((prev) => {
      const next = { ...prev };
      const matchedArea = data.available_areas.find((a) => a.id === newAreaId);
      const fare = matchedArea?.fare_cents ?? 0;

      for (const [didStr, row] of Object.entries(next)) {
        if (row.tripType === 'supper') {
          next[Number(didStr)] = {
            ...row,
            destAreaId: newAreaId,
            amountCents: fare,
          };
        }
      }
      return next;
    });
  };

  const handleTripTypeChange = (driverId: number, type: 'supper' | 'direct_return') => {
    if (!data) return;
    const driver = data.drivers.find((d) => d.id === driverId);
    let targetAreaId: number | '' = '';

    if (type === 'supper') {
      targetAreaId = supperAreaId || driver?.default_area?.id || '';
    } else {
      targetAreaId = driver?.default_area?.id || '';
    }

    const matchedArea = data.available_areas.find((a) => a.id === targetAreaId);
    const fare = matchedArea?.fare_cents ?? 0;

    setRows((prev) => ({
      ...prev,
      [driverId]: {
        ...prev[driverId],
        tripType: type,
        destAreaId: targetAreaId,
        amountCents: fare,
      },
    }));
  };

  const handleDestAreaChange = (driverId: number, areaId: number | '') => {
    if (!data) return;
    const matchedArea = data.available_areas.find((a) => a.id === areaId);
    const fare = matchedArea?.fare_cents ?? 0;

    setRows((prev) => ({
      ...prev,
      [driverId]: {
        ...prev[driverId],
        destAreaId: areaId,
        amountCents: fare,
      },
    }));
  };

  const handleRowSelectToggle = (driverId: number) => {
    setRows((prev) => ({
      ...prev,
      [driverId]: {
        ...prev[driverId],
        selected: !prev[driverId]?.selected,
      },
    }));
  };

  const handleAmountChange = (driverId: number, cents: number) => {
    setRows((prev) => ({
      ...prev,
      [driverId]: {
        ...prev[driverId],
        amountCents: Math.max(0, cents),
      },
    }));
  };

  const selectedCount = Object.values(rows).filter((r) => r.selected).length;
  const totalCents = Object.values(rows)
    .filter((r) => r.selected)
    .reduce((sum, r) => sum + r.amountCents, 0);

  const handleSubmit = () => {
    if (!data) return;
    const claims = Object.entries(rows)
      .filter(([_, r]) => r.selected && r.destAreaId !== '')
      .map(([did, r]) => ({
        driver_id: Number(did),
        trip_type: r.tripType,
        destination_area_id: Number(r.destAreaId),
        amount_cents: r.amountCents,
        note: r.note.trim() || undefined,
      }));

    if (claims.length === 0) {
      toast.error(t('postEventReimbNoSelection'));
      return;
    }

    run(async () => {
      await post(`/events/${eventId}/post-event-reimbursements`, {
        supper_area_id: supperAreaId || undefined,
        supper_note: supperNote.trim() || undefined,
        claims,
      });
      toast.success(
        t('postEventReimbSuccess', {
          count: claims.length,
          total: rm(totalCents),
        })
      );
      onSuccess?.();
      onClose();
    });
  };

  return (
    <Modal
      title={t('postEventReimbTitle')}
      description={data ? `${data.event_title} · ${data.event_area?.name || t('noAreaSet')}` : undefined}
      size="lg"
      onClose={onClose}
      footer={
        <div className="row between items-center wrap gap-sm grow">
          <div className="small">
            <span className="bold">
              {t('postEventReimbSummary', {
                count: selectedCount,
                total: rm(totalCents),
              })}
            </span>
            <span className="muted tiny block">
              {t('postEventReimbDirectApprovedHint')}
            </span>
          </div>
          <div className="row gap-xs">
            <Button variant="ghost" onClick={onClose} disabled={busy}>
              {t('cancel', { ns: 'common' })}
            </Button>
            <Button
              variant="primary"
              disabled={busy || selectedCount === 0}
              onClick={handleSubmit}
            >
              {t('postEventReimbBatchSubmitBtn', {
                amount: rm(totalCents),
              })}
            </Button>
          </div>
        </div>
      }
    >
      <div className="stack">
        <ErrorBox error={error} />

        {loading ? (
          <Spinner />
        ) : !data || data.drivers.length === 0 ? (
          <p className="muted small">{t('postEventNoDriversFound')}</p>
        ) : (
          <>
            {/* Global Supper Setting Card */}
            <div className="card flat tight stack-xs" style={{ background: 'var(--surface-2)' }}>
              <div className="bold small row items-center gap-xs">
                <UtensilsCrossed size={16} />
                <span>{t('defaultSupperSettingTitle')}</span>
              </div>
              <p className="tiny muted no-margin">
                {t('defaultSupperSettingHint')}
              </p>
              <div className="row wrap gap-sm mt-xs">
                <div className="grow min-w-0" style={{ flex: '1 1 180px' }}>
                  <label className="tiny muted block mb-xs">{t('supperAreaLabel')}</label>
                  <select
                    className="input w-full"
                    value={supperAreaId}
                    onChange={(e) => handleSupperAreaChange(e.target.value ? Number(e.target.value) : '')}
                  >
                    <option value="">{t('selectSupperArea')}</option>
                    {data.available_areas.map((a) => (
                      <option key={a.id} value={a.id}>
                        {a.name} {a.fare_cents != null ? `(${rm(a.fare_cents)})` : `(${t('noFareSet')})`}
                      </option>
                    ))}
                  </select>
                </div>
                <div className="grow min-w-0" style={{ flex: '2 1 240px' }}>
                  <label className="tiny muted block mb-xs">{t('supperPlaceNoteLabel')}</label>
                  <input
                    type="text"
                    className="input w-full"
                    placeholder={t('supperPlacePlaceholder')}
                    value={supperNote}
                    onChange={(e) => setSupperNote(e.target.value)}
                  />
                </div>
              </div>
            </div>

            {/* Drivers List */}
            <div className="stack-xs">
              <h4 className="no-margin">{t('driverRosterTitle', { count: data.drivers.length })}</h4>
              <div className="list">
                {data.drivers.map((d) => {
                  const row = rows[d.id];
                  if (!row) return null;

                  return (
                    <div
                      key={d.id}
                      className={`item stack-xs ${d.already_claimed ? 'opacity-60' : ''}`}
                      style={{ padding: '0.75rem', borderBottom: '1px solid var(--border)' }}
                    >
                      <div className="row between items-center wrap gap-xs">
                        <div className="row items-center gap-xs">
                          <input
                            type="checkbox"
                            checked={row.selected}
                            disabled={d.already_claimed || busy}
                            onChange={() => handleRowSelectToggle(d.id)}
                            style={{ cursor: d.already_claimed ? 'not-allowed' : 'pointer' }}
                          />
                          <div>
                            <span className="bold">{d.name}</span>
                            {d.vehicle && <span className="muted tiny ml-xs">({d.vehicle})</span>}
                          </div>
                        </div>

                        {/* Passenger Intent Badges */}
                        <div className="row items-center gap-xs tiny">
                          {d.passenger_stats.total > 0 ? (
                            <>
                              {d.passenger_stats.supper > 0 && (
                                <span className="pill pill-success" title={t('passengerIntentSupperTooltip')}>
                                  {d.passenger_stats.supper} {t('supperCountLabel')}
                                </span>
                              )}
                              {d.passenger_stats.direct_home > 0 && (
                                <span className="pill pill-muted" title={t('passengerIntentHomeTooltip')}>
                                  {d.passenger_stats.direct_home} {t('directHomeCountLabel')}
                                </span>
                              )}
                              {d.passenger_stats.unspecified > 0 && (
                                <span className="pill pill-warning" title={t('passengerIntentPendingTooltip')}>
                                  {d.passenger_stats.unspecified} {t('unspecifiedCountLabel')}
                                </span>
                              )}
                            </>
                          ) : (
                            <span className="muted tiny">{t('noReturnPassengerAssigned')}</span>
                          )}

                          {d.already_claimed && (
                            <span className="pill pill-success row nowrap gap-xs">
                              <Check size={12} aria-hidden="true" />
                              <span>{t('alreadyClaimedPill', { amount: rm(d.claimed_amount_cents || 0) })}</span>
                            </span>
                          )}
                        </div>
                      </div>

                      {/* Row Controls */}
                      {!d.already_claimed && (
                        <div className="row between items-center wrap gap-sm pt-xs">
                          {/* Trip Type Toggle */}
                          <div className="row gap-xs">
                            <Button
                              type="button"
                              size="sm"
                              variant={row.tripType === 'supper' ? 'primary' : 'secondary'}
                              onClick={() => handleTripTypeChange(d.id, 'supper')}
                              disabled={!row.selected || busy}
                            >
                              {t('tripSupperBtn')}
                            </Button>
                            <Button
                              type="button"
                              size="sm"
                              variant={row.tripType === 'direct_return' ? 'primary' : 'secondary'}
                              onClick={() => handleTripTypeChange(d.id, 'direct_return')}
                              disabled={!row.selected || busy}
                            >
                              {t('tripDirectHomeBtn')}
                            </Button>
                          </div>

                          {/* Destination Area Select */}
                          <div className="row items-center gap-xs grow min-w-0" style={{ maxWidth: '280px' }}>
                            <select
                              className="input w-full"
                              value={row.destAreaId}
                              disabled={!row.selected || busy}
                              onChange={(e) => handleDestAreaChange(d.id, e.target.value ? Number(e.target.value) : '')}
                            >
                              <option value="">{t('selectDestArea')}</option>
                              {data.available_areas.map((a) => (
                                <option key={a.id} value={a.id}>
                                  {a.name} {a.fare_cents != null ? `(${rm(a.fare_cents)})` : `(${t('noFare')})`}
                                </option>
                              ))}
                            </select>
                          </div>

                          {/* Fare Display / Editable Input */}
                          <div className="row items-center gap-xs">
                            <span className="tiny muted">RM</span>
                            <input
                              type="number"
                              step="0.5"
                              min="0"
                              max="500"
                              className="input text-right"
                              style={{ width: '85px' }}
                              disabled={!row.selected || busy}
                              value={(row.amountCents / 100).toFixed(2)}
                              onChange={(e) => handleAmountChange(d.id, Math.round(parseFloat(e.target.value || '0') * 100))}
                            />
                          </div>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          </>
        )}
      </div>
    </Modal>
  );
}
