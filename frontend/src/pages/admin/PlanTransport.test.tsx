import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import PlanTransport from './PlanTransport';
import * as apiMod from '../../api';
import * as authMod from '../../auth';

vi.mock('../../auth', () => ({
  useAuth: vi.fn(),
}));

const planFixture = {
  event: { id: 5, title: 'Sunday Service', starts_at: '2026-11-22T09:00:00', location: 'Church Hall', status: 'published' },
  rides: [
    {
      id: 11,
      status: 'pending_driver',
      pickup_at: '2026-11-22T08:15:00',
      pickup_address: '10 Jalan SS2',
      seats: 1,
      assistance: '',
      driver_id: null,
      area: { id: 1, name: 'SS2' },
      passenger: { id: 20, name: 'Uncle Lim' },
      last: null,
    },
  ],
  drivers: [
    {
      id: 30,
      name: 'Driver Grace',
      carpool: true,
      cap: 4,
      other_seats: 0,
      other_rides: 0,
      area_ids: [1],
      vehicle: { model: 'Innova', plate: 'VAA 1111', seats: 4 },
    },
  ],
};

describe('PlanTransport draft & traffic lead approval', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders Save Draft and Ask Lead to Approve for traffic group member', async () => {
    (authMod.useAuth as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      can: () => true,
      me: { id: 2, name: 'Member Bob', roles: ['traffic_member'] },
    });

    vi.spyOn(apiMod, 'useApi').mockReturnValue({
      data: {
        ...planFixture,
        can_approve: false,
        draft: null,
      },
      error: null,
      loading: false,
      refreshing: false,
      reload: vi.fn(),
      setData: vi.fn(),
    });

    const postSpy = vi.spyOn(apiMod, 'post').mockResolvedValue({ ok: true, status: 'draft' });

    render(
      <MemoryRouter initialEntries={['/admin/events/5/plan']}>
        <Routes>
          <Route path="/admin/events/:id/plan" element={<PlanTransport />} />
        </Routes>
      </MemoryRouter>,
    );

    const saveDraftBtn = screen.getByRole('button', { name: /save draft/i });
    const askApprovalBtn = screen.getByRole('button', { name: /ask lead to approve/i });

    expect(saveDraftBtn).toBeInTheDocument();
    expect(askApprovalBtn).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /approve & notify/i })).not.toBeInTheDocument();

    await userEvent.click(saveDraftBtn);
    expect(postSpy).toHaveBeenCalledWith('/admin/events/5/plan/draft', expect.objectContaining({
      assignments: expect.any(Array),
    }));
  });

  it('renders pending approval banner and allows traffic lead to approve or return', async () => {
    (authMod.useAuth as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      can: () => true,
      me: { id: 18, name: 'Lead Zhang', roles: ['traffic_head'] },
    });

    vi.spyOn(apiMod, 'useApi').mockReturnValue({
      data: {
        ...planFixture,
        can_approve: true,
        draft: {
          status: 'pending_approval',
          assignments: [{ ride_id: 11, driver_id: 30 }],
          submitted_by: { id: 2, name: 'Member Bob' },
          submitted_at: '2026-11-20T10:00:00',
        },
      },
      error: null,
      loading: false,
      refreshing: false,
      reload: vi.fn(),
      setData: vi.fn(),
    });

    const postSpy = vi.spyOn(apiMod, 'post').mockResolvedValue({ ok: true });
    vi.spyOn(window, 'prompt').mockReturnValue('Need to adjust route');

    render(
      <MemoryRouter initialEntries={['/admin/events/5/plan']}>
        <Routes>
          <Route path="/admin/events/:id/plan" element={<PlanTransport />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByText(/pending lead approval/i)).toBeInTheDocument();
    expect(screen.getByText(/Member Bob/)).toBeInTheDocument();

    const returnBtn = screen.getByRole('button', { name: /return for changes/i });
    const approveBtn = screen.getByRole('button', { name: /approve & notify/i });

    expect(returnBtn).toBeInTheDocument();
    expect(approveBtn).toBeInTheDocument();

    await userEvent.click(returnBtn);
    expect(postSpy).toHaveBeenCalledWith('/admin/events/5/plan/return', { notes: 'Need to adjust route' });
  });

  it('allows traffic group member to export assignment image and view high-quality preview modal', async () => {
    (authMod.useAuth as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      can: () => true,
      me: { id: 2, name: 'Member Bob', roles: ['traffic_member'] },
    });

    vi.spyOn(apiMod, 'useApi').mockReturnValue({
      data: {
        ...planFixture,
        can_approve: false,
        draft: null,
      },
      error: null,
      loading: false,
      refreshing: false,
      reload: vi.fn(),
      setData: vi.fn(),
    });

    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({
      fillStyle: '',
      strokeStyle: '',
      lineWidth: 1,
      font: '',
      textAlign: '',
      textBaseline: '',
      fillRect: vi.fn(),
      strokeRect: vi.fn(),
      fillText: vi.fn(),
      beginPath: vi.fn(),
      moveTo: vi.fn(),
      lineTo: vi.fn(),
      arcTo: vi.fn(),
      closePath: vi.fn(),
      fill: vi.fn(),
      stroke: vi.fn(),
      roundRect: vi.fn(),
      scale: vi.fn(),
      measureText: vi.fn((str: string) => ({ width: str.length * 8 })),
    } as unknown as CanvasRenderingContext2D);
    vi.spyOn(HTMLCanvasElement.prototype, 'toDataURL').mockReturnValue('data:image/png;base64,mockImageDataUrl');

    render(
      <MemoryRouter initialEntries={['/admin/events/5/plan']}>
        <Routes>
          <Route path="/admin/events/:id/plan" element={<PlanTransport />} />
        </Routes>
      </MemoryRouter>,
    );

    const exportBtn = screen.getByRole('button', { name: /export.*image/i });
    expect(exportBtn).toBeInTheDocument();

    await userEvent.click(exportBtn);

    // Modal opens with preview and action buttons
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /download image/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /copy image/i })).toBeInTheDocument();

    const previewImg = screen.getByRole('img', { name: /export.*image/i });
    expect(previewImg).toHaveAttribute('src', 'data:image/png;base64,mockImageDataUrl');
  });

  it('allows traffic lead to export final assignment image when approved', async () => {
    (authMod.useAuth as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      can: () => true,
      me: { id: 18, name: 'Lead Zhang', roles: ['traffic_head'] },
    });

    vi.spyOn(apiMod, 'useApi').mockReturnValue({
      data: {
        ...planFixture,
        can_approve: true,
        draft: {
          status: 'approved',
          assignments: [{ ride_id: 11, driver_id: 30 }],
          reviewed_by: { id: 18, name: 'Lead Zhang' },
          reviewed_at: '2026-11-20T11:00:00',
        },
      },
      error: null,
      loading: false,
      refreshing: false,
      reload: vi.fn(),
      setData: vi.fn(),
    });

    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({
      fillStyle: '',
      strokeStyle: '',
      lineWidth: 1,
      font: '',
      textAlign: '',
      textBaseline: '',
      fillRect: vi.fn(),
      strokeRect: vi.fn(),
      fillText: vi.fn(),
      beginPath: vi.fn(),
      moveTo: vi.fn(),
      lineTo: vi.fn(),
      arcTo: vi.fn(),
      closePath: vi.fn(),
      fill: vi.fn(),
      stroke: vi.fn(),
      roundRect: vi.fn(),
      scale: vi.fn(),
      measureText: vi.fn((str: string) => ({ width: str.length * 8 })),
    } as unknown as CanvasRenderingContext2D);
    vi.spyOn(HTMLCanvasElement.prototype, 'toDataURL').mockReturnValue('data:image/png;base64,mockApprovedDataUrl');

    render(
      <MemoryRouter initialEntries={['/admin/events/5/plan']}>
        <Routes>
          <Route path="/admin/events/:id/plan" element={<PlanTransport />} />
        </Routes>
      </MemoryRouter>,
    );

    const exportBtn = screen.getByRole('button', { name: /export.*image/i });
    expect(exportBtn).toBeInTheDocument();

    await userEvent.click(exportBtn);

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    const previewImg = screen.getByRole('img', { name: /export.*image/i });
    expect(previewImg).toHaveAttribute('src', 'data:image/png;base64,mockApprovedDataUrl');
  });
});
