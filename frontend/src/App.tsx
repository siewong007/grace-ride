import { ComponentType, lazy, ReactNode, Suspense, useEffect, useLayoutEffect } from 'react';
import { Navigate, Route, Routes, useLocation } from 'react-router-dom';
import { useAuth } from './auth';
import Layout from './components/Layout';
import { EmptyState, Page, Spinner } from './components/ui';
import type { Act, PageKey } from './perms';
import { useTranslation } from 'react-i18next';
import { hideSplash, holdSplash } from './splash';
import { AutoLockProvider } from './components/AutoLock';
import Login from './pages/Login';
import Home from './pages/Home';
import TermsPage from './components/Terms';

// Route-level code splitting: the login screen and home page load first; everything
// else (admin tools, Leaflet maps, cost splitting) downloads only when visited.
const named = <M extends Record<string, unknown>, K extends keyof M>(load: () => Promise<M>, key: K) =>
  lazy(() => load().then((m) => ({ default: m[key] as ComponentType })));
const Invite = lazy(() => import('./pages/Invite'));
const Rides = lazy(() => import('./pages/Rides'));
const RideNew = lazy(() => import('./pages/RideNew'));
const RideDetail = lazy(() => import('./pages/RideDetail'));
const Drive = lazy(() => import('./pages/Drive'));
const Events = lazy(() => import('./pages/Events'));
const EventDetail = lazy(() => import('./pages/EventDetail'));
const EventForm = lazy(() => import('./pages/EventForm'));
const Settlements = lazy(() => import('./pages/Settlements'));
const SettlementDetail = named(() => import('./pages/Settlements'), 'SettlementDetail');
const SettlementForm = named(() => import('./pages/Settlements'), 'SettlementForm');
const Costs = lazy(() => import('./pages/Costs'));
const Reimbursements = lazy(() => import('./pages/Reimbursements'));
const ReimbursementDetail = named(() => import('./pages/Reimbursements'), 'ReimbursementDetail');
const ReimbursementForm = named(() => import('./pages/Reimbursements'), 'ReimbursementForm');
const Help = lazy(() => import('./pages/Help'));
const HelpDetail = named(() => import('./pages/Help'), 'HelpDetail');
const HelpGuideDetail = lazy(() => import('./pages/HelpGuideDetail'));
const Report = lazy(() => import('./pages/Report'));
const Notifications = lazy(() => import('./pages/Notifications'));
const Profile = lazy(() => import('./pages/Profile'));
const NotFound = lazy(() => import('./pages/NotFound'));
const AdminDashboard = lazy(() => import('./pages/admin/Dashboard'));
const AdminRides = lazy(() => import('./pages/admin/Rides'));
const AdminLive = lazy(() => import('./pages/admin/Live'));
const AdminMembers = lazy(() => import('./pages/admin/Members'));
const AdminSettings = lazy(() => import('./pages/admin/Settings'));
const AdminReimbursements = lazy(() => import('./pages/admin/Reimbursements'));
const AdminAnnouncements = lazy(() => import('./pages/admin/Announcements'));
const AdminAudit = lazy(() => import('./pages/admin/Audit'));
const AdminReports = lazy(() => import('./pages/admin/Reports'));
const PlanTransport = lazy(() => import('./pages/admin/PlanTransport'));
const AdminRoles = lazy(() => import('./pages/admin/Roles'));

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
  const { t } = useTranslation();
  const allowed = any.some((n) => (typeof n === 'string' ? can(n) : can(n[0], n[1])));
  if (allowed) return <>{children}</>;
  // Staff permissions stay off until the member signs in with a passkey or 2FA.
  const needsMfa = !!me?.privileged && !me.mfa_ok;
  return (
    <Page>
      <EmptyState title={t('noAccess.title')} description={needsMfa ? t('noAccess.mfa') : t('noAccess.body')} />
    </Page>
  );
}

const gated = (el: ReactNode, ...any: Need[]) => page(<Gate any={any}>{el}</Gate>);

function RequireAuth({ children }: { children: JSX.Element }) {
  const { me, loading } = useAuth();
  const loc = useLocation();
  if (loading) return <Spinner />;
  if (!me) return <Navigate to="/login" state={{ from: loc.pathname }} replace />;
  return <AutoLockProvider>{children}</AutoLockProvider>;
}

export default function App() {
  const { loading } = useAuth();
  // The boot splash keeps playing until we know whether to show the app or the login screen.
  useEffect(() => {
    if (!loading) hideSplash();
  }, [loading]);

  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route path="/terms" element={<TermsPage />} />
      <Route path="/invite/:token" element={page(<Invite />)} />
      <Route element={<RequireAuth><Layout /></RequireAuth>}>
        <Route index element={<Home />} />
        <Route path="rides" element={gated(<Rides />, 'rides')} />
        <Route path="rides/new" element={gated(<RideNew />, ['rides', 'create'])} />
        <Route path="rides/:id" element={page(<RideDetail />)} />
        <Route path="drive" element={gated(<Drive />, 'drive')} />
        <Route path="events" element={gated(<Events />, 'events')} />
        <Route path="events/new" element={gated(<EventForm />, ['events', 'create'])} />
        <Route path="events/:id" element={gated(<EventDetail />, 'events', 'admin_events')} />
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
        <Route path="help/guides/:slug" element={gated(<HelpGuideDetail />, 'help')} />
        <Route path="report" element={gated(<Report />, 'report')} />
        <Route path="notifications" element={page(<Notifications />)} />
        <Route path="me" element={page(<Profile />)} />
        <Route path="admin" element={gated(<AdminDashboard />, 'admin_dashboard')} />
        <Route path="admin/rides" element={gated(<AdminRides />, 'admin_rides')} />
        <Route path="admin/rides/:id" element={gated(<RideDetail />, 'admin_rides')} />
        <Route path="admin/live" element={gated(<AdminLive />, 'admin_live')} />
        <Route path="admin/members" element={gated(<AdminMembers />, 'admin_members', 'admin_invites', 'admin_recovery')} />
        <Route path="admin/settings" element={gated(<AdminSettings />, 'admin_groups', 'admin_areas', 'admin_settings')} />
        <Route path="admin/reimbursements" element={gated(<AdminReimbursements />, 'admin_reimb_review', 'admin_reimb_pay')} />
        <Route path="admin/announcements" element={gated(<AdminAnnouncements />, 'admin_announcements')} />
        <Route path="admin/audit" element={gated(<AdminAudit />, 'admin_audit')} />
        <Route path="admin/reports" element={gated(<AdminReports />, 'admin_reports')} />
        <Route path="admin/roles" element={gated(<AdminRoles />, 'admin_roles')} />
        <Route path="admin/events/:id/plan" element={gated(<PlanTransport />, 'admin_plan')} />
      </Route>
      <Route path="*" element={page(<NotFound />)} />
    </Routes>
  );
}
