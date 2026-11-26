import { useState } from 'react';
import { Link } from 'react-router-dom';
import { Award, Calendar, CarFront, CheckCircle2, DollarSign, Download, FileText, Map, Receipt } from 'lucide-react';
import { useApi } from '../api';
import {
  Button,
  Card,
  EmptyState,
  ErrorBox,
  Field,
  Page,
  PageHeader,
  Pill,
  Spinner,
  Stat,
} from '../components/ui';
import { EVENT_KIND, fmtDateTime, parseLocal, rm, useFormatT } from '../format';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../auth';
import {
  BADGE_TIER_ROMAN,
  type BadgeCategory,
  BadgeIcon,
  isAnimatedBadgeIcon,
  resolveBadgeTier,
} from '../badgeIcon';
import {
  downloadCsv,
  escapeCsv,
  exportReportPdf,
  fmtRmExport,
  formatInclusivePeriodEnd,
} from '../reportExport';
import type { Badge, MonthReport } from '../types/domain';

export default function Report() {
  const { t, i18n } = useTranslation('report');
  const ft = useFormatT();
  const lang = i18n.language;
  const { can } = useAuth();
  const now = new Date();
  const [month, setMonth] = useState(
    `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`,
  );
  const [range, setRange] = useState<'monthly' | 'quarterly' | 'yearly'>('monthly');
  const [type, setType] = useState<'all' | 'driver' | 'passenger'>('all');
  const [badgeFilter, setBadgeFilter] = useState<'mine' | 'all' | BadgeCategory>('mine');
  const [exportingPdf, setExportingPdf] = useState(false);
  const [exportErr, setExportErr] = useState<string | null>(null);
  const [y, m] = month.split('-').map(Number);
  const params = new URLSearchParams({ month, range, type });
  const { data: r, error } = useApi<MonthReport>(
    `/reports/monthly?${params.toString()}`,
  );
  // Driving figures mean nothing to a member who has never driven: show them to drivers only.
  // A chosen ride type keeps the selector, so a former driver can switch back to "all".
  const drives = can('drive') || type !== 'all' || (r?.rides_driven ?? 0) > 0;

  const periodLabel = () => {
    const start = parseLocal(r?.period_start ?? `${month}-01`);
    const end = parseLocal(r?.period_end ?? `${month}-01`);
    // Backend period_end is half-open (first day of the month AFTER the period).
    if (r?.period_end) end.setMonth(end.getMonth() - 1);
    const fmt = (d: Date) =>
      d.toLocaleDateString(lang?.startsWith('zh') ? 'zh-CN' : 'en-GB', {
        year: 'numeric',
        month: 'long',
      });
    if (range === 'monthly') return fmt(start);
    return `${fmt(start)} – ${fmt(end)}`;
  };

  const isZh = lang?.startsWith('zh');
  const badgeChar = (b: Badge) => (isZh ? b.char_zh ?? b.char : b.char_en ?? b.char);
  const badgeName = (b: Badge) => (isZh ? b.name_zh ?? b.name : b.name_en ?? b.name);
  const badgeRule = (b: Badge) => (isZh ? b.rule_zh ?? b.rule : b.rule_en ?? b.rule);

  const fileBase = `grace-ride-report-${range}-${month}`;
  const inclusivePeriodEnd = r ? formatInclusivePeriodEnd(r.period_end, r.period_end_inclusive) : '';

  const exportCsv = () => {
    if (!r) return;
    const lines: string[] = [];
    lines.push('Grace Ride — Report Export');
    lines.push(`User,${escapeCsv(`${r.user.name} (${r.user.id})`)}`);
    lines.push(`Period,${escapeCsv(`${r.period_start} to ${inclusivePeriodEnd}`)}`);
    lines.push(`Range,${escapeCsv(r.range)}`);
    lines.push(`Type,${escapeCsv(r.type)}`);
    lines.push('');
    lines.push('Summary,Value');
    if (r.events_attended != null) lines.push(`Events attended,${r.events_attended}`);
    if (drives && r.rides_driven != null) lines.push(`Rides driven,${r.rides_driven}`);
    if (r.rides_received != null) lines.push(`Rides received,${r.rides_received}`);
    if (drives && type !== 'passenger' && r.distance_km != null) {
      lines.push(`Distance (km),${r.distance_km}`);
    }
    if (drives && type !== 'passenger' && r.reimbursement_cents != null) {
      lines.push(`Reimbursement (MYR),${escapeCsv(fmtRmExport(r.reimbursement_cents))}`);
    }
    if (r.costs_paid_cents != null) {
      lines.push(`Costs paid (MYR),${escapeCsv(fmtRmExport(r.costs_paid_cents))}`);
    }
    if (r.reimbursements_summary) {
      const rs = r.reimbursements_summary;
      lines.push('');
      lines.push('My Reimbursements,Count,Amount (MYR)');
      lines.push(`Total claims,${rs.claim_count},`);
      if (rs.pending_count != null && rs.pending_cents != null) {
        lines.push(`Pending,${rs.pending_count},${escapeCsv(fmtRmExport(rs.pending_cents))}`);
      }
      lines.push(`Approved,${rs.approved_count},${escapeCsv(fmtRmExport(rs.approved_cents))}`);
      lines.push(`Paid,${rs.paid_count},${escapeCsv(fmtRmExport(rs.paid_cents))}`);
      if (rs.returned_count != null) lines.push(`Returned,${rs.returned_count},`);
      if (rs.rejected_count != null) lines.push(`Rejected,${rs.rejected_count},`);
    }
    if (r.organised_events) {
      const oe = r.organised_events;
      lines.push('');
      lines.push('Events Organised,Value');
      lines.push(`Events organised,${oe.events_count}`);
      lines.push(`Total going,${oe.total_going}`);
      lines.push(`Total attended,${oe.total_attended}`);
      lines.push(`Attendance rate (%),${oe.attendance_rate_pct}%`);
      if (oe.settlements) {
        lines.push(`Bills organised,${oe.settlements.count}`);
        lines.push(`Bills gross (MYR),${escapeCsv(fmtRmExport(oe.settlements.gross_cents))}`);
        lines.push(`Bills subsidy (MYR),${escapeCsv(fmtRmExport(oe.settlements.subsidy_cents))}`);
        lines.push(`Bills collected (MYR),${escapeCsv(fmtRmExport(oe.settlements.collected_cents))}`);
        lines.push(`Bills outstanding (MYR),${escapeCsv(fmtRmExport(oe.settlements.outstanding_cents))}`);
      }
    }
    lines.push('');
    lines.push('Event Kind,Count');
    for (const k of r.by_kind) lines.push(`${escapeCsv(k.kind)},${k.count}`);
    lines.push('');
    lines.push('Event ID,Title,Kind,Starts At');
    for (const e of r.events)
      lines.push(`${e.id},${escapeCsv(e.title)},${escapeCsv(e.kind)},${escapeCsv(e.starts_at)}`);
    lines.push('');
    lines.push('Badge Key,Name,Tier,Rule,Progress,Target,Earned');
    for (const b of r.badges) {
      const tier = resolveBadgeTier(b);
      lines.push(
        `${escapeCsv(b.key)},${escapeCsv(badgeName(b))},${escapeCsv(tier)},${escapeCsv(badgeRule(b))},${b.progress},${b.target},${b.earned}`,
      );
    }
    downloadCsv(fileBase, lines);
  };

  const exportPdf = async () => {
    if (!r || exportingPdf) return;
    setExportErr(null);
    setExportingPdf(true);
    try {
      const rangeMap: Record<string, string> = {
        monthly: t('rangeMonthly'),
        quarterly: t('rangeQuarterly'),
        yearly: t('rangeYearly'),
      };
      const typeMap: Record<string, string> = {
        all: t('typeAll'),
        driver: t('typeDriver'),
        passenger: t('typePassenger'),
      };
      const kinds = r.by_kind.map((k) => `${ft(EVENT_KIND[k.kind]?.label ?? k.kind)} × ${k.count}`);
      const events = r.events.map((e) => `${e.title} — ${fmtDateTime(e.starts_at, ft)}`);
      const badges = r.badges.map((b) => {
        const tier = resolveBadgeTier(b);
        const tierText = tier !== 'none' ? `[${t(`badgeTier_${tier}`)}]` : b.earned ? t('earned') : '';
        return `${badgeName(b)} ${tierText} — ${badgeRule(b)} (${b.progress}/${b.target})`.trim();
      });
      const rows: [string, string][] = [];
      if (r.events_attended != null) rows.push([t('events_attended'), String(r.events_attended)]);
      if (drives && r.rides_driven != null) rows.push([t('rides_driven'), String(r.rides_driven)]);
      if (r.rides_received != null) rows.push([t('rides_received'), String(r.rides_received)]);
      if (drives && type !== 'passenger' && r.distance_km != null) {
        rows.push([t('distance_km'), `${r.distance_km} km`]);
      }
      if (drives && type !== 'passenger' && r.reimbursement_cents != null) {
        rows.push([t('reimbursement'), fmtRmExport(r.reimbursement_cents)]);
      }
      if (r.costs_paid_cents != null) {
        rows.push([t('costs'), fmtRmExport(r.costs_paid_cents)]);
      }

      const extraSections = [];
      if (r.reimbursements_summary) {
        const rs = r.reimbursements_summary;
        const items = [
          `${t('reimbTotalClaims')}: ${rs.claim_count}`,
          ...(rs.pending_count != null && rs.pending_cents != null
            ? [`${t('reimbPending')}: ${rs.pending_count} (${fmtRmExport(rs.pending_cents)})`]
            : []),
          `${t('reimbApproved')}: ${rs.approved_count} (${fmtRmExport(rs.approved_cents)})`,
          `${t('reimbPaid')}: ${rs.paid_count} (${fmtRmExport(rs.paid_cents)})`,
        ];
        extraSections.push({ heading: t('myReimbursements'), items });
      }
      if (r.organised_events) {
        const oe = r.organised_events;
        const items = [
          `${t('organisedCount')}: ${oe.events_count}`,
          `${t('organisedAttendance')}: ${oe.total_attended}/${oe.total_going} (${oe.attendance_rate_pct}%)`,
          ...(oe.settlements
            ? [
                `${t('organisedBills')}: ${oe.settlements.count} · ${t('billsCollected')}: ${fmtRmExport(oe.settlements.collected_cents)} · ${t('billsOutstanding')}: ${fmtRmExport(oe.settlements.outstanding_cents)}`,
              ]
            : []),
        ];
        extraSections.push({ heading: t('organisedEvents'), items });
      }

      await exportReportPdf({
        fileBase,
        title: t('pdfTitle'),
        meta: [
          `${t('user')}: ${r.user.name} (${r.user.id})`,
          `${t('period')}: ${r.period_start} to ${inclusivePeriodEnd}`,
          `${t('range')}: ${rangeMap[r.range] ?? r.range}`,
          `${t('type')}: ${typeMap[r.type] ?? r.type}`,
        ],
        summaryHeading: t('summary'),
        summaryRows: rows,
        sections: [
          { heading: t('byKind'), items: kinds },
          { heading: t('events'), items: events },
          ...extraSections,
          { heading: t('badges'), items: badges },
        ],
      });
    } catch (e) {
      setExportErr(e instanceof Error ? e.message : String(e));
    } finally {
      setExportingPdf(false);
    }
  };

  const emptyEventsText =
    range === 'monthly'
      ? t('noEvents', { y, m })
      : t('noEventsInPeriod', { period: `${r?.period_start ?? ''} – ${inclusivePeriodEnd}` });

  return (
    <Page width="wide">
      <div className="stack">
        <PageHeader
          title={t('title', { month: periodLabel() })}
          subtitle={t('desc')}
          back="/me"
          actions={
            <>
              <Button
                variant="secondary"
                size="sm"
                onClick={exportCsv}
                disabled={!r}
                icon={<FileText size={16} />}
              >
                {t('exportCsv')}
              </Button>
              <Button
                variant="secondary"
                size="sm"
                onClick={exportPdf}
                disabled={!r || exportingPdf}
                loading={exportingPdf}
                icon={<Download size={16} />}
              >
                {t('exportPdf')}
              </Button>
            </>
          }
        />

        <Card className={`report-filters report-filters-self${drives ? '' : ' cols-2-mobile'}`}>
          <Field label={t('monthLabel')}>
            <input
              className="input"
              type="month"
              value={month}
              onChange={(e) => {
                if (e.target.value) setMonth(e.target.value);
              }}
            />
          </Field>
          <Field label={t('rangeLabel')}>
            <select
              className="input"
              value={range}
              onChange={(e) => setRange(e.target.value as 'monthly' | 'quarterly' | 'yearly')}
            >
              <option value="monthly">{t('rangeMonthly')}</option>
              <option value="quarterly">{t('rangeQuarterly')}</option>
              <option value="yearly">{t('rangeYearly')}</option>
            </select>
          </Field>
          {drives && (
            <Field label={t('typeLabel')}>
              <select
                className="input"
                value={type}
                onChange={(e) => setType(e.target.value as 'all' | 'driver' | 'passenger')}
              >
                <option value="all">{t('typeAll')}</option>
                <option value="driver">{t('typeDriver')}</option>
                <option value="passenger">{t('typePassenger')}</option>
              </select>
            </Field>
          )}
        </Card>

        <ErrorBox error={error || exportErr} />

        {!r ? (
          <Spinner />
        ) : (
          <>
            {/* Stat tiles using standard Stat component */}
            {(() => {
              const statCount =
                (r.events_attended != null ? 1 : 0) +
                (drives && r.rides_driven != null ? 1 : 0) +
                (r.rides_received != null ? 1 : 0) +
                (r.costs_paid_cents != null ? 1 : 0) +
                (drives && type !== 'passenger' && r.distance_km != null ? 1 : 0) +
                (drives && type !== 'passenger' && r.reimbursement_cents != null ? 1 : 0);
              return (
                <div className={`report-stats${statCount === 6 || statCount === 3 ? ' cols-3' : ''}`}>
                  {r.events_attended != null && (
                    <Stat
                      label={t('attendance')}
                      value={r.events_attended}
                      icon={<Calendar size={20} />}
                    />
                  )}
                  {drives && r.rides_driven != null && (
                    <Stat
                      label={t('driven')}
                      value={r.rides_driven}
                      icon={<CarFront size={20} />}
                    />
                  )}
                  {r.rides_received != null && (
                    <Stat
                      label={t('received')}
                      value={r.rides_received}
                      icon={<CarFront size={20} />}
                    />
                  )}
                  {r.costs_paid_cents != null && (
                    <Stat
                      label={t('costs')}
                      value={<span className="nowrap">{rm(r.costs_paid_cents)}</span>}
                      icon={<DollarSign size={20} />}
                    />
                  )}
                  {drives && type !== 'passenger' && r.distance_km != null && (
                    <Stat
                      label={t('distance')}
                      value={<span className="nowrap">{r.distance_km} km</span>}
                      icon={<Map size={20} />}
                    />
                  )}
                  {drives && type !== 'passenger' && r.reimbursement_cents != null && (
                    <Stat
                      label={t('reimbursement')}
                      value={<span className="nowrap">{rm(r.reimbursement_cents)}</span>}
                      icon={<DollarSign size={20} />}
                    />
                  )}
                </div>
              );
            })()}

            <div className="split">
              {/* Left Column: Events attended */}
              <Card className="stack-sm">
                <h2>{t('events')}</h2>
                {r.by_kind.length > 0 ? (
                  <p className="bold">
                    {r.by_kind
                      .map((k) => `${ft(EVENT_KIND[k.kind]?.label ?? k.kind)} × ${k.count}`)
                      .join(' · ')}
                  </p>
                ) : (
                  <p className="muted">{emptyEventsText}</p>
                )}

                {r.events.length > 0 ? (
                  <div className="list">
                    {r.events.map((e) => (
                      <Link key={e.id} to={`/events/${e.id}`} className="item">
                        <span className="grow min-w-0 ellipsis">{e.title}</span>
                        <span className="muted small">{fmtDateTime(e.starts_at, ft)}</span>
                      </Link>
                    ))}
                  </div>
                ) : (
                  <EmptyState title={emptyEventsText} />
                )}
                <p className="muted tiny">{t('note')}</p>
              </Card>

              {/* Right Column: Badges as cards */}
              <Card className="stack">
                <div>
                  <h2 className="row nowrap gap-sm items-center">
                    <Award size={20} aria-hidden="true" />
                    <span>{t('badges')}</span>
                  </h2>
                  <p className="muted small">{t('badgesDesc')}</p>
                </div>

                {r.badges.length === 0 ? (
                  <p className="muted">{t('noBadges')}</p>
                ) : (() => {
                  const hasCategories = r.badges.some((b) => !!b.category);
                  const visibleBadges = !hasCategories || badgeFilter === 'all'
                    ? r.badges
                    : badgeFilter === 'mine'
                    ? r.badges.filter((b) => !b.category || b.role_match !== false)
                    : r.badges.filter((b) => b.category === badgeFilter);

                  const renderBadgeCard = (b: Badge) => {
                    const percent = b.target > 0 ? Math.min(100, Math.round((b.progress / b.target) * 100)) : 100;
                    const isAnim = isAnimatedBadgeIcon(b.icon);
                    const catClass = b.category ? `badge-cat-${b.category}` : '';
                    const tier = resolveBadgeTier(b);
                    const hasTiers = Array.isArray(b.tiers) && b.tiers.length > 0;
                    const progressText =
                      tier === 'platinum' ? (
                        <span className="row nowrap gap-xs text-primary bold">
                          <CheckCircle2 size={14} aria-hidden="true" />
                          <span>{t('badgeMaxTier')}</span>
                        </span>
                      ) : b.next_tier && b.next_target != null ? (
                        t('badgeNextTier', {
                          tier: t(`badgeTier_${b.next_tier}`),
                          progress: b.progress,
                          target: b.next_target,
                        })
                      ) : b.earned ? (
                        <span className="row nowrap gap-xs text-primary bold">
                          <CheckCircle2 size={14} aria-hidden="true" />
                          <span>{t('earned')}</span>
                        </span>
                      ) : (
                        `${Math.min(b.progress, b.target)}/${b.target}`
                      );
                    return (
                      <div key={b.key} className="card flat tight badge-card">
                        <div
                          className={`badge-char ${b.earned ? 'earned' : ''} ${isAnim ? 'badge-animated' : ''} ${catClass} badge-tier-${tier}`.trim()}
                          aria-hidden="true"
                        >
                          <BadgeIcon badge={b} fallbackChar={badgeChar(b)} />
                          {tier !== 'none' && (
                            <span className={`badge-tier-crest badge-tier-${tier}`}>
                              {BADGE_TIER_ROMAN[tier]}
                            </span>
                          )}
                        </div>
                        <div className="grow min-w-0 stack-xs">
                          <div className="row between gap-xs items-center">
                            <div className="row gap-xs items-center min-w-0 flex-wrap">
                              <strong>{badgeName(b)}</strong>
                              {tier !== 'none' && (
                                <span className={`badge-tier-pill badge-tier-${tier}`}>
                                  {t(`badgeTier_${tier}`)}
                                </span>
                              )}
                            </div>
                            <span className="small muted nowrap">{progressText}</span>
                          </div>
                          <div className={`bar badge-tier-${tier}`}>
                            <div style={{ width: `${percent}%` }} />
                          </div>
                          {hasTiers && (
                            <div className="badge-tier-steps">
                              {b.tiers!.map((step) => {
                                const isNext = !step.reached && b.next_tier === step.tier;
                                return (
                                  <span
                                    key={step.tier}
                                    className={`badge-tier-step badge-tier-${step.tier}${step.reached ? ' reached' : ''}${isNext ? ' next' : ''}`}
                                  >
                                    {t(`badgeTier_${step.tier}`)} {step.target}
                                  </span>
                                );
                              })}
                            </div>
                          )}
                          <div className="muted tiny">{badgeRule(b)}</div>
                        </div>
                      </div>
                    );
                  };

                  const categories: BadgeCategory[] = ['member', 'driver', 'traffic_lead'];
                  const filterOptions: { value: 'mine' | 'all' | BadgeCategory; label: string }[] = [
                    { value: 'mine', label: t('badgeFilterMine') },
                    { value: 'all', label: t('badgeFilterAll') },
                    { value: 'member', label: t('badgeRole_member') },
                    { value: 'driver', label: t('badgeRole_driver') },
                    { value: 'traffic_lead', label: t('badgeRole_traffic_lead') },
                  ];

                  return (
                    <div className="stack-sm">
                      {hasCategories && (
                        <div className="badge-filter-row" role="tablist" aria-label={t('badges')}>
                          {filterOptions.map((opt) => (
                            <button
                              key={opt.value}
                              type="button"
                              role="tab"
                              aria-selected={badgeFilter === opt.value}
                              className={`chip${badgeFilter === opt.value ? ' on' : ''}`}
                              onClick={() => setBadgeFilter(opt.value)}
                            >
                              {opt.label}
                            </button>
                          ))}
                        </div>
                      )}
                      {hasCategories ? (
                        categories.map((cat) => {
                          const group = visibleBadges.filter((b) => b.category === cat);
                          if (group.length === 0) return null;
                          const earnedCount = group.filter((b) => b.earned).length;
                          return (
                            <div key={cat} className="stack-xs">
                              <div className="badge-group-header">
                                <span className={`badge-role-dot badge-cat-${cat}`} aria-hidden="true" />
                                <span className="bold small">{t(`badgeRole_${cat}`)}</span>
                                <span className="muted tiny">{earnedCount}/{group.length}</span>
                              </div>
                              <div className="stack-sm">
                                {group.map(renderBadgeCard)}
                              </div>
                            </div>
                          );
                        })
                      ) : (
                        <div className="stack-sm">
                          {visibleBadges.map(renderBadgeCard)}
                        </div>
                      )}
                    </div>
                  );
                })()}
              </Card>
            </div>

            {(r.reimbursements_summary || r.organised_events) && (
              <div className={r.reimbursements_summary && r.organised_events ? 'report-cols-2' : 'stack'}>
                {r.reimbursements_summary && (
                  <Card className="stack-sm">
                    <div className="row between gap-sm items-center flex-wrap">
                      <h2 className="row nowrap gap-sm items-center">
                        <Receipt size={18} aria-hidden="true" />
                        <span>{t('myReimbursements')}</span>
                      </h2>
                      <Link className="btn ghost sm" to="/reimbursements">
                        {t('openReimbursements')}
                      </Link>
                    </div>
                    <div className="list">
                      <div className="item row between nowrap gap-sm">
                        <span className="muted">{t('reimbTotalClaims')}</span>
                        <span className="bold">{r.reimbursements_summary.claim_count}</span>
                      </div>
                      {r.reimbursements_summary.pending_count != null &&
                        r.reimbursements_summary.pending_cents != null && (
                          <div className="item row between nowrap gap-sm">
                            <span className="muted">{t('reimbPending')}</span>
                            <span className="bold nowrap">
                              {r.reimbursements_summary.pending_count} ·{' '}
                              {rm(r.reimbursements_summary.pending_cents)}
                            </span>
                          </div>
                        )}
                      <div className="item row between nowrap gap-sm">
                        <span className="muted">{t('reimbApproved')}</span>
                        <span className="bold nowrap">
                          {r.reimbursements_summary.approved_count} ·{' '}
                          {rm(r.reimbursements_summary.approved_cents)}
                        </span>
                      </div>
                      <div className="item row between nowrap gap-sm">
                        <span className="muted">{t('reimbPaid')}</span>
                        <span className="bold nowrap">
                          {r.reimbursements_summary.paid_count} ·{' '}
                          {rm(r.reimbursements_summary.paid_cents)}
                        </span>
                      </div>
                    </div>
                    {((r.reimbursements_summary.returned_count ?? 0) > 0 ||
                      (r.reimbursements_summary.rejected_count ?? 0) > 0) && (
                      <div className="row gap-xs flex-wrap">
                        {(r.reimbursements_summary.returned_count ?? 0) > 0 && (
                          <Pill tone="warning">
                            {t('reimbReturnedCount', {
                              count: r.reimbursements_summary.returned_count,
                            })}
                          </Pill>
                        )}
                        {(r.reimbursements_summary.rejected_count ?? 0) > 0 && (
                          <Pill tone="danger">
                            {t('reimbRejectedCount', {
                              count: r.reimbursements_summary.rejected_count,
                            })}
                          </Pill>
                        )}
                      </div>
                    )}
                  </Card>
                )}

                {r.organised_events && (
                  <Card className="stack-sm">
                    <div className="row between gap-sm items-center flex-wrap">
                      <h2 className="row nowrap gap-sm items-center">
                        <Calendar size={18} aria-hidden="true" />
                        <span>{t('organisedEvents')}</span>
                      </h2>
                      <div className="row gap-xs nowrap">
                        <Link className="btn ghost sm" to="/events">
                          {t('openEvents')}
                        </Link>
                        {r.organised_events.settlements && (
                          <Link className="btn ghost sm" to="/settlements">
                            {t('openSettlements')}
                          </Link>
                        )}
                      </div>
                    </div>
                    <div className="list">
                      <div className="item row between nowrap gap-sm">
                        <span className="muted">{t('organisedCount')}</span>
                        <span className="bold">{r.organised_events.events_count}</span>
                      </div>
                      <div className="item row between nowrap gap-sm">
                        <span className="muted">{t('organisedAttendance')}</span>
                        <span className="bold nowrap">
                          {r.organised_events.total_attended} / {r.organised_events.total_going} (
                          {r.organised_events.attendance_rate_pct}%)
                        </span>
                      </div>
                      {r.organised_events.settlements && (
                        <>
                          <div className="item row between nowrap gap-sm">
                            <span className="muted">{t('organisedBills')}</span>
                            <span className="bold">{r.organised_events.settlements.count}</span>
                          </div>
                          <div className="item row between nowrap gap-sm">
                            <span className="muted">{t('billsCollected')}</span>
                            <span className="bold nowrap">
                              {rm(r.organised_events.settlements.collected_cents)}
                            </span>
                          </div>
                          <div className="item row between nowrap gap-sm">
                            <span className="muted">{t('billsOutstanding')}</span>
                            <span className="bold nowrap">
                              {rm(r.organised_events.settlements.outstanding_cents)}
                            </span>
                          </div>
                        </>
                      )}
                    </div>
                  </Card>
                )}
              </div>
            )}
          </>
        )}
      </div>
    </Page>
  );
}
