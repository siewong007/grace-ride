import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import i18n from '../i18n';
import TurnstileWidget from './TurnstileWidget';
import { resetTurnstileForTests, type TurnstileApi, type TurnstileRenderOptions } from '../turnstile';

describe('TurnstileWidget', () => {
  let renderedOpts: TurnstileRenderOptions[] = [];
  let renderMock: ReturnType<typeof vi.fn>;
  let resetMock: ReturnType<typeof vi.fn>;
  let removeMock: ReturnType<typeof vi.fn>;

  beforeAll(async () => {
    await i18n.changeLanguage('en');
  });

  beforeEach(() => {
    resetTurnstileForTests();
    renderedOpts = [];
    delete document.documentElement.dataset.theme;
    let seq = 0;
    renderMock = vi.fn((_el: HTMLElement | string, opts: TurnstileRenderOptions) => {
      renderedOpts.push(opts);
      return `widget-${++seq}`;
    });
    resetMock = vi.fn();
    removeMock = vi.fn();
    window.turnstile = {
      render: renderMock,
      reset: resetMock,
      remove: removeMock,
      getResponse: vi.fn(),
    } as TurnstileApi;
  });

  afterEach(() => {
    delete window.turnstile;
    delete document.documentElement.dataset.theme;
  });

  it('renders nothing when siteKey is null', () => {
    const onToken = vi.fn();
    const { container } = render(
      <TurnstileWidget siteKey={null} action="passkey_login" onToken={onToken} />,
    );
    expect(container.firstChild).toBeNull();
    expect(renderMock).not.toHaveBeenCalled();
  });

  it('renders widget with expected options, emits token, handles expiry/error/retry, and cleans up', () => {
    const onToken = vi.fn();
    const { rerender, unmount } = render(
      <TurnstileWidget siteKey="1x00000000000000000000AA" action="passkey_login" resetKey={2} onToken={onToken} />,
    );

    expect(renderMock).toHaveBeenCalledTimes(1);
    expect(resetMock).not.toHaveBeenCalled();
    const opts = renderedOpts[0];
    expect(opts.sitekey).toBe('1x00000000000000000000AA');
    expect(opts.action).toBe('passkey_login');
    expect(opts.size).toBe('flexible');
    expect(opts.theme).toBe('auto');
    expect(opts.language).toBe('en');

    act(() => opts.callback?.('tok-abc'));
    expect(onToken).toHaveBeenLastCalledWith('tok-abc');

    act(() => opts['expired-callback']?.());
    expect(onToken).toHaveBeenLastCalledWith(null);
    expect(screen.getByRole('alert')).toHaveTextContent(/expired/i);

    fireEvent.click(screen.getByRole('button', { name: /retry verification/i }));
    expect(resetMock).toHaveBeenCalledWith('widget-1');

    act(() => opts['error-callback']?.('600010'));
    expect(onToken).toHaveBeenLastCalledWith(null);
    expect(screen.getByRole('alert')).toHaveTextContent(/could not load/i);

    rerender(
      <TurnstileWidget siteKey="1x00000000000000000000AA" action="passkey_login" resetKey={3} onToken={onToken} />,
    );
    expect(resetMock).toHaveBeenCalledWith('widget-1');

    unmount();
    expect(removeMock).toHaveBeenCalledWith('widget-1');
  });

  it('tracks data-theme changes on documentElement', async () => {
    const onToken = vi.fn();
    render(
      <TurnstileWidget siteKey="1x00000000000000000000AA" action="invite_accept" onToken={onToken} />,
    );
    expect(renderedOpts[0]?.theme).toBe('auto');

    act(() => {
      document.documentElement.dataset.theme = 'dark';
    });

    await waitFor(() => expect(renderMock).toHaveBeenCalledTimes(2));
    expect(removeMock).toHaveBeenCalledWith('widget-1');
    expect(renderedOpts[1]?.theme).toBe('dark');
  });
});
