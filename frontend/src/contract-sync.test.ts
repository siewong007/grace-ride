import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { ACTIONS, ADMIN_PAGES, SYSTEM_ROLES } from './perms';
import enErrors from './locales/errors.en';
import zhErrors from './locales/errors.zh';

const repoRoot = path.resolve(__dirname, '../..');
const ERROR_GROUPS = [
  'common',
  'admin',
  'auth',
  'events',
  'rides',
  'settlements',
  'community',
  'files',
  'family',
] as const;

describe('Cross-client backend contract sync', () => {
  it('keeps frontend PageKey, ADMIN_PAGES, ACTIONS, and SYSTEM_ROLES in sync with backend/src/perms.rs', () => {
    const rs = fs.readFileSync(path.join(repoRoot, 'backend/src/perms.rs'), 'utf-8');
    const ts = fs.readFileSync(path.join(repoRoot, 'frontend/src/perms.ts'), 'utf-8');

    // Resolve `pub const *_ROLE: &str = "...";` constants for SYSTEM_ROLES.
    const roleConsts = new Map<string, string>();
    for (const m of rs.matchAll(/pub const ([A-Z0-9_]+):\s*&str\s*=\s*"([^"]+)";/g)) {
      roleConsts.set(m[1], m[2]);
    }
    const sysRolesMatch = rs.match(/pub const SYSTEM_ROLES:\s*&\[&str\]\s*=\s*&\[([\s\S]*?)\];/);
    expect(sysRolesMatch).not.toBeNull();
    const rsRoles = sysRolesMatch![1]
      .split(',')
      .map((tok) => tok.trim())
      .filter(Boolean)
      .map((tok) => {
        if (tok.startsWith('"') && tok.endsWith('"')) return tok.slice(1, -1);
        const resolved = roleConsts.get(tok);
        expect(resolved, `Unknown role constant in SYSTEM_ROLES: ${tok}`).toBeDefined();
        return resolved!;
      });
    expect([...SYSTEM_ROLES].sort()).toEqual([...rsRoles].sort());

    // Verify Act::key() matches ACTIONS.
    const actMatch = rs.match(/impl Act \{[\s\S]*?pub fn key\(self\) -> &'static str \{([\s\S]*?)\n {4}\}/);
    expect(actMatch).not.toBeNull();
    const rsActions = [...actMatch![1].matchAll(/Act::\w+\s*=>\s*"([^"]+)"/g)].map((m) => m[1]);
    expect([...ACTIONS].sort()).toEqual(rsActions.sort());

    // Verify catalog! pages and staff/admin subset match PageKey and ADMIN_PAGES.
    const catalogMatch = rs.match(/catalog!\s*\{([\s\S]*?)\n\}/);
    expect(catalogMatch).not.toBeNull();
    const rsAllPages: string[] = [];
    const rsAdminPages: string[] = [];
    for (const m of catalogMatch![1].matchAll(
      /\w+:\s*"([a-z0-9_]+)",\s*(Member|Transport|Finance|Admin),\s*\[[^\]]+\];/g,
    )) {
      rsAllPages.push(m[1]);
      if (m[2] !== 'Member') rsAdminPages.push(m[1]);
    }
    expect(rsAllPages.length).toBeGreaterThan(0);

    const pageKeyBlock = ts.match(/export type PageKey =([\s\S]*?);/);
    expect(pageKeyBlock).not.toBeNull();
    const tsAllPages = [...pageKeyBlock![1].matchAll(/'([a-z0-9_]+)'/g)].map((m) => m[1]);
    expect(tsAllPages.sort()).toEqual(rsAllPages.sort());
    expect([...ADMIN_PAGES].sort()).toEqual(rsAdminPages.sort());
  });

  it('maps every ErrorKey::code() from backend/src/i18n/keys.rs in both errors.en.ts and errors.zh.ts', () => {
    const keysRs = fs.readFileSync(path.join(repoRoot, 'backend/src/i18n/keys.rs'), 'utf-8');
    const codeBlock = keysRs.match(
      /pub fn code\(&self\) -> &'static str \{([\s\S]*?)\n {4}\}/,
    );
    expect(codeBlock).not.toBeNull();
    const backendCodes = [
      ...codeBlock![1].matchAll(/Self::\w+(?:\([^)]*\)|\s*\{[^}]*\})?\s*=>\s*"([A-Z0-9_]+)"/g),
    ].map((m) => m[1]);
    expect(backendCodes.length).toBeGreaterThan(100);

    const flattenCodes = (bundle: Record<string, unknown>) => {
      const codes = new Set<string>();
      for (const group of ERROR_GROUPS) {
        const entries = bundle[group];
        if (entries && typeof entries === 'object') {
          for (const key of Object.keys(entries)) {
            codes.add(key);
          }
        }
      }
      return codes;
    };

    const enCodes = flattenCodes(enErrors as unknown as Record<string, unknown>);
    const zhCodes = flattenCodes(zhErrors as unknown as Record<string, unknown>);

    const missingInEn = backendCodes.filter((c) => !enCodes.has(c));
    const missingInZh = backendCodes.filter((c) => !zhCodes.has(c));

    expect(missingInEn).toEqual([]);
    expect(missingInZh).toEqual([]);
  });
});
