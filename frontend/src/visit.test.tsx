import { beforeAll, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import i18n from './i18n';
import { post } from './api';
import Visit from './pages/Visit';

const event = { title: 'Youth camp', starts_at: '2026-11-22T10:00', location: 'Church hall' };
const responses: Record<string, unknown> = {};

vi.mock('./api', () => ({
  useApi: (path: string | null) => ({
    data: path ? responses[path] ?? null : null,
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
  post: vi.fn().mockResolvedValue({ ok: true }),
}));

const show = (token: string) =>
  render(
    <MemoryRouter initialEntries={[`/visit/${token}`]}>
      <Routes>
        <Route path="/visit/:token" element={<Visit />} />
      </Routes>
    </MemoryRouter>,
  );

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

describe('Visitor page', () => {
  it('claims the link only when the visitor asks', async () => {
    responses['/visit/abc'] = { state: 'open', closed_reason: null, opens_at: '2026-11-21T09:00', name: 'Ann', host: 'Mary', event };
    show('abc');
    expect(screen.getByRole('heading', { name: /mary has arranged a ride/i })).toBeInTheDocument();
    expect(post).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole('button', { name: /open my ride/i }));
    expect(post).toHaveBeenCalledWith('/visit/abc/claim');
  });

  it('explains a link already opened elsewhere, and a finished journey', () => {
    responses['/visit/used'] = { state: 'used', closed_reason: null, opens_at: '', name: 'Ann', host: 'Mary', event };
    show('used');
    expect(screen.getByText(/ask mary to send you a new link/i)).toBeInTheDocument();
  });

  it('thanks the visitor once the ride is completed', () => {
    responses['/visit/done'] = { state: 'closed', closed_reason: 'completed', opens_at: '', name: 'Ann', host: 'Mary', event };
    show('done');
    expect(screen.getByRole('heading', { name: /thank you for coming/i })).toBeInTheDocument();
  });

  it('shows the claimed ride with its driver and ready buttons', () => {
    responses['/visit/mine'] = { state: 'mine', closed_reason: null, opens_at: '', name: 'Ann', host: 'Mary', event };
    responses['/visit/me'] = {
      name: 'Ann',
      host: { name: 'Mary', phone: '+60123456789' },
      event,
      ride: {
        status: 'confirmed', pickup_at: '2026-11-22T09:00', pickup_address: '1 Jalan Damai', area: 'PJ',
        destination: 'Church hall', readiness_status: 'unconfirmed', delay_minutes: null,
        arrived_at_gate_at: null, picked_up_at: null,
        driver: { name: 'Brother Silas', phone: '+60198765432', vehicle: { model: 'Vios', color: 'Silver', plate: 'WXX 8888' } },
      },
    };
    show('mine');
    expect(screen.getByRole('heading', { name: /your ride to youth camp/i })).toBeInTheDocument();
    expect(screen.getByText('Brother Silas')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /call driver/i })).toHaveAttribute('href', 'tel:+60198765432');
    expect(screen.getByRole('button', { name: /i’m ready/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /can’t make it/i })).toBeInTheDocument();
  });
});
