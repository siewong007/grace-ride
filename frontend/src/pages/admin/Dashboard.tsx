import { ReactNode } from 'react';
import { AlertTriangle, CarFront, CheckCircle2, ClipboardCheck, HeartHandshake, KeyRound, LifeBuoy } from 'lucide-react';
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
  Page,
  PageHeader,
  StatusPill,
  TableColumn,
  UniversalTable,
} from '../../components/ui';
import { fmtAgo, fmtDay, fmtTime, RIDE_STATUS, todayStr, useFormatT } from '../../format';

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
  oldest_approval_at: string | null;
  next_no_driver_at: string | null;
  oldest_help_at: string | null;
  help_claimed_by_me?: number;
}

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

export default function Dashboard() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const { can } = useAuth();
  const isStaff = can('admin_dashboard');
  const rides = can('admin_rides');
  const nav = useNavigate();

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
      detail: string;
      created_at: string;
    }[];
  }>(auditPerm ? '/admin/audit?limit=5' : null, 20000);

  if (!isStaff) {
    return (
      <Page>
        <EmptyState
          title={t('dashboard.noPermission')}
        />
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
      key: 'recovery', count: can('admin_recovery') ? d.pending_recovery : 0, icon: <KeyRound size={20} />, tone: 'warn' as const,
      title: t('dashboard.q.recovery'),
      to: '/admin/members?tab=recovery', action: t('dashboard.q.open'),
    },
  ].filter((r) => r.count > 0);

  const columns: TableColumn<Ride>[] = [
    {
      key: 'pickupTime',
      header: t('dashboard.pickupTime'),
      render: (r) => (
        <>
          {fmtDay(r.pickup_at, ft).split(' ')[0]} {r.pickup_at.slice(11, 16)}
        </>
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
      render: (r) => r.driver?.name ?? <span className="muted">—</span>,
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
        actions={
          <Link className="btn" to="/admin/live">
            {t('dashboard.liveMap', { live: d.live_drivers, max: d.max_live_drivers })}
          </Link>
        }
      />

      {/* Work queue: what is waiting on staff, most urgent first; each row goes to the filtered list. */}
      <Card className="stack work-queue">
        <h2>{t('dashboard.q.title')}</h2>
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
                <Link className="btn sm" to={r.to} aria-label={`${r.action}: ${r.title} (${r.count})`}>
                  {r.action}
                </Link>
              </li>
            ))}
          </ul>
        )}
      </Card>

      {/* Today at a glance: numbers that are not actions, in one line. */}
      <p className="muted work-queue-today">
        <Link to={`/admin/rides?date=${d.today}`}>
          {t('dashboard.q.today', { count: d.today_total, confirmed: d.today_confirmed })}
        </Link>
        {' · '}
        <Link to="/admin/live">{t('dashboard.q.live', { count: d.live_drivers })}</Link>
      </p>

      {/* Two-column layout on desktop */}
      <div className="split">
        {/* Left column: Today's rides */}
        <Card className="stack">
          <div className="row between">
            <h3>{t('dashboard.todayRides')}</h3>
            <Link to="/admin/rides" className="small">
              {t('rides.all')}
            </Link>
          </div>
          {today.loading ? (
            <CardSkeleton count={3} />
          ) : today.error ? (
            <ErrorBox error={today.error} />
          ) : !today.data?.length ? (
            <EmptyState title={t('dashboard.noTodayRides')} />
          ) : (
            <UniversalTable
              columns={columns}
              items={today.data ?? []}
              onRowClick={(r) => nav(`/admin/rides/${r.id}`)}
              itemKey={(r) => r.id}
            />
          )}
        </Card>

        {/* Right column: Pending approvals and Follow-ups */}
        <div className="stack">
          <Card className="stack">
            <h3>
              {t('dashboard.pendingApproval2', {
                count: approvals.data?.length ?? 0,
              })}
            </h3>
            {approvals.loading ? (
              <CardSkeleton count={2} />
            ) : approvals.error ? (
              <ErrorBox error={approvals.error} />
            ) : !approvals.data?.length ? (
              <EmptyState title={t('dashboard.noPending')} />
            ) : (
              <div className="list">
                {approvals.data.slice(0, 6).map((r) => (
                  <Link key={r.id} to={`/admin/rides/${r.id}`} className="item">
                    <div className="grow">
                      <div className="bold">
                        {r.passenger.name} ← {r.driver?.name}
                      </div>
                      <div className="muted tiny">
                        {fmtDay(r.pickup_at, ft)} {fmtTime(r.pickup_at, ft)} ·{' '}
                        {r.area.name}
                      </div>
                    </div>
                    <StatusPill map={RIDE_STATUS} status={r.status} />
                  </Link>
                ))}
              </div>
            )}
          </Card>

          <Card className="stack">
            <h3>{t('dashboard.followUp')}</h3>
            {attention.loading ? (
              <CardSkeleton count={2} />
            ) : attention.error ? (
              <ErrorBox error={attention.error} />
            ) : !attention.data?.length ? (
              <EmptyState title={t('dashboard.noFollowUp')} />
            ) : (
              <div className="list">
                {attention.data.slice(0, 8).map((r) => (
                  <Link key={r.id} to={`/admin/rides/${r.id}`} className="item">
                    <div className="grow">
                      <div className="bold">
                        {r.passenger.name} · {fmtDay(r.pickup_at, ft)}{' '}
                        {fmtTime(r.pickup_at, ft)}
                      </div>
                      <div className="tiny warn">
                        {r.attention_reason || t('dashboard.noDriverWithin24h')}
                      </div>
                    </div>
                    <StatusPill map={RIDE_STATUS} status={r.status} />
                  </Link>
                ))}
              </div>
            )}
          </Card>

          {auditPerm && (
            <Card className="stack">
              <div className="row between">
                <h3>{t('dashboard.recentAudit')}</h3>
                <Link to="/admin/audit" className="small muted">
                  {t('dashboard.viewAllAudit')}
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
                  {recentAudit.data.items.slice(0, 5).map((a) => (
                    <div key={a.id} className="item">
                      <div className="grow">
                        <div className="bold small">
                          {a.actor ?? t('audit.system')} · {a.action}
                        </div>
                        <div className="muted tiny">
                          {a.entity}{a.entity_id != null && ` #${a.entity_id}`} {a.detail && `· ${a.detail}`}
                        </div>
                      </div>
                      <span className="tiny mono muted">
                        {fmtTime(a.created_at, ft)}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </Card>
          )}
        </div>
      </div>
    </Page>
  );
}
