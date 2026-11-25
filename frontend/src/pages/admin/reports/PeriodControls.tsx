import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Card, Field, Pill } from '../../../components/ui';
import { rm } from '../../../format';

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

export function fmtCompactRm(cents: number): string {
  const val = cents / 100;
  if (val === 0) return 'RM 0';
  if (Math.abs(val) >= 1000) {
    const k = (val / 1000).toFixed(Math.abs(val) >= 10_000 ? 1 : 2).replace(/\.?0+$/, '');
    return `RM ${k}k`;
  }
  return `RM ${Math.round(val)}`;
}

export function StatCountAmount({ count, cents }: { count: number; cents: number }) {
  return (
    <span className="report-stat-val">
      <span className="nowrap">{count} ·</span>
      <span className="nowrap">{rm(cents)}</span>
    </span>
  );
}

export function MiniSeriesChart({
  title,
  items,
  formatVal,
  formatTooltip,
  headingLevel = 'h2',
}: {
  title: string;
  items: { month: string; value: number }[];
  formatVal?: (v: number) => string;
  formatTooltip?: (v: number) => string;
  headingLevel?: 'h2' | 'h3';
}) {
  const max = Math.max(1, ...items.map((i) => i.value));
  const spansYears = new Set(items.map((i) => i.month.slice(0, 4))).size > 1;
  const Heading = headingLevel === 'h3' ? 'h3' : 'h2';
  return (
    <Card className="stack-sm">
      <Heading>{title}</Heading>
      {/* eslint-disable-next-line jsx-a11y/no-noninteractive-tabindex -- scrollable region must be keyboard-focusable */}
      <div className="report-chart" role="region" aria-label={title} tabIndex={0}>
        {items.map((item, idx) => {
          const pct = item.value <= 0 ? 0 : Math.max(6, Math.round((item.value / max) * 100));
          const shortLabel = formatVal ? formatVal(item.value) : String(item.value);
          const fullLabel = formatTooltip ? formatTooltip(item.value) : shortLabel;
          const monthPart = item.month.slice(5);
          const monthLabel =
            spansYears && (idx === 0 || monthPart === '01')
              ? `${item.month.slice(2, 4)}/${monthPart}`
              : monthPart;
          return (
            <div key={item.month} className="report-chart-col" title={`${item.month}: ${fullLabel}`}>
              <span className="report-chart-val">{shortLabel}</span>
              <div className="report-chart-track">
                <div
                  className={`report-chart-fill${item.value <= 0 ? ' is-zero' : ''}`}
                  style={{ height: `${pct}%` }}
                />
              </div>
              <span className="muted tiny nowrap">{monthLabel}</span>
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
        <p className="report-inline-error" role="alert">{t('reports.invalidDateRange')}</p>
      )}
      {invalidReason === 'span' && (
        <p className="report-inline-error" role="alert">{t('reports.customRangeTooLong')}</p>
      )}
    </Card>
  );
}
