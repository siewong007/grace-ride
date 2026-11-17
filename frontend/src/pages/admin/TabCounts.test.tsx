import { beforeAll, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import i18n from '../../i18n';
import Members from './Members';
import AdminReimbursements from './Reimbursements';

const counts = { pending_approval: 0, no_driver: 0, open_help: 0, pending_reimbursements: 4, pending_recovery: 2, open_invites: 3, awaiting_payment: 5 };

vi.mock('../../api', () => ({
  del: vi.fn(), post: vi.fn(), put: vi.fn(),
  useApi: (path: string | null) => ({
    data: path === '/admin/dashboard' ? counts
      : path?.startsWith('/admin/users?') || path?.startsWith('/reimbursements?') ? { items: [], total: 0, page: 1, page_size: 25, total_pages: 1 }
      : path?.startsWith('/admin/roles') ? { roles: [] } : [],
    loading: false, error: null, reload: vi.fn(),
  }),
}));
vi.mock('../../auth', () => ({
  useAuth: () => ({ can: () => true, me: { status: 'active', roles: ['superadmin'] } }),
}));

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

describe('Waiting counts on tabs', () => {
  it('Members: invites and account recovery', () => {
    render(<MemoryRouter><Members /></MemoryRouter>);
    expect(screen.getByRole('tab', { name: /Invites.*3/ })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /Account recovery.*2/ })).toBeInTheDocument();
  });

  it('Reimbursements: claims to review and claims to pay', () => {
    render(<MemoryRouter><AdminReimbursements /></MemoryRouter>);
    expect(screen.getByRole('tab', { name: /Transport review.*4/ })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /Finance payout.*5/ })).toBeInTheDocument();
  });
});
