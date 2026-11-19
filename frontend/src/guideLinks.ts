/**
 * Guide step links.
 *
 * A step link is an in-app path ("/rides", "/admin/settings"). A `*` segment stands for
 * one item's id ("/rides/*" = any ride's page), so a step can belong to a detail page
 * without naming a real id. `:` is refused by the API, so route-style "/rides/:id" is
 * never stored. A pattern is never navigated to: following it would open the literal
 * "*" item. Instead the person is pointed at the list where they pick the item.
 */

/** The path part of a link, without ?query or #hash. */
export const linkPath = (link: string) => link.split(/[?#]/)[0];

/** True when the step link stands for an item page rather than one fixed page. */
export const isPatternLink = (link: string) => linkPath(link).split('/').includes('*');

/** True when `pathname` is the page the link names (a `*` segment matches any one segment). */
export function linkMatchesPath(link: string, pathname: string): boolean {
  const want = linkPath(link).replace(/\/+$/, '').split('/');
  const have = pathname.replace(/\/+$/, '').split('/');
  if (want.length !== have.length) return false;
  return want.every((seg, i) => (seg === '*' ? have[i] !== '' : seg === have[i]));
}

/** True when the link points at the page the person is on (a link with a ?query must match it too). */
export function isHere(link: string, pathname: string, search: string): boolean {
  const q = link.includes('?') ? `?${link.split('?')[1].split('#')[0]}` : '';
  return linkMatchesPath(link, pathname) && (!q || q === search);
}

/** Where an item-page pattern is opened from: the list the person picks the item in. */
const PATTERN_LISTS: [string, string][] = [
  ['/admin/events/', '/events'],
  ['/admin/rides/', '/admin/rides'],
  ['/events/', '/events'],
  ['/rides/', '/rides'],
  ['/settlements/', '/settlements'],
  ['/reimbursements/', '/reimbursements'],
  ['/help/', '/help'],
];

/**
 * The page to send someone to for this step: the link itself, or for an item-page
 * pattern the list it is picked from (null when there is none).
 */
export function navigableLink(link: string): string | null {
  if (!link) return null;
  if (!isPatternLink(link)) return link;
  const path = linkPath(link);
  return PATTERN_LISTS.find(([prefix]) => path.startsWith(prefix))?.[1] ?? null;
}

export interface PageGuideCandidate<G> {
  guide: G;
  stepIndex: number;
}

/**
 * The guide the page header offers on `pathname`, and the step to open it at.
 *
 * A guide that starts on this page beats one that only passes through it, and a step
 * naming this exact page beats an item pattern ("/events/new" before "/events/*").
 * Ties go to the guide listed first.
 */
export function guideForPage<G extends { published?: boolean; steps?: { link?: string }[] }>(
  guides: G[],
  pathname: string,
): PageGuideCandidate<G> | null {
  if (pathname === '/' || pathname.startsWith('/help')) return null;
  let best: (PageGuideCandidate<G> & { rank: number }) | null = null;
  for (const g of guides) {
    if (!g.published || !g.steps) continue;
    const stepIndex = g.steps.findIndex((s) => s.link && linkMatchesPath(s.link, pathname));
    if (stepIndex === -1) continue;
    const rank = (stepIndex === 0 ? 0 : 2) + (isPatternLink(g.steps[stepIndex].link!) ? 1 : 0);
    if (!best || rank < best.rank) best = { guide: g, stepIndex, rank };
    if (rank === 0) break;
  }
  return best && { guide: best.guide, stepIndex: best.stepIndex };
}
