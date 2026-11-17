import { ReactNode } from 'react';
import {
  ClipboardList,
  FileBarChart,
  HeartHandshake,
  KeyRound,
  LayoutDashboard,
  LifeBuoy,
  MapPinned,
  Megaphone,
  ScrollText,
  Settings,
  Users,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useApi } from './api';
import { useAuth } from './auth';
import type { PageKey } from './perms';

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
  /** Invite links not yet used or expired. */
  open_invites: number;
  /** Approved claims waiting for finance to pay them. */
  awaiting_payment: number;
}

export function waitingBadge(count: number | undefined, label: string) {
  return count ? (
    <>
      <span className="nav-badge" aria-hidden="true">{count > 99 ? '99+' : count}</span>
      <span className="sr-only">{label}</span>
    </>
  ) : null;
}

/**
 * Staff pages grouped by the job they serve, filtered to what this member may open.
 * Used by the desktop sidebar and the phone admin menu page.
 */
export function useStaffNav() {
  const { t } = useTranslation('layout');
  const { me, can } = useAuth();
  const pending = me?.status === 'pending';
  const helpDesk = can('admin_help');
  const counts = useApi<AdminCounts>(
    me && me.status !== 'pending' && can('admin_dashboard') ? '/admin/dashboard' : null,
    60000,
  ).data;

  const links = (items: (StaffNavItem & { show: boolean })[]): StaffNavItem[] =>
    items.filter((n) => n.show && !pending).map(({ to, label, icon, end, badge }) => ({ to, label, icon, end, badge }));

  const sz = 20;
  const staffGroups: StaffGroup[] = [
    {
      key: 'ops',
      label: t('groupOps'),
      items: links([
        { to: '/admin', label: t('dashboard'), icon: <LayoutDashboard size={sz} />, end: true, show: can('admin_dashboard') },
        {
          to: '/admin/rides', label: t('adminRides'), icon: <ClipboardList size={sz} />, show: can('admin_rides'),
          badge: counts ? counts.pending_approval + counts.no_driver : 0,
        },
        { to: '/admin/live', label: t('liveMap'), icon: <MapPinned size={sz} />, show: can('admin_live') },
      ]),
    },
    {
      key: 'people',
      label: t('groupPeople'),
      items: links([
        {
          to: '/admin/members', label: t('members'), icon: <Users size={sz} />,
          show: can('admin_members') || can('admin_invites') || can('admin_recovery'),
          badge: can('admin_recovery') ? counts?.pending_recovery : 0,
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
        { to: '/admin/settings', label: t('settings'), icon: <Settings size={sz} />, show: can('admin_groups') || can('admin_areas') || can('admin_settings') },
        { to: '/admin/audit', label: t('audit'), icon: <ScrollText size={sz} />, show: can('admin_audit') },
      ]),
    },
  ].filter((g) => g.items.length > 0);

  const staff = staffGroups.flatMap((g) => g.items);
  const staffWaiting = staff.reduce((n, i) => n + (i.badge ?? 0), 0);
  return { staffGroups, staff, staffWaiting, helpDesk, counts };
}

/**
 * Where an admin page's back arrow goes. The back arrow only shows on phones, where the
 * staff menu is the hub; the dashboard is just one of its rows and not everyone may open it.
 */
export const ADMIN_HOME = '/admin/menu';

/** Permissions that unlock at least one item on the admin menu. */
export const ADMIN_MENU_PERMS: PageKey[] = [
  'admin_dashboard',
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
  'admin_settings',
  'admin_audit',
];
