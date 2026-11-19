import './i18n';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import NotFound from './pages/NotFound';
import Invite from './pages/Invite';
import { TermsGate, TermsText } from './components/Terms';
import { ConfirmProvider } from './components/ui';
import { ApiError } from './api';

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

let inviteData = { kind: 'member', email: '', name: '' };
const post = vi.fn();
vi.mock('./api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./api')>()),
  useApi: () => ({
    data: inviteData,
    loading: false,
    error: null,
  }),
  post: (...args: unknown[]) => post(...args),
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

describe('Invite passkey step', () => {
  it('asks for confirmation before skipping the passkey', async () => {
    render(
      <ConfirmProvider>
        <MemoryRouter initialEntries={['/invite/test-token']}>
          <Routes>
            <Route path="/invite/:token" element={<Invite />} />
            <Route path="/" element={<div>home page</div>} />
          </Routes>
        </MemoryRouter>
      </ConfirmProvider>
    );
    fireEvent.change(screen.getByLabelText(/name/i), { target: { value: 'New Member' } });
    fireEvent.change(screen.getByLabelText(/email/i), { target: { value: 'new@example.com' } });
    fireEvent.change(screen.getByLabelText(/phone/i), { target: { value: '0123456789' } });
    fireEvent.click(screen.getByRole('button', { name: /continue/i }));
    fireEvent.click(screen.getByRole('checkbox'));
    fireEvent.click(screen.getByRole('button', { name: /agree/i }));
    await screen.findByText(/step 3 of 3/i);

    fireEvent.click(screen.getByRole('button', { name: /later/i }));
    const dialog = await screen.findByRole('dialog');
    expect(dialog).toHaveTextContent(/no password/i);

    // Choosing to set it up keeps the member on the passkey step.
    fireEvent.click(screen.getByRole('button', { name: /^set up passkey$/i }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(screen.getByText(/step 3 of 3/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /later/i }));
    fireEvent.click(await screen.findByRole('button', { name: /skip for now/i }));
    expect(await screen.findByText('home page')).toBeInTheDocument();
  });
});

const renderInvite = () =>
  render(
    <ConfirmProvider>
      <MemoryRouter initialEntries={['/invite/test-token']}>
        <Routes>
          <Route path="/invite/:token" element={<Invite />} />
        </Routes>
      </MemoryRouter>
    </ConfirmProvider>
  );

const fillDetails = (phone = '0123456789') => {
  fireEvent.change(screen.getByLabelText(/name/i), { target: { value: 'New Member' } });
  fireEvent.change(screen.getByLabelText(/email/i), { target: { value: 'new@example.com' } });
  fireEvent.change(screen.getByLabelText(/phone/i), { target: { value: phone } });
  fireEvent.click(screen.getByRole('button', { name: /continue/i }));
};

describe('Invite details match the server rules', () => {
  beforeEach(() => {
    inviteData = { kind: 'member', email: '', name: '' };
    post.mockReset();
  });

  it('rejects a phone number the server would refuse, before the terms step', () => {
    renderInvite();
    fillDetails('01234567');
    expect(screen.getByText(/step 1 of 3/i)).toBeInTheDocument();
    expect(screen.getByText(/full phone number/i)).toBeInTheDocument();
  });

  it('sends a taken email back to the email field with a sign-in link', async () => {
    post.mockRejectedValueOnce(new ApiError(400, 'This email is already registered', 'EMAIL_ALREADY_REGISTERED'));
    renderInvite();
    fillDetails();
    fireEvent.click(screen.getByRole('checkbox'));
    fireEvent.click(screen.getByRole('button', { name: /agree/i }));
    expect(await screen.findByText(/step 1 of 3/i)).toBeInTheDocument();
    expect(screen.getByText('This email is already registered')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /sign in instead/i })).toHaveAttribute('href', '/login');
    // Fixing the email clears the hint.
    fireEvent.change(screen.getByLabelText(/email/i), { target: { value: 'other@example.com' } });
    expect(screen.queryByRole('link', { name: /sign in instead/i })).toBeNull();
  });

  it('prefills the name and email an admin put on the invite', () => {
    inviteData = { kind: 'member', email: 'named@example.com', name: 'Named Person' };
    renderInvite();
    expect(screen.getByLabelText(/name/i)).toHaveValue('Named Person');
    expect(screen.getByLabelText(/email/i)).toHaveValue('named@example.com');
  });

  it('runs account recovery in two steps, starting at the terms', () => {
    inviteData = { kind: 'recovery', email: 'back@example.com', name: 'Back Again' };
    renderInvite();
    expect(screen.getByText(/step 1 of 2/i)).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: /recover account/i })).toBeInTheDocument();
    expect(screen.getByText('back@example.com')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /^back$/i })).toBeNull();
  });
});
