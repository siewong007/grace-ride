import './i18n';
import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import NotFound from './pages/NotFound';
import Invite from './pages/Invite';
import { TermsGate, TermsText } from './components/Terms';

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: { id: 1, name: 'Grace User', email: 'user@example.com' },
    refresh: vi.fn(),
  }),
}));

vi.mock('./webauthn', () => ({
  passkeySupported: () => true,
  registerPasskey: vi.fn(),
}));

vi.mock('./api', () => ({
  useApi: () => ({
    data: { kind: 'member', email: '', name: '' },
    loading: false,
    error: null,
  }),
  post: vi.fn(),
}));

describe('NotFound page', () => {
  it('renders MapPinOff illustration, title, Go home and Back buttons', () => {
    render(
      <MemoryRouter>
        <NotFound />
      </MemoryRouter>
    );

    // Header title
    expect(screen.getByRole('heading', { level: 1 })).toBeInTheDocument();

    // Two buttons: Go home and Back
    const homeLink = screen.getByRole('link', { name: /home/i });
    expect(homeLink).toHaveAttribute('href', '/');

    const backButton = screen.getByRole('button', { name: /back/i });
    expect(backButton).toBeInTheDocument();
  });
});

describe('Terms and TermsGate', () => {
  it('renders readable terms text with section headings', () => {
    render(
      <MemoryRouter>
        <TermsText />
      </MemoryRouter>
    );

    expect(screen.getByRole('heading', { level: 2 })).toBeInTheDocument();
    const headings = screen.getAllByRole('heading', { level: 3 });
    expect(headings.length).toBeGreaterThanOrEqual(5);
  });

  it('renders sticky accept bar with checkbox gating agree button', () => {
    render(
      <MemoryRouter>
        <TermsGate />
      </MemoryRouter>
    );

    const checkbox = screen.getByRole('checkbox');
    const agreeBtn = screen.getByRole('button', { name: /agree/i });

    // Initially disabled
    expect(checkbox).not.toBeChecked();
    expect(agreeBtn).toBeDisabled();

    // Ticking checkbox enables agree button
    fireEvent.click(checkbox);
    expect(checkbox).toBeChecked();
    expect(agreeBtn).not.toBeDisabled();
  });
});

describe('Invite 3-step stepper', () => {
  it('renders Step 1 with stepper header and progress bar', () => {
    render(
      <MemoryRouter initialEntries={['/invite/test-token']}>
        <Routes>
          <Route path="/invite/:token" element={<Invite />} />
        </Routes>
      </MemoryRouter>
    );

    // Stepper counter
    expect(screen.getByText(/step 1 of 3/i)).toBeInTheDocument();
    // Progress bar with step-1
    const progressBar = screen.getByRole('progressbar');
    expect(progressBar).toBeInTheDocument();
    expect(progressBar.querySelector('.step-1')).toBeInTheDocument();

    // Field validation when submitting empty form
    const continueBtn = screen.getByRole('button', { name: /continue/i });
    fireEvent.click(continueBtn);

    // Should remain on step 1 due to required field validations
    expect(screen.getByText(/step 1 of 3/i)).toBeInTheDocument();
  });
});
