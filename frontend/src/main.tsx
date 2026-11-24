import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import * as Sentry from '@sentry/react';
import App from './App';
import { AuthProvider } from './auth';
import { ConfirmProvider, ToastProvider } from './components/ui';
import { ErrorBoundary, OfflineBanner, SkipLink, UpdateBanner } from './components/AppChrome';
import { startUpdateWatch } from './appUpdate';
import { hideSplash } from './splash';
import './i18n';
import './prefs';
import './pwaInstall';
// Jost for the Grace Ride wordmark (Light "Ride", Medium splash captions, Bold "Grace").
// Self-hosted: the server CSP only allows fonts from 'self'.
import '@fontsource/jost/latin-300.css';
import '@fontsource/jost/latin-500.css';
import '@fontsource/jost/latin-700.css';
import './styles.css';

// Sentry: opt-in error monitoring. Set VITE_SENTRY_DSN in the production environment.
// Never send personal data (names, phone numbers, addresses) in Sentry events.
if (import.meta.env.VITE_SENTRY_DSN) {
  Sentry.init({
    dsn: import.meta.env.VITE_SENTRY_DSN,
    environment: import.meta.env.MODE,
    integrations: [],
    tracesSampleRate: 0.0, // Errors only; performance tracing is off.
    beforeSend(event) {
      // Strip any PII that may appear in breadcrumbs or extras.
      if (event.user) {
        delete event.user.email;
        delete event.user.username;
        delete event.user.ip_address;
      }
      return event;
    },
  });
}

// App normally hides the boot splash once auth resolves; never leave it covering a stuck or crashed load.
window.setTimeout(() => hideSplash(true), 10_000);

if (typeof window !== 'undefined') {
  const preloadKey = 'gr-preload-reload';
  window.addEventListener('vite:preloadError', (event) => {
    try {
      if (!sessionStorage.getItem(preloadKey)) {
        sessionStorage.setItem(preloadKey, '1');
        event.preventDefault();
        window.location.reload();
      }
    } catch {
      /* storage blocked */
    }
  });
  window.setTimeout(() => {
    try {
      sessionStorage.removeItem(preloadKey);
    } catch {
      /* storage blocked */
    }
  }, 15_000);
}

if (typeof window !== 'undefined' && import.meta.env.PROD) {
  window.addEventListener('load', () => {
    startUpdateWatch();
    // Ask the browser not to evict today's offline ride snapshot and queued trip actions
    // under storage pressure (granted silently for installed PWAs / engaged sites).
    void navigator.storage?.persist?.().catch(() => false);
  });
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ErrorBoundary>
      <BrowserRouter>
        <SkipLink />
        <OfflineBanner />
        <UpdateBanner />
        <AuthProvider>
          <ToastProvider>
            <ConfirmProvider>
              <App />
            </ConfirmProvider>
          </ToastProvider>
        </AuthProvider>
      </BrowserRouter>
    </ErrorBoundary>
  </React.StrictMode>,
);
