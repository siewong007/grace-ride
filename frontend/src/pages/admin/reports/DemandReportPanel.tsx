import { Download, FileText } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useApi } from '../../../api';
import {
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Notice,
  Stat,
} from '../../../components/ui';
import { downloadCsv, escapeCsv, exportReportPdf } from '../../../reportExport';
import type { DemandReportData } from '../../../types/domain';
import { buildPeriodQuery, PeriodControls, type PeriodFilterState } from './PeriodControls';

const WEEKDAY_KEYS = ['sun', 'mon', 'tue', 'wed', 'thu', 'fri', 'sat'] as const;

export default function DemandReportPanel({ period }: { period: PeriodFilterState }) {
  const { t, i18n } = useTranslation('admin');
  const isZh = i18n.language.startsWith('zh');
  const query = buildPeriodQuery(period);
  const { data, loading, error } = useApi<DemandReportData>(`/admin/reports/demand?${query}`);
  const d = data && 'totals' in data ? data : null;

  const exportCsv = () => {
    if (!d) return;
    const lines: string[] = [];
    lines.push('Grace Ride — Demand & Shortage Report Export');
    lines.push(`Period,${escapeCsv(d.period)}`);
    if (d.scope) lines.push(`Scope,${d.scope}`);
    lines.push('');
    lines.push('Metric,Value');
    lines.push(`Total requests,${d.totals.total_requests}`);
    lines.push(`Fulfilled,${d.totals.fulfilled}`);
    lines.push(`Unfilled,${d.totals.unfilled}`);
    lines.push(`Fulfillment rate (%),${d.totals.fulfillment_rate_pct}%`);
    lines.push(`Completed,${d.totals.completed}`);
    lines.push(`Active drivers,${d.totals.active_drivers}`);
    lines.push(`Unique passengers,${d.totals.unique_passengers}`);
    lines.push(`Wheelchair requests,${d.totals.wheelchair_requests}`);
    lines.push(`Return legs,${d.totals.return_requests}`);
    lines.push(`Cancelled unassigned,${d.totals.cancelled_unassigned}`);
    if (d.totals.plan_visitors_count != null) {
      lines.push(`Plan visitors,${d.totals.plan_visitors_count}`);
    }
    if (d.totals.visitor_passes_count != null) {
      lines.push(`Visitor passes,${d.totals.visitor_passes_count}`);
    }
    lines.push('');
    lines.push('Area,Total,Fulfilled,Unfilled,Seats Needed,Seats Unfilled,Rate (%)');
    for (const a of d.by_area) {
      const name = (isZh ? a.area_name_zh || a.area_name_en : a.area_name_en || a.area_name_zh) || `#${a.area_id}`;
      lines.push(
        `${escapeCsv(name)},${a.total},${a.fulfilled},${a.unfilled},${a.seats_needed},${a.seats_unfilled},${a.fulfillment_rate_pct}%`,
      );
    }
    if (d.by_group && d.by_group.length > 0) {
      lines.push('');
      lines.push('Group,Total,Fulfilled,Unfilled,Seats Needed,Rate (%)');
      for (const g of d.by_group) {
        lines.push(
          `${escapeCsv(g.group_name ?? t('reports.unassignedGroup'))},${g.total},${g.fulfilled},${g.unfilled},${g.seats_needed},${g.fulfillment_rate_pct}%`,
        );
      }
    }
    lines.push('');
    lines.push('Weekday,Slot,Total,Fulfilled,Unfilled,Seats Needed');
    for (const s of d.by_slot) {
      const wd = t(`reports.weekdays.${WEEKDAY_KEYS[s.weekday] ?? 'sun'}`);
      lines.push(`${escapeCsv(wd)},${escapeCsv(s.slot)},${s.total},${s.fulfilled},${s.unfilled},${s.seats_needed}`);
    }
    downloadCsv(`grace-ride-demand-report-${d.period}`, lines);
  };

  const exportPdf = async () => {
    if (!d) return;
    await exportReportPdf({
      fileBase: `grace-ride-demand-report-${d.period}`,
      title: t('reports.demandPdfTitle'),
      meta: [
        `${t('reports.period')}: ${d.period}`,
        ...(d.from && d.to ? [`${t('reports.window')}: ${d.from} – ${d.to}`] : []),
      ],
      summaryHeading: t('reports.summary'),
      summaryRows: [
        [t('reports.demandTotal'), String(d.totals.total_requests)],
        [t('reports.demandFulfilled'), String(d.totals.fulfilled)],
        [t('reports.demandUnfilled'), String(d.totals.unfilled)],
        [t('reports.demandFulfillmentRate'), `${d.totals.fulfillment_rate_pct}%`],
        [t('reports.demandActiveDrivers'), String(d.totals.active_drivers)],
        [t('reports.demandWheelchair'), String(d.totals.wheelchair_requests)],
      ],
      sections: [
        {
          heading: t('reports.demandByArea'),
          items: d.by_area.map((a) => {
            const name =
              (isZh ? a.area_name_zh || a.area_name_en : a.area_name_en || a.area_name_zh) ||
              `#${a.area_id}`;
            return `${name} — ${t('reports.demandAreaLine', { total: a.total, unfilled: a.unfilled, rate: a.fulfillment_rate_pct })}`;
          }),
        },
        ...(d.by_group && d.by_group.length > 0
          ? [
              {
                heading: t('reports.demandByGroup'),
                items: d.by_group.map(
                  (g) =>
                    `${g.group_name ?? t('reports.unassignedGroup')} — ${t('reports.demandAreaLine', { total: g.total, unfilled: g.unfilled, rate: g.fulfillment_rate_pct })}`,
                ),
              },
            ]
          : []),
        {
          heading: t('reports.demandBySlot'),
          items: d.by_slot.map(
            (s) =>
              `${t(`reports.weekdays.${WEEKDAY_KEYS[s.weekday] ?? 'sun'}`)} · ${s.slot} — ${t('reports.demandSlotLine', { total: s.total, unfilled: s.unfilled })}`,
          ),
        },
      ],
    });
  };

  return (
    <div className="stack">
      <PeriodControls
        period={period}
        scope={d?.scope}
        actions={
          <>
            <Button
              variant="secondary"
              size="sm"
              onClick={exportCsv}
              disabled={!d}
              icon={<FileText size={16} />}
            >
              {t('reports.exportCsv')}
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={exportPdf}
              disabled={!d}
              icon={<Download size={16} />}
            >
              {t('reports.exportPdf')}
            </Button>
          </>
        }
      />

      {d?.scope === 'own_events' && (
        <Notice tone="info">{t('reports.ownEventsScopeNotice')}</Notice>
      )}

      <ErrorBox error={error} />

      {loading ? (
        <CardSkeleton count={3} />
      ) : !d ? (
        <EmptyState title={t('reports.none')} />
      ) : (
        <>
          <div className="grid cols-4">
            <Stat label={t('reports.demandTotal')} value={d.totals.total_requests} />
            <Stat label={t('reports.demandFulfilled')} value={d.totals.fulfilled} />
            <Stat label={t('reports.demandUnfilled')} value={d.totals.unfilled} />
            <Stat
              label={t('reports.demandFulfillmentRate')}
              value={`${d.totals.fulfillment_rate_pct}%`}
            />
            <Stat label={t('reports.demandActiveDrivers')} value={d.totals.active_drivers} />
            <Stat label={t('reports.demandWheelchair')} value={d.totals.wheelchair_requests} />
            <Stat label={t('reports.demandReturnLegs')} value={d.totals.return_requests} />
            <Stat
              label={t('reports.demandCancelledUnassigned')}
              value={d.totals.cancelled_unassigned}
            />
            {d.totals.proxy_rides != null && (
              <Stat label={t('reports.proxyRides')} value={d.totals.proxy_rides} />
            )}
            {d.totals.plan_visitors_count != null && (
              <Stat label={t('reports.planVisitors')} value={d.totals.plan_visitors_count} />
            )}
            {d.totals.visitor_passes_count != null && (
              <Stat label={t('reports.visitorPasses')} value={d.totals.visitor_passes_count} />
            )}
          </div>

          <div className="grid cols-2">
            <Card className="stack-sm">
              <h3>{t('reports.demandByArea')}</h3>
              {d.by_area.length === 0 ? (
                <p className="muted small">{t('reports.none')}</p>
              ) : (
                <div className="stack-sm">
                  {d.by_area.map((a) => (
                    <div key={a.area_id} className="report-bar-row">
                      <div className="row between nowrap gap-sm">
                        <span className="bold ellipsis">
                          {(isZh ? a.area_name_zh || a.area_name_en : a.area_name_en || a.area_name_zh) ||
                            `#${a.area_id}`}
                        </span>
                        <span className="small muted nowrap">
                          {t('reports.demandAreaLine', {
                            total: a.total,
                            unfilled: a.unfilled,
                            rate: a.fulfillment_rate_pct,
                          })}
                        </span>
                      </div>
                      <div className="bar">
                        <div style={{ width: `${Math.min(100, a.fulfillment_rate_pct)}%` }} />
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </Card>

            <Card className="stack-sm">
              <h3>{t('reports.demandBySlot')}</h3>
              {d.by_slot.length === 0 ? (
                <p className="muted small">{t('reports.none')}</p>
              ) : (
                <div className="list">
                  {d.by_slot.map((s) => (
                    <div key={`${s.weekday}-${s.slot}`} className="item row between nowrap gap-sm">
                      <span className="bold">
                        {t(`reports.weekdays.${WEEKDAY_KEYS[s.weekday] ?? 'sun'}`)} · {s.slot}
                      </span>
                      <span className="small muted">
                        {t('reports.demandSlotLine', { total: s.total, unfilled: s.unfilled })}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </Card>
          </div>

          {d.by_group && d.by_group.length > 0 && (
            <Card className="stack-sm">
              <h3>{t('reports.demandByGroup')}</h3>
              <div className="grid cols-2">
                {d.by_group.map((g, idx) => (
                  <div key={g.group_id ?? `none-${idx}`} className="report-bar-row">
                    <div className="row between nowrap gap-sm">
                      <span className="bold ellipsis">
                        {g.group_name ?? t('reports.unassignedGroup')}
                      </span>
                      <span className="small muted nowrap">
                        {t('reports.demandAreaLine', {
                          total: g.total,
                          unfilled: g.unfilled,
                          rate: g.fulfillment_rate_pct,
                        })}
                      </span>
                    </div>
                    <div className="bar">
                      <div style={{ width: `${Math.min(100, g.fulfillment_rate_pct)}%` }} />
                    </div>
                  </div>
                ))}
              </div>
            </Card>
          )}
        </>
      )}
    </div>
  );
}
