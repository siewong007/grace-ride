import type { TFunction } from 'i18next';

/** People-facing name for an audit code such as "login.dev". */
export function auditActionLabel(action: string, t: TFunction): string {
  const key = `audit.actionLabels.${action.replaceAll('.', '_')}`;
  const label = t(key, { defaultValue: '' });
  if (typeof label === 'string' && label.length > 0 && label !== key) return label;
  return action
    .split(/[._]+/)
    .filter(Boolean)
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}
