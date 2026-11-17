import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { Link, MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import i18n from './i18n';
import Layout from './components/Layout';
import { BackNavigationProvider } from './components/BackNavigation';
import { PageHeader } from './components/ui';

let mockStatus: 'active' | 'pending' = 'active';

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
      name: 'Grace Member',
      email: 'grace@example.com',
      phone: '0123456789',
      photo_file_id: null,
      roles: ['member', 'admin'],
      groups: [],
      badges_opt_out: false,
      status: mockStatus,
      terms_accepted: true,
      privileged: false,
      mfa_ok: true,
      unread: 0,
    },
    can: () => true,
    refresh: vi.fn(),
  }),
}));

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

beforeEach(() => {
  mockStatus = 'active';
});

function LocationProbe() {
  const loc = useLocation();
  return <div data-testid="location">{loc.pathname + loc.search}</div>;
}

function renderAppShell(initialEntries: string[] = ['/'], initialIndex?: number) {
  return render(
    <MemoryRouter initialEntries={initialEntries} initialIndex={initialIndex}>
      <BackNavigationProvider>
        <Routes>
          <Route path="/login" element={<div data-testid="login-page">Login page</div>} />
          <Route element={<Layout />}>
            <Route
              path="/"
              element={
                <div>
                  <LocationProbe />
                  <Link to="/drive">Go drive</Link>
                  <Link to="/events">Go events</Link>
                  <Link to="/rides">Go rides</Link>
                </div>
              }
            />
            <Route path="/rides" element={<div><LocationProbe /><PageHeader title="My rides" /></div>} />
            <Route path="/rides/:id" element={<div><LocationProbe /><PageHeader title="Ride detail" back="/rides" /></div>} />
            <Route
              path="/drive"
              element={
                <div>
                  <LocationProbe />
                  <PageHeader title="Driving" />
                  <Link to="/rides/101">Open ride 101</Link>
                </div>
              }
            />
            <Route
              path="/events"
              element={
                <div>
                  <LocationProbe />
                  <PageHeader title="Events" />
                  <Link to="/notifications">Open notifications</Link>
                </div>
              }
            />
            <Route path="/notifications" element={<div><LocationProbe /><PageHeader title="Notifications" /></div>} />
            <Route path="/settlements" element={<div><LocationProbe /><PageHeader title="Cost splits" /></div>} />
            <Route path="/me" element={<div><LocationProbe /><h1>Grace Member</h1></div>} />
            <Route path="/admin/menu" element={<div><LocationProbe /><h1 className="sr-only">Admin</h1></div>} />
          </Route>
        </Routes>
      </BackNavigationProvider>
    </MemoryRouter>,
  );
}

function swipe(target: Element, startX: number, startY: number, endX: number, endY: number) {
  fireEvent.touchStart(target, {
    touches: [{ clientX: startX, clientY: startY }],
  });
  fireEvent.touchMove(target, {
    touches: [{ clientX: endX, clientY: endY }],
  });
  fireEvent.touchEnd(target, {
    changedTouches: [{ clientX: endX, clientY: endY }],
  });
}

describe('General Back Button & Smart History', () => {
  it('hides the back button on Home and on /me for pending members', () => {
    const { unmount } = renderAppShell(['/']);
    expect(screen.queryByRole('link', { name: 'Back' })).not.toBeInTheDocument();
    unmount();

    mockStatus = 'pending';
    renderAppShell(['/me']);
    expect(screen.queryByRole('link', { name: 'Back' })).not.toBeInTheDocument();
  });

  it('shows the general topbar back button on all non-Home pages across viewports without hide-tablet-up', () => {
    const paths = ['/rides', '/drive', '/events', '/notifications', '/settlements', '/me', '/admin/menu'];
    for (const path of paths) {
      const { unmount } = renderAppShell([path]);
      const backLinks = screen.getAllByRole('link', { name: 'Back' });
      expect(backLinks).toHaveLength(1);
      expect(backLinks[0]).toHaveClass('topbar-back');
      expect(backLinks[0]).not.toHaveClass('hide-tablet-up');
      expect(backLinks[0]).toHaveAttribute('href', '/');
      unmount();
    }
  });

  it('falls back to the parent route on direct entry (/rides/101 -> /rides)', () => {
    renderAppShell(['/rides/101']);
    const backLink = screen.getByRole('link', { name: 'Back' });
    expect(backLink).toHaveAttribute('href', '/rides');
    fireEvent.click(backLink);
    expect(screen.getByTestId('location')).toHaveTextContent('/rides');
  });

  it('navigates back to the previous in-app page when history exists (/ -> /drive -> /rides/101)', () => {
    renderAppShell(['/']);
    fireEvent.click(screen.getByRole('link', { name: 'Go drive' }));
    expect(screen.getByTestId('location')).toHaveTextContent('/drive');

    fireEvent.click(screen.getByRole('link', { name: 'Open ride 101' }));
    expect(screen.getByTestId('location')).toHaveTextContent('/rides/101');

    const backLink = screen.getByRole('link', { name: 'Back' });
    expect(backLink).toHaveAttribute('href', '/drive');
    fireEvent.click(backLink);
    expect(screen.getByTestId('location')).toHaveTextContent('/drive');
  });

  it('returns from /notifications to the previous in-app page (/events) via history', () => {
    renderAppShell(['/']);
    fireEvent.click(screen.getByRole('link', { name: 'Go events' }));
    fireEvent.click(screen.getByRole('link', { name: 'Open notifications' }));
    expect(screen.getByTestId('location')).toHaveTextContent('/notifications');

    const backLink = screen.getByRole('link', { name: 'Back' });
    expect(backLink).toHaveAttribute('href', '/events');
    fireEvent.click(backLink);
    expect(screen.getByTestId('location')).toHaveTextContent('/events');
  });

  it('never pops an authenticated user back to /login', () => {
    render(
      <MemoryRouter initialEntries={['/login']}>
        <BackNavigationProvider>
          <Routes>
            <Route
              path="/login"
              element={
                <div>
                  <Link to="/rides">Enter app</Link>
                </div>
              }
            />
            <Route element={<Layout />}>
              <Route path="/rides" element={<div><LocationProbe /><PageHeader title="My rides" /></div>} />
              <Route path="/" element={<div data-testid="home-landed"><LocationProbe />Home</div>} />
            </Route>
          </Routes>
        </BackNavigationProvider>
      </MemoryRouter>,
    );

    fireEvent.click(screen.getByRole('link', { name: 'Enter app' }));
    expect(screen.getByTestId('location')).toHaveTextContent('/rides');

    const backLink = screen.getByRole('link', { name: 'Back' });
    expect(backLink).toHaveAttribute('href', '/');
    fireEvent.click(backLink);
    expect(screen.getByTestId('home-landed')).toBeInTheDocument();
  });
});

describe('Swipe-Right-to-Go-Back Gesture', () => {
  it('shows the slide-in indicator on left-edge swipe and navigates back past threshold', () => {
    renderAppShell(['/rides/101']);
    const main = screen.getByRole('main');
    const indicator = screen.getByTestId('swipe-back-indicator');

    fireEvent.touchStart(main, { touches: [{ clientX: 16, clientY: 200 }] });
    fireEvent.touchMove(main, { touches: [{ clientX: 110, clientY: 202 }] });
    expect(indicator).toHaveClass('is-active', 'is-ready');

    fireEvent.touchEnd(main, { changedTouches: [{ clientX: 110, clientY: 202 }] });
    expect(screen.getByTestId('location')).toHaveTextContent('/rides');
  });

  it('triggers back navigation on horizontal right-drag in non-scrollable page content', () => {
    renderAppShell(['/']);
    fireEvent.click(screen.getByRole('link', { name: 'Go drive' }));
    expect(screen.getByTestId('location')).toHaveTextContent('/drive');

    const main = screen.getByRole('main');
    swipe(main, 140, 220, 260, 225);
    expect(screen.getByTestId('location')).toHaveTextContent('/');
  });

  it('cancels without navigating when drag distance is below threshold', () => {
    renderAppShell(['/rides/101']);
    const main = screen.getByRole('main');
    const indicator = screen.getByTestId('swipe-back-indicator');

    fireEvent.touchStart(main, { touches: [{ clientX: 16, clientY: 200 }] });
    fireEvent.touchMove(main, { touches: [{ clientX: 45, clientY: 200 }] });
    expect(indicator).toHaveClass('is-active');
    expect(indicator).not.toHaveClass('is-ready');

    fireEvent.touchEnd(main, { changedTouches: [{ clientX: 45, clientY: 200 }] });
    expect(indicator).not.toHaveClass('is-active');
    expect(screen.getByTestId('location')).toHaveTextContent('/rides/101');
  });

  it('does not trigger swipe-back on vertical scrolling', () => {
    renderAppShell(['/rides/101']);
    const main = screen.getByRole('main');
    swipe(main, 120, 200, 150, 320);
    expect(screen.getByTestId('location')).toHaveTextContent('/rides/101');
  });

  it('ignores swipes inside maps, form inputs, scrolled horizontal containers, and open modals', () => {
    render(
      <MemoryRouter initialEntries={['/rides/101']}>
        <BackNavigationProvider>
          <Routes>
            <Route element={<Layout />}>
              <Route
                path="/rides/:id"
                element={
                  <div>
                    <LocationProbe />
                    <div className="leaflet-container" data-testid="map-box">Map</div>
                    <input aria-label="Notes input" />
                    <div
                      data-testid="h-scroll"
                      style={{ overflowX: 'auto' }}
                    >
                      Wide table
                    </div>
                  </div>
                }
              />
              <Route path="/rides" element={<div><LocationProbe />Rides</div>} />
            </Route>
          </Routes>
        </BackNavigationProvider>
      </MemoryRouter>,
    );

    // 1. Inside .leaflet-container
    swipe(screen.getByTestId('map-box'), 16, 200, 140, 200);
    expect(screen.getByTestId('location')).toHaveTextContent('/rides/101');

    // 2. Inside input (non-edge start)
    swipe(screen.getByLabelText('Notes input'), 140, 200, 260, 200);
    expect(screen.getByTestId('location')).toHaveTextContent('/rides/101');

    // 3. Inside horizontally scrolled container (scrollLeft > 0)
    const hScroll = screen.getByTestId('h-scroll');
    Object.defineProperty(hScroll, 'scrollWidth', { value: 500, configurable: true });
    Object.defineProperty(hScroll, 'clientWidth', { value: 200, configurable: true });
    hScroll.scrollLeft = 40;
    swipe(hScroll, 140, 200, 260, 200);
    expect(screen.getByTestId('location')).toHaveTextContent('/rides/101');

    // 4. While a modal dialog is open
    const modal = document.createElement('div');
    modal.setAttribute('role', 'dialog');
    modal.setAttribute('aria-modal', 'true');
    document.body.appendChild(modal);
    try {
      swipe(screen.getByRole('main'), 16, 200, 140, 200);
      expect(screen.getByTestId('location')).toHaveTextContent('/rides/101');
    } finally {
      modal.remove();
    }
  });
});
