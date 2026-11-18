import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { fireEvent, render, screen, act, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { AutoLockProvider, clearAutoLock, useAutoLock } from './AutoLock';

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
  passkey_count: 1,
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

const reauth = vi.fn();
vi.mock('../webauthn', () => ({
  passkeySupported: () => true,
  reauthWithPasskey: () => reauth(),
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
    reauth.mockReset();
    reauth.mockResolvedValue(undefined);
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

  const renderLocked = () => {
    render(
      <MemoryRouter>
        <AutoLockProvider>
          <TestConsumer />
        </AutoLockProvider>
      </MemoryRouter>
    );
    fireEvent.click(screen.getByText('Trigger Lock'));
  };

  it('locks screen on manual lock() call and unlocks only after a passkey check', async () => {
    vi.useRealTimers();
    renderLocked();

    expect(screen.getByTestId('status')).toHaveTextContent('locked');
    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveAttribute('aria-modal', 'true');
    expect(screen.getByText('Grace Member')).toBeInTheDocument();
    expect(screen.getByText('grace@example.com')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /unlockWithPasskey|^Unlock$/i }));

    await waitFor(() => expect(screen.getByTestId('status')).toHaveTextContent('unlocked'));
    expect(reauth).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('stays locked and shows the error when the passkey check fails', async () => {
    vi.useRealTimers();
    reauth.mockRejectedValue(new Error('Passkey verification failed'));
    renderLocked();

    fireEvent.click(screen.getByRole('button', { name: /unlockWithPasskey|^Unlock$/i }));

    expect(await screen.findByRole('alert')).toHaveTextContent('Passkey verification failed');
    expect(screen.getByTestId('status')).toHaveTextContent('locked');
  });

  it('offers no unlock button when the member has no passkey', () => {
    currentMe = { ...defaultMockMe, passkey_count: 0 };
    renderLocked();

    expect(screen.getByTestId('status')).toHaveTextContent('locked');
    expect(screen.queryByRole('button', { name: /unlock/i })).toBeNull();
    expect(screen.getByRole('button', { name: /logout|Log out/i })).toBeInTheDocument();
    expect(reauth).not.toHaveBeenCalled();
  });

  it('keeps the code form behind the link under the card and unlocks with a 6-digit code', async () => {
    vi.useRealTimers();
    currentMe = { ...defaultMockMe, totp_enabled: true } as typeof currentMe;
    const { post } = await import('../api');
    renderLocked();

    expect(screen.queryByRole('textbox')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: /unlockWithCode|backup code/i }));
    fireEvent.change(screen.getByRole('textbox'), { target: { value: '123456' } });
    fireEvent.click(screen.getByRole('button', { name: /^(unlock|Unlock)$/ }));

    await waitFor(() => expect(screen.getByTestId('status')).toHaveTextContent('unlocked'));
    expect(post).toHaveBeenCalledWith('/auth/unlock', { totp_code: '123456' });
    expect(reauth).not.toHaveBeenCalled();
  });

  it('opens straight on the code form when a code is the only way in', () => {
    currentMe = { ...defaultMockMe, passkey_count: 0, totp_enabled: true } as typeof currentMe;
    renderLocked();

    expect(screen.getByRole('textbox')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /unlockUsePasskey|Use passkey/i })).toBeNull();
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

  const renderProvider = () =>
    render(
      <MemoryRouter>
        <AutoLockProvider>
          <TestConsumer />
        </AutoLockProvider>
      </MemoryRouter>
    );

  it('stays locked after a reload', () => {
    const first = renderProvider();
    fireEvent.click(screen.getByText('Trigger Lock'));
    first.unmount();

    renderProvider();
    expect(screen.getByTestId('status')).toHaveTextContent('locked');
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('ignores a lock saved for another member', () => {
    localStorage.setItem('gr_locked_user', '999');
    renderProvider();
    expect(screen.getByTestId('status')).toHaveTextContent('unlocked');
  });

  it('starts unlocked after a fresh sign-in clears the saved lock', () => {
    const first = renderProvider();
    fireEvent.click(screen.getByText('Trigger Lock'));
    first.unmount();

    clearAutoLock();
    renderProvider();
    expect(screen.getByTestId('status')).toHaveTextContent('unlocked');
  });

  it('forgets the lock when the session ends', () => {
    renderProvider();
    fireEvent.click(screen.getByText('Trigger Lock'));
    act(() => {
      window.dispatchEvent(new Event('gr:unauthorized'));
    });
    expect(screen.getByTestId('status')).toHaveTextContent('unlocked');
    expect(localStorage.getItem('gr_locked_user')).toBeNull();
  });

  it('forgets the lock on log out', async () => {
    vi.useRealTimers();
    renderProvider();
    fireEvent.click(screen.getByText('Trigger Lock'));
    fireEvent.click(screen.getByRole('button', { name: /logout|Log out/i }));
    await waitFor(() => expect(screen.getByTestId('status')).toHaveTextContent('unlocked'));
    expect(localStorage.getItem('gr_locked_user')).toBeNull();
  });

  it('follows a lock or unlock made in another tab', () => {
    renderProvider();
    act(() => {
      localStorage.setItem('gr_locked_user', '1');
      window.dispatchEvent(new StorageEvent('storage', { key: 'gr_locked_user', newValue: '1' }));
    });
    expect(screen.getByTestId('status')).toHaveTextContent('locked');

    act(() => {
      localStorage.removeItem('gr_locked_user');
      window.dispatchEvent(new StorageEvent('storage', { key: 'gr_locked_user', newValue: null }));
    });
    expect(screen.getByTestId('status')).toHaveTextContent('unlocked');
  });

  it('does not lock while another tab is in use', () => {
    renderProvider();
    act(() => {
      vi.advanceTimersByTime(10 * 60 * 1000);
    });
    // Another tab reports fresh activity.
    localStorage.setItem('gr_last_active', String(Date.now()));
    act(() => {
      vi.advanceTimersByTime(10 * 60 * 1000);
    });
    expect(screen.getByTestId('status')).toHaveTextContent('unlocked');
  });
});
