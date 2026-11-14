import { useCallback, useEffect, useRef, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { api } from '../../api';
import {
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Page,
  PageHeader,
  Pill,
} from '../../components/ui';
import { fmtDateTime, useFormatT } from '../../format';

interface AuditItem {
  id: number;
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
];

/** The filters the server understands; anything else in the address is ignored. */
const FILTERS = ['q', 'entity', 'entity_id', 'from', 'to'];

export default function Audit() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const [params, setParams] = useSearchParams();

  const q = params.get('q') ?? '';
  const entity = params.get('entity') ?? '';
  const entityId = params.get('entity_id') ?? '';
  const from = params.get('from') ?? '';
  const to = params.get('to') ?? '';
  const [search, setSearch] = useState(q);

  const filters = new URLSearchParams();
  for (const k of FILTERS) {
    const v = params.get(k);
    if (v) filters.set(k, v);
  }
  const query = filters.toString();

  const [items, setItems] = useState<AuditItem[]>([]);
  const [next, setNext] = useState<number | null>(null);
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

  const set = useCallback(
    (k: string, v: string) => {
      setParams(
        (prev) => {
          const p = new URLSearchParams(prev);
          if (v) p.set(k, v);
          else p.delete(k);
          // The id only makes sense with the object it belongs to.
          if (k === 'entity') p.delete('entity_id');
          return p;
        },
        { replace: true },
      );
    },
    [setParams],
  );

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

  return (
    <Page>
      <PageHeader title={t('audit.title')} subtitle={t('audit.desc')} />

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
        {/* Two dates side by side need their names shown, not only announced. */}
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
        {entity && entityId && (
          <Pill>
            {entityLabel(entity)} #{entityId}
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
            <div className="table-wrap">
              <table className="table responsive sticky-header">
                <thead>
                  <tr>
                    <th>{t('audit.time')}</th>
                    <th>{t('audit.actor')}</th>
                    <th>{t('audit.action')}</th>
                    <th>{t('audit.object')}</th>
                    <th>{t('audit.detail')}</th>
                  </tr>
                </thead>
                <tbody>
                  {items.map((a) => (
                    <tr key={a.id}>
                      <td data-label={t('audit.time')} className="mono small">
                        {fmtDateTime(a.created_at, ft)}
                      </td>
                      <td data-label={t('audit.actor')}>{a.actor ?? t('audit.system')}</td>
                      <td data-label={t('audit.action')} className="mono small">
                        {a.action}
                      </td>
                      <td data-label={t('audit.object')} className="small">
                        {a.entity}
                        {a.entity_id != null && ` #${a.entity_id}`}
                      </td>
                      <td data-label={t('audit.detail')} className="small">
                        {a.detail}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
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
        </>
      )}
    </Page>
  );
}
