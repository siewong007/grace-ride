// Transport planning board: arrange every ride request for one event at once.
// Drag a passenger onto a driver (desktop) or pick the driver from the card's menu / sheet (phones,
// keyboards, screen readers). Nothing is sent until "Confirm & notify".
import { DragEvent, FormEvent, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useParams, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import '../../styles/planTransport.css';
import {
  Accessibility,
  AlertCircle,
  Car,
  Check,
  CheckCircle2,
  Clock,
  Copy,
  Download,
  History,
  Lock,
  Plus,
  Repeat,
  RotateCcw,
  Save,
  Send,
  Sparkles,
  Trash2,
  Undo2,
  Unlock,
  UserRound,
} from 'lucide-react';
import { api, ApiError, post, useApi } from '../../api';
import { useAuth } from '../../auth';
import { canApprovePlan } from '../../perms';
import {
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Field,
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
  DraftVisitorEntry,
  driverLoad,
  mergeEffectiveVisitors,
  Plan,
  PlanDraft,
  PlanRide,
  PlanVisitor,
  Reason,
  inGroup,
  savedDraft,
  servesArea,
  summarizeDraftDiff,
} from './planLogic';
import PlanVisitors, { VisitorLine, visitorTotals } from './PlanVisitors';

const DRAG_TYPE = 'application/x-grace-ride';

export default function PlanTransport() {
  const { id } = useParams();
  const [searchParams, setSearchParams] = useSearchParams();
  const { t } = useTranslation('plan');
  const ft = useFormatT();
  const toast = useToast();
  const confirm = useConfirm();
  const { can, me } = useAuth();
  const canUpdate = can('admin_plan', 'update');
  const canEditRides = can('admin_rides', 'update');
  const { data, error, loading, reload } = useApi<Plan>(`/admin/events/${id}/plan`);
  const canApprove = data?.can_approve ?? canApprovePlan(me?.roles);
  const [selectedDraftKey, setSelectedDraftKey] = useState<'live' | 'legacy' | number>('live');
  const [draft, setDraft] = useState<Draft>({});
  const [draftVisitors, setDraftVisitors] = useState<DraftVisitorEntry[]>([]);
  const [acquiredLocks, setAcquiredLocks] = useState<Set<number>>(new Set());
  const lockInFlightRef = useRef<number | null>(null);
  const [undoStack, setUndoStack] = useState<Draft[]>([]);
  const [filter, setFilter] = useState<string>('all');
  const [pickerRideId, setPickerRideId] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [dropTarget, setDropTarget] = useState<string | null>(null);
  // Auto-arrange's explanations, and who it couldn't fit; cleared when the board resets.
  const [reasons, setReasons] = useState<Record<number, Reason>>({});
  const [noRoom, setNoRoom] = useState<Set<number>>(new Set());
  const [arranging, setArranging] = useState(false);
  const [diagnosisOpen, setDiagnosisOpen] = useState(false);
  const [lastArrangeResult, setLastArrangeResult] = useState<ArrangeResult | null>(null);
  const [showPostEventReimb, setShowPostEventReimb] = useState(false);
  const [exportModalOpen, setExportModalOpen] = useState(false);
  const [previewDataUrl, setPreviewDataUrl] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);
  const [copied, setCopied] = useState(false);
  const [newDraftOpen, setNewDraftOpen] = useState(false);
  const [newDraftLabel, setNewDraftLabel] = useState('');
  const [newDraftBase, setNewDraftBase] = useState<'live' | 'clone'>('live');
  const [returnModalOpen, setReturnModalOpen] = useState(false);
  const [returnNotes, setReturnNotes] = useState('');
  const [mobileToolsOpen, setMobileToolsOpen] = useState(false);
  const [visitorsExpanded, setVisitorsExpanded] = useState<boolean | null>(null);
  // Visitors are saved straight away in Live mode, or stored inside draftVisitors in Draft mode.
  const [visitors, setVisitors] = useState<PlanVisitor[] | null>(null);
  const reloadVisitors = useCallback(() => {
    api<{ visitors: PlanVisitor[] }>(`/admin/events/${id}/plan/visitors`)
      .then((r) => setVisitors(r.visitors))
      .catch(() => undefined);
  }, [id]);

  const allDrafts = useMemo<PlanDraft[]>(() => {
    if (data?.drafts && data.drafts.length > 0) return data.drafts;
    if (data?.draft) return [data.draft];
    return [];
  }, [data]);

  const applySelectionToBoard = useCallback((planData: Plan, targetDraft: PlanDraft | null) => {
    const base = savedDraft(planData);
    if (
      targetDraft &&
      targetDraft.status !== 'approved' &&
      targetDraft.status !== 'superseded' &&
      targetDraft.assignments &&
      targetDraft.assignments.length > 0
    ) {
      for (const a of targetDraft.assignments) {
        base[a.ride_id] = a.driver_id;
      }
    }
    setDraft(base);
    setDraftVisitors(targetDraft?.visitors ?? []);
    setUndoStack([]);
    setReasons({});
    setNoRoom(new Set());
  }, []);

  // A fresh board (first load, after sending, after a conflict) initializes selection & board state.
  useEffect(() => {
    if (!data) return;
    const paramDraft = searchParams.get('draft');
    const draftsList = data.drafts && data.drafts.length > 0 ? data.drafts : data.draft ? [data.draft] : [];
    let nextKey: 'live' | 'legacy' | number = 'live';
    let targetDraft: PlanDraft | null = null;
    if (paramDraft === 'live') {
      nextKey = 'live';
      targetDraft = null;
    } else if (paramDraft && draftsList.some((d) => d.id != null && String(d.id) === paramDraft)) {
      const matched = draftsList.find((d) => d.id != null && String(d.id) === paramDraft)!;
      nextKey = matched.id!;
      targetDraft = matched;
    } else if (data.draft && data.draft.status !== 'approved' && data.draft.status !== 'superseded') {
      nextKey = data.draft.id ?? 'legacy';
      targetDraft = data.draft;
    }
    setSelectedDraftKey(nextKey);
    setVisitors(null);
    setAcquiredLocks(new Set());
    lockInFlightRef.current = null;
    applySelectionToBoard(data, targetDraft);
    // Intentionally keyed only on `data` so query-param switches do not clobber explicit switching or local mocks.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data, applySelectionToBoard]);

  const activeDraft = useMemo<PlanDraft | null>(() => {
    if (selectedDraftKey === 'live') return null;
    if (selectedDraftKey === 'legacy') return data?.draft ?? null;
    return allDrafts.find((d) => d.id === selectedDraftKey) ?? null;
  }, [allDrafts, data?.draft, selectedDraftKey]);

  const isMyLock = Boolean(
    activeDraft?.id != null && (activeDraft.lock_is_mine || acquiredLocks.has(activeDraft.id)),
  );
  const lockedByOther = Boolean(
    activeDraft?.id != null && activeDraft.locked_by && !activeDraft.lock_is_mine && !acquiredLocks.has(activeDraft.id),
  );
  const draftStatusEditable = activeDraft
    ? activeDraft.status === 'draft' ||
      activeDraft.status === 'returned' ||
      (activeDraft.status === 'pending_approval' && canApprove)
    : true;
  const canEditBoard = canUpdate && !lockedByOther && draftStatusEditable;

  // Visibility-aware 3-minute lock heartbeat while editing a locked draft version.
  useEffect(() => {
    const lockedId = activeDraft?.id;
    if (!lockedId || !isMyLock || !id) return;
    const timer = window.setInterval(() => {
      if (typeof document === 'undefined' || document.visibilityState === 'visible') {
        post(`/admin/events/${id}/plan/drafts/${lockedId}/lock`, {}).catch(() => undefined);
      }
    }, 3 * 60 * 1000);
    return () => window.clearInterval(timer);
  }, [id, activeDraft?.id, isMyLock]);

  const ensureLock = useCallback(() => {
    const did = activeDraft?.id;
    if (!did || !id || isMyLock || lockedByOther || lockInFlightRef.current === did) return;
    lockInFlightRef.current = did;
    post(`/admin/events/${id}/plan/drafts/${did}/lock`, {})
      .then(() => {
        setAcquiredLocks((prev) => new Set(prev).add(did));
      })
      .catch((e) => {
        if (e instanceof ApiError && e.status === 423) {
          toast.error(e.message);
          reload();
        }
      })
      .finally(() => {
        if (lockInFlightRef.current === did) lockInFlightRef.current = null;
      });
  }, [activeDraft?.id, id, isMyLock, lockedByOther, toast, reload]);

  const selectDraftView = useCallback(
    (nextKey: 'live' | 'legacy' | number) => {
      if (!data) return;
      if (activeDraft?.id != null && isMyLock && id) {
        api(`/admin/events/${id}/plan/drafts/${activeDraft.id}/lock`, { method: 'DELETE' }).catch(() => undefined);
      }
      const nextTarget =
        nextKey === 'live'
          ? null
          : nextKey === 'legacy'
          ? (data.draft ?? null)
          : (allDrafts.find((d) => d.id === nextKey) ?? null);
      setSelectedDraftKey(nextKey);
      applySelectionToBoard(data, nextTarget);
      const nextParams = new URLSearchParams(searchParams);
      if (nextKey === 'live') nextParams.set('draft', 'live');
      else if (typeof nextKey === 'number') nextParams.set('draft', String(nextKey));
      else nextParams.delete('draft');
      setSearchParams(nextParams, { replace: true });
    },
    [data, activeDraft?.id, isMyLock, id, allDrafts, applySelectionToBoard, searchParams, setSearchParams],
  );

  const changes = useMemo(() => (data ? changesOf(data, draft) : []), [data, draft]);
  const visitorsDirty = useMemo(
    () =>
      activeDraft != null &&
      JSON.stringify(draftVisitors) !== JSON.stringify(activeDraft.visitors ?? []),
    [activeDraft, draftVisitors],
  );
  const dirty = changes.length > 0 || visitorsDirty;

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
      if (!canEditBoard || undoStack.length === 0) return;
      const tag = (e.target as HTMLElement | null)?.tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;
      if ((e.metaKey || e.ctrlKey) && !e.shiftKey && e.key.toLowerCase() === 'z') {
        e.preventDefault();
        undoStep();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [canEditBoard, undoStack.length, undoStep]);

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

  const baseVisitors = visitors ?? data.visitors ?? [];
  const effectiveVisitors = mergeEffectiveVisitors(
    baseVisitors,
    activeDraft ? draftVisitors : [],
    data.drivers,
  );
  const activeVisitorsForLoad = effectiveVisitors.filter((v) => v._draftOp !== 'delete');
  const plan: Plan = { ...data, visitors: activeVisitorsForLoad };
  const visitorList = activeVisitorsForLoad;
  const visitorCount = visitorTotals(visitorList);

  const assign = (rideId: number, driverId: number | null) => {
    if (!canEditBoard) return;
    if ((draft[rideId] ?? null) === driverId) return;
    ensureLock();
    setUndoStack((s) => [...s.slice(-24), draft]);
    setDraft((d) => ({ ...d, [rideId]: driverId }));
  };

  const handleDraftVisitorsChange = (next: DraftVisitorEntry[]) => {
    if (!canEditBoard) return;
    ensureLock();
    setDraftVisitors(next);
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
    if (!canEditBoard) return;
    const rideId = Number(e.dataTransfer.getData(DRAG_TYPE));
    if (rideId) assign(rideId, target);
  };
  const dropProps = (key: string, target: number | null) => ({
    onDragOver: (e: DragEvent) => {
      if (canEditBoard && e.dataTransfer.types.includes(DRAG_TYPE)) {
        e.preventDefault();
        setDropTarget(key);
      }
    },
    onDragLeave: () => setDropTarget((k) => (k === key ? null : k)),
    onDrop: onDrop(target),
  });

  const autoArrange = async () => {
    if (!canEditBoard) return;
    setArranging(true);
    try {
      const res = await post<ArrangeResult>(`/admin/events/${plan.event.id}/plan/arrange`, {
        draft: draftEntries(plan, draft),
      });
      setLastArrangeResult(res);
      if (res.placed.length > 0) {
        ensureLock();
        setUndoStack((s) => [...s.slice(-24), draft]);
        setDraft((d) => ({ ...d, ...Object.fromEntries(res.placed.map((p) => [p.ride_id, p.driver_id])) }));
        setReasons((cur) => ({ ...cur, ...Object.fromEntries(res.placed.map((p) => [p.ride_id, p])) }));
      }
      setNoRoom(new Set(res.unplaced));
      if (res.placed.length === 0 && res.unplaced.length === 0) toast.info(t('autoArrangeNone'));
      if (res.placed.length > 0) toast.success(t('autoArrangeDone', { count: res.placed.length }));
      if (res.unplaced.length > 0) toast.error(t('autoArrangeUnplaced', { count: res.unplaced.length }));
      if (res.approximate > 0 && res.placed.length > 0) toast.info(t('autoArrangeApprox', { count: res.approximate }));
      if (res.placed.length > 0 || res.unplaced.length > 0) {
        setDiagnosisOpen(true);
      }
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      setArranging(false);
    }
  };

  const sameAsLast = () => {
    if (!canEditBoard) return;
    const res = applyLastTime(plan, draft);
    if (res.placed > 0) {
      ensureLock();
      setUndoStack((s) => [...s.slice(-24), draft]);
    }
    setDraft(res.draft);
    if (res.candidates === 0) toast.info(t('sameAsLastNone'));
    else toast.success(t('sameAsLastDone', { placed: res.placed, candidates: res.candidates }));
  };

  const hasReturnRides = plan.rides.some((r) => r.leg === 'return' && draft[r.id] == null);
  const matchReturns = () => {
    if (!canEditBoard) return;
    const res = applyMatchReturnDrivers(plan, draft);
    if (res.placed > 0) {
      ensureLock();
      setUndoStack((s) => [...s.slice(-24), draft]);
    }
    setDraft(res.draft);
    if (res.candidates === 0) toast.info(t('matchReturnNone'));
    else toast.success(t('matchReturnDone', { placed: res.placed, candidates: res.candidates }));
  };

  const resetAll = () => {
    if (!canEditBoard) return;
    setUndoStack((s) => [...s.slice(-24), draft]);
    setDraft(savedDraft(plan));
    if (activeDraft) setDraftVisitors([]);
  };

  const saveDraft = async () => {
    if (!canEditBoard) return;
    setBusy(true);
    try {
      if (activeDraft?.id != null) {
        await api(`/admin/events/${plan.event.id}/plan/drafts/${activeDraft.id}`, {
          method: 'PUT',
          body: {
            assignments: draftEntries(plan, draft),
            visitors: draftVisitors,
          },
        });
      } else {
        await post(`/admin/events/${plan.event.id}/plan/draft`, {
          assignments: draftEntries(plan, draft),
          ...(draftVisitors.length > 0 ? { visitors: draftVisitors } : {}),
        });
      }
      toast.success(t('draftSaved'));
      reload();
    } catch (e) {
      if (e instanceof ApiError && e.status === 423) {
        toast.error(e.message);
        reload();
      } else {
        toast.error(e instanceof Error ? e.message : String(e));
      }
    } finally {
      setBusy(false);
    }
  };

  const askLeadApproval = async () => {
    if (!canEditBoard) return;
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
        ...(activeDraft?.id != null ? { draft_id: activeDraft.id } : {}),
        assignments: draftEntries(plan, draft),
        ...(draftVisitors.length > 0 ? { visitors: draftVisitors } : {}),
      });
      toast.success(t('submittedForApproval'));
      reload();
    } catch (e) {
      if (e instanceof ApiError && e.status === 423) {
        toast.error(e.message);
        reload();
      } else {
        toast.error(e instanceof Error ? e.message : String(e));
      }
    } finally {
      setBusy(false);
    }
  };

  const submitReturnModal = async (e: FormEvent) => {
    e.preventDefault();
    if (!canApprove) return;
    const notes = returnNotes.trim();
    if (!notes) return;
    setBusy(true);
    try {
      await post(`/admin/events/${plan.event.id}/plan/return`, {
        ...(activeDraft?.id != null ? { draft_id: activeDraft.id } : {}),
        notes,
      });
      toast.success(t('returnedSuccess'));
      setReturnModalOpen(false);
      setReturnNotes('');
      reload();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const send = async () => {
    if (!canEditBoard) return;
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
      if (activeDraft?.id != null && visitorsDirty) {
        await api(`/admin/events/${plan.event.id}/plan/drafts/${activeDraft.id}`, {
          method: 'PUT',
          body: {
            assignments: draftEntries(plan, draft),
            visitors: draftVisitors,
          },
        });
      }
      if (activeDraft != null) {
        await post(`/admin/events/${plan.event.id}/plan/approve`, {
          ...(activeDraft.id != null ? { draft_id: activeDraft.id } : {}),
          changes,
        });
      } else {
        await post(`/admin/events/${plan.event.id}/plan`, { changes });
      }
      toast.success(canApprove ? t('approvedSuccess') : t('saved'));
      reload();
    } catch (e) {
      if (e instanceof ApiError && (e.status === 409 || e.status === 423)) {
        toast.error(e.status === 409 ? t('stale') : e.message);
        reload();
      } else {
        toast.error(e instanceof Error ? e.message : String(e));
      }
    } finally {
      setBusy(false);
    }
  };

  const acquireExplicitLock = async (force = false) => {
    if (!activeDraft?.id) return;
    setBusy(true);
    try {
      await post(`/admin/events/${plan.event.id}/plan/drafts/${activeDraft.id}/lock`, force ? { force: true } : {});
      if (force) toast.success(t('lockTakenOver'));
      reload();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
      if (e instanceof ApiError && e.status === 423) reload();
    } finally {
      setBusy(false);
    }
  };

  const releaseExplicitLock = async () => {
    if (!activeDraft?.id) return;
    setBusy(true);
    try {
      await api(`/admin/events/${plan.event.id}/plan/drafts/${activeDraft.id}/lock`, { method: 'DELETE' });
      setAcquiredLocks((prev) => {
        const next = new Set(prev);
        next.delete(activeDraft.id!);
        return next;
      });
      toast.success(t('lockReleased'));
      reload();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const submitNewDraftModal = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    try {
      const cloneSource = newDraftBase === 'clone' ? activeDraft : null;
      const body: Record<string, unknown> = {
        label: newDraftLabel.trim(),
      };
      if (cloneSource?.id != null) {
        body.clone_from_draft_id = cloneSource.id;
      } else if (newDraftBase === 'clone') {
        body.assignments = draftEntries(plan, draft);
        body.visitors = draftVisitors;
      }
      const res = await post<{ ok: boolean; draft: PlanDraft }>(
        `/admin/events/${plan.event.id}/plan/drafts`,
        body,
      );
      toast.success(t('draftCreated', { version: res.draft.version ?? '' }));
      setNewDraftOpen(false);
      setNewDraftLabel('');
      if (res.draft.id != null) {
        const nextParams = new URLSearchParams(searchParams);
        nextParams.set('draft', String(res.draft.id));
        setSearchParams(nextParams, { replace: true });
      }
      reload();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const deleteActiveDraft = async () => {
    if (!activeDraft?.id) return;
    const ok = await confirm({
      title: t('deleteDraftConfirmTitle', { version: activeDraft.version ?? '' }),
      body: <p>{t('deleteDraftConfirmBody')}</p>,
      confirmLabel: t('deleteDraft'),
      tone: 'danger',
    });
    if (!ok) return;
    setBusy(true);
    try {
      await api(`/admin/events/${plan.event.id}/plan/drafts/${activeDraft.id}`, { method: 'DELETE' });
      toast.success(t('draftDeleted'));
      const nextParams = new URLSearchParams(searchParams);
      nextParams.delete('draft');
      setSearchParams(nextParams, { replace: true });
      reload();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const toggleRideReimb = async (r: PlanRide) => {
    const nextExcluded = !r.reimbursement_excluded;
    setBusy(true);
    try {
      await api(`/rides/${r.id}/reimbursement-excluded`, {
        method: 'PUT',
        body: { excluded: nextExcluded },
      });
      toast.success(nextExcluded ? t('reimbExcludedSaved') : t('reimbIncludedSaved'));
      reload();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const draftStatusText = (status: PlanDraft['status']) => {
    if (status === 'pending_approval') return t('statusPendingApproval');
    if (status === 'returned') return t('statusReturned');
    if (status === 'approved') return t('statusApproved');
    if (status === 'superseded') return t('statusSuperseded');
    return t('statusDraft');
  };

  const draftStatusTone = (status: PlanDraft['status']) => {
    if (status === 'pending_approval') return 'warning' as const;
    if (status === 'returned') return 'danger' as const;
    if (status === 'approved') return 'success' as const;
    if (status === 'superseded') return 'neutral' as const;
    return 'info' as const;
  };

  const diffSummary = activeDraft ? summarizeDraftDiff(plan, draft, draftVisitors) : null;
  const canApproveDraftNow = !busy && (dirty || activeDraft?.status === 'pending_approval');
  const canToggleReimb = canApprove || canEditRides;

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
      canUpdate={canEditBoard}
    />
  );

  const pickerRide = pickerRideId != null ? plan.rides.find((r) => r.id === pickerRideId) ?? null : null;
  const displayedDraft = activeDraft ?? (selectedDraftKey === 'live' ? null : data.draft ?? null);
  const showDock = dirty && canEditBoard;
  const showVisitors = visitorsExpanded ?? (visitorCount.count > 0 || Boolean(activeDraft && draftVisitors.length > 0));

  return (
    <Page width="wide" className={`plan-page density-ops${showDock ? ' has-dock' : ''}`}>
      <PageHeader
        title={t('title')}
        subtitle={t('subtitle', { event: plan.event.title, when: fmtDateTime(plan.event.starts_at, ft) })}
        back={can('admin_dashboard') || canUpdate ? '/admin/traffic' : `/events/${plan.event.id}`}
        actions={
          displayedDraft?.status ? (
            <Pill tone={draftStatusTone(displayedDraft.status)}>
              {draftStatusText(displayedDraft.status)}
            </Pill>
          ) : undefined
        }
      />

      {(allDrafts.length > 0 || canUpdate) && (
        <div className="plan-draft-bar" role="region" aria-label={t('title')}>
          <div className="chips">
            <button
              type="button"
              className={`chip ${selectedDraftKey === 'live' ? 'on' : ''}`}
              onClick={() => selectDraftView('live')}
            >
              {t('liveView')}
            </button>
            {allDrafts.map((d, idx) => {
              const key = d.id ?? 'legacy';
              const isSelected = selectedDraftKey === key;
              return (
                <button
                  key={String(key) + '-' + idx}
                  type="button"
                  className={`chip plan-draft-chip ${isSelected ? 'on' : ''}`}
                  onClick={() => selectDraftView(key)}
                >
                  <span>
                    {d.version != null ? t('draftVersion', { version: d.version }) : t('statusDraft')}
                    {d.label ? ` · ${d.label}` : ''}
                  </span>
                  <span className="muted tiny">({draftStatusText(d.status)})</span>
                  {d.locked_by && <Lock size={12} aria-hidden="true" />}
                </button>
              );
            })}
          </div>
          <div className="btn-row ml-auto">
            {canUpdate && (
              <Button
                size="sm"
                variant="secondary"
                icon={<Plus size={14} />}
                disabled={busy}
                onClick={() => {
                  setNewDraftLabel('');
                  setNewDraftBase(activeDraft ? 'clone' : 'live');
                  setNewDraftOpen(true);
                }}
              >
                {t('newDraft')}
              </Button>
            )}
            {activeDraft?.id != null && canUpdate && draftStatusEditable && !lockedByOther && (
              isMyLock ? (
                <>
                  <Pill tone="info" className="plan-lock-pill" icon={<Lock size={12} />}>
                    {t('lockedByYou')}
                  </Pill>
                  <Button size="sm" variant="ghost" icon={<Unlock size={14} />} disabled={busy} onClick={releaseExplicitLock}>
                    {t('releaseLock')}
                  </Button>
                </>
              ) : (
                <Button size="sm" variant="secondary" icon={<Lock size={14} />} disabled={busy} onClick={() => acquireExplicitLock(false)}>
                  {t('lockForEditing')}
                </Button>
              )
            )}
            {activeDraft?.id != null &&
              activeDraft.status !== 'approved' &&
              !lockedByOther &&
              (canApprove || activeDraft.created_by?.id === me?.id) && (
                <Button size="sm" variant="ghost" icon={<Trash2 size={14} />} disabled={busy} onClick={deleteActiveDraft}>
                  {t('deleteDraft')}
                </Button>
              )}
          </div>
        </div>
      )}

      {lockedByOther && activeDraft?.locked_by && (
        <Notice tone="warning" icon={<Lock size={16} />}>
          <div className="row between items-center wrap gap-sm">
            <span>
              {t('lockedByOtherUntil', {
                name: activeDraft.locked_by.name,
                time: activeDraft.lock_expires_at ? fmtDateTime(activeDraft.lock_expires_at, ft) : '',
              })}
            </span>
            <div className="btn-row">
              {canUpdate && (
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={busy}
                  onClick={() => {
                    setNewDraftLabel('');
                    setNewDraftBase('clone');
                    setNewDraftOpen(true);
                  }}
                >
                  {t('cloneNewVersion')}
                </Button>
              )}
              {canApprove && activeDraft.id != null && (
                <Button size="sm" variant="primary" disabled={busy} onClick={() => acquireExplicitLock(true)}>
                  {t('takeOverEditing')}
                </Button>
              )}
            </div>
          </div>
        </Notice>
      )}

      {displayedDraft?.status === 'pending_approval' && (
        <Notice tone="warning" icon={<Clock size={16} />}>
          {t('bannerPendingApproval', {
            name: displayedDraft.submitted_by?.name || '',
            time: displayedDraft.submitted_at ? fmtDateTime(displayedDraft.submitted_at, ft) : '',
          })}
        </Notice>
      )}

      {displayedDraft?.status === 'returned' && (
        <Notice tone="danger" icon={<AlertCircle size={16} />}>
          {t('bannerReturned', { notes: displayedDraft.review_notes || '' })}
        </Notice>
      )}

      {canApprove && displayedDraft?.status === 'pending_approval' && diffSummary && (
        <Card className="plan-diff-card stack-xs">
          <div className="row between wrap gap-sm">
            <strong>{t('diffSummaryTitle')}</strong>
            <div className="row wrap gap-xs">
              {diffSummary.assigned > 0 && <Pill tone="success">{t('diffAssigned', { count: diffSummary.assigned })}</Pill>}
              {diffSummary.reassigned > 0 && <Pill tone="info">{t('diffReassigned', { count: diffSummary.reassigned })}</Pill>}
              {diffSummary.unassigned > 0 && <Pill tone="warning">{t('diffUnassigned', { count: diffSummary.unassigned })}</Pill>}
              {diffSummary.visitorChanges > 0 && <Pill tone="info">{t('diffVisitors', { count: diffSummary.visitorChanges })}</Pill>}
            </div>
          </div>
          {diffSummary.totalRideChanges === 0 && diffSummary.visitorChanges === 0 && (
            <p className="small muted no-margin">{t('diffNoRideChanges')}</p>
          )}
        </Card>
      )}

      {plan.rides.length > 0 && (
        <>
          <div className="plan-toolbar">
            <div className="plan-toolbar-primary">
              <div className="plan-summary" aria-live="polite">
                {t('summary', { placed, total: plan.rides.length, seats: seatsUsed })}
                {visitorCount.count > 0 && <> · {t('visitorSummary', visitorCount)}</>}
              </div>
              <div className="btn-row">
                {canUpdate && (
                  <>
                    <Button
                      variant="secondary"
                      icon={<Sparkles size={16} />}
                      onClick={autoArrange}
                      loading={arranging}
                      disabled={!canEditBoard || unassigned.length === 0 || arranging}
                    >
                      {t('autoArrange')}
                    </Button>
                    <Button variant="secondary" icon={<Save size={16} />} onClick={saveDraft} disabled={!canEditBoard || busy} loading={busy}>
                      {t('saveDraft')}
                    </Button>
                    {!canApprove ? (
                      <Button variant="primary" icon={<Send size={16} />} onClick={askLeadApproval} disabled={!canEditBoard || busy} loading={busy}>
                        {t('askLeadApproval')}
                      </Button>
                    ) : (
                      <>
                        {displayedDraft?.status === 'pending_approval' && (
                          <Button
                            variant="danger"
                            icon={<RotateCcw size={16} />}
                            onClick={() => {
                              setReturnNotes('');
                              setReturnModalOpen(true);
                            }}
                            disabled={busy}
                          >
                            {t('returnDraft')}
                          </Button>
                        )}
                        <Button variant="primary" className="plan-confirm" icon={<CheckCircle2 size={16} />} onClick={send} disabled={!canApproveDraftNow} loading={busy}>
                          {t('approveAndNotify', { count: changes.length })}
                        </Button>
                      </>
                    )}
                  </>
                )}
                <Button
                  variant="ghost"
                  size="sm"
                  className="plan-more-tools-btn"
                  aria-expanded={mobileToolsOpen}
                  onClick={() => setMobileToolsOpen((open) => !open)}
                >
                  {t('moreTools')}
                </Button>
              </div>
            </div>
            <div className={`btn-row plan-toolbar-secondary${mobileToolsOpen ? ' is-mobile-open' : ''}`}>
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
                    onClick={() => setDiagnosisOpen(true)}
                  >
                    {t('diagnosisBtn')}
                  </Button>
                  <Button variant="secondary" icon={<History size={16} />} onClick={sameAsLast} disabled={!canEditBoard || unassigned.length === 0}>
                    {t('sameAsLast')}
                  </Button>
                  {hasReturnRides && (
                    <Button variant="secondary" icon={<Repeat size={16} />} onClick={matchReturns} disabled={!canEditBoard}>
                      {t('matchReturns')}
                    </Button>
                  )}
                  <Button variant="ghost" icon={<Undo2 size={16} />} onClick={undoStep} disabled={!canEditBoard || undoStack.length === 0}>
                    {t('undoStep')}
                  </Button>
                  <Button variant="ghost" icon={<RotateCcw size={16} />} onClick={resetAll} disabled={!canEditBoard || !dirty}>
                    {t('reset')}
                  </Button>
                  <Button
                    variant="secondary"
                    icon={<Car size={16} />}
                    onClick={() => setShowPostEventReimb(true)}
                  >
                    {t('postEventReimbBtn', { ns: 'events' })}
                  </Button>
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
            <button
              type="button"
              className={`chip plan-visitors-chip ${showVisitors ? 'on' : ''}`}
              aria-expanded={showVisitors}
              onClick={() => setVisitorsExpanded(!showVisitors)}
            >
              {t('visitorsChip', { count: visitorCount.count })}
            </button>
          </div>
        </>
      )}
      {dirty && <p className="plan-unsaved">{t('unsaved')}</p>}

      {(showVisitors || plan.rides.length === 0) && (
        <PlanVisitors
          eventId={plan.event.id}
          visitors={baseVisitors}
          drivers={plan.drivers}
          draftMode={Boolean(activeDraft)}
          draftVisitors={activeDraft ? draftVisitors : undefined}
          onChangeDraftVisitors={activeDraft ? handleDraftVisitorsChange : undefined}
          // On Live, only the traffic lead records visitors directly; inside a draft, members propose visitor changes too.
          canUpdate={
            activeDraft
              ? canEditBoard && plan.event.status !== 'cancelled'
              : canUpdate && canApprove && plan.event.status !== 'cancelled'
          }
          canManageReimb={canUpdate && canApprove && plan.event.status !== 'cancelled'}
          onChanged={reloadVisitors}
        />
      )}

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
                const carVisitors = visitorList.filter((v) => v.driver_id === d.id);
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
                      {mine.length === 0 && carVisitors.length === 0 ? <p className="plan-drop-hint">{t('dropHere')}</p> : mine.map(card)}
                      {carVisitors.map((v) => (
                        <VisitorLine key={`v${v.id}`} v={v} compact />
                      ))}
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
            {pickerRide.pickup_address && (
              <p className="muted small no-margin">
                {[pickerRide.area.name, fmtTime(pickerRide.pickup_at, ft), pickerRide.pickup_address].filter(Boolean).join(' · ')}
              </p>
            )}
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
            {canToggleReimb && (
              <>
                <div className="divider" />
                <div className="row end">
                  <Button
                    size="sm"
                    variant="ghost"
                    disabled={busy}
                    onClick={() => toggleRideReimb(pickerRide)}
                  >
                    {pickerRide.reimbursement_excluded ? t('toggleReimbInclude') : t('toggleReimbExclude')}
                  </Button>
                </div>
              </>
            )}
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

      {dirty && canEditBoard && (
        <aside className="plan-floating-dock" aria-label={t('floatingDockLabel')}>
          <div className="plan-floating-dock-content">
            <div className="plan-floating-dock-status">
              <span className="plan-dock-dot" aria-hidden="true" />
              <span className="bold">{t('floatingChanges', { count: changes.length })}</span>
            </div>
            <div className="btn-row nowrap">
              <Button
                variant="ghost"
                size="sm"
                icon={<Undo2 size={16} />}
                onClick={undoStep}
                disabled={undoStack.length === 0}
                title="Ctrl+Z"
                aria-label={t('dockUndo')}
              >
                {t('dockUndo')}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                icon={<RotateCcw size={16} />}
                onClick={resetAll}
                disabled={!dirty}
                aria-label={t('dockReset')}
              >
                {t('dockReset')}
              </Button>
              <Button
                variant="secondary"
                size="sm"
                icon={<Save size={16} />}
                onClick={saveDraft}
                disabled={busy}
                loading={busy}
                aria-label={t('dockSave')}
              >
                {t('dockSave')}
              </Button>
              {!canApprove ? (
                <Button
                  variant="primary"
                  size="sm"
                  icon={<Send size={16} />}
                  onClick={askLeadApproval}
                  disabled={busy}
                  loading={busy}
                  aria-label={t('dockAskApproval')}
                >
                  {t('dockAskApproval')}
                </Button>
              ) : (
                <Button
                  variant="primary"
                  size="sm"
                  icon={<CheckCircle2 size={16} />}
                  onClick={send}
                  disabled={!canApproveDraftNow}
                  loading={busy}
                  aria-label={t('dockApprove', { count: changes.length })}
                >
                  {t('dockApprove', { count: changes.length })}
                </Button>
              )}
            </div>
          </div>
        </aside>
      )}

      {diagnosisOpen && (
        <DiagnosisDrawerModal
          plan={plan}
          draft={draft}
          reasons={reasons}
          noRoom={noRoom}
          lastResult={lastArrangeResult}
          canUpdate={canEditBoard}
          onAssign={(rideId, driverId) => {
            ensureLock();
            setUndoStack((s) => [...s.slice(-24), draft]);
            setDraft((d) => ({ ...d, [rideId]: driverId }));
          }}
          onClose={() => setDiagnosisOpen(false)}
        />
      )}

      {newDraftOpen && (
        <Modal
          title={t('newDraftTitle')}
          onClose={() => {
            setNewDraftOpen(false);
            setNewDraftLabel('');
          }}
        >
          <form className="stack" onSubmit={submitNewDraftModal}>
            <Field label={t('newDraftLabel')}>
              <input
                className="input"
                aria-label={t('newDraftLabel')}
                maxLength={60}
                value={newDraftLabel}
                placeholder={t('newDraftLabelPlaceholder')}
                onChange={(e) => setNewDraftLabel(e.target.value)}
              />
            </Field>
            <Field label={t('newDraftBase')}>
              <select
                className="input"
                aria-label={t('newDraftBase')}
                value={newDraftBase}
                onChange={(e) => setNewDraftBase(e.target.value as 'live' | 'clone')}
              >
                <option value="live">{t('newDraftFromLive')}</option>
                {activeDraft && (
                  <option value="clone">
                    {t('newDraftCloneCurrent', { version: activeDraft.version ?? 1 })}
                  </option>
                )}
              </select>
            </Field>
            <div className="btn-row end">
              <Button
                type="button"
                variant="ghost"
                onClick={() => {
                  setNewDraftOpen(false);
                  setNewDraftLabel('');
                }}
              >
                {t('cancel', { ns: 'common' })}
              </Button>
              <Button type="submit" variant="primary" disabled={busy} loading={busy}>
                {t('createDraft')}
              </Button>
            </div>
          </form>
        </Modal>
      )}

      {returnModalOpen && (
        <Modal
          title={t('returnModalTitle')}
          onClose={() => {
            setReturnModalOpen(false);
            setReturnNotes('');
          }}
        >
          <form className="stack" onSubmit={submitReturnModal}>
            <div className="chips quick-reason-chips">
              {[
                t('returnChipRoute'),
                t('returnChipCapacity'),
                t('returnChipDriver'),
                t('returnChipArea'),
              ].map((chip) => (
                <button
                  key={chip}
                  type="button"
                  className="chip"
                  onClick={() => setReturnNotes((prev) => (prev ? `${prev}; ${chip}` : chip))}
                >
                  {chip}
                </button>
              ))}
            </div>
            <Field label={t('returnModalLabel')}>
              <textarea
                className="input"
                aria-label={t('returnModalLabel')}
                rows={3}
                value={returnNotes}
                placeholder={t('returnModalPlaceholder')}
                onChange={(e) => setReturnNotes(e.target.value)}
                required
              />
            </Field>
            <div className="btn-row end">
              <Button
                type="button"
                variant="ghost"
                onClick={() => {
                  setReturnModalOpen(false);
                  setReturnNotes('');
                }}
              >
                {t('cancel', { ns: 'common' })}
              </Button>
              <Button type="submit" variant="danger" disabled={busy || !returnNotes.trim()} loading={busy}>
                {t('returnModalSubmit')}
              </Button>
            </div>
          </form>
        </Modal>
      )}
    </Page>
  );
}

/** Explainability drawer breaking down matching rationale, carpool clustering and unassigned bottlenecks */
function DiagnosisDrawerModal({
  plan,
  draft,
  reasons,
  noRoom,
  lastResult,
  canUpdate,
  onAssign,
  onClose,
}: {
  plan: Plan;
  draft: Draft;
  reasons: Record<number, Reason>;
  noRoom: Set<number>;
  lastResult?: ArrangeResult | null;
  canUpdate: boolean;
  onAssign: (rideId: number, driverId: number | null) => void;
  onClose: () => void;
}) {
  const { t } = useTranslation('plan');
  const driversById = useMemo(() => new Map(plan.drivers.map((d) => [d.id, d])), [plan.drivers]);

  const placedRides = useMemo(
    () => plan.rides.filter((r) => draft[r.id] != null),
    [plan.rides, draft],
  );
  const unassignedRides = useMemo(
    () => plan.rides.filter((r) => draft[r.id] == null),
    [plan.rides, draft],
  );

  const clusterCount = useMemo(
    () => placedRides.filter((r) => reasons[r.id]?.near != null).length,
    [placedRides, reasons],
  );

  return (
    <Modal title={t('diagnosisTitle')} onClose={onClose} size="lg">
      <div className="stack gap-md">
        <p className="muted small">{t('diagnosisDesc')}</p>
        {lastResult && lastResult.approximate > 0 && (
          <Pill tone="info">{t('autoArrangeApprox', { count: lastResult.approximate })}</Pill>
        )}

        {/* Top metrics summary */}
        <div className="row wrap gap-sm" role="region" aria-label="Summary statistics">
          <div className="card flat tight plan-diag-metric">
            <div className="muted tiny">Placed Rides</div>
            <div className="bold plan-diag-val">{placedRides.length} / {plan.rides.length}</div>
          </div>
          <div className="card flat tight plan-diag-metric">
            <div className="muted tiny">Carpool Clusters</div>
            <div className="bold plan-diag-val text-success">{clusterCount} rides</div>
          </div>
          <div className="card flat tight plan-diag-metric">
            <div className="muted tiny">Unassigned</div>
            <div className={`bold plan-diag-val ${unassignedRides.length > 0 ? 'text-warning' : 'text-muted'}`}>
              {unassignedRides.length}
            </div>
          </div>
        </div>

        {/* Unassigned Bottlenecks Section */}
        <div className="stack-sm">
          <h4 className="no-margin">{t('diagnosisUnassigned', { count: unassignedRides.length })}</h4>
          {unassignedRides.length === 0 ? (
            <p className="muted small">{t('diagnosisNoUnassigned')}</p>
          ) : (
            <ul className="stack-xs plan-diag-list">
              {unassignedRides.map((r) => {
                const areaDrivers = plan.drivers.filter((d) => d.area_ids.includes(r.area.id));
                const allFull =
                  areaDrivers.length > 0 &&
                  areaDrivers.every((d) => driverLoad(plan, draft, d).seats >= d.cap);
                const diagnosisMsg = r.assistance
                  ? t('diagnosisBottleneckAssistance', { assistance: r.assistance })
                  : areaDrivers.length === 0
                    ? t('diagnosisBottleneckNoDriver', { area: r.area.name })
                    : allFull
                      ? t('diagnosisBottleneckCapacity', {
                          area: r.area.name,
                          drivers: areaDrivers.map((d) => d.name).join(', '),
                        })
                      : t('noRoom');

                return (
                  <li key={r.id} className="card flat tight stack-xs plan-diag-warn">
                    <div className="row between wrap gap-xs">
                      <div className="bold small">
                        <span>{r.passenger.name}</span> · <span className="muted">{r.area.name}</span>
                        {r.assistance && <Pill tone="warning">{r.assistance}</Pill>}
                        {noRoom.has(r.id) && <Pill tone="danger">{t('noRoom')}</Pill>}
                      </div>
                      <div className="muted tiny">{r.pickup_address}</div>
                    </div>
                    <p className="small text-warning no-margin">{diagnosisMsg}</p>
                    {canUpdate && (
                      <div className="row nowrap gap-xs align-center plan-diag-assign">
                        <span className="muted tiny">{t('diagnosisActionAssign')}</span>
                        <select
                          className="input plan-diag-select"
                          aria-label={`Driver for ${r.passenger.name}`}
                          value=""
                          onChange={(e) => {
                            const val = e.target.value ? Number(e.target.value) : null;
                            if (val != null) onAssign(r.id, val);
                          }}
                        >
                          <option value="">-- Choose driver --</option>
                          {plan.drivers.map((d) => {
                            const used = driverLoad(plan, draft, d).seats;
                            return (
                              <option key={d.id} value={d.id} disabled={used >= d.cap}>
                                {d.name} ({used}/{d.cap} seats{used >= d.cap ? ' - Full' : ''})
                              </option>
                            );
                          })}
                        </select>
                      </div>
                    )}
                  </li>
                );
              })}
            </ul>
          )}
        </div>

        {/* Successfully Placed Clusters Section */}
        {placedRides.length > 0 && (
          <div className="stack-sm">
            <h4 className="no-margin">{t('diagnosisMatched', { count: placedRides.length })}</h4>
            <div className="grid gap-xs" style={{ maxHeight: '220px', overflowY: 'auto' }}>
              {placedRides.map((r) => {
                const driver = driversById.get(draft[r.id]!);
                const reason = reasons[r.id];
                return (
                  <div key={r.id} className="card flat tight row between wrap gap-xs">
                    <div>
                      <span className="bold small">{r.passenger.name}</span>
                      <span className="muted tiny"> ➔ {driver?.name ?? 'Assigned'}</span>
                    </div>
                    <div>
                      {reason?.near ? (
                        <Pill tone="success">
                          {t('diagnosisClusterKm', { km: reason.near.km, name: reason.near.name })}
                        </Pill>
                      ) : reason?.in_area ? (
                        <Pill tone="info">
                          {t('diagnosisSameAreaDriver', { driver: driver?.name ?? '' })}
                        </Pill>
                      ) : (
                        <Pill tone="gray">{t('assigned')}</Pill>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        <div className="row end">
          <Button variant="primary" onClick={onClose}>
            {t('common:close')}
          </Button>
        </div>
      </div>
    </Modal>
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
  const [dragging, setDragging] = useState(false);
  const current = draft[r.id] ?? null;
  const driver = plan.drivers.find((d) => d.id === current);
  const selectId = `plan-ride-${r.id}`;
  const metaLine = [r.area.name, fmtTime(r.pickup_at, ft), r.pickup_address].filter(Boolean).join(' · ');
  return (
    <article
      className={`plan-passenger ${changed ? 'is-changed' : ''}${dragging ? ' is-dragging' : ''}`}
      draggable={canUpdate}
      onDragStart={(e) => {
        if (!canUpdate) return;
        setDragging(true);
        e.dataTransfer.setData(DRAG_TYPE, String(r.id));
        e.dataTransfer.effectAllowed = 'move';
      }}
      onDragEnd={() => setDragging(false)}
    >
      <div className="plan-passenger-head">
        <strong className="plan-passenger-name">
          <UserRound size={15} aria-hidden="true" />
          <span className="ellipsis">{r.passenger.name}</span>
        </strong>
        <div className="plan-passenger-head-end">
          {changed ? <Pill tone="info">{t('changed')}</Pill> : <StatusPill map={RIDE_STATUS} status={r.status} />}
          {canUpdate && (
            <IconButton
              size="sm"
              variant="secondary"
              className="plan-picker-icon-btn"
              label={t('chooseDriverSheet', { name: r.passenger.name })}
              icon={<Car size={16} />}
              onClick={onOpenPicker}
            />
          )}
        </div>
      </div>
      <div className="plan-passenger-meta ellipsis" title={r.pickup_address || undefined}>
        {metaLine}
      </div>
      <div className="plan-tags">
        {r.seats > 1 && <Pill tone="neutral">{t('paxTag', { count: r.seats })}</Pill>}
        {r.leg === 'return' && <Pill tone="info">{t('returnTag')}</Pill>}
        {r.reimbursement_excluded && <Pill tone="warning">{t('noReimbTag')}</Pill>}
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
      {canUpdate && (
        <button
          type="button"
          className="plan-mobile-assign-btn"
          onClick={onOpenPicker}
        >
          <span className="ellipsis">{driver ? driver.name : t('noDriver')}</span>
          <span className="plan-mobile-assign-cta" aria-hidden="true">›</span>
        </button>
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
    </article>
  );
}
