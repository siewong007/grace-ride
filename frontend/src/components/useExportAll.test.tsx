import { beforeEach, describe, expect, it, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useExportAll } from './useExportAll';

const mocks = vi.hoisted(() => ({ api: vi.fn(), csv: vi.fn(), toast: { success: vi.fn(), info: vi.fn(), error: vi.fn() } }));

vi.mock('../api', () => ({ api: mocks.api }));
vi.mock('../format', () => ({ exportToCsv: mocks.csv }));
vi.mock('./ui', () => ({ useToast: () => mocks.toast }));

type Row = { id: number };
const page = (from: number, count: number, total: number, pages: number) => ({
  items: Array.from({ length: count }, (_, i) => ({ id: from + i })),
  total,
  total_pages: pages,
});
const opts = {
  path: '/admin/rides',
  query: new URLSearchParams('status=confirmed&page=3&limit=50'),
  headers: ['ID'],
  row: (r: Row) => [r.id],
  filename: 'rides',
};

describe('useExportAll', () => {
  beforeEach(() => {
    mocks.api.mockReset();
    mocks.csv.mockReset();
    Object.values(mocks.toast).forEach((f) => f.mockReset());
  });

  it('collects every page of the current filters, not just the one on screen', async () => {
    mocks.api
      .mockResolvedValueOnce(page(1, 200, 250, 2))
      .mockResolvedValueOnce(page(201, 50, 250, 2));
    const { result } = renderHook(() => useExportAll<Row>());
    await act(() => result.current.run(opts));

    const paths = mocks.api.mock.calls.map((c) => c[0] as string);
    expect(paths).toHaveLength(2);
    expect(paths[0]).toContain('status=confirmed');
    expect(paths[0]).toContain('page=1');
    expect(paths[0]).toContain('limit=200');
    expect(paths[1]).toContain('page=2');
    const [name, , rows] = mocks.csv.mock.calls[0];
    expect(name).toBe('rides');
    expect(rows).toHaveLength(250);
    expect(mocks.toast.success).toHaveBeenCalled();
  });

  it('says so when the export is cut short', async () => {
    mocks.api.mockImplementation(async (path: string) => {
      const p = Number(new URL(path, 'http://x').searchParams.get('page'));
      return page((p - 1) * 200 + 1, 200, 20000, 100);
    });
    const { result } = renderHook(() => useExportAll<Row>());
    await act(() => result.current.run(opts));
    expect(mocks.csv.mock.calls[0][2]).toHaveLength(5000);
    expect(mocks.toast.info).toHaveBeenCalled();
    expect(mocks.toast.success).not.toHaveBeenCalled();
  });

  it('reports a failure instead of writing a partial file', async () => {
    mocks.api.mockResolvedValueOnce(page(1, 200, 400, 2)).mockRejectedValueOnce(new Error('boom'));
    const { result } = renderHook(() => useExportAll<Row>());
    await act(() => result.current.run(opts));
    expect(mocks.csv).not.toHaveBeenCalled();
    expect(mocks.toast.error).toHaveBeenCalled();
    expect(result.current.busy).toBe(false);
  });
});
