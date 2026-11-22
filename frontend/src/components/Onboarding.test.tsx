import '../i18n';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { GettingStarted, WaitingRoom } from './Onboarding';

let me: Record<string, unknown> = {};
let granted: string[] = [];
const refresh = vi.fn();
vi.mock('../auth', () => ({
  useAuth: () => ({
    me,
    can: (page: string, act = 'view') => granted.includes(act === 'view' ? page : `${page}.${act}`),
    refresh,
  }),
}));

let apiData: Record<string, unknown> = {};
const put = vi.fn().mockResolvedValue({ ok: true });
vi.mock('../api', () => ({
  useApi: (path: string | null) => ({ data: path ? apiData[path] : null, loading: false, error: null, reload: vi.fn() }),
  put: (...args: unknown[]) => put(...args),
}));

const inRouter = (el: React.ReactNode) => render(<MemoryRouter>{el}</MemoryRouter>);

beforeEach(() => {
  put.mockClear();
  refresh.mockClear();
  apiData = { '/saved-places': [], '/driver/profile': { vehicle: null } };
  granted = ['rides', 'rides.create', 'events', 'help'];
  me = { id: 7, name: 'New', status: 'active', passkey_count: 1, photo_file_id: null, onboarding_dismissed: false };
});

describe('WaitingRoom', () => {
  it('lists what a pending member can do now and that approval is in progress', () => {
    me = { ...me, status: 'pending', passkey_count: 0 };
    inRouter(<WaitingRoom />);
    expect(screen.getByRole('heading', { name: 'Almost there' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /add a passkey/i })).toHaveAttribute('href', '/me?section=security');
    expect(screen.getByRole('link', { name: /add a profile photo/i })).toHaveAttribute('href', '/me?section=personal');
    // The admin's step is not a link: there is nothing for the member to do.
    expect(screen.getByText(/assigns your role and group/i).closest('a')).toBeNull();
    expect(screen.getByText(/updates by itself/i)).toBeInTheDocument();
  });

  it('marks a passkey already added as done', () => {
    me = { ...me, status: 'pending', passkey_count: 1 };
    inRouter(<WaitingRoom />);
    expect(screen.queryByRole('link', { name: /add a passkey/i })).toBeNull();
    expect(screen.getByText('Add a passkey').closest('li')).toHaveTextContent('Done');
  });
});

describe('GettingStarted', () => {
  it('shows only the steps the member can take, with progress', () => {
    inRouter(<GettingStarted hasRide={false} hasEvent={false} />);
    expect(screen.getByRole('heading', { name: 'Getting started' })).toBeInTheDocument();
    expect(screen.getByText('1 of 4 done')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /book your first ride/i })).toHaveAttribute('href', '/events');
    expect(screen.getByRole('link', { name: /save your pickup address/i })).toHaveAttribute('href', '/me?section=places');
    // Not a driver: no car step.
    expect(screen.queryByText(/add your car/i)).toBeNull();
  });

  it('adds the car step for drivers', () => {
    granted = [...granted, 'drive'];
    inRouter(<GettingStarted hasRide={false} hasEvent={false} />);
    expect(screen.getByRole('link', { name: /add your car/i })).toHaveAttribute('href', '/drive?tab=settings');
  });

  it('goes away once every step is done', () => {
    apiData['/saved-places'] = [{ id: 1 }];
    const { container } = inRouter(<GettingStarted hasRide hasEvent />);
    expect(container).toBeEmptyDOMElement();
  });

  it('stays away for a member who hid it, or who is still pending', () => {
    me = { ...me, onboarding_dismissed: true };
    const first = inRouter(<GettingStarted hasRide={false} hasEvent={false} />);
    expect(first.container).toBeEmptyDOMElement();
    first.unmount();
    me = { ...me, onboarding_dismissed: false, status: 'pending' };
    const second = inRouter(<GettingStarted hasRide={false} hasEvent={false} />);
    expect(second.container).toBeEmptyDOMElement();
  });

  it('saves the choice to hide it on the member', async () => {
    inRouter(<GettingStarted hasRide={false} hasEvent={false} />);
    fireEvent.click(screen.getByRole('button', { name: 'Hide' }));
    await waitFor(() => expect(put).toHaveBeenCalledWith('/me', { onboarding_dismissed: true }));
    await waitFor(() => expect(refresh).toHaveBeenCalled());
  });
});
