import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import i18n from '../../i18n';
import { ConfirmProvider, ToastProvider } from '../../components/ui';
import Members from './Members';

const mocks = vi.hoisted(() => ({ post: vi.fn() }));

vi.mock('../../api', () => ({
  del: vi.fn(),
  put: vi.fn(),
  post: mocks.post,
  useApi: (path: string | null) => ({
    data: path === '/admin/recovery'
      ? [
          { id: 1, name: 'Uncle Raj', email: 'raj@example.org', reason: 'Lost my phone', status: 'pending', created_at: '2026-11-19T08:00:00' },
        ]
      : path?.startsWith('/admin/roles') ? { roles: [] } : [],
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
}));

vi.mock('../../auth', () => ({
  useAuth: () => ({ can: () => true, me: { roles: ['superadmin'] } }),
}));

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

beforeEach(() => {
  mocks.post.mockReset().mockResolvedValue({ url: 'https://example.org/recover/abc123' });
});

const show = () =>
  render(
    <MemoryRouter initialEntries={['/admin/members?tab=recovery']}>
      <ToastProvider>
        <ConfirmProvider>
          <Members />
        </ConfirmProvider>
      </ToastProvider>
    </MemoryRouter>,
  );

describe('Account recovery requests', () => {
  it('asks before approving, then shows the one-time link with a copy button', async () => {
    show();
    fireEvent.click(screen.getByRole('button', { name: 'Verify & approve' }));
    // Nothing is sent until the admin agrees; the dialog says who it is for and what happens.
    const dialog = await screen.findByRole('dialog');
    expect(within(dialog).getByText(/Uncle Raj \(raj@example\.org\)/)).toBeInTheDocument();
    expect(within(dialog).getByText(/signs them out of every device/i)).toBeInTheDocument();
    expect(mocks.post).not.toHaveBeenCalled();

    fireEvent.click(within(dialog).getByRole('button', { name: 'Verify & approve' }));
    await waitFor(() => expect(mocks.post).toHaveBeenCalledWith('/admin/recovery/1/approve'));
    expect(await screen.findByText('https://example.org/recover/abc123')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Copy' })).toBeInTheDocument();
  });

  it('does nothing when the admin backs out of an approval', async () => {
    show();
    fireEvent.click(screen.getByRole('button', { name: 'Verify & approve' }));
    const dialog = await screen.findByRole('dialog');
    fireEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(mocks.post).not.toHaveBeenCalled();
  });

  it('asks before rejecting', async () => {
    show();
    fireEvent.click(screen.getByRole('button', { name: 'Reject' }));
    const dialog = await screen.findByRole('dialog');
    expect(mocks.post).not.toHaveBeenCalled();
    fireEvent.click(within(dialog).getByRole('button', { name: 'Reject' }));
    await waitFor(() => expect(mocks.post).toHaveBeenCalledWith('/admin/recovery/1/reject'));
  });
});
