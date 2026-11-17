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
} from '../../components/ui';
import { EVENT_KIND, fmtDateTime, rm, useFormatT } from '../../format';
import { useTranslation } from 'react-i18next';
import jsPDF from 'jspdf';
import { ADMIN_HOME } from '../../staffNav';

interface Member {
  id: number;
  name: string;
  email: string;
  roles: string[];
}

interface Badge {
  key: string;
  char: string;
  name: string;
  rule: string;
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

export default function AdminReports() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  // The route only renders this page for members who may view member reports.
  const { me } = useAuth();
  const [selectedId, setSelectedId] = useState<number | null>(me?.id ?? null);
  const [month, setMonth] = useState(
    `${new Date().getFullYear()}-${String(new Date().getMonth() + 1).padStart(2, '0')}`,
  );
  const [range, setRange] = useState<'monthly' | 'quarterly' | 'yearly'>('monthly');
  const [type, setType] = useState<'all' | 'driver' | 'passenger'>('all');
  const [y, m] = month.split('-').map(Number);

  const { data: membersData } = useApi<Member[]>('/admin/users?q=');
  const members = membersData ?? [];
  // The picker lists everyone, so a search box narrows it. The chosen member always stays listed.
  const [memberQuery, setMemberQuery] = useState('');
  const needle = memberQuery.trim().toLowerCase();
  const shownMembers = needle
    ? members.filter(
        (m) =>
          m.id === selectedId ||
          m.name.toLowerCase().includes(needle) ||
          m.email.toLowerCase().includes(needle),
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
        `${escapeCsv(b.key)},${escapeCsv(b.name)},${escapeCsv(b.rule)},${b.progress},${b.target},${b.earned}`,
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

  const exportPdf = () => {
    if (!r) return;
    // The PDF's built-in font cannot draw Chinese: print the page instead, which uses the
    // device's own fonts and can be saved as a PDF from the print dialog.
    const drawable = (s: string) => !/[^\u0000-\u024f\u2000-\u206f]/.test(s);
    const texts = [
      r.user.name,
      t('reports.pdfTitle'),
      t('reports.summary'),
      t('reports.events_attended'),
      ...r.events.map((e) => e.title),
      ...r.by_kind.map((k) => ft(EVENT_KIND[k.kind]?.label ?? k.kind)),
      ...r.badges.flatMap((b) => [b.name, b.rule]),
    ];
    if (!texts.every(drawable)) {
      window.print();
      return;
    }
    const doc = new jsPDF();
    const pageW = doc.internal.pageSize.getWidth();
    const pageH = doc.internal.pageSize.getHeight();
    let y = 20;

    doc.setFontSize(18);
    doc.text(t('reports.pdfTitle'), 20, y);
    y += 10;
    doc.setFontSize(11);
    doc.text(`${t('reports.user')}: ${r.user.name} (${r.user.id})`, 20, y);
    y += 7;
    doc.text(`${t('reports.period')}: ${r.period_start} to ${r.period_end}`, 20, y);
    y += 7;
    doc.text(`${t('reports.range')}: ${r.range}`, 20, y);
    y += 7;
    doc.text(`${t('reports.type')}: ${r.type}`, 20, y);
    y += 12;

    doc.setFontSize(14);
    doc.text(t('reports.summary'), 20, y);
    y += 8;
    const rows: [string, string][] = [
      [t('reports.events_attended'), String(r.events_attended)],
      [t('reports.rides_driven'), String(r.rides_driven)],
      [t('reports.rides_received'), String(r.rides_received)],
      [t('reports.distance_km'), `${r.distance_km} km`],
      [t('reports.reimbursement_cents'), `${r.reimbursement_cents}`],
      [t('reports.costs_paid_cents'), `${r.costs_paid_cents}`],
    ];
    for (const [label, val] of rows) {
      doc.text(label, 25, y);
      doc.text(val, pageW - 40 - doc.getTextWidth(val), y);
      y += 7;
    }
    y += 8;

    if (r.by_kind.length > 0) {
      doc.setFontSize(14);
      doc.text(t('reports.byKind'), 20, y);
      y += 8;
      for (const k of r.by_kind) {
        doc.text(
          `${ft(EVENT_KIND[k.kind]?.label ?? k.kind)} × ${k.count}`,
          25,
          y,
        );
        y += 6;
      }
      y += 8;
    }

    if (r.events.length > 0) {
      doc.setFontSize(14);
      doc.text(t('reports.events'), 20, y);
      y += 8;
      for (const e of r.events) {
        if (y > pageH - 40) {
          doc.addPage();
          y = 20;
        }
        doc.text(`${e.title} — ${fmtDateTime(e.starts_at, ft)}`, 25, y);
        y += 6;
      }
      y += 8;
    }

    if (r.badges.length > 0) {
      doc.setFontSize(14);
      doc.text(t('reports.badges'), 20, y);
      y += 8;
      for (const b of r.badges) {
        if (y > pageH - 40) {
          doc.addPage();
          y = 20;
        }
        doc.text(
          `${b.name} — ${b.rule} (${b.progress}/${b.target}) ${b.earned ? t('reports.earned') : ''}`,
          25,
          y,
        );
        y += 6;
      }
    }

    doc.save(`${fileBase}.pdf`);
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
                    {mem.name} ({mem.email})
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
                            {b.char}
                          </div>
                          <div className="grow min-w-0 stack-xs">
                            <div className="row between nowrap">
                              <strong className="ellipsis">{b.name}</strong>
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
                            <div className="muted tiny">{b.rule}</div>
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
