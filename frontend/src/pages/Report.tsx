import { useState } from 'react';
import { Link } from 'react-router-dom';
import { Award, Calendar, CarFront, CheckCircle2, DollarSign, Download, FileText, Map } from 'lucide-react';
import { useApi } from '../api';
import {
  Button,
  Card,
  EmptyState,
  ErrorBox,
  Page,
  PageHeader,
  Spinner,
  Stat,
} from '../components/ui';
import { EVENT_KIND, fmtDateTime, parseLocal, rm, useFormatT } from '../format';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../auth';
import { BadgeIcon } from '../badgeIcon';
import { createPdfWriter } from '../pdfWriter';

interface Badge {
  key: string;
  char: string;
  icon?: string;
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

  const escapeCsv = (v: unknown) => {
    const s = String(v ?? '');
    if (/[",\n\r]/.test(s)) return `"${s.replace(/"/g, '""')}"`;
    return s;
  };

  const exportCsv = () => {
    if (!r) return;
    const lines: string[] = [];
    lines.push('Grace Ride — Report Export');
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
    const badges = r.badges.map(
      (b) => `${badgeName(b)} — ${badgeRule(b)} (${b.progress}/${b.target}) ${b.earned ? t('earned') : ''}`,
    );
    const heads = [t('summary'), t('byKind'), t('events'), t('badges')];
    const rows: [string, string][] = [
      [t('events_attended'), String(r.events_attended)],
      [t('rides_driven'), String(r.rides_driven)],
      [t('rides_received'), String(r.rides_received)],
      [t('distance_km'), `${r.distance_km} km`],
      [t('reimbursement_cents'), `${r.reimbursement_cents}`],
      [t('costs_paid_cents'), `${r.costs_paid_cents}`],
    ];
    const meta = [
      `${t('user')}: ${r.user.name} (${r.user.id})`,
      `${t('period')}: ${r.period_start} to ${r.period_end}`,
      `${t('range')}: ${rangeMap[r.range] ?? r.range}`,
      `${t('type')}: ${typeMap[r.type] ?? r.type}`,
    ];
    const w = await createPdfWriter([t('pdfTitle'), ...meta, ...heads, ...rows.flat(), ...kinds, ...events, ...badges]);
    const M = 20;
    const body = w.width - M - 25;
    let yPos = 20;
    const room = () => {
      if (yPos > w.height - 40) {
        w.addPage();
        yPos = 20;
      }
    };
    const heading = (text: string) => {
      room();
      yPos += w.text(text, M, yPos, { size: 14, bold: true }) + 3;
    };
    const list = (items: string[]) => {
      for (const item of items) {
        room();
        yPos += w.text(item, M + 5, yPos, { maxWidth: body }) + 1.5;
      }
      yPos += 6;
    };

    yPos += w.text(t('pdfTitle'), M, yPos, { size: 18, bold: true }) + 4;
    for (const line of meta) yPos += w.text(line, M, yPos) + 1;
    yPos += 8;

    heading(t('summary'));
    for (const [label, val] of rows) {
      w.text(label, M + 5, yPos);
      yPos += w.text(val, w.width - 40, yPos, { align: 'right' }) + 1;
    }
    yPos += 6;

    if (kinds.length > 0) {
      heading(t('byKind'));
      list(kinds);
    }
    if (events.length > 0) {
      heading(t('events'));
      list(events);
    }
    if (badges.length > 0) {
      heading(t('badges'));
      list(badges);
    }

    w.save(`${fileBase}.pdf`);
  };

  return (
    <Page width="wide">
      <div className="stack">
        <PageHeader
          title={t('title', { month: periodLabel() })}
          subtitle={t('desc')}
          back="/me"
          actions={
            <>
              <input
                className="input report-month-input"
                type="month"
                value={month}
                onChange={(e) => setMonth(e.target.value)}
                aria-label={t('monthLabel')}
              />
              <select
                className="input"
                value={range}
                onChange={(e) => setRange(e.target.value as 'monthly' | 'quarterly' | 'yearly')}
                aria-label={t('rangeLabel')}
              >
                <option value="monthly">{t('rangeMonthly')}</option>
                <option value="quarterly">{t('rangeQuarterly')}</option>
                <option value="yearly">{t('rangeYearly')}</option>
              </select>
              {drives && (
                <select
                  className="input"
                  value={type}
                  onChange={(e) => setType(e.target.value as 'all' | 'driver' | 'passenger')}
                  aria-label={t('typeLabel')}
                >
                  <option value="all">{t('typeAll')}</option>
                  <option value="driver">{t('typeDriver')}</option>
                  <option value="passenger">{t('typePassenger')}</option>
                </select>
              )}
              <Button
                variant="ghost"
                size="sm"
                onClick={exportCsv}
                disabled={!r}
                icon={<FileText size={16} />}
              >
                {t('exportCsv')}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={exportPdf}
                disabled={!r}
                icon={<Download size={16} />}
              >
                {t('exportPdf')}
              </Button>
            </>
          }
        />

        <ErrorBox error={error} />

        {!r ? (
          <Spinner />
        ) : (
          <>
            {/* Stat tiles using standard Stat component */}
            <div className="grid cols-4">
              <Stat
                label={t('attendance')}
                value={r.events_attended}
                icon={<Calendar size={20} />}
              />
              {drives && (
                <Stat
                  label={t('driven')}
                  value={r.rides_driven}
                  icon={<CarFront size={20} />}
                />
              )}
              <Stat
                label={t('received')}
                value={r.rides_received}
                icon={<CarFront size={20} />}
              />
              <Stat
                label={t('costs')}
                value={<span className="nowrap">{rm(r.costs_paid_cents)}</span>}
                icon={<DollarSign size={20} />}
              />
              {drives && type !== 'passenger' && (
                <>
                  <Stat
                    label={t('distance')}
                    value={`${r.distance_km} km`}
                    icon={<Map size={20} />}
                  />
                  <Stat
                    label={t('reimbursement')}
                    value={<span className="nowrap">{rm(r.reimbursement_cents)}</span>}
                    icon={<DollarSign size={20} />}
                  />
                </>
              )}
            </div>

            <div className="split">
              {/* Left Column: Events attended */}
              <Card className="stack-sm">
                <h3>{t('events')}</h3>
                {r.by_kind.length > 0 ? (
                  <p className="bold">
                    {r.by_kind
                      .map((k) => `${ft(EVENT_KIND[k.kind]?.label ?? k.kind)} × ${k.count}`)
                      .join(' · ')}
                  </p>
                ) : (
                  <p className="muted">{t('noEvents', { y, m })}</p>
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
                  <EmptyState title={t('noEvents', { y, m })} />
                )}
                <p className="muted tiny">{t('note')}</p>
              </Card>

              {/* Right Column: Badges as cards */}
              <Card className="stack">
                <div>
                  <h3 className="row nowrap gap-sm">
                    <Award size={20} aria-hidden="true" />
                    <span>{t('badges')}</span>
                  </h3>
                  <p className="muted small">{t('badgesDesc')}</p>
                </div>

                {r.badges.length === 0 ? (
                  <p className="muted">{t('noBadges')}</p>
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
                            <BadgeIcon badge={b} fallbackChar={badgeChar(b)} />
                          </div>
                          <div className="grow min-w-0 stack-xs">
                            <div className="row between nowrap">
                              <strong className="ellipsis">{badgeName(b)}</strong>
                              <span className="small muted">
                                {b.earned ? (
                                  <span className="row nowrap gap-xs text-primary bold">
                                    <CheckCircle2 size={14} aria-hidden="true" />
                                    <span>{t('earned')}</span>
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
      </div>
    </Page>
  );
}
