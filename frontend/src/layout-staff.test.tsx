import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import Layout from './components/Layout';
import AdminMenu from './pages/admin/Menu';
import './i18n';

vi.mock('./api', () => ({
  api: vi.fn(),
  post: vi.fn(),
  useApi: (path: string | null) => ({
    data: path === '/admin/dashboard'
      ? { pending_approval: 2, no_driver: 1, open_help: 4, pending_reimbursements: 3, pending_recovery: 0, pending_members: 2 }
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
          <Route path="/admin/menu" element={<AdminMenu />} />
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

const openAdminMenu = (path = '/') => {
  const result = renderShell(path);
  if (path !== '/admin/menu') {
    fireEvent.click(within(screen.getByRole('navigation', { name: 'Bottom navigation' })).getByRole('link', { name: /Admin/ }));
  }
  return { ...result, page: screen.getByRole('heading', { name: 'Admin' }).closest('.profile-page') as HTMLElement };
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

  it('opens the admin menu from the phone Admin tab and navigates from a row', () => {
    granted = [...transportGranted];
    renderShell();
    const bottom = screen.getByRole('navigation', { name: 'Bottom navigation' });
    const adminTab = within(bottom).getByRole('link', { name: /Admin/ });
    expect(adminTab).toHaveAttribute('href', '/admin/menu');
    expect(adminTab).toHaveAccessibleName(/10 waiting/);
    fireEvent.click(adminTab);
    expect(screen.queryByRole('dialog')).toBeNull();
    const heading = screen.getByRole('heading', { name: 'Admin' });
    const page = heading.closest('.profile-page') as HTMLElement;
    fireEvent.click(within(page).getByRole('link', { name: /Ride management/ }));
    expect(screen.getByText('rides page')).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Admin' })).toBeNull();
  });

  it('renders admin menu rows with a chevron after any badge', () => {
    granted = [...transportGranted];
    const { page } = openAdminMenu('/admin/menu');
    const links = within(page).getAllByRole('link');
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
    const { page } = openAdminMenu('/admin/menu');
    expect(within(page).getByRole('link', { name: /Transport dashboard/ })).toBeInTheDocument();
    expect(within(page).getByRole('link', { name: /Ride management/ })).toBeInTheDocument();
    expect(within(page).getByRole('link', { name: /Live map/ })).toBeInTheDocument();
    expect(within(page).getByRole('link', { name: /Reimbursement review/ })).toBeInTheDocument();
    expect(within(page).getByRole('link', { name: /Help desk/ })).toBeInTheDocument();
    expect(within(page).queryByRole('link', { name: /Members & invites/ })).toBeNull();
    expect(within(page).queryByRole('link', { name: /Roles & permissions/ })).toBeNull();
    expect(within(page).queryByRole('link', { name: /Member reports/ })).toBeNull();
    expect(within(page).queryByRole('link', { name: /Announcements/ })).toBeNull();
    expect(within(page).queryByRole('link', { name: /Settings/ })).toBeNull();
    expect(within(page).queryByRole('link', { name: /Audit log/ })).toBeNull();
    expect(within(page).queryByRole('link', { name: /Plan transport/ })).toBeNull();
  });

  it('lists every permitted standalone admin page on the menu', () => {
    granted = [...allAdminGranted];
    const { page } = openAdminMenu('/admin/menu');
    const hrefs = within(page).getAllByRole('link').map((a) => a.getAttribute('href'));
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

  it('marks the Admin tab active on the menu and on other admin pages', () => {
    granted = [...allAdminGranted];
    const activeTab = (path: string) => {
      const { unmount } = renderShell(path);
      const bottom = screen.getByRole('navigation', { name: 'Bottom navigation' });
      const tab = within(bottom).getByRole('link', { name: /Admin/ });
      expect(tab).toHaveClass('active');
      unmount();
    };
    activeTab('/admin/menu');
    activeTab('/admin/rides');
    activeTab('/admin');
    activeTab('/help');
  });

  it('does not highlight a destination row while on the admin menu', () => {
    granted = [...allAdminGranted];
    const { page } = openAdminMenu('/admin/menu');
    for (const link of within(page).getAllByRole('link')) {
      expect(link).not.toHaveClass('active');
    }
  });

  it('counts members waiting for approval on the Members link, only for member admins', () => {
    granted = [...allAdminGranted];
    const { page, unmount } = openAdminMenu('/admin/menu');
    expect(within(page).getByRole('link', { name: /Members/ })).toHaveAccessibleName(/2 waiting/);
    unmount();
    granted = [...transportGranted, 'admin_recovery'];
    const second = openAdminMenu('/admin/menu');
    expect(within(second.page).getByRole('link', { name: /Members/ })).not.toHaveAccessibleName(/waiting/);
  });

  it('names the page for assistive tech and keeps waiting counts on badges only', () => {
    granted = [...transportGranted];
    const { page } = openAdminMenu('/admin/menu');
    const heading = screen.getByRole('heading', { name: 'Admin' });
    expect(heading).toHaveClass('sr-only');
    expect(page.querySelector('.profile-identity')).toBeNull();
    expect(within(page).queryByText('10 waiting')).not.toBeInTheDocument();
    expect(within(page).getByRole('link', { name: /Ride management/ })).toHaveAccessibleName(/3 waiting/);
    const bottom = screen.getByRole('navigation', { name: 'Bottom navigation' });
    expect(within(bottom).getByRole('link', { name: /Admin/ })).toHaveAccessibleName(/10 waiting/);
  });
});
