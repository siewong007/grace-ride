import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Card, Field, Pill } from '../../../components/ui';

export interface PeriodFilterState {
  month: string;
  onMonthChange: (m: string) => void;
  range: 'monthly' | 'quarterly' | 'yearly';
  onRangeChange: (r: 'monthly' | 'quarterly' | 'yearly') => void;
  useCustom: boolean;
  onToggleCustom: (custom: boolean) => void;
  fromDate: string;
  toDate: string;
  onFromChange: (d: string) => void;
  onToChange: (d: string) => void;
}

export interface GroupFilterOption {
  id: number;
  name: string;
  is_all?: boolean;
  is_visitor?: boolean;
}

export function isPeriodRangeInvalid(p: PeriodFilterState): 'order' | 'span' | null {
  if (!p.useCustom || !p.fromDate || !p.toDate) return null;
  if (p.toDate < p.fromDate) return 'order';
  const diffMs = Date.parse(`${p.toDate}T00:00:00Z`) - Date.parse(`${p.fromDate}T00:00:00Z`);
  if (Number.isFinite(diffMs) && diffMs / 86_400_000 > 366) return 'span';
  return null;
}

export function buildPeriodQuery(p: PeriodFilterState): string | null {
  if (isPeriodRangeInvalid(p)) return null;
  const params = new URLSearchParams();
  if (p.useCustom && p.fromDate && p.toDate) {
    params.set('from', p.fromDate);
    params.set('to', p.toDate);
  } else {
    params.set('month', p.month);
    params.set('range', p.range);
  }
  return params.toString();
}

export function DeltaPill({
  delta,
  unit = '',
  invertGood = false,
  formatAbs,
  prevPeriod,
}: {
  delta: number | null | undefined;
  unit?: string;
  invertGood?: boolean;
  formatAbs?: (absVal: number) => string;
  prevPeriod?: string;
}) {
  const { t } = useTranslation('admin');
  if (delta == null) return null;
  const absText = formatAbs ? formatAbs(Math.abs(delta)) : `${Math.abs(delta)}${unit}`;
  const label = delta > 0 ? `+${absText}` : delta < 0 ? `-${absText}` : t('reports.deltaFlat');
  const tone =
    delta === 0
      ? 'neutral'
      : (delta > 0) !== invertGood
      ? 'success'
      : 'danger';
  return (
    <span
      className="report-delta"
      title={prevPeriod ? t('reports.vsPrevPeriod', { period: prevPeriod }) : undefined}
    >
      <Pill tone={tone}>{label}</Pill>
    </span>
  );
}

export function MiniSeriesChart({
  title,
  items,
  formatVal,
}: {
  title: string;
  items: { month: string; value: number }[];
  formatVal?: (v: number) => string;
}) {
  const max = Math.max(1, ...items.map((i) => i.value));
  return (
    <Card className="stack-sm">
      <h4>{title}</h4>
      <div className="report-chart" role="img" aria-label={title}>
        {items.map((item) => {
          const pct = Math.max(4, Math.round((item.value / max) * 100));
          return (
            <div key={item.month} className="report-chart-col">
              <span className="muted tiny">{formatVal ? formatVal(item.value) : item.value}</span>
              <div className="report-chart-track">
                <div className="report-chart-fill" style={{ height: `${pct}%` }} />
              </div>
              <span className="muted tiny">{item.month.slice(5)}</span>
            </div>
          );
        })}
      </div>
    </Card>
  );
}

export function PeriodControls({
  period,
  scope,
  extraFilter,
  actions,
}: {
  period: PeriodFilterState;
  scope?: 'global' | 'own_events';
  extraFilter?: ReactNode;
  actions?: ReactNode;
}) {
  const { t } = useTranslation('admin');
  const invalidReason = isPeriodRangeInvalid(period);

  return (
    <Card className="stack-sm">
      <div className="report-scope-bar">
        <div className="row gap-xs flex-wrap items-center">
          <button
            type="button"
            className={`chip${!period.useCustom ? ' on' : ''}`}
            onClick={() => period.onToggleCustom(false)}
          >
            {t('reports.presetPeriod')}
          </button>
          <button
            type="button"
            className={`chip${period.useCustom ? ' on' : ''}`}
            onClick={() => period.onToggleCustom(true)}
          >
            {t('reports.customPeriod')}
          </button>
          {scope && (
            <Pill tone={scope === 'global' ? 'info' : 'warning'}>
              {scope === 'global' ? t('reports.scopeGlobal') : t('reports.scopeOwnEvents')}
            </Pill>
          )}
        </div>
        {actions && <div className="row gap-xs flex-wrap">{actions}</div>}
      </div>

      <div className="report-period-controls">
        {!period.useCustom ? (
          <>
            <Field label={t('reports.monthLabel')}>
              <input
                className="input"
                type="month"
                value={period.month}
                onChange={(e) => period.onMonthChange(e.target.value)}
              />
            </Field>
            <Field label={t('reports.range')}>
              <select
                className="input"
                value={period.range}
                onChange={(e) =>
                  period.onRangeChange(e.target.value as 'monthly' | 'quarterly' | 'yearly')
                }
              >
                <option value="monthly">{t('reports.rangeMonthly')}</option>
                <option value="quarterly">{t('reports.rangeQuarterly')}</option>
                <option value="yearly">{t('reports.rangeYearly')}</option>
              </select>
            </Field>
          </>
        ) : (
          <>
            <Field label={t('reports.fromDate')}>
              <input
                className="input"
                type="date"
                value={period.fromDate}
                onChange={(e) => period.onFromChange(e.target.value)}
              />
            </Field>
            <Field label={t('reports.toDate')}>
              <input
                className="input"
                type="date"
                value={period.toDate}
                min={period.fromDate || undefined}
                onChange={(e) => period.onToChange(e.target.value)}
              />
            </Field>
          </>
        )}
        {extraFilter}
      </div>
      {invalidReason === 'order' && (
        <p className="muted tiny">{t('reports.invalidDateRange')}</p>
      )}
      {invalidReason === 'span' && (
        <p className="muted tiny">{t('reports.customRangeTooLong')}</p>
      )}
    </Card>
  );
}
