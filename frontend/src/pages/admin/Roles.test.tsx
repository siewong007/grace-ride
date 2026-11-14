import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import i18n from '../../i18n';
import Roles from './Roles';

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

const mocks = vi.hoisted(() => ({
  put: vi.fn(),
  post: vi.fn(),
  del: vi.fn(),
  allowed: new Set<string>(),
}));

const data = {
  catalog: [
    { page: 'rides', area: 'member', actions: ['view', 'create', 'update', 'delete'] },
    { page: 'report', area: 'member', actions: ['view'] },
    { page: 'admin_announcements', area: 'transport', actions: ['view', 'create', 'delete'] },
    { page: 'admin_reimb_pay', area: 'finance', actions: ['view', 'update'] },
    { page: 'admin_roles', area: 'admin', actions: ['view', 'create', 'update', 'delete'] },
  ],
  roles: [
    {
      key: 'member', name: '', description: '', system: true, implicit: true,
      permissions: ['rides.view', 'rides.create', 'report.view'], locked: [], members: 40,
    },
    {
      key: 'superadmin', name: '', description: '', system: true, implicit: false, fixed: true,
      permissions: ['rides.view', 'rides.create', 'rides.update', 'rides.delete', 'report.view',
        'admin_announcements.view', 'admin_announcements.create', 'admin_announcements.delete',
        'admin_reimb_pay.view', 'admin_reimb_pay.update',
        'admin_roles.view', 'admin_roles.create', 'admin_roles.update', 'admin_roles.delete'],
      locked: ['rides.view', 'rides.create', 'rides.update', 'rides.delete', 'report.view',
        'admin_announcements.view', 'admin_announcements.create', 'admin_announcements.delete',
        'admin_reimb_pay.view', 'admin_reimb_pay.update',
        'admin_roles.view', 'admin_roles.create', 'admin_roles.update', 'admin_roles.delete'],
      members: 1,
    },
    {
      key: 'admin', name: '', description: '', system: true, implicit: false,
      permissions: ['admin_roles.view', 'admin_roles.update'], locked: [], members: 2,
    },
    {
      key: 'c_1', name: 'Welcome team', description: 'Posts notices', system: false, implicit: false,
      permissions: ['admin_announcements.view'], locked: [], members: 3,
    },
  ],
};

vi.mock('../../auth', () => ({
  useAuth: () => ({
    me: { id: 1 },
    refresh: vi.fn().mockResolvedValue(undefined),
    can: (page: string, act = 'view') => mocks.allowed.has(`${page}.${act}`),
  }),
}));

vi.mock('../../api', () => ({
  useApi: (path: string | null) => ({
    data: path === '/admin/roles' ? data : null,
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
  put: mocks.put,
  post: mocks.post,
  del: mocks.del,
}));

const show = () =>
  render(
    <MemoryRouter>
      <Roles />
    </MemoryRouter>,
  );

describe('Roles & permissions', () => {
  beforeEach(() => {
    mocks.put.mockReset().mockResolvedValue({ ok: true });
    mocks.post.mockReset().mockResolvedValue({ key: 'c_2' });
    mocks.allowed.clear();
    for (const a of ['view', 'create', 'update', 'delete']) mocks.allowed.add(`admin_roles.${a}`);
  });

  it('shows one tick box per action a page offers, grouped by area', () => {
    show();
    expect(screen.getByRole('heading', { level: 1, name: /roles & permissions/i })).toBeInTheDocument();
    // The member role is selected first and applies to everyone.
    expect(screen.getByText(/every approved member holds this role/i)).toBeInTheDocument();
    expect(screen.getByRole('checkbox', { name: 'My rides · View' })).toBeChecked();
    expect(screen.getByRole('checkbox', { name: 'My rides · Delete' })).not.toBeChecked();
    // "My report" can only be viewed: no create / update / delete boxes exist for it.
    expect(screen.getByRole('checkbox', { name: 'My report · View' })).toBeInTheDocument();
    expect(screen.queryByRole('checkbox', { name: 'My report · Delete' })).toBeNull();
    expect(screen.getByText('Finance')).toBeInTheDocument();
    expect(screen.getByText(/nobody can give these to themselves/i)).toBeInTheDocument();
  });

  it('saves exactly the ticked permissions for the selected role', async () => {
    show();
    fireEvent.click(screen.getByRole('button', { name: /welcome team/i }));
    expect(screen.getByRole('checkbox', { name: 'Announcements · View' })).toBeChecked();
    fireEvent.click(screen.getByRole('checkbox', { name: 'Announcements · Create' }));
    fireEvent.click(screen.getByRole('button', { name: /save permissions/i }));

    await waitFor(() => expect(mocks.put).toHaveBeenCalledTimes(1));
    const [path, body] = mocks.put.mock.calls[0];
    expect(path).toBe('/admin/roles/c_1');
    expect(body.name).toBe('Welcome team');
    expect([...body.permissions].sort()).toEqual(['admin_announcements.create', 'admin_announcements.view']);
  });

  it('shows the super admin as fixed: everything ticked, nothing to change', () => {
    show();
    fireEvent.click(screen.getByRole('button', { name: /^super admin/i }));
    expect(screen.getByText(/always holds every permission on every page/i)).toBeInTheDocument();
    const boxes = screen.getAllByRole('checkbox');
    expect(boxes).toHaveLength(14);
    for (const box of boxes) {
      expect(box).toBeChecked();
      expect(box).toBeDisabled();
    }
    expect(screen.queryByRole('button', { name: /save permissions/i })).toBeNull();
    expect(screen.queryByRole('button', { name: /delete role/i })).toBeNull();
  });

  it('lets every permission of the admin role be changed, but not the role deleted', () => {
    show();
    fireEvent.click(screen.getByRole('button', { name: /^admin/i }));
    expect(screen.getByRole('checkbox', { name: 'Roles & permissions · Update' })).toBeEnabled();
    expect(screen.getByRole('button', { name: /save permissions/i })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /delete role/i })).toBeNull();
  });

  it('is read-only for someone who may only view roles', () => {
    mocks.allowed.clear();
    mocks.allowed.add('admin_roles.view');
    show();
    expect(screen.getByText(/you can view roles but not change them/i)).toBeInTheDocument();
    expect(screen.getByRole('checkbox', { name: 'My rides · View' })).toBeDisabled();
    expect(screen.queryByRole('button', { name: /save permissions/i })).toBeNull();
    expect(screen.queryByRole('button', { name: /new role/i })).toBeNull();
  });
});
