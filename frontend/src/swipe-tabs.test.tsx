import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { Link, MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import i18n from './i18n';
import Layout from './components/Layout';
import { BackNavigationProvider } from './components/BackNavigation';
import { PageHeader } from './components/ui';
import { MOBILE_NAV_MQ, SWIPE_EDGE_IGNORE_PX } from './components/SwipeTabs';

type Audience = 'member' | 'staff';

let audience: Audience = 'member';
const staffGranted = [
  'rides', 'events', 'help', 'admin_dashboard', 'admin_rides', 'admin_live', 'admin_help',
];

vi.mock('./api', () => ({
  api: vi.fn(),
  post: vi.fn(),
  subscribe: () => () => {},
  useApi: () => ({ data: null, loading: false, error: null, reload: vi.fn() }),
}));

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: {
      id: 1,
      name: audience === 'staff' ? 'Staff' : 'Grace Member',
      email: 'grace@example.com',
      roles: audience === 'staff' ? ['transport'] : ['member'],
      groups: [],
      status: 'active',
      terms_accepted: true,
      privileged: false,
      mfa_ok: true,
      unread: 0,
    },
    can: (page: string) =>
      audience === 'staff'
        ? staffGranted.includes(page)
        : ['events', 'rides', 'costs', 'reimbursements', 'help', 'report'].includes(page),
    refresh: vi.fn(),
  }),
}));

function mockMobileViewport(matches = true) {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value: (query: string) => ({
      matches: query.includes('max-width: 767px') ? matches : false,
      media: query || MOBILE_NAV_MQ,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
      dispatchEvent: () => false,
      onchange: null,
    }),
  });
}

function LocationProbe() {
  const loc = useLocation();
  return <div data-testid="location">{loc.pathname + loc.search}</div>;
}

function renderShell(initialEntries: string[] = ['/'], initialIndex?: number) {
  return render(
    <MemoryRouter initialEntries={initialEntries} initialIndex={initialIndex}>
      <BackNavigationProvider>
        <Routes>
          <Route element={<Layout />}>
            <Route
              path="/"
              element={(
                <div>
                  <LocationProbe />
                  <Link to="/events">Open events</Link>
                  <Link to="/events/12">Open event 12</Link>
                </div>
              )}
            />
            <Route
              path="/events"
              element={(
                <div>
                  <LocationProbe />
                  <PageHeader title="Events" />
                  <Link to="/events/12">Open event 12</Link>
                </div>
              )}
            />
            <Route path="/events/:id" element={<div><LocationProbe /><PageHeader title="Event detail" back="/events" /></div>} />
            <Route path="/me" element={<div><LocationProbe /><h1>Grace Member</h1></div>} />
            <Route path="/admin/menu" element={<div><LocationProbe /><h1 className="sr-only">Admin</h1></div>} />
            <Route path="/admin/users" element={<div><LocationProbe /><PageHeader title="Members" /></div>} />
            <Route path="/rides/:id" element={<div><LocationProbe /><PageHeader title="Ride detail" back="/rides" /></div>} />
          </Route>
        </Routes>
      </BackNavigationProvider>
    </MemoryRouter>,
  );
}

function swipe(target: Element, startX: number, startY: number, endX: number, endY: number) {
  fireEvent.touchStart(target, { touches: [{ clientX: startX, clientY: startY }] });
  fireEvent.touchMove(target, { touches: [{ clientX: endX, clientY: endY }] });
  fireEvent.touchEnd(target, { changedTouches: [{ clientX: endX, clientY: endY }] });
}

const MID = 140;
const Y = 220;

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

beforeEach(() => {
  audience = 'member';
  mockMobileViewport(true);
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 390 });
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('Mobile bottom-tab swipe', () => {
  it('lets a member swipe Home → Events → Profile and stops at the ends', () => {
    renderShell(['/']);
    const main = screen.getByRole('main');
    swipe(main, MID, Y, MID - 120, Y);
    expect(screen.getByTestId('location')).toHaveTextContent('/events');
    swipe(main, MID, Y, MID - 120, Y);
    expect(screen.getByTestId('location')).toHaveTextContent('/me');
    swipe(main, MID, Y, MID - 120, Y);
    expect(screen.getByTestId('location')).toHaveTextContent('/me');
    swipe(main, MID, Y, MID + 120, Y);
    expect(screen.getByTestId('location')).toHaveTextContent('/events');
    swipe(main, MID, Y, MID + 120, Y);
    expect(screen.getByTestId('location')).toHaveTextContent('/');
    swipe(main, MID, Y, MID + 120, Y);
    expect(screen.getByTestId('location')).toHaveTextContent('/');
  });

  it('includes the Admin tab for staff', () => {
    audience = 'staff';
    renderShell(['/events']);
    const main = screen.getByRole('main');
    swipe(main, MID, Y, MID - 120, Y);
    expect(screen.getByTestId('location')).toHaveTextContent('/admin/menu');
    swipe(main, MID, Y, MID - 120, Y);
    expect(screen.getByTestId('location')).toHaveTextContent('/me');
    swipe(main, MID, Y, MID + 120, Y);
    expect(screen.getByTestId('location')).toHaveTextContent('/admin/menu');
  });

  it('uses in-app history on a sub-page when that direction has an entry', () => {
    renderShell(['/']);
    fireEvent.click(screen.getByRole('link', { name: 'Open event 12' }));
    expect(screen.getByTestId('location')).toHaveTextContent('/events/12');
    swipe(screen.getByRole('main'), MID, Y, MID + 120, Y);
    expect(screen.getByTestId('location')).toHaveTextContent('/');
  });

  it('falls back to the neighbouring tab on a sub-page with no history in that direction', () => {
    renderShell(['/events/12']);
    const main = screen.getByRole('main');
    swipe(main, MID, Y, MID + 120, Y);
    expect(screen.getByTestId('location')).toHaveTextContent('/');
  });

  it('falls back to the next staff tab from an admin sub-page with no forward history', () => {
    audience = 'staff';
    renderShell(['/admin/users']);
    swipe(screen.getByRole('main'), MID, Y, MID - 120, Y);
    expect(screen.getByTestId('location')).toHaveTextContent('/me');
  });

  it('ignores mostly vertical moves, short moves, and starts near the screen edge', () => {
    renderShell(['/']);
    const main = screen.getByRole('main');
    swipe(main, MID, Y, MID - 20, Y + 140);
    expect(screen.getByTestId('location')).toHaveTextContent('/');
    swipe(main, MID, Y, MID - 40, Y);
    expect(screen.getByTestId('location')).toHaveTextContent('/');
    swipe(main, SWIPE_EDGE_IGNORE_PX - 4, Y, SWIPE_EDGE_IGNORE_PX - 4 - 120, Y);
    expect(screen.getByTestId('location')).toHaveTextContent('/');
  });

  it('does not swipe tabs on desktop widths', () => {
    mockMobileViewport(false);
    renderShell(['/']);
    swipe(screen.getByRole('main'), MID, Y, MID - 120, Y);
    expect(screen.getByTestId('location')).toHaveTextContent('/');
  });

  it('ignores swipes inside inputs, maps, horizontal scrollers, and open dialogs', () => {
    render(
      <MemoryRouter initialEntries={['/']}>
        <BackNavigationProvider>
          <Routes>
            <Route element={<Layout />}>
              <Route
                path="/"
                element={(
                  <div>
                    <LocationProbe />
                    <div className="leaflet-container" data-testid="map-box">Map</div>
                    <input aria-label="Notes input" />
                    <div data-testid="h-scroll" style={{ overflowX: 'auto' }}>Wide table</div>
                    <Link to="/events">Events</Link>
                  </div>
                )}
              />
              <Route path="/events" element={<div><LocationProbe />Events</div>} />
            </Route>
          </Routes>
        </BackNavigationProvider>
      </MemoryRouter>,
    );

    swipe(screen.getByTestId('map-box'), MID, Y, MID - 120, Y);
    expect(screen.getByTestId('location')).toHaveTextContent('/');

    swipe(screen.getByLabelText('Notes input'), MID, Y, MID - 120, Y);
    expect(screen.getByTestId('location')).toHaveTextContent('/');

    const hScroll = screen.getByTestId('h-scroll');
    Object.defineProperty(hScroll, 'scrollWidth', { value: 500, configurable: true });
    Object.defineProperty(hScroll, 'clientWidth', { value: 200, configurable: true });
    swipe(hScroll, MID, Y, MID - 120, Y);
    expect(screen.getByTestId('location')).toHaveTextContent('/');

    const modal = document.createElement('div');
    modal.setAttribute('role', 'dialog');
    modal.setAttribute('aria-modal', 'true');
    document.body.appendChild(modal);
    try {
      swipe(screen.getByRole('main'), MID, Y, MID - 120, Y);
      expect(screen.getByTestId('location')).toHaveTextContent('/');
    } finally {
      modal.remove();
    }
  });
});
