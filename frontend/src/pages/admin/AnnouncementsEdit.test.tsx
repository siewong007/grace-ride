import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import i18n from '../../i18n';
import { ConfirmProvider, ToastProvider } from '../../components/ui';
import Announcements from './Announcements';

const mocks = vi.hoisted(() => ({ put: vi.fn(), post: vi.fn(), allowed: new Set<string>() }));

const list = [{ id: 4, title: 'Car park closed', body: 'Use the side gate.', pinned: false, by: 'Silas', created_at: '2026-11-12T08:00:00', targeted: false, groups: [] }];
const groups = [
  { id: 1, name: 'All members', is_all: true, parent_id: null, member_count: 0 },
  { id: 2, name: 'Youth', is_all: false, parent_id: null, member_count: 12 },
  { id: 3, name: 'Youth staff', is_all: false, parent_id: 2, member_count: 3 },
];

vi.mock('../../api', () => ({
  put: mocks.put,
  post: mocks.post,
  del: vi.fn(),
  useApi: (url: string) => ({ data: url === '/groups' ? groups : list, loading: false, error: null, reload: vi.fn() }),
}));
vi.mock('../../auth', () => ({
  useAuth: () => ({ me: { name: 'Mary' }, can: (p: string, a = 'view') => mocks.allowed.has(`${p}.${a}`) }),
}));

beforeAll(async () => {
  await i18n.changeLanguage('en');
});
beforeEach(() => {
  mocks.put.mockReset().mockResolvedValue({});
  mocks.post.mockReset().mockResolvedValue({ id: 9 });
  mocks.allowed = new Set(['admin_announcements.view', 'admin_announcements.create', 'admin_announcements.delete']);
});

const show = () =>
  render(
    <MemoryRouter>
      <ToastProvider>
        <ConfirmProvider>
          <Announcements />
        </ConfirmProvider>
      </ToastProvider>
    </MemoryRouter>,
  );

describe('Announcements', () => {
  it('pins a published announcement without touching its text', async () => {
    show();
    fireEvent.click(screen.getByRole('button', { name: 'Pin' }));
    await waitFor(() =>
      expect(mocks.put).toHaveBeenCalledWith('/announcements/4', { title: 'Car park closed', body: 'Use the side gate.', pinned: true }),
    );
  });

  it('edits the text in a dialog', async () => {
    show();
    fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
    const dialog = await screen.findByRole('dialog');
    fireEvent.change(within(dialog).getByLabelText('Content'), { target: { value: 'Use the back gate.' } });
    fireEvent.click(within(dialog).getByRole('button', { name: 'Save changes' }));
    await waitFor(() =>
      expect(mocks.put).toHaveBeenCalledWith('/announcements/4', { title: 'Car park closed', body: 'Use the back gate.', pinned: false }),
    );
  });

  it('asks before notifying every member, and does nothing if the author backs out', async () => {
    show();
    fireEvent.change(screen.getAllByLabelText('Title')[0], { target: { value: 'Christmas' } });
    fireEvent.change(screen.getAllByLabelText('Content')[0], { target: { value: 'Sign up' } });
    fireEvent.click(screen.getByRole('button', { name: 'Publish' }));
    const dialog = await screen.findByRole('dialog');
    expect(within(dialog).getByText(/every active member gets a notification/i)).toBeInTheDocument();
    expect(mocks.post).not.toHaveBeenCalled();
    fireEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(mocks.post).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: 'Publish' }));
    fireEvent.click(within(await screen.findByRole('dialog')).getByRole('button', { name: 'Publish' }));
    await waitFor(() => expect(mocks.post).toHaveBeenCalledTimes(1));
  });

  it('publishes for chosen groups and says only they are notified', async () => {
    show();
    fireEvent.change(screen.getAllByLabelText('Title')[0], { target: { value: 'Youth retreat' } });
    fireEvent.change(screen.getAllByLabelText('Content')[0], { target: { value: 'Sign up' } });
    fireEvent.click(screen.getByRole('checkbox', { name: /^Youth\s*·/ }));
    expect(screen.getByText('For Youth')).toBeInTheDocument();
    expect(screen.getByRole('checkbox', { name: 'Notify members of the chosen groups' })).toBeChecked();
    fireEvent.click(screen.getByRole('button', { name: 'Publish' }));
    const dialog = await screen.findByRole('dialog');
    expect(within(dialog).getByText(/for Youth \(and their sub-groups\)/)).toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole('button', { name: 'Publish' }));
    await waitFor(() =>
      expect(mocks.post).toHaveBeenCalledWith('/announcements', {
        title: 'Youth retreat',
        body: 'Sign up',
        pinned: false,
        notify_all: true,
        group_ids: [2],
      }),
    );
  });

  it('does not offer editing to someone who may not publish', () => {
    mocks.allowed = new Set(['admin_announcements.view']);
    show();
    expect(screen.queryByRole('button', { name: 'Edit' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Pin' })).toBeNull();
  });
});
