import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import i18n from '../i18n';
import Help from './Help';

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

const mocks = vi.hoisted(() => ({
  allowed: new Set<string>(),
  guides: [] as unknown[],
}));

vi.mock('../auth', () => ({
  useAuth: () => ({
    me: { id: 1 },
    refresh: vi.fn().mockResolvedValue(undefined),
    can: (page: string, act = 'view') => mocks.allowed.has(`${page}.${act}`),
  }),
}));

vi.mock('../api', () => ({
  useApi: (path: string | null) => {
    let data: unknown = [];
    if (path?.startsWith('/help/guides')) {
      // Mirror the server: the list hides drafts unless include_drafts=true.
      // Detail paths are returned whole (Tasks 6–7 reuse mocks.guides).
      data =
        path === '/help/guides'
          ? (mocks.guides as { published?: boolean }[]).filter((g) => g.published)
          : mocks.guides;
    }
    return { data: path === null ? null : data, loading: false, error: null, reload: vi.fn() };
  },
  post: vi.fn(),
  put: vi.fn(),
  del: vi.fn(),
}));

beforeEach(() => {
  mocks.allowed.clear();
  mocks.guides = [
    {
      id: 1, slug: 'plan-transport', title_en: 'Plan transport', title_zh: '安排交通',
      summary_en: 'Walk through it', summary_zh: '', published: true, step_count: 3,
    },
    {
      id: 2, slug: 'draft-one', title_en: 'Draft guide', title_zh: '草稿',
      summary_en: '', summary_zh: '', published: false, step_count: 1,
    },
  ];
});

describe('Help admin guide tab', () => {
  it('hides the tab from plain members', () => {
    mocks.allowed.add('help.view');
    mocks.allowed.add('help.create');
    render(
      <MemoryRouter>
        <Help />
      </MemoryRouter>,
    );
    expect(screen.queryByRole('tab', { name: 'Admin guide' })).toBeNull();
    // The member ask form is still there.
    expect(screen.getByText('Ask a question')).toBeTruthy();
  });

  it('shows the tab to staff and lists guides when opened', async () => {
    mocks.allowed.add('help.view');
    mocks.allowed.add('admin_help.view');
    render(
      <MemoryRouter>
        <Help />
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByRole('tab', { name: 'Admin guide' }));
    expect(await screen.findByText('Plan transport')).toBeTruthy();
    expect(screen.getByText(/3 steps/)).toBeTruthy();
    // Drafts only render for editors; this user cannot update.
    expect(screen.queryByText('Draft guide')).toBeNull();
  });
});
