import { FormEvent, useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import {
  AlertCircle,
  Calendar,
  CheckCircle2,
  ClipboardList,
  Clock,
  Lock,
  Map as MapIcon,
  MapPinned,
  RotateCcw,
  Route,
} from 'lucide-react';
import { post, useApi } from '../../api';
import { useAuth } from '../../auth';
import { canApprovePlan } from '../../perms';
import type { AdminCounts } from '../../staffNav';
import {
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Field,
  Modal,
  Notice,
  Page,
  PageHeader,
  Pill,
  Tabs,
  useAction,
  type PillTone,
  type TabItem,
} from '../../components/ui';
import { fmtDateTime, todayStr, useFormatT } from '../../format';
import { AdminRidesContent } from './Rides';
import { AdminLiveContent } from './Live';
import AreaPhotos from './AreaPhotos';
import AreaFares from './AreaFares';
import { Areas } from './Settings';

type HubTab = 'plans' | 'rides' | 'live' | 'areas';
type PlanFilter = 'all' | 'pending' | 'needs_plan' | 'returned' | 'has_draft';
type RidesView = 'all' | 'drafts';
type AreaSubTab = 'photos' | 'fares' | 'areas';

interface TrafficDashCounts extends AdminCounts {
  live_drivers?: number;
}

export interface TrafficPlanDraftSummary {
  id: number;
  event_id: number;
  version: number;
  label: string;
  status: 'draft' | 'pending_approval' | 'approved' | 'returned' | 'superseded';
  created_by: { id: number; name: string };
  updated_by: { id: number; name: string };
  updated_at: string;
  submitted_by?: { id: number; name: string } | null;
  locked_by?: { id: number; name: string } | null;
  lock_is_mine?: boolean;
}

export interface TrafficEventPlan {
  id: number;
  title: string;
  kind: string;
  icon: string | null;
  starts_at: string;
  location: string;
  status: string;
  area_name: string | null;
  groups: string;
  mileage_claimable: boolean;
  ride_count: number;
  unassigned_count: number;
  pending_confirm_count: number;
  visitor_count: number;
  draft_count: number;
  pending_draft_count: number;
  returned_draft_count: number;
  latest_draft: TrafficPlanDraftSummary | null;
}

export interface HubRideDraft {
  id: number;
  ride_id: number;
  version: number;
  action: 'approve' | 'reassign' | 'return' | 'alt_transport' | 'reimbursement_excluded';
  driver_id: number | null;
  driver_name: string | null;
  reason: string;
  reimbursement_excluded: boolean | null;
  status: 'draft' | 'pending_approval' | 'approved' | 'returned' | 'superseded';
  created_by: { id: number; name: string };
  updated_by: { id: number; name: string };
  submitted_by?: { id: number; name: string } | null;
  submitted_at?: string | null;
  review_notes: string;
  locked_by?: { id: number; name: string } | null;
  ride: {
    id: number;
    status: string;
    pickup_at: string;
    passenger_name: string;
    area_name: string;
    current_driver_name: string | null;
    event_id: number | null;
    event_title: string | null;
  };
}

function draftStatusTone(status: string): PillTone {
  if (status === 'pending_approval') return 'warning';
  if (status === 'returned') return 'danger';
  if (status === 'approved') return 'success';
  if (status === 'superseded') return 'neutral';
  return 'info';
}

export default function TrafficHub() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const { run, busy } = useAction();
  const auth = useAuth();
  const me = auth?.me;
  const can = auth?.can ?? (() => true);
  const isLead = canApprovePlan(me?.roles);
  const [searchParams, setSearchParams] = useSearchParams();

  const canPlans = can('admin_plan');
  const canRides = can('admin_rides');
  const canLive = can('admin_live');
  const canAreaPhotos = can('admin_area_photos');
  const canFares = can('admin_fares');
  const canManageAreas = can('admin_areas');
  const canAreas = canAreaPhotos || canFares || canManageAreas;

  const dash = useApi<TrafficDashCounts>(can('admin_dashboard') ? '/admin/dashboard' : null, 30000);
  const plans = useApi<TrafficEventPlan[]>(canPlans ? '/admin/traffic/plans' : null, 30000);
  const rideDrafts = useApi<HubRideDraft[]>(canRides ? '/admin/ride-drafts' : null, 30000);

  const pendingPlanCount = useMemo(
    () => (plans.data ?? []).filter((e) => e.pending_draft_count > 0).length || (dash.data?.pending_plan_drafts ?? 0),
    [plans.data, dash.data?.pending_plan_drafts],
  );
  const returnedPlanCount = useMemo(
    () => (plans.data ?? []).filter((e) => e.returned_draft_count > 0).length || (dash.data?.my_returned_plan_drafts ?? 0),
    [plans.data, dash.data?.my_returned_plan_drafts],
  );
  const pendingRideDraftCount = useMemo(
    () => (rideDrafts.data ?? []).filter((d) => d.status === 'pending_approval').length || (dash.data?.pending_ride_drafts ?? 0),
    [rideDrafts.data, dash.data?.pending_ride_drafts],
  );
  const returnedRideDraftCount = useMemo(
    () => (rideDrafts.data ?? []).filter((d) => d.status === 'returned' && d.created_by.id === me?.id).length
      || (dash.data?.my_returned_ride_drafts ?? 0),
    [rideDrafts.data, dash.data?.my_returned_ride_drafts, me?.id],
  );

  const liveCount = dash.data?.live_drivers ?? 0;

  const tabs: TabItem[] = useMemo(() => {
    const list: TabItem[] = [];
    if (canPlans) {
      const c = isLead ? pendingPlanCount : returnedPlanCount;
      list.push({
        value: 'plans',
        label: t('trafficHub.tabPlans'),
        icon: <Route size={16} />,
        count: c > 0 ? c : undefined,
      });
    }
    if (canRides) {
      const c = isLead
        ? pendingRideDraftCount + (dash.data?.pending_approval ?? 0) + (dash.data?.no_driver ?? 0)
        : returnedRideDraftCount;
      list.push({
        value: 'rides',
        label: t('trafficHub.tabRides'),
        icon: <ClipboardList size={16} />,
        count: c > 0 ? c : undefined,
      });
    }
    if (canLive) {
      list.push({
        value: 'live',
        label: liveCount > 0 ? (
          <span className="row items-center gap-xs">
            <span className="dash-live-dot on" aria-hidden="true" />
            <span>{t('trafficHub.tabLive')}</span>
          </span>
        ) : t('trafficHub.tabLive'),
        icon: <MapPinned size={16} />,
        count: liveCount > 0 ? liveCount : undefined,
      });
    }
    if (canAreas) {
      list.push({
        value: 'areas',
        label: t('trafficHub.tabAreas'),
        icon: <MapIcon size={16} />,
      });
    }
    return list;
  }, [
    canPlans,
    canRides,
    canLive,
    canAreas,
    isLead,
    pendingPlanCount,
    returnedPlanCount,
    pendingRideDraftCount,
    returnedRideDraftCount,
    dash.data?.pending_approval,
    dash.data?.no_driver,
    liveCount,
    t,
  ]);

  const rawTab = searchParams.get('tab') as HubTab | null;
  const activeTab: HubTab = (rawTab && tabs.some((item) => item.value === rawTab)
    ? rawTab
    : (tabs[0]?.value as HubTab | undefined)) ?? 'plans';

  const [planFilter, setPlanFilter] = useState<PlanFilter>('all');
  const ridesView: RidesView = searchParams.get('view') === 'drafts' ? 'drafts' : 'all';

  const [returningRideDraft, setReturningRideDraft] = useState<HubRideDraft | null>(null);
  const [returnNotes, setReturnNotes] = useState('');

  const setTab = (nextTab: string, extra?: Record<string, string>) => {
    const next = new URLSearchParams(searchParams);
    next.set('tab', nextTab);
    if (nextTab !== 'rides') next.delete('view');
    if (nextTab !== 'areas') next.delete('sub');
    if (extra) {
      for (const [k, v] of Object.entries(extra)) {
        next.set(k, v);
      }
    }
    setSearchParams(next, { replace: true });
  };

  const areaSubTabs = useMemo(() => {
    const list: { key: AreaSubTab; label: string }[] = [];
    if (canAreaPhotos) list.push({ key: 'photos', label: t('trafficHub.areasSubPhotos') });
    if (canFares) list.push({ key: 'fares', label: t('trafficHub.areasSubFares') });
    if (canManageAreas) list.push({ key: 'areas', label: t('trafficHub.areasSubAreas') });
    return list;
  }, [canAreaPhotos, canFares, canManageAreas, t]);
  const rawSub = searchParams.get('sub') as AreaSubTab | null;
  const activeAreaSub: AreaSubTab | undefined =
    rawSub && areaSubTabs.some((s) => s.key === rawSub) ? rawSub : areaSubTabs[0]?.key;
  const setAreaSub = (sub: AreaSubTab) => setTab('areas', { sub });

  const setRidesView = (view: RidesView) => {
    const next = new URLSearchParams(searchParams);
    next.set('tab', 'rides');
    if (view === 'drafts') next.set('view', 'drafts');
    else next.delete('view');
    setSearchParams(next, { replace: true });
  };

  const draftStatusLabel = (status: string) => {
    if (status === 'pending_approval') return t('trafficHub.statusPending');
    if (status === 'returned') return t('trafficHub.statusReturned');
    if (status === 'approved') return t('trafficHub.statusApproved');
    if (status === 'superseded') return t('trafficHub.statusSuperseded');
    return t('trafficHub.statusDraft');
  };

  const planHrefFor = (ev: TrafficEventPlan) =>
    ev.latest_draft && !['approved', 'superseded'].includes(ev.latest_draft.status)
      ? `/admin/events/${ev.id}/plan?draft=${ev.latest_draft.id}`
      : `/admin/events/${ev.id}/plan`;

  const planCtaLabel = (ev: TrafficEventPlan) => {
    if (isLead && ev.pending_draft_count > 0) return t('trafficHub.reviewDraft');
    if (ev.latest_draft && ['draft', 'returned'].includes(ev.latest_draft.status)) return t('trafficHub.continueDraft');
    if (ev.draft_count === 0) return t('trafficHub.startDraft');
    return t('trafficHub.openPlan');
  };

  const totalPendingForLead = isLead ? pendingPlanCount + pendingRideDraftCount : 0;
  const totalReturnedForMember = !isLead ? returnedPlanCount + returnedRideDraftCount : 0;

  const handleReviewBannerClick = () => {
    if (pendingPlanCount > 0 && canPlans) {
      setPlanFilter('pending');
      setTab('plans');
    } else if (pendingRideDraftCount > 0 && canRides) {
      setTab('rides', { view: 'drafts' });
    }
  };

  const handleReturnedBannerClick = () => {
    if (returnedPlanCount > 0 && canPlans) {
      setPlanFilter('returned');
      setTab('plans');
    } else if (returnedRideDraftCount > 0 && canRides) {
      setTab('rides', { view: 'drafts' });
    }
  };

  const today = todayStr();
  const planList = useMemo(() => plans.data ?? [], [plans.data]);
  const nextEvent = useMemo(
    () => planList.find((e) => e.starts_at.slice(0, 10) >= today) ?? null,
    [planList, today],
  );

  const filteredPlans = useMemo(
    () =>
      planList.filter((ev) => {
        if (planFilter === 'pending') return ev.pending_draft_count > 0;
        if (planFilter === 'needs_plan') return ev.unassigned_count > 0;
        if (planFilter === 'returned') return ev.returned_draft_count > 0;
        if (planFilter === 'has_draft') return ev.draft_count > 0;
        return true;
      }),
    [planList, planFilter],
  );

  const rideActionLabel = (d: HubRideDraft) => {
    if (d.action === 'approve') return t('trafficHub.actionApprove');
    if (d.action === 'reassign') return t('trafficHub.actionReassign', { driver: d.driver_name ?? '—' });
    if (d.action === 'return') return t('trafficHub.actionReturn');
    if (d.action === 'alt_transport') return t('trafficHub.actionAltTransport');
    return d.reimbursement_excluded ? t('trafficHub.actionReimbExcluded') : t('trafficHub.actionReimbIncluded');
  };

  const handleApproveRideDraft = async (d: HubRideDraft) => {
    const res = await run(
      () => post(`/rides/${d.ride_id}/drafts/${d.id}/approve`, {}),
      t('trafficHub.draftApprovedToast'),
    );
    if (res !== undefined) {
      rideDrafts.reload();
      dash.reload();
    }
  };

  const handleReturnRideDraft = async (e: FormEvent) => {
    e.preventDefault();
    if (!returningRideDraft) return;
    const notes = returnNotes.trim();
    if (!notes) return;
    const res = await run(
      () => post(`/rides/${returningRideDraft.ride_id}/drafts/${returningRideDraft.id}/return`, { notes }),
      t('trafficHub.draftReturnedToast'),
    );
    if (res !== undefined) {
      setReturningRideDraft(null);
      setReturnNotes('');
      rideDrafts.reload();
      dash.reload();
    }
  };

  const quickChips = [
    t('trafficHub.quickReasonRoute'),
    t('trafficHub.quickReasonCapacity'),
    t('trafficHub.quickReasonDriver'),
  ];

  return (
    <Page width="wide">
      <PageHeader title={t('trafficHub.title')} subtitle={t('trafficHub.desc')} />

      <div className="stack">
        {totalPendingForLead > 0 && (
          <Notice tone="warning" icon={<Clock size={16} />}>
            <div className="row between items-center wrap gap-sm">
              <span>{t('trafficHub.leadBanner', { count: totalPendingForLead })}</span>
              <Button size="sm" variant="primary" onClick={handleReviewBannerClick}>
                {t('trafficHub.reviewNow')}
              </Button>
            </div>
          </Notice>
        )}

        {totalReturnedForMember > 0 && (
          <Notice tone="danger" icon={<AlertCircle size={16} />}>
            <div className="row between items-center wrap gap-sm">
              <span>{t('trafficHub.memberBanner', { count: totalReturnedForMember })}</span>
              <Button size="sm" variant="secondary" onClick={handleReturnedBannerClick}>
                {t('trafficHub.reviewNow')}
              </Button>
            </div>
          </Notice>
        )}

        {tabs.length > 0 && (
          <Tabs
            value={activeTab}
            onChange={(v) => setTab(v)}
            items={tabs}
            ariaLabel={t('trafficHub.title')}
          />
        )}

        {activeTab === 'plans' && canPlans && (
          <div className="stack">
            <ErrorBox error={plans.error} onRetry={plans.reload} />
            {plans.loading && !plans.data ? (
              <CardSkeleton count={3} />
            ) : (
              <>
                {nextEvent && (
                  <Card className="traffic-next-event stack-sm">
                    <div className="row between wrap gap-sm">
                      <div className="stack-xs">
                        <span className="eyebrow row items-center gap-xs">
                          <Calendar size={14} aria-hidden="true" />
                          <span>{t('trafficHub.nextEventEyebrow')}</span>
                        </span>
                        <h2 className="no-margin">{nextEvent.title}</h2>
                        <p className="small muted no-margin">
                          {[
                            fmtDateTime(nextEvent.starts_at, ft),
                            nextEvent.area_name || nextEvent.location,
                            nextEvent.groups,
                          ]
                            .filter(Boolean)
                            .join(' · ')}
                        </p>
                      </div>
                      <Button variant="primary" to={planHrefFor(nextEvent)}>
                        {planCtaLabel(nextEvent)}
                      </Button>
                    </div>
                    <div className="row wrap gap-xs">
                      <Pill tone="neutral">{t('trafficHub.ridesCount', { count: nextEvent.ride_count })}</Pill>
                      {nextEvent.unassigned_count > 0 && (
                        <Pill tone="warning">{t('trafficHub.unassignedCount', { count: nextEvent.unassigned_count })}</Pill>
                      )}
                      {nextEvent.visitor_count > 0 && (
                        <Pill tone="info">{t('trafficHub.visitorsCount', { count: nextEvent.visitor_count })}</Pill>
                      )}
                      {nextEvent.draft_count > 0 && (
                        <Pill tone="neutral">{t('trafficHub.draftVersionsCount', { count: nextEvent.draft_count })}</Pill>
                      )}
                      {nextEvent.latest_draft && (
                        <Pill tone={draftStatusTone(nextEvent.latest_draft.status)}>
                          {t('trafficHub.latestDraftLabel', {
                            version: nextEvent.latest_draft.version,
                            status: draftStatusLabel(nextEvent.latest_draft.status),
                            name: nextEvent.latest_draft.updated_by.name,
                          })}
                        </Pill>
                      )}
                      {nextEvent.latest_draft?.locked_by && (
                        <Pill tone="warning" icon={<Lock size={12} />}>
                          {t('trafficHub.editingLock', { name: nextEvent.latest_draft.locked_by.name })}
                        </Pill>
                      )}
                    </div>
                  </Card>
                )}

                <div className="chips" role="group" aria-label={t('trafficHub.tabPlans')}>
                  {(
                    [
                      ['all', t('trafficHub.filterAll')],
                      ['pending', t('trafficHub.filterPending')],
                      ['needs_plan', t('trafficHub.filterNeedsPlan')],
                      ['returned', t('trafficHub.filterReturned')],
                      ['has_draft', t('trafficHub.filterHasDraft')],
                    ] as const
                  ).map(([key, label]) => (
                    <button
                      key={key}
                      type="button"
                      className={`chip ${planFilter === key ? 'on' : ''}`}
                      onClick={() => setPlanFilter(key)}
                    >
                      {label}
                    </button>
                  ))}
                </div>

                {filteredPlans.length === 0 ? (
                  <EmptyState title={t('trafficHub.emptyPlans')} />
                ) : (
                  <div className="stack-sm">
                    {filteredPlans.map((ev) => (
                      <Card key={ev.id} className="stack-xs">
                        <div className="row between wrap gap-sm">
                          <div className="stack-xs">
                            <div className="row wrap items-center gap-xs">
                              <strong>{ev.title}</strong>
                              {ev.latest_draft && (
                                <Pill tone={draftStatusTone(ev.latest_draft.status)}>
                                  v{ev.latest_draft.version} · {draftStatusLabel(ev.latest_draft.status)}
                                </Pill>
                              )}
                              {ev.latest_draft?.locked_by && (
                                <Pill tone="warning" icon={<Lock size={12} />}>
                                  {t('trafficHub.editingLock', { name: ev.latest_draft.locked_by.name })}
                                </Pill>
                              )}
                            </div>
                            <div className="small muted">
                              {[fmtDateTime(ev.starts_at, ft), ev.area_name || ev.location, ev.groups]
                                .filter(Boolean)
                                .join(' · ')}
                            </div>
                          </div>
                          <Button variant="secondary" size="sm" to={planHrefFor(ev)}>
                            {planCtaLabel(ev)}
                          </Button>
                        </div>
                        <div className="row wrap gap-xs small">
                          <Pill tone="neutral">{t('trafficHub.ridesCount', { count: ev.ride_count })}</Pill>
                          {ev.unassigned_count > 0 && (
                            <Pill tone="warning">{t('trafficHub.unassignedCount', { count: ev.unassigned_count })}</Pill>
                          )}
                          {ev.visitor_count > 0 && (
                            <Pill tone="info">{t('trafficHub.visitorsCount', { count: ev.visitor_count })}</Pill>
                          )}
                          {ev.draft_count > 0 && (
                            <Pill tone="neutral">{t('trafficHub.draftVersionsCount', { count: ev.draft_count })}</Pill>
                          )}
                        </div>
                      </Card>
                    ))}
                  </div>
                )}
              </>
            )}
          </div>
        )}

        {activeTab === 'rides' && canRides && (
          <div className="stack">
            <div className="chips" role="group" aria-label={t('trafficHub.tabRides')}>
              <button
                type="button"
                className={`chip ${ridesView === 'all' ? 'on' : ''}`}
                onClick={() => setRidesView('all')}
              >
                {t('trafficHub.ridesViewAll')}
              </button>
              <button
                type="button"
                className={`chip ${ridesView === 'drafts' ? 'on' : ''}`}
                onClick={() => setRidesView('drafts')}
              >
                {t('trafficHub.ridesViewDrafts')}
                {pendingRideDraftCount > 0 ? ` (${pendingRideDraftCount})` : ''}
              </button>
            </div>

            {ridesView === 'all' ? (
              <AdminRidesContent showExportTop={false} />
            ) : (
              <div className="stack-sm">
                <ErrorBox error={rideDrafts.error} onRetry={rideDrafts.reload} />
                {rideDrafts.loading && !rideDrafts.data ? (
                  <CardSkeleton count={3} />
                ) : !(rideDrafts.data ?? []).length ? (
                  <EmptyState title={t('trafficHub.emptyRideDrafts')} />
                ) : (
                  (rideDrafts.data ?? []).map((d) => (
                    <Card key={d.id} className="stack-xs">
                      <div className="row between wrap gap-sm">
                        <div className="stack-xs">
                          <div className="row wrap items-center gap-xs">
                            <strong>{d.ride.passenger_name}</strong>
                            <Pill tone={draftStatusTone(d.status)}>
                              v{d.version} · {draftStatusLabel(d.status)}
                            </Pill>
                            <Pill tone="info">{rideActionLabel(d)}</Pill>
                            {d.locked_by && (
                              <Pill tone="warning" icon={<Lock size={12} />}>
                                {t('trafficHub.editingLock', { name: d.locked_by.name })}
                              </Pill>
                            )}
                          </div>
                          <div className="small muted">
                            {[
                              d.ride.event_title,
                              d.ride.area_name,
                              fmtDateTime(d.ride.pickup_at, ft),
                              d.submitted_by
                                ? t('trafficHub.submittedBy', { name: d.submitted_by.name })
                                : t('trafficHub.proposedBy', { name: d.created_by.name }),
                            ]
                              .filter(Boolean)
                              .join(' · ')}
                          </div>
                          {d.reason && <div className="small">{d.reason}</div>}
                          {d.status === 'returned' && d.review_notes && (
                            <div className="small text-danger">{d.review_notes}</div>
                          )}
                        </div>
                        <div className="btn-row">
                          <Button variant="secondary" size="sm" to={`/admin/rides/${d.ride_id}`}>
                            {t('trafficHub.openRideDetail')}
                          </Button>
                          {isLead && d.status === 'pending_approval' && (
                            <>
                              <Button
                                size="sm"
                                variant="danger"
                                icon={<RotateCcw size={14} />}
                                disabled={busy}
                                onClick={() => {
                                  setReturningRideDraft(d);
                                  setReturnNotes('');
                                }}
                              >
                                {t('trafficHub.returnDraftBtn')}
                              </Button>
                              <Button
                                size="sm"
                                variant="primary"
                                icon={<CheckCircle2 size={14} />}
                                disabled={busy}
                                loading={busy}
                                onClick={() => handleApproveRideDraft(d)}
                              >
                                {t('trafficHub.approveDraftBtn')}
                              </Button>
                            </>
                          )}
                        </div>
                      </div>
                    </Card>
                  ))
                )}
              </div>
            )}
          </div>
        )}

        {activeTab === 'live' && canLive && <AdminLiveContent showCountPill />}

        {activeTab === 'areas' && canAreas && (
          <div className="stack">
            {areaSubTabs.length > 1 && (
              <div className="chips" role="tablist" aria-label={t('trafficHub.tabAreas')}>
                {areaSubTabs.map((sub) => (
                  <button
                    key={sub.key}
                    type="button"
                    role="tab"
                    aria-selected={activeAreaSub === sub.key}
                    className={`chip ${activeAreaSub === sub.key ? 'on' : ''}`}
                    onClick={() => setAreaSub(sub.key)}
                  >
                    {sub.label}
                  </button>
                ))}
              </div>
            )}
            {activeAreaSub === 'photos' && can('admin_area_photos') && <AreaPhotos />}
            {activeAreaSub === 'fares' && can('admin_fares') && <AreaFares />}
            {activeAreaSub === 'areas' && can('admin_areas') && <Areas />}
          </div>
        )}
      </div>

      {returningRideDraft && (
        <Modal
          title={t('trafficHub.returnModalTitle')}
          description={`${returningRideDraft.ride.passenger_name} · v${returningRideDraft.version}`}
          onClose={() => {
            setReturningRideDraft(null);
            setReturnNotes('');
          }}
        >
          <form className="stack" onSubmit={handleReturnRideDraft}>
            <div className="chips quick-reason-chips">
              {quickChips.map((chip) => (
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
            <Field label={t('trafficHub.returnModalLabel')}>
              <textarea
                className="input"
                rows={3}
                value={returnNotes}
                placeholder={t('trafficHub.returnModalPlaceholder')}
                onChange={(e) => setReturnNotes(e.target.value)}
                required
              />
            </Field>
            <div className="btn-row end">
              <Button
                type="button"
                variant="ghost"
                onClick={() => {
                  setReturningRideDraft(null);
                  setReturnNotes('');
                }}
              >
                {t('cancel', { ns: 'common' })}
              </Button>
              <Button type="submit" variant="danger" disabled={busy || !returnNotes.trim()} loading={busy}>
                {t('trafficHub.returnConfirm')}
              </Button>
            </div>
          </form>
        </Modal>
      )}
    </Page>
  );
}
