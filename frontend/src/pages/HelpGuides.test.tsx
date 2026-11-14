import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import i18n from '../i18n';
import Help from './Help';
import HelpGuideDetail from './HelpGuideDetail';

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
    if (path === '/help/guides' || path?.startsWith('/help/guides?')) {
      // Mirror the server: the list hides drafts unless include_drafts=true.
      data =
        path === '/help/guides'
          ? (mocks.guides as { published?: boolean }[]).filter((g) => g.published)
          : mocks.guides;
    } else if (path?.startsWith('/help/guides/')) {
      // Detail fetch: Tasks 6–7 reuse mocks.guides as the detail object.
      data = (mocks.guides as unknown[])[0];
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
      id: 1, slug: 'plan-transport', title_en: 'Plan transport', title_zh: '',
      summary_en: 'Walk through it', summary_zh: '', published: true, audience: 'staff', step_count: 3,
    },
    {
      id: 2, slug: 'draft-one', title_en: 'Draft guide', title_zh: '',
      summary_en: '', summary_zh: '', published: false, audience: 'staff', step_count: 1,
    },
    {
      id: 3, slug: 'getting-started', title_en: 'Getting started', title_zh: '',
      summary_en: 'Set up your account', summary_zh: '', published: true, audience: 'member', step_count: 2,
    },
  ];
});

describe('Help guides tab', () => {
  it('shows members only their guides', async () => {
    mocks.allowed.add('help.view');
    mocks.allowed.add('help.create');
    render(
      <MemoryRouter>
        <Help />
      </MemoryRouter>,
    );
    expect(screen.getByRole('tab', { name: 'Guides' })).toBeTruthy();
    expect(screen.getByText('Ask a question')).toBeTruthy();
    fireEvent.click(screen.getByRole('tab', { name: 'Guides' }));
    expect(await screen.findByText('Getting started')).toBeTruthy();
    expect(screen.queryByText('Plan transport')).toBeNull();
  });

  it('shows the tab to staff and lists guides when opened', async () => {
    mocks.allowed.add('help.view');
    mocks.allowed.add('admin_help.view');
    render(
      <MemoryRouter>
        <Help />
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByRole('tab', { name: 'Guides' }));
    expect(await screen.findByText('Plan transport')).toBeTruthy();
    expect(screen.getByText('For members')).toBeTruthy();
    expect(screen.getByText('For staff')).toBeTruthy();
    expect(screen.getByText(/3 steps/)).toBeTruthy();
    // Drafts only render for editors; this user cannot update.
    expect(screen.queryByText('Draft guide')).toBeNull();
  });

  it('filters the staff list by audience', async () => {
    mocks.allowed.add('help.view');
    mocks.allowed.add('admin_help.view');
    render(<MemoryRouter><Help /></MemoryRouter>);
    fireEvent.click(screen.getByRole('tab', { name: 'Guides' }));
    fireEvent.change(screen.getByLabelText('Show guides for'), { target: { value: 'member' } });
    expect(await screen.findByText('Getting started')).toBeTruthy();
    expect(screen.queryByText('Plan transport')).toBeNull();
  });
});

describe('HelpGuideDetail', () => {
  it('renders numbered steps with deep links and a draft pill', async () => {
    mocks.allowed.add('admin_help.view');
    mocks.allowed.add('admin_help.update');
    const detail = {
      id: 1,
      slug: 'plan-transport',
      title_en: 'Plan transport',
      title_zh: '',
      summary_en: 'Walk through it',
      summary_zh: '',
      published: false,
      audience: 'staff' as const,
      steps: [
        { title_en: 'Find the event', title_zh: '', body_en: 'Open Events', body_zh: '', link: '/events' },
        { title_en: 'Watch it run', title_zh: '', body_en: '', body_zh: '', link: '' },
      ],
    };
    // useApi returns mocks.guides for any /help/guides path; reuse it as the detail object.
    mocks.guides = [detail as unknown as never];
    render(
      <MemoryRouter initialEntries={['/help/guides/plan-transport']}>
        <Routes>
          <Route path="/help/guides/:slug" element={<HelpGuideDetail />} />
        </Routes>
      </MemoryRouter>,
    );
    expect(await screen.findByText('1. Find the event')).toBeTruthy();
    expect(screen.getByText('2. Watch it run')).toBeTruthy();
    expect(screen.getByRole('link', { name: 'Open →' })).toHaveProperty('href', expect.stringContaining('/events'));
    expect(screen.getByText('Draft')).toBeTruthy();
  });
});

describe('GuideForm editor', () => {
  it('creates a guide through the API with steps', async () => {
    mocks.allowed.add('admin_help.view');
    mocks.allowed.add('admin_help.update');
    const { post } = await import('../api');
    render(
      <MemoryRouter>
        <Help />
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByRole('tab', { name: 'Guides' }));
    fireEvent.click(await screen.findByRole('button', { name: 'New guide' }));
    fireEvent.change(screen.getByLabelText('Slug (lowercase letters, numbers, dashes)'), {
      target: { value: 'test-guide' },
    });
    fireEvent.change(screen.getByLabelText('Title (English)'), { target: { value: 'Test' } });
    fireEvent.change(screen.getByLabelText('Audience'), { target: { value: 'member' } });
    fireEvent.click(screen.getByRole('button', { name: 'Add step' }));
    fireEvent.change(screen.getAllByLabelText('Step title (English)')[0], { target: { value: 'One' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() =>
      expect(post).toHaveBeenCalledWith(
        '/help/guides',
        expect.objectContaining({
          slug: 'test-guide',
          title_en: 'Test',
          audience: 'member',
          steps: expect.arrayContaining([expect.objectContaining({ title_en: 'One' })]),
        }),
      ),
    );
  });

  it('hides editor controls from non-editors', async () => {
    mocks.allowed.add('admin_help.view'); // staff reader, not editor
    render(
      <MemoryRouter>
        <Help />
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByRole('tab', { name: 'Guides' }));
    await screen.findByText('Plan transport');
    expect(screen.queryByRole('button', { name: 'New guide' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Edit' })).toBeNull();
  });
});
