import { beforeAll, expect, it, vi } from 'vitest';
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

  expect(await screen.findByRole('button', { name: 'Event #11 · Sunday Lunch' })).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'View details #11' }));
  const details = within(await screen.findByRole('dialog', { name: 'Audit entry details' }));
  expect(details.getByText('Log ID')).toBeVisible();
  expect(details.getByText('#11')).toBeVisible();
  expect(details.getByText('Sunday Lunch')).toBeVisible();
  expect(details.queryByText(/Event #11/)).not.toBeInTheDocument();
  expect(details.queryByRole('button', { name: /filter by|actor ID|copy JSON/i })).not.toBeInTheDocument();
  expect(details.getByRole('link', { name: 'Open object' })).toHaveAttribute('href', '/events/11');
});
