import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { post, put } from '../api';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { SettlementForm, SettlementDetail, equalSplit } from './Settlements';

vi.mock('../auth', () => ({
  useAuth: () => ({ me: { id: 1 }, can: () => false }),
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

vi.mock('../components/ui', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../components/ui')>();
  return {
    ...actual,
    // Like the real one: a failed action is caught and shown, not thrown.
    useAction: () => ({
      busy: false,
      error: null,
      run: async (fn: () => Promise<void>) => {
        try {
          await fn();
          return true;
        } catch {
          return false;
        }
      },
    }),
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
  };
});

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

  const renderNew = () =>
    render(
      <MemoryRouter initialEntries={['/events/1/settle']}>
        <Routes>
          <Route path="/events/:eventId/settle" element={<SettlementForm />} />
          <Route path="/settlements/:id" element={<div>bill page</div>} />
        </Routes>
      </MemoryRouter>
    );

  const fillBill = () => {
    fireEvent.change(screen.getByPlaceholderText('titlePlaceholder'), { target: { value: 'Lunch' } });
    fireEvent.change(screen.getByPlaceholderText('amount'), { target: { value: '100' } });
    const [payeeName, payeeAccount] = [screen.getByText('payeeLabel'), screen.getByText('payeeAccountLabel')].map(
      (l) => l.querySelector('input') as HTMLInputElement,
    );
    fireEvent.change(payeeName, { target: { value: 'Alice' } });
    fireEvent.change(payeeAccount, { target: { value: '0123456789' } });
    for (const box of screen.getAllByRole('checkbox')) fireEvent.click(box);
  };

  it('creates the bill under its event, then publishes it', async () => {
    vi.mocked(post).mockImplementation(async (path: string) => (path.endsWith('/settlements') ? { id: 55 } : { ok: true }));
    renderNew();
    fillBill();
    fireEvent.click(screen.getByText('publishBill'));

    await waitFor(() => expect(screen.getByText('bill page')).toBeDefined());
    expect(vi.mocked(post).mock.calls[0][0]).toBe('/events/1/settlements');
    const body = vi.mocked(post).mock.calls[0][1] as Record<string, unknown>;
    expect(body.participants).toEqual([{ user_id: 1 }, { user_id: 2 }]);
    expect(body.items).toEqual([{ label: 'itemLabel', amount_cents: 10000, receipt_file_id: null }]);
    expect(vi.mocked(post).mock.calls[1][0]).toBe('/settlements/55/publish');
  });

  it('saves a draft without publishing', async () => {
    vi.mocked(post).mockResolvedValue({ id: 56 });
    renderNew();
    fillBill();
    fireEvent.click(screen.getByText('saveDraft'));

    await waitFor(() => expect(screen.getByText('bill page')).toBeDefined());
    expect(vi.mocked(post)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(post).mock.calls[0][0]).toBe('/events/1/settlements');
  });

  it('a refused publish keeps the draft, and trying again updates it instead of making another', async () => {
    let refuse = true;
    vi.mocked(post).mockImplementation(async (path: string) => {
      if (path.endsWith('/publish') && refuse) throw new Error('UNATTENDED_MEMBERS');
      return path.endsWith('/settlements') ? { id: 57 } : { ok: true };
    });
    vi.mocked(put).mockResolvedValue({ ok: true });
    renderNew();
    fillBill();
    fireEvent.click(screen.getByText('publishBill'));
    await waitFor(() => expect(vi.mocked(post)).toHaveBeenCalledTimes(2));

    refuse = false;
    fireEvent.click(screen.getByText('publishBill'));
    await waitFor(() => expect(screen.getByText('bill page')).toBeDefined());
    expect(vi.mocked(post).mock.calls.filter(([p]) => p === '/events/1/settlements')).toHaveLength(1);
    expect(vi.mocked(put).mock.calls[0][0]).toBe('/settlements/57');
  });

  it('editing a published bill asks for a reason and sends it', async () => {
    mockApiData['/settlements/77'] = {
      id: 77,
      title: 'Lunch',
      status: 'published',
      split_mode: 'equal',
      subsidy_cents: 0,
      payee_name: 'Alice',
      payee_account: '0123456789',
      event: { id: 1, title: 'Dinner' },
      items: [{ id: 1, label: 'Food', amount_cents: 10000, receipt_file_id: null }],
      shares: [
        { id: 1, user_id: 1, name: 'Alice', amount_cents: 5000, status: 'unpaid' },
        { id: 2, user_id: 2, name: 'Bob', amount_cents: 5000, status: 'unpaid' },
      ],
    };
    vi.mocked(put).mockResolvedValue({ ok: true });
    render(
      <MemoryRouter initialEntries={['/settlements/77/edit']}>
        <Routes>
          <Route path="/settlements/:id/edit" element={<SettlementForm />} />
          <Route path="/settlements/:id" element={<div>bill page</div>} />
        </Routes>
      </MemoryRouter>
    );

    const save = screen.getByText('saveChanges').closest('button') as HTMLButtonElement;
    expect(save.disabled).toBe(true);
    expect(screen.queryByText('saveDraft')).toBeNull();
    const reason = screen.getByText('revisionReason').querySelector('input') as HTMLInputElement;
    fireEvent.change(reason, { target: { value: 'Added drinks' } });
    fireEvent.click(save);

    await waitFor(() => expect(screen.getByText('bill page')).toBeDefined());
    expect(vi.mocked(put).mock.calls[0][0]).toBe('/settlements/77');
    expect((vi.mocked(put).mock.calls[0][1] as Record<string, unknown>).reason).toBe('Added drinks');
    expect(vi.mocked(post)).not.toHaveBeenCalled();
  });

  it('splits equally to the cent, like the server', () => {
    expect(equalSplit(10000, 3)).toEqual([3334, 3333, 3333]);
    expect(equalSplit(16800, 6)).toEqual(Array(6).fill(2800));
    expect(equalSplit(100, 0)).toEqual([]);
  });
});

