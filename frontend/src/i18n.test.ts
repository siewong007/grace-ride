import { beforeAll, describe, expect, it } from 'vitest';
import i18n from './i18n';
import { resources } from './locales';
import { apiMessage } from './api';
import { fmtAgo, fmtDay } from './format';

const pad = (n: number) => String(n).padStart(2, '0');
const local = (d: Date) =>
  `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

describe('placeholders', () => {
  it('fills single-brace placeholders used by the locale files', () => {
    expect(i18n.t('rideDetail:chatMembers', { count: 3 })).toBe('Only 3 members on this trip');
    expect(i18n.t('admin:settings.groupMembers', { name: 'Youth', count: 4 })).toBe('Youth members (4)');
  });

  it('fills placeholders in server error codes', () => {
    expect(apiMessage({ code: 'NOT_ENOUGH_DRIVER_SEATS', params: { load: 4, cap: 4 } }, 'network')).toBe(
      'Not enough driver seats: 4 booked, 4 available',
    );
  });

  it('formats relative times without a translator passed in', () => {
    expect(fmtAgo(local(new Date()))).toBe('Just now');
    expect(fmtAgo(local(new Date(Date.now() - 5 * 60_000)))).toBe('5 min ago');
    const far = new Date(2026, 2, 7, 9, 0);
    expect(fmtDay(local(far))).toMatch(/^3\/7 /);
  });

  it('has no leftover {{x}} or escaped quotes in any locale string', () => {
    const bad: string[] = [];
    const walk = (v: unknown, path: string) => {
      if (typeof v === 'string') {
        if (/\{\{|\\"|\w"\w/.test(v)) bad.push(`${path}: ${v}`);
      } else if (v && typeof v === 'object') {
        for (const [k, child] of Object.entries(v)) walk(child, `${path}.${k}`);
      }
    };
    walk(resources, 'locales');
    expect(bad).toEqual([]);
  });

  it('uses the same placeholder names in English and Chinese', () => {
    const vars = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort().join(',');
    const diffs: string[] = [];
    const walk = (en: unknown, zh: unknown, path: string) => {
      if (typeof en === 'string' && typeof zh === 'string') {
        if (vars(en) !== vars(zh)) diffs.push(path);
      } else if (en && zh && typeof en === 'object' && typeof zh === 'object') {
        for (const k of Object.keys(en)) walk((en as any)[k], (zh as any)[k], `${path}.${k}`);
      }
    };
    walk((resources as any).en, (resources as any).zh, '');
    expect(diffs).toEqual([]);
  });
});
