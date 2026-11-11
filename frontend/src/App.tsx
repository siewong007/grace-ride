import { ComponentType, lazy, ReactNode, Suspense } from 'react';
import { Navigate, Route, Routes, useLocation } from 'react-router-dom';
import { useAuth } from './auth';
import Layout from './components/Layout';
import { Spinner } from './components/ui';
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

/** Wrap a lazily-loaded page so only the content area shows a spinner while it loads. */
const page = (el: ReactNode) => <Suspense fallback={<div className="page-loading"><Spinner /></div>}>{el}</Suspense>;

function RequireAuth({ children }: { children: JSX.Element }) {
  const { me, loading } = useAuth();
  const loc = useLocation();
  if (loading) return <Spinner />;
  if (!me) return <Navigate to="/login" state={{ from: loc.pathname }} replace />;
  return children;
}

export default function App() {
  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route path="/terms" element={<TermsPage />} />
      <Route path="/invite/:token" element={page(<Invite />)} />
      <Route element={<RequireAuth><Layout /></RequireAuth>}>
        <Route index element={<Home />} />
        <Route path="rides" element={page(<Rides />)} />
        <Route path="rides/new" element={page(<RideNew />)} />
        <Route path="rides/:id" element={page(<RideDetail />)} />
        <Route path="drive" element={page(<Drive />)} />
        <Route path="events" element={page(<Events />)} />
        <Route path="events/new" element={page(<EventForm />)} />
        <Route path="events/:id" element={page(<EventDetail />)} />
        <Route path="events/:id/edit" element={page(<EventForm />)} />
        <Route path="events/:eventId/settle" element={page(<SettlementForm />)} />
        <Route path="settlements" element={page(<Settlements />)} />
        <Route path="settlements/:id" element={page(<SettlementDetail />)} />
        <Route path="settlements/:id/edit" element={page(<SettlementForm />)} />
        <Route path="costs" element={page(<Costs />)} />
        <Route path="reimbursements" element={page(<Reimbursements />)} />
        <Route path="reimbursements/new" element={page(<ReimbursementForm />)} />
        <Route path="reimbursements/:id" element={page(<ReimbursementDetail />)} />
        <Route path="help" element={page(<Help />)} />
        <Route path="help/:id" element={page(<HelpDetail />)} />
        <Route path="report" element={page(<Report />)} />
        <Route path="notifications" element={page(<Notifications />)} />
        <Route path="me" element={page(<Profile />)} />
        <Route path="admin" element={page(<AdminDashboard />)} />
        <Route path="admin/rides" element={page(<AdminRides />)} />
        <Route path="admin/rides/:id" element={page(<RideDetail />)} />
        <Route path="admin/live" element={page(<AdminLive />)} />
        <Route path="admin/members" element={page(<AdminMembers />)} />
        <Route path="admin/settings" element={page(<AdminSettings />)} />
        <Route path="admin/reimbursements" element={page(<AdminReimbursements />)} />
        <Route path="admin/announcements" element={page(<AdminAnnouncements />)} />
        <Route path="admin/audit" element={page(<AdminAudit />)} />
      </Route>
      <Route path="*" element={page(<NotFound />)} />
    </Routes>
  );
}
