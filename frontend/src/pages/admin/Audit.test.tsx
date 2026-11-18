import { beforeAll, expect, it, onTestFinished, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter, useSearchParams } from 'react-router-dom';
import i18n from '../../i18n';
import Audit from './Audit';

const mocks = vi.hoisted(() => ({
  api: vi.fn(),
}));

vi.mock('../../api', () => ({ api: mocks.api }));

function SearchState() {
  const [params] = useSearchParams();
  return <output aria-label="search params">{params.toString()}</output>;
}

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

it('sets both object filter parameters in one interaction', async () => {
  mocks.api.mockResolvedValue({
    items: [{
      id: 10, actor_id: 3, actor: 'Transport', action: 'ride.claim',
      entity: 'ride', entity_id: 17, detail: 'Claimed', created_at: '2026-11-14T10:00:00',
    }],
    next_before: null,
    retention_days: 730,
  });
  render(
    <MemoryRouter initialEntries={['/admin/audit']}>
      <Audit />
      <SearchState />
    </MemoryRouter>,
  );

  fireEvent.click(await screen.findByRole('button', { name: 'Ride #17' }));
  await waitFor(() => {
    expect(screen.getByLabelText('search params').textContent).toContain('entity=ride');
    expect(screen.getByLabelText('search params').textContent).toContain('entity_id=17');
  });
});

it('keeps the audit log ID separate from the affected record and omits filter and copy controls in details', async () => {
  mocks.api.mockResolvedValue({
    items: [{
      id: 11, actor_id: 3, actor: 'Transport', action: 'event.update',
      entity: 'event', entity_id: 11, entity_name: 'Sunday Lunch',
      detail: '', created_at: '2026-11-14T10:00:00',
    }],
    next_before: null,
  });
  render(<MemoryRouter><Audit /></MemoryRouter>);

  fireEvent.click(await screen.findByRole('button', { name: 'Sunday Lunch' }));
  await waitFor(() => expect(mocks.api).toHaveBeenLastCalledWith('/admin/audit?entity=event&entity_id=11'));
  expect(screen.queryByRole('textbox', { name: 'Item ID' })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'View details #11' }));
  const details = within(await screen.findByRole('dialog', { name: 'Audit entry details' }));
  expect(details.getByText('Log ID')).toBeVisible();
  expect(details.getByText('#11')).toBeVisible();
  expect(details.getByText('Sunday Lunch')).toBeVisible();
  expect(details.queryByText(/Event #11/)).not.toBeInTheDocument();
  expect(details.queryByRole('button', { name: /filter by|actor ID|copy JSON/i })).not.toBeInTheDocument();
  expect(details.getByRole('link', { name: 'Open item' })).toHaveAttribute('href', '/events/11');
});

it('shows readable attendance changes in the audit table and details', async () => {
  const event = {
    actor: 'Organiser', action: 'event.attendance', entity: 'event', entity_id: 6,
    created_at: '2026-11-14T10:00:00',
  };
  mocks.api.mockResolvedValue({
    items: [
      { ...event, id: 42, detail: 'user 9 = Some(true)', attendance: { member_id: 9, member_name: 'Mei Tan', status: 'attended' } },
      { ...event, id: 41, detail: 'user 9 = Some(false)', attendance: { member_id: 9, member_name: 'Mei Tan', status: 'absent' } },
      { ...event, id: 40, detail: 'user 10 = None', attendance: { member_id: 10, member_name: null, status: 'cleared' } },
      { ...event, id: 39, action: 'login.passkey', entity: 'user', entity_id: 9, entity_name: 'Mei Tan', detail: '' },
    ],
    next_before: null,
  });
  render(<MemoryRouter><Audit /></MemoryRouter>);

  expect(await screen.findByText('Marked Mei Tan as attended')).toBeVisible();
  expect(screen.getByText('Marked Mei Tan as absent')).toBeVisible();
  expect(screen.getByText('Cleared attendance for an unknown member')).toBeVisible();
  expect(screen.getAllByRole('button', { name: 'Event' })).toHaveLength(3);
  expect(screen.getByRole('button', { name: 'Mei Tan' })).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'View details #42' }));
  const details = within(await screen.findByRole('dialog'));
  expect(details.getByText('Marked Mei Tan as attended')).toBeVisible();
  expect(details.getByText('Event')).toBeVisible();
  expect(details.queryByText('user 9 = Some(true)')).not.toBeInTheDocument();
});

it('leads with the name and keeps the kind, number and pick-up time beneath it', async () => {
  // Pin the clock: fmtDay turns a date near today into "Tomorrow", which this check does not expect.
  vi.useFakeTimers({ toFake: ['Date'] });
  vi.setSystemTime(new Date('2026-11-01T10:00:00'));
  onTestFinished(() => {
    vi.useRealTimers();
  });
  const base = { actor: 'Transport', detail: '', created_at: '2026-11-14T10:00:00' };
  mocks.api.mockResolvedValue({
    items: [
      { ...base, id: 50, action: 'ride.cancel', entity: 'ride', entity_id: 17, entity_name: 'Mei Tan', entity_at: '2026-11-20T09:30:00' },
      { ...base, id: 49, action: 'help.close', entity: 'help_ticket', entity_id: 4, entity_name: 'Lost umbrella' },
      { ...base, id: 48, action: 'role.update', entity: 'role', entity_id: null, entity_name: 'Ride lead' },
      { ...base, id: 47, action: 'announcement.delete', entity: 'announcement', entity_id: 8, entity_name: 'Old news' },
    ],
    next_before: null,
  });
  render(
    <MemoryRouter initialEntries={['/admin/audit']}>
      <Audit />
      <SearchState />
    </MemoryRouter>,
  );

  expect(await screen.findByRole('columnheader', { name: 'Affected item' })).toBeVisible();
  expect(screen.getByText(/^Ride #17 · .*2026|^Ride #17 · .*Nov/)).toBeVisible();
  expect(screen.getByText('Help request #4')).toBeVisible();
  const roleRow = within(screen.getByRole('button', { name: 'Ride lead' }).closest('tr')!);
  expect(roleRow.getByText('Role')).toBeVisible();
  // Only a page for that one item earns the open icon; a list page does not.
  expect(screen.getByRole('link', { name: 'Open item Mei Tan' })).toHaveAttribute('href', '/admin/rides/17');
  expect(screen.getByRole('link', { name: 'Open item Lost umbrella' })).toHaveAttribute('href', '/help/4');
  expect(screen.queryByRole('link', { name: 'Open item Old news' })).not.toBeInTheDocument();

  fireEvent.click(screen.getByRole('button', { name: 'View details #47' }));
  const details = within(await screen.findByRole('dialog'));
  expect(details.getByRole('link', { name: 'Open list' })).toHaveAttribute('href', '/admin/announcements');
  fireEvent.click(details.getAllByRole('button', { name: 'Close' })[0]);

  fireEvent.click(screen.getByRole('button', { name: 'Mei Tan' }));
  await waitFor(() => {
    expect(screen.getByLabelText('search params').textContent).toContain('entity=ride');
    expect(screen.getByLabelText('search params').textContent).toContain('entity_id=17');
  });
});
