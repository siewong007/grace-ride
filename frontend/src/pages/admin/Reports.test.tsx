import { beforeAll, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import i18n from '../../i18n';
import { put } from '../../api';
import Reports from './Reports';

const members = [
  { id: 2, name: 'Brother Silas', email: 'silas@example.org', roles: [] },
  { id: 3, name: 'Sister Phoebe', email: 'phoebe@example.org', roles: [] },
  { id: 4, name: 'Auntie Lim', email: 'lim@example.org', roles: [] },
];
const report = {
  user: { id: 1, name: 'Mary' }, month: '2026-11', range: 'monthly', type: 'all', period_start: '2026-11-01',
  period_end: '2026-11-30', rides_driven: 1, rides_received: 0, distance_km: 5, reimbursement_cents: 300,
  events_attended: 0, by_kind: [], events: [], costs_paid_cents: 0,
  badges: [
    {
      key: 'night',
      category: 'driver',
      role_match: true,
      char: 'N',
      icon: 'moon',
      name: 'Night Owl',
      rule: 'Complete rides after 9 pm (Bronze 3 · Silver 8 · Gold 20 · Platinum 40)',
      progress: 22,
      target: 40,
      earned: true,
      tier: 'gold',
      tier_level: 3,
      next_tier: 'platinum',
      next_target: 40,
      tiers: [
        { tier: 'bronze', target: 3, reached: true },
        { tier: 'silver', target: 8, reached: true },
        { tier: 'gold', target: 20, reached: true },
        { tier: 'platinum', target: 40, reached: false },
      ],
    },
  ],
};

vi.mock('../../api', () => ({
  put: vi.fn().mockResolvedValue({ ok: true }),
  useApi: (path: string | null) => ({
    data: path?.startsWith('/admin/users') ? members : path?.startsWith('/reports/') ? report : null,
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
}));
vi.mock('../../auth', () => ({ useAuth: () => ({ me: { id: 1 }, can: () => true }) }));
vi.mock('jspdf', () => ({ default: class {} }));

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

describe('Member reports filters', () => {
  it('labels every filter and narrows the member list as you type', () => {
    render(<MemoryRouter><Reports /></MemoryRouter>);
    for (const label of ['Month', 'Range', 'Type', 'Find a member', 'Select member']) {
      expect(screen.getByLabelText(label)).toBeInTheDocument();
    }
    const picker = screen.getByLabelText('Select member');
    expect(within(picker).getAllByRole('option')).toHaveLength(4); // yourself + 3 members

    fireEvent.change(screen.getByLabelText('Find a member'), { target: { value: 'phoe' } });
    const names = within(picker).getAllByRole('option').map((o) => o.textContent);
    expect(names).toEqual(['Yourself (current user)', 'Sister Phoebe (phoebe@example.org)']);
  });

  it('allows admin to change the icon used for the achievement for the user', async () => {
    render(<MemoryRouter><Reports /></MemoryRouter>);
    const editBtn = screen.getByLabelText(/Change achievement icon - Night Owl/i);
    expect(editBtn).toBeInTheDocument();
    fireEvent.click(editBtn);

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    const trophyBtn = screen.getByRole('radio', { name: 'Trophy' });
    fireEvent.click(trophyBtn);

    const saveBtn = screen.getByRole('button', { name: 'Save' });
    fireEvent.click(saveBtn);

    await waitFor(() => {
      expect(put).toHaveBeenCalledWith('/admin/users/1/badges/night/icon', { icon: 'trophy' });
    });
  });

  it('allows admin to select an adorable animated role badge icon in the modal', async () => {
    vi.mocked(put).mockClear();
    render(<MemoryRouter><Reports /></MemoryRouter>);
    fireEvent.click(screen.getByLabelText(/Change achievement icon - Night Owl/i));

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    const trafficLightBtn = screen.getByRole('radio', { name: 'Green-Light Captain' });
    fireEvent.click(trafficLightBtn);
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => {
      expect(put).toHaveBeenCalledWith('/admin/users/1/badges/night/icon', { icon: 'badge_traffic_light' });
    });
  });

  it('displays progressive tier pills, next-tier target, and milestone steps in admin reports', () => {
    const { container } = render(<MemoryRouter><Reports /></MemoryRouter>);
    expect(container.querySelector('.badge-char.badge-tier-gold')).toBeInTheDocument();
    expect(screen.getByText('Gold')).toBeInTheDocument();
    expect(screen.getByText('Next: Platinum (22/40)')).toBeInTheDocument();
    expect(screen.getByText('Platinum 40')).toBeInTheDocument();
  });
});
