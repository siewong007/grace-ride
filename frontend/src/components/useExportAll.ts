import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { api } from '../api';
import { exportToCsv } from '../format';
import { useToast } from './ui';

interface Paged<T> {
  items: T[];
  total: number;
  total_pages: number;
}

/** Most rows one export will collect, so a huge list cannot freeze the browser. */
export const EXPORT_MAX_ROWS = 5000;
const CHUNK = 200;

export interface ExportAllOptions<T> {
  /** Endpoint without a query string, e.g. `/admin/rides`. */
  path: string;
  /** The list's current filters and sort; its page and limit are replaced. */
  query: URLSearchParams;
  headers: string[];
  row: (item: T) => (string | number | null | undefined)[];
  filename: string;
}

/**
 * Download every row that matches the list's filters, not just the page on screen.
 * Pages are fetched one after another and written as one CSV.
 */
export function useExportAll<T>() {
  const { t } = useTranslation('common');
  const toast = useToast();
  const [busy, setBusy] = useState(false);

  const run = async ({ path, query, headers, row, filename }: ExportAllOptions<T>) => {
    setBusy(true);
    try {
      const rows: T[] = [];
      let page = 1;
      let pages = 1;
      while (page <= pages && rows.length < EXPORT_MAX_ROWS) {
        const q = new URLSearchParams(query);
        q.set('page', String(page));
        q.set('limit', String(CHUNK));
        const res = await api<Paged<T> | T[]>(`${path}?${q}`);
        if (Array.isArray(res)) {
          rows.push(...res);
          break;
        }
        rows.push(...res.items);
        pages = res.total_pages;
        page += 1;
      }
      const capped = rows.slice(0, EXPORT_MAX_ROWS);
      exportToCsv(filename, headers, capped.map(row));
      if (page <= pages) toast.info(t('exportCapped', { count: capped.length }));
      else toast.success(t('exportDone', { count: capped.length }));
    } catch {
      toast.error(t('exportFailed'));
    } finally {
      setBusy(false);
    }
  };

  return { run, busy };
}
