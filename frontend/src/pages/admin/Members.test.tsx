import { beforeAll, beforeEach, expect, it, vi } from 'vitest';
import { render, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import i18n from '../../i18n';
import Members from './Members';

const mocks = vi.hoisted(() => ({ paths: [] as string[] }));

vi.mock('../../api', () => ({
  del: vi.fn(),
  post: vi.fn(),
  put: vi.fn(),
  useApi: (path: string | null) => {
    if (path) mocks.paths.push(path);
    return {
      data: path?.startsWith('/admin/users?')
        ? { items: [], total: 0, page: 1, page_size: 50, total_pages: 1 }
        : path?.startsWith('/admin/roles') ? { roles: [] } : [],
      loading: false,
      error: null,
      reload: vi.fn(),
    };
  },
}));

vi.mock('../../auth', () => ({
  useAuth: () => ({ can: () => true, me: { roles: ['superadmin'] } }),
}));

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

beforeEach(() => {
  mocks.paths = [];
});

it('always requests the paginated members response shape', async () => {
  render(<MemoryRouter><Members /></MemoryRouter>);
  await waitFor(() => {
    expect(mocks.paths.some((path) => path.includes('/admin/users?') && path.includes('page=1') && path.includes('limit=50'))).toBe(true);
  });
});
