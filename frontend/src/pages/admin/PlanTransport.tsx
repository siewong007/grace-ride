// Transport planning board: arrange every ride request for one event at once.
// Drag a passenger onto a driver (desktop) or pick the driver from the card's menu / sheet (phones,
// keyboards, screen readers). Nothing is sent until "Confirm & notify".
import { DragEvent, useCallback, useEffect, useMemo, useState } from 'react';
import { useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Accessibility, AlertCircle, Car, Check, CheckCircle2, Clock, Copy, Download, History, Repeat, RotateCcw, Save, Send, Sparkles, Undo2, UserRound } from 'lucide-react';
import { ApiError, post, useApi } from '../../api';
import { useAuth } from '../../auth';
import { canApprovePlan } from '../../perms';
import {
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  IconButton,
  Modal,
  Notice,
  Page,
  PageHeader,
  Pill,
  StatusPill,
  useConfirm,
  useToast,
} from '../../components/ui';
import PostEventReimbursementModal from '../../components/events/PostEventReimbursementModal';
import { fmtDateTime, fmtTime, RIDE_STATUS, useFormatT } from '../../format';
import { copyCanvasToClipboard, downloadDataUrl, renderPlanToCanvas, sanitizeFilename } from './planImageExport';
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
  const { can, me } = useAuth();
  const canUpdate = can('admin_plan', 'update');
  const { data, error, loading, reload } = useApi<Plan>(`/admin/events/${id}/plan`);
  const canApprove = data?.can_approve ?? canApprovePlan(me?.roles);
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
  const [showPostEventReimb, setShowPostEventReimb] = useState(false);
  const [exportModalOpen, setExportModalOpen] = useState(false);
  const [previewDataUrl, setPreviewDataUrl] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);
  const [copied, setCopied] = useState(false);

  // A fresh board (first load, after sending, after a conflict) resets the draft.
  useEffect(() => {
    if (data) {
      const base = savedDraft(data);
      if (data.draft?.assignments && data.draft.assignments.length > 0) {
        for (const a of data.draft.assignments) {
          base[a.ride_id] = a.driver_id;
        }
      }
      setDraft(base);
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

  const saveDraft = async () => {
    if (!canUpdate) return;
    setBusy(true);
    try {
      await post(`/admin/events/${plan.event.id}/plan/draft`, {
        assignments: draftEntries(plan, draft),
      });
      toast.success(t('draftSaved'));
      reload();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const askLeadApproval = async () => {
    if (!canUpdate) return;
    if (anyOver) {
      toast.error(t('fixOver'));
      return;
    }
    const ok = await confirm({
      title: t('askLeadApprovalTitle'),
      body: <p>{t('askLeadApprovalBody')}</p>,
      confirmLabel: t('askLeadApprovalGo'),
      tone: 'primary',
    });
    if (!ok) return;
    setBusy(true);
    try {
      await post(`/admin/events/${plan.event.id}/plan/request-approval`, {
        assignments: draftEntries(plan, draft),
      });
      toast.success(t('submittedForApproval'));
      reload();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const returnDraft = async () => {
    if (!canApprove) return;
    const notes = window.prompt(t('returnDraftPrompt')) ?? '';
    if (!notes.trim()) return;
    setBusy(true);
    try {
      await post(`/admin/events/${plan.event.id}/plan/return`, { notes: notes.trim() });
      toast.success(t('returnedSuccess'));
      reload();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const send = async () => {
    if (!canUpdate) return;
    if (anyOver) {
      toast.error(t('fixOver'));
      return;
    }
    const reconfirm = changes.filter((c) => plan.rides.find((r) => r.id === c.ride_id)?.status === 'confirmed').length;
    const ok = await confirm({
      title: canApprove ? t('approveTitle') : t('confirmTitle'),
      body: (
        <div className="stack">
          <p>{canApprove ? t('approveBody', { count: changes.length }) : t('confirmBody', { count: changes.length })}</p>
          {reconfirm > 0 && <p className="muted">{t('confirmReconfirm', { count: reconfirm })}</p>}
        </div>
      ),
      confirmLabel: canApprove ? t('approveGo') : t('confirmGo'),
      tone: 'primary',
    });
    if (!ok) return;
    setBusy(true);
    try {
      await post(`/admin/events/${plan.event.id}/plan`, { changes });
      toast.success(canApprove ? t('approvedSuccess') : t('saved'));
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

  const openExportModal = () => {
    setExporting(true);
    try {
      const canvas = renderPlanToCanvas({
        plan,
        draft,
        t: (k, o) => String(t(k as never, o as never)),
        formatT: (k, o) => String(ft(k as never, o as never)),
        scale: 2,
        isApproved: plan.draft?.status === 'approved' || (!dirty && plan.event.status === 'confirmed'),
      });
      const url = canvas.toDataURL('image/png');
      setPreviewDataUrl(url);
      setExportModalOpen(true);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      setExporting(false);
    }
  };

  const handleDownloadImage = () => {
    if (!previewDataUrl) return;
    const filename = `${sanitizeFilename(plan.event.title)}-transport-assignments.png`;
    downloadDataUrl(previewDataUrl, filename);
  };

  const handleCopyImage = async () => {
    if (!previewDataUrl) return;
    try {
      const canvas = renderPlanToCanvas({
        plan,
        draft,
        t: (k, o) => String(t(k as never, o as never)),
        formatT: (k, o) => String(ft(k as never, o as never)),
        scale: 2,
        isApproved: plan.draft?.status === 'approved' || (!dirty && plan.event.status === 'confirmed'),
      });
      const success = await copyCanvasToClipboard(canvas);
      if (success) {
        setCopied(true);
        toast.success(t('copyImageSuccess'));
        setTimeout(() => setCopied(false), 2000);
      } else {
        toast.info(t('copyImageFailed'));
      }
    } catch {
      toast.info(t('copyImageFailed'));
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
        actions={
          plan.draft?.status ? (
            <Pill
              tone={
                plan.draft.status === 'pending_approval'
                  ? 'warning'
                  : plan.draft.status === 'returned'
                  ? 'danger'
                  : plan.draft.status === 'approved'
                  ? 'success'
                  : 'info'
              }
            >
              {plan.draft.status === 'pending_approval'
                ? t('statusPendingApproval')
                : plan.draft.status === 'returned'
                ? t('statusReturned')
                : plan.draft.status === 'approved'
                ? t('statusApproved')
                : t('statusDraft')}
            </Pill>
          ) : undefined
        }
      />

      {plan.draft?.status === 'pending_approval' && (
        <Notice tone="warning" icon={<Clock size={16} />}>
          {t('bannerPendingApproval', {
            name: plan.draft.submitted_by?.name || '',
            time: plan.draft.submitted_at ? fmtDateTime(plan.draft.submitted_at, ft) : '',
          })}
        </Notice>
      )}

      {plan.draft?.status === 'returned' && (
        <Notice tone="danger" icon={<AlertCircle size={16} />}>
          {t('bannerReturned', { notes: plan.draft.review_notes || '' })}
        </Notice>
      )}

      {plan.rides.length > 0 && (
        <>
          <div className="plan-toolbar">
            <div className="plan-summary" aria-live="polite">
              {t('summary', { placed, total: plan.rides.length, seats: seatsUsed })}
            </div>
            <div className="btn-row">
              <Button
                variant="secondary"
                icon={<Download size={16} />}
                onClick={openExportModal}
                loading={exporting}
              >
                {t('exportImage')}
              </Button>
              {canUpdate && (
                <>
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
                <Button variant="secondary" icon={<Save size={16} />} onClick={saveDraft} disabled={busy} loading={busy}>
                  {t('saveDraft')}
                </Button>
                <Button
                  variant="secondary"
                  icon={<Car size={16} />}
                  onClick={() => setShowPostEventReimb(true)}
                >
                  {t('postEventReimbBtn', { ns: 'events' })}
                </Button>
                {!canApprove ? (
                  <Button variant="primary" icon={<Send size={16} />} onClick={askLeadApproval} disabled={busy} loading={busy}>
                    {t('askLeadApproval')}
                  </Button>
                ) : (
                  <>
                    {plan.draft?.status === 'pending_approval' && (
                      <Button variant="danger" icon={<RotateCcw size={16} />} onClick={returnDraft} disabled={busy}>
                        {t('returnDraft')}
                      </Button>
                    )}
                    <Button variant="primary" className="plan-confirm" icon={<CheckCircle2 size={16} />} onClick={send} disabled={!dirty || busy} loading={busy}>
                      {t('approveAndNotify', { count: changes.length })}
                    </Button>
                  </>
                )}
              </>
            )}
          </div>
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

      {showPostEventReimb && (
        <PostEventReimbursementModal
          eventId={plan.event.id}
          onClose={() => setShowPostEventReimb(false)}
          onSuccess={() => reload()}
        />
      )}

      {exportModalOpen && previewDataUrl && (
        <Modal
          title={t('exportModalTitle')}
          onClose={() => setExportModalOpen(false)}
        >
          <div className="stack">
            <p className="muted" style={{ margin: 0 }}>
              {t('exportModalDesc')}
            </p>
            <div className="row between items-center gap-xs">
              <Pill tone="info">{t('exportHint')}</Pill>
            </div>
            <div className="plan-export-preview-container">
              <img
                src={previewDataUrl}
                alt={t('exportModalTitle')}
                className="plan-export-preview-img"
              />
            </div>
            <div className="row wrap between gap-sm">
              <div className="row gap-xs">
                <Button
                  variant="primary"
                  icon={<Download size={16} />}
                  onClick={handleDownloadImage}
                >
                  {t('downloadImage')}
                </Button>
                <Button
                  variant="secondary"
                  icon={copied ? <Check size={16} /> : <Copy size={16} />}
                  onClick={handleCopyImage}
                >
                  {copied ? t('copyImageSuccess') : t('copyImage')}
                </Button>
              </div>
              <Button variant="ghost" onClick={() => setExportModalOpen(false)}>
                {t('close', { ns: 'common' })}
              </Button>
            </div>
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
