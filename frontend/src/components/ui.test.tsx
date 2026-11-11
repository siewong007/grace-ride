import { useState } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import {
  Button,
  Card,
  ConfirmProvider,
  EmptyState,
  IconButton,
  KeyValue,
  Modal,
  Page,
  Pill,
  Skeleton,
  Stat,
  Tabs,
  useConfirm,
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
