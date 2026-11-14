import { beforeAll, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
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

  fireEvent.click(await screen.findByTitle('Filter by object'));
  await waitFor(() => {
    expect(screen.getByLabelText('search params').textContent).toContain('entity=ride');
    expect(screen.getByLabelText('search params').textContent).toContain('entity_id=17');
  });
});
