import { useState } from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { AddressInput } from './AddressInput';

const places = [
  { label: 'Jalan SS2/28, SS 2, 57300 Petaling Jaya, Selangor', lat: 3.11, lng: 101.62 },
  { label: 'SS2 Murni, 53 Jalan SS 2/75, SS 2, 47300 Petaling Jaya, Selangor', lat: 3.12, lng: 101.62 },
];

function Harness({ onSubmit }: { onSubmit: () => void }) {
  const [v, setV] = useState('');
  return (
    <form onSubmit={(e) => { e.preventDefault(); onSubmit(); }}>
      <AddressInput id="addr" label="Pickup address" value={v} onChange={setV} />
      <output data-testid="value">{v}</output>
    </form>
  );
}

describe('AddressInput', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('suggests places as you type and picks one with the keyboard', async () => {
    const fetchMock = vi.fn(async () => new Response(JSON.stringify(places), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    const submit = vi.fn();
    render(<Harness onSubmit={submit} />);

    const input = screen.getByRole('combobox', { name: 'Pickup address' });
    await userEvent.type(input, 'jalan ss2');

    const options = await screen.findAllByRole('option');
    expect(options).toHaveLength(2);
    expect(fetchMock).toHaveBeenCalledWith('/api/places?q=jalan%20ss2', expect.anything());
    expect(input).toHaveAttribute('aria-expanded', 'true');

    await userEvent.keyboard('{ArrowDown}{ArrowDown}{Enter}');
    expect(screen.getByTestId('value')).toHaveTextContent(places[1].label);
    expect(submit).not.toHaveBeenCalled(); // Enter picked the option, it didn't submit
    expect(input).toHaveAttribute('aria-expanded', 'false');
  });

  it('does not search until three characters are typed', async () => {
    const fetchMock = vi.fn(async () => new Response('[]', { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    render(<Harness onSubmit={() => {}} />);
    await userEvent.type(screen.getByRole('combobox'), 'ja');
    await new Promise((r) => setTimeout(r, 400));
    expect(fetchMock).not.toHaveBeenCalled();
    await waitFor(() => expect(screen.queryByRole('option')).toBeNull());
  });
});
