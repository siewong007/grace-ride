import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { DisplayPreferences } from './components/AppChrome';
import { Notice, PageHeader } from './components/ui';
import Layout from './components/Layout';
import * as i18nModule from './i18n';

vi.mock('./webauthn', () => ({
  passkeySupported: () => true,
  registerPasskey: vi.fn(),
}));

const mockMe = {
  id: 1,
  name: 'Grace Member',
  email: 'grace@example.com',
  phone: '0123456789',
  photo_file_id: null,
  roles: ['member'],
  groups: [{ id: 10, name: 'Youth' }],
  badges_opt_out: false,
  status: 'active',
  terms_accepted: true,
  privileged: false,
  mfa_ok: true,
  unread: 15,
};

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: mockMe,
    has: (role: string) => role === 'member',
    isStaff: false,
    refresh: vi.fn(),
  }),
}));

describe('DisplayPreferences', () => {
  beforeEach(() => {
    localStorage.clear();
    delete document.documentElement.dataset.theme;
    document.documentElement.dataset.text = 'normal';
  });

  it('renders theme, text size, and language segmented controls with live preview', () => {
    render(<DisplayPreferences />);

    expect(screen.getByRole('radiogroup', { name: /theme|appearance/i })).toBeInTheDocument();
    expect(screen.getByRole('radiogroup', { name: /text size/i })).toBeInTheDocument();
    expect(screen.getByRole('radiogroup', { name: /language/i })).toBeInTheDocument();

    expect(screen.getByText('Aa')).toBeInTheDocument();
  });

  it('updates text size on click and applies to html[data-text]', () => {
    render(<DisplayPreferences />);

    const largeBtn = screen.getByRole('radio', { name: /^large$/i });
    fireEvent.click(largeBtn);

    expect(document.documentElement.dataset.text).toBe('large');
  });

  it('updates theme on click and applies to html[data-theme]', () => {
    render(<DisplayPreferences />);

    const darkBtn = screen.getByRole('radio', { name: /dark/i });
    fireEvent.click(darkBtn);

    expect(document.documentElement.dataset.theme).toBe('dark');
  });

  it('calls setLanguage when switching language', () => {
    const setLanguageSpy = vi.spyOn(i18nModule, 'setLanguage');
    render(<DisplayPreferences />);

    const langBtn = screen.getByRole('radio', { name: 'English' });
    fireEvent.click(langBtn);

    expect(setLanguageSpy).toHaveBeenCalled();
  });
});

describe('Notice primitive', () => {
  it('renders warning and info tones with icon and content', () => {
    const { rerender } = render(
      <Notice tone="warning" icon={<span data-testid="test-icon">!</span>}>
        Pending approval notice
      </Notice>
    );

    const banner = screen.getByRole('status');
    expect(banner).toHaveClass('notice', 'warning');
    expect(screen.getByTestId('test-icon')).toBeInTheDocument();
    expect(screen.getByText('Pending approval notice')).toBeInTheDocument();

    rerender(
      <Notice tone="info">
        Admin notice
      </Notice>
    );
    expect(screen.getByRole('status')).toHaveClass('notice', 'info');
    expect(screen.getByText('Admin notice')).toBeInTheDocument();
  });
});

describe('Layout and Navigation', () => {
  it('caps unread notification badge display at 9+', () => {
    render(
      <MemoryRouter initialEntries={['/']}>
        <Routes>
          <Route element={<Layout />}>
            <Route path="/" element={<div>Home content</div>} />
          </Route>
        </Routes>
      </MemoryRouter>
    );

    // mockMe.unread is 15, should render "9+"
    expect(screen.getByText('9+')).toBeInTheDocument();
  });

  it('shows the page title in the top bar without adding a second h1', () => {
    render(
      <MemoryRouter initialEntries={['/rides']}>
        <Routes>
          <Route element={<Layout />}>
            <Route path="/rides" element={<div>Rides content</div>} />
          </Route>
        </Routes>
      </MemoryRouter>
    );

    // The page owns the h1; the top bar title is plain text.
    expect(screen.getByText('My rides', { selector: '.topbar-title' })).toBeInTheDocument();
    expect(screen.queryByRole('heading', { level: 1 })).not.toBeInTheDocument();
  });

  it('moves a page back link into the top bar', () => {
    render(
      <MemoryRouter initialEntries={['/rides/new']}>
        <Routes>
          <Route element={<Layout />}>
            <Route path="/rides/new" element={<PageHeader title="Request a ride" back="/rides" />} />
          </Route>
        </Routes>
      </MemoryRouter>
    );

    const backLinks = screen.getAllByRole('link', { name: 'Back' });
    expect(backLinks).toHaveLength(1);
    expect(backLinks[0]).toHaveClass('topbar-back');
    expect(backLinks[0]).toHaveAttribute('href', '/rides');
    expect(screen.getAllByRole('heading', { level: 1 })).toHaveLength(1);
  });
});
