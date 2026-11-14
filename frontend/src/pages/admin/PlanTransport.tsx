// Transport planning board: arrange every ride request for one event at once.
// Drag a passenger onto a driver (desktop) or pick the driver from the card's menu (phones,
// keyboards, screen readers). Nothing is sent until "Confirm & notify".
import { DragEvent, useEffect, useMemo, useState } from 'react';
import { useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Accessibility, Car, History, RotateCcw, Send, Sparkles, UserRound } from 'lucide-react';
import { ApiError, post, useApi } from '../../api';
import { useAuth } from '../../auth';
import {
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
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
  ArrangeResult,
  changesOf,
  Draft,
  draftEntries,
  driverLoad,
  Plan,
  PlanRide,
  Reason,
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
    setDraft((d) => ({ ...d, [rideId]: driverId }));
  };
  const loads = new Map(plan.drivers.map((d) => [d.id, driverLoad(plan, draft, d)]));
  const anyOver = plan.drivers.some((d) => loads.get(d.id)!.over);
  const unassigned = plan.rides.filter((r) => draft[r.id] == null);
  const placed = plan.rides.length - unassigned.length;
  const seatsUsed = plan.rides.filter((r) => draft[r.id] != null).reduce((n, r) => n + r.seats, 0);

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
      setDraft((d) => ({ ...d, ...Object.fromEntries(res.placed.map((p) => [p.ride_id, p.driver_id])) }));
      setReasons((cur) => ({ ...cur, ...Object.fromEntries(res.placed.map((p) => [p.ride_id, p])) }));
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
    const res = applyLastTime(plan, draft);
    setDraft(res.draft);
    if (res.candidates === 0) toast.info(t('sameAsLastNone'));
    else toast.success(t('sameAsLastDone', { placed: res.placed, candidates: res.candidates }));
  };

  const send = async () => {
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
      changed={(draft[r.id] ?? null) !== r.driver_id}
      reason={reasons[r.id]?.driver_id === draft[r.id] ? reasons[r.id] : undefined}
      noRoom={draft[r.id] == null && noRoom.has(r.id)}
      canUpdate={canUpdate}
    />
  );

  return (
    <Page width="wide" className="plan-page">
      <PageHeader
        title={t('title')}
        subtitle={t('subtitle', { event: plan.event.title, when: fmtDateTime(plan.event.starts_at, ft) })}
        back={`/events/${plan.event.id}`}
      />

      {plan.rides.length > 0 && (
        <div className="plan-toolbar">
          <div className="plan-summary" aria-live="polite">
            {t('summary', { placed, total: plan.rides.length, seats: seatsUsed })}
          </div>
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
            <Button variant="ghost" icon={<RotateCcw size={16} />} onClick={() => setDraft(savedDraft(plan))} disabled={!dirty}>
              {t('reset')}
            </Button>
            <Button variant="primary" icon={<Send size={16} />} onClick={send} disabled={!dirty || busy} loading={busy}>
              {t('confirm', { count: changes.length })}
            </Button>
          </div>
        </div>
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
            {unassigned.length === 0 ? <p className="muted">{t('unassignedEmpty')}</p> : unassigned.map(card)}
          </section>

          <div className="plan-drivers">
            {plan.drivers.length === 0 && <EmptyState title={t('noDrivers')} />}
            {plan.drivers.map((d) => {
              const load = loads.get(d.id)!;
              const mine = plan.rides.filter((r) => draft[r.id] === d.id);
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
                        {[d.vehicle.model, d.vehicle.plate].filter(Boolean).join(' · ')}
                        {' · '}
                        {d.carpool ? t('carpool') : t('oneBooking')}
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
        </div>
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
  changed,
  reason,
  noRoom,
  canUpdate = true,
}: {
  ride: PlanRide;
  plan: Plan;
  draft: Draft;
  onAssign: (driverId: number | null) => void;
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
        {r.assistance && (
          <Pill tone="warning" icon={<Accessibility size={14} />}>
            {t('assistance', { what: r.assistance })}
          </Pill>
        )}
        {r.last && <Pill tone="neutral" icon={<History size={14} />}>{t('last', { name: r.last.driver_name, times: r.last.times })}</Pill>}
        {driver && !servesArea(driver, r) && <Pill tone="warning">{t('outsideArea')}</Pill>}
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
      <label className="sr-only" htmlFor={selectId}>
        {t('assignTo', { name: r.passenger.name })}
      </label>
      <select
        id={selectId}
        className="input plan-select"
        value={current ?? ''}
        disabled={!canUpdate}
        onChange={(e) => onAssign(e.target.value ? Number(e.target.value) : null)}
      >
        <option value="">{t('noDriver')}</option>
        {plan.drivers
          .filter((d) => d.id !== r.passenger.id)
          .map((d) => {
            const l = driverLoad(plan, draft, d);
            return (
              <option key={d.id} value={d.id}>
                {t('driverOption', { name: d.name, used: l.seats, cap: d.cap })}
              </option>
            );
          })}
      </select>
    </article>
  );
}
