import { ReactNode, useEffect, useMemo, useRef, useState } from 'react';
import { Link, NavLink, Navigate, Outlet, useLocation } from 'react-router-dom';
import {
  ArrowLeft, Bell, Calendar, Car, CarFront, ClipboardList, Clock, HandCoins, HeartHandshake, Home, LayoutDashboard,
  LifeBuoy, MapPinned, Megaphone, Receipt, ScrollText, Settings, ShieldAlert, User, Users, Wallet, BarChart3,
} from 'lucide-react';
import { useAuth } from '../auth';
import { LogoMark, Wordmark } from './Brand';
import { PageChromeContext } from './PageChrome';
import { TermsGate } from './Terms';
import { Avatar, IconButton, Notice } from './ui';
import { useTranslation } from 'react-i18next';

interface NavItem {
  to: string;
  label: string;
  icon: ReactNode;
  end?: boolean;
}

function getPageTitle(pathname: string, t: (key: string) => string): string {
  if (pathname === '/' || pathname === '') return t('home');
  if (pathname.startsWith('/rides/new')) return t('newRide');
  if (pathname.startsWith('/rides/')) return t('rideDetail');
  if (pathname === '/rides') return t('myRides');
  if (pathname.startsWith('/events/new')) return t('newEvent');
  if (pathname.includes('/events/') && pathname.endsWith('/edit')) return t('editEvent');
  if (pathname.startsWith('/events/')) return t('eventDetail');
  if (pathname === '/events') return t('events');
  if (pathname === '/costs') return t('myCosts');
  if (pathname === '/reimbursements') return t('reimbursements');
  if (pathname === '/report') return t('myReport');
  if (pathname === '/help') return t('help');
  if (pathname === '/drive') return t('drive');
  if (pathname === '/settlements') return t('settlements');
  if (pathname === '/notifications') return t('notifications');
  if (pathname === '/me') return t('profile');
  if (pathname === '/admin/rides') return t('adminRides');
  if (pathname === '/admin/live') return t('liveMap');
  if (pathname === '/admin/announcements') return t('announcements');
  if (pathname === '/admin/reimbursements') return t('reimbReview');
  if (pathname === '/admin/members') return t('members');
  if (pathname === '/admin/settings') return t('settings');
  if (pathname === '/admin/audit') return t('audit');
  if (pathname === '/admin') return t('dashboard');
  return '';
}

export default function Layout() {
  const { t } = useTranslation('layout');
  const { t: tc } = useTranslation();
  const { me, has, isStaff } = useAuth();
  const loc = useLocation();
  const [scrolled, setScrolled] = useState(false);
  const sentinelRef = useRef<HTMLDivElement>(null);
  // Set by the page's PageHeader: its back target, and whether its heading is on screen.
  const [back, setBack] = useState<string | null>(null);
  const [headingVisible, setHeadingVisible] = useState(false);
  const chrome = useMemo(() => ({ setBack, setHeadingVisible }), []);

  useEffect(() => {
    const el = sentinelRef.current;
    if (!el || typeof IntersectionObserver === 'undefined') return;
    const obs = new IntersectionObserver(
      ([entry]) => {
        setScrolled(!entry.isIntersecting);
      },
      { threshold: 0 }
    );
    obs.observe(el);
    return () => obs.disconnect();
  }, []);

  if (!me) return null;
  if (!me.terms_accepted) return <TermsGate />;
  const pending = me.status === 'pending';
  if (pending && loc.pathname !== '/me') return <Navigate to="/me" replace />;
  const sz = 20;

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
      { to: '/admin/settings', label: t('settings'), icon: <Settings size={sz} /> },
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

  const sidebarLink = (n: NavItem) => (
    <NavLink
      key={n.to}
      to={n.to}
      end={n.end}
      className={({ isActive }) => `sidebar-link ${isActive ? 'active' : ''}`}
    >
      <span className="sidebar-icon">{n.icon}</span>
      <span className="sidebar-label">{n.label}</span>
      <span className="sidebar-tooltip" role="tooltip">{n.label}</span>
    </NavLink>
  );

  const unreadLabel = me.unread ? t('unread', { count: me.unread }) : '';
  const isHome = loc.pathname === '/';
  const pageTitle = getPageTitle(loc.pathname, t);

  return (
    <div className="shell">
      <nav className="sidebar" aria-label={t('mainNav')}>
        <Link to={pending ? '/me' : '/'} className="sidebar-brand">
          <LogoMark />
          <span className="sidebar-brand-text">
            <Wordmark />
            <small>{t('brandSub')}</small>
          </span>
        </Link>
        {member.length > 0 && <div className="section-label">{t('sectionMember')}</div>}
        {member.map(sidebarLink)}
        {(driver.length > 0 || leader.length > 0) && <div className="section-label">{t('sectionService')}</div>}
        {driver.map(sidebarLink)}
        {leader.map(sidebarLink)}
        {staff.length > 0 && <div className="section-label">{t('sectionAdmin')}</div>}
        {staff.map(sidebarLink)}
        <div className="spacer" />
        {sidebarLink({ to: '/me', label: me.name, icon: <User size={sz} /> })}
      </nav>

      <div className="shell-content">
        <div ref={sentinelRef} className="scroll-sentinel" aria-hidden="true" />
        <header className={`topbar ${scrolled ? 'is-scrolled' : ''}`}>
          <div className="topbar-left">
            {isHome ? (
              <Link to="/" className="topbar-brand">
                <LogoMark />
                <Wordmark className="topbar-brand-name" />
              </Link>
            ) : (
              <>
                {back && (
                  <Link to={back} className="icon-btn topbar-back hide-tablet-up" aria-label={tc('back')}>
                    <ArrowLeft size={22} />
                  </Link>
                )}
                {/* The page's own h1 is the heading; this is a reminder once it scrolls away. */}
                <span className={`topbar-title ${headingVisible ? 'is-hidden' : ''}`.trim()} aria-hidden={headingVisible}>
                  {pageTitle}
                </span>
              </>
            )}
          </div>

          <div className="topbar-actions row nowrap">
            {!pending && (
              <IconButton
                to="/notifications"
                icon={<Bell size={20} />}
                label={t('notifications') + (unreadLabel ? ` ${unreadLabel}` : '')}
                variant="ghost"
                className="topbar-bell"
              >
                {me.unread > 0 && (
                  <span className="dot" aria-hidden="true">
                    {me.unread > 9 ? '9+' : me.unread}
                  </span>
                )}
              </IconButton>
            )}
            <Link to="/me" className="topbar-avatar" aria-label={t('profile')}>
              <Avatar name={me.name} fileId={me.photo_file_id} size="sm" />
            </Link>
          </div>
        </header>

        {pending && (
          <div className="shell-notice-container">
            <Notice tone="warning" icon={<Clock size={20} />}>
              {t('pendingNotice')}
            </Notice>
          </div>
        )}
        {me.privileged && !me.mfa_ok && (
          <div className="shell-notice-container">
            <Notice tone="info" icon={<ShieldAlert size={20} />}>
              {t('adminNotice')}
            </Notice>
          </div>
        )}
        <main className="main" id="main">
          <PageChromeContext.Provider value={chrome}>
            <Outlet />
          </PageChromeContext.Provider>
        </main>
      </div>

      <nav className="bottom-nav" aria-label={t('bottomNav')}>
        {bottom.map((n) => (
          <NavLink
            key={n.to}
            to={n.to}
            end={n.end}
            className={({ isActive }) => `bottom-tab ${isActive ? 'active' : ''}`}
          >
            <span className="bottom-tab-icon">{n.icon}</span>
            <span className="bottom-tab-label">{n.label}</span>
          </NavLink>
        ))}
      </nav>
    </div>
  );
}
