import { useCallback, useEffect, useRef, useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Download, ExternalLink, Eye, FileText, ShieldAlert } from 'lucide-react';
import { createPdfWriter } from '../../pdfWriter';
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
import { ADMIN_HOME } from '../../staffNav';
import {
  ACTION_CATEGORIES,
  ENTITIES,
  FILTERS,
  SECURITY_ACTIONS,
  auditHelpers,
  getActionTone,
  getDayOffset,
  getFirstOfThisMonth,
  getObjectLink,
  highlightText,
  showObjectId,
  DetailDiff,
} from './auditHelpers';
import type { AuditItem, AuditPage } from './auditHelpers';

// Re-export so Dashboard.tsx keeps working with its `import { getObjectLink } from './Audit'`
export { getObjectLink } from './auditHelpers';

export default function Audit() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const h = auditHelpers(t);
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
  const filteredObject = items.find((item) =>
    item.entity === entity && String(item.entity_id) === entityId,
  );

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
    lines.push('ID,Time,Actor,Action,Entity,Entity ID,Entity Name,Detail');
    for (const a of items) {
      lines.push(
        [
          a.id,
          a.created_at,
          escapeCsv(a.actor ?? t('audit.system')),
          escapeCsv(a.action),
          escapeCsv(a.entity),
          a.entity_id ?? '',
          escapeCsv(a.entity_name ?? ''),
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

  const exportPdf = async () => {
    if (!items.length) return;
    const heads = [t('audit.time'), t('audit.actor'), t('audit.action'), t('audit.object'), t('audit.detail')];
    const generated = t('audit.pdfGenerated', { time: new Date().toLocaleString() });
    const pdfFilters = query ? t('audit.pdfFilters', { filters: query }) : '';
    const cells = items.map((a) => ({
      time: a.created_at.replace('T', ' ').slice(0, 19),
      actor: a.actor ?? t('audit.system'),
      action: h.actionLabel(a.action),
      object: h.objectLabel(a),
      detail: h.detailLabel(a),
    }));
    // Chinese cannot be drawn with the PDF's own font; the writer switches to the device's fonts.
    const w = await createPdfWriter([
      t('audit.pdfTitle'), generated, pdfFilters, ...heads,
      ...cells.flatMap((c) => [c.actor, c.action, c.object, c.detail]),
    ]);
    // Column left edges and widths in mm.
    const cols = [
      { x: 18, w: 35 }, { x: 55, w: 33 }, { x: 90, w: 38 }, { x: 130, w: 28 }, { x: 160, w: 35 },
    ];
    let y = 20;
    y += w.text(t('audit.pdfTitle'), 20, y, { size: 18, bold: true }) + 3;
    y += w.text(generated, 20, y, { size: 10 }) + 1;
    if (pdfFilters) y += w.text(pdfFilters, 20, y, { size: 10 }) + 1;
    y += 4;

    const header = () => {
      w.band(15, y, w.width - 30, 8);
      heads.forEach((hd, i) => w.text(hd, cols[i].x, y + 1.8, { size: 9, bold: true, maxWidth: cols[i].w, maxLines: 1 }));
      y += 10;
    };
    header();
    for (const c of cells) {
      if (y > w.height - 25) {
        w.addPage();
        y = 20;
        header();
      }
      const row = [c.time, c.actor, c.action, c.object, c.detail];
      const heights = row.map((v, i) => w.text(v, cols[i].x, y, { size: 9, maxWidth: cols[i].w, maxLines: i === 0 ? 1 : 2 }));
      y += Math.max(...heights) + 2.5;
    }

    const dateStr = new Date().toISOString().slice(0, 10);
    w.save(`grace-ride-audit-${dateStr}.pdf`);
  };

  const selectedTarget = selectedItem ? getObjectLink(selectedItem.entity, selectedItem.entity_id) : null;
  const securityCount = items.filter((a) => SECURITY_ACTIONS.includes(a.action)).length;
  const escalationCount = items.filter((a) => a.action === 'ride.escalate').length;
  const isSecurityActive = q === 'failed';

  const columns: TableColumn<AuditItem>[] = [
    {
      key: 'time',
      header: t('audit.time'),
      render: (a) => <span className="mono small no-wrap">{fmtDateTime(a.created_at, ft)}</span>,
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
            {highlightText(h.actionLabel(a.action), search)}
          </Pill>
        </button>
      ),
    },
    {
      key: 'object',
      header: t('audit.object'),
      render: (a) => {
        const target = getObjectLink(a.entity, a.entity_id);
        const label = h.objectLabel(a);
        return (
          <div style={{ display: 'inline-flex', flexDirection: 'column', alignItems: 'flex-start', minWidth: '11rem' }}>
            <div className="row gap-xs" style={{ display: 'inline-flex', alignItems: 'center', flexWrap: 'nowrap' }}>
              <button
                type="button"
                className="table-filter-link"
                onClick={() => {
                  setMany({ entity: a.entity, entity_id: a.entity_id == null ? '' : String(a.entity_id) });
                }}
                title={t('audit.filterByObject')}
              >
                {highlightText(label, search)}
              </button>
              {target?.exact && (
                <Link
                  to={target.url}
                  title={t('audit.viewObject')}
                  aria-label={`${t('audit.viewObject')} ${label}`}
                  className="icon-link"
                >
                  <ExternalLink size={14} />
                </Link>
              )}
            </div>
            {h.showsKind(a) && <span className="muted tiny">{highlightText(h.objectKind(a, ft), search)}</span>}
          </div>
        );
      },
    },
    {
      key: 'detail',
      header: t('audit.detail'),
      render: (a) => highlightText(h.detailLabel(a), search),
    },
    {
      key: 'viewDetail',
      header: t('audit.viewDetail'),
      headerClassName: 'col-sticky-end',
      cellClassName: 'col-sticky-end',
      render: (a) => (
        <Button
          variant="ghost"
          size="sm"
          icon={<Eye size={16} />}
          onClick={() => setSelectedItem(a)}
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
        back={ADMIN_HOME}
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
          onChange={(e) => setMany({ entity: e.target.value, entity_id: '' })}
          aria-label={t('audit.object')}
        >
          <option value="">{t('audit.allObjects')}</option>
          {entities.map((k) => (
            <option key={k} value={k}>
              {h.entityLabel(k)}
            </option>
          ))}
        </select>
        {entity && showObjectId(entity) && (
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
            {showObjectId(entity)
              ? `${h.entityLabel(entity)} #${entityId}`
              : filteredObject?.entity_name?.trim() || h.entityLabel(entity)}
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
              onRowClick={(a) => setSelectedItem(a)}
              compactRow={(a) => (
                <div className="grow min-w-0 stack-xs">
                  <div className="row between nowrap">
                    <span className="bold ellipsis">{a.actor ?? t('audit.system')}</span>
                    <span className="mono tiny muted">{fmtDateTime(a.created_at, ft)}</span>
                  </div>
                  <div>
                    <Pill tone={getActionTone(a.action)}>{h.actionLabel(a.action)}</Pill>
                  </div>
                  <div className="small ellipsis">{h.itemLine(a)}</div>
                  <div className="muted tiny clamp-2">{h.detailLabel(a)}</div>
                </div>
              )}
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
          title={t('audit.entryDetail')}
          onClose={() => setSelectedItem(null)}
          footer={
            <Button variant="secondary" style={{ marginInlineStart: 'auto' }} onClick={() => setSelectedItem(null)}>
              {t('common:close', { defaultValue: 'Close' })}
            </Button>
          }
        >
          <div className="audit-entry">
            <div className="audit-entry-row small">
              <span className="muted">{t('audit.logId')}</span>
              <span>#{selectedItem.id}</span>
            </div>
            <div className="audit-entry-row small">
              <span className="muted">{t('audit.time')}</span>
              <div className="audit-entry-time"><span>{fmtDateTime(selectedItem.created_at, ft)}</span><span className="mono muted small">{selectedItem.created_at}</span></div>
            </div>
            <div className="audit-entry-row small">
              <span className="muted">{t('audit.actor')}</span>
              <span>{selectedItem.actor ?? t('audit.system')}</span>
            </div>
            <div className="audit-entry-row small">
              <span className="muted">{t('audit.action')}</span>
              <div className="row gap-xs">
                <Pill tone={getActionTone(selectedItem.action)}>
                  {h.actionLabel(selectedItem.action)}
                </Pill>
                <span className="mono tiny muted">{selectedItem.action}</span>
              </div>
            </div>
            <div className="audit-entry-row small">
              <span className="muted">{t('audit.object')}</span>
              <div className="row gap-xs" style={{ alignItems: 'center' }}>
                <div style={{ display: 'flex', flexDirection: 'column' }}>
                  <span>{h.objectLabel(selectedItem)}</span>
                  {h.showsKind(selectedItem) && <span className="muted tiny">{h.objectKind(selectedItem, ft)}</span>}
                </div>
                {selectedTarget && (
                  <Link to={selectedTarget.url} className="btn ghost sm" onClick={() => setSelectedItem(null)}>
                    <ExternalLink size={14} />
                    {t(selectedTarget.exact ? 'audit.viewObject' : 'audit.viewList')}
                  </Link>
                )}
              </div>
            </div>
            <div className="flow-xs">
              <span className="muted small">{t('audit.detail')}</span>
              <Card style={{ maxHeight: 240, overflowY: 'auto', background: 'var(--surface-2)' }}>
                {selectedItem.attendance
                  ? <span className="small">{h.detailLabel(selectedItem)}</span>
                  : <DetailDiff detail={selectedItem.detail} />}
              </Card>
            </div>
          </div>
        </Modal>
      )}
      <BackToTop />
    </Page>
  );
}
