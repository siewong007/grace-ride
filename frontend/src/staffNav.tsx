import { createContext, ReactNode, useContext } from 'react';
import {
  FileBarChart,
  HeartHandshake,
  KeyRound,
  LayoutDashboard,
  LifeBuoy,
  Megaphone,
  Route,
  ScrollText,
  Settings,
  Users,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useApi } from './api';
import { useAuth } from './auth';
import { canApprovePlan, type PageKey } from './perms';

/** A staff page link shown in the sidebar and on the phone admin menu. */
export interface StaffNavItem {
  to: string;
  label: string;
  icon: ReactNode;
  end?: boolean;
  /** Items waiting on staff at this page (shown as a count). */
  badge?: number;
}

export interface StaffGroup {
  key: string;
  label: string;
  items: StaffNavItem[];
}

/** The waiting counts the nav badges read; a subset of /admin/dashboard. */
export interface AdminCounts {
  pending_approval: number;
  no_driver: number;
  open_help: number;
  pending_reimbursements: number;
  pending_recovery: number;
  /** Members who joined from an invite and are waiting for approval. */
  pending_members?: number;
  /** Invite links not yet used or expired. */
  open_invites: number;
  /** Approved claims waiting for finance to pay them. */
  awaiting_payment: number;
  live_drivers?: number;
  pending_plan_drafts?: number;
  pending_ride_drafts?: number;
  my_returned_plan_drafts?: number;
  my_returned_ride_drafts?: number;
}

export interface StaffNavValue {
  staffGroups: StaffGroup[];
  staff: StaffNavItem[];
  staffWaiting: number;
  helpDesk: boolean;
  counts: (AdminCounts & Record<string, unknown>) | null;
  dashError: string | null;
  reloadDash: () => Promise<void>;
  isTrafficOnlyStaff: boolean;
  canTrafficHub: boolean;
  trafficBadge: number;
  fromContext: boolean;
}

/** Shared nav state from Layout so child admin/home pages do not re-fetch /admin/dashboard on mount. */
export const StaffNavContext = createContext<StaffNavValue | null>(null);

export function waitingBadge(count: number | undefined, label: string) {
  return count ? (
    <>
      <span className="nav-badge" aria-hidden="true">{count > 99 ? '99+' : count}</span>
      <span className="sr-only">{label}</span>
    </>
  ) : null;
}

/** Permissions that unlock at least one tab inside the unified Traffic Hub. */
export const TRAFFIC_HUB_PERMS: PageKey[] = [
  'admin_plan',
  'admin_rides',
  'admin_live',
  'admin_area_photos',
  'admin_areas',
  'admin_fares',
];

const NON_TRAFFIC_STAFF_PERMS: PageKey[] = [
  'admin_members',
  'admin_invites',
  'admin_recovery',
  'admin_roles',
  'admin_reimb_review',
  'admin_reimb_pay',
  'admin_reports',
  'admin_announcements',
  'admin_help',
  'admin_groups',
  'admin_settings',
  'admin_audit',
];

/**
 * Staff pages grouped by the job they serve, filtered to what this member may open.
 * Used by the desktop sidebar and the phone admin menu page.
 * When called inside Layout's StaffNavContext, reuses Layout's single /admin/dashboard fetch.
 */
export function useStaffNav(): StaffNavValue {
  const parent = useContext(StaffNavContext);
  const own = useStaffNavImpl(!parent);
  return parent ?? own;
}

function useStaffNavImpl(enabled: boolean): StaffNavValue {
  const { t } = useTranslation('layout');
  const { me, can } = useAuth();
  const pending = me?.status === 'pending';
  const helpDesk = can('admin_help');
  const dashApi = useApi<AdminCounts & Record<string, unknown>>(
    enabled && me && me.status !== 'pending' && can('admin_dashboard') ? '/admin/dashboard' : null,
    60000,
  );
  const counts = dashApi.data;

  const canTrafficHub = TRAFFIC_HUB_PERMS.some((p) => can(p));
  const hasNonTrafficStaff = NON_TRAFFIC_STAFF_PERMS.some((p) => can(p));
  const isTrafficOnlyStaff = !pending && canTrafficHub && !hasNonTrafficStaff;
  const isLead = canApprovePlan(me?.roles);
  const trafficBadge = counts
    ? isLead
      ? (counts.pending_plan_drafts ?? 0) + (counts.pending_ride_drafts ?? 0)
      : (counts.my_returned_plan_drafts ?? 0) + (counts.my_returned_ride_drafts ?? 0)
    : 0;
  const opsTrafficBadge = isTrafficOnlyStaff
    ? trafficBadge
    : trafficBadge + (can('admin_rides') && counts ? counts.pending_approval + counts.no_driver : 0);

  const links = (items: (StaffNavItem & { show: boolean })[]): StaffNavItem[] =>
    items.filter((n) => n.show && !pending).map(({ to, label, icon, end, badge }) => ({ to, label, icon, end, badge }));

  const sz = 20;
  const staffGroups: StaffGroup[] = (
    isTrafficOnlyStaff
      ? [
          {
            key: 'ops',
            label: t('trafficTab'),
            items: links([
              { to: '/admin/traffic', label: t('trafficHub'), icon: <Route size={sz} />, show: canTrafficHub, badge: trafficBadge },
              { to: '/admin', label: t('dashboard'), icon: <LayoutDashboard size={sz} />, end: true, show: can('admin_dashboard') },
            ]),
          },
        ]
      : [
          {
            key: 'ops',
            label: t('groupOps'),
            items: links([
              { to: '/admin/traffic', label: t('trafficHub'), icon: <Route size={sz} />, show: canTrafficHub, badge: opsTrafficBadge },
              { to: '/admin', label: t('dashboard'), icon: <LayoutDashboard size={sz} />, end: true, show: can('admin_dashboard') },
            ]),
          },
          {
            key: 'people',
            label: t('groupPeople'),
            items: links([
              {
                to: '/admin/members', label: t('members'), icon: <Users size={sz} />,
                show: can('admin_members') || can('admin_invites') || can('admin_recovery'),
                badge:
                  (can('admin_recovery') ? counts?.pending_recovery ?? 0 : 0) +
                  (can('admin_members') ? counts?.pending_members ?? 0 : 0),
              },
              { to: '/admin/roles', label: t('roles'), icon: <KeyRound size={sz} />, show: can('admin_roles') },
            ]),
          },
          {
            key: 'money',
            label: t('groupMoney'),
            items: links([
              {
                to: '/admin/reimbursements', label: t('reimbReview'), icon: <HeartHandshake size={sz} />,
                show: can('admin_reimb_review') || can('admin_reimb_pay'),
                badge: can('admin_reimb_review') ? counts?.pending_reimbursements : 0,
              },
              { to: '/admin/reports', label: t('reports'), icon: <FileBarChart size={sz} />, show: can('admin_reports') },
            ]),
          },
          {
            key: 'content',
            label: t('groupContent'),
            items: links([
              { to: '/admin/announcements', label: t('announcements'), icon: <Megaphone size={sz} />, show: can('admin_announcements') },
              { to: '/help', label: t('helpDesk'), icon: <LifeBuoy size={sz} />, show: helpDesk, badge: counts?.open_help },
            ]),
          },
          {
            key: 'system',
            label: t('groupSystem'),
            items: links([
              { to: '/admin/settings', label: t('settings'), icon: <Settings size={sz} />, show: can('admin_groups') || can('admin_areas') || can('admin_settings') || can('admin_fares') || can('admin_area_photos') },
              { to: '/admin/audit', label: t('audit'), icon: <ScrollText size={sz} />, show: can('admin_audit') },
            ]),
          },
        ]
  ).filter((g) => g.items.length > 0);

  const staff = staffGroups.flatMap((g) => g.items);
  const staffWaiting = staff.reduce((n, i) => n + (i.badge ?? 0), 0);
  return {
    staffGroups,
    staff,
    staffWaiting,
    helpDesk,
    counts,
    dashError: dashApi.error,
    reloadDash: dashApi.reload,
    isTrafficOnlyStaff,
    canTrafficHub,
    trafficBadge,
    fromContext: false,
  };
}

/**
 * Where an admin page's back arrow goes. The back arrow only shows on phones, where the
 * staff menu is the hub; the dashboard is just one of its rows and not everyone may open it.
 */
export const ADMIN_HOME = '/admin/menu';

/** Permissions that unlock at least one item on the admin menu. */
export const ADMIN_MENU_PERMS: PageKey[] = [
  'admin_dashboard',
  'admin_plan',
  'admin_rides',
  'admin_live',
  'admin_members',
  'admin_invites',
  'admin_recovery',
  'admin_roles',
  'admin_reimb_review',
  'admin_reimb_pay',
  'admin_reports',
  'admin_announcements',
  'admin_help',
  'admin_groups',
  'admin_areas',
  'admin_fares',
  'admin_area_photos',
  'admin_settings',
  'admin_audit',
];
