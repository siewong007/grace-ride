// Mirrors backend/src/perms.rs. A permission is one action on one page, written
// '<page>.<action>'. The server decides; this only keeps buttons and links the member
// cannot use out of their way.

export const ACTIONS = ['view', 'create', 'update', 'delete'] as const;
export type Act = (typeof ACTIONS)[number];

export const AREAS = ['member', 'transport', 'finance', 'admin'] as const;
export type Area = (typeof AREAS)[number];

export type PageKey =
  // Member pages
  | 'rides'
  | 'drive'
  | 'events'
  | 'settlements'
  | 'costs'
  | 'reimbursements'
  | 'help'
  | 'report'
  // Staff pages (need a passkey / 2FA session)
  | 'admin_dashboard'
  | 'admin_rides'
  | 'admin_live'
  | 'admin_plan'
  | 'admin_events'
  | 'admin_reimb_review'
  | 'admin_announcements'
  | 'admin_help'
  | 'admin_reports'
  | 'admin_reimb_pay'
  | 'admin_bills'
  | 'admin_members'
  | 'admin_invites'
  | 'admin_recovery'
  | 'admin_groups'
  | 'admin_areas'
  | 'admin_roles'
  | 'admin_audit'
  | 'admin_settings';

export type Can = (page: PageKey, act?: Act) => boolean;

/** Staff pages (the non-member areas). Mirrors backend `any_admin_page`. */
export const ADMIN_PAGES: PageKey[] = [
  'admin_dashboard',
  'admin_rides',
  'admin_live',
  'admin_plan',
  'admin_events',
  'admin_reimb_review',
  'admin_announcements',
  'admin_help',
  'admin_reports',
  'admin_reimb_pay',
  'admin_bills',
  'admin_members',
  'admin_invites',
  'admin_recovery',
  'admin_groups',
  'admin_areas',
  'admin_roles',
  'admin_audit',
  'admin_settings',
];

/** Can the user see any staff page? Gates the admin guide tab and route. */
export const canAnyAdminPage = (can: Can): boolean => ADMIN_PAGES.some((p) => can(p));

/** Built-in roles are named by the app; anything else carries its own name. */
export const SYSTEM_ROLES = ['member', 'admin', 'transport', 'finance', 'leader', 'driver'];
