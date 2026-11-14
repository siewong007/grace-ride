import { useState } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import '../i18n';
import {
  Button,
  Card,
  ConfirmProvider,
  EmptyState,
  Field,
  IconButton,
  InfiniteSentinel,
  KeyValue,
  Modal,
  Page,
  Pagination,
  Pill,
  Skeleton,
  Stat,
  Tabs,
  useConfirm,
  VirtualList,
  BackToTop,
  TableColumn,
  TableFilter,
  UniversalTable,
} from './ui';

describe('Button primitive', () => {
  it('defaults to type="button"', () => {
    render(<Button>Click me</Button>);
    const btn = screen.getByRole('button', { name: 'Click me' });
    expect(btn).toHaveAttribute('type', 'button');
  });

  it('respects explicit type="submit"', () => {
    render(<Button type="submit">Submit</Button>);
    const btn = screen.getByRole('button', { name: 'Submit' });
    expect(btn).toHaveAttribute('type', 'submit');
  });

  it('shows loading spinner and sets aria-busy while loading', () => {
    render(<Button loading>Save</Button>);
    const btn = screen.getByRole('button');
    expect(btn).toHaveAttribute('aria-busy', 'true');
    expect(btn).toBeDisabled();
    expect(btn.querySelector('.btn-spinner')).toBeTruthy();
  });

  it('renders as a router link when "to" prop is passed', () => {
    render(
      <MemoryRouter>
        <Button to="/dest">Go</Button>
      </MemoryRouter>,
    );
    const link = screen.getByRole('link', { name: 'Go' });
    expect(link).toHaveAttribute('href', '/dest');
  });
});

describe('IconButton primitive', () => {
  it('sets required aria-label on the button', () => {
    render(<IconButton label="Close panel" icon={<span>X</span>} />);
    const btn = screen.getByRole('button', { name: 'Close panel' });
    expect(btn).toHaveAttribute('aria-label', 'Close panel');
  });
});

describe('Tabs primitive & keyboard navigation', () => {
  function TabsHarness() {
    const [val, setVal] = useState('t1');
    return (
      <Tabs
        value={val}
        onChange={setVal}
        items={[
          { value: 't1', label: 'Tab 1' },
          { value: 't2', label: 'Tab 2' },
          { value: 't3', label: 'Tab 3' },
        ]}
      />
    );
  }

  it('renders tablist and tabs with correct aria attributes', () => {
    render(<TabsHarness />);
    const tablist = screen.getByRole('tablist');
    expect(tablist).toBeInTheDocument();

    const tabs = screen.getAllByRole('tab');
    expect(tabs).toHaveLength(3);
    expect(tabs[0]).toHaveAttribute('aria-selected', 'true');
    expect(tabs[0]).toHaveAttribute('tabindex', '0');
    expect(tabs[1]).toHaveAttribute('aria-selected', 'false');
    expect(tabs[1]).toHaveAttribute('tabindex', '-1');
  });

  it('navigates tabs using ArrowRight, ArrowLeft, Home, and End keys', () => {
    render(<TabsHarness />);
    const tabs = screen.getAllByRole('tab');

    tabs[0].focus();
    expect(document.activeElement).toBe(tabs[0]);

    // ArrowRight -> tab 2
    fireEvent.keyDown(tabs[0], { key: 'ArrowRight' });
    expect(tabs[1]).toHaveAttribute('aria-selected', 'true');

    // ArrowRight -> tab 3
    fireEvent.keyDown(tabs[1], { key: 'ArrowRight' });
    expect(tabs[2]).toHaveAttribute('aria-selected', 'true');

    // ArrowRight loops back -> tab 1
    fireEvent.keyDown(tabs[2], { key: 'ArrowRight' });
    expect(tabs[0]).toHaveAttribute('aria-selected', 'true');

    // ArrowLeft -> tab 3
    fireEvent.keyDown(tabs[0], { key: 'ArrowLeft' });
    expect(tabs[2]).toHaveAttribute('aria-selected', 'true');

    // Home -> tab 1
    fireEvent.keyDown(tabs[2], { key: 'Home' });
    expect(tabs[0]).toHaveAttribute('aria-selected', 'true');

    // End -> tab 3
    fireEvent.keyDown(tabs[0], { key: 'End' });
    expect(tabs[2]).toHaveAttribute('aria-selected', 'true');
  });
});

describe('Modal primitive', () => {
  it('closes on Escape and returns focus to the triggering element', async () => {
    const user = userEvent.setup();
    const handleClose = vi.fn();

    function ModalTrigger() {
      const [open, setOpen] = useState(false);
      return (
        <div>
          <button id="trigger-btn" onClick={() => setOpen(true)}>Open Modal</button>
          {open && (
            <Modal
              title="Test Modal"
              onClose={() => {
                setOpen(false);
                handleClose();
              }}
            >
              <p>Modal content</p>
            </Modal>
          )}
        </div>
      );
    }

    render(<ModalTrigger />);
    const trigger = screen.getByRole('button', { name: 'Open Modal' });
    await user.click(trigger);

    expect(screen.getByRole('dialog', { name: 'Test Modal' })).toBeInTheDocument();

    // Press Escape
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(handleClose).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();

    // Focus returns to the trigger
    expect(document.activeElement).toBe(trigger);
  });
});

describe('ConfirmDialog & useConfirm()', () => {
  it('resolves true on confirm and false on cancel', async () => {
    const user = userEvent.setup();
    let result: boolean | null = null;

    function TestComponent() {
      const confirm = useConfirm();
      return (
        <button
          onClick={async () => {
            const ok = await confirm({
              title: 'Delete Item',
              body: 'Are you sure?',
              confirmLabel: 'Yes Delete',
              cancelLabel: 'Keep',
            });
            result = ok;
          }}
        >
          Prompt Delete
        </button>
      );
    }

    const { unmount } = render(
      <ConfirmProvider>
        <TestComponent />
      </ConfirmProvider>,
    );

    // Test Cancel resolution
    await user.click(screen.getByRole('button', { name: 'Prompt Delete' }));
    expect(screen.getByText('Delete Item')).toBeInTheDocument();
    expect(screen.getByText('Are you sure?')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Keep' }));
    expect(result).toBe(false);
    expect(screen.queryByText('Delete Item')).not.toBeInTheDocument();

    // Test Confirm resolution
    await user.click(screen.getByRole('button', { name: 'Prompt Delete' }));
    await user.click(screen.getByRole('button', { name: 'Yes Delete' }));
    expect(result).toBe(true);
    expect(screen.queryByText('Delete Item')).not.toBeInTheDocument();

    unmount();
  });
});

describe('Other UI primitives', () => {
  it('renders Card with padding variants', () => {
    render(<Card variant="flat" padding="lg">Card content</Card>);
    const card = screen.getByText('Card content');
    expect(card).toHaveClass('card', 'flat', 'pad-lg');
  });

  it('renders Page layout with width constraint', () => {
    render(<Page width="narrow">Page content</Page>);
    const page = screen.getByText('Page content');
    expect(page).toHaveClass('page-container', 'page-narrow');
  });

  it('renders Pill with mapped semantic tones', () => {
    render(<Pill tone="green">Success Tag</Pill>);
    const pill = screen.getByText('Success Tag');
    expect(pill).toHaveClass('pill', 'success');
  });

  it('renders Stat metric summary', () => {
    render(<Stat label="Total Rides" value={42} tone="warn" />);
    expect(screen.getByText('Total Rides')).toBeInTheDocument();
    expect(screen.getByText('42')).toBeInTheDocument();
  });

  it('renders KeyValue list', () => {
    render(
      <KeyValue
        items={[
          { label: 'Origin', value: 'Station' },
          { label: 'Destination', value: 'Church' },
        ]}
      />,
    );
    expect(screen.getByText('Origin')).toBeInTheDocument();
    expect(screen.getByText('Church')).toBeInTheDocument();
  });

  it('renders Field with label, hint and error', () => {
    const { rerender } = render(
      <Field label="Phone number" hint="Shown after approval">
        <input type="tel" />
      </Field>,
    );
    expect(screen.getByText('Phone number')).toBeInTheDocument();
    expect(screen.getByText('Shown after approval')).toBeInTheDocument();

    rerender(
      <Field label="Phone number" error="Invalid phone format">
        <input type="tel" />
      </Field>,
    );
    expect(screen.getByRole('alert')).toHaveTextContent('Invalid phone format');
    expect(screen.queryByText('Shown after approval')).not.toBeInTheDocument();
  });

  it('renders EmptyState with title and action', () => {
    render(
      <EmptyState
        title="No items found"
        description="Try searching again"
        action={<button>Add One</button>}
      />,
    );
    expect(screen.getByText('No items found')).toBeInTheDocument();
    expect(screen.getByText('Try searching again')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Add One' })).toBeInTheDocument();
  });

  it('renders Skeleton placeholders', () => {
    const { container } = render(<Skeleton lines={2} />);
    expect(container.querySelectorAll('.skeleton')).toHaveLength(2);
  });
});

describe('Pagination component', () => {
  it('renders page buttons and marks current page', () => {
    const onPage = vi.fn();
    render(<Pagination page={2} totalPages={5} onPageChange={onPage} />);
    const page2 = screen.getByRole('button', { name: /Page 2/ });
    expect(page2).toHaveAttribute('aria-current', 'page');
    expect(page2).toHaveClass('active');
  });

  it('navigates next and previous', async () => {
    const onPage = vi.fn();
    render(<Pagination page={2} totalPages={5} onPageChange={onPage} />);
    const prev = screen.getByRole('button', { name: /Previous/ });
    const next = screen.getByRole('button', { name: /Next/ });

    fireEvent.click(prev);
    expect(onPage).toHaveBeenCalledWith(1);

    fireEvent.click(next);
    expect(onPage).toHaveBeenCalledWith(3);
  });

  it('disables previous on first page and next on last page', () => {
    const { rerender } = render(<Pagination page={1} totalPages={3} onPageChange={vi.fn()} />);
    expect(screen.getByRole('button', { name: /Previous/ })).toBeDisabled();
    expect(screen.getByRole('button', { name: /Next/ })).not.toBeDisabled();

    rerender(<Pagination page={3} totalPages={3} onPageChange={vi.fn()} />);
    expect(screen.getByRole('button', { name: /Previous/ })).not.toBeDisabled();
    expect(screen.getByRole('button', { name: /Next/ })).toBeDisabled();
  });

  it('triggers page size change', () => {
    const onPageSize = vi.fn();
    render(
      <Pagination
        page={1}
        totalPages={10}
        total={250}
        pageSize={25}
        onPageChange={vi.fn()}
        onPageSizeChange={onPageSize}
      />,
    );
    const select = screen.getByRole('combobox');
    fireEvent.change(select, { target: { value: '50' } });
    expect(onPageSize).toHaveBeenCalledWith(50);
  });

  it('renders range counter showing records from and to', () => {
    render(
      <Pagination
        page={2}
        totalPages={10}
        total={250}
        pageSize={25}
        onPageChange={vi.fn()}
      />,
    );
    expect(screen.getByText(/Showing 26–50 of 250 records/i)).toBeInTheDocument();
  });

  it('supports jump to page input when totalPages > 5', () => {
    const onPage = vi.fn();
    render(
      <Pagination
        page={1}
        totalPages={12}
        total={300}
        pageSize={25}
        onPageChange={onPage}
      />,
    );

    const jumpInput = screen.getByRole('spinbutton', { name: /Go to/i });
    fireEvent.change(jumpInput, { target: { value: '7' } });
    const goBtn = screen.getByRole('button', { name: /Go/i });
    fireEvent.click(goBtn);

    expect(onPage).toHaveBeenCalledWith(7);
  });
});

describe('BackToTop primitive', () => {
  it('renders sentinel element for zero scroll-listener visibility', () => {
    const { container } = render(<BackToTop />);
    expect(container.querySelector('.back-to-top-sentinel')).toBeInTheDocument();
  });
});

describe('InfiniteSentinel primitive', () => {
  it('renders loading state when loading is true', () => {
    render(<InfiniteSentinel loading={true} hasMore={true} />);
    expect(screen.getByText(/Loading more/i)).toBeInTheDocument();
  });

  it('renders end-of-list message when hasMore is false and not loading', () => {
    render(<InfiniteSentinel loading={false} hasMore={false} />);
    expect(screen.getByText(/No more records/i)).toBeInTheDocument();
  });
});

describe('VirtualList primitive', () => {
  it('renders virtual list container and items', () => {
    const items = ['Alpha', 'Beta', 'Gamma'];
    render(
      <VirtualList
        items={items}
        height={300}
        estimateSize={50}
        renderItem={(item) => <div data-testid="virtual-row">{item}</div>}
      />,
    );

    const rows = screen.getAllByTestId('virtual-row');
    expect(rows.length).toBeGreaterThan(0);
    expect(screen.getByText('Alpha')).toBeInTheDocument();
  });
});

describe('UniversalTable component', () => {
  interface User {
    id: number;
    name: string;
  }

  const users: User[] = [
    { id: 1, name: 'Ana' },
    { id: 2, name: 'Bo' },
  ];

  const columns: TableColumn<User>[] = [
    {
      key: 'name',
      header: 'Name',
      render: (u) => <span data-testid="name-cell">{u.name}</span>,
    },
  ];

  it('renders header and body rows', () => {
    render(<UniversalTable columns={columns} items={users} itemKey={(u) => u.id} />);
    const headers = screen.getAllByRole('columnheader');
    expect(headers).toHaveLength(1);
    expect(headers[0]).toHaveTextContent('Name');

    const cells = screen.getAllByTestId('name-cell');
    expect(cells).toHaveLength(2);
    expect(cells[0]).toHaveTextContent('Ana');
    expect(cells[1]).toHaveTextContent('Bo');
  });

  it('adds data-label to cells in responsive stacked mode', () => {
    render(<UniversalTable columns={columns} items={users} itemKey={(u) => u.id} />);
    const dataLabels = screen
      .getAllByRole('cell')
      .map((cell) => cell.getAttribute('data-label'));
    expect(dataLabels).toEqual(['Name', 'Name']);
  });

  it('uses explicit mobile labels for JSX headers and omits missing labels', () => {
    render(
      <UniversalTable
        columns={[
          { ...columns[0], header: <span>Name</span>, dataLabel: 'Name' },
          { key: 'actions', header: <span>Actions</span>, render: () => 'Edit' },
        ]}
        items={users}
        itemKey={(u) => u.id}
      />,
    );
    expect(screen.getAllByRole('cell').map((cell) => cell.getAttribute('data-label'))).toEqual([
      'Name',
      null,
      'Name',
      null,
    ]);
  });

  it('renders empty content when items is empty', () => {
    const emptyContent = <p data-testid="empty">No rows</p>;
    render(
      <UniversalTable
        columns={columns}
        items={[]}
        emptyContent={emptyContent}
        itemKey={(u) => u.id}
      />,
    );
    expect(screen.getByTestId('empty')).toHaveTextContent('No rows');
  });

  it('calls onRowClick with the item when a row is clicked', () => {
    const onRowClick = vi.fn();
    render(
      <UniversalTable columns={columns} items={users} onRowClick={onRowClick} itemKey={(u) => u.id} />,
    );
    const firstRow = screen.getAllByRole('row')[1];
    fireEvent.click(firstRow);
    expect(onRowClick).toHaveBeenCalledWith(users[0], 0);
  });

  it('is keyboard accessible when rows are clickable', () => {
    const onRowClick = vi.fn();
    render(
      <UniversalTable columns={columns} items={users} onRowClick={onRowClick} itemKey={(u) => u.id} />,
    );
    const rows = screen.getAllByRole('row');
    fireEvent.keyDown(rows[1], { key: 'Enter' });
    expect(onRowClick).toHaveBeenCalledWith(users[0], 0);
  });

  interface SortableUser {
    id: number;
    name: string;
    age: number | null;
  }

  const sortableUsers: SortableUser[] = [
    { id: 1, name: 'Bo', age: 2 },
    { id: 2, name: 'Ana', age: null },
    { id: 3, name: 'Cy', age: 1 },
  ];

  const sortableColumns: TableColumn<SortableUser>[] = [
    {
      key: 'name',
      header: 'Name',
      sortable: true,
      sortValue: (u) => u.name,
      render: (u) => <span data-testid="sort-name">{u.name}</span>,
    },
    {
      key: 'age',
      header: 'Age',
      sortable: true,
      sortValue: (u) => u.age,
      render: (u) => <span data-testid="sort-age">{u.age ?? 'none'}</span>,
    },
  ];

  it('sorts locally and toggles direction', () => {
    render(
      <UniversalTable
        columns={sortableColumns}
        items={sortableUsers}
        defaultSort={{ key: 'name', dir: 'asc' }}
        itemKey={(u) => u.id}
      />,
    );
    expect(screen.getAllByTestId('sort-name').map((n) => n.textContent)).toEqual([
      'Ana',
      'Bo',
      'Cy',
    ]);
    const header = screen.getAllByRole('columnheader')[0];
    expect(header).toHaveAttribute('aria-sort', 'ascending');
    fireEvent.click(screen.getByRole('button', { name: /name/i }));
    expect(screen.getAllByTestId('sort-name').map((n) => n.textContent)).toEqual([
      'Cy',
      'Bo',
      'Ana',
    ]);
    expect(header).toHaveAttribute('aria-sort', 'descending');
  });

  it('keeps null values last in both directions', () => {
    render(
      <UniversalTable
        columns={sortableColumns}
        items={sortableUsers}
        defaultSort={{ key: 'age', dir: 'desc' }}
        itemKey={(u) => u.id}
      />,
    );
    expect(screen.getAllByTestId('sort-age').map((n) => n.textContent)).toEqual([
      '2',
      '1',
      'none',
    ]);
    fireEvent.click(screen.getByRole('button', { name: /age/i }));
    expect(screen.getAllByTestId('sort-age').map((n) => n.textContent)).toEqual([
      '1',
      '2',
      'none',
    ]);
  });

  it('does not reorder controlled rows and emits the next sort', () => {
    const onSortChange = vi.fn();
    render(
      <UniversalTable
        columns={sortableColumns}
        items={sortableUsers}
        sort={{ key: 'name', dir: 'asc' }}
        onSortChange={onSortChange}
        itemKey={(u) => u.id}
      />,
    );
    expect(screen.getAllByTestId('sort-name').map((n) => n.textContent)).toEqual([
      'Bo',
      'Ana',
      'Cy',
    ]);
    fireEvent.click(screen.getByRole('button', { name: /name/i }));
    expect(onSortChange).toHaveBeenCalledWith({ key: 'name', dir: 'desc' });
  });

  it('renders a declarative filter toolbar and clear action', () => {
    const onFilterChange = vi.fn();
    const onClearFilters = vi.fn();
    const filters: TableFilter[] = [
      { type: 'text', key: 'q', label: 'Search' },
      {
        type: 'select',
        key: 'status',
        label: 'Status',
        allLabel: 'All statuses',
        options: [
          { value: '', label: 'Any status' },
          { value: 'active', label: 'Active' },
        ],
      },
      { type: 'date', key: 'date', label: 'Date' },
      { type: 'checkbox', key: 'attention', label: 'Only attention' },
    ];
    render(
      <UniversalTable
        columns={columns}
        items={users}
        itemKey={(u) => u.id}
        filters={filters}
        filterValues={{ q: 'ana' }}
        onFilterChange={onFilterChange}
        onClearFilters={onClearFilters}
        clearFiltersLabel="Clear filters"
      />,
    );
    expect(screen.getByRole('textbox', { name: 'Search' })).toHaveValue('ana');
    expect(screen.getAllByRole('option').map((option) => option.textContent)).toEqual([
      'All statuses',
      'Active',
    ]);
    fireEvent.change(screen.getByRole('combobox', { name: 'Status' }), {
      target: { value: 'active' },
    });
    expect(onFilterChange).toHaveBeenCalledWith('status', 'active');
    fireEvent.click(screen.getByLabelText('Only attention'));
    expect(onFilterChange).toHaveBeenCalledWith('attention', 'true');
    fireEvent.click(screen.getByRole('button', { name: 'Clear filters' }));
    expect(onClearFilters).toHaveBeenCalled();
  });

  it('keeps a select filter empty until its first option is chosen', async () => {
    const onFilterChange = vi.fn();
    const user = userEvent.setup();
    render(
      <UniversalTable
        columns={columns}
        items={users}
        itemKey={(u) => u.id}
        filters={[
          {
            type: 'select',
            key: 'status',
            label: 'Status',
            options: [{ value: 'active', label: 'Active' }],
          },
        ]}
        onFilterChange={onFilterChange}
      />,
    );
    const filter = screen.getByRole('combobox', { name: 'Status' });
    expect(filter).toHaveValue('');
    await user.selectOptions(filter, 'active');
    expect(onFilterChange).toHaveBeenCalledWith('status', 'active');
  });

  it('keeps the toolbar visible when there are no rows', () => {
    render(
      <UniversalTable
        columns={columns}
        items={[]}
        itemKey={(u) => u.id}
        filters={[{ type: 'text', key: 'q', label: 'Search' }]}
        filterValues={{}}
        emptyContent={<p data-testid="empty">No rows</p>}
      />,
    );
    expect(screen.getByRole('textbox', { name: 'Search' })).toBeInTheDocument();
    expect(screen.getByTestId('empty')).toBeInTheDocument();
  });

  it('does not fire row clicks from interactive children', () => {
    const onRowClick = vi.fn();
    const actionColumns: TableColumn<User>[] = [
      {
        key: 'action',
        header: 'Action',
        render: () => <button type="button">Edit</button>,
      },
    ];
    render(
      <UniversalTable
        columns={actionColumns}
        items={users}
        onRowClick={onRowClick}
        itemKey={(u) => u.id}
      />,
    );
    fireEvent.click(screen.getAllByRole('button', { name: 'Edit' })[0]);
    expect(onRowClick).not.toHaveBeenCalled();
  });

  it('changes the local sort on mobile and restores the default order', () => {
    const onSortChange = vi.fn();
    render(
      <UniversalTable
        columns={sortableColumns}
        items={sortableUsers}
        defaultSort={{ key: 'name', dir: 'asc' }}
        onSortChange={onSortChange}
        itemKey={(u) => u.id}
      />,
    );
    const control = screen.getByRole('combobox', { name: 'Sort by' });
    fireEvent.change(control, { target: { value: 'name:desc' } });
    expect(onSortChange).toHaveBeenLastCalledWith({ key: 'name', dir: 'desc' });
    expect(screen.getAllByTestId('sort-name').map((n) => n.textContent)).toEqual([
      'Cy',
      'Bo',
      'Ana',
    ]);

    fireEvent.change(control, { target: { value: '' } });
    expect(onSortChange).toHaveBeenLastCalledWith(null);
    expect(screen.getAllByTestId('sort-name').map((n) => n.textContent)).toEqual([
      'Ana',
      'Bo',
      'Cy',
    ]);
  });

  it('preserves colons in sort keys selected on mobile', async () => {
    const onSortChange = vi.fn();
    const user = userEvent.setup();
    render(
      <UniversalTable
        columns={[{ ...sortableColumns[0], sortKey: 'ride:pickup' }]}
        items={sortableUsers}
        onSortChange={onSortChange}
        itemKey={(u) => u.id}
      />,
    );
    await user.selectOptions(screen.getByRole('combobox', { name: 'Sort by' }), 'ride:pickup:desc');
    expect(onSortChange).toHaveBeenCalledWith({ key: 'ride:pickup', dir: 'desc' });
  });
});

