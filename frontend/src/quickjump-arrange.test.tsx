import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import GuideArranger from './components/GuideArranger';
import QuickJump from './components/QuickJump';
import './i18n';

const post = vi.fn().mockResolvedValue({ ok: true });
const guides = [
  { id: 1, slug: 'book', title_en: 'Book a ride', title_zh: '', summary_en: '', summary_zh: '', audience: 'member', published: true, step_count: 2 },
  { id: 2, slug: 'plan', title_en: 'Plan transport', title_zh: '', summary_en: '', summary_zh: '', audience: 'staff', published: false, step_count: 3 },
];
vi.mock('./api', () => ({
  post: (...a: unknown[]) => post(...a),
  api: vi.fn(),
  useApi: (path: string | null) => ({
    data: path === '/help/guides' ? guides : path?.startsWith('/help') ? [{ id: 7, subject: 'Driver arrived early', user_name: 'Tan' }] : null,
    loading: false, error: null, reload: vi.fn(),
  }),
}));
vi.mock('./auth', () => ({ useAuth: () => ({ me: { id: 1 }, can: (p: string) => p === 'help' || p === 'admin_help', refresh: vi.fn() }) }));

describe('GuideArranger', () => {
  it('moves a guide and saves the whole order', async () => {
    const onDone = vi.fn();
    render(<MemoryRouter><GuideArranger guides={guides as never} onDone={onDone} /></MemoryRouter>);
    fireEvent.click(screen.getByRole('button', { name: 'Move Plan transport up' }));
    const items = within(screen.getByRole('list', { name: 'Arrange' })).getAllByRole('listitem');
    expect(items[0]).toHaveTextContent('Plan transport');
    fireEvent.click(screen.getByRole('button', { name: 'Save order' }));
    await waitFor(() => expect(onDone).toHaveBeenCalledWith(true));
    expect(post).toHaveBeenCalledWith('/help/guides/reorder', { ids: [2, 1] });
  });
});

describe('QuickJump', () => {
  it('filters pages, guides and requests and opens the chosen one with Enter', () => {
    const onClose = vi.fn();
    render(
      <MemoryRouter initialEntries={['/']}>
        <Routes>
          <Route path="/" element={<QuickJump pages={[{ to: '/events', label: 'Events', icon: null }]} onClose={onClose} />} />
          <Route path="/help/:id" element={<p>thread page</p>} />
        </Routes>
      </MemoryRouter>,
    );
    const box = screen.getByRole('combobox');
    fireEvent.change(box, { target: { value: 'driver' } });
    expect(screen.getAllByRole('option')).toHaveLength(1);
    expect(screen.getByRole('option')).toHaveTextContent('Driver arrived early · Tan');
    fireEvent.keyDown(box, { key: 'Enter' });
    expect(onClose).toHaveBeenCalled();
    expect(screen.getByText('thread page')).toBeInTheDocument();
  });
});
