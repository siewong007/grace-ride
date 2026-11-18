import { ReactNode, RefObject, useEffect, useMemo, useRef, useState } from 'react';
import { ArrowUpDown, ChevronDown, ChevronUp } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from './Button';

export type SortDirection = 'asc' | 'desc';

export interface TableSort {
  key: string;
  dir: SortDirection;
}

export type TableFilter =
  | { type: 'text'; key: string; label: string; placeholder?: string }
  | {
      type: 'select';
      key: string;
      label: string;
      allLabel?: string;
      options: { value: string; label: string }[];
    }
  | { type: 'date'; key: string; label: string }
  | { type: 'checkbox'; key: string; label: string };

export interface TableColumn<T = unknown> {
  key: string;
  header: ReactNode;
  render: (item: T, rowIndex: number) => ReactNode;
  dataLabel?: string;
  headerClassName?: string;
  cellClassName?: string;
  sortable?: boolean;
  sortKey?: string;
  sortValue?: (item: T) => string | number | null | undefined;
  sortLabel?: string;
}

export interface UniversalTableProps<T> {
  columns: TableColumn<T>[];
  items: T[];
  onRowClick?: (item: T, rowIndex: number) => void;
  itemKey?: (item: T, index: number) => string | number;
  emptyContent?: ReactNode;
  isSectionRow?: (item: T, index: number) => boolean;
  className?: string;
  sort?: TableSort | null;
  defaultSort?: TableSort | null;
  onSortChange?: (sort: TableSort | null) => void;
  filters?: TableFilter[];
  filterValues?: Record<string, string>;
  onFilterChange?: (key: string, value: string) => void;
  onClearFilters?: () => void;
  clearFiltersLabel?: string;
  toolbarActions?: ReactNode;
  compactRow?: (item: T, index: number) => ReactNode;
}

function columnSortKey<T>(col: TableColumn<T>): string {
  return col.sortKey ?? col.key;
}

function defaultDataLabel<T>(col: TableColumn<T>): string | undefined {
  return col.dataLabel ?? (typeof col.header === 'string' ? col.header : undefined);
}

function sortLabel<T>(col: TableColumn<T>): string {
  return col.sortLabel ?? (typeof col.header === 'string' ? col.header : col.key);
}

function compareValues(a: unknown, b: unknown): number {
  if (typeof a === 'number' && typeof b === 'number') return a - b;
  return String(a).localeCompare(String(b), undefined, {
    numeric: true,
    sensitivity: 'base',
  });
}

const COMPACT_BELOW_PX = 860;

function useNarrowBox(ref: RefObject<HTMLElement | null>, enabled: boolean): boolean {
  const [narrow, setNarrow] = useState(false);
  useEffect(() => {
    const el = ref.current;
    if (!enabled || !el || typeof ResizeObserver === 'undefined') return;
    const obs = new ResizeObserver(([entry]) => setNarrow(entry.contentRect.width < COMPACT_BELOW_PX));
    obs.observe(el);
    return () => obs.disconnect();
  }, [ref, enabled]);
  return narrow;
}

function useNarrowScreen(): boolean {
  const [narrow, setNarrow] = useState(false);
  useEffect(() => {
    if (typeof window.matchMedia !== 'function') return;
    const mq = window.matchMedia('(max-width: 767px)');
    const apply = () => setNarrow(mq.matches);
    apply();
    mq.addEventListener?.('change', apply);
    return () => mq.removeEventListener?.('change', apply);
  }, []);
  return narrow;
}

function isInteractive(target: EventTarget | null): boolean {
  return (
    target instanceof Element &&
    Boolean(
      target.closest(
        'a,button,input,select,textarea,[role="button"],[data-row-click-ignore]',
      ),
    )
  );
}

export function UniversalTable<T>({
  columns,
  items,
  onRowClick,
  itemKey,
  emptyContent,
  isSectionRow,
  className = '',
  sort,
  defaultSort = null,
  onSortChange,
  filters,
  filterValues,
  onFilterChange,
  onClearFilters,
  clearFiltersLabel,
  toolbarActions,
  compactRow,
}: UniversalTableProps<T>) {
  const { t } = useTranslation('common');
  const blockRef = useRef<HTMLDivElement>(null);
  const narrowScreen = useNarrowScreen();
  const narrowBox = useNarrowBox(blockRef, !!compactRow);
  const showCompact = (narrowScreen || narrowBox) && !!compactRow && items.length > 0;
  const [localSort, setLocalSort] = useState<TableSort | null>(defaultSort);
  const controlled = sort !== undefined;
  const activeSort = controlled ? sort : localSort;
  const sortableColumns = columns.filter((col) => col.sortable);

  const rows = useMemo(() => {
    if (controlled || !activeSort) return items;
    const col = columns.find((c) => columnSortKey(c) === activeSort.key);
    if (!col?.sortValue) return items;
    const direction = activeSort.dir === 'asc' ? 1 : -1;
    return items
      .map((item, index) => ({ item, index }))
      .sort((a, b) => {
        const av = col.sortValue?.(a.item);
        const bv = col.sortValue?.(b.item);
        if (av == null && bv == null) return a.index - b.index;
        if (av == null) return 1;
        if (bv == null) return -1;
        const cmp = compareValues(av, bv);
        if (cmp === 0) return a.index - b.index;
        return cmp * direction;
      })
      .map(({ item }) => item);
  }, [activeSort, columns, controlled, items]);

  const changeSort = (col: TableColumn<T>) => {
    const key = columnSortKey(col);
    const next: TableSort = {
      key,
      dir: activeSort?.key === key && activeSort.dir === 'asc' ? 'desc' : 'asc',
    };
    if (!controlled) setLocalSort(next);
    onSortChange?.(next);
  };

  const filtersList = filters ?? [];
  const values = filterValues ?? {};
  const hasActiveFilters = filtersList.some((f) => (values[f.key] ?? '') !== '');

  const renderFilter = (f: TableFilter) => {
    const value = values[f.key] ?? '';
    switch (f.type) {
      case 'text':
        return (
          <div key={f.key} className="table-filter">
            <input
              className="input admin-input-compact"
              type="text"
              value={value}
              placeholder={f.placeholder ?? f.label}
              aria-label={f.label}
              onChange={(e) => onFilterChange?.(f.key, e.target.value)}
            />
          </div>
        );
      case 'select':
        return (
          <div key={f.key} className="table-filter">
            <select
              className="input admin-input-compact"
              value={value}
              aria-label={f.label}
              onChange={(e) => onFilterChange?.(f.key, e.target.value)}
            >
              {!f.options.some((o) => o.value === '') && (
                <option value="">{f.allLabel ?? f.label}</option>
              )}
              {f.options.map((o) => (
                <option key={o.value} value={o.value}>
                  {o.value === '' ? (f.allLabel ?? o.label) : o.label}
                </option>
              ))}
            </select>
          </div>
        );
      case 'date':
        return (
          <div key={f.key} className="table-filter">
            <input
              className="input admin-input-compact"
              type="date"
              value={value}
              aria-label={f.label}
              onChange={(e) => onFilterChange?.(f.key, e.target.value)}
            />
          </div>
        );
      case 'checkbox':
        return (
          <div key={f.key} className="table-filter">
            <label className={`check ${value === 'true' ? 'on' : ''}`}>
              <input
                type="checkbox"
                checked={value === 'true'}
                aria-label={f.label}
                onChange={(e) => onFilterChange?.(f.key, e.target.checked ? 'true' : '')}
              />
              {f.label}
            </label>
          </div>
        );
    }
  };

  const tableClassName = onRowClick
    ? 'table responsive sticky-header row-clickable'
    : 'table responsive sticky-header';

  return (
    <div ref={blockRef} className={`table-block${className ? ` ${className}` : ''}`}>
      {sortableColumns.length > 0 && (
        <label className="table-mobile-sort">
          <span>{t('table.sortBy')}</span>
          <select
            className="input admin-input-compact"
            value={activeSort ? `${activeSort.key}:${activeSort.dir}` : ''}
            aria-label={t('table.sortBy')}
            onChange={(e) => {
              const raw = e.target.value;
              if (!raw) {
                if (!controlled) setLocalSort(defaultSort);
                onSortChange?.(null);
                return;
              }
              const separator = raw.lastIndexOf(':');
              const key = raw.slice(0, separator);
              const dir = raw.slice(separator + 1);
              const next: TableSort = { key, dir: dir === 'desc' ? 'desc' : 'asc' };
              if (!controlled) setLocalSort(next);
              onSortChange?.(next);
            }}
          >
            <option value="">{t('table.defaultOrder')}</option>
            {sortableColumns.map((col) => {
              const key = columnSortKey(col);
              const label = sortLabel(col);
              return (
                <optgroup key={key} label={label}>
                  <option value={`${key}:asc`}>{label} ↑</option>
                  <option value={`${key}:desc`}>{label} ↓</option>
                </optgroup>
              );
            })}
          </select>
        </label>
      )}

      {(filtersList.length > 0 || toolbarActions) && (
        <div className="table-toolbar">
          {filtersList.map(renderFilter)}
          {hasActiveFilters && onClearFilters && (
            <Button variant="ghost" size="sm" onClick={onClearFilters}>
              {clearFiltersLabel ?? t('table.clearFilters')}
            </Button>
          )}
          {toolbarActions && <div className="table-toolbar-actions">{toolbarActions}</div>}
        </div>
      )}

      {showCompact ? (
        <div className="list table-compact">
          {rows.map((item, rowIndex) => {
            const key = itemKey ? itemKey(item, rowIndex) : rowIndex;
            const body = compactRow?.(item, rowIndex);
            if (onRowClick) {
              return (
                <button
                  key={key}
                  type="button"
                  className="item"
                  onClick={() => onRowClick(item, rowIndex)}
                >
                  {body}
                </button>
              );
            }
            return (
              <div key={key} className="item">
                {body}
              </div>
            );
          })}
        </div>
      ) : (
        <div className="table-wrap">
          <table className={tableClassName}>
            <thead>
              <tr>
                {columns.map((col) => {
                  const sorted = col.sortable && activeSort?.key === columnSortKey(col);
                  const direction = sorted ? activeSort?.dir : undefined;
                  return (
                    <th
                      key={col.key}
                      scope="col"
                      className={col.headerClassName}
                      aria-sort={
                        col.sortable
                          ? sorted
                            ? direction === 'asc'
                              ? 'ascending'
                              : 'descending'
                            : 'none'
                          : undefined
                      }
                    >
                      {col.sortable ? (
                        <button
                          type="button"
                          className="th-sort"
                          onClick={() => changeSort(col)}
                          aria-label={`${t('table.sortBy')} ${sortLabel(col)}`}
                        >
                          <span>{col.header}</span>
                          {direction === 'asc' ? (
                            <ChevronUp size={14} aria-hidden="true" />
                          ) : direction === 'desc' ? (
                            <ChevronDown size={14} aria-hidden="true" />
                          ) : (
                            <ArrowUpDown size={14} aria-hidden="true" className="sort-indicator" />
                          )}
                        </button>
                      ) : (
                        col.header
                      )}
                    </th>
                  );
                })}
              </tr>
            </thead>
            <tbody>
              {rows.map((item, rowIndex) => {
                const key = itemKey ? itemKey(item, rowIndex) : rowIndex;
                const isSection = isSectionRow?.(item, rowIndex) ?? false;
                return (
                  <tr
                    key={key}
                    className={onRowClick ? 'clickable' : ''}
                    tabIndex={onRowClick ? 0 : undefined}
                    onClick={(e) => {
                      if (isInteractive(e.target)) return;
                      onRowClick?.(item, rowIndex);
                    }}
                    onKeyDown={(e) => {
                      if (e.target !== e.currentTarget) return;
                      if (onRowClick && (e.key === 'Enter' || e.key === ' ')) {
                        e.preventDefault();
                        onRowClick?.(item, rowIndex);
                      }
                    }}
                  >
                    {isSection ? (
                      <td key={`section-${key}`} colSpan={columns.length}>
                        {columns[0]?.render?.(item, rowIndex)}
                      </td>
                    ) : (
                      columns.map((col) => (
                        <td
                          key={col.key}
                          data-label={defaultDataLabel(col)}
                          className={col.cellClassName}
                        >
                          {col.render(item, rowIndex)}
                        </td>
                      ))
                    )}
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {items.length === 0 && emptyContent != null && <div>{emptyContent}</div>}
    </div>
  );
}
