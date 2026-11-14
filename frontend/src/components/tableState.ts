import { useSearchParams } from 'react-router-dom';
import type { TableSort } from './ui';

export interface UseTableParamsOptions {
  defaultPageSize?: number;
  defaultSort?: TableSort | null;
  filterKeys?: string[];
}

export interface TableParams {
  page: number;
  pageSize: number;
  sort: TableSort | null;
  filters: Record<string, string>;
  hasFilters: boolean;
  queryString: string;
  setPage: (page: number) => void;
  setPageSize: (pageSize: number) => void;
  setSort: (sort: TableSort | null) => void;
  setFilter: (key: string, value: string) => void;
  clearFilters: () => void;
}

export function useTableParams({
  defaultPageSize = 25,
  defaultSort = null,
  filterKeys = [],
}: UseTableParamsOptions = {}): TableParams {
  const [params, setParams] = useSearchParams();

  const page = Math.max(1, Number(params.get('page') ?? '1') || 1);
  const pageSize = Math.max(
    1,
    Number(params.get('limit') ?? String(defaultPageSize)) || defaultPageSize,
  );
  const sortKey = params.get('sort');
  const sort: TableSort | null = sortKey
    ? { key: sortKey, dir: params.get('dir') === 'desc' ? 'desc' : 'asc' }
    : defaultSort;

  const filters: Record<string, string> = {};
  for (const key of filterKeys) filters[key] = params.get(key) ?? '';
  const hasFilters = filterKeys.some((key) => (params.get(key) ?? '') !== '');

  const update = (mutate: (next: URLSearchParams) => void) => {
    const next = new URLSearchParams(params);
    mutate(next);
    setParams(next, { replace: true });
  };

  const setPage = (value: number) => {
    update((next) => {
      if (value > 1) next.set('page', String(value));
      else next.delete('page');
    });
  };

  const setPageSize = (value: number) => {
    update((next) => {
      if (value === defaultPageSize) next.delete('limit');
      else next.set('limit', String(value));
      next.delete('page');
    });
  };

  const setSort = (value: TableSort | null) => {
    update((next) => {
      next.delete('page');
      if (
        !value ||
        (defaultSort &&
          value.key === defaultSort.key &&
          value.dir === defaultSort.dir)
      ) {
        next.delete('sort');
        next.delete('dir');
      } else {
        next.set('sort', value.key);
        next.set('dir', value.dir);
      }
    });
  };

  const setFilter = (key: string, value: string) => {
    update((next) => {
      if (value) next.set(key, value);
      else next.delete(key);
      next.delete('page');
    });
  };

  const clearFilters = () => {
    update((next) => {
      for (const key of filterKeys) next.delete(key);
      next.delete('sort');
      next.delete('dir');
      next.delete('page');
    });
  };

  const query = new URLSearchParams();
  if (page > 1) query.set('page', String(page));
  if (pageSize !== defaultPageSize) query.set('limit', String(pageSize));
  if (
    sort &&
    (!defaultSort ||
      sort.key !== defaultSort.key ||
      sort.dir !== defaultSort.dir)
  ) {
    query.set('sort', sort.key);
    query.set('dir', sort.dir);
  }
  for (const key of filterKeys) {
    const value = params.get(key);
    if (value) query.set(key, value);
  }

  return {
    page,
    pageSize,
    sort,
    filters,
    hasFilters,
    queryString: query.toString(),
    setPage,
    setPageSize,
    setSort,
    setFilter,
    clearFilters,
  };
}
