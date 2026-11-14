import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import Layout from './components/Layout';
import './i18n';

vi.mock('./api', () => ({
  api: vi.fn(),
  post: vi.fn(),
  useApi: (path: string | null) => ({
    data: path === '/admin/dashboard'
      ? { pending_approval: 2, no_driver: 1, open_help: 4, pending_reimbursements: 3, pending_recovery: 0 }
      : null,
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
}));

// Transport staff: rides, live map, help desk and reimbursement review, but no roles or audit.
const granted = ['rides', 'events', 'help', 'admin_dashboard', 'admin_rides', 'admin_live', 'admin_help', 'admin_reimb_review'];
vi.mock('./auth', () => ({
  useAuth: () => ({
    me: { id: 1, name: 'Staff', email: 's@e.org', roles: ['transport'], groups: [], status: 'active', terms_accepted: true, privileged: false, mfa_ok: true, unread: 0 },
    can: (page: string) => granted.includes(page),
    refresh: vi.fn(),
  }),
}));

const renderShell = () =>
  render(
    <MemoryRouter initialEntries={['/']}>
      <Routes>
        <Route element={<Layout />}>
          <Route path="/" element={<p>home</p>} />
          <Route path="/admin/rides" element={<p>rides page</p>} />
        </Route>
      </Routes>
    </MemoryRouter>,
  );

describe('Staff navigation', () => {
  it('groups staff links by job, shows waiting counts and drops empty groups', () => {
    renderShell();
    const sidebar = screen.getByRole('navigation', { name: 'Main navigation' });
    const ops = within(sidebar).getByRole('group', { name: 'Operations' });
    expect(within(ops).getByRole('link', { name: /Ride management/ })).toHaveTextContent('3');
    expect(within(ops).getByRole('link', { name: /Ride management/ })).toHaveAccessibleName(/3 waiting/);
    expect(within(sidebar).getByRole('group', { name: 'Content' })).toHaveTextContent('Help desk');
    expect(within(sidebar).queryByRole('group', { name: 'System' })).toBeNull();
    // The member "Help centre" link gives way to "Help desk" for help-desk staff.
    expect(within(sidebar).queryByRole('link', { name: 'Help centre' })).toBeNull();
  });

  it('opens every staff page from the phone Admin tab and closes on navigation', () => {
    renderShell();
    const bottom = screen.getByRole('navigation', { name: 'Bottom navigation' });
    const adminTab = within(bottom).getByRole('button', { name: /Admin/ });
    expect(adminTab).toHaveAccessibleName(/10 waiting/);
    fireEvent.click(adminTab);
    const sheet = screen.getByRole('dialog', { name: 'Admin' });
    fireEvent.click(within(sheet).getByRole('link', { name: /Ride management/ }));
    expect(screen.getByText('rides page')).toBeInTheDocument();
    expect(screen.queryByRole('dialog')).toBeNull();
  });
});
