import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MemoryRouter, useSearchParams } from 'react-router-dom';
import { useApi } from './api';
import { AuthProvider, useAuth } from './auth';

function deferredResponse() {
  let resolve!: (response: Response) => void;
  const promise = new Promise<Response>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

function response(value: string) {
  return new Response(JSON.stringify({ value }), { status: 200 });
}

function Harness() {
  const [params, setParams] = useSearchParams();
  const { data, error } = useApi<{ value: string }>(`/members?sort=${params.get('sort')}`);
  return (
    <div>
      <output data-testid="value">{data?.value ?? ''}</output>
      <output data-testid="error">{error ?? ''}</output>
      <button onClick={() => setParams({ sort: 'desc' })}>Desc</button>
    </div>
  );
}

function AuthStatusProbe() {
  const { me, loading } = useAuth();
  return <output data-testid="auth-status">{loading ? 'loading' : me ? me.name : 'anon'}</output>;
}

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe('useApi stale responses and request cancellation', () => {
  it('does not let an older query response replace newer data', async () => {
    const requests: ReturnType<typeof deferredResponse>[] = [];
    vi.stubGlobal('fetch', vi.fn(() => {
      const request = deferredResponse();
      requests.push(request);
      return request.promise;
    }));

    render(
      <MemoryRouter initialEntries={['/members?sort=asc']}>
        <Harness />
      </MemoryRouter>,
    );

    await waitFor(() => expect(requests).toHaveLength(1));
    fireEvent.click(screen.getByRole('button', { name: 'Desc' }));
    await waitFor(() => expect(requests).toHaveLength(2));

    await act(async () => requests[1].resolve(response('desc')));
    await waitFor(() => expect(screen.getByTestId('value')).toHaveTextContent('desc'));

    await act(async () => requests[0].resolve(response('asc')));
    expect(screen.getByTestId('value')).toHaveTextContent('desc');
  });

  it('aborts in-flight requests when path changes or unmounts without setting an error', async () => {
    const signals: AbortSignal[] = [];
    vi.stubGlobal(
      'fetch',
      vi.fn((_input: RequestInfo | URL, init?: RequestInit) => {
        if (init?.signal) signals.push(init.signal);
        return new Promise<Response>((_resolve, reject) => {
          init?.signal?.addEventListener('abort', () => {
            reject(new DOMException('Aborted', 'AbortError'));
          });
        });
      }),
    );

    const { unmount } = render(
      <MemoryRouter initialEntries={['/members?sort=asc']}>
        <Harness />
      </MemoryRouter>,
    );

    await waitFor(() => expect(signals).toHaveLength(1));
    expect(signals[0].aborted).toBe(false);

    fireEvent.click(screen.getByRole('button', { name: 'Desc' }));
    await waitFor(() => expect(signals).toHaveLength(2));
    expect(signals[0].aborted).toBe(true);
    expect(signals[1].aborted).toBe(false);
    expect(screen.getByTestId('error')).toHaveTextContent('');

    unmount();
    expect(signals[1].aborted).toBe(true);
  });
});

describe('AuthProvider network polling hygiene', () => {
  it('does not poll /api/auth/me when unauthenticated', async () => {
    vi.useFakeTimers();
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({ error: 'unauthorized' }), { status: 401 }));
    vi.stubGlobal('fetch', fetchMock);

    render(
      <AuthProvider>
        <AuthStatusProbe />
      </AuthProvider>,
    );

    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(screen.getByTestId('auth-status')).toHaveTextContent('anon');
    expect(fetchMock).toHaveBeenCalledTimes(1);

    await act(async () => {
      vi.advanceTimersByTime(120_000);
      await Promise.resolve();
    });
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
});
