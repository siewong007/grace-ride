import { describe, expect, it, vi } from 'vitest';
import { exportToCsv, parseLocal, RIDE_STATUS, REIMB_STATUS } from './format';

describe('parseLocal', () => {
  it('parses ISO local timestamps', () => {
    const d = parseLocal('2026-11-09T08:30:00');
    expect(d.getFullYear()).toBe(2026);
    expect(d.getMonth()).toBe(10);
    expect(d.getDate()).toBe(9);
    expect(d.getHours()).toBe(8);
    expect(d.getMinutes()).toBe(30);
  });

  it('accepts space separator', () => {
    const d = parseLocal('2026-11-09 20:15');
    expect(d.getHours()).toBe(20);
    expect(d.getMinutes()).toBe(15);
  });
});

describe('status maps', () => {
  it('covers ride and reimbursement states used by the app', () => {
    expect(RIDE_STATUS.pending_driver.tone).toBe('amber');
    expect(RIDE_STATUS.alt_transport.tone).toBe('blue');
    expect(RIDE_STATUS.no_show.tone).toBe('red');
    expect(REIMB_STATUS.rejected.tone).toBe('red');
    expect(REIMB_STATUS.paid.tone).toBe('green');
  });
});

describe('exportToCsv', () => {
  it('creates and clicks a download link with CSV data', () => {
    if (typeof URL.createObjectURL !== 'function') {
      URL.createObjectURL = vi.fn().mockReturnValue('blob:test');
    }
    if (typeof URL.revokeObjectURL !== 'function') {
      URL.revokeObjectURL = vi.fn();
    }
    const clickSpy = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});

    exportToCsv('test-export', ['Name', 'Role'], [['Alice', 'Driver'], ['Bob', 'Rider']]);

    expect(clickSpy).toHaveBeenCalled();
    clickSpy.mockRestore();
  });
});
