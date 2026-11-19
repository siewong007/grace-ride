import '../i18n';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import Chat from './Chat';

let chatData: Record<string, unknown> | null = null;
const reload = vi.fn().mockResolvedValue(undefined);
const post = vi.fn().mockResolvedValue({ ok: true });
vi.mock('../api', () => ({
  useApi: () => ({ data: chatData, loading: false, error: null, reload }),
  post: (...args: unknown[]) => post(...args),
}));

const msg = (over: Record<string, unknown>) => ({
  user_id: 2,
  name: 'Ali',
  created_at: '2026-11-19T10:00:00',
  mine: false,
  reactions: [],
  ...over,
});

beforeEach(() => {
  post.mockClear();
  reload.mockClear();
  chatData = {
    open: true,
    members: ['Me', 'Ali'],
    reactions_allowed: ['👍', '❤️', '😂', '😮', '😢', '🙏'],
    messages: [
      msg({ id: 1, user_id: null, name: null, body: 'Trip approved' }),
      msg({ id: 2, body: 'On my way', reactions: [{ emoji: '👍', count: 2, mine: true, names: ['Me', 'Ali'] }] }),
      msg({ id: 3, user_id: 1, name: 'Me', body: 'sent one', mine: true, status: 'sent' }),
      msg({ id: 4, user_id: 1, name: 'Me', body: 'delivered one', mine: true, status: 'delivered' }),
      msg({ id: 5, user_id: 1, name: 'Me', body: 'read one', mine: true, status: 'read' }),
    ],
  };
});

describe('Chat receipts and reactions', () => {
  it('shows sent, delivered and read ticks on my messages only', () => {
    render(<Chat rideId={9} viewerRole="passenger" />);
    expect(screen.getByRole('img', { name: 'Sent' })).toBeInTheDocument();
    expect(screen.getByRole('img', { name: 'Delivered' })).toBeInTheDocument();
    const read = screen.getByRole('img', { name: 'Read' });
    expect(read).toHaveClass('read');
    expect(screen.getAllByRole('img', { name: /Sent|Delivered|Read/ })).toHaveLength(3);
  });

  it('marks the chat read up to the newest message', async () => {
    render(<Chat rideId={9} />);
    await waitFor(() => expect(post).toHaveBeenCalledWith('/rides/9/messages/read', { up_to: 5 }));
  });

  it('waits until the chat is on screen before marking it read', async () => {
    let fire: (visible: boolean) => void = () => {};
    class FakeIO {
      constructor(cb: (e: { isIntersecting: boolean }[]) => void) {
        fire = (visible) => cb([{ isIntersecting: visible }]);
      }
      observe() {}
      disconnect() {}
    }
    vi.stubGlobal('IntersectionObserver', FakeIO);
    try {
      render(<Chat rideId={9} />);
      act(() => fire(false));
      await new Promise((r) => setTimeout(r, 20));
      expect(post).not.toHaveBeenCalledWith('/rides/9/messages/read', expect.anything());
      act(() => fire(true));
      await waitFor(() => expect(post).toHaveBeenCalledWith('/rides/9/messages/read', { up_to: 5 }));
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it('shows reaction counts and toggles my reaction from the chip', async () => {
    render(<Chat rideId={9} />);
    const chip = screen.getByRole('button', { name: /👍 2/ });
    expect(chip).toHaveAttribute('aria-pressed', 'true');
    expect(chip).toHaveTextContent('2');
    fireEvent.click(chip);
    await waitFor(() => expect(post).toHaveBeenCalledWith('/rides/9/messages/2/reactions', { emoji: '👍' }));
    expect(reload).toHaveBeenCalled();
  });

  it('opens the picker and reacts with a chosen emoji', async () => {
    render(<Chat rideId={9} />);
    // The system message has no react button.
    expect(screen.getAllByRole('button', { name: 'React to message' })).toHaveLength(4);
    fireEvent.click(screen.getAllByRole('button', { name: 'React to message' })[2]);
    const picker = screen.getByRole('group', { name: 'Choose a reaction' });
    fireEvent.click(within(picker).getByRole('button', { name: 'React with ❤️' }));
    await waitFor(() => expect(post).toHaveBeenCalledWith('/rides/9/messages/4/reactions', { emoji: '❤️' }));
    expect(screen.queryByRole('group', { name: 'Choose a reaction' })).toBeNull();
  });

  it('offers no reacting once the chat is read-only', () => {
    chatData = { ...chatData, open: false };
    render(<Chat rideId={9} />);
    expect(screen.queryByRole('button', { name: 'React to message' })).toBeNull();
    expect(screen.getByRole('button', { name: /👍 2/ })).toBeDisabled();
  });
});
