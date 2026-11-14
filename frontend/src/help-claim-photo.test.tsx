import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import HelpDetail from './pages/HelpDetail';
import './i18n';

const post = vi.fn().mockResolvedValue({ ok: true });
const uploadFile = vi.fn().mockResolvedValue({ id: 55, mime: 'image/png', name: 'gate.png' });
const ticket = {
  id: 7, subject: 'Driver arrived early', status: 'open', ride_id: null, user_id: 2,
  assignee_id: null, assignee_name: null, member: { name: 'Tan', phone: '', other_open: [] },
  replies: [{ id: 1, user_id: 2, name: 'Tan', body: '', staff: false, created_at: '2026-11-15T07:00:00', file_id: 40 }],
};
vi.mock('./api', () => ({
  post: (...a: unknown[]) => post(...a),
  uploadFile: (...a: unknown[]) => uploadFile(...a),
  api: vi.fn(),
  useApi: (path: string | null) => ({ data: path === '/help/7' ? ticket : null, loading: false, error: null, reload: vi.fn() }),
}));
vi.mock('./auth', () => ({
  useAuth: () => ({ me: { id: 1, name: 'Staff' }, can: (p: string) => p === 'admin_help' || p === 'help', refresh: vi.fn() }),
}));

describe('Help thread: claim and photos', () => {
  it('shows a member photo, lets staff claim, and sends a photo-only reply', async () => {
    render(<MemoryRouter><HelpDetail ticketId={7} /></MemoryRouter>);
    expect(screen.getByRole('img', { name: /Photo in message/ })).toHaveAttribute('src', '/api/files/40');

    fireEvent.click(screen.getByRole('button', { name: 'Claim' }));
    await waitFor(() => expect(post).toHaveBeenCalledWith('/help/7/claim', { claim: true }));

    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    fireEvent.change(input, { target: { files: [new File(['x'], 'gate.png', { type: 'image/png' })] } });
    await screen.findByText('Photo attached');
    const send = screen.getByRole('button', { name: /Send/ });
    expect(send).toBeEnabled();
    fireEvent.click(send);
    await waitFor(() => expect(post).toHaveBeenCalledWith('/help/7/replies', { body: '', file_id: 55 }));
  });
});
