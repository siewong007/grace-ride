import { CSSProperties, ReactNode, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Link, NavLink, Navigate, Outlet, useLocation, useNavigate } from 'react-router-dom';
import {
  ArrowLeft, Bell, Calendar, Car, CarFront, ChevronRight, ClipboardList, Clock, Fingerprint, HandCoins, Home,
  LayoutGrid, LifeBuoy, MapPinned, PanelLeftClose, PanelLeftOpen, Route, Search, Receipt, ShieldAlert, User, Wallet, BarChart3, WifiOff,
} from 'lucide-react';
import { useAuth } from '../auth';
import { BackNavigationProvider, getDefaultBack, SwipeBackGesture, useOptionalBackNavigation, useSmartBack } from './BackNavigation';
import { LogoMark, Wordmark } from './Brand';
import { GuideHeaderActionContext, PageChromeContext } from './PageChrome';
import { ErrorBoundary } from './AppChrome';
import { TermsGate } from './Terms';
import { Avatar, Button, IconButton, Notice, useToast } from './ui';
import { GuideRunnerProvider, PageHeaderGuideButton } from './GuideRunner';
import { SwipeTabsGesture, useMobileNav } from './SwipeTabs';
import { getBottomNavTabs, matchBottomTab } from '../bottomNav';
import QuickJump, { useQuickJumpShortcut } from './QuickJump';
import { useTranslation } from 'react-i18next';
import { useStaffNav, waitingBadge } from '../staffNav';
import { flushOfflineQueue, getOfflineQueue, useOfflineQueueTick } from '../offlineQueue';

interface NavItem {
  to: string;
  label: string;
  icon: ReactNode;
  end?: boolean;
  /** Items waiting on staff at this page (shown as a count). */
  badge?: number;
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
  if (pathname.startsWith('/reimbursements')) return t('reimbursements');
  if (pathname === '/report') return t('myReport');
  if (pathname === '/help/guides/new') return t('guideNew');
  if (pathname.includes('/help/guides/') && pathname.endsWith('/edit')) return t('guideEdit');
  if (pathname.startsWith('/help/guides/')) return t('guideDetail');
  if (pathname.startsWith('/help/')) return t('ticketDetail');
  if (pathname === '/help') return t('help');
  if (pathname === '/drive') return t('drive');
  if (pathname.startsWith('/settlements')) return t('settlements');
  if (pathname === '/notifications') return t('notifications');
  if (pathname === '/me') return t('profile');
  if (pathname === '/admin/menu') return t('adminSheetTitle');
  if (pathname === '/admin/traffic') return t('trafficHub');
  if (pathname === '/admin/rides') return t('adminRides');
  if (pathname.startsWith('/admin/rides/')) return t('rideDetail');
  if (/^\/admin\/events\/[^/]+\/plan$/.test(pathname)) return t('planTransport');
  if (pathname === '/admin/live') return t('liveMap');
  if (pathname === '/admin/announcements') return t('announcements');
  if (pathname === '/admin/reimbursements') return t('reimbReview');
  if (pathname === '/admin/members') return t('members');
  if (pathname === '/admin/roles') return t('roles');
  if (pathname === '/admin/reports') return t('reports');
  if (pathname === '/admin/settings') return t('settings');
  if (pathname === '/admin/audit') return t('audit');
  if (pathname === '/admin') return t('dashboard');
  return '';
}

function CollapsibleGroup({ open, children }: { open: boolean; children: ReactNode }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (ref.current) ref.current.inert = !open;
  }, [open]);
  return (
    <div ref={ref} className={`collapsible-grid${open ? ' is-open' : ''}`} aria-hidden={!open || undefined}>
      <div className="collapsible-grid-inner">{children}</div>
    </div>
  );
}

export default function Layout() {
  return (
    <BackNavigationProvider>
      <LayoutInner />
    </BackNavigationProvider>
  );
}

function LayoutInner() {
  const { t } = useTranslation('layout');
  const { t: tc } = useTranslation();
  const { me, can } = useAuth();
  const loc = useLocation();
  const navigate = useNavigate();
  const toast = useToast();
  const mobileNav = useMobileNav();
  const inAppHistory = useOptionalBackNavigation();
  const [scrolled, setScrolled] = useState(false);
  const sentinelRef = useRef<HTMLDivElement>(null);
  // Set by the page's PageHeader: its back target, and whether its heading is on screen.
  const [back, setBack] = useState<string | null>(null);
  const [headingVisible, setHeadingVisible] = useState(false);
  const [isOffline, setIsOffline] = useState(typeof navigator !== 'undefined' ? !navigator.onLine : false);
  const [syncingQueue, setSyncingQueue] = useState(false);
  useOfflineQueueTick();
  const chrome = useMemo(() => ({ setBack, setHeadingVisible }), []);
  const [navDir, setNavDir] = useState<'left' | 'right' | null>(null);
  const navDirTimerRef = useRef<number | null>(null);
  const [jumpOpen, setJumpOpen] = useState(false);
  const openJump = useCallback(() => {
    try {
      if (localStorage.getItem('gr_locked_user')) return;
    } catch {
      /* ignore */
    }
    setJumpOpen(true);
  }, []);
  useQuickJumpShortcut(openJump);
  const { staffGroups, staff, staffWaiting, helpDesk, isTrafficOnlyStaff, trafficBadge } = useStaffNav();
  const isHome = loc.pathname === '/';
  const pending = me?.status === 'pending';
  // The profile page itself, where a pending member's waiting-room checklist says what the banners would.
  const onMeRoot = loc.pathname === '/me' && !new URLSearchParams(loc.search).get('section');
  const effectiveBack = !isHome && !pending ? (back ?? getDefaultBack(loc.pathname, can)) : null;
  const smartBack = useSmartBack(effectiveBack, {
    disabled: !effectiveBack,
    isAuthenticated: Boolean(me),
  });
  // On tablets the sidebar is a rail of icons; this opens it to show the names. Remembered per device.
  const [railOpen, setRailOpen] = useState(() => {
    try {
      return localStorage.getItem('gr.railOpen') === '1';
    } catch {
      return false;
    }
  });
  const toggleRail = () =>
    setRailOpen((open) => {
      try {
        localStorage.setItem('gr.railOpen', open ? '0' : '1');
      } catch {
        // Not remembered when storage is blocked; the toggle still works.
      }
      return !open;
    });

  const [toolsOpen, setToolsOpen] = useState(() => {
    try {
      return localStorage.getItem('gr.sidebarToolsOpen') === '1';
    } catch {
      return false;
    }
  });
  const toggleTools = () =>
    setToolsOpen((open) => {
      const next = !open;
      try {
        localStorage.setItem('gr.sidebarToolsOpen', next ? '1' : '0');
      } catch {
        // Ignore storage restrictions; in-memory toggle still works.
      }
      return next;
    });

  const meId = me?.id;
  const runQueueSync = useCallback(async () => {
    if (!meId) return;
    setSyncingQueue(true);
    try {
      const res = await flushOfflineQueue(meId);
      if (res.conflicts > 0) {
        toast.info(tc('offlineSyncConflicts', { synced: res.synced, conflicts: res.conflicts }));
      } else if (res.synced > 0) {
        toast.success(tc('offlineSynced', { count: res.synced }));
      }
      if (res.synced > 0 || res.conflicts > 0) {
        window.dispatchEvent(new Event('gr:ride-stale'));
      }
    } finally {
      setSyncingQueue(false);
    }
  }, [meId, tc, toast]);

  useEffect(() => {
    const onOnline = () => {
      setIsOffline(false);
    };
    const onOffline = () => setIsOffline(true);
    window.addEventListener('online', onOnline);
    window.addEventListener('offline', onOffline);
    if (!isOffline && meId && getOfflineQueue(meId).length > 0) {
      void runQueueSync();
    }
    return () => {
      window.removeEventListener('online', onOnline);
      window.removeEventListener('offline', onOffline);
    };
  }, [isOffline, meId, runQueueSync]);

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

  useEffect(() => {
    return () => {
      if (navDirTimerRef.current !== null) window.clearTimeout(navDirTimerRef.current);
    };
  }, []);

  const pendingOfflineCount = me ? getOfflineQueue(me.id).length : 0;

  if (!me) return null;
  if (!me.terms_accepted) return <TermsGate />;
  const onMe = loc.pathname === '/me' || loc.pathname === '/me/family';
  if (pending && !onMe) return <Navigate to="/me" replace />;
  const sz = 20;

  // Each link shows only when the member may open that page; the server enforces the same.
  const links = (items: (NavItem & { show: boolean })[]): NavItem[] =>
    items.filter((n) => n.show && !pending).map(({ to, label, icon, end, badge }) => ({ to, label, icon, end, badge }));
  const member = links([
    { to: '/', label: t('home'), icon: <Home size={sz} />, end: true, show: true },
    { to: '/events', label: t('events'), icon: <Calendar size={sz} />, show: can('events') },
  ]);
  const sidebarMemberTools = links([
    { to: '/drive', label: t('drive'), icon: <CarFront size={sz} />, show: can('drive') },
    { to: '/rides', label: t('myRides'), icon: <Car size={sz} />, show: can('rides') },
    { to: '/costs', label: t('myCosts'), icon: <Wallet size={sz} />, show: can('costs') },
    { to: '/settlements', label: t('settlements'), icon: <HandCoins size={sz} />, show: can('settlements') || can('admin_bills') },
    { to: '/reimbursements', label: t('reimbursements'), icon: <Receipt size={sz} />, show: can('reimbursements') },
    { to: '/report', label: t('myReport'), icon: <BarChart3 size={sz} />, show: can('report') },
    // Help-desk staff reach the same page as "Help desk" in their own section.
    { to: '/help', label: t('help'), icon: <LifeBuoy size={sz} />, show: can('help') && !helpDesk },
  ]);
  const profileTab: NavItem = { to: '/me', label: t('profile'), icon: <User size={sz} /> };
  // Phone tab bar: home, events, optional staff/traffic tab, and profile.
  // Swipe uses this same list so it cannot wander onto sidebar-only pages.
  const bottomTabs = getBottomNavTabs({
    pending,
    canEvents: can('events'),
    hasStaffNav: !pending && staff.length > 0,
    trafficOnly: isTrafficOnlyStaff,
  });
  const onAdminPage = loc.pathname.startsWith('/admin') || (helpDesk && loc.pathname.startsWith('/help'));
  const badge = (n?: number) => waitingBadge(n, t('waitingCount', { count: n ?? 0 }));
  const tabIconSize = 22;
  const bottomNavItems = bottomTabs.map((tab) => {
    switch (tab.id) {
      case 'home':
        return {
          id: tab.id,
          to: '/',
          end: true,
          label: t('home'),
          icon: <Home size={tabIconSize} />,
          active: loc.pathname === '/',
          isActiveOverride: undefined as boolean | undefined,
        };
      case 'events':
        return {
          id: tab.id,
          to: '/events',
          end: false,
          label: t('events'),
          icon: <Calendar size={tabIconSize} />,
          active: loc.pathname === '/events' || loc.pathname.startsWith('/events/'),
          isActiveOverride: undefined as boolean | undefined,
        };
      case 'traffic':
        return {
          id: tab.id,
          to: '/admin/traffic',
          end: false,
          label: t('trafficTab'),
          icon: (
            <>
              <Route size={tabIconSize} />
              {badge(trafficBadge)}
            </>
          ),
          active: onAdminPage,
          isActiveOverride: onAdminPage,
        };
      case 'admin':
        return {
          id: tab.id,
          to: '/admin/menu',
          end: false,
          label: t('sectionAdmin'),
          icon: (
            <>
              <LayoutGrid size={tabIconSize} />
              {badge(staffWaiting)}
            </>
          ),
          active: onAdminPage,
          isActiveOverride: onAdminPage,
        };
      case 'profile':
        return {
          id: tab.id,
          to: '/me',
          end: false,
          label: t('profile'),
          icon: (
            <span className="bottom-tab-avatar">
              <Avatar name={me.name} fileId={me.photo_file_id} color={me.avatar_color} icon={me.avatar_icon} size="sm" />
            </span>
          ),
          active: loc.pathname === '/me' || loc.pathname.startsWith('/me/'),
          isActiveOverride: undefined as boolean | undefined,
        };
    }
  });
  const activeBottomIndex = bottomNavItems.findIndex((item) => item.active);
  const matchedTab = matchBottomTab(loc.pathname, bottomTabs, helpDesk);
  const matchedTabIndex = matchedTab ? bottomTabs.findIndex((tab) => tab.id === matchedTab.id) : -1;
  const triggerNavDir = (direction: 'left' | 'right' | null) => {
    if (navDirTimerRef.current !== null) window.clearTimeout(navDirTimerRef.current);
    setNavDir(direction);
    navDirTimerRef.current = window.setTimeout(() => {
      setNavDir(null);
      navDirTimerRef.current = null;
    }, 280);
  };
  const handleBottomTabClick = (targetIdx: number) => {
    if (matchedTabIndex !== -1 && targetIdx !== matchedTabIndex) {
      triggerNavDir(targetIdx > matchedTabIndex ? 'left' : 'right');
    }
  };

  const sidebarLink = (n: NavItem) => (
    <NavLink
      key={n.to}
      to={n.to}
      end={n.end}
      className={({ isActive }) => `sidebar-link ${isActive ? 'active' : ''}`}
    >
      <span className="sidebar-icon">{n.icon}</span>
      <span className="sidebar-label">{n.label}</span>
      {badge(n.badge)}
      <span className="sidebar-tooltip" role="tooltip">{n.label}</span>
    </NavLink>
  );

  const prevInApp = inAppHistory?.previousEntry;
  const canSwipeBack = Boolean(
    prevInApp && !(prevInApp.pathname === '/login' || prevInApp.pathname.startsWith('/invite/')),
  );
  const nextInApp = inAppHistory?.nextEntry;
  const canSwipeForward = Boolean(
    nextInApp && !(nextInApp.pathname === '/login' || nextInApp.pathname.startsWith('/invite/')),
  );

  const unreadLabel = me.unread ? t('unread', { count: me.unread }) : '';
  const pageTitle = getPageTitle(loc.pathname, t);

  return (
    <div className={`shell${railOpen ? ' rail-open' : ''}`}>
      <nav className="sidebar" aria-label={t('mainNav')}>
        <button
          type="button"
          className="sidebar-toggle"
          aria-expanded={railOpen}
          aria-label={railOpen ? t('collapseNav') : t('expandNav')}
          onClick={toggleRail}
        >
          {railOpen ? <PanelLeftClose size={20} aria-hidden="true" /> : <PanelLeftOpen size={20} aria-hidden="true" />}
        </button>
        <Link to={pending ? '/me' : '/'} className="sidebar-brand">
          <LogoMark animated />
          <span className="sidebar-brand-text">
            <Wordmark />
            <small>{t('brandSub')}</small>
          </span>
        </Link>
        {member.length > 0 && staffGroups.length > 0 && <div className="section-label">{t('sectionMember')}</div>}
        {member.map(sidebarLink)}
        {sidebarMemberTools.length > 0 && (() => {
          const inMemberTools = sidebarMemberTools.some(
            (item) => loc.pathname === item.to || loc.pathname.startsWith(`${item.to}/`),
          );
          const showToolsItems = staffGroups.length === 0 || toolsOpen || inMemberTools;
          return (
            <div className="sidebar-group" role="group" aria-label={t('sectionTools')}>
              {staffGroups.length > 0 ? (
                <button
                  type="button"
                  className="sidebar-section-toggle"
                  aria-expanded={showToolsItems}
                  onClick={toggleTools}
                >
                  <span>{t('sectionTools')} ({sidebarMemberTools.length})</span>
                  <ChevronRight
                    size={14}
                    className={`sidebar-section-chevron${showToolsItems ? ' is-open' : ''}`}
                    aria-hidden="true"
                  />
                </button>
              ) : (
                <div className="section-label" aria-hidden="true">{t('sectionTools')}</div>
              )}
              <CollapsibleGroup open={showToolsItems}>
                {sidebarMemberTools.map(sidebarLink)}
              </CollapsibleGroup>
            </div>
          );
        })()}
        {staffGroups.map((g) => (
          <div key={g.key} className="sidebar-group" role="group" aria-label={g.label}>
            <div className="section-label" aria-hidden="true">{g.label}</div>
            {g.items.map(sidebarLink)}
          </div>
        ))}
        <div className="spacer" />
        {sidebarLink({ to: '/me', label: me.name, icon: <User size={sz} /> })}
      </nav>

      <div className="shell-content">
        <div ref={sentinelRef} className="scroll-sentinel" aria-hidden="true" />
        <header className={`topbar ${scrolled ? 'is-scrolled' : ''}`}>
          <div className="topbar-left">
            {isHome ? (
              // Phones only: from tablet up the sidebar already carries the brand.
              <Link to="/" className="topbar-brand hide-tablet-up">
                <LogoMark animated />
                <Wordmark className="topbar-brand-name" />
              </Link>
            ) : (
              <>
                {effectiveBack && (
                  <Link
                    to={smartBack.href}
                    onClick={smartBack.onClick}
                    className="icon-btn topbar-back"
                    aria-label={tc('back')}
                  >
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
              <>
                <button type="button" className="btn sm ghost topbar-jump hide-phone" onClick={openJump}>
                  <Search size={16} aria-hidden="true" />
                  <span>{t('jumpButton')}</span>
                  <kbd aria-hidden="true">{/Mac|iPhone|iPad/.test(navigator.platform) ? '⌘K' : 'Ctrl K'}</kbd>
                </button>
                <IconButton
                  icon={<Search size={20} />}
                  label={t('jumpButton')}
                  variant="ghost"
                  className="hide-tablet-up"
                  onClick={openJump}
                />
              </>
            )}
            {!pending && (
              <IconButton
                to="/notifications"
                icon={<Bell size={20} />}
                label={t('notifications') + (unreadLabel ? ` ${unreadLabel}` : '')}
                variant="ghost"
                className="topbar-bell"
              >
                {me.unread > 0 && (
                  <span key={me.unread > 9 ? '9+' : me.unread} className="dot" aria-hidden="true">
                    {me.unread > 9 ? '9+' : me.unread}
                  </span>
                )}
              </IconButton>
            )}
            <Link to="/me" className="topbar-avatar hide-mobile" aria-label={t('profile')}>
              <Avatar name={me.name} fileId={me.photo_file_id} color={me.avatar_color} icon={me.avatar_icon} size="sm" />
            </Link>
          </div>
        </header>

        {pending && !onMeRoot && (
          <div className="shell-notice-container">
            <Notice tone="warning" icon={<Clock size={20} />}>
              {t('pendingNotice')}
            </Notice>
          </div>
        )}
        {me.passkey_count === 0 && !me.dev_mode && !(pending && onMeRoot) && !(loc.pathname === '/me' && loc.search.includes('section=security')) && (
          <div className="shell-notice-container">
            <Notice tone="warning" icon={<Fingerprint size={20} />}>
              {t('noPasskeyNotice')}{' '}
              <Link to="/me?section=security" className="bold">{t('noPasskeyAction')}</Link>
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
        {isOffline && (
          <div className="shell-notice-container" role="status" aria-live="polite">
            <Notice tone="warning" icon={<WifiOff size={20} />}>
              {tc('offlineNotice')}
            </Notice>
          </div>
        )}
        {pendingOfflineCount > 0 && (
          <div className="shell-notice-container" role="status" aria-live="polite">
            <Notice tone="info">
              <div className="row between items-center wrap gap-xs">
                <span>{tc('offlinePendingBanner', { count: pendingOfflineCount })}</span>
                {!isOffline && (
                  <Button variant="secondary" size="sm" loading={syncingQueue} onClick={() => void runQueueSync()}>
                    {tc('offlineSyncNow')}
                  </Button>
                )}
              </div>
            </Notice>
          </div>
        )}
        <main className="main" id="main" data-nav-dir={navDir ?? undefined}>
          <PageChromeContext.Provider value={chrome}>
            <GuideRunnerProvider>
              <GuideHeaderActionContext.Provider value={<PageHeaderGuideButton />}>
                <ErrorBoundary key={loc.pathname}>
                  <Outlet />
                </ErrorBoundary>
              </GuideHeaderActionContext.Provider>
            </GuideRunnerProvider>
          </PageChromeContext.Provider>
        </main>
      </div>

      <nav
        className="bottom-nav"
        aria-label={t('bottomNav')}
        data-has-active={activeBottomIndex >= 0 ? 'true' : 'false'}
        data-tab-count={bottomNavItems.length}
        style={{
          '--tab-count': bottomNavItems.length,
          '--active-index': Math.max(0, activeBottomIndex),
        } as CSSProperties}
      >
        {bottomNavItems.map((item, idx) => (
          <NavLink
            key={item.id}
            to={item.to}
            end={item.end}
            onClick={() => handleBottomTabClick(idx)}
            className={({ isActive }) =>
              `bottom-tab ${(item.isActiveOverride ?? isActive) ? 'active' : ''}`
            }
          >
            <span className="bottom-tab-icon">{item.icon}</span>
            <span className="bottom-tab-label sr-only">{item.label}</span>
          </NavLink>
        ))}
      </nav>

      {jumpOpen && !pending && (
        <QuickJump
          pages={[
            ...member,
            ...sidebarMemberTools,
            ...staff,
            ...links([
              { to: '/admin/rides', label: t('adminRides'), icon: <ClipboardList size={sz} />, show: can('admin_rides') },
              { to: '/admin/live', label: t('liveMap'), icon: <MapPinned size={sz} />, show: can('admin_live') },
            ]),
            profileTab,
          ].filter(
            (p, i, all) => all.findIndex((x) => x.to === p.to) === i,
          )}
          onClose={() => setJumpOpen(false)}
        />
      )}
      <SwipeBackGesture enabled={Boolean(effectiveBack)} onBack={smartBack.goBack} edgeOnly={mobileNav} />
      <SwipeTabsGesture
        enabled={mobileNav && bottomTabs.length > 1}
        tabs={bottomTabs}
        helpDesk={helpDesk}
        pathname={loc.pathname}
        search={loc.search}
        canGoBack={canSwipeBack}
        canGoForward={canSwipeForward}
        onAction={(action, direction) => {
          triggerNavDir(direction ?? null);
          if (action.type === 'history-back') navigate(-1);
          else if (action.type === 'history-forward') navigate(1);
          else navigate(action.to);
        }}
      />
    </div>
  );
}
