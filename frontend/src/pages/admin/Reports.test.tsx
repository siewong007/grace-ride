import { beforeAll, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import i18n from '../../i18n';
import Reports from './Reports';

const members = [
  { id: 2, name: 'Brother Silas', email: 'silas@example.org', roles: [] },
  { id: 3, name: 'Sister Phoebe', email: 'phoebe@example.org', roles: [] },
  { id: 4, name: 'Auntie Lim', email: 'lim@example.org', roles: [] },
];
const report = {
  user: { id: 1, name: 'Mary' }, month: '2026-11', range: 'monthly', type: 'all', period_start: '2026-11-01',
  period_end: '2026-11-30', rides_driven: 1, rides_received: 0, distance_km: 5, reimbursement_cents: 300,
  events_attended: 0, by_kind: [], events: [], costs_paid_cents: 0, badges: [],
};

vi.mock('../../api', () => ({
  useApi: (path: string | null) => ({
    data: path?.startsWith('/admin/users') ? members : path?.startsWith('/reports/') ? report : null,
    loading: false,
    error: null,
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
});
