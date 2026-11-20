import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import i18n from '../../i18n';
import AdminReimbursements from './Reimbursements';

const mocks = vi.hoisted(() => ({ post: vi.fn(), me: { id: 1 } as { id: number }, items: [] as unknown[] }));

// A traffic lead's special reimbursement for a visitor: no receipt, a reason, paid to the driver.
const claim = {
  id: 8, user: { id: 30, name: 'Driver Grace' }, ride_id: null, ride_at: null, kind: 'visitor_special',
  event_id: 5, event_title: 'Sunday Service', event_starts_at: '2026-11-22T09:00:00', amount_cents: 3000,
  description: '[Visitor special] Mr Tan · Detour to Klang', receipt_file_id: null,
  payee_name: 'Driver Grace', payee_phone: '0123456789', status: 'pending', review_note: '', reviewer: null,
  payee_verified: false, paid_ref: null, paid_at: null, created_at: '2026-11-20T08:00:00',
  visitor: { id: 4, name: 'Mr Tan' }, special_reason: 'Detour to Klang to fetch him',
  requested_by: { id: 2, name: 'Head Wei' },
};

vi.mock('../../api', () => ({
  post: mocks.post,
  useApi: () => ({
    data: { items: mocks.items, total: 1, page: 1, page_size: 25, total_pages: 1 },
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
}));

vi.mock('../../auth', () => ({
  useAuth: () => ({ can: (p: string) => p === 'admin_reimb_review', me: mocks.me }),
}));

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

beforeEach(() => {
  mocks.post.mockReset().mockResolvedValue({});
  mocks.me = { id: 1 };
  mocks.items = [claim];
});

const show = () =>
  render(
    <MemoryRouter>
      <AdminReimbursements />
    </MemoryRouter>,
  );

describe('Visitor special reimbursements in review', () => {
  it('is marked, with the visitor, the reason and who asked, and needs no receipt', async () => {
    show();
    expect(screen.getAllByText('Visitor special').length).toBeGreaterThan(0);
    expect(screen.getAllByText('For visitor Mr Tan').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Detour to Klang to fetch him').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Asked for by Head Wei').length).toBeGreaterThan(0);

    fireEvent.click(screen.getAllByRole('button', { name: 'Review' })[0]);
    const dialog = screen.getByRole('dialog');
    expect(within(dialog).queryByRole('img')).not.toBeInTheDocument();
    expect(within(dialog).getByText(/No receipt: asked for by a traffic lead/)).toBeInTheDocument();
    expect(within(dialog).getByText('Detour to Klang to fetch him')).toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole('button', { name: 'Approve' }));
    await waitFor(() =>
      expect(mocks.post).toHaveBeenCalledWith('/reimbursements/8/review', { action: 'approve', note: '' }),
    );
  });

  it('flags a high amount (over RM 100) to reviewers without blocking approval', async () => {
    mocks.items = [{ ...claim, amount_cents: 15_000, high_amount: true }];
    show();
    expect(screen.getAllByText('High amount (over RM 100)').length).toBeGreaterThan(0);
    fireEvent.click(screen.getAllByRole('button', { name: 'Review' })[0]);
    const dialog = screen.getByRole('dialog');
    expect(within(dialog).getByText(/High amount: over RM 100/)).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: 'Approve' })).toBeEnabled();
  });

  it('does not flag RM 100 or less', () => {
    show();
    expect(screen.queryByText('High amount (over RM 100)')).not.toBeInTheDocument();
  });

  it('is not offered for review to whoever asked for it', () => {
    mocks.me = { id: 2 };
    show();
    expect(screen.queryByRole('button', { name: 'Review' })).not.toBeInTheDocument();
    expect(screen.getAllByText(/You asked for this/).length).toBeGreaterThan(0);
  });
});
