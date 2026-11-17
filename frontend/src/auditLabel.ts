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

/** People-facing name for what an audit entry is about, e.g. "ride" -> "Ride". */
export function auditEntityLabel(entity: string, t: TFunction): string {
  return t(`audit.entities.${entity}`, {
    defaultValue: (entity.charAt(0).toUpperCase() + entity.slice(1)).replace(/[_-]+/g, ' '),
  }) as string;
}

/** What an audit entry is about, as the server sends it. */
export interface AuditTarget {
  entity: string;
  entity_id: number | null;
  entity_name?: string | null;
  /** When the thing happens, where that tells two apart: a ride's pick-up time. */
  entity_at?: string | null;
}

/** Members and events are known by name alone; their number means nothing to an admin. */
export const showAuditItemId = (entity: string) => entity !== 'event' && entity !== 'user';

/**
 * The two halves of "what was this done to": the name a person recognises, when the
 * server knows one, and the kind with its number ("Ride #17") for telling it apart.
 */
export function auditItemParts(item: AuditTarget, t: TFunction): { name: string | null; kind: string } {
  const type = auditEntityLabel(item.entity, t);
  const id = showAuditItemId(item.entity) && item.entity_id != null ? ` #${item.entity_id}` : '';
  return { name: item.entity_name?.trim() || null, kind: `${type}${id}` };
}

/** One line for places with no room for two: the name, else the kind and number. */
export function auditItemLabel(item: AuditTarget, t: TFunction): string {
  const { name, kind } = auditItemParts(item, t);
  return name ?? kind;
}

/** One line that still tells two apart: "Mei Tan · Ride #17". For rows too small for two lines. */
export function auditItemLine(item: AuditTarget, t: TFunction): string {
  const { name, kind } = auditItemParts(item, t);
  return name && showAuditItemId(item.entity) ? `${name} · ${kind}` : name ?? kind;
}
