import { describe, expect, it } from 'vitest';
import { guideForPage, isHere, isPatternLink, linkMatchesPath, navigableLink } from './guideLinks';

const g = (slug: string, links: string[], published = true) => ({
  slug,
  published,
  steps: links.map((link) => ({ link })),
});

// Mirrors the seeded order: staff guides first, then member guides.
const guides = [
  g('plan-transport', ['/events/*', '/admin/events/*/plan', '/admin/live']),
  g('send-announcement', ['/admin/announcements', '/admin/announcements']),
  g('audit-settings', ['/admin/audit']),
  g('request-a-ride', ['/rides/new', '/rides/new']),
  g('track-your-ride', ['/rides', '/rides/*']),
  g('join-events', ['/events', '/events/*']),
  g('organise-an-event', ['/events/new', '/events/*']),
  g('claim-reimbursement', ['/reimbursements/new', '/reimbursements/*']),
  g('answer-a-driver-offer', ['/rides', '/rides/*']),
  g('notifications-and-report', ['/notifications', '/report']),
  g('live-map', ['/admin/live']),
  g('groups-areas-rates', ['/admin/settings', '/admin/settings']),
  g('hidden', ['/drive'], false),
];

const pick = (path: string) => {
  const m = guideForPage(guides, path);
  return m && `${m.guide.slug}#${m.stepIndex + 1}`;
};

describe('linkMatchesPath', () => {
  it('matches exact pages and one-segment item patterns', () => {
    expect(linkMatchesPath('/rides', '/rides')).toBe(true);
    expect(linkMatchesPath('/rides/*', '/rides/42')).toBe(true);
    expect(linkMatchesPath('/rides/*', '/rides')).toBe(false);
    expect(linkMatchesPath('/rides/*', '/rides/42/edit')).toBe(false);
    expect(linkMatchesPath('/admin/events/*/plan', '/admin/events/5/plan')).toBe(true);
    expect(linkMatchesPath('/help?tab=guides', '/help')).toBe(true);
  });

  it('isHere still checks a ?query', () => {
    expect(isHere('/help?tab=guides', '/help', '?tab=guides')).toBe(true);
    expect(isHere('/help?tab=guides', '/help', '')).toBe(false);
  });
});

describe('navigableLink', () => {
  it('never returns a * link; patterns go to their list', () => {
    expect(isPatternLink('/rides/*')).toBe(true);
    expect(navigableLink('/rides')).toBe('/rides');
    expect(navigableLink('/rides/*')).toBe('/rides');
    expect(navigableLink('/events/*/settle')).toBe('/events');
    expect(navigableLink('/admin/events/*/plan')).toBe('/events');
    expect(navigableLink('/unknown/*')).toBeNull();
    expect(navigableLink('')).toBeNull();
  });
});

describe('guideForPage', () => {
  it('prefers a guide that starts on this page over one passing through', () => {
    expect(pick('/events')).toBe('join-events#1');
    expect(pick('/admin/live')).toBe('live-map#1');
    expect(pick('/admin/settings')).toBe('groups-areas-rates#1');
    expect(pick('/notifications')).toBe('notifications-and-report#1');
    expect(pick('/rides')).toBe('track-your-ride#1');
  });

  it('prefers an exact page over an item pattern', () => {
    expect(pick('/events/new')).toBe('organise-an-event#1');
  });

  it('offers item-page guides on detail pages', () => {
    expect(pick('/events/7')).toBe('plan-transport#1');
    expect(pick('/rides/42')).toBe('track-your-ride#2');
    expect(pick('/admin/events/5/plan')).toBe('plan-transport#2');
    expect(pick('/reimbursements/3')).toBe('claim-reimbursement#2');
  });

  it('skips unpublished guides, home and help', () => {
    expect(pick('/drive')).toBeNull();
    expect(pick('/')).toBeNull();
    expect(pick('/help')).toBeNull();
  });
});
