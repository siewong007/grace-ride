import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MemoryRouter, useSearchParams } from 'react-router-dom';
import { useApi } from './api';

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
  const { data } = useApi<{ value: string }>(
    `/members?sort=${params.get('sort')}`,
    undefined,
    { latestOnly: true },
  );
  return (
    <div>
      <output data-testid="value">{data?.value ?? ''}</output>
      <button onClick={() => setParams({ sort: 'desc' })}>Desc</button>
    </div>
  );
}

afterEach(() => vi.unstubAllGlobals());

describe('useApi latestOnly', () => {
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
});
