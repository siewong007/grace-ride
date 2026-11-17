import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { act, render, screen } from '@testing-library/react';
import i18n from '../i18n';
import { UniversalTable } from './ui';

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

type Row = { id: number; name: string };
const rows: Row[] = [{ id: 1, name: 'Sister Phoebe' }];
const columns = [{ key: 'name', header: 'Name', render: (r: Row) => r.name }];

/** A ResizeObserver whose reports the test controls. */
function stubResizeObserver() {
  let notify: (width: number) => void = () => {};
  class FakeObserver {
    constructor(cb: ResizeObserverCallback) {
      notify = (width) => cb([{ contentRect: { width } } as ResizeObserverEntry], this as unknown as ResizeObserver);
    }
    observe() {}
    disconnect() {}
    unobserve() {}
  }
  vi.stubGlobal('ResizeObserver', FakeObserver);
  return (width: number) => act(() => notify(width));
}

afterEach(() => vi.unstubAllGlobals());

describe('UniversalTable phone layout', () => {
  it('switches to compact rows when the space is narrower than the columns need, wherever the screen is', () => {
    const resize = stubResizeObserver();
    render(
      <UniversalTable
        columns={columns}
        items={rows}
        itemKey={(r) => r.id}
        compactRow={(r) => <span>compact {r.name}</span>}
      />,
    );
    resize(1000);
    expect(screen.getByRole('table')).toBeInTheDocument();
    expect(screen.queryByText('compact Sister Phoebe')).toBeNull();

    resize(700);
    expect(screen.queryByRole('table')).toBeNull();
    expect(screen.getByText('compact Sister Phoebe')).toBeInTheDocument();
  });

  it('keeps the table when no compact row is offered', () => {
    const resize = stubResizeObserver();
    render(<UniversalTable columns={columns} items={rows} itemKey={(r) => r.id} />);
    resize(300);
    expect(screen.getByRole('table')).toBeInTheDocument();
  });
});
