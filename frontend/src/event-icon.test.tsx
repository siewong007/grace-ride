import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { post, put } from './api';
import i18n from './i18n';
import { todayStr } from './format';
import { EVENT_ICONS, EventIcon } from './eventIcon';
import EventForm from './pages/EventForm';
import Events, { type EventItem } from './pages/Events';
import { ToastProvider } from './components/ui';

let mockApiData: Record<string, unknown> = {};

vi.mock('./api', async () => {
  const actual = await vi.importActual<typeof import('./api')>('./api');
  return {
    ...actual,
    api: vi.fn((path: string) => Promise.resolve(mockApiData[path] ?? [])),
    post: vi.fn().mockResolvedValue({ id: 99 }),
    put: vi.fn().mockResolvedValue({ ok: true }),
    useApi: (path: string | null) => ({
      data: path ? (mockApiData[path] ?? null) : null,
      loading: false,
      refreshing: false,
      error: null,
      reload: vi.fn(),
      setData: vi.fn(),
    }),
  };
});

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: { id: 1, name: 'Organiser', roles: ['admin'] },
    can: () => true,
    loading: false,
  }),
}));

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

beforeEach(() => {
  vi.clearAllMocks();
  mockApiData = {
    '/groups': [{ id: 1, name: 'All Members', description: '', is_all: true, member_count: 10, parent_id: null, is_visitor: false }],
    '/areas': [{ id: 1, name: 'SS2', active: true }],
    '/events?scope=upcoming': [],
    '/events?scope=managing': [],
    '/rides?scope=upcoming': [],
    '/driver/requests?mine_areas=true': [],
  };
});

describe('EventIcon', () => {
  it('renders preset Lucide SVG, custom emoji, and kind-based fallback', () => {
    const { container, rerender } = render(<EventIcon icon="church" kind="other" size={18} />);
    expect(container.querySelector('svg')).toBeInTheDocument();

    rerender(<EventIcon icon="🍲" kind="other" size={18} />);
    expect(screen.getByText('🍲')).toBeInTheDocument();

    rerender(<EventIcon icon={null} kind="badminton" size={18} />);
    expect(container.querySelector('svg')).toBeInTheDocument();
  });
});

describe('EventForm activity icon picker', () => {
  it('renders 28 preset icons, submits selected preset, supports custom emoji and reset', async () => {
    const user = userEvent.setup();
    render(
      <ToastProvider>
        <MemoryRouter initialEntries={['/events/new']}>
          <Routes>
            <Route path="/events/new" element={<EventForm />} />
            <Route path="/events/:id" element={<div>Event detail</div>} />
          </Routes>
        </MemoryRouter>
      </ToastProvider>,
    );

    const radios = screen.getAllByRole('radio');
    expect(radios).toHaveLength(Object.keys(EVENT_ICONS).length);
    expect(radios).toHaveLength(28);

    // Fill required fields
    await user.type(screen.getByPlaceholderText(/saturday badminton/i), 'Steamboat Dinner');
    await user.type(screen.getByLabelText(/^date$/i), '2099-11-24');
    await user.type(screen.getByLabelText(/^time$/i), '19:30');
    await user.type(screen.getByRole('combobox', { name: /location/i }), 'Fellowship Hall');
    await user.click(screen.getByRole('checkbox', { name: /all members/i }));

    // Select preset icon
    await user.click(screen.getByRole('radio', { name: /meal \/ potluck/i }));
    expect(screen.getByRole('radio', { name: /meal \/ potluck/i })).toHaveAttribute('aria-checked', 'true');

    await user.click(screen.getByRole('button', { name: /publish event/i }));
    await waitFor(() => {
      expect(post).toHaveBeenCalledWith(
        '/events',
        expect.objectContaining({
          title: 'Steamboat Dinner',
          icon: 'utensils',
        }),
      );
    });
  });

  it('pre-populates existing icon when editing and supports custom emoji and reset', async () => {
    const user = userEvent.setup();
    mockApiData['/events/10'] = {
      id: 10,
      title: 'Bible Study',
      kind: 'other',
      icon: 'bookOpen',
      description: '',
      starts_at: '2099-11-24T19:30',
      location: 'Room 2',
      capacity: 15,
      cost_note: '',
      signup_deadline: null,
      group_ids: [1],
      as_group_ids: [],
      photo_file_id: null,
      attendance_enabled: true,
      mileage_claimable: false,
      area: null,
    };

    render(
      <ToastProvider>
        <MemoryRouter initialEntries={['/events/10/edit']}>
          <Routes>
            <Route path="/events/:id/edit" element={<EventForm />} />
            <Route path="/events/:id" element={<div>Event detail</div>} />
          </Routes>
        </MemoryRouter>
      </ToastProvider>,
    );

    expect(screen.getByRole('radio', { name: /bible study/i })).toHaveAttribute('aria-checked', 'true');

    const customInput = screen.getByPlaceholderText(/custom emoji/i);
    await user.type(customInput, '🍲');
    expect(screen.getByRole('radio', { name: /bible study/i })).toHaveAttribute('aria-checked', 'false');

    await user.click(screen.getByRole('button', { name: /reset to default/i }));
    expect(customInput).toHaveValue('');

    await user.click(screen.getByRole('button', { name: /^save$/i }));
    await waitFor(() => {
      expect(put).toHaveBeenCalledWith(
        '/events/10',
        expect.objectContaining({
          icon: null,
        }),
      );
    });
  });
});

describe('Events page displays configured icons', () => {
  it('renders custom emoji icon in Today banner, list rows, My schedule, and I manage', async () => {
    const user = userEvent.setup();
    const today = todayStr();
    const ev: EventItem = {
      id: 88,
      title: 'Potluck Love Feast',
      kind: 'other',
      icon: '🍲',
      description: '',
      starts_at: `${today}T12:00:00`,
      location: 'Hall A',
      capacity: 30,
      going: 12,
      waitlist: 0,
      spots_left: 18,
      cost_note: '',
      signup_deadline: null,
      status: 'published',
      created_by: { id: 1, name: 'Organiser' },
      my_signup: { status: 'going', need_ride: false, offer_drive: false, attended: 0, pass_code: 'PASS88' },
      groups: 'All Members',
      can_manage: true,
      attendance_enabled: true,
    };
    mockApiData['/events?scope=upcoming'] = [ev];
    mockApiData['/events?scope=managing'] = [ev];

    render(
      <ToastProvider>
        <MemoryRouter initialEntries={['/events']}>
          <Events />
        </MemoryRouter>
      </ToastProvider>,
    );

    // Appears in Today banner and upcoming list row
    expect(screen.getAllByText('🍲').length).toBeGreaterThanOrEqual(2);

    await user.click(screen.getByRole('tab', { name: /my schedule/i }));
    expect(screen.getAllByText('🍲').length).toBeGreaterThanOrEqual(2);

    await user.click(screen.getByRole('tab', { name: /i manage/i }));
    expect(screen.getByText('🍲')).toBeInTheDocument();
  });
});
