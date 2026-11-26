import { ReactNode } from 'react';
import type { TFunction } from 'i18next';
import { auditActionLabel, auditEntityLabel, auditItemLabel, auditItemLine, auditItemParts, showAuditItemId } from '../../auditLabel';
import { fmtDateTime } from '../../format';

export interface AuditItem {
  id: number;
  actor_id?: number | null;
  actor: string | null;
  action: string;
  entity: string;
  entity_id: number | null;
  entity_name?: string | null;
  entity_at?: string | null;
  detail: string;
  attendance?: {
    member_id: number;
    member_name: string | null;
    status: 'attended' | 'absent' | 'cleared';
  } | null;
  created_at: string;
}

export const showObjectId = showAuditItemId;

/** One page of the log, newest first. `next_before` is null on the last page. */
export interface AuditPage {
  items: AuditItem[];
  next_before: number | null;
  retention_days?: number;
}

// What an entry can be about, as the server names it in `entity`.
export const ENTITIES = [
  'user',
  'ride',
  'event',
  'settlement',
  'settlement_share',
  'reimbursement',
  'help_ticket',
  'file',
  'group',
  'area',
  'role',
  'invite',
  'announcement',
  'recovery',
  'guide_workflow',
  'visitor_pass',
  'area_fares',
  'system_settings',
  'payout_batch',
  'app_secrets',
  'family',
  'finance_report',
  'users',
];

export const SECURITY_ACTIONS = [
  'login.failed',
  'totp.failed',
  'password.failed',
  'recovery.reject',
];

const WARNING_ACTIONS = [
  'ride.escalate',
  'ride.no_show',
  'ride.return',
  'ride.cancel',
  'group.delete',
  'role.delete',
  'invite.revoke',
];

const INFO_ACTIONS = [
  'role.create',
  'role.update',
  'recovery.approve',
  'recovery.await_second',
  'reimbursement.pay',
  'settlement.publish',
];

export const getActionTone = (act: string): 'danger' | 'warning' | 'info' | 'neutral' => {
  if (SECURITY_ACTIONS.includes(act)) return 'danger';
  if (WARNING_ACTIONS.includes(act)) return 'warning';
  if (INFO_ACTIONS.includes(act)) return 'info';
  return 'neutral';
};

/** Predefined action groups for quick filtering. */
export const ACTION_CATEGORIES = [
  {
    category: 'security',
    actions: [
      'login.passkey',
      'login.reauth',
      'login.dev',
      'login.failed',
      'password.failed',
      'logout',
      'passkey.register',
      'passkey.delete',
      'totp.setup',
      'totp.enable',
      'totp.failed',
      'password.set',
      'recovery.request',
      'recovery.use',
      'recovery.approve',
      'recovery.reject',
      'recovery.await_second',
      'invite.accept',
      'terms.accept',
      'totp.backup_code_used',
      'totp.backup_codes_regenerated',
      'totp.verify',
      'logout.all',
    ],
  },
  {
    category: 'rides',
    actions: [
      'ride.create',
      'ride.claim',
      'ride.driver_confirm',
      'ride.driver_withdraw',
      'ride.passenger_accept',
      'ride.passenger_decline',
      'ride.ready',
      'ride.approve',
      'ride.reassign',
      'ride.plan_assign',
      'ride.alt_transport',
      'ride.return',
      'ride.delayed',
      'ride.driver_delay',
      'ride.arrived_gate',
      'ride.unable_to_join',
      'ride.start',
      'ride.pickup',
      'ride.no_show',
      'ride.complete',
      'ride.cancel',
      'ride.escalate',
      'location.share',
      'location.stop',
      'driver.profile_update',
    ],
  },
  {
    category: 'events',
    actions: [
      'event.create',
      'events.create',
      'event.update',
      'event.cancel',
      'event.finish',
      'event.signup',
      'event.withdraw',
      'event.attendance',
      'event.waitlist_promote',
    ],
  },
  {
    category: 'finance',
    actions: [
      'reimbursement.create',
      'reimbursement.resubmit',
      'reimbursement.review',
      'reimbursement.approve',
      'reimbursement.reject',
      'reimbursement.pay',
      'settlement.create',
      'settlement.edit_draft',
      'settlement.publish',
      'settlement.close',
      'settlement.delete',
      'share.proof',
      'share.confirm',
      'share.needs_info',
      'settlement.revise',
    ],
  },
  {
    category: 'admin',
    actions: [
      'user.update',
      'role.create',
      'role.update',
      'role.delete',
      'role.bootstrap',
      'admin_roles.update',
      'group.create',
      'group.update',
      'group.delete',
      'group.members',
      'area.create',
      'area.update',
      'settings.rates',
      'settings.system',
      'announcement.create',
      'announcement.update',
      'announcement.delete',
      'invite.create',
      'invite.revoke',
      'help.create',
      'help.reply',
      'help.close',
      'guide.create',
      'guide.update',
      'guide.delete',
      'profile.update',
      'file.upload',
      'report.delete',
    ],
  },
];

/** The filters the server understands; anything else in the address is ignored. */
export const FILTERS = ['q', 'entity', 'entity_id', 'action', 'actor_id', 'from', 'to'];

export const formatIsoDate = (d: Date) => {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
};

export const getDayOffset = (days: number) => {
  const d = new Date();
  d.setDate(d.getDate() + days);
  return formatIsoDate(d);
};

export const getFirstOfThisMonth = () => {
  const d = new Date();
  d.setDate(1);
  return formatIsoDate(d);
};

/**
 * Where an audit entry's item can be opened. `exact` is true when the page is that one
 * item; otherwise it is only the list the item lives in, and the link must say so.
 */
export const getObjectLink = (
  entity: string,
  id: number | null,
): { url: string; exact: boolean } | null => {
  if (id == null) return null;
  const item = (url: string) => ({ url, exact: true });
  const list = (url: string) => ({ url, exact: false });
  switch (entity) {
    case 'ride':
      return item(`/admin/rides/${id}`);
    case 'event':
      return item(`/events/${id}`);
    case 'help_ticket':
      return item(`/help/${id}`);
    case 'settlement':
      return item(`/settlements/${id}`);
    case 'user':
      return list('/admin/members');
    case 'reimbursement':
      return list('/admin/reimbursements');
    case 'group':
    case 'area':
      return list('/admin/settings');
    case 'role':
      return list('/admin/roles');
    case 'announcement':
      return list('/admin/announcements');
    default:
      return null;
  }
};

export function highlightText(text: string, query: string): ReactNode {
  if (!query.trim()) return text;
  const qLower = query.trim().toLowerCase();
  const textLower = text.toLowerCase();
  const idx = textLower.indexOf(qLower);
  if (idx === -1) return text;
  const before = text.slice(0, idx);
  const match = text.slice(idx, idx + query.trim().length);
  const after = text.slice(idx + query.trim().length);
  return (
    <>
      {before}
      <mark className="search-hit">{match}</mark>
      {highlightText(after, query)}
    </>
  );
}

export function DetailDiff({ detail }: { detail: string }) {
  const parts = detail.split(';').map((p) => p.trim()).filter(Boolean);
  const diffs = parts.map((part) => {
    const m = part.match(/^(.*?)(?:\s*(?:→|->)\s*)(.*)$/);
    if (m) {
      return { from: m[1], to: m[2], isDiff: true };
    }
    return { raw: part, isDiff: false };
  });

  const hasDiff = diffs.some((d) => d.isDiff);
  if (!hasDiff) {
    return (
      <pre className="mono small" style={{ margin: 0, whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>
        {detail || '-'}
      </pre>
    );
  }

  return (
    <div className="flow-xs">
      {diffs.map((d, i) =>
        d.isDiff ? (
          <div key={i} className="row nowrap between small" style={{ padding: '4px 0', borderBottom: '1px solid var(--border)' }}>
            <span className="mono" style={{ textDecoration: 'line-through', opacity: 0.7 }}>
              {d.from}
            </span>
            <span style={{ margin: '0 8px' }}>→</span>
            <span className="mono font-bold" style={{ color: 'var(--primary)' }}>
              {d.to}
            </span>
          </div>
        ) : (
          <div key={i} className="small mono muted">
            {d.raw}
          </div>
        ),
      )}
    </div>
  );
}

/** Helpers that need a `t()` function. */
export const auditHelpers = (t: TFunction) => ({
  entityLabel: (k: string) => auditEntityLabel(k, t),
  objectLabel: (item: AuditItem) => auditItemLabel(item, t),
  objectKind: (item: AuditItem, ft: ReturnType<typeof import('../../format').useFormatT>) => {
    const { kind } = auditItemParts(item, t);
    return item.entity === 'ride' && item.entity_at
      ? `${kind} · ${fmtDateTime(item.entity_at, ft)}`
      : kind;
  },
  showsKind: (item: AuditItem) =>
    showObjectId(item.entity) && Boolean(item.entity_name?.trim()),
  detailLabel: ({ detail, attendance }: AuditItem) => {
    if (!attendance) return detail;
    const member = attendance.member_name?.trim() || t('audit.attendance.member');
    return t(`audit.attendance.${attendance.status}`, { member });
  },
  actionLabel: (action: string) => auditActionLabel(action, t),
  itemLine: (item: AuditItem) => auditItemLine(item, t),
});
