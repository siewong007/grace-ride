import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import i18n from '../../i18n';
import { ConfirmProvider } from '../../components/ui';
import Settings from './Settings';

const mocks = vi.hoisted(() => ({ put: vi.fn() }));

vi.mock('../../api', () => ({
  put: mocks.put,
  post: vi.fn(),
  del: vi.fn(),
  useApi: (path: string | null) => ({
    data:
      path === '/admin/settings/rates'
        ? { rates: [{ vehicle_type: 'standard', rate_cents_per_km: 60, updated_at: '' }, { vehicle_type: 'other', rate_cents_per_km: 40, updated_at: '' }] }
        : path === '/admin/settings/system'
          ? { audit_retention_days: 730, chat_retention_days: 180, max_live_drivers: 10, session_timeout_mins: 15 }
          : [],
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
}));
vi.mock('../../auth', () => ({ useAuth: () => ({ can: () => true }) }));

beforeAll(async () => {
  await i18n.changeLanguage('en');
});
beforeEach(() => mocks.put.mockReset().mockResolvedValue({}));

const show = () =>
  render(
    <MemoryRouter>
      <ConfirmProvider>
        <Settings />
      </ConfirmProvider>
    </MemoryRouter>,
  );

describe('Settings forms', () => {
  it('shows and saves mileage rates in ringgit, stored as cents', async () => {
    show();
    const std = screen.getByLabelText('Standard vehicle (RM per km)');
    expect(std).toHaveValue(0.6);
    fireEvent.change(std, { target: { value: '0.75' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save rates' }));
    await waitFor(() =>
      expect(mocks.put).toHaveBeenCalledWith('/admin/settings/rates', {
        rates: [
          { vehicle_type: 'standard', rate_cents_per_km: 75 },
          { vehicle_type: 'other', rate_cents_per_km: 40 },
        ],
      }),
    );
  });

  it('explains an out-of-range limit and will not save it, instead of changing it silently', async () => {
    show();
    const audit = screen.getByLabelText(/audit log retention/i);
    fireEvent.change(audit, { target: { value: '5' } });
    expect(await screen.findByText('Enter a whole number from 30 to 3650.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Save system settings' })).toBeDisabled();

    fireEvent.change(audit, { target: { value: '365' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save system settings' }));
    await waitFor(() =>
      expect(mocks.put).toHaveBeenCalledWith('/admin/settings/system', {
        audit_retention_days: 365,
        chat_retention_days: 180,
        max_live_drivers: 10,
        session_timeout_mins: 15,
      }),
    );
  });
});
