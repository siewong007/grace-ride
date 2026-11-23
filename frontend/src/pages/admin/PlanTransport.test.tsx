import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { ConfirmProvider } from '../../components/ui';
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

    render(
      <MemoryRouter initialEntries={['/admin/events/5/plan']}>
        <Routes>
          <Route path="/admin/events/:id/plan" element={<PlanTransport />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getAllByText(/pending lead approval/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/Member Bob/)).toBeInTheDocument();

    const returnBtn = screen.getByRole('button', { name: /return for changes/i });
    const approveBtn = screen.getByRole('button', { name: /approve & notify/i });

    expect(returnBtn).toBeInTheDocument();
    expect(approveBtn).toBeInTheDocument();

    await userEvent.click(returnBtn);
    const dialog = screen.getByRole('dialog');
    await userEvent.type(within(dialog).getByLabelText(/return reason or suggestions/i), 'Need to adjust route');
    await userEvent.click(within(dialog).getByRole('button', { name: /return draft/i }));
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
      save: vi.fn(),
      restore: vi.fn(),
      setLineDash: vi.fn(),
      clip: vi.fn(),
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
      save: vi.fn(),
      restore: vi.fn(),
      setLineDash: vi.fn(),
      clip: vi.fn(),
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

  it('shows read-only lock banner when locked by another member and lets Lead take over editing', async () => {
    (authMod.useAuth as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      can: () => true,
      me: { id: 18, name: 'Lead Zhang', roles: ['traffic_head'] },
    });

    vi.spyOn(apiMod, 'useApi').mockReturnValue({
      data: {
        ...planFixture,
        can_approve: true,
        drafts: [
          {
            id: 7,
            version: 1,
            label: 'Sunday Draft',
            status: 'draft',
            assignments: [{ ride_id: 11, driver_id: 30 }],
            visitors: [],
            locked_by: { id: 2, name: 'Member Bob' },
            lock_expires_at: '2099-01-01T12:00:00',
            lock_is_mine: false,
          },
        ],
        draft: {
          id: 7,
          version: 1,
          label: 'Sunday Draft',
          status: 'draft',
          assignments: [{ ride_id: 11, driver_id: 30 }],
          visitors: [],
          locked_by: { id: 2, name: 'Member Bob' },
          lock_expires_at: '2099-01-01T12:00:00',
          lock_is_mine: false,
        },
      },
      error: null,
      loading: false,
      refreshing: false,
      reload: vi.fn(),
      setData: vi.fn(),
    });

    const postSpy = vi.spyOn(apiMod, 'post').mockResolvedValue({ ok: true });

    render(
      <MemoryRouter initialEntries={['/admin/events/5/plan?draft=7']}>
        <Routes>
          <Route path="/admin/events/:id/plan" element={<PlanTransport />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByText(/currently locked by Member Bob/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /save draft/i })).toBeDisabled();

    const takeoverBtn = screen.getByRole('button', { name: /take over editing/i });
    await userEvent.click(takeoverBtn);
    expect(postSpy).toHaveBeenCalledWith('/admin/events/5/plan/drafts/7/lock', { force: true });
  });

  it('enables Approve & notify on zero-diff pending draft and shows Lead diff summary', async () => {
    (authMod.useAuth as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      can: () => true,
      me: { id: 18, name: 'Lead Zhang', roles: ['traffic_head'] },
    });

    vi.spyOn(apiMod, 'useApi').mockReturnValue({
      data: {
        ...planFixture,
        can_approve: true,
        drafts: [
          {
            id: 9,
            version: 1,
            label: '',
            status: 'pending_approval',
            assignments: [{ ride_id: 11, driver_id: null }],
            visitors: [],
            submitted_by: { id: 2, name: 'Member Bob' },
            submitted_at: '2026-11-20T10:00:00',
          },
        ],
        draft: {
          id: 9,
          version: 1,
          label: '',
          status: 'pending_approval',
          assignments: [{ ride_id: 11, driver_id: null }],
          visitors: [],
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

    render(
      <ConfirmProvider>
        <MemoryRouter initialEntries={['/admin/events/5/plan?draft=9']}>
          <Routes>
            <Route path="/admin/events/:id/plan" element={<PlanTransport />} />
          </Routes>
        </MemoryRouter>
      </ConfirmProvider>,
    );

    expect(screen.getByText(/changes ready for lead approval/i)).toBeInTheDocument();
    expect(screen.getByText(/no ride assignment changes/i)).toBeInTheDocument();
    const approveBtn = screen.getByRole('button', { name: /approve & notify \(0\)/i });
    expect(approveBtn).toBeEnabled();

    await userEvent.click(approveBtn);
    const dialog = screen.getByRole('dialog');
    await userEvent.click(within(dialog).getByRole('button', { name: /^approve$/i }));
    expect(postSpy).toHaveBeenCalledWith('/admin/events/5/plan/approve', { draft_id: 9, changes: [] });
  });

  it('supports draft version switcher, creating a new draft, and draft visitors for traffic member', async () => {
    (authMod.useAuth as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      can: () => true,
      me: { id: 2, name: 'Member Bob', roles: ['traffic_member'] },
    });

    const boardData = {
      ...planFixture,
      can_approve: false,
      drafts: [
        {
          id: 12,
          version: 1,
          label: 'Plan A',
          status: 'draft',
          assignments: [{ ride_id: 11, driver_id: 30 }],
          visitors: [],
          created_by: { id: 2, name: 'Member Bob' },
        },
      ],
      draft: {
        id: 12,
        version: 1,
        label: 'Plan A',
        status: 'draft',
        assignments: [{ ride_id: 11, driver_id: 30 }],
        visitors: [],
        created_by: { id: 2, name: 'Member Bob' },
      },
      visitors: [],
    };
    vi.spyOn(apiMod, 'useApi').mockImplementation(((path: string | null) =>
      path === '/areas'
        ? { data: [{ id: 1, name: 'SS2', active: true }], error: null, loading: false, refreshing: false, reload: vi.fn(), setData: vi.fn() }
        : { data: boardData, error: null, loading: false, refreshing: false, reload: vi.fn(), setData: vi.fn() }) as unknown as typeof apiMod.useApi);

    const postSpy = vi.spyOn(apiMod, 'post').mockResolvedValue({ ok: true, draft: { id: 13, version: 2 } });

    render(
      <MemoryRouter initialEntries={['/admin/events/5/plan?draft=12']}>
        <Routes>
          <Route path="/admin/events/:id/plan" element={<PlanTransport />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByRole('button', { name: /v1 · Plan A/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Live' })).toBeInTheDocument();
    // Empty visitors starts collapsed behind filter chip; expanding lets member add a visitor in draftMode.
    await userEvent.click(screen.getByRole('button', { name: /visitors \(0\)/i }));
    await userEvent.click(screen.getByRole('button', { name: 'Add visitor' }));
    const visitorDlg = screen.getByRole('dialog');
    await userEvent.type(within(visitorDlg).getByLabelText('Name'), 'Draft Guest');
    await userEvent.click(within(visitorDlg).getByRole('button', { name: 'Add visitor' }));
    expect(screen.getByText('Draft Guest')).toBeInTheDocument();
    expect(postSpy).not.toHaveBeenCalledWith('/admin/events/5/plan/visitors', expect.anything());

    // Member can create a new draft version.
    await userEvent.click(screen.getByRole('button', { name: /new draft/i }));
    const newDraftDlg = screen.getByRole('dialog');
    await userEvent.type(within(newDraftDlg).getByLabelText(/label/i), 'Plan B');
    await userEvent.click(within(newDraftDlg).getByRole('button', { name: /create draft/i }));
    expect(postSpy).toHaveBeenCalledWith('/admin/events/5/plan/drafts', expect.objectContaining({
      label: 'Plan B',
    }));
  });
});
