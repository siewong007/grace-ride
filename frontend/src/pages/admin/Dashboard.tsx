import { Link, useNavigate } from 'react-router-dom';
import { useApi } from '../../api';
import { useAuth } from '../../auth';
import { Ride } from '../../components/RideCard';
import { Empty, ErrorBox, Spinner, StatusPill } from '../../components/ui';
import { fmtDay, fmtTime, RIDE_STATUS, todayStr, useFormatT } from '../../format';
import { useTranslation } from 'react-i18next';

interface Dash {
  today: string; today_total: number; today_confirmed: number; today_changes: number; pending_approval: number;
  no_driver: number; needs_attention: number; live_drivers: number; max_live_drivers: number; open_help: number;
  pending_reimbursements: number; members: number; pending_recovery: number;
}

export default function Dashboard() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const { isStaff } = useAuth();
  const { data: d, error } = useApi<Dash>(isStaff ? '/admin/dashboard' : null, 20000);
  const today = useApi<Ride[]>(isStaff ? `/admin/rides?date=${todayStr()}` : null, 20000);
  const attention = useApi<Ride[]>(isStaff ? '/admin/rides?attention=true' : null, 20000);
  const approvals = useApi<Ride[]>(isStaff ? '/admin/rides?status=pending_approval' : null, 20000);
  const nav = useNavigate();
  if (!isStaff) return <Empty>{t('dashboard.noPermission')}</Empty>;
  if (error) return <ErrorBox error={error} />;
  if (!d) return <Spinner />;

  const stat = (n: number, label: string, to: string, cls = '') => (
    <Link to={to} className={`card link stat ${cls}`}><span className="num">{n}</span><span className="label">{label}</span></Link>
  );

  return (
    <div className="stack">
      <div className="page-head">
        <div><h1>{ft(fmtDay(`${d.today}T00:00`))} {t('dashboard.title')} </h1><p>{t('dashboard.desc')}</p></div>
        <Link className="btn" to="/admin/live">{t('dashboard.liveMap', { live: d.live_drivers, max: d.max_live_drivers })}</Link>
      </div>
      <div className="grid cols-4">
        {stat(d.today_total, t('dashboard.todayArranged'), '/admin/rides')}
        {stat(d.today_confirmed, t('dashboard.todayConfirmed'), '/admin/rides')}
        {stat(d.pending_approval, t('dashboard.pendingApproval'), '/admin/rides?status=pending_approval', d.pending_approval ? 'warn' : '')}
        {stat(d.no_driver, t('dashboard.noDriver'), '/admin/rides?status=pending_driver', d.no_driver ? 'warn' : '')}
        {stat(d.needs_attention, t('dashboard.needsAttention'), '/admin/rides?attention=true', d.needs_attention ? 'bad' : '')}
        {stat(d.live_drivers, t('dashboard.liveDrivers'), '/admin/live')}
        {stat(d.open_help, t('dashboard.openHelp'), '/help', d.open_help ? 'warn' : '')}
        {stat(d.pending_reimbursements, t('dashboard.pendingReimb'), '/admin/reimbursements', d.pending_reimbursements ? 'warn' : '')}
      </div>

      <div className="split">
        <div className="card">
          <div className="card-title"><h3>{t('dashboard.todayRides')}</h3><Link to="/admin/rides" className="small">{t('all')} →</Link></div>
          {today.loading ? <Spinner /> : !today.data?.length ? <Empty>{t('dashboard.noTodayRides')}</Empty> : <RideTable rides={today.data} onOpen={(id) => nav(`/admin/rides/${id}`)} t={t} ft={ft} />}
        </div>
        <div className="stack">
          <div className="card">
            <h3 style={{ marginBottom: 8 }}>{t('dashboard.pendingApproval2', { count: approvals.data?.length ?? 0 })}</h3>
            {!approvals.data?.length ? <p className="muted">{t('dashboard.noPending')}</p> : (
              <div className="list">
                {approvals.data.slice(0, 6).map((r) => (
                  <Link key={r.id} to={`/admin/rides/${r.id}`} className="item">
                    <div className="grow"><div className="bold">{r.passenger.name} ← {r.driver?.name}</div><div className="muted tiny">{fmtDay(r.pickup_at, ft)} {fmtTime(r.pickup_at, ft)} · {r.area.name}</div></div>
                    <StatusPill map={RIDE_STATUS} status={r.status} />
                  </Link>
                ))}
              </div>
            )}
          </div>
          <div className="card">
            <h3 style={{ marginBottom: 8 }}>{t('dashboard.followUp')}</h3>
            {!attention.data?.length ? <p className="muted">{t('dashboard.noFollowUp')}</p> : (
              <div className="list">
                {attention.data.slice(0, 8).map((r) => (
                  <Link key={r.id} to={`/admin/rides/${r.id}`} className="item">
                    <div className="grow">
                      <div className="bold">{r.passenger.name} · {fmtDay(r.pickup_at, ft)} {fmtTime(r.pickup_at, ft)}</div>
                      <div className="tiny" style={{ color: 'var(--accent)' }}>{r.attention_reason || '24 小时内仍未有司机'}</div>
                    </div>
                    <StatusPill map={RIDE_STATUS} status={r.status} />
                  </Link>
                ))}
              </div>
            )}
          </div>
          {d.pending_recovery > 0 && <Link className="notice" to="/admin/members">{t('dashboard.recovery', { count: d.pending_recovery })} →</Link>}
        </div>
      </div>
    </div>
  );
}

export function RideTable({ rides, onOpen, t, ft }: { rides: Ride[]; onOpen: (id: number) => void; t: (k: string, o?: any) => string; ft: (k: string) => string }) {
  return (
    <div className="table-wrap">
      <table className="table responsive">
        <thead><tr><th>{t('dashboard.pickupTime')}</th><th>{t('dashboard.passenger')}</th><th>{t('dashboard.area')}</th><th>{t('dashboard.driver')}</th><th>{t('dashboard.status')}</th></tr></thead>
        <tbody>
          {rides.map((r) => (
            <tr key={r.id} className="clickable" onClick={() => onOpen(r.id)}>
              <td data-label={t('dashboard.pickupTime')} className="mono bold">{ft(fmtDay(r.pickup_at)).split(' ')[0]} {r.pickup_at.slice(11, 16)}</td>
              <td data-label={t('dashboard.passenger')}>{r.passenger.name}{r.seats > 1 && ` (${r.seats})`}</td>
              <td data-label={t('dashboard.area')}>{r.area.name}</td>
              <td data-label={t('dashboard.driver')}>{r.driver?.name ?? <span className="muted">—</span>}</td>
              <td data-label={t('dashboard.status')}><StatusPill map={RIDE_STATUS} status={r.status} />{r.needs_attention && ' ⚠'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
