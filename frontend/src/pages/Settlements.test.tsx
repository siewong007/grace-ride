import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { SettlementForm, SettlementDetail } from './Settlements';

vi.mock('../auth', () => ({
  useAuth: () => ({ me: { id: 1 }, has: () => false }),
}));

const mockApiData: Record<string, unknown> = {
  '/events/1/signups': [
    { user_id: 1, name: 'Alice', status: 'confirmed', attended: 1 },
    { user_id: 2, name: 'Bob', status: 'confirmed', attended: 1 },
  ],
  '/events/1': { title: 'Dinner' },
};

vi.mock('../api', () => ({
  api: vi.fn(),
  post: vi.fn(),
  put: vi.fn(),
  del: vi.fn(),
  useApi: (path: string | null) => ({
    data: path ? mockApiData[path] ?? null : null,
    error: null,
    loading: false,
    reload: vi.fn(),
  }),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (k: string, opts?: Record<string, unknown>) => {
      if (opts && 'sum' in opts) return `${k}:${opts.sum}/${opts.total}`;
      return k;
    },
    i18n: { language: 'en' },
  }),
}));

vi.mock('../format', () => ({
  useFormatT: () => (k: string) => k,
  fmtAgo: () => 'recently',
  rm: (cents: number) => `RM ${(cents / 100).toFixed(2)}`,
  toCents: (val: string) => {
    const n = parseFloat(val);
    return isNaN(n) ? null : Math.round(n * 100);
  },
  SHARE_STATUS: {},
  SETTLEMENT_STATUS: {},
}));

vi.mock('../components/ui', () => ({
  useAction: () => ({ busy: false, error: null, run: (fn: () => Promise<void>) => fn() }),
  ErrorBox: () => null,
  Empty: ({ children }: { children?: unknown }) => <div>{children as string}</div>,
  Spinner: () => <div>Loading...</div>,
  StatusPill: () => <span>Pill</span>,
  Modal: ({ children }: { children?: unknown }) => <div>{children as never}</div>,
  Field: ({ label, children }: { label: string; children: unknown }) => (
    <label>
      {label}
      {children as never}
    </label>
  ),
  FileInput: () => <input type="file" />,
}));

describe('SettlementForm', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders form and handles custom split mode', () => {
    render(
      <MemoryRouter initialEntries={['/events/1/settle']}>
        <Routes>
          <Route path="/events/:eventId/settle" element={<SettlementForm />} />
        </Routes>
      </MemoryRouter>
    );

    // Initial equal split
    const splitSelect = screen.getByDisplayValue('splitEqual');
    expect(splitSelect).toBeDefined();

    // Switch to custom split
    fireEvent.change(splitSelect, { target: { value: 'custom' } });
    expect(screen.getByDisplayValue('splitCustom')).toBeDefined();
  });

  it('add and delete buttons have type="button"', () => {
    render(
      <MemoryRouter initialEntries={['/events/1/settle']}>
        <Routes>
          <Route path="/events/:eventId/settle" element={<SettlementForm />} />
        </Routes>
      </MemoryRouter>
    );

    const addBtn = screen.getByText('+ item');
    expect(addBtn.getAttribute('type')).toBe('button');
  });

  it('SettlementDetail displays paid progress and edit link when allowed', () => {
    mockApiData['/settlements/10'] = {
      id: 10,
      title: 'Lunch',
      status: 'published',
      split_mode: 'equal',
      subsidy_cents: 0,
      total_cents: 10000,
      version: 1,
      can_manage: true,
      created_by: { id: 1, name: 'Alice' },
      event: { id: 1, title: 'Fellowship' },
      items: [{ id: 1, label: 'Food', amount_cents: 10000 }],
      shares: [
        { id: 1, user_id: 1, name: 'Alice', amount_cents: 5000, status: 'confirmed' },
        { id: 2, user_id: 2, name: 'Bob', amount_cents: 5000, status: 'unpaid' },
      ],
    };

    const { container } = render(
      <MemoryRouter initialEntries={['/settlements/10']}>
        <Routes>
          <Route path="/settlements/:id" element={<SettlementDetail />} />
        </Routes>
      </MemoryRouter>
    );

    expect(screen.getByText('settlementDetail.edit')).toBeDefined();
    const bar = container.querySelector('.bar div');
    expect(bar?.getAttribute('style')).toContain('50%');
  });
});
