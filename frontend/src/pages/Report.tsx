import { useState } from 'react';
import { Link } from 'react-router-dom';
import { Award, Calendar, CarFront, CheckCircle2, DollarSign, Map } from 'lucide-react';
import { useApi } from '../api';
import {
  Card,
  EmptyState,
  ErrorBox,
  Page,
  PageHeader,
  Spinner,
  Stat,
} from '../components/ui';
import { EVENT_KIND, fmtDateTime, rm, useFormatT } from '../format';
import { useTranslation } from 'react-i18next';

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

export default function Report() {
  const { t, i18n } = useTranslation('report');
  const ft = useFormatT();
  const lang = i18n.language;
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

  const periodLabel = () => {
    const start = new Date(r?.period_start ?? `${month}-01`);
    const end = new Date(r?.period_end ?? `${month}-01`);
    end.setMonth(end.getMonth() + 1);
    const fmt = (d: Date) =>
      d.toLocaleDateString(lang === 'zh' ? 'zh-CN' : 'en-GB', {
        year: 'numeric',
        month: 'long',
      });
    if (range === 'monthly') return fmt(start);
    return `${fmt(start)} – ${fmt(end)}`;
  };

  return (
    <Page width="wide">
      <div className="stack">
        <PageHeader
          title={t('title', { month: periodLabel() })}
          subtitle={t('desc')}
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
                onChange={(e) => setRange(e.target.value as any)}
                aria-label="range"
              >
                <option value="monthly">{t('rangeMonthly')}</option>
                <option value="quarterly">{t('rangeQuarterly')}</option>
                <option value="yearly">{t('rangeYearly')}</option>
              </select>
              <select
                className="input"
                value={type}
                onChange={(e) => setType(e.target.value as any)}
                aria-label="type"
              >
                <option value="all">{t('typeAll')}</option>
                <option value="driver">{t('typeDriver')}</option>
                <option value="passenger">{t('typePassenger')}</option>
              </select>
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
              <Stat
                label={t('driven')}
                value={r.rides_driven}
                icon={<CarFront size={20} />}
              />
              <Stat
                label={t('received')}
                value={r.rides_received}
                icon={<CarFront size={20} />}
              />
              <Stat
                label={t('costs')}
                value={rm(r.costs_paid_cents)}
                icon={<DollarSign size={20} />}
              />
              {type !== 'passenger' && (
                <>
                  <Stat
                    label={t('distance')}
                    value={`${r.distance_km} km`}
                    icon={<Map size={20} />}
                  />
                  <Stat
                    label={t('reimbursement')}
                    value={rm(r.reimbursement_cents)}
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
                            {b.char}
                          </div>
                          <div className="grow min-w-0 stack-xs">
                            <div className="row between nowrap">
                              <strong className="ellipsis">{b.name}</strong>
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
