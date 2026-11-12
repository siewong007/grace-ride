import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import App from './App';
import { AuthProvider } from './auth';
import { ConfirmProvider, ToastProvider } from './components/ui';
import { ErrorBoundary, OfflineBanner, SkipLink } from './components/AppChrome';
import { hideSplash } from './splash';
import './i18n';
import './prefs';
// Jost for the Grace Ride wordmark (Light "Ride", Medium splash captions, Bold "Grace").
// Self-hosted: the server CSP only allows fonts from 'self'.
import '@fontsource/jost/latin-300.css';
import '@fontsource/jost/latin-500.css';
import '@fontsource/jost/latin-700.css';
import './styles.css';

// App normally hides the boot splash once auth resolves; never leave it covering a stuck or crashed load.
window.setTimeout(hideSplash, 10_000);

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ErrorBoundary>
      <BrowserRouter>
        <SkipLink />
        <OfflineBanner />
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
