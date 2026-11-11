import fs from 'node:fs';
import path from 'node:path';
import { beforeAll, describe, expect, it } from 'vitest';
import i18n from './i18n';
import { resources } from './locales';
import { apiMessage } from './api';
import { fmtAgo, fmtDay, HELP_STATUS, SETTLEMENT_STATUS } from './format';

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

  it('has identical key structure across English and Chinese', () => {
    const missingInZh: string[] = [];
    const missingInEn: string[] = [];
    const checkKeys = (a: Record<string, unknown>, b: Record<string, unknown>, prefix: string, missing: string[]) => {
      for (const k of Object.keys(a)) {
        const next = prefix ? `${prefix}.${k}` : k;
        if (!(k in b)) {
          missing.push(next);
        } else if (typeof a[k] === 'object' && a[k] !== null && typeof b[k] === 'object' && b[k] !== null) {
          checkKeys(a[k] as Record<string, unknown>, b[k] as Record<string, unknown>, next, missing);
        }
      }
    };
    checkKeys(resources.en as any, resources.zh as any, '', missingInZh);
    checkKeys(resources.zh as any, resources.en as any, '', missingInEn);
    expect(missingInZh).toEqual([]);
    expect(missingInEn).toEqual([]);
  });

  it('defines valid format keys for SETTLEMENT_STATUS and HELP_STATUS', () => {
    for (const item of Object.values(SETTLEMENT_STATUS)) {
      expect(i18n.exists(item.label)).toBe(true);
      expect(item.tone).toBeDefined();
    }
    for (const item of Object.values(HELP_STATUS)) {
      expect(i18n.exists(item.label)).toBe(true);
      expect(item.tone).toBeDefined();
    }
  });

  it('contains no CJK characters in TypeScript source files outside locales', () => {
    const cjk = /[\u4e00-\u9fff]/;
    const cjkHits: string[] = [];
    const srcDir = path.resolve(__dirname);
    const walkFiles = (dir: string) => {
      for (const f of fs.readdirSync(dir)) {
        const full = path.join(dir, f);
        if (fs.statSync(full).isDirectory()) {
          if (f !== 'locales') walkFiles(full);
        } else if (f.endsWith('.ts') || f.endsWith('.tsx')) {
          const content = fs.readFileSync(full, 'utf8');
          if (cjk.test(content)) {
            cjkHits.push(path.relative(srcDir, full));
          }
        }
      }
    };
    walkFiles(srcDir);
    expect(cjkHits).toEqual([]);
  });
});
