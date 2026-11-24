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
  Notice,
  Pill,
  Stat,
} from '../../../components/ui';
import { rm } from '../../../format';
import { downloadCsv, escapeCsv, exportReportPdf, fmtRmExport } from '../../../reportExport';
import type { FinanceReportData } from '../../../types/domain';
import { buildPeriodQuery, PeriodControls, type PeriodFilterState } from './PeriodControls';

export default function FinanceReportPanel({ period }: { period: PeriodFilterState }) {
  const { t } = useTranslation('admin');
  const query = buildPeriodQuery(period);
  const { data, loading, error } = useApi<FinanceReportData>(`/admin/reports/finance?${query}`);
  const f = data && 'permissions' in data ? data : null;

  const exportCsv = () => {
    if (!f) return;
    const lines: string[] = [];
    lines.push('Grace Ride — Finance & Reimbursements Report Export');
    lines.push(`Period,${escapeCsv(f.period)}`);
    lines.push(`Window,${escapeCsv(`${f.from} to ${f.to}`)}`);
    lines.push('');

    if (f.reimbursement_review) {
      const rr = f.reimbursement_review;
      lines.push('Reimbursement Review Queue,Count,Amount (MYR)');
      for (const st of ['pending', 'returned', 'rejected', 'approved', 'paid'] as const) {
        lines.push(`${st},${rr.by_status[st].count},${escapeCsv(fmtRmExport(rr.by_status[st].amount_cents))}`);
      }
      lines.push('');
      lines.push('Claim Kind,Count,Amount (MYR)');
      for (const k of rr.by_kind) {
        lines.push(`${escapeCsv(k.kind)},${k.count},${escapeCsv(fmtRmExport(k.amount_cents))}`);
      }
      lines.push('');
      lines.push('Driver Mileage Rides,Value');
      lines.push(`Eligible rides,${rr.driver_mileage_rides.eligible_count}`);
      lines.push(`Eligible amount,${escapeCsv(fmtRmExport(rr.driver_mileage_rides.eligible_cents))}`);
      lines.push(`Excluded rides,${rr.driver_mileage_rides.excluded_count}`);
      lines.push(`Receipt needed rides,${rr.driver_mileage_rides.receipt_needed_count}`);
      lines.push('');
    }

    if (f.reimbursement_payouts) {
      const rp = f.reimbursement_payouts;
      lines.push('Finance Payouts Summary,Value');
      lines.push(
        `Approved unpaid (period),${rp.approved_unpaid_count} (${fmtRmExport(rp.approved_unpaid_cents)})`,
      );
      lines.push(
        `All-time awaiting payout,${rp.all_time_awaiting_count} (${fmtRmExport(rp.all_time_awaiting_cents)})`,
      );
      lines.push(`Paid (period),${rp.paid_count} (${fmtRmExport(rp.paid_cents)})`);
      lines.push(`Payee verified,${rp.payee_verified_count}`);
      lines.push(`Payee unverified,${rp.payee_unverified_count}`);
      lines.push('');
      if (rp.batches.length > 0) {
        lines.push('Batch ID,Month,Title,Status,Claims,Total (MYR),Created By,Paid By,Maker-Checker OK,Paid Ref');
        for (const b of rp.batches) {
          lines.push(
            `${b.id},${escapeCsv(b.month)},${escapeCsv(b.title)},${escapeCsv(b.status)},${b.claim_count},${escapeCsv(fmtRmExport(b.total_cents))},${escapeCsv(b.created_by_name)},${escapeCsv(b.paid_by_name ?? '')},${b.maker_checker_ok},${escapeCsv(b.paid_ref)}`,
          );
        }
        lines.push('');
      }
    }

    if (f.bills) {
      const bt = f.bills.totals;
      lines.push('Activity Settlements Summary,Value');
      lines.push(`Settlements (published/closed),${bt.total_settlements}`);
      lines.push(`Drafts,${bt.draft_count}`);
      lines.push(`Gross,${escapeCsv(fmtRmExport(bt.gross_cents))}`);
      lines.push(`Subsidy,${escapeCsv(fmtRmExport(bt.subsidy_cents))}`);
      lines.push(`Net billed,${escapeCsv(fmtRmExport(bt.net_billed_cents))}`);
      lines.push(`Collected,${escapeCsv(fmtRmExport(bt.collected_cents))}`);
      lines.push(`Collection rate (%),${bt.collection_rate_pct}%`);
      lines.push('');
      if (f.bills.settlements.length > 0) {
        lines.push('Settlement ID,Title,Event,Organiser,Status,Net Billed (MYR),Collected (MYR),Outstanding (MYR),Confirmed/Total');
        for (const s of f.bills.settlements) {
          lines.push(
            `${s.id},${escapeCsv(s.title)},${escapeCsv(s.event_title)},${escapeCsv(s.organiser_name)},${escapeCsv(s.status)},${escapeCsv(fmtRmExport(s.net_billed_cents))},${escapeCsv(fmtRmExport(s.collected_cents))},${escapeCsv(fmtRmExport(s.outstanding_cents))},${s.confirmed_count}/${s.participant_count}`,
          );
        }
      }
    }

    downloadCsv(`grace-ride-finance-report-${f.period}`, lines);
  };

  const exportPdf = async () => {
    if (!f) return;
    const summaryRows: [string, string][] = [];
    const sections = [];

    if (f.reimbursement_review) {
      const rr = f.reimbursement_review;
      summaryRows.push(
        [
          t('reports.reimbPendingReview'),
          `${rr.by_status.pending.count} (${fmtRmExport(rr.by_status.pending.amount_cents)})`,
        ],
        [
          t('reports.driverMileageEligible'),
          `${rr.driver_mileage_rides.eligible_count} (${fmtRmExport(rr.driver_mileage_rides.eligible_cents)})`,
        ],
      );
      sections.push({
        heading: t('reports.reimbReviewTitle'),
        items: rr.by_kind.map(
          (k) => `${t(`reports.reimbKind_${k.kind}`)}: ${k.count} (${fmtRmExport(k.amount_cents)})`,
        ),
      });
    }

    if (f.reimbursement_payouts) {
      const rp = f.reimbursement_payouts;
      summaryRows.push(
        [
          t('reports.allTimeAwaitingPayout'),
          `${rp.all_time_awaiting_count} (${fmtRmExport(rp.all_time_awaiting_cents)})`,
        ],
        [t('reports.payoutsPaid'), `${rp.paid_count} (${fmtRmExport(rp.paid_cents)})`],
      );
      if (rp.batches.length > 0) {
        sections.push({
          heading: t('reports.payoutBatches'),
          items: rp.batches.map(
            (b) =>
              `${b.title} (${b.month}) — ${b.status} · ${fmtRmExport(b.total_cents)} · ${b.maker_checker_ok ? t('reports.makerCheckerOk') : t('reports.makerCheckerWarn')}`,
          ),
        });
      }
    }

    if (f.bills) {
      const bt = f.bills.totals;
      summaryRows.push(
        [t('reports.billsNetBilled'), fmtRmExport(bt.net_billed_cents)],
        [t('reports.billsCollected'), `${fmtRmExport(bt.collected_cents)} (${bt.collection_rate_pct}%)`],
      );
      if (f.bills.settlements.length > 0) {
        sections.push({
          heading: t('reports.billsSettlements'),
          items: f.bills.settlements.map(
            (s) =>
              `${s.title} (${s.event_title}) — ${fmtRmExport(s.collected_cents)} / ${fmtRmExport(s.net_billed_cents)}`,
          ),
        });
      }
    }

    await exportReportPdf({
      fileBase: `grace-ride-finance-report-${f.period}`,
      title: t('reports.financePdfTitle'),
      meta: [
        `${t('reports.period')}: ${f.period}`,
        `${t('reports.window')}: ${f.from} – ${f.to}`,
      ],
      summaryHeading: t('reports.summary'),
      summaryRows,
      sections,
    });
  };

  const reviewOnlyNotice =
    f && f.permissions.reimb_review && !f.permissions.reimb_payouts && !f.permissions.bills;

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
              disabled={!f}
              icon={<FileText size={16} />}
            >
              {t('reports.exportCsv')}
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={exportPdf}
              disabled={!f}
              icon={<Download size={16} />}
            >
              {t('reports.exportPdf')}
            </Button>
          </>
        }
      />

      {reviewOnlyNotice && <Notice tone="info">{t('reports.financeSeparationNotice')}</Notice>}

      <ErrorBox error={error} />

      {loading ? (
        <CardSkeleton count={3} />
      ) : !f ? (
        <EmptyState title={t('reports.none')} />
      ) : (
        <>
          {f.reimbursement_review && (
            <Card className="stack">
              <h3>{t('reports.reimbReviewTitle')}</h3>
              <div className="grid cols-4">
                <Stat
                  label={t('reports.reimbPendingReview')}
                  value={`${f.reimbursement_review.by_status.pending.count} · ${rm(f.reimbursement_review.by_status.pending.amount_cents)}`}
                />
                <Stat
                  label={t('reports.reimbReturned')}
                  value={f.reimbursement_review.by_status.returned.count}
                />
                <Stat
                  label={t('reports.reimbApproved')}
                  value={`${f.reimbursement_review.by_status.approved.count} · ${rm(f.reimbursement_review.by_status.approved.amount_cents)}`}
                />
                <Stat
                  label={t('reports.reimbRejected')}
                  value={f.reimbursement_review.by_status.rejected.count}
                />
              </div>

              <div className="grid cols-2">
                <div className="stack-xs">
                  <h4>{t('reports.reimbByKind')}</h4>
                  <div className="list">
                    {f.reimbursement_review.by_kind.map((k) => (
                      <div key={k.kind} className="item row between nowrap gap-sm">
                        <span className="bold">{t(`reports.reimbKind_${k.kind}`)}</span>
                        <span className="small muted nowrap">
                          {k.count} · {rm(k.amount_cents)}
                        </span>
                      </div>
                    ))}
                  </div>
                  {f.reimbursement_review.high_amount_special_count > 0 && (
                    <p className="muted tiny">
                      {t('reports.highAmountSpecialCount', {
                        count: f.reimbursement_review.high_amount_special_count,
                      })}
                    </p>
                  )}
                </div>

                <div className="stack-xs">
                  <h4>{t('reports.driverMileageSummary')}</h4>
                  <div className="list">
                    <div className="item row between nowrap gap-sm">
                      <span className="muted">{t('reports.driverMileageEligible')}</span>
                      <span className="bold nowrap">
                        {f.reimbursement_review.driver_mileage_rides.eligible_count} ·{' '}
                        {rm(f.reimbursement_review.driver_mileage_rides.eligible_cents)}
                      </span>
                    </div>
                    <div className="item row between nowrap gap-sm">
                      <span className="muted">{t('reports.driverMileageExcluded')}</span>
                      <span className="bold">
                        {f.reimbursement_review.driver_mileage_rides.excluded_count}
                      </span>
                    </div>
                    <div className="item row between nowrap gap-sm">
                      <span className="muted">{t('reports.driverMileageReceiptNeeded')}</span>
                      <span className="bold">
                        {f.reimbursement_review.driver_mileage_rides.receipt_needed_count}
                      </span>
                    </div>
                  </div>
                </div>
              </div>
            </Card>
          )}

          {f.reimbursement_payouts && (
            <Card className="stack">
              <h3>{t('reports.payoutsTitle')}</h3>
              <div className="grid cols-4">
                <Stat
                  label={t('reports.reimbApprovedAwaiting')}
                  value={`${f.reimbursement_payouts.approved_unpaid_count} · ${rm(f.reimbursement_payouts.approved_unpaid_cents)}`}
                />
                <Stat
                  label={t('reports.allTimeAwaitingPayout')}
                  value={`${f.reimbursement_payouts.all_time_awaiting_count} · ${rm(f.reimbursement_payouts.all_time_awaiting_cents)}`}
                />
                <Stat
                  label={t('reports.payoutsPaid')}
                  value={`${f.reimbursement_payouts.paid_count} · ${rm(f.reimbursement_payouts.paid_cents)}`}
                />
                <Stat
                  label={t('reports.payeeVerifiedRatio')}
                  value={`${f.reimbursement_payouts.payee_verified_count} / ${f.reimbursement_payouts.payee_verified_count + f.reimbursement_payouts.payee_unverified_count}`}
                />
              </div>

              <div className="grid cols-2">
                <div className="stack-xs">
                  <h4>{t('reports.payoutsByKind')}</h4>
                  <div className="list">
                    {f.reimbursement_payouts.by_kind.map((k) => (
                      <div key={k.kind} className="item row between nowrap gap-sm">
                        <span className="bold">{t(`reports.reimbKind_${k.kind}`)}</span>
                        <span className="small muted nowrap">
                          {t('reports.payoutKindLine', {
                            approved: rm(k.approved_cents),
                            paid: rm(k.paid_cents),
                          })}
                        </span>
                      </div>
                    ))}
                  </div>
                </div>

                <div className="stack-xs">
                  <h4>{t('reports.payoutBatches')}</h4>
                  {f.reimbursement_payouts.batches.length === 0 ? (
                    <p className="muted small">{t('reports.noBatches')}</p>
                  ) : (
                    <div className="list">
                      {f.reimbursement_payouts.batches.map((b) => (
                        <div key={b.id} className="item">
                          <div className="grow min-w-0 stack-xs">
                            <div className="row gap-xs items-center flex-wrap">
                              <span className="bold ellipsis">{b.title}</span>
                              <Pill tone={b.status === 'paid' ? 'success' : 'warning'}>
                                {b.status}
                              </Pill>
                              <Pill tone={b.maker_checker_ok ? 'info' : 'danger'}>
                                {b.maker_checker_ok
                                  ? t('reports.makerCheckerOk')
                                  : t('reports.makerCheckerWarn')}
                              </Pill>
                            </div>
                            <div className="muted tiny">
                              {b.month} · {b.claim_count} · {b.created_by_name}
                              {b.paid_by_name ? ` → ${b.paid_by_name}` : ''}
                            </div>
                          </div>
                          <span className="bold nowrap">{rm(b.total_cents)}</span>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            </Card>
          )}

          {f.bills && (
            <Card className="stack">
              <h3>{t('reports.billsTitle')}</h3>
              <div className="grid cols-4">
                <Stat
                  label={t('reports.billsTotal')}
                  value={f.bills.totals.total_settlements}
                />
                <Stat
                  label={t('reports.billsNetBilled')}
                  value={rm(f.bills.totals.net_billed_cents)}
                />
                <Stat
                  label={t('reports.billsCollected')}
                  value={rm(f.bills.totals.collected_cents)}
                />
                <Stat
                  label={t('reports.collectionRate')}
                  value={`${f.bills.totals.collection_rate_pct}%`}
                />
                <Stat
                  label={t('reports.billsSubsidy')}
                  value={rm(f.bills.totals.subsidy_cents)}
                />
                <Stat
                  label={t('reports.billsPendingProof')}
                  value={rm(f.bills.totals.pending_verification_cents)}
                />
                <Stat
                  label={t('reports.billsNeedsInfo')}
                  value={rm(f.bills.totals.needs_info_cents)}
                />
                <Stat
                  label={t('reports.billsUnpaid')}
                  value={rm(f.bills.totals.unpaid_cents)}
                />
              </div>

              <div className="bar">
                <div style={{ width: `${Math.min(100, f.bills.totals.collection_rate_pct)}%` }} />
              </div>

              {f.bills.settlements.length === 0 ? (
                <p className="muted small">{t('reports.none')}</p>
              ) : (
                <div className="list">
                  {f.bills.settlements.map((s) => (
                    <Link key={s.id} to={`/settlements/${s.id}`} className="item">
                      <div className="grow min-w-0 stack-xs">
                        <div className="row gap-xs items-center flex-wrap">
                          <span className="bold ellipsis">{s.title}</span>
                          <Pill tone={s.status === 'closed' ? 'success' : 'info'}>
                            {s.status}
                          </Pill>
                        </div>
                        <div className="muted tiny">
                          {s.event_title} · {s.organiser_name} · {s.confirmed_count}/{s.participant_count}
                        </div>
                      </div>
                      <div className="stack-xs text-right no-wrap">
                        <span className="bold small">{rm(s.collected_cents)}</span>
                        <span className="muted tiny">
                          / {rm(s.net_billed_cents)}
                        </span>
                      </div>
                    </Link>
                  ))}
                </div>
              )}
            </Card>
          )}
        </>
      )}
    </div>
  );
}
