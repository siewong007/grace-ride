// Transport planning board: arrange every ride request for one event at once.
// Drag a passenger onto a driver (desktop) or pick the driver from the card's menu / sheet (phones,
// keyboards, screen readers). Nothing is sent until "Confirm & notify".
import { DragEvent, useCallback, useEffect, useMemo, useState } from 'react';
import { useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Accessibility, Car, History, Repeat, RotateCcw, Send, Sparkles, Undo2, UserRound } from 'lucide-react';
import { ApiError, post, useApi } from '../../api';
import { useAuth } from '../../auth';
import {
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  IconButton,
  Modal,
  Page,
  PageHeader,
  Pill,
  StatusPill,
  useConfirm,
  useToast,
} from '../../components/ui';
import { fmtDateTime, fmtTime, RIDE_STATUS, useFormatT } from '../../format';
import {
  applyLastTime,
  applyMatchReturnDrivers,
  ArrangeResult,
  changesOf,
  Draft,
  draftEntries,
  driverLoad,
  Plan,
  PlanRide,
  Reason,
  inGroup,
  savedDraft,
  servesArea,
} from './planLogic';

const DRAG_TYPE = 'application/x-grace-ride';

export default function PlanTransport() {
  const { id } = useParams();
  const { t } = useTranslation('plan');
  const ft = useFormatT();
  const toast = useToast();
  const confirm = useConfirm();
  const { can } = useAuth();
  const canUpdate = can('admin_plan', 'update');
  const { data, error, loading, reload } = useApi<Plan>(`/admin/events/${id}/plan`);
  const [draft, setDraft] = useState<Draft>({});
  const [undoStack, setUndoStack] = useState<Draft[]>([]);
  const [filter, setFilter] = useState<string>('all');
  const [pickerRideId, setPickerRideId] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [dropTarget, setDropTarget] = useState<string | null>(null);
  // Auto-arrange's explanations, and who it couldn't fit; cleared when the board resets.
  const [reasons, setReasons] = useState<Record<number, Reason>>({});
  const [noRoom, setNoRoom] = useState<Set<number>>(new Set());
  const [arranging, setArranging] = useState(false);

  // A fresh board (first load, after sending, after a conflict) resets the draft.
  useEffect(() => {
    if (data) {
      setDraft(savedDraft(data));
      setUndoStack([]);
      setReasons({});
      setNoRoom(new Set());
    }
  }, [data]);

  const changes = useMemo(() => (data ? changesOf(data, draft) : []), [data, draft]);
  const dirty = changes.length > 0;

  useEffect(() => {
    if (!dirty) return;
    const warn = (e: BeforeUnloadEvent) => e.preventDefault();
    window.addEventListener('beforeunload', warn);
    return () => window.removeEventListener('beforeunload', warn);
  }, [dirty]);

  const undoStep = useCallback(() => {
    setUndoStack((stack) => {
      if (stack.length === 0) return stack;
      const prev = stack[stack.length - 1];
      setDraft(prev);
      return stack.slice(0, -1);
    });
  }, []);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!canUpdate || undoStack.length === 0) return;
      const tag = (e.target as HTMLElement | null)?.tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;
      if ((e.metaKey || e.ctrlKey) && !e.shiftKey && e.key.toLowerCase() === 'z') {
        e.preventDefault();
        undoStep();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [canUpdate, undoStack.length, undoStep]);

  if (loading && !data) {
    return (
      <Page>
        <CardSkeleton count={3} />
      </Page>
    );
  }
  if (error || !data) {
    return (
      <Page>
        <ErrorBox error={error} />
      </Page>
    );
  }

  const plan = data;
  const assign = (rideId: number, driverId: number | null) => {
    if (!canUpdate) return;
    if ((draft[rideId] ?? null) === driverId) return;
    setUndoStack((s) => [...s.slice(-24), draft]);
    setDraft((d) => ({ ...d, [rideId]: driverId }));
  };
  const loads = new Map(plan.drivers.map((d) => [d.id, driverLoad(plan, draft, d)]));
  const anyOver = plan.drivers.some((d) => loads.get(d.id)!.over);
  const unassigned = plan.rides.filter((r) => draft[r.id] == null);
  const placed = plan.rides.length - unassigned.length;
  const seatsUsed = plan.rides.filter((r) => draft[r.id] != null).reduce((n, r) => n + r.seats, 0);
  const assistanceCount = plan.rides.filter((r) => Boolean(r.assistance)).length;
  const uniqueAreas = Array.from(new Set(plan.rides.map((r) => r.area.name)));

  const matchesFilter = (r: PlanRide): boolean => {
    if (filter === 'unassigned') return draft[r.id] == null;
    if (filter === 'assistance') return Boolean(r.assistance);
    if (filter.startsWith('area:')) return r.area.name === filter.slice(5);
    return true;
  };

  const visibleUnassigned = unassigned.filter(matchesFilter);

  const onDrop = (target: number | null) => (e: DragEvent) => {
    e.preventDefault();
    setDropTarget(null);
    if (!canUpdate) return;
    const rideId = Number(e.dataTransfer.getData(DRAG_TYPE));
    if (rideId) assign(rideId, target);
  };
  const dropProps = (key: string, target: number | null) => ({
    onDragOver: (e: DragEvent) => {
      if (canUpdate && e.dataTransfer.types.includes(DRAG_TYPE)) {
        e.preventDefault();
        setDropTarget(key);
      }
    },
    onDragLeave: () => setDropTarget((k) => (k === key ? null : k)),
    onDrop: onDrop(target),
  });

  const autoArrange = async () => {
    if (!canUpdate) return;
    setArranging(true);
    try {
      const res = await post<ArrangeResult>(`/admin/events/${plan.event.id}/plan/arrange`, {
        draft: draftEntries(plan, draft),
      });
      if (res.placed.length > 0) {
        setUndoStack((s) => [...s.slice(-24), draft]);
        setDraft((d) => ({ ...d, ...Object.fromEntries(res.placed.map((p) => [p.ride_id, p.driver_id])) }));
        setReasons((cur) => ({ ...cur, ...Object.fromEntries(res.placed.map((p) => [p.ride_id, p])) }));
      }
      setNoRoom(new Set(res.unplaced));
      if (res.placed.length === 0 && res.unplaced.length === 0) toast.info(t('autoArrangeNone'));
      if (res.placed.length > 0) toast.success(t('autoArrangeDone', { count: res.placed.length }));
      if (res.unplaced.length > 0) toast.error(t('autoArrangeUnplaced', { count: res.unplaced.length }));
      if (res.approximate > 0 && res.placed.length > 0) toast.info(t('autoArrangeApprox', { count: res.approximate }));
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      setArranging(false);
    }
  };

  const sameAsLast = () => {
    if (!canUpdate) return;
    const res = applyLastTime(plan, draft);
    if (res.placed > 0) setUndoStack((s) => [...s.slice(-24), draft]);
    setDraft(res.draft);
    if (res.candidates === 0) toast.info(t('sameAsLastNone'));
    else toast.success(t('sameAsLastDone', { placed: res.placed, candidates: res.candidates }));
  };

  const hasReturnRides = plan.rides.some((r) => r.leg === 'return' && draft[r.id] == null);
  const matchReturns = () => {
    if (!canUpdate) return;
    const res = applyMatchReturnDrivers(plan, draft);
    if (res.placed > 0) setUndoStack((s) => [...s.slice(-24), draft]);
    setDraft(res.draft);
    if (res.candidates === 0) toast.info(t('matchReturnNone'));
    else toast.success(t('matchReturnDone', { placed: res.placed, candidates: res.candidates }));
  };

  const resetAll = () => {
    if (!canUpdate) return;
    setUndoStack((s) => [...s.slice(-24), draft]);
    setDraft(savedDraft(plan));
  };

  const send = async () => {
    if (!canUpdate) return;
    if (anyOver) {
      toast.error(t('fixOver'));
      return;
    }
    const reconfirm = changes.filter((c) => plan.rides.find((r) => r.id === c.ride_id)?.status === 'confirmed').length;
    const ok = await confirm({
      title: t('confirmTitle'),
      body: (
        <div className="stack">
          <p>{t('confirmBody', { count: changes.length })}</p>
          {reconfirm > 0 && <p className="muted">{t('confirmReconfirm', { count: reconfirm })}</p>}
        </div>
      ),
      confirmLabel: t('confirmGo'),
      tone: 'primary',
    });
    if (!ok) return;
    setBusy(true);
    try {
      await post(`/admin/events/${plan.event.id}/plan`, { changes });
      toast.success(t('saved'));
      reload();
    } catch (e) {
      if (e instanceof ApiError && e.status === 409) {
        toast.error(t('stale'));
        reload();
      } else {
        toast.error(e instanceof Error ? e.message : String(e));
      }
    } finally {
      setBusy(false);
    }
  };

  const card = (r: PlanRide) => (
    <PassengerCard
      key={r.id}
      ride={r}
      plan={plan}
      draft={draft}
      onAssign={(d) => assign(r.id, d)}
      onOpenPicker={() => setPickerRideId(r.id)}
      changed={(draft[r.id] ?? null) !== r.driver_id}
      reason={reasons[r.id]?.driver_id === draft[r.id] ? reasons[r.id] : undefined}
      noRoom={draft[r.id] == null && noRoom.has(r.id)}
      canUpdate={canUpdate}
    />
  );

  const pickerRide = pickerRideId != null ? plan.rides.find((r) => r.id === pickerRideId) ?? null : null;

  return (
    <Page width="wide" className="plan-page">
      <PageHeader
        title={t('title')}
        subtitle={t('subtitle', { event: plan.event.title, when: fmtDateTime(plan.event.starts_at, ft) })}
        back={`/events/${plan.event.id}`}
      />

      {plan.rides.length > 0 && (
        <>
          <div className="plan-toolbar">
            <div className="plan-summary" aria-live="polite">
              {t('summary', { placed, total: plan.rides.length, seats: seatsUsed })}
            </div>
            {canUpdate && (
              <div className="btn-row">
                <Button
                  variant="secondary"
                  icon={<Sparkles size={16} />}
                  onClick={autoArrange}
                  loading={arranging}
                  disabled={unassigned.length === 0 || arranging}
                >
                  {t('autoArrange')}
                </Button>
                <Button variant="secondary" icon={<History size={16} />} onClick={sameAsLast} disabled={unassigned.length === 0}>
                  {t('sameAsLast')}
                </Button>
                {hasReturnRides && (
                  <Button variant="secondary" icon={<Repeat size={16} />} onClick={matchReturns}>
                    {t('matchReturns')}
                  </Button>
                )}
                <Button variant="ghost" icon={<Undo2 size={16} />} onClick={undoStep} disabled={undoStack.length === 0}>
                  {t('undoStep')}
                </Button>
                <Button variant="ghost" icon={<RotateCcw size={16} />} onClick={resetAll} disabled={!dirty}>
                  {t('reset')}
                </Button>
                <Button variant="primary" className="plan-confirm" icon={<Send size={16} />} onClick={send} disabled={!dirty || busy} loading={busy}>
                  {t('confirm', { count: changes.length })}
                </Button>
              </div>
            )}
          </div>

          <div className="chips plan-filters" role="group" aria-label={t('title')}>
            <button
              type="button"
              className={`chip ${filter === 'all' ? 'on' : ''}`}
              onClick={() => setFilter('all')}
            >
              {t('filterAll', { count: plan.rides.length })}
            </button>
            <button
              type="button"
              className={`chip ${filter === 'unassigned' ? 'on' : ''}`}
              onClick={() => setFilter('unassigned')}
            >
              {t('filterUnassigned', { count: unassigned.length })}
            </button>
            {assistanceCount > 0 && (
              <button
                type="button"
                className={`chip ${filter === 'assistance' ? 'on' : ''}`}
                onClick={() => setFilter('assistance')}
              >
                {t('filterAssistance', { count: assistanceCount })}
              </button>
            )}
            {uniqueAreas.length > 1 &&
              uniqueAreas.map((areaName) => (
                <button
                  key={areaName}
                  type="button"
                  className={`chip ${filter === `area:${areaName}` ? 'on' : ''}`}
                  onClick={() => setFilter(`area:${areaName}`)}
                >
                  {areaName}
                </button>
              ))}
          </div>
        </>
      )}
      {dirty && <p className="plan-unsaved">{t('unsaved')}</p>}

      {plan.rides.length === 0 ? (
        <EmptyState title={t('noRides')} />
      ) : (
        <div className="plan-board">
          <section
            className={`plan-column plan-unassigned ${dropTarget === 'none' ? 'is-drop' : ''}`}
            aria-label={t('unassigned')}
            {...dropProps('none', null)}
          >
            <h2 className="plan-col-title">
              {t('unassigned')} <span className="plan-count">{unassigned.length}</span>
            </h2>
            {unassigned.length === 0 ? (
              <p className="muted">{t('unassignedEmpty')}</p>
            ) : visibleUnassigned.length === 0 ? (
              <p className="muted">{t('filterEmpty')}</p>
            ) : (
              visibleUnassigned.map(card)
            )}
          </section>

          {filter !== 'unassigned' && (
            <div className="plan-drivers">
              {plan.drivers.length === 0 && <EmptyState title={t('noDrivers')} />}
              {plan.drivers.map((d) => {
                const load = loads.get(d.id)!;
                const mine = plan.rides.filter((r) => draft[r.id] === d.id && matchesFilter(r));
                return (
                  <Card
                    key={d.id}
                    className={`plan-driver ${load.over ? 'is-over' : ''} ${dropTarget === `d${d.id}` ? 'is-drop' : ''}`}
                    {...dropProps(`d${d.id}`, d.id)}
                  >
                    <div className="plan-driver-head">
                      <div>
                        <h3 className="plan-driver-name">
                          <Car size={18} aria-hidden="true" /> {d.name}
                        </h3>
                        <div className="muted plan-driver-meta">
                          {[
                            d.vehicle.model,
                            d.vehicle.plate,
                            d.carpool ? t('carpool') : t('oneBooking'),
                            d.vehicle.wheelchair_ok ? t('wheelchairTag') : null,
                            d.vehicle.low_step ? t('lowStepTag') : null,
                            d.vehicle.child_seat ? t('childSeatTag') : null,
                          ].filter(Boolean).join(' · ')}
                        </div>
                      </div>
                      <SeatBar used={load.seats} cap={d.cap} over={load.over} label={t('seats', { used: load.seats, cap: d.cap })} />
                    </div>
                    {d.other_seats > 0 && <p className="plan-note">{t('otherRides', { count: d.other_seats })}</p>}
                    {load.over && <p className="plan-note is-bad">{t('overLimit')}</p>}
                    <div className="plan-driver-rides">
                      {mine.length === 0 ? <p className="plan-drop-hint">{t('dropHere')}</p> : mine.map(card)}
                    </div>
                  </Card>
                );
              })}
            </div>
          )}
        </div>
      )}

      {pickerRide && (
        <Modal
          title={t('chooseDriverSheet', { name: pickerRide.passenger.name })}
          onClose={() => setPickerRideId(null)}
        >
          <div className="stack-sm">
            <button
              type="button"
              className={`item row between ${draft[pickerRide.id] == null ? 'active' : ''}`}
              onClick={() => {
                assign(pickerRide.id, null);
                setPickerRideId(null);
              }}
            >
              <span className="bold">{t('noDriver')}</span>
            </button>
            {plan.drivers
              .filter((d) => d.id !== pickerRide.passenger.id)
              .sort((a, b) => Number(inGroup(b, pickerRide)) - Number(inGroup(a, pickerRide)))
              .map((d) => {
                const l = driverLoad(
                  plan,
                  draft,
                  d,
                  pickerRide.leg === 'return' ? 'return' : 'outbound',
                );
                const sameGrp = inGroup(d, pickerRide);
                const sameArea = servesArea(d, pickerRide);
                return (
                  <button
                    key={d.id}
                    type="button"
                    className="item stack-xs plan-picker-option"
                    onClick={() => {
                      assign(pickerRide.id, d.id);
                      setPickerRideId(null);
                    }}
                  >
                    <div className="row between nowrap">
                      <span className="bold">{d.name}</span>
                      <SeatBar used={l.seats} cap={d.cap} over={l.over} label={t('seats', { used: l.seats, cap: d.cap })} />
                    </div>
                    <div className="row wrap gap-xs">
                      <Pill tone={sameGrp ? 'success' : 'warning'}>
                        {sameGrp ? t('inGroup') : t('outsideGroupAdmin')}
                      </Pill>
                      {sameArea && <Pill tone="info">{t('whyInArea')}</Pill>}
                    </div>
                  </button>
                );
              })}
          </div>
        </Modal>
      )}
    </Page>
  );
}

function SeatBar({ used, cap, over, label }: { used: number; cap: number; over: boolean; label: string }) {
  const pct = cap > 0 ? Math.min(100, (used / cap) * 100) : 100;
  return (
    <div className={`plan-seatbar ${over ? 'is-over' : ''}`}>
      <span className="plan-seatbar-label">{label}</span>
      <span className="plan-seatbar-track" aria-hidden="true">
        <span className="plan-seatbar-fill" style={{ width: `${pct}%` }} />
      </span>
    </div>
  );
}

function PassengerCard({
  ride: r,
  plan,
  draft,
  onAssign,
  onOpenPicker,
  changed,
  reason,
  noRoom,
  canUpdate = true,
}: {
  ride: PlanRide;
  plan: Plan;
  draft: Draft;
  onAssign: (driverId: number | null) => void;
  onOpenPicker: () => void;
  changed: boolean;
  reason?: Reason;
  noRoom?: boolean;
  canUpdate?: boolean;
}) {
  const { t } = useTranslation('plan');
  const ft = useFormatT();
  const current = draft[r.id] ?? null;
  const driver = plan.drivers.find((d) => d.id === current);
  const selectId = `plan-ride-${r.id}`;
  return (
    <article
      className={`plan-passenger ${changed ? 'is-changed' : ''}`}
      draggable={canUpdate}
      onDragStart={(e) => {
        if (!canUpdate) return;
        e.dataTransfer.setData(DRAG_TYPE, String(r.id));
        e.dataTransfer.effectAllowed = 'move';
      }}
    >
      <div className="plan-passenger-head">
        <strong className="plan-passenger-name">
          <UserRound size={16} aria-hidden="true" /> {r.passenger.name}
        </strong>
        {changed ? <Pill tone="info">{t('changed')}</Pill> : <StatusPill map={RIDE_STATUS} status={r.status} />}
      </div>
      <div className="plan-passenger-meta">
        {r.area.name} · {fmtTime(r.pickup_at, ft)} · {t('passengers', { count: r.seats })}
      </div>
      {r.pickup_address && <div className="plan-passenger-addr">{r.pickup_address}</div>}
      <div className="plan-tags">
        {r.leg === 'return' && <Pill tone="info">{t('returnTag')}</Pill>}
        {r.assistance && (
          <Pill tone="warning" icon={<Accessibility size={14} />}>
            {t('assistance', { what: r.assistance })}
          </Pill>
        )}
        {r.last && <Pill tone="neutral" icon={<History size={14} />}>{t('last', { name: r.last.driver_name, times: r.last.times })}</Pill>}
        {driver && !servesArea(driver, r) && <Pill tone="warning">{t('outsideArea')}</Pill>}
        {driver && !inGroup(driver, r) && <Pill tone="warning">{t('outsideGroup')}</Pill>}
        {noRoom && <Pill tone="danger">{t('noRoom')}</Pill>}
      </div>
      {reason && (
        <p className="plan-why">
          <Sparkles size={13} aria-hidden="true" />
          <span>
            {[
              reason.near &&
                (reason.near.km != null
                  ? t('whyNear', { name: reason.near.name, km: reason.near.km })
                  : t('whySameArea', { name: reason.near.name })),
              reason.together > 0 && t('whyTogether', { times: reason.together }),
              reason.in_area && t('whyInArea'),
            ]
              .filter(Boolean)
              .join(' · ')}
          </span>
        </p>
      )}
      <div className="row nowrap gap-xs">
        <label className="sr-only" htmlFor={selectId}>
          {t('assignTo', { name: r.passenger.name })}
        </label>
        <select
          id={selectId}
          className="input plan-select grow"
          value={current ?? ''}
          disabled={!canUpdate}
          onChange={(e) => onAssign(e.target.value ? Number(e.target.value) : null)}
        >
          <option value="">{t('noDriver')}</option>
          {/* Drivers from the rider's group first; anyone else only by an admin's choice (audited). */}
          {[true, false].map((inside) => {
            const list = plan.drivers.filter((d) => d.id !== r.passenger.id && inGroup(d, r) === inside);
            if (!list.length) return null;
            const options = list.map((d) => {
              const l = driverLoad(plan, draft, d, r.leg === 'return' ? 'return' : 'outbound');
              return (
                <option key={d.id} value={d.id}>
                  {t('driverOption', { name: d.name, used: l.seats, cap: d.cap })}
                </option>
              );
            });
            return inside && r.match_group_id == null ? options : (
              <optgroup key={String(inside)} label={inside ? t('inGroup') : t('outsideGroupAdmin')}>
                {options}
              </optgroup>
            );
          })}
        </select>
        {canUpdate && (
          <IconButton
            size="sm"
            variant="secondary"
            label={t('chooseDriverSheet', { name: r.passenger.name })}
            icon={<Car size={16} />}
            onClick={onOpenPicker}
          />
        )}
      </div>
    </article>
  );
}
