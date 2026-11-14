import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import i18n from '../i18n';
import Help from './Help';
import HelpDetail from './HelpDetail';
import HelpGuideDetail from './HelpGuideDetail';
import HelpGuideEditor from './HelpGuideEditor';

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

const mocks = vi.hoisted(() => ({
  allowed: new Set<string>(),
  guides: [] as unknown[],
  tickets: [] as unknown[],
  counts: { open: 1, answered: 1, closed: 1 },
}));

vi.mock('../auth', () => ({
  useAuth: () => ({
    me: { id: 1, name: 'Grace Member', email: 'member@example.org' },
    refresh: vi.fn().mockResolvedValue(undefined),
    can: (page: string, act = 'view') => mocks.allowed.has(`${page}.${act}`),
  }),
}));

vi.mock('../api', () => ({
  useApi: (path: string | null) => {
    let data: unknown = [];
    if (!path) {
      data = null;
    } else if (path.startsWith('/help/counts')) {
      data = mocks.counts;
    } else if (path === '/help/guides' || path.startsWith('/help/guides?')) {
      data = path.includes('include_drafts=true')
        ? mocks.guides
        : (mocks.guides as { published?: boolean }[]).filter((g) => g.published);
    } else if (path.startsWith('/help/guides/')) {
      data = (mocks.guides as unknown[])[0];
    } else if (path === '/help' || path.startsWith('/help?')) {
      data = mocks.tickets;
    } else if (path.startsWith('/help/')) {
      data = (mocks.tickets as unknown[])[0];
    }
    return { data, loading: false, error: null, reload: vi.fn() };
  },
  post: vi.fn().mockResolvedValue({ id: 99 }),
  put: vi.fn().mockResolvedValue({ id: 99 }),
  del: vi.fn().mockResolvedValue({}),
}));

beforeEach(() => {
  localStorage.clear();
  mocks.allowed.clear();
  mocks.guides = [
    {
      id: 1,
      slug: 'plan-transport',
      title_en: 'Plan transport',
      title_zh: 'Plan transport ZH',
      summary_en: 'Walk through planning an event',
      summary_zh: 'Event planning ZH',
      published: true,
      audience: 'staff',
      step_count: 2,
      steps: [
        {
          title_en: 'Open events page',
          title_zh: 'Open events ZH',
          body_en: 'Navigate to events list',
          body_zh: 'Navigate to events list ZH',
          link: '/events',
        },
        {
          title_en: 'Assign driver',
          title_zh: 'Assign driver ZH',
          body_en: 'Pick an available volunteer',
          body_zh: 'Pick volunteer ZH',
          link: '/admin/rides',
        },
      ],
    },
    {
      id: 2,
      slug: 'draft-guide',
      title_en: 'Draft unreleased guide',
      title_zh: 'Draft guide ZH',
      summary_en: 'Internal notes',
      summary_zh: 'Internal notes ZH',
      published: false,
      audience: 'staff',
      step_count: 1,
      steps: [{ title_en: 'Step 1', title_zh: 'Step 1 ZH', body_en: '', body_zh: '', link: '' }],
    },
    {
      id: 3,
      slug: 'getting-started',
      title_en: 'Getting started',
      title_zh: 'Getting started ZH',
      summary_en: 'Set up your church ride account',
      summary_zh: 'Set up account ZH',
      published: true,
      audience: 'member',
      step_count: 1,
      steps: [
        {
          title_en: 'Check your profile',
          title_zh: 'Check profile ZH',
          body_en: 'Make sure phone number is correct',
          body_zh: 'Make sure phone is correct ZH',
          link: '/me',
        },
      ],
    },
  ];
  mocks.tickets = [
    {
      id: 10,
      subject: 'Driver is late',
      user_id: 1,
      user_name: 'Grace Member',
      status: 'open',
      ride_id: null,
      created_at: '2026-11-14T10:00:00',
      updated_at: '2026-11-14T10:00:00',
      last_body: 'Has the driver departed yet?',
      last_staff: false,
      last_at: '2026-11-14T10:00:00',
      replies: [
        {
          id: 101,
          user_id: 1,
          name: 'Grace Member',
          body: 'Has the driver departed yet?',
          staff: false,
          created_at: '2026-11-14T10:00:00',
        },
      ],
    },
  ];
});

describe('Help hub without tabs', () => {
  it('shows members only their guides and own requests section', async () => {
    mocks.allowed.add('help.view');
    mocks.allowed.add('help.create');

    render(
      <MemoryRouter>
        <Help />
      </MemoryRouter>,
    );

    // Member sees search box, common questions, member guides, and my requests
    expect(screen.getByRole('heading', { level: 1, name: 'How can we help?' })).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/search guides and your requests/i)).toBeInTheDocument();
    expect(screen.getByText('Getting started')).toBeInTheDocument();
    // Staff guides and drafts are hidden from regular members
    expect(screen.queryByText('Plan transport')).toBeNull();
    expect(screen.queryByText('Draft unreleased guide')).toBeNull();
    expect(screen.getByText('My requests')).toBeInTheDocument();
    expect(screen.getAllByText('Driver is late').length).toBeGreaterThanOrEqual(1);
  });

  it('shows staff inbox with counts and filter chips', async () => {
    mocks.allowed.add('help.view');
    mocks.allowed.add('admin_help.view');

    render(
      <MemoryRouter>
        <Help />
      </MemoryRouter>,
    );

    expect(screen.getByRole('heading', { level: 2, name: 'Requests inbox' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /needs reply \(1\)/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /answered \(1\)/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /resolved \(1\)/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /all \(3\)/i })).toBeInTheDocument();
    // Staff can toggle audience on guides section
    expect(screen.getByRole('button', { name: 'Staff' })).toBeInTheDocument();
  });
});

describe('In-browser search', () => {
  it('instantly finds matching guides, steps, and requests', async () => {
    mocks.allowed.add('help.view');

    render(
      <MemoryRouter>
        <Help />
      </MemoryRouter>,
    );

    const searchInput = screen.getByPlaceholderText(/search guides and your requests/i);
    fireEvent.change(searchInput, { target: { value: 'account' } });

    expect(await screen.findByRole('heading', { level: 2, name: 'Search results' })).toBeInTheDocument();
    expect(screen.getByText('Getting started')).toBeInTheDocument();
    expect(screen.getByText(/set up your church ride/i)).toBeInTheDocument();
  });

  it('shows no-results empty state with contact us action', async () => {
    mocks.allowed.add('help.view');

    render(
      <MemoryRouter>
        <Help />
      </MemoryRouter>,
    );

    const searchInput = screen.getByPlaceholderText(/search guides and your requests/i);
    fireEvent.change(searchInput, { target: { value: 'nonexistenttermxyz' } });

    expect(await screen.findByText(/no results for "nonexistenttermxyz"/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Contact us' })).toBeInTheDocument();
  });
});

describe('Composer suggestions', () => {
  it('suggests matching guides when typing subject', async () => {
    mocks.allowed.add('help.view');

    render(
      <MemoryRouter>
        <Help />
      </MemoryRouter>,
    );

    // Open composer via Contact us button
    fireEvent.click(screen.getByRole('button', { name: 'Contact us' }));
    expect(await screen.findByRole('dialog')).toBeInTheDocument();

    const subjectInput = screen.getByLabelText(/subject/i);
    fireEvent.change(subjectInput, { target: { value: 'account' } });

    await waitFor(() => {
      expect(screen.getByText('These might answer it')).toBeInTheDocument();
    });
    const dialog = screen.getByRole('dialog');
    expect(within(dialog).getByRole('link', { name: /getting started/i })).toBeInTheDocument();
  });
});

describe('Request thread: "Did this solve it?" and reopen', () => {
  it('shows "Did this solve it?" when staff have replied and allows resolving', async () => {
    mocks.allowed.add('help.view');
    mocks.allowed.add('help.update');

    const answeredTicket = {
      id: 20,
      subject: 'Where is my pickup?',
      user_id: 1,
      status: 'answered',
      ride_id: null,
      replies: [
        {
          id: 1,
          user_id: 1,
          name: 'Grace Member',
          body: 'Where is the car waiting?',
          staff: false,
          created_at: '2026-11-14T09:00:00',
        },
        {
          id: 2,
          user_id: 99,
          name: 'Staff Mary',
          body: 'Driver is waiting at Gate 2: /help/guides/getting-started',
          staff: true,
          created_at: '2026-11-14T09:05:00',
        },
      ],
    };
    mocks.tickets = [answeredTicket as unknown as never];

    const { post } = await import('../api');

    render(
      <MemoryRouter initialEntries={['/help/20']}>
        <Routes>
          <Route path="/help/:id" element={<HelpDetail />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(await screen.findByText('Did this solve it?')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Yes, resolved' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'I still need help' })).toBeInTheDocument();

    // Inline guide link rendered in bubble
    expect(screen.getByRole('link', { name: '/help/guides/getting-started' })).toBeInTheDocument();

    // Click Yes, resolved
    fireEvent.click(screen.getByRole('button', { name: 'Yes, resolved' }));
    await waitFor(() => {
      expect(post).toHaveBeenCalledWith('/help/20/close');
    });
  });

  it('shows Reopen button on resolved requests', async () => {
    mocks.allowed.add('help.view');
    mocks.allowed.add('help.update');

    const closedTicket = {
      id: 30,
      subject: 'Past question',
      user_id: 1,
      status: 'closed',
      ride_id: null,
      replies: [
        {
          id: 1,
          user_id: 1,
          name: 'Grace Member',
          body: 'All good now',
          staff: false,
          created_at: '2026-11-14T09:00:00',
        },
      ],
    };
    mocks.tickets = [closedTicket as unknown as never];

    const { post } = await import('../api');

    render(
      <MemoryRouter initialEntries={['/help/30']}>
        <Routes>
          <Route path="/help/:id" element={<HelpDetail />} />
        </Routes>
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getAllByText('Resolved').length).toBeGreaterThan(0);
    });
    const reopenBtns = screen.getAllByRole('button', { name: 'Reopen' });
    expect(reopenBtns.length).toBeGreaterThan(0);

    fireEvent.click(reopenBtns[0]);
    await waitFor(() => {
      expect(post).toHaveBeenCalledWith('/help/30/reopen');
    });
  });
});

describe('HelpGuideDetail', () => {
  it('renders numbered vertical steps, feedback prompt, and draft indicator', async () => {
    mocks.allowed.add('admin_help.view');
    mocks.allowed.add('admin_help.update');

    const detail = {
      id: 1,
      slug: 'plan-transport',
      title_en: 'Plan transport',
      title_zh: 'Plan transport ZH',
      summary_en: 'Walk through it',
      summary_zh: 'Event planning ZH',
      published: false,
      audience: 'staff' as const,
      steps: [
        { title_en: 'Find the event', title_zh: '', body_en: 'Open Events', body_zh: '', link: '/events' },
        { title_en: 'Assign driver', title_zh: '', body_en: '', body_zh: '', link: '' },
      ],
    };
    mocks.guides = [detail as unknown as never];

    render(
      <MemoryRouter initialEntries={['/help/guides/plan-transport']}>
        <Routes>
          <Route path="/help/guides/:slug" element={<HelpGuideDetail />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(await screen.findByText('Find the event')).toBeInTheDocument();
    expect(screen.getByText('Assign driver')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /open/i })).toHaveAttribute('href', '/events');
    expect(screen.getByText('Was this step helpful?')).toBeInTheDocument();
    expect(screen.getByText('Start walkthrough')).toBeInTheDocument();
  });
});

describe('HelpGuideEditor', () => {
  it('creates a new guide with slug auto-generation and live preview', async () => {
    mocks.allowed.add('admin_help.view');
    mocks.allowed.add('admin_help.update');

    const { post } = await import('../api');

    render(
      <MemoryRouter initialEntries={['/help/guides/new']}>
        <Routes>
          <Route path="/help/guides/new" element={<HelpGuideEditor />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(await screen.findByRole('heading', { level: 1, name: 'New guide' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 3, name: 'Live Preview' })).toBeInTheDocument();

    const titleInput = screen.getByPlaceholderText('e.g. Plan transport for an event');
    fireEvent.change(titleInput, { target: { value: 'New Test Guide' } });

    // Slug should auto-generate
    const slugInput = screen.getByLabelText(/slug/i);
    expect(slugInput).toHaveValue('new-test-guide');

    const saveBtn = screen.getByRole('button', { name: 'Save draft' });
    fireEvent.click(saveBtn);

    await waitFor(() => {
      expect(post).toHaveBeenCalledWith(
        '/help/guides',
        expect.objectContaining({
          slug: 'new-test-guide',
          title_en: 'New Test Guide',
          published: false,
        }),
      );
    });
  });
});
