import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen } from '@testing-library/react';
import i18n from '../i18n';
import { UniversalTable } from './ui';

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

type Row = { id: number; name: string };
const rows: Row[] = [{ id: 1, name: 'Sister Phoebe' }];
const columns = [{ key: 'name', header: 'Name', sortable: true, render: (r: Row) => r.name }];

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
  it('switches to compact rows with is-compact class and sort control when container is narrow', () => {
    const resize = stubResizeObserver();
    const { container } = render(
      <UniversalTable
        columns={columns}
        items={rows}
        itemKey={(r) => r.id}
        compactRow={(r) => <span>compact {r.name}</span>}
      />,
    );
    resize(1000);
    expect(screen.getByRole('table')).toBeInTheDocument();
    expect(container.querySelector('.table-block')).not.toHaveClass('is-compact');
    expect(screen.queryByText('compact Sister Phoebe')).toBeNull();

    resize(700);
    expect(screen.queryByRole('table')).toBeNull();
    expect(container.querySelector('.table-block')).toHaveClass('is-compact');
    expect(screen.getByRole('combobox', { name: /sort by/i })).toBeInTheDocument();
    expect(screen.getByText('compact Sister Phoebe')).toBeInTheDocument();
  });

  it('keeps interactive checkboxes as accessible siblings beside clickable compact rows without triggering onRowClick', () => {
    const resize = stubResizeObserver();
    const onRowClick = vi.fn();
    const onToggle = vi.fn();
    render(
      <UniversalTable
        columns={columns}
        items={rows}
        itemKey={(r) => r.id}
        onRowClick={onRowClick}
        compactRowLead={(r) => (
          <input
            type="checkbox"
            aria-label={`Select ${r.name}`}
            onChange={(e) => onToggle(e.target.checked)}
          />
        )}
        compactRow={(r) => <span>compact {r.name}</span>}
      />,
    );
    resize(700);

    const checkbox = screen.getByRole('checkbox', { name: 'Select Sister Phoebe' });
    expect(checkbox.closest('button')).toBeNull();
    fireEvent.click(checkbox);
    expect(onToggle).toHaveBeenCalledWith(true);
    expect(onRowClick).not.toHaveBeenCalled();

    fireEvent.click(screen.getByText('compact Sister Phoebe'));
    expect(onRowClick).toHaveBeenCalledTimes(1);
  });

  it('keeps the table when no compact row is offered', () => {
    const resize = stubResizeObserver();
    render(<UniversalTable columns={columns} items={rows} itemKey={(r) => r.id} />);
    resize(300);
    expect(screen.getByRole('table')).toBeInTheDocument();
  });
});
