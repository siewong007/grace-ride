import { beforeAll, beforeEach, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import i18n from '../../i18n';
import AdminRides from './Rides';

const mocks = vi.hoisted(() => ({ paths: [] as string[] }));

vi.mock('../../api', () => ({
  api: vi.fn(),
  useApi: (path: string | null) => {
    if (path) mocks.paths.push(path);
    return { data: { items: [], total: 0, page: 1, page_size: 50, total_pages: 1 }, loading: false, error: null, reload: vi.fn() };
  },
}));

beforeAll(async () => {
  await i18n.changeLanguage('en');
});
beforeEach(() => {
  mocks.paths = [];
});

it('searches rides by passenger, driver or place on the server', async () => {
  render(<MemoryRouter><AdminRides /></MemoryRouter>);
  fireEvent.change(screen.getByLabelText('Search passenger, driver or place'), { target: { value: 'Phoebe' } });
  await waitFor(() => expect(mocks.paths.some((p) => p.includes('/admin/rides?') && p.includes('q=Phoebe'))).toBe(true));
});
