import { useMemo } from 'react';
import { useApi } from '../../api';
import type { Act, Area, PageKey } from '../../perms';

export interface RoleDef {
  key: string;
  /** Empty for built-in roles: the app names those in the reader's language. */
  name: string;
  description: string;
  system: boolean;
  /** Held by every active member; not handed out one by one. */
  implicit: boolean;
  permissions: string[];
  /** Permissions this role can never lose. */
  locked: string[];
  members: number;
}

export interface CatalogPage {
  page: PageKey;
  area: Area;
  actions: Act[];
}

export interface RolesResponse {
  catalog: CatalogPage[];
  roles: RoleDef[];
}

/** Until the role list loads (or if it can't), offer the built-in roles. */
const BUILT_IN = ['admin', 'transport', 'finance', 'leader', 'driver'];

/** Roles that can be given to a member, and the names of the custom ones. */
export function useRoleOptions(enabled = true) {
  const { data, reload } = useApi<RolesResponse>(enabled ? '/admin/roles' : null);
  return useMemo(() => {
    const assignable = data ? data.roles.filter((r) => !r.implicit).map((r) => r.key) : BUILT_IN;
    const names: Record<string, string> = {};
    for (const r of data?.roles ?? []) if (r.name) names[r.key] = r.name;
    return { assignable, names, reload };
  }, [data, reload]);
}
