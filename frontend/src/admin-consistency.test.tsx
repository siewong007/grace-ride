import { beforeAll, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import i18n from './i18n';
import RideDetail from './pages/RideDetail';
import Settings from './pages/admin/Settings';

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

const ride = {
  id: 42, status: 'pending_approval', pickup_at: '2026-11-20T09:30:00', area: { id: 1, name: 'PJ' },
  pickup_address: 'Jalan 1', destination: 'Church', seats: 1, assistance: '', notes: '',
  event: null, trip_id: null, passenger: { id: 5, name: 'Tan', phone: '' }, driver: null,
  needs_attention: false, attention_reason: '', started_at: null, picked_up_at: null, completed_at: null,
  updated_at: '2026-11-15T10:00:00', viewer: { is_passenger: false, is_driver: false, is_staff: true },
};
const post = vi.fn().mockResolvedValue({ ok: true });
const api = vi.fn().mockResolvedValue([{ id: 42 }, { id: 51 }]);
vi.mock('./api', () => ({
  post: (...a: unknown[]) => post(...a),
  api: (...a: unknown[]) => api(...a),
  del: vi.fn(),
  uploadFile: vi.fn(),
  subscribe: () => () => {},
  useApi: (path: string | null) => ({ data: path === '/rides/42' ? ride : null, loading: false, error: null, reload: vi.fn() }),
}));
let granted: string[] = ['admin_rides'];
vi.mock('./auth', () => ({
  useAuth: () => ({ me: { id: 1, name: 'Staff', groups: [] }, can: (p: string) => granted.includes(p), refresh: vi.fn() }),
}));

describe('Admin consistency', () => {
  it('offers the next ride waiting for approval after approving one', async () => {
    granted = ['admin_rides'];
    render(
      <MemoryRouter initialEntries={['/admin/rides/42']}>
        <Routes><Route path="/admin/rides/:id" element={<RideDetail />} /></Routes>
      </MemoryRouter>,
    );
    fireEvent.click(screen.getAllByRole('button', { name: 'Approve ride' })[0]);
    await waitFor(() => expect(post).toHaveBeenCalledWith('/rides/42/approve'));
    const next = await screen.findAllByRole('link', { name: /Next ride waiting for approval/ });
    expect(next[0]).toHaveAttribute('href', '/admin/rides/51');
  });

  it('shows one shared no-access message with a way home', () => {
    granted = [];
    render(<MemoryRouter><Settings /></MemoryRouter>);
    expect(screen.getByRole('heading', { name: "You don't have access to this page" })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Go to home' })).toHaveAttribute('href', '/');
  });
});
