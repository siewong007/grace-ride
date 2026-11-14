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
  Spinner,
  Stat,
  StatusPill,
} from '../../components/ui';
import { fmtDay, fmtTime, RIDE_STATUS, todayStr, useFormatT } from '../../format';
import { RideTable } from './RideTable';

export { RideTable };

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
  if (!d) return <Page><Spinner /></Page>;

  return (
    <Page>
      <PageHeader
        title={`${fmtDay(`${d.today}T00:00`, ft)} ${t('dashboard.title')}`}
        subtitle={t('dashboard.desc')}
        actions={
          <Link className="btn" to="/admin/live">
            {t('dashboard.liveMap', { live: d.live_drivers, max: d.max_live_drivers })}
          </Link>
        }
      />

      {/* Key numbers: one block split by hairlines; each links to its filtered list */}
      <div className="stat-strip">
        <Stat
          label={t('dashboard.todayArranged')}
          value={d.today_total}
          to={`/admin/rides?date=${d.today}`}
        />
        <Stat
          label={t('dashboard.todayConfirmed')}
          value={d.today_confirmed}
          to={`/admin/rides?date=${d.today}&status=confirmed`}
        />
        <Stat
          label={t('dashboard.pendingApproval')}
          value={d.pending_approval}
          tone={d.pending_approval > 0 ? 'warn' : 'normal'}
          to="/admin/rides?status=pending_approval"
        />
        <Stat
          label={t('dashboard.noDriver')}
          value={d.no_driver}
          tone={d.no_driver > 0 ? 'warn' : 'normal'}
          to="/admin/rides?status=pending_driver"
        />
        <Stat
          label={t('dashboard.needsAttention')}
          value={d.needs_attention}
          tone={d.needs_attention > 0 ? 'bad' : 'normal'}
          to="/admin/rides?attention=true"
        />
        <Stat
          label={t('dashboard.liveDrivers')}
          value={d.live_drivers}
          to="/admin/live"
        />
        <Stat
          label={t('dashboard.openHelp')}
          value={d.open_help}
          tone={d.open_help > 0 ? 'warn' : 'normal'}
          to="/help"
        />
        <Stat
          label={t('dashboard.pendingReimb')}
          value={d.pending_reimbursements}
          tone={d.pending_reimbursements > 0 ? 'warn' : 'normal'}
          to="/admin/reimbursements"
        />
      </div>

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
          ) : !today.data?.length ? (
            <EmptyState title={t('dashboard.noTodayRides')} />
          ) : (
            <RideTable
              rides={today.data}
              onOpen={(id) => nav(`/admin/rides/${id}`)}
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

          {d.pending_recovery > 0 && (
            <Link className="notice" to="/admin/members">
              {t('dashboard.recovery', { count: d.pending_recovery })} →
            </Link>
          )}

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
