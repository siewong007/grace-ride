import { afterEach, describe, expect, it, vi } from 'vitest';
import { act, render, screen, waitFor } from '@testing-library/react';
import { AuthProvider, useAuth } from './auth';
import * as apiMod from './api';
import { createPdfWriter, isLatin } from './pdfWriter';
import { buildXlsxWorkbook, readXlsxWorkbook } from './xlsxCodec';
import { escapeCsv, formatInclusivePeriodEnd } from './reportExport';
import {
  clearOfflineSnapshots,
  loadTodayRidesSnapshot,
  saveTodayRidesSnapshot,
} from './offlineSnapshot';
import errorsEn from './locales/errors.en';
import errorsZh from './locales/errors.zh';
import type { Ride } from './components/RideCard';

vi.mock('jspdf', () => ({
  default: class {
    setFont() {}
    setFontSize() {}
    splitTextToSize(s: string) {
      return s.split('\n');
    }
    getTextWidth(s: string) {
      return s.length * 2;
    }
    text() {}
    setFillColor() {}
    rect() {}
    addPage() {}
    save() {}
  },
}));

describe('audit regression fixes', () => {
  afterEach(() => {
    vi.restoreAllMocks();
    localStorage.clear();
  });

  it('pdfWriter handles CP1252 Latin detection and preserves explicit newlines cleanly', async () => {
    expect(isLatin('Hello RM 12.50 – Grace’s €5 résumé')).toBe(true);
    expect(isLatin('\u6069\u5178\u540c\u884c Sunday')).toBe(false);
    expect(isLatin('Ride \ud83d\ude97')).toBe(false);

    const w = await createPdfWriter(['Hello – Grace’s €5\nLine two']);
    const used = w.text('First line\nSecond line', 10, 20, { maxWidth: 180 });
    expect(used).toBeGreaterThan(0);
  });

  it('xlsxCodec strips illegal XML control characters during export', async () => {
    const bytes = buildXlsxWorkbook('AuditSheet', [[{ value: 'Safe\x00Text\x08Here\x1F!\tOK' }]]);
    const parsed = await readXlsxWorkbook(bytes);
    expect(parsed.sheets).toHaveLength(1);
    expect(parsed.sheets[0].rows[0][0].value).toBe('SafeTextHere!\tOK');
  });

  it('reportExport escapes formulas and formats half-open boundaries', () => {
    expect(escapeCsv('=cmd|calc')).toBe("'=cmd|calc");
    expect(escapeCsv('  -2+3')).toBe("'  -2+3");
    expect(escapeCsv('\t@SUM(A1)')).toBe("'\t@SUM(A1)");
    expect(formatInclusivePeriodEnd('2026-03-01')).toBe('2026-02-28');
    expect(formatInclusivePeriodEnd('2024-03-01')).toBe('2024-02-29');
    expect(formatInclusivePeriodEnd('2026-03-15')).toBe('2026-03-15');
    expect(formatInclusivePeriodEnd('2026-03-01', '2026-02-27')).toBe('2026-02-27');
  });

  it('offlineSnapshot clears empty ride list without wiping queued offline actions', () => {
    const sampleRide: Ride = {
      id: 7,
      status: 'confirmed',
      pickup_at: (() => {
        const d = new Date();
        const pad = (n: number) => String(n).padStart(2, '0');
        return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T09:00:00`;
      })(),
      destination: 'Grace Church',
      area: { id: 1, name: 'PJ' },
      pickup_address: '10 Jalan SS2',
      seats: 1,
      assistance: '',
      notes: '',
      event: null,
      trip_id: null,
      passenger: { id: 2, name: 'Member', phone: null },
      driver: { id: 3, name: 'Driver', phone: null, vehicle: null },
      needs_attention: false,
      attention_reason: '',
      started_at: null,
      picked_up_at: null,
      completed_at: null,
      updated_at: '2026-11-25T08:00:00',
      viewer: { is_passenger: true, is_driver: false, is_staff: false },
    };

    localStorage.setItem('gr_offline_actions_v1', JSON.stringify([{ id: 'q1' }]));
    saveTodayRidesSnapshot(2, [sampleRide]);
    expect(loadTodayRidesSnapshot(2)?.rides).toHaveLength(1);

    // Terminal update drops today's ride and removes the snapshot without touching queued actions.
    saveTodayRidesSnapshot(2, [{ ...sampleRide, status: 'completed' }]);
    expect(loadTodayRidesSnapshot(2)).toBeNull();
    expect(localStorage.getItem('gr_offline_actions_v1')).not.toBeNull();

    clearOfflineSnapshots();
    expect(localStorage.getItem('gr_offline_actions_v1')).toBeNull();
  });

  it('AuthProvider keeps me on transient non-401 error and clears on 401/unauthorized', async () => {
    const me = {
      id: 5,
      name: 'Grace Member',
      email: 'm@example.org',
      phone: '+60123456789',
      status: 'active',
      lang: 'en',
      terms_accepted: true,
      mfa_ok: false,
      totp_enabled: false,
      has_password: true,
      passkey_count: 1,
      roles: ['member'],
      permissions: [],
    };
    sessionStorage.setItem('graceride_walkthrough', 'done');
    localStorage.setItem('graceride_help_draft_5', 'draft text');

    const apiSpy = vi.spyOn(apiMod, 'api');
    apiSpy.mockResolvedValueOnce(me);

    function Harness() {
      const { me: cur, refresh } = useAuth();
      return (
        <div>
          <span data-testid="user">{cur ? cur.name : 'none'}</span>
          <button onClick={() => void refresh()}>refresh</button>
        </div>
      );
    }

    render(
      <AuthProvider>
        <Harness />
      </AuthProvider>,
    );

    await waitFor(() => expect(screen.getByTestId('user')).toHaveTextContent('Grace Member'));

    // Non-401 (e.g. 503 network hiccup) must preserve existing me
    apiSpy.mockRejectedValueOnce(new apiMod.ApiError(503, 'Service unavailable'));
    await act(async () => {
      screen.getByRole('button', { name: 'refresh' }).click();
    });
    expect(screen.getByTestId('user')).toHaveTextContent('Grace Member');

    // Unauthorized event clears me & user-scoped drafts
    await act(async () => {
      window.dispatchEvent(new Event('gr:unauthorized'));
    });
    await waitFor(() => expect(screen.getByTestId('user')).toHaveTextContent('none'));
    expect(sessionStorage.getItem('graceride_walkthrough')).toBeNull();
    expect(localStorage.getItem('graceride_help_draft_5')).toBeNull();
  });

  it('errors.en and errors.zh define matching non-empty keys', () => {
    const enKeys = Object.keys(errorsEn).sort();
    const zhKeys = Object.keys(errorsZh).sort();
    expect(enKeys).toEqual(zhKeys);
    for (const key of enKeys) {
      expect(String(errorsEn[key as keyof typeof errorsEn]).trim().length).toBeGreaterThan(0);
      expect(String(errorsZh[key as keyof typeof errorsZh]).trim().length).toBeGreaterThan(0);
    }
  });

  it('escapes | and % formula prefixes and blocks DTD entities in XLSX XML', async () => {
    const { escapeCsv } = await import('./reportExport');
    const { escapeCsvCell } = await import('./format');
    expect(escapeCsv('|calc')).toBe("'|calc");
    expect(escapeCsv('%CMD%')).toBe("'%CMD%");
    expect(escapeCsvCell('  |cmd')).toBe('"\'  |cmd"');

    const { assertSafeXlsxXml, buildXlsxWorkbook, readXlsxWorkbook } = await import('./xlsxCodec');
    expect(() => assertSafeXlsxXml('<!DOCTYPE lolz><a/>')).toThrow();
    expect(() => assertSafeXlsxXml('<!ENTITY x "y"><a/>')).toThrow();
    const bytes = buildXlsxWorkbook('Sheet1', [[{ value: 'a' }]]);
    const parsed = await readXlsxWorkbook(bytes);
    expect(parsed.sheets[0].rows[0][0].value).toBe('a');
  });

  it('clearClientSessionStorage wipes sensitive keys and aborts in-flight offline queue flush', async () => {
    localStorage.setItem('gr_last_active', '123');
    localStorage.setItem('gr_device_passkey', '{"email":"a@b.c","savedAt":1}');
    localStorage.setItem('gr-share-loc-42', '1');
    localStorage.setItem('graceride_help_draft_1', '{"body":"secret"}');
    sessionStorage.setItem('graceride_walkthrough', '[]');

    apiMod.clearClientSessionStorage();
    expect(localStorage.getItem('gr_last_active')).toBeNull();
    expect(localStorage.getItem('gr_device_passkey')).toBeNull();
    expect(localStorage.getItem('gr-share-loc-42')).toBeNull();
    expect(localStorage.getItem('graceride_help_draft_1')).toBeNull();
    expect(sessionStorage.getItem('graceride_walkthrough')).toBeNull();

    const queueMod = await import('./offlineQueue');
    queueMod.enqueueOfflineAction({ userId: 7, rideId: 10, path: '/rides/10/readiness', body: { status: 'ready' }, label: 'Ready' });
    const postSpy = vi.spyOn(apiMod, 'post').mockImplementation(async () => {
      // Simulate account sign-out happening mid-flush
      queueMod.clearOfflineQueue();
      return { ok: true };
    });
    queueMod.enqueueOfflineAction({ userId: 7, rideId: 11, path: '/rides/11/readiness', body: { status: 'ready' }, label: 'Ready 2' });
    const res = await queueMod.flushOfflineQueue(7);
    expect(postSpy).toHaveBeenCalledTimes(1);
    expect(res.remaining).toBe(0);
    postSpy.mockRestore();
  });

  it('AuthProvider keeps user signed out when gr_pending_logout is set offline', async () => {
    localStorage.setItem('gr_pending_logout', '1');
    const postSpy = vi.spyOn(apiMod, 'post').mockRejectedValue(new TypeError('Failed to fetch'));
    const apiSpy = vi.spyOn(apiMod, 'api');

    function Harness() {
      const { me, loading } = useAuth();
      return <span data-testid="state">{loading ? 'loading' : me ? me.name : 'signed-out'}</span>;
    }
    render(
      <AuthProvider>
        <Harness />
      </AuthProvider>,
    );
    await waitFor(() => expect(screen.getByTestId('state')).toHaveTextContent('signed-out'));
    expect(apiSpy).not.toHaveBeenCalled();
    localStorage.removeItem('gr_pending_logout');
    postSpy.mockRestore();
    apiSpy.mockRestore();
  });
});
