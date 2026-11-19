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
      path === '/admin/settings/fares'
        ? { fares: [{ id: 7, area_a: { id: 1, name: 'Cheras' }, area_b: { id: 2, name: 'PJ' }, amount_cents: 1500, updated_at: '' }] }
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
  it('lists area fares and saves one in ringgit, stored as cents', async () => {
    show();
    expect(screen.getByText('Cheras ↔ PJ')).toBeInTheDocument();
    expect(screen.getByText('RM 15.00')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
    fireEvent.change(screen.getByLabelText('Fare'), { target: { value: '18.50' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save fare' }));
    await waitFor(() =>
      expect(mocks.put).toHaveBeenCalledWith('/admin/settings/fares', {
        area_a_id: 1,
        area_b_id: 2,
        amount_cents: 1850,
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
