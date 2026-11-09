import { ReactNode } from 'react';
import { Link, NavLink, Navigate, Outlet, useLocation } from 'react-router-dom';
import {
  Bell, Calendar, Car, CarFront, ClipboardList, HandCoins, HeartHandshake, Home, LayoutDashboard,
  LifeBuoy, MapPinned, Megaphone, Receipt, ScrollText, User, Users, Wallet, BarChart3,
} from 'lucide-react';
import { useAuth } from '../auth';
import { TermsGate } from './Terms';
import { useTranslation } from 'react-i18next';

interface NavItem { to: string; label: string; icon: ReactNode; end?: boolean }

export default function Layout() {
  const { t } = useTranslation('layout');
  const { me, has, isStaff } = useAuth();
  const loc = useLocation();
  if (!me) return null;
  if (!me.terms_accepted) return <TermsGate />;
  const pending = me.status === 'pending';
  if (pending && loc.pathname !== '/me') return <Navigate to="/me" replace />;
  const sz = 22;

  const member: NavItem[] = pending ? [] : [
    { to: '/', label: t('home'), icon: <Home size={sz} />, end: true },
    { to: '/rides', label: t('myRides'), icon: <Car size={sz} /> },
    { to: '/events', label: t('events'), icon: <Calendar size={sz} /> },
    { to: '/costs', label: t('myCosts'), icon: <Wallet size={sz} /> },
    { to: '/reimbursements', label: t('reimbursements'), icon: <Receipt size={sz} /> },
    { to: '/report', label: t('myReport'), icon: <BarChart3 size={sz} /> },
    { to: '/help', label: t('help'), icon: <LifeBuoy size={sz} /> },
  ];
  const driver: NavItem[] = has('driver') ? [{ to: '/drive', label: t('drive'), icon: <CarFront size={sz} /> }] : [];
  const leader: NavItem[] = has('leader') ? [{ to: '/settlements', label: t('settlements'), icon: <HandCoins size={sz} /> }] : [];
  const staff: NavItem[] = [];
  if (isStaff) {
    staff.push(
      { to: '/admin', label: t('dashboard'), icon: <LayoutDashboard size={sz} />, end: true },
      { to: '/admin/rides', label: t('adminRides'), icon: <ClipboardList size={sz} /> },
      { to: '/admin/live', label: t('liveMap'), icon: <MapPinned size={sz} /> },
      { to: '/admin/announcements', label: t('announcements'), icon: <Megaphone size={sz} /> },
    );
  }
  if ((has('transport') || has('finance')) && me.mfa_ok) {
    staff.push({ to: '/admin/reimbursements', label: t('reimbReview'), icon: <HeartHandshake size={sz} /> });
  }
  if (has('admin') && me.mfa_ok) {
    staff.push(
      { to: '/admin/members', label: t('members'), icon: <Users size={sz} /> },
      { to: '/admin/settings', label: t('settings'), icon: <ScrollText size={sz} /> },
    );
  }
  if (isStaff) staff.push({ to: '/admin/audit', label: t('audit'), icon: <ScrollText size={sz} /> });

  const bottom: NavItem[] = pending
    ? [{ to: '/me', label: t('profile'), icon: <User size={sz} /> }]
    : [
        member[0],
        member[1],
        ...(has('driver') ? driver : []),
        member[2],
        ...(isStaff && !has('driver') ? [staff[0]] : []),
        { to: '/me', label: t('profile'), icon: <User size={sz} /> },
      ].slice(0, 5);

  const link = (n: NavItem) => (
    <NavLink key={n.to} to={n.to} end={n.end} className={({ isActive }) => (isActive ? 'active' : '')}>
      {n.icon}
      <span>{n.label}</span>
    </NavLink>
  );

  const unreadLabel = me.unread ? t('unread', { count: me.unread }) : '';

  return (
    <div className="shell">
      <nav className="sidebar" aria-label="主选单">
        <Link to={pending ? '/me' : '/'} className="brand">
          <img className="brand-mark" src="/icon.svg" alt="" />
          <span>{t('brand')}<small>{t('brandSub')}</small></span>
        </Link>
        {member.length > 0 && <div className="section-label">{t('sectionMember')}</div>}
        {member.map(link)}
        {(driver.length > 0 || leader.length > 0) && <div className="section-label">{t('sectionService')}</div>}
        {driver.map(link)}
        {leader.map(link)}
        {staff.length > 0 && <div className="section-label">{t('sectionAdmin')}</div>}
        {staff.map(link)}
        <div className="spacer" />
        {link({ to: '/me', label: me.name, icon: <User size={sz} /> })}
      </nav>

      <div>
        <header className="topbar">
          <Link to={pending ? '/me' : '/'} className="brand">
            <img className="brand-mark" src="/icon.svg" alt="" />
            {t('brand')}
          </Link>
          {!pending && (
            <Link to="/notifications" className="icon-btn" aria-label={t('notifications') + (unreadLabel ? ` ${unreadLabel}` : '')}>
              <Bell size={22} />
              {me.unread > 0 && <span className="dot">{me.unread > 99 ? '99+' : me.unread}</span>}
            </Link>
          )}
        </header>
        {pending && (
          <div className="main" style={{ paddingBottom: 0 }}>
            <div className="notice">{t('pendingNotice')}</div>
          </div>
        )}
        {me.privileged && !me.mfa_ok && (
          <div className="main" style={{ paddingBottom: 0 }}>
            <div className="notice">
              {t('adminNotice')}
            </div>
          </div>
        )}
        <main className="main">
          <Outlet />
        </main>
      </div>

      <nav className="bottom-nav" aria-label="底部选单">
        {bottom.map(link)}
      </nav>
    </div>
  );
}
