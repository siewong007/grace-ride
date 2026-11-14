import { ReactNode, useCallback, useEffect, useRef, useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Check, Copy, Download, ExternalLink, Eye, FileText, ShieldAlert } from 'lucide-react';
import jsPDF from 'jspdf';
import { api } from '../../api';
import {
  BackToTop,
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Modal,
  Page,
  PageHeader,
  Pill,
  Stat,
  TableColumn,
  UniversalTable,
  useInfiniteScroll,
} from '../../components/ui';
import { fmtDateTime, useFormatT } from '../../format';

interface AuditItem {
  id: number;
  actor_id?: number | null;
  actor: string | null;
  action: string;
  entity: string;
  entity_id: number | null;
  detail: string;
  created_at: string;
}

/** One page of the log, newest first. `next_before` is null on the last page. */
interface AuditPage {
  items: AuditItem[];
  next_before: number | null;
  retention_days?: number;
}

// What an entry can be about, as the server names it in `entity`.
const ENTITIES = [
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
  'reimbursement_rates',
  'system_settings',
];

const SECURITY_ACTIONS = [
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

const getActionTone = (act: string): 'danger' | 'warning' | 'info' | 'neutral' => {
  if (SECURITY_ACTIONS.includes(act)) return 'danger';
  if (WARNING_ACTIONS.includes(act)) return 'warning';
  if (INFO_ACTIONS.includes(act)) return 'info';
  return 'neutral';
};

/** Predefined action groups for quick filtering. */
const ACTION_CATEGORIES = [
  {
    category: 'security',
    actions: [
      'login.passkey',
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
const FILTERS = ['q', 'entity', 'entity_id', 'action', 'actor_id', 'from', 'to'];

const formatIsoDate = (d: Date) => {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
};

const getDayOffset = (days: number) => {
  const d = new Date();
  d.setDate(d.getDate() + days);
  return formatIsoDate(d);
};

const getFirstOfThisMonth = () => {
  const d = new Date();
  d.setDate(1);
  return formatIsoDate(d);
};

export const getObjectLink = (entity: string, id: number | null): string | null => {
  if (id == null) return null;
  switch (entity) {
    case 'ride':
      return `/admin/rides/${id}`;
    case 'event':
      return `/events/${id}`;
    case 'user':
      return `/admin/members`;
    case 'help_ticket':
      return `/help/${id}`;
    case 'settlement':
      return `/settlements/${id}`;
    case 'reimbursement':
      return `/admin/reimbursements`;
    case 'group':
    case 'area':
      return `/admin/settings`;
    case 'role':
      return `/admin/roles`;
    case 'announcement':
      return `/admin/announcements`;
    default:
      return null;
  }
};

function highlightText(text: string, query: string): ReactNode {
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

function DetailDiff({ detail }: { detail: string }) {
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

export default function Audit() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const [params, setParams] = useSearchParams();

  const q = params.get('q') ?? '';
  const entity = params.get('entity') ?? '';
  const entityId = params.get('entity_id') ?? '';
  const action = params.get('action') ?? '';
  const actorId = params.get('actor_id') ?? '';
  const from = params.get('from') ?? '';
  const to = params.get('to') ?? '';
  const [search, setSearch] = useState(q);

  const [selectedItem, setSelectedItem] = useState<AuditItem | null>(null);
  const [copied, setCopied] = useState(false);
  const [isStreaming, setIsStreaming] = useState(false);

  const filters = new URLSearchParams();
  for (const k of FILTERS) {
    const v = params.get(k);
    if (v && !(k === 'entity_id' && !/^\d+$/.test(v))) filters.set(k, v);
  }
  const query = filters.toString();

  const [items, setItems] = useState<AuditItem[]>([]);
  const [next, setNext] = useState<number | null>(null);
  const [retentionDays, setRetentionDays] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Only the latest request may update the list: a slow answer to old filters is dropped.
  const latest = useRef(0);

  const load = useCallback(
    async (before: number | null) => {
      const mine = ++latest.current;
      if (before == null) setLoading(true);
      else setLoadingMore(true);
      try {
        const p = new URLSearchParams(query);
        if (before != null) p.set('before', String(before));
        const qs = p.toString();
        const page = await api<AuditPage>(`/admin/audit${qs ? `?${qs}` : ''}`);
        if (mine !== latest.current) return;
        setItems((prev) => (before == null ? page.items : [...prev, ...page.items]));
        setNext(page.next_before);
        if (page.retention_days != null) setRetentionDays(page.retention_days);
        setError(null);
      } catch (e) {
        if (mine === latest.current) setError((e as Error).message);
      } finally {
        if (mine === latest.current) {
          setLoading(false);
          setLoadingMore(false);
        }
      }
    },
    [query],
  );

  useEffect(() => {
    load(null);
  }, [load]);

  const { sentinelRef } = useInfiniteScroll({
    hasMore: next != null,
    loading: loadingMore || loading,
    onLoadMore: () => {
      if (next != null && !loadingMore && !loading) {
        load(next);
      }
    },
  });

  const set = useCallback(
    (k: string, v: string) => {
      setParams(
        (prev) => {
          const p = new URLSearchParams(prev);
          if (v) p.set(k, v);
          else p.delete(k);
          // The id only makes sense with the object it belongs to.
          if (k === 'entity' && !v) p.delete('entity_id');
          return p;
        },
        { replace: true },
      );
    },
    [setParams],
  );

  const setMany = useCallback(
    (values: Record<string, string>) => {
      setParams(
        (prev) => {
          const p = new URLSearchParams(prev);
          for (const [key, value] of Object.entries(values)) {
            if (value) p.set(key, value);
            else p.delete(key);
          }
          return p;
        },
        { replace: true },
      );
    },
    [setParams],
  );

  const setDates = (fromDate: string, toDate: string) => {
    setParams(
      (prev) => {
        const p = new URLSearchParams(prev);
        p.set('from', fromDate);
        p.set('to', toDate);
        return p;
      },
      { replace: true },
    );
  };

  // Search as the admin types, once they pause.
  useEffect(() => {
    const v = search.trim();
    if (v === q) return;
    const timer = setTimeout(() => set('q', v), 300);
    return () => clearTimeout(timer);
  }, [search, q, set]);

  const clearFilters = () => {
    setSearch('');
    setParams(new URLSearchParams(), { replace: true });
  };

  const hasFilters = Boolean(query);
  // A link may name an object this list doesn't know; keep it selectable.
  const entities = entity && !ENTITIES.includes(entity) ? [...ENTITIES, entity] : ENTITIES;
  const entityLabel = (k: string) => t(`audit.entities.${k}`, { defaultValue: k });

  const escapeCsv = (v: unknown): string => {
    const s = String(v ?? '');
    if (/[",\n\r]/.test(s)) return `"${s.replace(/"/g, '""')}"`;
    return s;
  };

  const exportCsv = () => {
    if (!items.length) return;
    const lines: string[] = [];
    lines.push('Grace Ride — Audit Log Export');
    lines.push(`Generated,${new Date().toISOString()}`);
    if (query) lines.push(`Filters,${escapeCsv(query)}`);
    lines.push('');
    lines.push('ID,Time,Actor,Action,Entity,Entity ID,Detail');
    for (const a of items) {
      lines.push(
        [
          a.id,
          a.created_at,
          escapeCsv(a.actor ?? t('audit.system')),
          escapeCsv(a.action),
          escapeCsv(a.entity),
          a.entity_id ?? '',
          escapeCsv(a.detail),
        ].join(','),
      );
    }
    const blob = new Blob([lines.join('\n')], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    const dateStr = new Date().toISOString().slice(0, 10);
    a.download = `grace-ride-audit-${dateStr}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const exportStreamCsv = async () => {
    try {
      setIsStreaming(true);
      const res = await fetch(`/api/admin/audit/export?${query}`, {
        credentials: 'same-origin',
      });
      if (!res.ok) throw new Error('Stream export failed');
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      const dateStr = new Date().toISOString().slice(0, 10);
      a.download = `grace-ride-audit-full-${dateStr}.csv`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      console.error('Failed to stream export audit logs', e);
      setError(t('audit.exportFailed'));
    } finally {
      setIsStreaming(false);
    }
  };

  const exportPdf = () => {
    if (!items.length) return;
    const doc = new jsPDF();
    const pageW = doc.internal.pageSize.getWidth();
    const pageH = doc.internal.pageSize.getHeight();
    let y = 20;

    doc.setFontSize(18);
    doc.text(t('audit.pdfTitle'), 20, y);
    y += 9;
    doc.setFontSize(10);
    doc.text(t('audit.pdfGenerated', { time: new Date().toLocaleString() }), 20, y);
    y += 6;
    if (query) {
      doc.text(t('audit.pdfFilters', { filters: query }), 20, y);
      y += 6;
    }
    y += 4;

    doc.setFontSize(9);
    doc.setFillColor(245, 245, 245);
    doc.rect(15, y, pageW - 30, 8, 'F');
    doc.setFont('helvetica', 'bold');
    doc.text(t('audit.time'), 18, y + 5.5);
    doc.text(t('audit.actor'), 55, y + 5.5);
    doc.text(t('audit.action'), 90, y + 5.5);
    doc.text(t('audit.object'), 130, y + 5.5);
    doc.text(t('audit.detail'), 160, y + 5.5);
    y += 10;
    doc.setFont('helvetica', 'normal');

    for (const a of items) {
      if (y > pageH - 20) {
        doc.addPage();
        y = 20;
        doc.setFont('helvetica', 'bold');
        doc.text(t('audit.time'), 18, y);
        doc.text(t('audit.actor'), 55, y);
        doc.text(t('audit.action'), 90, y);
        doc.text(t('audit.object'), 130, y);
        doc.text(t('audit.detail'), 160, y);
        y += 8;
        doc.setFont('helvetica', 'normal');
      }
      const timeStr = a.created_at.replace('T', ' ').slice(0, 19);
      const actorStr = doc.splitTextToSize(a.actor ?? t('audit.system'), 32)[0] || '';
      const actionStr = doc.splitTextToSize(a.action, 38)[0] || '';
      const objStr = `${a.entity}${a.entity_id != null ? ` #${a.entity_id}` : ''}`;
      const detailStr = doc.splitTextToSize(a.detail, pageW - 175)[0] || '';

      doc.text(timeStr, 18, y);
      doc.text(actorStr, 55, y);
      doc.text(actionStr, 90, y);
      doc.text(objStr, 130, y);
      doc.text(detailStr, 160, y);
      y += 7;
    }

    const dateStr = new Date().toISOString().slice(0, 10);
    doc.save(`grace-ride-audit-${dateStr}.pdf`);
  };

  const handleCopyJson = async () => {
    if (!selectedItem) return;
    try {
      await navigator.clipboard.writeText(JSON.stringify(selectedItem, null, 2));
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      setError(t('audit.copyFailed'));
    }
  };

  const selectedTargetUrl = selectedItem ? getObjectLink(selectedItem.entity, selectedItem.entity_id) : null;
  const securityCount = items.filter((a) => SECURITY_ACTIONS.includes(a.action)).length;
  const escalationCount = items.filter((a) => a.action === 'ride.escalate').length;
  const isSecurityActive = q === 'failed';

  const columns: TableColumn<AuditItem>[] = [
    {
      key: 'time',
      header: t('audit.time'),
      render: (a) => <span className="mono small">{fmtDateTime(a.created_at, ft)}</span>,
    },
    {
      key: 'actor',
      header: t('audit.actor'),
      render: (a) =>
        a.actor ? (
          <button
            type="button"
            className="table-filter-link"
            onClick={() => set('q', a.actor!)}
            title={t('audit.filterByActor')}
          >
            {highlightText(a.actor, search)}
          </button>
        ) : (
          t('audit.system')
        ),
    },
    {
      key: 'action',
      header: t('audit.action'),
      render: (a) => (
        <button
          type="button"
          className="table-filter-link mono"
          onClick={() => set('action', a.action)}
          title={t('audit.filterByAction')}
        >
          <Pill tone={getActionTone(a.action)}>
            {highlightText(a.action, search)}
          </Pill>
        </button>
      ),
    },
    {
      key: 'object',
      header: t('audit.object'),
      render: (a) => {
        const targetUrl = getObjectLink(a.entity, a.entity_id);
        return (
          <div className="row gap-xs" style={{ display: 'inline-flex', alignItems: 'center' }}>
            <button
              type="button"
              className="table-filter-link"
              onClick={() => {
                setMany({ entity: a.entity, entity_id: a.entity_id == null ? '' : String(a.entity_id) });
              }}
              title={t('audit.filterByObject')}
            >
              {highlightText(`${a.entity}${a.entity_id != null ? ` #${a.entity_id}` : ''}`, search)}
            </button>
            {targetUrl && (
              <Link
                to={targetUrl}
                title={t('audit.viewObject')}
                aria-label={`${t('audit.viewObject')} ${a.entity} #${a.entity_id}`}
                style={{ color: 'var(--muted)', display: 'inline-flex' }}
              >
                <ExternalLink size={12} />
              </Link>
            )}
          </div>
        );
      },
    },
    {
      key: 'detail',
      header: t('audit.detail'),
      render: (a) => highlightText(a.detail, search),
    },
    {
      key: 'viewDetail',
      header: t('audit.viewDetail'),
      render: (a) => (
        <Button
          variant="ghost"
          size="sm"
          icon={<Eye size={16} />}
          onClick={() => {
            setSelectedItem(a);
            setCopied(false);
          }}
          aria-label={`${t('audit.viewDetail')} #${a.id}`}
        >
          {t('audit.viewDetail')}
        </Button>
      ),
    },
  ];

  return (
    <Page className="audit-page">
      <PageHeader
        title={t('audit.title')}
        subtitle={t('audit.desc')}
        actions={
          <div className="row gap-xs">
            <Button
              variant="ghost"
              size="sm"
              onClick={exportStreamCsv}
              loading={isStreaming}
              icon={<Download size={16} />}
              title={t('audit.exportStreamCsvDesc')}
            >
              {t('audit.exportStreamCsv')}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={exportCsv}
              disabled={!items.length}
              icon={<FileText size={16} />}
            >
              {t('audit.exportCsv')}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={exportPdf}
              disabled={!items.length}
              icon={<Download size={16} />}
            >
              {t('audit.exportPdf')}
            </Button>
          </div>
        }
      />

      <div className="stat-strip" style={{ marginBottom: 'var(--space-3)' }}>
        <Stat label={t('audit.statsShown')} value={items.length} />
        <Stat label={t('audit.statsSecurity')} value={securityCount} />
        <Stat label={t('audit.statsEscalations')} value={escalationCount} />
        {retentionDays != null && (
          <Stat label={t('settings.auditRetention')} value={`${retentionDays}d`} />
        )}
      </div>

      <div className="admin-filters-bar">
        <input
          className="input admin-input-compact"
          type="search"
          placeholder={t('audit.search')}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          aria-label={t('audit.search')}
        />
        <select
          className="input admin-input-compact"
          value={entity}
          onChange={(e) => set('entity', e.target.value)}
          aria-label={t('audit.object')}
        >
          <option value="">{t('audit.allObjects')}</option>
          {entities.map((k) => (
            <option key={k} value={k}>
              {entityLabel(k)}
            </option>
          ))}
        </select>
        {entity && (
          <input
            className="input admin-input-compact"
            type="text"
            placeholder={t('audit.entityId')}
            style={{ width: '100px' }}
            value={/^\d+$/.test(entityId) ? entityId : ''}
            onChange={(e) => {
              const value = e.target.value.trim();
              if (!value || /^\d+$/.test(value)) set('entity_id', value);
            }}
            aria-label={t('audit.entityId')}
          />
        )}
        <select
          className="input admin-input-compact"
          value={action}
          onChange={(e) => set('action', e.target.value)}
          aria-label={t('audit.action')}
        >
          <option value="">{t('audit.allActions')}</option>
          {ACTION_CATEGORIES.map((cat) => (
            <optgroup key={cat.category} label={t(`audit.categories.${cat.category}`, { defaultValue: cat.category })}>
              {cat.actions.map((act) => (
                <option key={act} value={act}>
                  {act}
                </option>
              ))}
            </optgroup>
          ))}
          {action && !ACTION_CATEGORIES.some((c) => c.actions.includes(action)) && (
            <option value={action}>{action}</option>
          )}
        </select>
        <label className="row nowrap small muted">
          {t('audit.from')}
          <input
            className="input admin-input-compact"
            type="date"
            value={from}
            max={to || undefined}
            onChange={(e) => set('from', e.target.value)}
          />
        </label>
        <label className="row nowrap small muted">
          {t('audit.to')}
          <input
            className="input admin-input-compact"
            type="date"
            value={to}
            min={from || undefined}
            onChange={(e) => set('to', e.target.value)}
          />
        </label>
        <div className="row gap-xs">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setDates(getDayOffset(0), getDayOffset(0))}
          >
            {t('audit.today')}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setDates(getDayOffset(-1), getDayOffset(-1))}
          >
            {t('audit.yesterday')}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setDates(getDayOffset(-6), getDayOffset(0))}
          >
            {t('audit.last7Days')}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setDates(getFirstOfThisMonth(), getDayOffset(0))}
          >
            {t('audit.thisMonth')}
          </Button>
        </div>
        <Button
          variant={isSecurityActive ? 'primary' : 'ghost'}
          size="sm"
          onClick={() => set('q', isSecurityActive ? '' : 'failed')}
          icon={<ShieldAlert size={14} />}
        >
          {t('audit.securityOnly')}
        </Button>
        {entity && entityId && (
          <Pill>
            {entityLabel(entity)} #{entityId}
          </Pill>
        )}
        {action && (
          <Pill>
            {action}
          </Pill>
        )}
        {actorId && (
          <Pill>
            {t('audit.actorId')}: #{actorId}
          </Pill>
        )}
        {hasFilters && (
          <Button variant="ghost" size="sm" onClick={clearFilters}>
            {t('audit.clearFilters')}
          </Button>
        )}
      </div>

      <ErrorBox error={error} />

      {loading ? (
        <CardSkeleton count={4} />
      ) : !items.length ? (
        !error && <EmptyState title={t('audit.none')} />
      ) : (
        <>
          <Card>


            <UniversalTable
              columns={columns}
              items={items}
              itemKey={(a) => a.id}
            />
          </Card>

          <div className="row between">
            <span className="small muted" aria-live="polite">
              {t(next == null ? 'audit.shownAll' : 'audit.shown', { count: items.length })}
            </span>
            {next != null && (
              <Button onClick={() => load(next)} loading={loadingMore}>
                {t('audit.loadMore')}
              </Button>
            )}
          </div>
          <div ref={sentinelRef} className="infinite-sentinel" aria-hidden="true" />
        </>
      )}

      {selectedItem && (
        <Modal
          title={t('audit.entryDetail', { id: selectedItem.id })}
          onClose={() => setSelectedItem(null)}
          footer={
            <div className="row between" style={{ width: '100%' }}>
              <Button
                variant="ghost"
                size="sm"
                onClick={handleCopyJson}
                icon={copied ? <Check size={16} /> : <Copy size={16} />}
              >
                {copied ? t('audit.copied') : t('audit.copyJson')}
              </Button>
              <Button variant="secondary" onClick={() => setSelectedItem(null)}>
                {t('common:close', { defaultValue: 'Close' })}
              </Button>
            </div>
          }
        >
          <div className="audit-entry">
            <div className="audit-entry-row small">
              <span className="muted">{t('audit.time')}</span>
              <div className="audit-entry-time"><span>{fmtDateTime(selectedItem.created_at, ft)}</span><span className="mono muted small">{selectedItem.created_at}</span></div>
            </div>
            <div className="audit-entry-row small">
              <span className="muted">{t('audit.actor')}</span>
              <div className="row gap-xs">
                <span>{selectedItem.actor ?? t('audit.system')}</span>
                {selectedItem.actor && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      set('q', selectedItem.actor!);
                      setSelectedItem(null);
                    }}
                  >
                    {t('audit.filterByActor')}
                  </Button>
                )}
                {selectedItem.actor_id != null && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      set('actor_id', String(selectedItem.actor_id));
                      setSelectedItem(null);
                    }}
                  >
                    {t('audit.actorId')}: #{selectedItem.actor_id}
                  </Button>
                )}
              </div>
            </div>
            <div className="audit-entry-row small">
              <span className="muted">{t('audit.action')}</span>
              <div className="row gap-xs">
                <Pill tone={getActionTone(selectedItem.action)}>
                  {selectedItem.action}
                </Pill>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    set('action', selectedItem.action);
                    setSelectedItem(null);
                  }}
                >
                  {t('audit.filterByAction')}
                </Button>
              </div>
            </div>
            <div className="audit-entry-row small">
              <span className="muted">{t('audit.object')}</span>
              <div className="row gap-xs" style={{ alignItems: 'center' }}>
                <span>
                  {entityLabel(selectedItem.entity)}
                  {selectedItem.entity_id != null ? ` #${selectedItem.entity_id}` : ''}
                </span>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    setMany({ entity: selectedItem.entity, entity_id: selectedItem.entity_id == null ? '' : String(selectedItem.entity_id) });
                    setSelectedItem(null);
                  }}
                >
                  {t('audit.filterByObject')}
                </Button>
                {selectedTargetUrl && (
                  <Link to={selectedTargetUrl} className="btn ghost sm" onClick={() => setSelectedItem(null)}>
                    <ExternalLink size={14} />
                    {t('audit.viewObject')}
                  </Link>
                )}
              </div>
            </div>
            <div className="flow-xs">
              <span className="muted small">{t('audit.detail')}</span>
              <Card style={{ maxHeight: 240, overflowY: 'auto', background: 'var(--surface-2)' }}>
                <DetailDiff detail={selectedItem.detail} />
              </Card>
            </div>
          </div>
        </Modal>
      )}
      <BackToTop />
    </Page>
  );
}
