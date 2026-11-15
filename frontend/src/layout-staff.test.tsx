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

// Default: transport staff — rides, live map, help desk and reimbursement review, but no roles or audit.
const transportGranted = ['rides', 'events', 'help', 'admin_dashboard', 'admin_rides', 'admin_live', 'admin_help', 'admin_reimb_review'];
const allAdminGranted = [
  ...transportGranted,
  'admin_members', 'admin_invites', 'admin_recovery', 'admin_roles',
  'admin_reimb_pay', 'admin_reports', 'admin_announcements',
  'admin_groups', 'admin_areas', 'admin_settings', 'admin_audit',
];
let granted = [...transportGranted];
vi.mock('./auth', () => ({
  useAuth: () => ({
    me: { id: 1, name: 'Staff', email: 's@e.org', roles: ['transport'], groups: [], status: 'active', terms_accepted: true, privileged: false, mfa_ok: true, unread: 0 },
    can: (page: string) => granted.includes(page),
    refresh: vi.fn(),
  }),
}));

const renderShell = (path = '/') =>
  render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route element={<Layout />}>
          <Route path="/" element={<p>home</p>} />
          <Route path="/admin" element={<p>dashboard page</p>} />
          <Route path="/admin/rides" element={<p>rides page</p>} />
          <Route path="/admin/live" element={<p>live page</p>} />
          <Route path="/admin/members" element={<p>members page</p>} />
          <Route path="/admin/roles" element={<p>roles page</p>} />
          <Route path="/admin/reimbursements" element={<p>reimb page</p>} />
          <Route path="/admin/reports" element={<p>reports page</p>} />
          <Route path="/admin/announcements" element={<p>announcements page</p>} />
          <Route path="/admin/settings" element={<p>settings page</p>} />
          <Route path="/admin/audit" element={<p>audit page</p>} />
          <Route path="/help" element={<p>help page</p>} />
        </Route>
      </Routes>
    </MemoryRouter>,
  );

const openAdminSheet = (path = '/') => {
  renderShell(path);
  fireEvent.click(within(screen.getByRole('navigation', { name: 'Bottom navigation' })).getByRole('button', { name: /Admin/ }));
  return screen.getByRole('dialog', { name: 'Admin' });
};

describe('Staff navigation', () => {
  it('groups staff links by job, shows waiting counts and drops empty groups', () => {
    granted = [...transportGranted];
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
    granted = [...transportGranted];
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

  it('renders admin sheet rows with a chevron after any badge', () => {
    granted = [...transportGranted];
    const sheet = openAdminSheet();
    const links = within(sheet).getAllByRole('link');
    expect(links.length).toBeGreaterThan(0);
    for (const link of links) {
      const chevron = link.querySelector('.sheet-link-chevron');
      expect(chevron).not.toBeNull();
      const badge = link.querySelector('.nav-badge');
      if (badge && chevron) {
        const position = badge.compareDocumentPosition(chevron);
        expect(position & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      }
    }
  });

  it('hides admin pages the user is not allowed to open', () => {
    granted = [...transportGranted];
    const sheet = openAdminSheet();
    expect(within(sheet).getByRole('link', { name: /Transport dashboard/ })).toBeInTheDocument();
    expect(within(sheet).getByRole('link', { name: /Ride management/ })).toBeInTheDocument();
    expect(within(sheet).getByRole('link', { name: /Live map/ })).toBeInTheDocument();
    expect(within(sheet).getByRole('link', { name: /Reimbursement review/ })).toBeInTheDocument();
    expect(within(sheet).getByRole('link', { name: /Help desk/ })).toBeInTheDocument();
    expect(within(sheet).queryByRole('link', { name: /Members & invites/ })).toBeNull();
    expect(within(sheet).queryByRole('link', { name: /Roles & permissions/ })).toBeNull();
    expect(within(sheet).queryByRole('link', { name: /Member reports/ })).toBeNull();
    expect(within(sheet).queryByRole('link', { name: /Announcements/ })).toBeNull();
    expect(within(sheet).queryByRole('link', { name: /Groups & areas/ })).toBeNull();
    expect(within(sheet).queryByRole('link', { name: /Audit log/ })).toBeNull();
    expect(within(sheet).queryByRole('link', { name: /Plan transport/ })).toBeNull();
  });

  it('lists every permitted standalone admin page in the sheet', () => {
    granted = [...allAdminGranted];
    const sheet = openAdminSheet();
    const hrefs = within(sheet).getAllByRole('link').map((a) => a.getAttribute('href'));
    expect(hrefs).toEqual([
      '/admin',
      '/admin/rides',
      '/admin/live',
      '/admin/members',
      '/admin/roles',
      '/admin/reimbursements',
      '/admin/reports',
      '/admin/announcements',
      '/help',
      '/admin/settings',
      '/admin/audit',
    ]);
  });

  it('marks the current admin page as active in the sheet', () => {
    granted = [...allAdminGranted];
    const sheet = openAdminSheet('/admin/rides');
    expect(within(sheet).getByRole('link', { name: /Ride management/ })).toHaveClass('active');
    expect(within(sheet).getByRole('link', { name: /Transport dashboard/ })).not.toHaveClass('active');
    expect(within(sheet).getByRole('link', { name: /Live map/ })).not.toHaveClass('active');
  });

  it('summarises waiting work in the sheet header when counts exist', () => {
    granted = [...transportGranted];
    const sheet = openAdminSheet();
    expect(sheet).toHaveAccessibleDescription('10 waiting');
  });
});
