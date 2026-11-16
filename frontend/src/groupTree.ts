// Groups form a tree (a group can be part of another). Mirrors backend/src/audience.rs:
// an event for a group is for that group and every group below it.

export interface TreeGroup {
  id: number;
  name: string;
  is_all: boolean;
  parent_id?: number | null;
  is_visitor?: boolean;
}

/** Groups in tree order (each parent followed by its sub-groups), with their depth (0 = top). */
export function treeOrder<G extends TreeGroup>(groups: G[]): { group: G; depth: number }[] {
  const ids = new Set(groups.map((g) => g.id));
  const kids = new Map<number | null, G[]>();
  for (const g of groups) {
    // A parent that isn't in the list (not visible to this member) puts the group at the top.
    const p = g.parent_id != null && ids.has(g.parent_id) ? g.parent_id : null;
    kids.set(p, [...(kids.get(p) ?? []), g]);
  }
  const out: { group: G; depth: number }[] = [];
  const seen = new Set<number>();
  const walk = (parent: number | null, depth: number) => {
    for (const g of kids.get(parent) ?? []) {
      if (seen.has(g.id)) continue;
      seen.add(g.id);
      out.push({ group: g, depth });
      walk(g.id, depth + 1);
    }
  };
  walk(null, 0);
  return out;
}

/** The ids of `id` and every group below it. */
export function withDescendants(groups: TreeGroup[], id: number): Set<number> {
  const out = new Set([id]);
  let grew = true;
  while (grew) {
    grew = false;
    for (const g of groups) {
      if (g.parent_id != null && out.has(g.parent_id) && !out.has(g.id)) {
        out.add(g.id);
        grew = true;
      }
    }
  }
  return out;
}

/** Groups people can belong to and events can target: not the built-in visitor group. */
export const ordinary = <G extends TreeGroup>(groups: G[]) => groups.filter((g) => !g.is_visitor);
