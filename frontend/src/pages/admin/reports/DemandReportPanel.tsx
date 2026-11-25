import { useState } from 'react';
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
import { downloadCsv, escapeCsv, exportReportPdf } from '../../../reportExport';
import type { DemandReportData } from '../../../types/domain';
import {
  buildPeriodQuery,
  DeltaPill,
  type GroupFilterOption,
  PeriodControls,
  type PeriodFilterState,
} from './PeriodControls';

const WEEKDAY_KEYS = ['sun', 'mon', 'tue', 'wed', 'thu', 'fri', 'sat'] as const;

export default function DemandReportPanel({
  period,
  groupId: controlledGroupId,
  onGroupChange,
}: {
  period: PeriodFilterState;
  groupId?: string;
  onGroupChange?: (next: string) => void;
}) {
  const { t, i18n } = useTranslation('admin');
  const isZh = i18n.language.startsWith('zh');
  const [localGroupId, setLocalGroupId] = useState('');
  const [exportingPdf, setExportingPdf] = useState(false);
  const [exportErr, setExportErr] = useState<string | null>(null);
  const groupId = controlledGroupId !== undefined ? controlledGroupId : localGroupId;
  const setGroupId = onGroupChange ?? setLocalGroupId;

  const { data: groupsData } = useApi<GroupFilterOption[]>('/groups');
  const groups = Array.isArray(groupsData) ? groupsData.filter((g) => !g.is_visitor) : [];

  const baseQuery = buildPeriodQuery(period);
  const query = (() => {
    if (!baseQuery) return null;
    const sp = new URLSearchParams(baseQuery);
    if (groupId) sp.set('group_id', groupId);
    return sp.toString();
  })();

  const { data, loading, error } = useApi<DemandReportData>(
    query ? `/admin/reports/demand?${query}` : null,
  );
  const d = data && 'totals' in data ? data : null;
  const cmp = d?.comparison;

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
    if (d.totals.pending_confirmation != null) {
      lines.push(`Pending confirmation,${d.totals.pending_confirmation}`);
    }
    lines.push(`Unfilled,${d.totals.unfilled}`);
    lines.push(`Fulfillment rate (%),${d.totals.fulfillment_rate_pct}%`);
    lines.push(`Completed,${d.totals.completed}`);
    if (d.totals.no_show != null) lines.push(`No-show,${d.totals.no_show}`);
    if (d.totals.alt_transport != null) lines.push(`Alt transport,${d.totals.alt_transport}`);
    lines.push(`Active drivers,${d.totals.active_drivers}`);
    lines.push(`Unique passengers,${d.totals.unique_passengers}`);
    lines.push(`Wheelchair requests,${d.totals.wheelchair_requests}`);
    if (d.totals.stroller_requests != null) lines.push(`Stroller requests,${d.totals.stroller_requests}`);
    if (d.totals.oxygen_requests != null) lines.push(`Oxygen requests,${d.totals.oxygen_requests}`);
    lines.push(`Return legs,${d.totals.return_requests}`);
    if (d.totals.proxy_rides != null) lines.push(`Proxy rides,${d.totals.proxy_rides}`);
    lines.push(`Cancelled unassigned,${d.totals.cancelled_unassigned}`);
    if (d.totals.plan_visitors_count != null) {
      lines.push(`Plan visitors,${d.totals.plan_visitors_count}`);
    }
    if (d.totals.visitor_passes_count != null) {
      lines.push(`Visitor passes,${d.totals.visitor_passes_count}`);
    }
    lines.push('');
    const hasSupply = d.by_area.some((a) => a.registered_drivers != null);
    lines.push(
      hasSupply
        ? 'Area,Total,Fulfilled,Unfilled,Seats Needed,Seats Unfilled,Rate (%),Registered Drivers'
        : 'Area,Total,Fulfilled,Unfilled,Seats Needed,Seats Unfilled,Rate (%)',
    );
    for (const a of d.by_area) {
      const name =
        (isZh ? a.area_name_zh || a.area_name_en : a.area_name_en || a.area_name_zh) ||
        `#${a.area_id}`;
      const base = `${escapeCsv(name)},${a.total},${a.fulfilled},${a.unfilled},${a.seats_needed},${a.seats_unfilled},${a.fulfillment_rate_pct}%`;
      lines.push(hasSupply ? `${base},${a.registered_drivers ?? 0}` : base);
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
      lines.push(
        `${escapeCsv(wd)},${escapeCsv(s.slot)},${s.total},${s.fulfilled},${s.unfilled},${s.seats_needed}`,
      );
    }
    downloadCsv(`grace-ride-demand-report-${d.period}`, lines);
  };

  const exportPdf = async () => {
    if (!d || exportingPdf) return;
    setExportErr(null);
    setExportingPdf(true);
    try {
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
              const supply =
                a.registered_drivers != null
                  ? ` · ${t('reports.registeredDriversTag', { count: a.registered_drivers })}`
                  : '';
              return `${name} — ${t('reports.demandAreaLine', { total: a.total, unfilled: a.unfilled, rate: a.fulfillment_rate_pct })}${supply}`;
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
    } catch (e) {
      setExportErr(e instanceof Error ? e.message : String(e));
    } finally {
      setExportingPdf(false);
    }
  };

  return (
    <div className="stack">
      <PeriodControls
        period={period}
        scope={d?.scope}
        extraFilter={
          groups.length > 0 ? (
            <Field label={t('reports.groupFilter')}>
              <select
                className="input"
                value={groupId}
                onChange={(e) => setGroupId(e.target.value)}
              >
                <option value="">{t('reports.allGroups')}</option>
                {groups.map((g) => (
                  <option key={g.id} value={g.id}>
                    {g.name}
                  </option>
                ))}
              </select>
            </Field>
          ) : undefined
        }
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
              disabled={!d || exportingPdf}
              loading={exportingPdf}
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

      <ErrorBox error={error || exportErr} />

      {loading ? (
        <CardSkeleton count={3} />
      ) : !d ? (
        <EmptyState title={t('reports.none')} />
      ) : (
        <>
          <div className="report-stats">
            <Stat
              label={t('reports.demandTotal')}
              value={
                <span className="report-stat-val">
                  <span>{d.totals.total_requests}</span>
                  <DeltaPill delta={cmp?.total_requests_delta} prevPeriod={cmp?.prev_period} />
                </span>
              }
            />
            <Stat label={t('reports.demandFulfilled')} value={d.totals.fulfilled} />
            <Stat
              label={t('reports.demandUnfilled')}
              value={
                <span className="report-stat-val">
                  <span>{d.totals.unfilled}</span>
                  <DeltaPill
                    delta={cmp?.unfilled_delta}
                    invertGood
                    prevPeriod={cmp?.prev_period}
                  />
                </span>
              }
            />
            <Stat
              label={t('reports.demandFulfillmentRate')}
              value={
                <span className="report-stat-val">
                  <span>{d.totals.fulfillment_rate_pct}%</span>
                  <DeltaPill
                    delta={cmp?.fulfillment_rate_delta}
                    unit="%"
                    prevPeriod={cmp?.prev_period}
                  />
                </span>
              }
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

          {((d.totals.pending_confirmation ?? 0) > 0 ||
            (d.totals.stroller_requests ?? 0) > 0 ||
            (d.totals.oxygen_requests ?? 0) > 0 ||
            (d.totals.no_show ?? 0) > 0 ||
            (d.totals.alt_transport ?? 0) > 0) && (
            <div className="row gap-xs flex-wrap">
              {(d.totals.pending_confirmation ?? 0) > 0 && (
                <Pill tone="warning">
                  {t('reports.pendingConfirmationTag', { count: d.totals.pending_confirmation })}
                </Pill>
              )}
              {(d.totals.stroller_requests ?? 0) > 0 && (
                <Pill tone="info">
                  {t('reports.strollerTag', { count: d.totals.stroller_requests })}
                </Pill>
              )}
              {(d.totals.oxygen_requests ?? 0) > 0 && (
                <Pill tone="info">
                  {t('reports.oxygenTag', { count: d.totals.oxygen_requests })}
                </Pill>
              )}
              {(d.totals.no_show ?? 0) > 0 && (
                <Pill tone="danger">{t('reports.noShowTag', { count: d.totals.no_show })}</Pill>
              )}
              {(d.totals.alt_transport ?? 0) > 0 && (
                <Pill tone="neutral">
                  {t('reports.altTransportTag', { count: d.totals.alt_transport })}
                </Pill>
              )}
            </div>
          )}

          <div className="report-cols-2">
            <Card className="stack-sm">
              <h2>{t('reports.demandByArea')}</h2>
              {d.by_area.length === 0 ? (
                <p className="muted small">{t('reports.none')}</p>
              ) : (
                <div className="stack-sm">
                  {d.by_area.map((a) => (
                    <div key={a.area_id} className="report-bar-row">
                      <div className="report-bar-head">
                        <span className="bold">
                          {(isZh ? a.area_name_zh || a.area_name_en : a.area_name_en || a.area_name_zh) ||
                            `#${a.area_id}`}
                        </span>
                        <span className="small muted">
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
                      {a.registered_drivers != null && (
                        <div className="muted tiny">
                          {t('reports.registeredDriversTag', { count: a.registered_drivers })}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </Card>

            <Card className="stack-sm">
              <h2>{t('reports.demandBySlot')}</h2>
              {d.by_slot.length === 0 ? (
                <p className="muted small">{t('reports.none')}</p>
              ) : (
                <div className="list">
                  {d.by_slot.map((s) => (
                    <div key={`${s.weekday}-${s.slot}`} className="item report-kv-item">
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
              <h2>{t('reports.demandByGroup')}</h2>
              <div className="report-cols-2">
                {d.by_group.map((g, idx) => (
                  <div key={g.group_id ?? `none-${idx}`} className="report-bar-row">
                    <div className="report-bar-head">
                      <span className="bold">
                        {g.group_name ?? t('reports.unassignedGroup')}
                      </span>
                      <span className="small muted">
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
