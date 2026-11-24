import { useState } from 'react';
import { Link } from 'react-router-dom';
import { Download, FileText } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useApi } from '../../../api';
import {
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Field,
  Notice,
  Pill,
  Stat,
} from '../../../components/ui';
import { EVENT_KIND, fmtDateTime, useFormatT } from '../../../format';
import { downloadCsv, escapeCsv, exportReportPdf } from '../../../reportExport';
import type { EventsReportData } from '../../../types/domain';
import { buildPeriodQuery, PeriodControls, type PeriodFilterState } from './PeriodControls';

const KIND_OPTIONS = ['badminton', 'bowling', 'kayak', 'walk', 'other'] as const;

export default function EventsReportPanel({ period }: { period: PeriodFilterState }) {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const [kind, setKind] = useState('');
  const baseQuery = buildPeriodQuery(period);
  const query = kind ? `${baseQuery}&kind=${encodeURIComponent(kind)}` : baseQuery;
  const { data, loading, error } = useApi<EventsReportData>(`/admin/reports/events?${query}`);
  const ev = data && 'totals' in data ? data : null;

  const exportCsv = () => {
    if (!ev) return;
    const lines: string[] = [];
    lines.push('Grace Ride — Events Report Export');
    lines.push(`Scope,${ev.scope}`);
    lines.push(`Period,${escapeCsv(ev.period)}`);
    lines.push(`Window,${escapeCsv(`${ev.from} to ${ev.to}`)}`);
    lines.push('');
    lines.push('Summary,Value');
    lines.push(`Events,${ev.totals.events_count}`);
    lines.push(`Cancelled events,${ev.totals.cancelled_events}`);
    lines.push(`Total going,${ev.totals.total_going}`);
    lines.push(`Total waitlist,${ev.totals.total_waitlist}`);
    lines.push(`Total attended,${ev.totals.total_attended}`);
    lines.push(`Attendance rate (%),${ev.totals.attendance_rate_pct}%`);
    lines.push(`Need ride,${ev.totals.need_ride_count}`);
    lines.push(`Self transport,${ev.totals.self_transport_count}`);
    lines.push(`Offer drive,${ev.totals.offer_drive_count}`);
    lines.push(`Visitor passes,${ev.totals.visitor_passes_count}`);
    if (ev.totals.plan_visitors_count != null) {
      lines.push(`Plan visitors,${ev.totals.plan_visitors_count}`);
    }
    lines.push('');
    lines.push('Group,Going,Attended,Attendance Rate (%),Need Ride,Self Transport,Offer Drive');
    for (const g of ev.by_group) {
      lines.push(
        `${escapeCsv(g.group_name ?? t('reports.unassignedGroup'))},${g.going},${g.attended},${g.attendance_rate_pct}%,${g.need_ride},${g.self_transport},${g.offer_drive}`,
      );
    }
    lines.push('');
    const hasRideCols = ev.events.some((item) => item.rides_matched != null);
    lines.push(
      hasRideCols
        ? 'Event ID,Title,Kind,Starts At,Status,Organiser,Going,Attended,Attendance Rate (%),Rides Matched,Rides Unfilled'
        : 'Event ID,Title,Kind,Starts At,Status,Organiser,Going,Attended,Attendance Rate (%)',
    );
    for (const item of ev.events) {
      const base = `${item.id},${escapeCsv(item.title)},${escapeCsv(item.kind)},${escapeCsv(item.starts_at)},${escapeCsv(item.status)},${escapeCsv(item.creator_name)},${item.going},${item.attended},${item.attendance_rate_pct}%`;
      lines.push(
        hasRideCols
          ? `${base},${item.rides_matched ?? ''},${item.rides_unfilled ?? ''}`
          : base,
      );
    }
    downloadCsv(`grace-ride-events-report-${ev.period}`, lines);
  };

  const exportPdf = async () => {
    if (!ev) return;
    await exportReportPdf({
      fileBase: `grace-ride-events-report-${ev.period}`,
      title: t('reports.eventsPdfTitle'),
      meta: [
        `${t('reports.period')}: ${ev.period}`,
        `${t('reports.window')}: ${ev.from} – ${ev.to}`,
        `${t('reports.scopeLabel')}: ${ev.scope === 'global' ? t('reports.scopeGlobal') : t('reports.scopeOwnEvents')}`,
      ],
      summaryHeading: t('reports.summary'),
      summaryRows: [
        [t('reports.eventsCount'), String(ev.totals.events_count)],
        [t('reports.totalGoing'), String(ev.totals.total_going)],
        [t('reports.totalAttended'), String(ev.totals.total_attended)],
        [t('reports.attendanceRate'), `${ev.totals.attendance_rate_pct}%`],
        [t('reports.needRideCount'), String(ev.totals.need_ride_count)],
        [t('reports.selfTransportCount'), String(ev.totals.self_transport_count)],
        [t('reports.visitorPasses'), String(ev.totals.visitor_passes_count)],
      ],
      sections: [
        {
          heading: t('reports.byGroupHeading'),
          items: ev.by_group.map(
            (g) =>
              `${g.group_name ?? t('reports.unassignedGroup')}: ${g.attended}/${g.going} (${g.attendance_rate_pct}%)`,
          ),
        },
        {
          heading: t('reports.byKind'),
          items: ev.by_kind.map(
            (k) =>
              `${ft(EVENT_KIND[k.kind]?.label ?? k.kind)}: ${k.events_count} · ${k.attended}/${k.going} (${k.attendance_rate_pct}%)`,
          ),
        },
        {
          heading: t('reports.eventsListHeading'),
          items: ev.events.map(
            (item) =>
              `${item.title} (${fmtDateTime(item.starts_at, ft)}) — ${item.attended}/${item.going} (${item.attendance_rate_pct}%)`,
          ),
        },
      ],
    });
  };

  return (
    <div className="stack">
      <PeriodControls
        period={period}
        scope={ev?.scope}
        extraFilter={
          <Field label={t('reports.eventKind')}>
            <select
              className="input"
              value={kind}
              onChange={(e) => setKind(e.target.value)}
            >
              <option value="">{t('reports.allKinds')}</option>
              {KIND_OPTIONS.map((k) => (
                <option key={k} value={k}>
                  {ft(EVENT_KIND[k]?.label ?? k)}
                </option>
              ))}
            </select>
          </Field>
        }
        actions={
          <>
            <Button
              variant="secondary"
              size="sm"
              onClick={exportCsv}
              disabled={!ev}
              icon={<FileText size={16} />}
            >
              {t('reports.exportCsv')}
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={exportPdf}
              disabled={!ev}
              icon={<Download size={16} />}
            >
              {t('reports.exportPdf')}
            </Button>
          </>
        }
      />

      {ev?.scope === 'own_events' && (
        <Notice tone="info">{t('reports.ownEventsScopeNotice')}</Notice>
      )}

      <ErrorBox error={error} />

      {loading ? (
        <CardSkeleton count={3} />
      ) : !ev ? (
        <EmptyState title={t('reports.none')} />
      ) : (
        <>
          <div className="grid cols-4">
            <Stat label={t('reports.eventsCount')} value={ev.totals.events_count} />
            <Stat label={t('reports.totalGoing')} value={ev.totals.total_going} />
            <Stat label={t('reports.totalAttended')} value={ev.totals.total_attended} />
            <Stat label={t('reports.attendanceRate')} value={`${ev.totals.attendance_rate_pct}%`} />
            <Stat label={t('reports.needRideCount')} value={ev.totals.need_ride_count} />
            <Stat label={t('reports.selfTransportCount')} value={ev.totals.self_transport_count} />
            <Stat label={t('reports.offerDriveCount')} value={ev.totals.offer_drive_count} />
            <Stat label={t('reports.visitorPasses')} value={ev.totals.visitor_passes_count} />
            {ev.totals.plan_visitors_count != null && (
              <Stat label={t('reports.planVisitors')} value={ev.totals.plan_visitors_count} />
            )}
          </div>

          <div className="grid cols-2">
            <Card className="stack-sm">
              <h3>{t('reports.byGroupHeading')}</h3>
              {ev.by_group.length === 0 ? (
                <p className="muted small">{t('reports.none')}</p>
              ) : (
                <div className="stack-sm">
                  {ev.by_group.map((g, idx) => (
                    <div key={g.group_id ?? `none-${idx}`} className="report-bar-row">
                      <div className="row between nowrap gap-sm">
                        <span className="bold ellipsis">
                          {g.group_name ?? t('reports.unassignedGroup')}
                        </span>
                        <span className="small muted nowrap">
                          {g.attended}/{g.going} ({g.attendance_rate_pct}%)
                        </span>
                      </div>
                      <div className="bar">
                        <div style={{ width: `${Math.min(100, g.attendance_rate_pct)}%` }} />
                      </div>
                      <div className="row gap-xs flex-wrap muted tiny">
                        <span>{t('reports.needRideTag', { count: g.need_ride })}</span>
                        <span>·</span>
                        <span>{t('reports.selfTransportTag', { count: g.self_transport })}</span>
                        <span>·</span>
                        <span>{t('reports.offerDriveTag', { count: g.offer_drive })}</span>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </Card>

            <Card className="stack-sm">
              <h3>{t('reports.byKind')}</h3>
              {ev.by_kind.length === 0 ? (
                <p className="muted small">{t('reports.none')}</p>
              ) : (
                <div className="list">
                  {ev.by_kind.map((k) => (
                    <div key={k.kind} className="item row between nowrap gap-sm">
                      <span className="bold">{ft(EVENT_KIND[k.kind]?.label ?? k.kind)}</span>
                      <span className="small muted nowrap">
                        {k.events_count} · {k.attended}/{k.going} ({k.attendance_rate_pct}%)
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </Card>
          </div>

          <Card className="stack-sm">
            <h3>{t('reports.eventsListHeading')}</h3>
            {ev.events.length === 0 ? (
              <p className="muted small">{t('reports.none')}</p>
            ) : (
              <div className="list">
                {ev.events.map((item) => (
                  <Link key={item.id} to={`/events/${item.id}`} className="item">
                    <div className="grow min-w-0 stack-xs">
                      <div className="row gap-xs items-center flex-wrap">
                        <span className="bold ellipsis">{item.title}</span>
                        <Pill tone={item.status === 'cancelled' ? 'danger' : 'neutral'}>
                          {ft(EVENT_KIND[item.kind]?.label ?? item.kind)}
                        </Pill>
                      </div>
                      <div className="muted tiny">
                        {fmtDateTime(item.starts_at, ft)} · {item.creator_name}
                      </div>
                    </div>
                    <div className="stack-xs text-right no-wrap">
                      <span className="bold small">
                        {item.attended}/{item.going} ({item.attendance_rate_pct}%)
                      </span>
                      {item.rides_matched != null && (
                        <span className="muted tiny">
                          {t('reports.eventRidesLine', {
                            matched: item.rides_matched,
                            unfilled: item.rides_unfilled ?? 0,
                          })}
                        </span>
                      )}
                    </div>
                  </Link>
                ))}
              </div>
            )}
          </Card>
        </>
      )}
    </div>
  );
}
