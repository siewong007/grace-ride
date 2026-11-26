import { useMemo, useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import {
  Award,
  Calendar,
  CarFront,
  CheckCircle2,
  DollarSign,
  Download,
  FileText,
  Map,
  Pencil,
  Receipt,
} from 'lucide-react';
import { put, useApi } from '../../api';
import { useAuth } from '../../auth';
import {
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Field,
  Modal,
  Notice,
  Page,
  PageHeader,
  Pill,
  Stat,
  Tabs,
  useAction,
} from '../../components/ui';
import { EVENT_KIND, fmtDateTime, rm, useFormatT } from '../../format';
import { useTranslation } from 'react-i18next';
import { ADMIN_HOME } from '../../staffNav';
import {
  ACHIEVEMENT_ICONS,
  ANIMATED_BADGE_ICONS,
  BADGE_TIER_ROMAN,
  type BadgeCategory,
  BadgeIcon,
  isAnimatedBadgeIcon,
  resolveBadgeTier,
} from '../../badgeIcon';
import {
  downloadCsv,
  escapeCsv,
  exportReportPdf,
  fmtRmExport,
  formatInclusivePeriodEnd,
} from '../../reportExport';
import type { Badge, MonthReport, ReportCapabilities } from '../../types/domain';
import DemandReportPanel from './reports/DemandReportPanel';
import EventsReportPanel from './reports/EventsReportPanel';
import FinanceReportPanel from './reports/FinanceReportPanel';
import OverviewReportPanel from './reports/OverviewReportPanel';
import { type PeriodFilterState, StatCountAmount } from './reports/PeriodControls';

interface Member {
  id: number;
  name: string;
  email?: string;
  roles?: string[];
}

type ReportTab = 'member' | 'overview' | 'events' | 'demand' | 'finance';

/** Locale keys for the range and type a report was made for, so the PDF names them in the reader's language. */
const RANGE_LABEL: Record<string, string> = {
  monthly: 'reports.rangeMonthly',
  quarterly: 'reports.rangeQuarterly',
  yearly: 'reports.rangeYearly',
};
const TYPE_LABEL: Record<string, string> = {
  all: 'reports.typeAll',
  driver: 'reports.typeDriver',
  passenger: 'reports.typePassenger',
};

function monthLastDay(ym: string): string {
  const [y, m] = ym.split('-').map(Number);
  if (!y || !m) return `${ym}-28`;
  const last = new Date(y, m, 0).getDate();
  return `${y}-${String(m).padStart(2, '0')}-${String(last).padStart(2, '0')}`;
}

export default function AdminReports() {
  const { t, i18n } = useTranslation('admin');
  const ft = useFormatT();
  const isZh = i18n.language?.startsWith('zh');
  const badgeChar = (b: Badge) => (isZh ? b.char_zh ?? b.char : b.char_en ?? b.char);
  const badgeName = (b: Badge) => (isZh ? b.name_zh ?? b.name : b.name_en ?? b.name);
  const badgeRule = (b: Badge) => (isZh ? b.rule_zh ?? b.rule : b.rule_en ?? b.rule);

  // The route only renders this page for members who may view member reports.
  const { me, can } = useAuth();
  const [searchParams, setSearchParams] = useSearchParams();
  const [selectedId, setSelectedId] = useState<number | null>(me?.id ?? null);
  const defaultMonth = `${new Date().getFullYear()}-${String(new Date().getMonth() + 1).padStart(2, '0')}`;
  const rawMonth = searchParams.get('month');
  const month = rawMonth && /^\d{4}-\d{2}$/.test(rawMonth) ? rawMonth : defaultMonth;
  const rawRange = searchParams.get('range');
  const range: 'monthly' | 'quarterly' | 'yearly' =
    rawRange === 'quarterly' || rawRange === 'yearly' ? rawRange : 'monthly';
  const useCustomPeriod = searchParams.get('custom') === '1';
  const fromDate = searchParams.get('from') || `${month}-01`;
  const toDate = searchParams.get('to') || monthLastDay(month);
  const groupId = searchParams.get('group_id') || '';
  const [type, setType] = useState<'all' | 'driver' | 'passenger'>('all');
  const [badgeFilter, setBadgeFilter] = useState<'all' | BadgeCategory>('all');
  const [exportingPdf, setExportingPdf] = useState(false);
  const [exportErr, setExportErr] = useState<string | null>(null);
  const [y, m] = month.split('-').map(Number);

  const [editingBadge, setEditingBadge] = useState<{
    badge: Badge;
    userId: number;
    userName: string;
  } | null>(null);
  const [selectedIcon, setSelectedIcon] = useState('');
  const [customIcon, setCustomIcon] = useState('');
  const { busy: savingIcon, error: iconErr, run: runIconAction } = useAction();

  const updateParams = (mutate: (next: URLSearchParams) => void) => {
    setSearchParams(
      (prev) => {
        const next = new URLSearchParams(prev);
        mutate(next);
        return next;
      },
      { replace: true },
    );
  };

  const { data: capsData } = useApi<ReportCapabilities>('/admin/reports/capabilities');
  const caps: ReportCapabilities = useMemo(() => {
    if (capsData && typeof capsData.overview === 'boolean') return capsData;
    const canDemand = can('admin_rides') || can('admin_plan');
    const canEvents = can('admin_events') || can('events', 'create');
    const canReimbReview = can('admin_reimb_review');
    const canReimbPay = can('admin_reimb_pay');
    const canBills = can('admin_bills');
    const canFinance = canReimbReview || canReimbPay || canBills;
    const canMembership = can('admin_members', 'update') || can('admin_invites');
    return {
      overview: canDemand || canEvents || canFinance || canMembership,
      member_reports_global:
        can('admin_dashboard') ||
        can('admin_reimb_pay') ||
        can('admin_bills') ||
        can('admin_events'),
      member_badge_icon_edit: can('admin_members', 'update'),
      demand: canDemand,
      demand_scope: !canDemand ? 'none' : can('admin_dashboard') ? 'global' : 'own_events',
      events: canEvents,
      events_scope: !canEvents ? 'none' : can('admin_events') ? 'global' : 'own_events',
      finance: canFinance,
      finance_sections: {
        reimb_review: canReimbReview,
        reimb_payouts: canReimbPay,
        bills: canBills,
      },
      membership: canMembership,
    };
  }, [capsData, can]);

  const tabItems = useMemo<{ value: ReportTab; label: string }[]>(
    () => [
      { value: 'member', label: t('reports.memberTab') },
      ...(caps.overview ? [{ value: 'overview' as const, label: t('reports.overviewTab') }] : []),
      ...(caps.events ? [{ value: 'events' as const, label: t('reports.eventsTab') }] : []),
      ...(caps.demand ? [{ value: 'demand' as const, label: t('reports.demandTab') }] : []),
      ...(caps.finance ? [{ value: 'finance' as const, label: t('reports.financeTab') }] : []),
    ],
    [caps, t],
  );

  const defaultTab: ReportTab =
    !caps.member_reports_global && caps.overview ? 'overview' : 'member';
  const rawTab = searchParams.get('tab');
  const reportTab: ReportTab = tabItems.some((item) => item.value === rawTab)
    ? (rawTab as ReportTab)
    : defaultTab;

  const setReportTab = (nextTab: ReportTab) => {
    updateParams((next) => {
      if (nextTab === defaultTab) {
        next.delete('tab');
      } else {
        next.set('tab', nextTab);
      }
    });
  };

  const setGroupId = (nextGroupId: string) => {
    updateParams((next) => {
      if (nextGroupId) next.set('group_id', nextGroupId);
      else next.delete('group_id');
    });
  };

  const handleOpenMember = (userId: number) => {
    setSelectedId(userId);
    setReportTab('member');
  };

  const handleOpenEventsGroup = (gid: number) => {
    updateParams((next) => {
      next.set('tab', 'events');
      next.set('group_id', String(gid));
    });
  };

  const periodState: PeriodFilterState = {
    month,
    onMonthChange: (nextMonth) => {
      updateParams((next) => {
        if (!nextMonth || nextMonth === defaultMonth) next.delete('month');
        else next.set('month', nextMonth);
        if (next.get('custom') !== '1') {
          next.delete('from');
          next.delete('to');
        }
      });
    },
    range,
    onRangeChange: (nextRange) => {
      updateParams((next) => {
        if (nextRange === 'monthly') next.delete('range');
        else next.set('range', nextRange);
      });
    },
    useCustom: useCustomPeriod,
    onToggleCustom: (custom) => {
      updateParams((next) => {
        if (custom) {
          next.set('custom', '1');
          if (!next.get('from')) next.set('from', `${month}-01`);
          if (!next.get('to')) next.set('to', monthLastDay(month));
        } else {
          next.delete('custom');
          next.delete('from');
          next.delete('to');
        }
      });
    },
    fromDate,
    toDate,
    onFromChange: (d) => {
      updateParams((next) => {
        if (d) next.set('from', d);
        else next.delete('from');
      });
    },
    onToChange: (d) => {
      updateParams((next) => {
        if (d) next.set('to', d);
        else next.delete('to');
      });
    },
  };

  const { data: membersData } = useApi<Member[] | { items: Member[] }>(
    caps.member_reports_global && reportTab === 'member' ? '/admin/users?q=' : null,
  );
  const members = Array.isArray(membersData) ? membersData : membersData?.items ?? [];
  // The picker lists everyone, so a search box narrows it. The chosen member always stays listed.
  const [memberQuery, setMemberQuery] = useState('');
  const needle = memberQuery.trim().toLowerCase();
  const shownMembers = needle
    ? members.filter(
        (mem) =>
          mem.id === selectedId ||
          mem.name.toLowerCase().includes(needle) ||
          Boolean(mem.email && mem.email.toLowerCase().includes(needle)),
      )
    : members;

  const effectiveTargetId = caps.member_reports_global ? selectedId : null;
  const params = new URLSearchParams({
    month,
    range,
    type,
    ...(effectiveTargetId ? { user_id: String(effectiveTargetId) } : {}),
  });
  const { data: r, error, loading, reload: reloadReport } = useApi<MonthReport>(
    reportTab === 'member' ? `/reports/monthly?${params.toString()}` : null,
  );

  const saveBadgeIcon = () =>
    runIconAction(async () => {
      if (!editingBadge) return;
      await put(`/admin/users/${editingBadge.userId}/badges/${editingBadge.badge.key}/icon`, {
        icon: selectedIcon,
      });
      await reloadReport();
      setEditingBadge(null);
    }, t('reports.badgeIconUpdated'));

  const resetBadgeIcon = () =>
    runIconAction(async () => {
      if (!editingBadge) return;
      await put(`/admin/users/${editingBadge.userId}/badges/${editingBadge.badge.key}/icon`, {
        icon: '',
      });
      await reloadReport();
      setEditingBadge(null);
    }, t('reports.badgeIconUpdated'));

  const fileBase = `grace-ride-admin-report-${range}-${month}-${effectiveTargetId ?? 'self'}`;
  const showBadges = r?.visible_sections?.badges !== false;
  const inclusivePeriodEnd = r
    ? formatInclusivePeriodEnd(r.period_end, r.period_end_inclusive)
    : '';
  const emptyEventsText =
    range === 'monthly'
      ? t('reports.noEvents', { y, m })
      : t('reports.noEventsPeriod', { start: r?.period_start ?? '', end: inclusivePeriodEnd });

  const exportCsv = () => {
    if (!r) return;
    const lines: string[] = [];
    lines.push('Grace Ride — Admin Report Export');
    lines.push(`User,${escapeCsv(`${r.user.name} (${r.user.id})`)}`);
    lines.push(`Period,${escapeCsv(`${r.period_start} to ${inclusivePeriodEnd}`)}`);
    lines.push(`Range,${escapeCsv(r.range)}`);
    lines.push(`Type,${escapeCsv(r.type)}`);
    lines.push('');
    lines.push('Summary,Value');
    if (r.events_attended != null) lines.push(`Events attended,${r.events_attended}`);
    if (r.rides_driven != null) lines.push(`Rides driven,${r.rides_driven}`);
    if (r.rides_received != null) lines.push(`Rides received,${r.rides_received}`);
    if (r.distance_km != null) lines.push(`Distance (km),${r.distance_km}`);
    if (r.reimbursement_cents != null) {
      lines.push(`Reimbursement (MYR),${escapeCsv(fmtRmExport(r.reimbursement_cents))}`);
    }
    if (r.costs_paid_cents != null) {
      lines.push(`Costs paid (MYR),${escapeCsv(fmtRmExport(r.costs_paid_cents))}`);
    }
    if (r.reimbursements_summary) {
      const rs = r.reimbursements_summary;
      lines.push('');
      lines.push('Reimbursements Summary,Count,Amount (MYR)');
      lines.push(`Claims in scope,${rs.claim_count},`);
      if (rs.pending_count != null && rs.pending_cents != null) {
        lines.push(`Pending,${rs.pending_count},${escapeCsv(fmtRmExport(rs.pending_cents))}`);
      }
      lines.push(`Approved,${rs.approved_count},${escapeCsv(fmtRmExport(rs.approved_cents))}`);
      lines.push(`Paid,${rs.paid_count},${escapeCsv(fmtRmExport(rs.paid_cents))}`);
      if (rs.returned_count != null) lines.push(`Returned,${rs.returned_count},`);
      if (rs.rejected_count != null) lines.push(`Rejected,${rs.rejected_count},`);
    }
    if (r.visible_sections?.events !== false) {
      lines.push('');
      lines.push('Event Kind,Count');
      for (const k of r.by_kind) lines.push(`${escapeCsv(k.kind)},${k.count}`);
      lines.push('');
      lines.push('Event ID,Title,Kind,Starts At');
      for (const e of r.events) {
        lines.push(`${e.id},${escapeCsv(e.title)},${escapeCsv(e.kind)},${escapeCsv(e.starts_at)}`);
      }
    }
    if (showBadges) {
      lines.push('');
      lines.push('Badge Key,Name,Tier,Rule,Progress,Target,Earned');
      for (const b of r.badges) {
        const tier = resolveBadgeTier(b);
        lines.push(
          `${escapeCsv(b.key)},${escapeCsv(badgeName(b))},${escapeCsv(tier)},${escapeCsv(badgeRule(b))},${b.progress},${b.target},${b.earned}`,
        );
      }
    }
    downloadCsv(fileBase, lines);
  };

  const exportPdf = async () => {
    if (!r || exportingPdf) return;
    setExportErr(null);
    setExportingPdf(true);
    try {
      const showEvents = r.visible_sections?.events !== false;
      const kinds = showEvents
        ? r.by_kind.map((k) => `${ft(EVENT_KIND[k.kind]?.label ?? k.kind)} × ${k.count}`)
        : [];
      const events = showEvents
        ? r.events.map((e) => `${e.title} — ${fmtDateTime(e.starts_at, ft)}`)
        : [];
      const badges = showBadges
        ? r.badges.map((b) => {
            const tier = resolveBadgeTier(b);
            const tierText =
              tier !== 'none'
                ? `[${t(`reports.badgeTier_${tier}`)}]`
                : b.earned
                ? t('reports.earned')
                : '';
            return `${badgeName(b)} ${tierText} — ${badgeRule(b)} (${b.progress}/${b.target})`.trim();
          })
        : [];
      const rows: [string, string][] = [];
      if (r.events_attended != null) rows.push([t('reports.events_attended'), String(r.events_attended)]);
      if (r.rides_driven != null) rows.push([t('reports.rides_driven'), String(r.rides_driven)]);
      if (r.rides_received != null) rows.push([t('reports.rides_received'), String(r.rides_received)]);
      if (r.distance_km != null) rows.push([t('reports.distance_km'), `${r.distance_km} km`]);
      if (r.reimbursement_cents != null) {
        rows.push([t('reports.reimbursement'), fmtRmExport(r.reimbursement_cents)]);
      }
      if (r.costs_paid_cents != null) {
        rows.push([t('reports.costs'), fmtRmExport(r.costs_paid_cents)]);
      }

      const extraSections = [];
      if (r.reimbursements_summary) {
        const rs = r.reimbursements_summary;
        extraSections.push({
          heading: t('reports.memberReimbTitle'),
          items: [
            `${t('reports.reimbClaimsInScope')}: ${rs.claim_count}`,
            ...(rs.pending_count != null && rs.pending_cents != null
              ? [`${t('reports.reimbPendingReview')}: ${rs.pending_count} (${fmtRmExport(rs.pending_cents)})`]
              : []),
            `${t('reports.reimbApproved')}: ${rs.approved_count} (${fmtRmExport(rs.approved_cents)})`,
            `${t('reports.payoutsPaid')}: ${rs.paid_count} (${fmtRmExport(rs.paid_cents)})`,
          ],
        });
      }

      await exportReportPdf({
        fileBase,
        title: t('reports.pdfTitle'),
        meta: [
          `${t('reports.user')}: ${r.user.name} (${r.user.id})`,
          `${t('reports.period')}: ${r.period_start} to ${inclusivePeriodEnd}`,
          `${t('reports.range')}: ${t(RANGE_LABEL[r.range] ?? 'reports.rangeMonthly')}`,
          `${t('reports.type')}: ${t(TYPE_LABEL[r.type] ?? 'reports.typeAll')}`,
        ],
        summaryHeading: t('reports.summary'),
        summaryRows: rows,
        sections: [
          ...(showEvents ? [{ heading: t('reports.byKind'), items: kinds }] : []),
          ...(showEvents ? [{ heading: t('reports.events'), items: events }] : []),
          ...extraSections,
          ...(showBadges ? [{ heading: t('reports.badges'), items: badges }] : []),
        ],
      });
    } catch (e) {
      setExportErr(e instanceof Error ? e.message : String(e));
    } finally {
      setExportingPdf(false);
    }
  };

  const hasPartialRedaction = Boolean(
    caps.member_reports_global &&
      r?.visible_sections &&
      (!r.visible_sections.events ||
        !r.visible_sections.costs_paid ||
        !r.visible_sections.driving ||
        !r.visible_sections.passenger_rides ||
        !r.visible_sections.badges),
  );

  const tabMeta: Record<ReportTab, { title: string; subtitle: string }> = {
    member: { title: t('reports.title'), subtitle: t('reports.desc') },
    overview: { title: t('reports.overviewTab'), subtitle: t('reports.overviewDesc') },
    events: { title: t('reports.eventsTab'), subtitle: t('reports.eventsDesc') },
    demand: { title: t('reports.demandTab'), subtitle: t('reports.demandDesc') },
    finance: { title: t('reports.financeTab'), subtitle: t('reports.financeDesc') },
  };
  const { title: pageTitle, subtitle: pageSubtitle } = tabMeta[reportTab];

  return (
    <Page width="wide">
      <div className="stack">
        <PageHeader
          title={pageTitle}
          subtitle={pageSubtitle}
          back={ADMIN_HOME}
          actions={
            reportTab === 'member' ? (
              <>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={exportCsv}
                  disabled={!r}
                  icon={<FileText size={16} />}
                >
                  {t('reports.exportCsv')}
                </Button>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={exportPdf}
                  disabled={!r || exportingPdf}
                  loading={exportingPdf}
                  icon={<Download size={16} />}
                >
                  {t('reports.exportPdf')}
                </Button>
              </>
            ) : undefined
          }
        />

        <Tabs
          value={reportTab}
          onChange={(v) => setReportTab(v as ReportTab)}
          items={tabItems}
          ariaLabel={t('reports.title')}
        />

        {reportTab === 'overview' && caps.overview ? (
          <OverviewReportPanel
            period={periodState}
            onOpenMember={caps.member_reports_global ? handleOpenMember : undefined}
            onOpenEventsGroup={caps.events ? handleOpenEventsGroup : undefined}
          />
        ) : reportTab === 'events' && caps.events ? (
          <EventsReportPanel
            period={periodState}
            groupId={groupId}
            onGroupChange={setGroupId}
          />
        ) : reportTab === 'demand' && caps.demand ? (
          <DemandReportPanel
            period={periodState}
            groupId={groupId}
            onGroupChange={setGroupId}
          />
        ) : reportTab === 'finance' && caps.finance ? (
          <FinanceReportPanel period={periodState} />
        ) : (
          <>
            <ErrorBox error={error || exportErr} />

            {!caps.member_reports_global && (
              <Notice tone="info">{t('reports.ownReportOnlyNotice')}</Notice>
            )}

            <Card className="report-filters">
              <Field label={t('reports.monthLabel')}>
                <input
                  className="input"
                  type="month"
                  value={month}
                  onChange={(e) => periodState.onMonthChange(e.target.value)}
                />
              </Field>
              <Field label={t('reports.range')}>
                <select
                  className="input"
                  value={range}
                  onChange={(e) => periodState.onRangeChange(e.target.value as typeof range)}
                >
                  <option value="monthly">{t('reports.rangeMonthly')}</option>
                  <option value="quarterly">{t('reports.rangeQuarterly')}</option>
                  <option value="yearly">{t('reports.rangeYearly')}</option>
                </select>
              </Field>
              <Field label={t('reports.type')}>
                <select
                  className="input"
                  value={type}
                  onChange={(e) => setType(e.target.value as typeof type)}
                >
                  <option value="all">{t('reports.typeAll')}</option>
                  <option value="driver">{t('reports.typeDriver')}</option>
                  <option value="passenger">{t('reports.typePassenger')}</option>
                </select>
              </Field>
              {caps.member_reports_global && (
                <div className="report-member">
                  <Field label={t('reports.findMember')}>
                    <input
                      className="input"
                      type="search"
                      value={memberQuery}
                      placeholder={
                        can('admin_members')
                          ? t('reports.findMemberHint')
                          : t('reports.findMemberNameOnly')
                      }
                      onChange={(e) => setMemberQuery(e.target.value)}
                    />
                  </Field>
                  <Field label={t('reports.selectMember')}>
                    <select
                      className="input"
                      value={selectedId ?? ''}
                      onChange={(e) =>
                        setSelectedId(e.target.value ? Number(e.target.value) : null)
                      }
                    >
                      <option value="">{t('reports.self')}</option>
                      {shownMembers.map((mem) => (
                        <option key={mem.id} value={mem.id}>
                          {mem.email ? `${mem.name} (${mem.email})` : mem.name}
                        </option>
                      ))}
                    </select>
                  </Field>
                </div>
              )}
            </Card>

            {loading ? (
              <CardSkeleton count={3} />
            ) : !r ? (
              <EmptyState title={t('reports.none')} />
            ) : (
              <>
                {hasPartialRedaction && (
                  <p className="muted small">{t('reports.partialVisibilityNote')}</p>
                )}

                {(() => {
                  const visibleStatCount =
                    (r.events_attended != null ? 1 : 0) +
                    (r.rides_driven != null ? 1 : 0) +
                    (r.rides_received != null ? 1 : 0) +
                    (r.costs_paid_cents != null ? 1 : 0) +
                    (r.distance_km != null ? 1 : 0) +
                    (r.reimbursement_cents != null ? 1 : 0);
                  return (
                    <div className={`report-stats${visibleStatCount === 6 || visibleStatCount === 3 ? ' cols-3' : ''}`}>
                      {r.events_attended != null && (
                        <Stat
                          label={t('reports.attendance')}
                          value={r.events_attended}
                          icon={<Calendar size={20} />}
                        />
                      )}
                      {r.rides_driven != null && (
                        <Stat
                          label={t('reports.driven')}
                          value={r.rides_driven}
                          icon={<CarFront size={20} />}
                        />
                      )}
                      {r.rides_received != null && (
                        <Stat
                          label={t('reports.received')}
                          value={r.rides_received}
                          icon={<CarFront size={20} />}
                        />
                      )}
                      {r.costs_paid_cents != null && (
                        <Stat
                          label={t('reports.costs')}
                          value={<span className="nowrap">{rm(r.costs_paid_cents)}</span>}
                          icon={<DollarSign size={20} />}
                        />
                      )}
                      {r.distance_km != null && (
                        <Stat
                          label={t('reports.distance')}
                          value={<span className="nowrap">{r.distance_km} km</span>}
                          icon={<Map size={20} />}
                        />
                      )}
                      {r.reimbursement_cents != null && (
                        <Stat
                          label={t('reports.reimbursement')}
                          value={<span className="nowrap">{rm(r.reimbursement_cents)}</span>}
                          icon={<DollarSign size={20} />}
                        />
                      )}
                    </div>
                  );
                })()}

                {r.reimbursements_summary && (() => {
                  const rs = r.reimbursements_summary;
                  const hasPending = rs.pending_count != null && rs.pending_cents != null;
                  const hasReturned = rs.returned_count != null;
                  const reimbStatCount = 2 + (hasPending ? 1 : 0) + (hasReturned ? 1 : 0);
                  return (
                    <Card className="stack-sm">
                      <div className="row between gap-sm items-center flex-wrap">
                        <h2 className="row nowrap gap-sm items-center">
                          <Receipt size={18} aria-hidden="true" />
                          <span>{t('reports.memberReimbTitle')}</span>
                        </h2>
                        <Pill tone="neutral">
                          {t('reports.reimbClaimsInScope')}: {rs.claim_count}
                        </Pill>
                      </div>
                      <div className={`report-stats${reimbStatCount === 3 ? ' cols-3' : ''}`}>
                        {hasPending && (
                          <Stat
                            label={t('reports.reimbPendingReview')}
                            value={<StatCountAmount count={rs.pending_count!} cents={rs.pending_cents!} />}
                          />
                        )}
                        <Stat
                          label={t('reports.reimbApproved')}
                          value={<StatCountAmount count={rs.approved_count} cents={rs.approved_cents} />}
                        />
                        <Stat
                          label={t('reports.payoutsPaid')}
                          value={<StatCountAmount count={rs.paid_count} cents={rs.paid_cents} />}
                        />
                        {hasReturned && (
                          <Stat
                            label={t('reports.reimbReturned')}
                            value={rs.returned_count}
                          />
                        )}
                      </div>
                    </Card>
                  );
                })()}

                {(r.visible_sections?.events !== false || showBadges) && (
                  <div className={r.visible_sections?.events !== false && showBadges ? 'report-cols-2' : 'stack'}>
                    {r.visible_sections?.events !== false && (
                      <Card className="stack-sm">
                        <div>
                          <h2>{t('reports.events')}</h2>
                          {r.by_kind.length > 0 ? (
                            <p className="bold">
                              {r.by_kind
                                .map((k) => `${ft(EVENT_KIND[k.kind]?.label ?? k.kind)} × ${k.count}`)
                                .join(' · ')}
                            </p>
                          ) : (
                            <p className="muted">{emptyEventsText}</p>
                          )}
                        </div>
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
                        <p className="muted tiny">{t('reports.note')}</p>
                      </Card>
                    )}

                    {showBadges && (
                      <Card className="stack">
                        <div>
                          <h2 className="row nowrap gap-sm items-center">
                            <Award size={20} aria-hidden="true" />
                            <span>{t('reports.badges')}</span>
                          </h2>
                          <p className="muted small">{t('reports.badgesDesc')}</p>
                        </div>

                        {r.badges.length === 0 ? (
                          <p className="muted">{t('reports.noBadges')}</p>
                        ) : (
                          (() => {
                            const hasCategories = r.badges.some((b) => !!b.category);
                            const visibleBadges =
                              !hasCategories || badgeFilter === 'all'
                                ? r.badges
                                : r.badges.filter((b) => b.category === badgeFilter);

                            const renderBadgeCard = (b: Badge) => {
                              const percent =
                                b.target > 0
                                  ? Math.min(100, Math.round((b.progress / b.target) * 100))
                                  : 100;
                              const isAnim = isAnimatedBadgeIcon(b.icon);
                              const catClass = b.category ? `badge-cat-${b.category}` : '';
                              const tier = resolveBadgeTier(b);
                              const hasTiers = Array.isArray(b.tiers) && b.tiers.length > 0;
                              const progressText =
                                tier === 'platinum' ? (
                                  <span className="row nowrap gap-xs text-primary bold">
                                    <CheckCircle2 size={14} aria-hidden="true" />
                                    <span>{t('reports.badgeMaxTier')}</span>
                                  </span>
                                ) : b.next_tier && b.next_target != null ? (
                                  t('reports.badgeNextTier', {
                                    tier: t(`reports.badgeTier_${b.next_tier}`),
                                    progress: b.progress,
                                    target: b.next_target,
                                  })
                                ) : b.earned ? (
                                  <span className="row nowrap gap-xs text-primary bold">
                                    <CheckCircle2 size={14} aria-hidden="true" />
                                    <span>{t('reports.earned')}</span>
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
                                            {t(`reports.badgeTier_${tier}`)}
                                          </span>
                                        )}
                                        {caps.member_badge_icon_edit && (
                                          <Button
                                            variant="ghost"
                                            size="sm"
                                            className="btn-icon-tight"
                                            onClick={() => {
                                              setEditingBadge({
                                                badge: b,
                                                userId: r.user.id,
                                                userName: r.user.name,
                                              });
                                              setSelectedIcon(b.icon ?? '');
                                              setCustomIcon(
                                                ACHIEVEMENT_ICONS[b.icon ?? ''] ? '' : (b.icon ?? ''),
                                              );
                                            }}
                                            title={t('reports.changeBadgeIcon')}
                                            aria-label={`${t('reports.changeBadgeIcon')} - ${badgeName(b)}`}
                                          >
                                            <Pencil size={13} aria-hidden="true" />
                                          </Button>
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
                                              {t(`reports.badgeTier_${step.tier}`)} {step.target}
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
                            const filterOptions: { value: 'all' | BadgeCategory; label: string }[] = [
                              { value: 'all', label: t('reports.badgeFilterAll') },
                              { value: 'member', label: t('reports.badgeRole_member') },
                              { value: 'driver', label: t('reports.badgeRole_driver') },
                              { value: 'traffic_lead', label: t('reports.badgeRole_traffic_lead') },
                            ];

                            return (
                              <div className="stack-sm">
                                {hasCategories && (
                                  <div
                                    className="badge-filter-row"
                                    role="tablist"
                                    aria-label={t('reports.badges')}
                                  >
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
                                          <span
                                            className={`badge-role-dot badge-cat-${cat}`}
                                            aria-hidden="true"
                                          />
                                          <span className="bold small">
                                            {t(`reports.badgeRole_${cat}`)}
                                          </span>
                                          <span className="muted tiny">
                                            {earnedCount}/{group.length}
                                          </span>
                                        </div>
                                        <div className="stack-sm">{group.map(renderBadgeCard)}</div>
                                      </div>
                                    );
                                  })
                                ) : (
                                  <div className="stack-sm">{visibleBadges.map(renderBadgeCard)}</div>
                                )}
                              </div>
                            );
                          })()
                        )}
                      </Card>
                    )}
                  </div>
                )}
              </>
            )}
          </>
        )}
      </div>

      {editingBadge && (
        <Modal
          title={t('reports.changeBadgeIconTitle', { user: editingBadge.userName })}
          onClose={() => setEditingBadge(null)}
          footer={
            <div className="btn-row between">
              <Button variant="ghost" onClick={resetBadgeIcon} disabled={savingIcon}>
                {t('reports.badgeIconReset')}
              </Button>
              <div className="btn-row">
                <Button
                  variant="ghost"
                  onClick={() => setEditingBadge(null)}
                  disabled={savingIcon}
                >
                  {t('common:cancel')}
                </Button>
                <Button variant="primary" onClick={saveBadgeIcon} loading={savingIcon}>
                  {t('common:save')}
                </Button>
              </div>
            </div>
          }
        >
          <div className="stack">
            <div className="row gap-md items-center">
              {(() => {
                const modalTier = resolveBadgeTier(editingBadge.badge);
                return (
                  <div
                    className={`badge-char ${editingBadge.badge.earned ? 'earned' : ''} ${isAnimatedBadgeIcon(selectedIcon) ? `badge-animated badge-cat-${editingBadge.badge.category ?? 'driver'}` : ''} badge-tier-${modalTier}`.trim()}
                    aria-hidden="true"
                  >
                    <BadgeIcon icon={selectedIcon} fallbackChar={badgeChar(editingBadge.badge)} />
                    {modalTier !== 'none' && (
                      <span className={`badge-tier-crest badge-tier-${modalTier}`}>
                        {BADGE_TIER_ROMAN[modalTier]}
                      </span>
                    )}
                  </div>
                );
              })()}
              <div>
                <div className="bold">{badgeName(editingBadge.badge)}</div>
                <div className="muted small">{badgeRule(editingBadge.badge)}</div>
              </div>
            </div>

            <div className="stack-sm">
              <label className="bold small">{t('reports.badgeIconLabel')}</label>
              <div className="badge-icon-picker-sections">
                <div className="stack-xs">
                  <div className="badge-picker-section-title">
                    {t('reports.animatedBadgesLabel')}
                  </div>
                  {(['member', 'driver', 'traffic_lead'] as const).map((cat) => (
                    <div key={cat} className="stack-xs">
                      <div className="badge-group-header">
                        <span
                          className={`badge-role-dot badge-cat-${cat}`}
                          aria-hidden="true"
                        />
                        <span className="muted tiny bold">{t(`reports.badgeRole_${cat}`)}</span>
                      </div>
                      <div
                        className="badge-icon-grid"
                        role="radiogroup"
                        aria-label={t(`reports.badgeRole_${cat}`)}
                      >
                        {ANIMATED_BADGE_ICONS.filter((item) => item.category === cat).map(
                          (item) => {
                            const isSelected = selectedIcon === item.key;
                            return (
                              <button
                                key={item.key}
                                type="button"
                                role="radio"
                                aria-checked={isSelected}
                                aria-label={item.label}
                                title={item.label}
                                className={`badge-icon-btn animated badge-cat-${cat}${isSelected ? ' on' : ''}`}
                                onClick={() => {
                                  setSelectedIcon(item.key);
                                  setCustomIcon('');
                                }}
                              >
                                <BadgeIcon icon={item.key} />
                              </button>
                            );
                          },
                        )}
                      </div>
                    </div>
                  ))}
                </div>

                <div className="stack-xs">
                  <div className="badge-picker-section-title">{t('reports.classicIconsLabel')}</div>
                  <div
                    className="badge-icon-grid"
                    role="radiogroup"
                    aria-label={t('reports.classicIconsLabel')}
                  >
                    {Object.entries(ACHIEVEMENT_ICONS)
                      .filter(([key, item]) => !item.animated && !isAnimatedBadgeIcon(key))
                      .map(([key, item]) => {
                        const isSelected = selectedIcon === key;
                        return (
                          <button
                            key={key}
                            type="button"
                            role="radio"
                            aria-checked={isSelected}
                            aria-label={item.label}
                            title={item.label}
                            className={`badge-icon-btn${isSelected ? ' on' : ''}`}
                            onClick={() => {
                              setSelectedIcon(key);
                              setCustomIcon('');
                            }}
                          >
                            <BadgeIcon icon={key} size={34} />
                          </button>
                        );
                      })}
                  </div>
                </div>
              </div>
            </div>

            <Field label={t('reports.customIconPlaceholder')}>
              <input
                className="input"
                type="text"
                value={customIcon}
                placeholder={t('reports.customIconPlaceholder')}
                onChange={(e) => {
                  const val = e.target.value;
                  setCustomIcon(val);
                  setSelectedIcon(val);
                }}
              />
            </Field>

            <ErrorBox error={iconErr} />
          </div>
        </Modal>
      )}
    </Page>
  );
}
