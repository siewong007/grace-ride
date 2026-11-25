import React, { ComponentType, lazy, ReactNode, Suspense, useEffect, useLayoutEffect } from 'react';
import { Navigate, Route, Routes, useLocation } from 'react-router-dom';
import { useAuth } from './auth';
import { BackNavigationProvider } from './components/BackNavigation';
import Layout from './components/Layout';
import { NoAccess, Page, Spinner } from './components/ui';
import { ensureLocaleNamespaces, type Namespace } from './i18n';
import type { Act, PageKey } from './perms';
import { ADMIN_MENU_PERMS, TRAFFIC_HUB_PERMS } from './staffNav';
import { hideSplash, holdSplash } from './splash';
import { AutoLockProvider } from './components/AutoLock';
import Login from './pages/Login';
import Home from './pages/Home';
import TermsPage from './components/Terms';

// Route-level code splitting: the login screen and home page load first; everything
// else (admin tools, Leaflet maps, cost splitting) downloads only when visited, with
// its route translation namespaces loaded in parallel.
const lazyRoute = <M extends { default: ComponentType<never> } | { default: ComponentType }>(
  load: () => Promise<M>,
  ns: readonly Namespace[] = [],
) =>
  lazy(async () => {
    const [mod] = await Promise.all([
      load(),
      ns.length ? ensureLocaleNamespaces(ns) : Promise.resolve(),
    ]);
    return mod as { default: ComponentType };
  });

const namedRoute = <M extends Record<string, unknown>, K extends keyof M>(
  load: () => Promise<M>,
  key: K,
  ns: readonly Namespace[] = [],
) =>
  lazy(async () => {
    const [mod] = await Promise.all([
      load(),
      ns.length ? ensureLocaleNamespaces(ns) : Promise.resolve(),
    ]);
    return { default: mod[key] as ComponentType };
  });

const Invite = lazyRoute(() => import('./pages/Invite'), ['invite']);
const Visit = lazyRoute(() => import('./pages/Visit'), ['visit']);
const CarePass = lazyRoute(() => import('./pages/CarePass'), ['visit']);
const Rides = lazyRoute(() => import('./pages/Rides'), ['rides']);
const RideNew = lazyRoute(() => import('./pages/RideNew'), ['rideNew']);
const RideDetail = lazyRoute(() => import('./pages/RideDetail'), ['rideDetail']);
const Drive = lazyRoute(() => import('./pages/Drive'), ['drive']);
const Events = lazyRoute(() => import('./pages/Events'), ['events']);
const EventDetail = lazyRoute(() => import('./pages/EventDetail'), ['eventDetail']);
const EventCheckin = lazyRoute(() => import('./pages/EventCheckin'), ['eventDetail']);
const EventForm = lazyRoute(() => import('./pages/EventForm'), ['eventForm']);
const Settlements = lazyRoute(() => import('./pages/Settlements'), ['settlements']);
const SettlementDetail = namedRoute(() => import('./pages/Settlements'), 'SettlementDetail', ['settlements']);
const SettlementForm = namedRoute(() => import('./pages/Settlements'), 'SettlementForm', ['settlements']);
const Costs = lazyRoute(() => import('./pages/Costs'), ['costs']);
const Reimbursements = lazyRoute(() => import('./pages/Reimbursements'), ['reimbursements']);
const ReimbursementDetail = namedRoute(() => import('./pages/Reimbursements'), 'ReimbursementDetail', ['reimbursements']);
const ReimbursementForm = namedRoute(() => import('./pages/Reimbursements'), 'ReimbursementForm', ['reimbursements']);
const Help = lazyRoute(() => import('./pages/Help'), ['help']);
const HelpDetail = lazyRoute(() => import('./pages/HelpDetail'), ['help']);
const HelpGuideDetail = lazyRoute(() => import('./pages/HelpGuideDetail'), ['help']);
const HelpGuideEditor = lazyRoute(() => import('./pages/HelpGuideEditor'), ['help']);
const Report = lazyRoute(() => import('./pages/Report'), ['report']);
const Notifications = lazyRoute(() => import('./pages/Notifications'), ['notifications']);
const Profile = lazyRoute(() => import('./pages/Profile'), ['profile', 'studio', 'rideNew']);
const GrokBotStudioPage = lazyRoute(() => import('./pages/GrokBotStudioPage'), ['studio']);
const NotFound = lazyRoute(() => import('./pages/NotFound'));
const Licenses = lazyRoute(() => import('./pages/Licenses'));
const AdminDashboard = lazyRoute(() => import('./pages/admin/Dashboard'), ['admin']);
const AdminRides = lazyRoute(() => import('./pages/admin/Rides'), ['admin']);
const AdminLive = lazyRoute(() => import('./pages/admin/Live'), ['admin']);
const AdminMembers = lazyRoute(() => import('./pages/admin/Members'), ['admin']);
const AdminSettings = lazyRoute(() => import('./pages/admin/Settings'), ['admin']);
const AdminReimbursements = lazyRoute(() => import('./pages/admin/Reimbursements'), ['admin']);
const AdminAnnouncements = lazyRoute(() => import('./pages/admin/Announcements'), ['admin']);
const AdminAudit = lazyRoute(() => import('./pages/admin/Audit'), ['admin']);
const AdminReports = lazyRoute(() => import('./pages/admin/Reports'), ['admin']);
const PlanTransport = lazyRoute(() => import('./pages/admin/PlanTransport'), ['plan']);
const AdminRoles = lazyRoute(() => import('./pages/admin/Roles'), ['admin']);
const AdminMenu = lazyRoute(() => import('./pages/admin/Menu'), ['admin']);
const TrafficHub = lazyRoute(() => import('./pages/admin/TrafficHub'), ['admin', 'plan']);

/** Wrap a lazily-loaded page so only the content area shows a spinner while it loads. */
const page = (el: ReactNode) => <Suspense fallback={<PageLoading />}>{el}</Suspense>;

/** Suspense fallback for a lazy page. On first load it keeps the boot splash up instead, so the
 *  splash hands straight over to the page rather than flashing a second loading mark. */
function PageLoading() {
  useLayoutEffect(() => holdSplash(), []);
  return <div className="page-loading"><Spinner /></div>;
}

type Need = PageKey | [PageKey, Act];

/** Show the page only to members who hold one of the listed permissions. The API
 *  enforces the same rule; this saves a member from opening a page that can't load. */
function Gate({ any, children }: { any: Need[]; children: ReactNode }) {
  const { me, can } = useAuth();
  const allowed = any.some((n) => (typeof n === 'string' ? can(n) : can(n[0], n[1])));
  if (allowed) return <>{children}</>;
  // Staff permissions stay off until the member signs in with a passkey or 2FA.
  const needsMfa = !!me?.privileged && !me.mfa_ok;
  return (
    <Page>
      <NoAccess mfa={needsMfa} />
    </Page>
  );
}

const gated = (el: ReactNode, ...any: Need[]) => page(<Gate any={any}>{el}</Gate>);

function RequireAuth({ children }: { children: React.JSX.Element }) {
  const { me, loading } = useAuth();
  const loc = useLocation();
  if (loading) return <Spinner />;
  if (!me) return <Navigate to="/login" state={{ from: loc.pathname + loc.search }} replace />;
  return <AutoLockProvider>{children}</AutoLockProvider>;
}

export default function App() {
  const { loading } = useAuth();
  // The boot splash keeps playing until we know whether to show the app or the login screen.
  useEffect(() => {
    if (!loading) hideSplash();
  }, [loading]);

  return (
    <BackNavigationProvider>
      <Routes>
        <Route path="/login" element={<Login />} />
        <Route path="/terms" element={<TermsPage />} />
        <Route path="/licences" element={page(<Licenses />)} />
        <Route path="/invite/:token" element={page(<Invite />)} />
        {/* A visitor's ride: no account, outside the signed-in app. */}
        <Route path="/visit/:token" element={page(<Visit />)} />
        {/* Scoped caregiver safe-arrival view: no street address, read-only status + readiness toggle. */}
        <Route path="/care/:token" element={page(<CarePass />)} />
        <Route element={<RequireAuth><Layout /></RequireAuth>}>
          <Route index element={<Home />} />
          <Route path="rides" element={gated(<Rides />, 'rides')} />
          <Route path="rides/new" element={gated(<RideNew />, ['rides', 'create'])} />
          <Route path="rides/:id" element={page(<RideDetail />)} />
          <Route path="drive" element={gated(<Drive />, 'drive')} />
          <Route path="events" element={gated(<Events />, 'events')} />
          <Route path="events/new" element={gated(<EventForm />, ['events', 'create'])} />
          <Route path="events/:id" element={gated(<EventDetail />, 'events', 'admin_events')} />
          <Route path="events/:id/checkin" element={gated(<EventCheckin />, 'events', 'admin_events')} />
          <Route path="events/:id/edit" element={gated(<EventForm />, ['events', 'update'], ['admin_events', 'update'])} />
          <Route path="events/:eventId/settle" element={gated(<SettlementForm />, ['settlements', 'create'])} />
          <Route path="settlements" element={gated(<Settlements />, 'settlements', 'admin_bills')} />
          <Route path="settlements/:id" element={page(<SettlementDetail />)} />
          <Route path="settlements/:id/edit" element={gated(<SettlementForm />, ['settlements', 'update'])} />
          <Route path="costs" element={gated(<Costs />, 'costs')} />
          <Route path="reimbursements" element={gated(<Reimbursements />, 'reimbursements')} />
          <Route path="reimbursements/new" element={gated(<ReimbursementForm />, ['reimbursements', 'create'], ['reimbursements', 'update'])} />
          <Route path="reimbursements/:id" element={page(<ReimbursementDetail />)} />
          <Route path="help" element={gated(<Help />, 'help', 'admin_help')} />
          <Route path="help/:id" element={page(<HelpDetail />)} />
          <Route path="help/guides/new" element={gated(<HelpGuideEditor />, ['admin_help', 'update'])} />
          <Route path="help/guides/:slug/edit" element={gated(<HelpGuideEditor />, ['admin_help', 'update'])} />
          <Route path="help/guides/:slug" element={gated(<HelpGuideDetail />, 'help')} />
          <Route path="report" element={gated(<Report />, 'report')} />
          <Route path="notifications" element={page(<Notifications />)} />
          <Route path="studio" element={page(<GrokBotStudioPage />)} />
          <Route path="me" element={page(<Profile />)} />
          <Route path="me/family" element={page(<Profile />)} />
          <Route path="admin/menu" element={gated(<AdminMenu />, ...ADMIN_MENU_PERMS)} />
          <Route path="admin/traffic" element={gated(<TrafficHub />, ...TRAFFIC_HUB_PERMS)} />
          <Route path="admin" element={gated(<AdminDashboard />, 'admin_dashboard')} />
          <Route path="admin/rides" element={gated(<AdminRides />, 'admin_rides')} />
          <Route path="admin/rides/:id" element={gated(<RideDetail />, 'admin_rides')} />
          <Route path="admin/live" element={gated(<AdminLive />, 'admin_live')} />
          <Route path="admin/members" element={gated(<AdminMembers />, 'admin_members', 'admin_invites', 'admin_recovery')} />
          <Route path="admin/settings" element={gated(<AdminSettings />, 'admin_groups', 'admin_areas', 'admin_settings', 'admin_fares', 'admin_area_photos')} />
          <Route path="admin/reimbursements" element={gated(<AdminReimbursements />, 'admin_reimb_review', 'admin_reimb_pay')} />
          <Route path="admin/announcements" element={gated(<AdminAnnouncements />, 'admin_announcements')} />
          <Route path="admin/audit" element={gated(<AdminAudit />, 'admin_audit')} />
          <Route path="admin/reports" element={gated(<AdminReports />, 'admin_reports')} />
          <Route path="admin/roles" element={gated(<AdminRoles />, 'admin_roles')} />
          <Route path="admin/events/:id/plan" element={gated(<PlanTransport />, 'admin_plan')} />
        </Route>
        <Route path="*" element={page(<NotFound />)} />
      </Routes>
    </BackNavigationProvider>
  );
}
