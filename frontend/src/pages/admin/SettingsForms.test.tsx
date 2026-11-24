import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import i18n from '../../i18n';
import { ConfirmProvider } from '../../components/ui';
import Settings from './Settings';

const mocks = vi.hoisted(() => ({
  put: vi.fn(),
  post: vi.fn(),
  can: (() => true) as (page: string, act?: string) => boolean,
  alertsData: null as Record<string, unknown> | null,
}));

vi.mock('../../api', () => ({
  put: mocks.put,
  post: mocks.post,
  del: vi.fn(),
  useApi: (path: string | null) => ({
    data:
      path === '/admin/settings/fares'
        ? { fares: [{ id: 7, area_a: { id: 1, name: 'Cheras' }, area_b: { id: 2, name: 'PJ' }, amount_cents: 1500, updated_at: '' }] }
        : path === '/admin/settings/system'
          ? { audit_retention_days: 730, chat_retention_days: 180, max_live_drivers: 10, session_timeout_mins: 15 }
          : path === '/admin/settings/alerts'
            ? mocks.alertsData
            : path === '/areas'
              ? [
                  { id: 1, name: 'Cheras', active: true },
                  { id: 2, name: 'PJ', active: true },
                  { id: 3, name: 'KK', active: true },
                  { id: 4, name: 'Old Town', active: false },
                ]
              : [],
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
}));
vi.mock('../../auth', () => ({ useAuth: () => ({ can: (page: string, act?: string) => mocks.can(page, act) }) }));

beforeAll(async () => {
  await i18n.changeLanguage('en');
});
beforeEach(() => {
  mocks.put.mockReset().mockResolvedValue({});
  mocks.post.mockReset().mockResolvedValue({ ok: true, status_code: 200, latency_ms: 42 });
  mocks.can = () => true;
  mocks.alertsData = null;
});

const show = (url = '/admin/settings') =>
  render(
    <MemoryRouter initialEntries={[url]}>
      <ConfirmProvider>
        <Settings />
      </ConfirmProvider>
    </MemoryRouter>,
  );

describe('Settings forms', () => {
  it('shows every route from an area, flags unset fares and saves one in ringgit, stored as cents', async () => {
    show();
    // 3 open areas: 6 routes including within one area; Cheras ↔ PJ is the only one priced.
    expect(screen.getByText('1 of 6 routes priced')).toBeInTheDocument();
    expect(screen.getByText('5 not set')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Old Town/ })).not.toBeInTheDocument();
    expect(screen.getByText('Within Cheras')).toBeInTheDocument();
    expect(screen.getByText('Cheras ↔ KK')).toBeInTheDocument();
    expect(screen.getAllByText('Not set')).toHaveLength(2);

    const pj = screen.getByLabelText('Fare for Cheras ↔ PJ');
    expect(pj).toHaveValue(15);
    fireEvent.change(pj, { target: { value: '18.50' } });
    fireEvent.click(screen.getAllByRole('button', { name: 'Save' }).at(-1)!);
    await waitFor(() =>
      expect(mocks.put).toHaveBeenCalledWith('/admin/settings/fares', {
        area_a_id: 1,
        area_b_id: 2,
        amount_cents: 1850,
      }),
    );

    // Switching area lists that area's routes; the pair is the same from either side.
    fireEvent.click(screen.getByRole('button', { name: /^KK/ }));
    const kk = screen.getByLabelText('Fare for KK ↔ PJ');
    fireEvent.change(kk, { target: { value: '12' } });
    fireEvent.submit(kk.closest('form')!);
    await waitFor(() =>
      expect(mocks.put).toHaveBeenCalledWith('/admin/settings/fares', {
        area_a_id: 3,
        area_b_id: 2,
        amount_cents: 1200,
      }),
    );
  });

  it('shows fares read-only to someone who may view but not change them', () => {
    mocks.can = (page, act) => page === 'admin_fares' && act !== 'update';
    show();
    expect(screen.getByText('RM 15.00')).toBeInTheDocument();
    expect(screen.queryByLabelText('Fare for Cheras ↔ PJ')).not.toBeInTheDocument();
    expect(screen.queryByRole('tab', { name: 'Data & limits' })).not.toBeInTheDocument();
  });

  it('explains an out-of-range limit and will not save it, instead of changing it silently', async () => {
    show('/admin/settings?tab=rates');
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

  it('never re-submits masked alert_webhook_url when editing Telegram username, and tests webhook + shows 24h health', async () => {
    mocks.alertsData = {
      telegram_bot_configured: true,
      telegram_bot_username: 'OldBot',
      telegram_webhook_path: '/api/push/telegram/webhook/abc',
      alert_webhook_configured: true,
      alert_webhook_url: 'https://hooks.example.org/***',
      alert_webhook_secret_configured: true,
      mobile_push_webhook_configured: false,
      mobile_push_subscriptions_count: 2,
      vapid_public_key: 'VAPID_PUB',
      vapid_private_configured: true,
      push_subscriptions_count: 3,
      delivery_stats_24h: {
        web_push: { delivered: 12, retry: 1, failed: 0, gone: 1 },
      },
    };
    show('/admin/settings?tab=alerts');

    expect(screen.getByText(/web_push: 12 delivered/i)).toBeInTheDocument();
    const webhookInput = screen.getByLabelText('WhatsApp / Alert webhook URL') as HTMLInputElement;
    expect(webhookInput.value).toBe('');

    fireEvent.change(screen.getByLabelText('Telegram bot username'), {
      target: { value: 'NewBot' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Save alert channels' }));
    await waitFor(() =>
      expect(mocks.put).toHaveBeenCalledWith(
        '/admin/settings/alerts',
        expect.objectContaining({
          telegram_bot_username: 'NewBot',
          alert_webhook_url: undefined,
        }),
      ),
    );

    fireEvent.click(screen.getByRole('button', { name: 'Send test webhook' }));
    await waitFor(() => expect(mocks.post).toHaveBeenCalledWith('/admin/settings/alerts/test-webhook'));
    expect(await screen.findByText(/HTTP 200 in 42 ms/i)).toBeInTheDocument();
  });
});
