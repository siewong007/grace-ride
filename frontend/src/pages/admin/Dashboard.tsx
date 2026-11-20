import { ReactNode, useState } from 'react';
import {
  AlertTriangle,
  ArrowRight,
  CarFront,
  CheckCircle2,
  ChevronRight,
  ClipboardCheck,
  ClipboardList,
  HeartHandshake,
  KeyRound,
  LifeBuoy,
  MapPinned,
  UserPlus,
  Wallet,
} from 'lucide-react';
import { Link, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useApi } from '../../api';
import { useAuth } from '../../auth';
import { Ride } from '../../components/RideCard';
import {
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  NoAccess,
  Page,
  PageHeader,
  Pill,
  StatusPill,
  TableColumn,
  UniversalTable,
} from '../../components/ui';
import { auditActionLabel, auditItemLine } from '../../auditLabel';
import { fmtAgo, fmtDay, fmtTime, RIDE_STATUS, todayStr, useFormatT } from '../../format';
import { Can } from '../../perms';
import { ADMIN_HOME } from '../../staffNav';
import { getObjectLink } from './Audit';
import { rideCompactRow } from './rideRows';

interface Dash {
  today: string;
  today_total: number;
  today_confirmed: number;
  today_changes: number;
  pending_approval: number;
  no_driver: number;
  needs_attention: number;
  live_drivers: number;
  max_live_drivers: number;
  open_help: number;
  pending_reimbursements: number;
  members: number;
  pending_recovery: number;
  pending_members?: number;
  open_invites?: number;
  awaiting_payment?: number;
  oldest_approval_at: string | null;
  next_no_driver_at: string | null;
  oldest_help_at: string | null;
  help_claimed_by_me?: number;
}

/** How many rides each preview card lists; the queue above is the place to work through the rest. */
const PREVIEW_ROWS = 3;

type TodayFilter = 'all' | 'unassigned' | 'confirmed' | 'in_progress';

/** One line of the work queue: how many things wait, where, and the one action that clears them. */
interface QueueRow {
  key: string;
  count: number;
  icon: ReactNode;
  tone: 'warn' | 'bad';
  title: string;
  meta?: string;
  to: string;
  action: string;
}

function canOpenAuditEntity(entity: string, can: Can): boolean {
  switch (entity) {
    case 'ride':
      return can('admin_rides');
    case 'event':
      return can('events') || can('admin_events');
    case 'help_ticket':
      return can('admin_help');
    case 'settlement':
      return can('settlements') || can('admin_bills');
    case 'user':
      return can('admin_members') || can('admin_invites') || can('admin_recovery');
    case 'reimbursement':
      return can('admin_reimb_review') || can('admin_reimb_pay');
    case 'group':
    case 'area':
      return can('admin_groups') || can('admin_areas') || can('admin_settings') || can('admin_fares') || can('admin_area_photos');
    case 'role':
      return can('admin_roles');
    case 'announcement':
      return can('admin_announcements');
    default:
      return false;
  }
}

interface DashKpiTileProps {
  label: string;
  value: number;
  sub: string;
  icon: ReactNode;
  tone?: 'normal' | 'warn' | 'bad' | 'success';
  progress?: number;
  to?: string;
}

function DashKpiTile({
  label,
  value,
  sub,
  icon,
  tone = 'normal',
  progress,
  to,
}: DashKpiTileProps) {
  const cls = ['card', to ? 'link' : '', 'stat', 'dash-kpi', tone !== 'normal' ? tone : '']
    .filter(Boolean)
    .join(' ');
  const body = (
    <>
      <span className="dash-kpi-top">
        <span className="dash-kpi-icon" aria-hidden="true">{icon}</span>
        <span className="label grow">{label}</span>
        {to && <ArrowRight size={14} className="dash-kpi-arrow" aria-hidden="true" />}
      </span>
      <span className="num">{value}</span>
      {progress !== undefined && (
        <span className="dash-kpi-bar" aria-hidden="true">
          <span className="dash-kpi-bar-fill" style={{ width: `${progress}%` }} />
        </span>
      )}
      <span className="muted tiny dash-kpi-sub">{sub}</span>
    </>
  );

  return to ? <Link to={to} className={cls}>{body}</Link> : <div className={cls}>{body}</div>;
}

export default function Dashboard() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const { can } = useAuth();
  const isStaff = can('admin_dashboard');
  const rides = can('admin_rides');
  const live = can('admin_live');
  const nav = useNavigate();
  const [todayFilter, setTodayFilter] = useState<TodayFilter>('all');

  const { data: d, error } = useApi<Dash>(
    isStaff ? '/admin/dashboard' : null,
    20000,
  );
  const today = useApi<Ride[]>(
    rides ? `/admin/rides?date=${todayStr()}` : null,
    20000,
  );
  const attention = useApi<Ride[]>(
    rides ? '/admin/rides?attention=true' : null,
    20000,
  );
  const approvals = useApi<Ride[]>(
    rides ? '/admin/rides?status=pending_approval' : null,
    20000,
  );
  const auditPerm = can('admin_audit');
  const recentAudit = useApi<{
    items: {
      id: number;
      actor: string | null;
      action: string;
      entity: string;
      entity_id: number | null;
      entity_name?: string | null;
      detail: string;
      created_at: string;
    }[];
  }>(auditPerm ? '/admin/audit?limit=5' : null, 20000);

  if (!isStaff) {
    return (
      <Page>
        <NoAccess />
      </Page>
    );
  }

  if (error) return <Page><ErrorBox error={error} /></Page>;
  if (!d) return <Page><CardSkeleton count={3} /></Page>;

  // Ordered by urgency: a pickup with no driver beats everything else.
  const queue: QueueRow[] = [
    {
      key: 'noDriver', count: rides ? d.no_driver : 0, icon: <CarFront size={20} />, tone: 'bad' as const,
      title: t('dashboard.q.noDriver'),
      meta: d.next_no_driver_at
        ? t('dashboard.q.nextPickup', { when: `${fmtDay(d.next_no_driver_at, ft)} ${fmtTime(d.next_no_driver_at, ft)}` })
        : undefined,
      to: '/admin/rides?status=pending_driver', action: t('dashboard.q.assign'),
    },
    {
      key: 'approve', count: rides ? d.pending_approval : 0, icon: <ClipboardCheck size={20} />, tone: 'warn' as const,
      title: t('dashboard.q.approve'),
      meta: d.oldest_approval_at ? t('dashboard.q.oldest', { ago: fmtAgo(d.oldest_approval_at, ft) }) : undefined,
      to: '/admin/rides?status=pending_approval', action: t('dashboard.q.review'),
    },
    {
      key: 'attention', count: rides ? d.needs_attention : 0, icon: <AlertTriangle size={20} />, tone: 'bad' as const,
      title: t('dashboard.q.attention'),
      to: '/admin/rides?attention=true', action: t('dashboard.q.open'),
    },
    {
      key: 'help', count: can('admin_help') ? d.open_help : 0, icon: <LifeBuoy size={20} />, tone: 'warn' as const,
      title: t('dashboard.q.help'),
      meta: [
        d.oldest_help_at ? t('dashboard.q.oldest', { ago: fmtAgo(d.oldest_help_at, ft) }) : '',
        d.help_claimed_by_me ? t('dashboard.q.claimedByYou', { count: d.help_claimed_by_me }) : '',
      ].filter(Boolean).join(' · ') || undefined,
      to: '/help', action: t('dashboard.q.answer'),
    },
    {
      key: 'reimb', count: can('admin_reimb_review') ? d.pending_reimbursements : 0, icon: <HeartHandshake size={20} />, tone: 'warn' as const,
      title: t('dashboard.q.reimb'),
      to: '/admin/reimbursements', action: t('dashboard.q.review'),
    },
    {
      key: 'payout', count: can('admin_reimb_pay') ? (d.awaiting_payment ?? 0) : 0, icon: <Wallet size={20} />, tone: 'warn' as const,
      title: t('dashboard.q.payout'),
      to: '/admin/reimbursements?tab=payout', action: t('dashboard.q.pay'),
    },
    {
      key: 'newMembers', count: can('admin_members') ? (d.pending_members ?? 0) : 0, icon: <UserPlus size={20} />, tone: 'warn' as const,
      title: t('dashboard.q.newMembers'),
      to: '/admin/members?status=pending', action: t('dashboard.q.review'),
    },
    {
      key: 'recovery', count: can('admin_recovery') ? d.pending_recovery : 0, icon: <KeyRound size={20} />, tone: 'warn' as const,
      title: t('dashboard.q.recovery'),
      to: '/admin/members?tab=recovery', action: t('dashboard.q.open'),
    },
  ].filter((r) => r.count > 0);

  const totalWaiting = queue.reduce((sum, r) => sum + r.count, 0);
  const confirmedPct = d.today_total > 0
    ? Math.min(100, Math.round((d.today_confirmed / d.today_total) * 100))
    : undefined;
  const allConfirmed = d.today_total > 0 && d.today_confirmed >= d.today_total;
  const remainingToday = Math.max(0, d.today_total - d.today_confirmed);

  const todayList = today.data ?? [];
  const todayCounts = {
    all: todayList.length,
    unassigned: todayList.filter((r) => !r.driver || r.status === 'pending_driver').length,
    confirmed: todayList.filter((r) => r.status === 'confirmed').length,
    in_progress: todayList.filter((r) => r.status === 'in_progress').length,
  };
  const activeTodayFilter: TodayFilter =
    todayFilter !== 'all' && todayCounts[todayFilter] === 0 ? 'all' : todayFilter;
  const filteredToday =
    activeTodayFilter === 'all'
      ? todayList
      : activeTodayFilter === 'unassigned'
        ? todayList.filter((r) => !r.driver || r.status === 'pending_driver')
        : todayList.filter((r) => r.status === activeTodayFilter);
  const todayFilterOptions: { key: TodayFilter; label: string; count: number }[] = [
    { key: 'all' as const, label: t('dashboard.filterAll', { count: todayCounts.all }), count: todayCounts.all },
    { key: 'unassigned' as const, label: t('dashboard.filterUnassigned', { count: todayCounts.unassigned }), count: todayCounts.unassigned },
    { key: 'confirmed' as const, label: t('dashboard.filterConfirmed', { count: todayCounts.confirmed }), count: todayCounts.confirmed },
    { key: 'in_progress' as const, label: t('dashboard.filterInProgress', { count: todayCounts.in_progress }), count: todayCounts.in_progress },
  ].filter((o) => o.key === 'all' || o.count > 0);

  const columns: TableColumn<Ride>[] = [
    {
      key: 'pickupTime',
      header: t('dashboard.pickupTime'),
      render: (r) => (
        <span className="bold mono">
          {fmtDay(r.pickup_at, ft).split(' ')[0]} {r.pickup_at.slice(11, 16)}
        </span>
      ),
    },
    {
      key: 'passenger',
      header: t('dashboard.passenger'),
      render: (r) => `${r.passenger.name}${r.seats > 1 ? ` (${r.seats})` : ''}`,
    },
    {
      key: 'area',
      header: t('dashboard.area'),
      render: (r) => r.area.name,
    },
    {
      key: 'driver',
      header: t('dashboard.driver'),
      render: (r) => r.driver?.name ?? <span className="muted">{t('dashboard.unassigned')}</span>,
    },
    {
      key: 'status',
      header: t('dashboard.status'),
      render: (r) => (
        <>
          <StatusPill map={RIDE_STATUS} status={r.status} />
          {r.needs_attention && (
            <AlertTriangle size={14} aria-hidden="true" className="pill-icon warn" />
          )}
        </>
      ),
    },
  ];

  return (
    <Page>
      <PageHeader
        title={t('dashboard.title')}
        subtitle={`${fmtDay(`${d.today}T00:00`, ft)} · ${t('dashboard.desc')}`}
        back={ADMIN_HOME}
        actions={
          rides || live ? (
            <>
              {rides && (
                <Link className="btn sm" to="/admin/rides">
                  <ClipboardList size={16} aria-hidden="true" />
                  <span>{t('rides.title')}</span>
                </Link>
              )}
              {live && (
                <Link className="btn sm" to="/admin/live">
                  <span
                    className={`dash-live-dot ${d.live_drivers > 0 ? 'on' : ''}`.trim()}
                    aria-hidden="true"
                  />
                  <MapPinned size={16} aria-hidden="true" />
                  <span>{t('dashboard.liveMap', { live: d.live_drivers, max: d.max_live_drivers })}</span>
                </Link>
              )}
            </>
          ) : undefined
        }
      />

      {/* Today at a glance: structured interactive KPI tiles instead of inline prose links. */}
      <section aria-label={t('dashboard.kpiLabel')} className="stat-strip dash-kpi-strip">
        <DashKpiTile
          label={t('dashboard.todayArranged')}
          value={d.today_total}
          tone={allConfirmed ? 'success' : 'normal'}
          icon={<CarFront size={16} />}
          progress={confirmedPct}
          sub={
            d.today_total > 0
              ? t('dashboard.kpiConfirmedSub', { confirmed: d.today_confirmed, total: d.today_total })
              : t('dashboard.noTodayRides')
          }
          to={rides ? `/admin/rides?date=${d.today}` : undefined}
        />
        <DashKpiTile
          label={t('dashboard.todayConfirmed')}
          value={d.today_confirmed}
          tone={allConfirmed ? 'success' : 'normal'}
          icon={<CheckCircle2 size={16} />}
          sub={
            d.today_total === 0
              ? t('dashboard.kpiNoScheduleSub')
              : allConfirmed
                ? t('dashboard.kpiAllReady')
                : t('dashboard.kpiRemainingSub', { remaining: remainingToday })
          }
          to={rides ? `/admin/rides?date=${d.today}&status=confirmed` : undefined}
        />
        <DashKpiTile
          label={t('dashboard.liveDrivers')}
          value={d.live_drivers}
          tone={d.live_drivers > 0 ? 'success' : 'normal'}
          icon={<MapPinned size={16} />}
          sub={
            d.live_drivers > 0
              ? t('dashboard.q.live', { count: d.live_drivers })
              : t('dashboard.kpiLiveCapSub', { max: d.max_live_drivers })
          }
          to={live ? '/admin/live' : undefined}
        />
        <DashKpiTile
          label={t('dashboard.kpiChanges')}
          value={d.today_changes}
          tone={d.today_changes > 0 ? 'warn' : 'normal'}
          icon={<AlertTriangle size={16} />}
          sub={d.today_changes > 0 ? t('dashboard.kpiChangesSub') : t('dashboard.noFollowUp')}
          to={rides ? '/admin/rides?attention=true' : undefined}
        />
      </section>

      {/* Work queue: what is waiting on staff, most urgent first; each row goes to the filtered list. */}
      <Card className="stack work-queue">
        <div className="row between dash-card-head">
          <h2>{t('dashboard.q.title')}</h2>
          <Pill tone={queue.length === 0 ? 'success' : queue.some((r) => r.tone === 'bad') ? 'danger' : 'warning'}>
            {queue.length === 0
              ? t('dashboard.q.clearBadge')
              : t('dashboard.q.waitingTotal', { count: totalWaiting })}
          </Pill>
        </div>
        {queue.length === 0 ? (
          <div className="row nowrap work-queue-clear">
            <CheckCircle2 size={20} aria-hidden="true" />
            <span>{t('dashboard.q.allClear')}</span>
          </div>
        ) : (
          <ul className="list work-queue-list">
            {queue.map((r) => (
              <li key={r.key} className="item work-queue-row">
                <span className={`work-queue-icon ${r.tone}`} aria-hidden="true">{r.icon}</span>
                <span className="work-queue-count">{r.count}</span>
                <div className="grow min-w-0">
                  <div className="bold">{r.title}</div>
                  {r.meta && <div className="muted small">{r.meta}</div>}
                </div>
                <Link
                  className={`btn sm ${r.tone === 'bad' ? 'primary' : ''}`.trim()}
                  to={r.to}
                  aria-label={`${r.action}: ${r.title} (${r.count})`}
                >
                  <span>{r.action}</span>
                  <ArrowRight size={15} aria-hidden="true" />
                </Link>
              </li>
            ))}
          </ul>
        )}
      </Card>

      {/* Two-column layout on desktop */}
      <div className="split">
        {/* Left column: Today's rides */}
        <Card className="stack">
          <div className="row between dash-card-head">
            <div className="row gap-sm">
              <h3>{t('dashboard.todayRides')}</h3>
              {today.data && <Pill>{todayList.length}</Pill>}
            </div>
            {rides && (
              <Link to={`/admin/rides?date=${d.today}`} className="btn ghost sm dash-card-action">
                <span>{t('dashboard.viewAll')}</span>
                <ArrowRight size={15} aria-hidden="true" />
              </Link>
            )}
          </div>
          {todayList.length > 1 && todayFilterOptions.length > 1 && (
            <div className="chips dash-filter-chips">
              {todayFilterOptions.map((o) => (
                <button
                  key={o.key}
                  type="button"
                  className={`chip ${activeTodayFilter === o.key ? 'on' : ''}`.trim()}
                  aria-pressed={activeTodayFilter === o.key}
                  onClick={() => setTodayFilter(o.key)}
                >
                  {o.label}
                </button>
              ))}
            </div>
          )}
          {today.loading ? (
            <CardSkeleton count={3} />
          ) : today.error ? (
            <ErrorBox error={today.error} />
          ) : !todayList.length ? (
            <EmptyState title={t('dashboard.noTodayRides')} />
          ) : (
            <UniversalTable
              columns={columns}
              items={filteredToday}
              onRowClick={(r) => nav(`/admin/rides/${r.id}`)}
              itemKey={(r) => r.id}
              compactRow={(r) => rideCompactRow(r, ft)}
            />
          )}
        </Card>

        {/* Right column: Pending approvals and Follow-ups */}
        <div className="stack">
          <Card className="stack">
            <div className="row between dash-card-head">
              <h3>
                {t('dashboard.pendingApproval2', {
                  count: approvals.data?.length ?? 0,
                })}
              </h3>
              {rides && (approvals.data?.length ?? 0) > 0 && (
                <Link to="/admin/rides?status=pending_approval" className="btn ghost sm dash-card-action">
                  <span>{t('dashboard.viewAll')}</span>
                  <ArrowRight size={15} aria-hidden="true" />
                </Link>
              )}
            </div>
            {approvals.loading ? (
              <CardSkeleton count={2} />
            ) : approvals.error ? (
              <ErrorBox error={approvals.error} />
            ) : !approvals.data?.length ? (
              <EmptyState title={t('dashboard.noPending')} />
            ) : (
              <div className="list">
                {approvals.data.slice(0, PREVIEW_ROWS).map((r) => (
                  <Link key={r.id} to={`/admin/rides/${r.id}`} className="item dash-item-link">
                    <div className="grow min-w-0">
                      <div className="bold ellipsis">
                        {r.driver ? `${r.passenger.name} ← ${r.driver.name}` : r.passenger.name}
                      </div>
                      <div className="muted tiny">
                        {fmtDay(r.pickup_at, ft)} {fmtTime(r.pickup_at, ft)} ·{' '}
                        {r.area.name}
                      </div>
                    </div>
                    <div className="item-status">
                      <StatusPill map={RIDE_STATUS} status={r.status} />
                      <ChevronRight size={16} aria-hidden="true" className="dash-row-chevron" />
                    </div>
                  </Link>
                ))}
              </div>
            )}
          </Card>

          <Card className="stack">
            <div className="row between dash-card-head">
              <h3>{t('dashboard.followUp')}</h3>
              {rides && (attention.data?.length ?? 0) > 0 && (
                <Link to="/admin/rides?attention=true" className="btn ghost sm dash-card-action">
                  <span>{t('dashboard.viewAll')}</span>
                  <ArrowRight size={15} aria-hidden="true" />
                </Link>
              )}
            </div>
            {attention.loading ? (
              <CardSkeleton count={2} />
            ) : attention.error ? (
              <ErrorBox error={attention.error} />
            ) : !attention.data?.length ? (
              <EmptyState title={t('dashboard.noFollowUp')} />
            ) : (
              <div className="list">
                {attention.data.slice(0, PREVIEW_ROWS).map((r) => (
                  <Link key={r.id} to={`/admin/rides/${r.id}`} className="item dash-item-link">
                    <div className="grow min-w-0">
                      <div className="bold ellipsis">
                        {r.passenger.name} · {fmtDay(r.pickup_at, ft)}{' '}
                        {fmtTime(r.pickup_at, ft)}
                      </div>
                      <div className="tiny warn">
                        {r.attention_reason || t('dashboard.noDriverWithin24h')}
                      </div>
                    </div>
                    <div className="item-status">
                      <StatusPill map={RIDE_STATUS} status={r.status} />
                      <ChevronRight size={16} aria-hidden="true" className="dash-row-chevron" />
                    </div>
                  </Link>
                ))}
              </div>
            )}
          </Card>

          {auditPerm && (
            <Card className="stack">
              <div className="row between dash-card-head">
                <h3>{t('dashboard.recentAudit')}</h3>
                <Link to="/admin/audit" className="btn ghost sm dash-card-action">
                  <span>{t('dashboard.viewAll')}</span>
                  <ArrowRight size={15} aria-hidden="true" />
                </Link>
              </div>
              {recentAudit.loading ? (
                <CardSkeleton count={2} />
              ) : recentAudit.error ? (
                <ErrorBox error={recentAudit.error} />
              ) : !recentAudit.data?.items?.length ? (
                <EmptyState title={t('dashboard.noAudit')} />
              ) : (
                <div className="list">
                  {recentAudit.data.items.slice(0, 5).map((a) => {
                    const target = canOpenAuditEntity(a.entity, can)
                      ? getObjectLink(a.entity, a.entity_id)
                      : null;
                    const rowBody = (
                      <>
                        <div className="grow min-w-0">
                          <div className="bold small ellipsis">
                            {a.actor ?? t('audit.system')} · {auditActionLabel(a.action, t)}
                          </div>
                          <div className="muted tiny">
                            {auditItemLine(a, t)} {a.detail && a.detail !== a.entity_name && `· ${a.detail}`}
                          </div>
                        </div>
                        <span className="tiny mono muted no-wrap">
                          {fmtTime(a.created_at, ft)}
                        </span>
                        {target && <ChevronRight size={15} aria-hidden="true" className="dash-row-chevron" />}
                      </>
                    );
                    return target ? (
                      <Link key={a.id} to={target.url} className="item dash-item-link">
                        {rowBody}
                      </Link>
                    ) : (
                      <div key={a.id} className="item">
                        {rowBody}
                      </div>
                    );
                  })}
                </div>
              )}
            </Card>
          )}
        </div>
      </div>
    </Page>
  );
}
