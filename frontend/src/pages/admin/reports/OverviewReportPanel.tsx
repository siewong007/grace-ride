import { useState } from 'react';
import { ChevronRight, Download, FileText } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useApi } from '../../../api';
import { Button, Card, CardSkeleton, EmptyState, ErrorBox, Pill, Stat } from '../../../components/ui';
import { rm } from '../../../format';
import { downloadCsv, escapeCsv, exportReportPdf, fmtRmExport } from '../../../reportExport';
import type { OverviewReportData } from '../../../types/domain';
import {
  buildPeriodQuery,
  DeltaPill,
  fmtCompactRm,
  MiniSeriesChart,
  PeriodControls,
  type PeriodFilterState,
  StatCountAmount,
} from './PeriodControls';

export default function OverviewReportPanel({
  period,
  onOpenMember,
  onOpenEventsGroup,
}: {
  period: PeriodFilterState;
  onOpenMember?: (userId: number) => void;
  onOpenEventsGroup?: (groupId: number) => void;
}) {
  const { t } = useTranslation('admin');
  const [exportingPdf, setExportingPdf] = useState(false);
  const [exportErr, setExportErr] = useState<string | null>(null);
  const query = buildPeriodQuery(period);
  const { data, loading, error } = useApi<OverviewReportData>(
    query ? `/admin/reports/overview?${query}` : null,
  );
  const o = data && 'period' in data ? data : null;
  const cmp = o?.comparison;

  const exportCsv = () => {
    if (!o) return;
    const lines: string[] = [];
    lines.push('Grace Ride — Overview Report Export');
    lines.push(`Period,${escapeCsv(o.period)}`);
    lines.push(`Window,${escapeCsv(`${o.from} to ${o.to}`)}`);
    lines.push('');

    if (o.transport) {
      lines.push('Transport Summary,Value');
      lines.push(`Scope,${o.transport.scope}`);
      lines.push(`Total rides,${o.transport.total_rides}`);
      lines.push(`Completed rides,${o.transport.completed_rides}`);
      if (o.transport.pending_confirmation_rides != null) {
        lines.push(`Pending confirmation rides,${o.transport.pending_confirmation_rides}`);
      }
      lines.push(`Unfilled rides,${o.transport.unfilled_rides}`);
      lines.push(`Cancelled rides,${o.transport.cancelled_rides}`);
      lines.push(`No-show rides,${o.transport.no_show_rides}`);
      lines.push(`Alt-transport rides,${o.transport.alt_transport_rides}`);
      lines.push(`Fulfillment rate (%),${o.transport.fulfillment_rate_pct}%`);
      lines.push(`Distance (km),${o.transport.total_distance_km}`);
      lines.push(`Active drivers,${o.transport.active_drivers}`);
      lines.push(`Unique passengers,${o.transport.unique_passengers}`);
      lines.push('');
      if (o.transport.top_drivers.length > 0) {
        lines.push('Driver,Completed Rides,Distance (km),Carpool Trips');
        for (const d of o.transport.top_drivers) {
          lines.push(`${escapeCsv(d.name)},${d.completed_rides},${d.distance_km},${d.carpool_trips}`);
        }
        lines.push('');
      }
    }

    if (o.events) {
      lines.push('Events Summary,Value');
      lines.push(`Scope,${o.events.scope}`);
      lines.push(`Events count,${o.events.events_count}`);
      if (o.events.tracked_events_count != null) {
        lines.push(`Tracked attendance events,${o.events.tracked_events_count}`);
      }
      lines.push(`Total going,${o.events.total_going}`);
      lines.push(`Total attended,${o.events.total_attended}`);
      lines.push(`Attendance rate (%),${o.events.attendance_rate_pct}%`);
      lines.push('');
    }

    if (o.reimb_review) {
      lines.push('Reimbursement Review Queue,Count,Amount (MYR)');
      lines.push(
        `Pending,${o.reimb_review.pending_count},${escapeCsv(fmtRmExport(o.reimb_review.pending_cents))}`,
      );
      lines.push(`Returned,${o.reimb_review.returned_count},`);
      lines.push(
        `Approved awaiting payout,${o.reimb_review.approved_count},${escapeCsv(fmtRmExport(o.reimb_review.approved_cents))}`,
      );
      lines.push('');
    }

    if (o.finance?.reimb_payouts) {
      const rp = o.finance.reimb_payouts;
      lines.push('Finance Payouts,Count,Amount (MYR)');
      lines.push(
        `Approved unpaid,${rp.approved_unpaid_count},${escapeCsv(fmtRmExport(rp.approved_unpaid_cents))}`,
      );
      lines.push(`Paid,${rp.paid_count},${escapeCsv(fmtRmExport(rp.paid_cents))}`);
      lines.push(`Open batches,${rp.open_batches},`);
      lines.push(`Paid batches,${rp.paid_batches},`);
      lines.push('');
    }

    if (o.finance?.bills) {
      const b = o.finance.bills;
      lines.push('Activity Bills,Value');
      lines.push(`Published,${b.published_count}`);
      lines.push(`Closed,${b.closed_count}`);
      lines.push(`Gross,${escapeCsv(fmtRmExport(b.gross_cents))}`);
      lines.push(`Subsidy,${escapeCsv(fmtRmExport(b.subsidy_cents))}`);
      lines.push(`Collected,${escapeCsv(fmtRmExport(b.collected_cents))}`);
      lines.push(`Outstanding,${escapeCsv(fmtRmExport(b.outstanding_cents))}`);
      lines.push(`Collection rate (%),${b.collection_rate_pct}%`);
      lines.push('');
    }

    if (o.membership) {
      lines.push('Membership,Value');
      lines.push(`Active members,${o.membership.active_members}`);
      lines.push(`Pending members,${o.membership.pending_members}`);
      lines.push(`New in period,${o.membership.new_members_in_period}`);
      lines.push(`Active drivers,${o.membership.active_drivers_total}`);
    }

    downloadCsv(`grace-ride-overview-report-${o.period}`, lines);
  };

  const exportPdf = async () => {
    if (!o || exportingPdf) return;
    setExportErr(null);
    setExportingPdf(true);
    try {
      const summaryRows: [string, string][] = [];
      if (o.transport) {
        summaryRows.push(
          [t('reports.overviewCompletedRides'), String(o.transport.completed_rides)],
          [t('reports.demandFulfillmentRate'), `${o.transport.fulfillment_rate_pct}%`],
          [t('reports.distance'), `${o.transport.total_distance_km} km`],
        );
      }
      if (o.events) {
        summaryRows.push(
          [t('reports.eventsCount'), String(o.events.events_count)],
          [t('reports.attendanceRate'), `${o.events.attendance_rate_pct}%`],
        );
      }
      if (o.reimb_review) {
        summaryRows.push([
          t('reports.reimbPendingReview'),
          `${o.reimb_review.pending_count} (${fmtRmExport(o.reimb_review.pending_cents)})`,
        ]);
      }
      if (o.finance?.reimb_payouts) {
        summaryRows.push([
          t('reports.payoutsPaid'),
          `${o.finance.reimb_payouts.paid_count} (${fmtRmExport(o.finance.reimb_payouts.paid_cents)})`,
        ]);
      }
      if (o.finance?.bills) {
        summaryRows.push([
          t('reports.billsCollected'),
          `${fmtRmExport(o.finance.bills.collected_cents)} (${o.finance.bills.collection_rate_pct}%)`,
        ]);
      }
      if (o.membership) {
        summaryRows.push([t('reports.activeMembers'), String(o.membership.active_members)]);
      }

      const sections = [];
      if (o.transport && o.transport.top_drivers.length > 0) {
        sections.push({
          heading: t('reports.topDrivers'),
          items: o.transport.top_drivers.map(
            (d) =>
              `${d.name} — ${d.completed_rides} ${t('reports.ridesUnit')} · ${d.distance_km} km · ${d.carpool_trips} ${t('reports.carpoolUnit')}`,
          ),
        });
      }
      if (o.events && o.events.top_groups.length > 0) {
        sections.push({
          heading: t('reports.topGroups'),
          items: o.events.top_groups.map(
            (g) =>
              `${g.group_name ?? t('reports.unassignedGroup')} — ${g.attended}/${g.going}`,
          ),
        });
      }

      await exportReportPdf({
        fileBase: `grace-ride-overview-report-${o.period}`,
        title: t('reports.overviewPdfTitle'),
        meta: [
          `${t('reports.period')}: ${o.period}`,
          `${t('reports.window')}: ${o.from} – ${o.to}`,
        ],
        summaryHeading: t('reports.summary'),
        summaryRows,
        sections,
      });
    } catch (e) {
      setExportErr(e instanceof Error ? e.message : String(e));
    } finally {
      setExportingPdf(false);
    }
  };

  const ridesSeries =
    o?.monthly_series
      .filter((m) => m.rides_completed != null)
      .map((m) => ({ month: m.month, value: m.rides_completed ?? 0 })) ?? [];
  const eventsSeries =
    o?.monthly_series
      .filter((m) => m.events_attended != null)
      .map((m) => ({ month: m.month, value: m.events_attended ?? 0 })) ?? [];
  const payoutsSeries =
    o?.monthly_series
      .filter((m) => m.reimb_paid_cents != null)
      .map((m) => ({ month: m.month, value: m.reimb_paid_cents ?? 0 })) ?? [];
  const billsSeries =
    o?.monthly_series
      .filter((m) => m.bills_collected_cents != null)
      .map((m) => ({ month: m.month, value: m.bills_collected_cents ?? 0 })) ?? [];

  return (
    <div className="stack">
      <PeriodControls
        period={period}
        actions={
          <>
            <Button
              variant="secondary"
              size="sm"
              onClick={exportCsv}
              disabled={!o}
              icon={<FileText size={16} />}
            >
              {t('reports.exportCsv')}
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={exportPdf}
              disabled={!o || exportingPdf}
              loading={exportingPdf}
              icon={<Download size={16} />}
            >
              {t('reports.exportPdf')}
            </Button>
          </>
        }
      />

      <ErrorBox error={error || exportErr} />

      {loading ? (
        <CardSkeleton count={3} />
      ) : !o ? (
        <EmptyState title={t('reports.none')} />
      ) : (
        <>
          {o.transport && (
            <Card className="stack-sm">
              <div className="row between gap-sm items-center flex-wrap">
                <h2>{t('reports.overviewTransport')}</h2>
                <Pill tone={o.transport.scope === 'global' ? 'info' : 'warning'}>
                  {o.transport.scope === 'global'
                    ? t('reports.scopeGlobal')
                    : t('reports.scopeOwnEvents')}
                </Pill>
              </div>
              <div className="report-stats">
                <Stat
                  label={t('reports.demandTotal')}
                  value={
                    <span className="report-stat-val">
                      <span>{o.transport.total_rides}</span>
                      <DeltaPill
                        delta={cmp?.transport?.total_rides_delta}
                        prevPeriod={cmp?.prev_period}
                      />
                    </span>
                  }
                />
                <Stat
                  label={t('reports.overviewCompletedRides')}
                  value={
                    <span className="report-stat-val">
                      <span>{o.transport.completed_rides}</span>
                      <DeltaPill
                        delta={cmp?.transport?.completed_rides_delta}
                        prevPeriod={cmp?.prev_period}
                      />
                    </span>
                  }
                />
                <Stat label={t('reports.demandUnfilled')} value={o.transport.unfilled_rides} />
                <Stat
                  label={t('reports.demandFulfillmentRate')}
                  value={
                    <span className="report-stat-val">
                      <span>{o.transport.fulfillment_rate_pct}%</span>
                      <DeltaPill
                        delta={cmp?.transport?.fulfillment_rate_delta}
                        unit="%"
                        prevPeriod={cmp?.prev_period}
                      />
                    </span>
                  }
                />
                <Stat
                  label={t('reports.distance')}
                  value={<span className="nowrap">{o.transport.total_distance_km} km</span>}
                />
                <Stat label={t('reports.demandActiveDrivers')} value={o.transport.active_drivers} />
                <Stat label={t('reports.uniquePassengers')} value={o.transport.unique_passengers} />
                <Stat label={t('reports.cancelledRides')} value={o.transport.cancelled_rides} />
              </div>
              {((o.transport.pending_confirmation_rides ?? 0) > 0 ||
                o.transport.no_show_rides > 0 ||
                o.transport.alt_transport_rides > 0) && (
                <div className="row gap-xs flex-wrap">
                  {(o.transport.pending_confirmation_rides ?? 0) > 0 && (
                    <Pill tone="warning">
                      {t('reports.pendingConfirmationTag', {
                        count: o.transport.pending_confirmation_rides,
                      })}
                    </Pill>
                  )}
                  {o.transport.no_show_rides > 0 && (
                    <Pill tone="danger">
                      {t('reports.noShowTag', { count: o.transport.no_show_rides })}
                    </Pill>
                  )}
                  {o.transport.alt_transport_rides > 0 && (
                    <Pill tone="neutral">
                      {t('reports.altTransportTag', { count: o.transport.alt_transport_rides })}
                    </Pill>
                  )}
                </div>
              )}
            </Card>
          )}

          {o.events && (
            <Card className="stack-sm">
              <div className="row between gap-sm items-center flex-wrap">
                <h2>{t('reports.overviewEvents')}</h2>
                <Pill tone={o.events.scope === 'global' ? 'info' : 'warning'}>
                  {o.events.scope === 'global'
                    ? t('reports.scopeGlobal')
                    : t('reports.scopeOwnEvents')}
                </Pill>
              </div>
              <div className="report-stats">
                <Stat
                  label={t('reports.eventsCount')}
                  value={
                    <span className="report-stat-val">
                      <span>{o.events.events_count}</span>
                      <DeltaPill
                        delta={cmp?.events?.events_count_delta}
                        prevPeriod={cmp?.prev_period}
                      />
                    </span>
                  }
                />
                <Stat label={t('reports.totalGoing')} value={o.events.total_going} />
                <Stat
                  label={t('reports.totalAttended')}
                  value={
                    <span className="report-stat-val">
                      <span>{o.events.total_attended}</span>
                      <DeltaPill
                        delta={cmp?.events?.total_attended_delta}
                        prevPeriod={cmp?.prev_period}
                      />
                    </span>
                  }
                />
                <Stat
                  label={t('reports.attendanceRate')}
                  value={
                    <span className="report-stat-val">
                      <span>{o.events.attendance_rate_pct}%</span>
                      <DeltaPill
                        delta={cmp?.events?.attendance_rate_delta}
                        unit="%"
                        prevPeriod={cmp?.prev_period}
                      />
                    </span>
                  }
                />
              </div>
              {o.events.tracked_events_count != null &&
                o.events.tracked_events_count < o.events.events_count && (
                  <p className="muted tiny">
                    {t('reports.attendanceTrackedNote', {
                      tracked: o.events.tracked_events_count,
                      total: o.events.events_count,
                    })}
                  </p>
                )}
            </Card>
          )}

          {o.reimb_review && (
            <Card className="stack-sm">
              <h2>{t('reports.overviewReimbReview')}</h2>
              <div className="report-stats cols-3">
                <Stat
                  label={t('reports.reimbPendingReview')}
                  value={
                    <span className="report-stat-val">
                      <StatCountAmount
                        count={o.reimb_review.pending_count}
                        cents={o.reimb_review.pending_cents}
                      />
                      <DeltaPill
                        delta={cmp?.reimb_review?.pending_count_delta}
                        invertGood
                        prevPeriod={cmp?.prev_period}
                      />
                    </span>
                  }
                />
                <Stat label={t('reports.reimbReturned')} value={o.reimb_review.returned_count} />
                <Stat
                  label={t('reports.reimbApprovedAwaiting')}
                  value={
                    <StatCountAmount
                      count={o.reimb_review.approved_count}
                      cents={o.reimb_review.approved_cents}
                    />
                  }
                />
              </div>
            </Card>
          )}

          {o.finance && (
            <div className={o.finance.reimb_payouts && o.finance.bills ? 'report-cols-2' : 'stack'}>
              {o.finance.reimb_payouts && (
                <Card className="stack-sm">
                  <h2>{t('reports.overviewPayouts')}</h2>
                  <div className="report-stats cols-2">
                    <Stat
                      label={t('reports.reimbApprovedAwaiting')}
                      value={<span className="nowrap">{rm(o.finance.reimb_payouts.approved_unpaid_cents)}</span>}
                    />
                    <Stat
                      label={t('reports.payoutsPaid')}
                      value={
                        <span className="report-stat-val">
                          <span className="nowrap">{rm(o.finance.reimb_payouts.paid_cents)}</span>
                          <DeltaPill
                            delta={cmp?.reimb_payouts?.paid_cents_delta}
                            formatAbs={rm}
                            prevPeriod={cmp?.prev_period}
                          />
                        </span>
                      }
                    />
                    <Stat
                      label={t('reports.openBatches')}
                      value={o.finance.reimb_payouts.open_batches}
                    />
                    <Stat
                      label={t('reports.paidBatches')}
                      value={o.finance.reimb_payouts.paid_batches}
                    />
                  </div>
                </Card>
              )}

              {o.finance.bills && (
                <Card className="stack-sm">
                  <h2>{t('reports.overviewBills')}</h2>
                  <div className="report-stats cols-2">
                    <Stat
                      label={t('reports.billsCollected')}
                      value={
                        <span className="report-stat-val">
                          <span className="nowrap">{rm(o.finance.bills.collected_cents)}</span>
                          <DeltaPill
                            delta={cmp?.bills?.collected_cents_delta}
                            formatAbs={rm}
                            prevPeriod={cmp?.prev_period}
                          />
                        </span>
                      }
                    />
                    <Stat
                      label={t('reports.billsOutstanding')}
                      value={<span className="nowrap">{rm(o.finance.bills.outstanding_cents)}</span>}
                    />
                    <Stat
                      label={t('reports.collectionRate')}
                      value={
                        <span className="report-stat-val">
                          <span>{o.finance.bills.collection_rate_pct}%</span>
                          <DeltaPill
                            delta={cmp?.bills?.collection_rate_delta}
                            unit="%"
                            prevPeriod={cmp?.prev_period}
                          />
                        </span>
                      }
                    />
                    <Stat
                      label={t('reports.billsPublishedClosed')}
                      value={`${o.finance.bills.published_count} / ${o.finance.bills.closed_count}`}
                    />
                  </div>
                </Card>
              )}
            </div>
          )}

          {o.membership && (
            <Card className="stack-sm">
              <h2>{t('reports.overviewMembership')}</h2>
              <div className="report-stats">
                <Stat label={t('reports.activeMembers')} value={o.membership.active_members} />
                <Stat label={t('reports.pendingMembers')} value={o.membership.pending_members} />
                <Stat
                  label={t('reports.newMembers')}
                  value={
                    <span className="report-stat-val">
                      <span>{o.membership.new_members_in_period}</span>
                      <DeltaPill
                        delta={cmp?.membership?.new_members_delta}
                        prevPeriod={cmp?.prev_period}
                      />
                    </span>
                  }
                />
                <Stat label={t('reports.activeDriversTotal')} value={o.membership.active_drivers_total} />
              </div>
            </Card>
          )}

          {o.monthly_series.length > 1 && (
            <div className="report-cols-2">
              {ridesSeries.length > 1 && (
                <MiniSeriesChart title={t('reports.trendRides')} items={ridesSeries} headingLevel="h2" />
              )}
              {eventsSeries.length > 1 && (
                <MiniSeriesChart title={t('reports.trendAttendance')} items={eventsSeries} headingLevel="h2" />
              )}
              {payoutsSeries.length > 1 && (
                <MiniSeriesChart
                  title={t('reports.trendPayouts')}
                  items={payoutsSeries}
                  formatVal={fmtCompactRm}
                  formatTooltip={rm}
                  headingLevel="h2"
                />
              )}
              {billsSeries.length > 1 && (
                <MiniSeriesChart
                  title={t('reports.trendBills')}
                  items={billsSeries}
                  formatVal={fmtCompactRm}
                  formatTooltip={rm}
                  headingLevel="h2"
                />
              )}
            </div>
          )}

          {(o.transport?.top_drivers.length || o.events?.top_groups.length) ? (
            <div className={o.transport?.top_drivers.length && o.events?.top_groups.length ? 'report-cols-2' : 'stack'}>
              {o.transport && o.transport.top_drivers.length > 0 && (
                <Card className="stack-sm">
                  <h2>{t('reports.topDrivers')}</h2>
                  <div className="list">
                    {o.transport.top_drivers.map((d) =>
                      onOpenMember ? (
                        <button
                          key={d.user_id}
                          type="button"
                          className="item report-rank-row report-drill-btn"
                          onClick={() => onOpenMember(d.user_id)}
                        >
                          <span className="bold">{d.name}</span>
                          <span className="report-rank-meta small muted">
                            <span>
                              {d.completed_rides} {t('reports.ridesUnit')} · {d.distance_km} km
                              {d.carpool_trips > 0
                                ? ` · ${d.carpool_trips} ${t('reports.carpoolUnit')}`
                                : ''}
                            </span>
                            <ChevronRight size={16} className="report-drill-chevron" aria-hidden="true" />
                          </span>
                        </button>
                      ) : (
                        <div key={d.user_id} className="item report-rank-row">
                          <span className="bold">{d.name}</span>
                          <span className="report-rank-meta small muted">
                            {d.completed_rides} {t('reports.ridesUnit')} · {d.distance_km} km
                            {d.carpool_trips > 0
                              ? ` · ${d.carpool_trips} ${t('reports.carpoolUnit')}`
                              : ''}
                          </span>
                        </div>
                      ),
                    )}
                  </div>
                </Card>
              )}

              {o.events && o.events.top_groups.length > 0 && (
                <Card className="stack-sm">
                  <h2>{t('reports.topGroups')}</h2>
                  <div className="list">
                    {o.events.top_groups.map((g, idx) =>
                      onOpenEventsGroup && g.group_id != null ? (
                        <button
                          key={g.group_id}
                          type="button"
                          className="item report-rank-row report-drill-btn"
                          onClick={() => onOpenEventsGroup(g.group_id!)}
                        >
                          <span className="bold">
                            {g.group_name ?? t('reports.unassignedGroup')}
                          </span>
                          <span className="report-rank-meta small muted">
                            <span>
                              {g.attended} / {g.going} {t('reports.attendedShort')}
                            </span>
                            <ChevronRight size={16} className="report-drill-chevron" aria-hidden="true" />
                          </span>
                        </button>
                      ) : (
                        <div key={g.group_id ?? `none-${idx}`} className="item report-rank-row">
                          <span className="bold">
                            {g.group_name ?? t('reports.unassignedGroup')}
                          </span>
                          <span className="report-rank-meta small muted">
                            {g.attended} / {g.going} {t('reports.attendedShort')}
                          </span>
                        </div>
                      ),
                    )}
                  </div>
                </Card>
              )}
            </div>
          ) : null}
        </>
      )}
    </div>
  );
}
