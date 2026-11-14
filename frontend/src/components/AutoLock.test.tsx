import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { fireEvent, render, screen, act } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { AutoLockProvider, useAutoLock } from './AutoLock';

let currentMe = {
  id: 1,
  name: 'Grace Member',
  email: 'grace@example.com',
  phone: '0123456789',
  photo_file_id: null,
  roles: ['member'],
  groups: [{ id: 10, name: 'Youth' }],
  badges_opt_out: false,
  status: 'active',
  terms_accepted: true,
  privileged: false,
  mfa_ok: true,
  unread: 0,
  auto_lock_mins: 15,
  session_timeout_mins: 15,
};

const defaultMockMe = { ...currentMe };

vi.mock('../auth', () => ({
  useAuth: () => ({
    me: currentMe,
    can: () => true,
    refresh: vi.fn(),
  }),
}));

vi.mock('../api', () => ({
  post: vi.fn().mockResolvedValue({ ok: true }),
}));

function TestConsumer() {
  const { isLocked, lock } = useAutoLock();
  return (
    <div>
      <span data-testid="status">{isLocked ? 'locked' : 'unlocked'}</span>
      <button onClick={lock}>Trigger Lock</button>
    </div>
  );
}

describe('AutoLockProvider', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    localStorage.clear();
    currentMe = { ...defaultMockMe };
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('renders children without lock screen initially', () => {
    render(
      <MemoryRouter>
        <AutoLockProvider>
          <TestConsumer />
        </AutoLockProvider>
      </MemoryRouter>
    );

    expect(screen.getByTestId('status')).toHaveTextContent('unlocked');
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('locks screen on manual lock() call and unlocks on click', () => {
    render(
      <MemoryRouter>
        <AutoLockProvider>
          <TestConsumer />
        </AutoLockProvider>
      </MemoryRouter>
    );

    // Trigger manual lock
    fireEvent.click(screen.getByText('Trigger Lock'));

    expect(screen.getByTestId('status')).toHaveTextContent('locked');
    const dialog = screen.getByRole('dialog');
    expect(dialog).toBeInTheDocument();
    expect(dialog).toHaveAttribute('aria-modal', 'true');
    expect(screen.getByText('Grace Member')).toBeInTheDocument();
    expect(screen.getByText('grace@example.com')).toBeInTheDocument();

    // Click Unlock button
    const unlockBtn = screen.getByRole('button', { name: /unlock/i });
    fireEvent.click(unlockBtn);

    expect(screen.getByTestId('status')).toHaveTextContent('unlocked');
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('triggers lock when idle time exceeds configured auto_lock_mins', () => {
    render(
      <MemoryRouter>
        <AutoLockProvider>
          <TestConsumer />
        </AutoLockProvider>
      </MemoryRouter>
    );

    expect(screen.getByTestId('status')).toHaveTextContent('unlocked');

    // Advance timers by 15 minutes + 5 seconds
    act(() => {
      vi.advanceTimersByTime(15 * 60 * 1000 + 5000);
    });

    expect(screen.getByTestId('status')).toHaveTextContent('locked');
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('resets idle timer on user activity', () => {
    render(
      <MemoryRouter>
        <AutoLockProvider>
          <TestConsumer />
        </AutoLockProvider>
      </MemoryRouter>
    );

    // Advance 10 minutes
    act(() => {
      vi.advanceTimersByTime(10 * 60 * 1000);
    });

    // Simulate user activity
    fireEvent.pointerDown(window);

    // Advance another 10 minutes (total 20 mins, but only 10 mins since activity)
    act(() => {
      vi.advanceTimersByTime(10 * 60 * 1000);
    });

    expect(screen.getByTestId('status')).toHaveTextContent('unlocked');
    expect(screen.queryByRole('dialog')).toBeNull();

    // Advance another 5 minutes + 5 seconds (15 mins since last activity)
    act(() => {
      vi.advanceTimersByTime(5 * 60 * 1000 + 5000);
    });

    expect(screen.getByTestId('status')).toHaveTextContent('locked');
  });

  it('falls back to organization session_timeout_mins when auto_lock_mins is 0', () => {
    currentMe = {
      ...defaultMockMe,
      auto_lock_mins: 0,
      session_timeout_mins: 10,
    };

    render(
      <MemoryRouter>
        <AutoLockProvider>
          <TestConsumer />
        </AutoLockProvider>
      </MemoryRouter>
    );

    expect(screen.getByTestId('status')).toHaveTextContent('unlocked');

    // Advance 10 minutes + 5 seconds
    act(() => {
      vi.advanceTimersByTime(10 * 60 * 1000 + 5000);
    });

    expect(screen.getByTestId('status')).toHaveTextContent('locked');
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('does not lock when auto_lock_mins is -1 (disabled)', () => {
    currentMe = {
      ...defaultMockMe,
      auto_lock_mins: -1,
      session_timeout_mins: 10,
    };

    render(
      <MemoryRouter>
        <AutoLockProvider>
          <TestConsumer />
        </AutoLockProvider>
      </MemoryRouter>
    );

    expect(screen.getByTestId('status')).toHaveTextContent('unlocked');

    // Advance 30 minutes
    act(() => {
      vi.advanceTimersByTime(30 * 60 * 1000);
    });

    expect(screen.getByTestId('status')).toHaveTextContent('unlocked');
    expect(screen.queryByRole('dialog')).toBeNull();
  });
});
