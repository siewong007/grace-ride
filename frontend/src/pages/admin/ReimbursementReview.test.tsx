import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import i18n from '../../i18n';
import AdminReimbursements from './Reimbursements';

const mocks = vi.hoisted(() => ({ post: vi.fn() }));

const claim = {
  id: 7, user: { id: 3, name: 'Sister Phoebe' }, ride_id: null, ride_at: null, amount_cents: 1250,
  description: 'Fuel', receipt_file_id: 9, payee_name: 'Sister Phoebe', payee_phone: '0123456789',
  status: 'pending', review_note: '', reviewer: null, payee_verified: false, paid_ref: null, paid_at: null,
  created_at: '2026-11-17T08:00:00',
};

vi.mock('../../api', () => ({
  post: mocks.post,
  useApi: () => ({
    data: { items: [claim], total: 1, page: 1, page_size: 25, total_pages: 1 },
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
}));

vi.mock('../../auth', () => ({
  useAuth: () => ({ can: (p: string) => p === 'admin_reimb_review' }),
}));

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

beforeEach(() => {
  mocks.post.mockReset().mockResolvedValue({});
});

const open = () => {
  render(
    <MemoryRouter>
      <AdminReimbursements />
    </MemoryRouter>,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Review' }));
  return screen.getByRole('dialog');
};

describe('Reimbursement review dialog', () => {
  it('needs a note before a claim can be returned or rejected, but not approved', async () => {
    const dialog = open();
    expect(within(dialog).getByRole('button', { name: 'Approve' })).toBeEnabled();
    expect(within(dialog).getByRole('button', { name: 'Return for info' })).toBeDisabled();
    expect(within(dialog).getByRole('button', { name: 'Reject' })).toBeDisabled();
    expect(within(dialog).getByText(/write a note to enable return and reject/i)).toBeInTheDocument();

    fireEvent.change(within(dialog).getByRole('textbox'), { target: { value: 'Receipt is blurry' } });
    expect(within(dialog).getByRole('button', { name: 'Return for info' })).toBeEnabled();
    expect(within(dialog).getByRole('button', { name: 'Reject' })).toBeEnabled();

    fireEvent.click(within(dialog).getByRole('button', { name: 'Reject' }));
    await waitFor(() =>
      expect(mocks.post).toHaveBeenCalledWith('/reimbursements/7/review', { action: 'reject', note: 'Receipt is blurry' }),
    );
  });

  it('treats a note of only spaces as no note', () => {
    const dialog = open();
    fireEvent.change(within(dialog).getByRole('textbox'), { target: { value: '   ' } });
    expect(within(dialog).getByRole('button', { name: 'Reject' })).toBeDisabled();
  });
});
