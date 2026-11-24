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

export function buildPeriodQuery(p: PeriodFilterState): string {
  const params = new URLSearchParams();
  if (p.useCustom && p.fromDate && p.toDate && p.toDate >= p.fromDate) {
    params.set('from', p.fromDate);
    params.set('to', p.toDate);
  } else {
    params.set('month', p.month);
    params.set('range', p.range);
  }
  return params.toString();
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
  const rangeInvalid = period.useCustom && period.fromDate && period.toDate && period.toDate < period.fromDate;

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
      {rangeInvalid && <p className="muted tiny">{t('reports.invalidDateRange')}</p>}
    </Card>
  );
}
