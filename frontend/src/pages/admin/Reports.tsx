import { useState } from 'react';
import { Link } from 'react-router-dom';
import { Award, Calendar, CarFront, CheckCircle2, DollarSign, Download, FileText, Map } from 'lucide-react';
import { useApi } from '../../api';
import { useAuth } from '../../auth';
import {
  Button,
  Card,
  EmptyState,
  ErrorBox,
  Field,
  Page,
  PageHeader,
  CardSkeleton,
  Stat,
  Tabs,
} from '../../components/ui';
import { EVENT_KIND, fmtDateTime, rm, useFormatT } from '../../format';
import { useTranslation } from 'react-i18next';
import { createPdfWriter } from '../../pdfWriter';
import { ADMIN_HOME } from '../../staffNav';

interface Member {
  id: number;
  name: string;
  email?: string;
  roles?: string[];
}

interface Badge {
  key: string;
  char: string;
  name: string;
  rule: string;
  char_zh?: string;
  char_en?: string;
  name_zh?: string;
  name_en?: string;
  rule_zh?: string;
  rule_en?: string;
  progress: number;
  target: number;
  earned: boolean;
}

interface MonthReport {
  user: { id: number; name: string };
  month: string;
  range: string;
  type: string;
  period_start: string;
  period_end: string;
  rides_driven: number;
  rides_received: number;
  distance_km: number;
  reimbursement_cents: number;
  events_attended: number;
  by_kind: { kind: string; count: number }[];
  events: { id: number; title: string; kind: string; starts_at: string }[];
  costs_paid_cents: number;
  badges: Badge[];
}

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

export default function AdminReports() {
  const { t, i18n } = useTranslation('admin');
  const ft = useFormatT();
  const isZh = i18n.language?.startsWith('zh');
  const badgeChar = (b: Badge) => (isZh ? b.char_zh ?? b.char : b.char_en ?? b.char);
  const badgeName = (b: Badge) => (isZh ? b.name_zh ?? b.name : b.name_en ?? b.name);
  const badgeRule = (b: Badge) => (isZh ? b.rule_zh ?? b.rule : b.rule_en ?? b.rule);
  // The route only renders this page for members who may view member reports.
  const { me } = useAuth();
  const [selectedId, setSelectedId] = useState<number | null>(me?.id ?? null);
  const [month, setMonth] = useState(
    `${new Date().getFullYear()}-${String(new Date().getMonth() + 1).padStart(2, '0')}`,
  );
  const [range, setRange] = useState<'monthly' | 'quarterly' | 'yearly'>('monthly');
  const [type, setType] = useState<'all' | 'driver' | 'passenger'>('all');
  const [reportTab, setReportTab] = useState<'member' | 'demand'>('member');
  const [y, m] = month.split('-').map(Number);

  const { data: membersData } = useApi<Member[] | { items: Member[] }>('/admin/users?q=');
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

  const params = new URLSearchParams({
    month,
    range,
    type,
    ...(selectedId ? { user_id: String(selectedId) } : {}),
  });
  const { data: r, error, loading } = useApi<MonthReport>(
    `/reports/monthly?${params.toString()}`,
  );

  const fileBase = `grace-ride-admin-report-${range}-${month}-${selectedId ?? 'self'}`;

  const escapeCsv = (v: unknown) => {
    const s = String(v ?? '');
    if (/[",\n\r]/.test(s)) return `"${s.replace(/"/g, '""')}"`;
    return s;
  };

  const exportCsv = () => {
    if (!r) return;
    const lines: string[] = [];
    lines.push('Grace Ride — Admin Report Export');
    lines.push(`User,${r.user.name} (${r.user.id})`);
    lines.push(`Period,${r.period_start} to ${r.period_end}`);
    lines.push(`Range,${r.range}`);
    lines.push(`Type,${r.type}`);
    lines.push('');
    lines.push('Summary,Value');
    lines.push(`Events attended,${r.events_attended}`);
    lines.push(`Rides driven,${r.rides_driven}`);
    lines.push(`Rides received,${r.rides_received}`);
    lines.push(`Distance (km),${r.distance_km}`);
    lines.push(`Reimbursement (cents),${r.reimbursement_cents}`);
    lines.push(`Costs paid (cents),${r.costs_paid_cents}`);
    lines.push('');
    lines.push('Event Kind,Count');
    for (const k of r.by_kind) lines.push(`${escapeCsv(k.kind)},${k.count}`);
    lines.push('');
    lines.push('Event ID,Title,Kind,Starts At');
    for (const e of r.events)
      lines.push(`${e.id},${escapeCsv(e.title)},${escapeCsv(e.kind)},${e.starts_at}`);
    lines.push('');
    lines.push('Badge Key,Name,Rule,Progress,Target,Earned');
    for (const b of r.badges)
      lines.push(
        `${escapeCsv(b.key)},${escapeCsv(badgeName(b))},${escapeCsv(badgeRule(b))},${b.progress},${b.target},${b.earned}`,
      );
    const csv = lines.join('\n');
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${fileBase}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const exportPdf = async () => {
    if (!r) return;
    const kinds = r.by_kind.map((k) => `${ft(EVENT_KIND[k.kind]?.label ?? k.kind)} × ${k.count}`);
    const events = r.events.map((e) => `${e.title} — ${fmtDateTime(e.starts_at, ft)}`);
    const badges = r.badges.map(
      (b) => `${badgeName(b)} — ${badgeRule(b)} (${b.progress}/${b.target}) ${b.earned ? t('reports.earned') : ''}`,
    );
    const heads = [t('reports.summary'), t('reports.byKind'), t('reports.events'), t('reports.badges')];
    const rows: [string, string][] = [
      [t('reports.events_attended'), String(r.events_attended)],
      [t('reports.rides_driven'), String(r.rides_driven)],
      [t('reports.rides_received'), String(r.rides_received)],
      [t('reports.distance_km'), `${r.distance_km} km`],
      [t('reports.reimbursement_cents'), `${r.reimbursement_cents}`],
      [t('reports.costs_paid_cents'), `${r.costs_paid_cents}`],
    ];
    const meta = [
      `${t('reports.user')}: ${r.user.name} (${r.user.id})`,
      `${t('reports.period')}: ${r.period_start} to ${r.period_end}`,
      `${t('reports.range')}: ${t(RANGE_LABEL[r.range] ?? 'reports.rangeMonthly')}`,
      `${t('reports.type')}: ${t(TYPE_LABEL[r.type] ?? 'reports.typeAll')}`,
    ];
    // Chinese cannot be drawn with the PDF's own font; the writer switches to the device's fonts.
    const w = await createPdfWriter([t('reports.pdfTitle'), ...meta, ...heads, ...rows.flat(), ...kinds, ...events, ...badges]);
    const M = 20;
    const body = w.width - M - 25;
    let y = 20;
    const room = () => {
      if (y > w.height - 40) {
        w.addPage();
        y = 20;
      }
    };
    const heading = (text: string) => {
      room();
      y += w.text(text, M, y, { size: 14, bold: true }) + 3;
    };
    const list = (items: string[]) => {
      for (const item of items) {
        room();
        y += w.text(item, M + 5, y, { maxWidth: body }) + 1.5;
      }
      y += 6;
    };

    y += w.text(t('reports.pdfTitle'), M, y, { size: 18, bold: true }) + 4;
    for (const line of meta) y += w.text(line, M, y) + 1;
    y += 8;

    heading(t('reports.summary'));
    for (const [label, val] of rows) {
      w.text(label, M + 5, y);
      y += w.text(val, w.width - 40, y, { align: 'right' }) + 1;
    }
    y += 6;

    if (kinds.length > 0) {
      heading(t('reports.byKind'));
      list(kinds);
    }
    if (events.length > 0) {
      heading(t('reports.events'));
      list(events);
    }
    if (badges.length > 0) {
      heading(t('reports.badges'));
      list(badges);
    }
    w.save(`${fileBase}.pdf`);
  };

  return (
    <Page width="wide">
      <div className="stack">
        <PageHeader
          title={t('reports.title')}
          subtitle={t('reports.desc')}
          back={ADMIN_HOME}
          actions={
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
                disabled={!r}
                icon={<Download size={16} />}
              >
                {t('reports.exportPdf')}
              </Button>
            </>
          }
        />
        <ErrorBox error={error} />

        <Tabs
          value={reportTab}
          onChange={(v) => setReportTab(v as 'member' | 'demand')}
          items={[
            { value: 'member', label: t('reports.memberTab') },
            { value: 'demand', label: t('reports.demandTab') },
          ]}
        />

        {reportTab === 'demand' ? (
          <DemandAnalyticsPanel month={month} onMonthChange={setMonth} />
        ) : (
          <>
        <Card className="report-filters">
          <Field label={t('reports.monthLabel')}>
            <input
              className="input"
              type="month"
              value={month}
              onChange={(e) => setMonth(e.target.value)}
            />
          </Field>
          <Field label={t('reports.range')}>
            <select className="input" value={range} onChange={(e) => setRange(e.target.value as typeof range)}>
              <option value="monthly">{t('reports.rangeMonthly')}</option>
              <option value="quarterly">{t('reports.rangeQuarterly')}</option>
              <option value="yearly">{t('reports.rangeYearly')}</option>
            </select>
          </Field>
          <Field label={t('reports.type')}>
            <select className="input" value={type} onChange={(e) => setType(e.target.value as typeof type)}>
              <option value="all">{t('reports.typeAll')}</option>
              <option value="driver">{t('reports.typeDriver')}</option>
              <option value="passenger">{t('reports.typePassenger')}</option>
            </select>
          </Field>
          <div className="report-member">
            <Field label={t('reports.findMember')}>
              <input
                className="input"
                type="search"
                value={memberQuery}
                placeholder={t('reports.findMemberHint')}
                onChange={(e) => setMemberQuery(e.target.value)}
              />
            </Field>
            <Field label={t('reports.selectMember')}>
              <select
                className="input"
                value={selectedId ?? ''}
                onChange={(e) => setSelectedId(e.target.value ? Number(e.target.value) : null)}
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
        </Card>

        {loading ? (
          <CardSkeleton count={3} />
        ) : !r ? (
          <EmptyState title={t('reports.none')} />
        ) : (
          <>
            <div className="grid cols-4">
              <Stat
                label={t('reports.attendance')}
                value={r.events_attended}
                icon={<Calendar size={20} />}
              />
              <Stat
                label={t('reports.driven')}
                value={r.rides_driven}
                icon={<CarFront size={20} />}
              />
              <Stat
                label={t('reports.received')}
                value={r.rides_received}
                icon={<CarFront size={20} />}
              />
              <Stat
                label={t('reports.costs')}
                value={rm(r.costs_paid_cents)}
                icon={<DollarSign size={20} />}
              />
              <Stat
                label={t('reports.distance')}
                value={`${r.distance_km} km`}
                icon={<Map size={20} />}
              />
              <Stat
                label={t('reports.reimbursement')}
                value={rm(r.reimbursement_cents)}
                icon={<DollarSign size={20} />}
              />
            </div>

            <div className="grid cols-2">
              <Card className="stack-sm">
                <div>
                  <h3>{t('reports.events')}</h3>
                  {r.by_kind.length > 0 ? (
                    <p className="bold">
                      {r.by_kind
                        .map((k) => `${ft(EVENT_KIND[k.kind]?.label ?? k.kind)} × ${k.count}`)
                        .join(' · ')}
                    </p>
                  ) : (
                    <p className="muted">{t('reports.noEvents', { y, m })}</p>
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
                  <EmptyState title={t('reports.noEvents', { y, m })} />
                )}
                <p className="muted tiny">{t('reports.note')}</p>
              </Card>

              <Card className="stack">
                <div>
                  <h3 className="row nowrap gap-sm">
                    <Award size={20} aria-hidden="true" />
                    <span>{t('reports.badges')}</span>
                  </h3>
                  <p className="muted small">{t('reports.badgesDesc')}</p>
                </div>

                {r.badges.length === 0 ? (
                  <p className="muted">{t('reports.noBadges')}</p>
                ) : (
                  <div className="stack-sm">
                    {r.badges.map((b) => {
                      const percent = Math.min(100, Math.round((b.progress / b.target) * 100));
                      return (
                        <div key={b.key} className="card flat tight badge-card">
                          <div
                            className={`badge-char ${b.earned ? 'earned' : ''}`}
                            aria-hidden="true"
                          >
                            {badgeChar(b)}
                          </div>
                          <div className="grow min-w-0 stack-xs">
                            <div className="row between nowrap">
                              <strong className="ellipsis">{badgeName(b)}</strong>
                              <span className="small muted">
                                {b.earned ? (
                                  <span className="row nowrap gap-xs text-primary bold">
                                    <CheckCircle2 size={14} aria-hidden="true" />
                                    <span>{t('reports.earned')}</span>
                                  </span>
                                ) : (
                                  `${Math.min(b.progress, b.target)}/${b.target}`
                                )}
                              </span>
                            </div>
                            <div className="bar">
                              <div style={{ width: `${percent}%` }} />
                            </div>
                            <div className="muted tiny">{badgeRule(b)}</div>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </Card>
            </div>
          </>
        )}
          </>
        )}
      </div>
    </Page>
  );
}

interface DemandReportData {
  period: string;
  totals: {
    total_requests: number;
    fulfilled: number;
    unfilled: number;
    completed: number;
    cancelled_unassigned: number;
    wheelchair_requests: number;
    return_requests: number;
    seats_unfilled: number;
    active_drivers: number;
    unique_passengers: number;
    fulfillment_rate_pct: number;
  };
  by_area: {
    area_id: number;
    area_name_zh: string;
    area_name_en: string;
    total: number;
    fulfilled: number;
    unfilled: number;
    cancelled_unassigned: number;
    seats_needed: number;
    seats_unfilled: number;
    fulfillment_rate_pct: number;
  }[];
  by_slot: {
    weekday: number;
    slot: string;
    total: number;
    fulfilled: number;
    unfilled: number;
    cancelled_unassigned: number;
    seats_needed: number;
  }[];
}

const WEEKDAY_KEYS = ['sun', 'mon', 'tue', 'wed', 'thu', 'fri', 'sat'] as const;

function DemandAnalyticsPanel({
  month,
  onMonthChange,
}: {
  month: string;
  onMonthChange: (m: string) => void;
}) {
  const { t, i18n } = useTranslation('admin');
  const isZh = i18n.language.startsWith('zh');
  const { data, loading, error } = useApi<DemandReportData>(
    `/admin/reports/demand?month=${encodeURIComponent(month)}`,
  );
  const d = data && 'totals' in data ? data : null;

  return (
    <div className="stack">
      <Card className="report-filters">
        <Field label={t('reports.monthLabel')}>
          <input
            className="input"
            type="month"
            value={month}
            onChange={(e) => onMonthChange(e.target.value)}
          />
        </Field>
      </Card>

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
            <Stat label={t('reports.demandFulfillmentRate')} value={`${d.totals.fulfillment_rate_pct}%`} />
            <Stat label={t('reports.demandActiveDrivers')} value={d.totals.active_drivers} />
            <Stat label={t('reports.demandWheelchair')} value={d.totals.wheelchair_requests} />
            <Stat label={t('reports.demandReturnLegs')} value={d.totals.return_requests} />
            <Stat label={t('reports.demandCancelledUnassigned')} value={d.totals.cancelled_unassigned} />
          </div>

          <div className="grid cols-2">
            <Card className="stack-sm">
              <h3>{t('reports.demandByArea')}</h3>
              {d.by_area.length === 0 ? (
                <p className="muted small">{t('reports.none')}</p>
              ) : (
                <div className="list">
                  {d.by_area.map((a) => (
                    <div key={a.area_id} className="item row between nowrap gap-sm">
                      <span className="bold ellipsis">
                        {(isZh ? a.area_name_zh || a.area_name_en : a.area_name_en || a.area_name_zh) || `#${a.area_id}`}
                      </span>
                      <span className="small muted">
                        {t('reports.demandAreaLine', {
                          total: a.total,
                          unfilled: a.unfilled,
                          rate: a.fulfillment_rate_pct,
                        })}
                      </span>
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
        </>
      )}
    </div>
  );
}
