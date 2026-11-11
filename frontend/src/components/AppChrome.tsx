// App-wide UX safety nets: error boundary, offline banner, skip link, display preferences.
import { Component, ErrorInfo, ReactNode, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import i18n from '../i18n';
import { loadPrefs, Prefs, savePrefs, TextSize, Theme } from '../prefs';

export class ErrorBoundary extends Component<{ children: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('UI error', error, info.componentStack);
  }
  render() {
    if (!this.state.failed) return this.props.children;
    return (
      <div className="auth-page" role="alert">
        <div className="card auth-card stack">
          <img src="/icon.svg" width={48} height={48} alt="" />
          <h2>{i18n.t('chrome:errTitle')}</h2>
          <p className="muted">{i18n.t('chrome:errBody')}</p>
          <div className="btn-row">
            <button className="btn primary" onClick={() => window.location.reload()}>{i18n.t('chrome:reload')}</button>
            <a className="btn" href="/">{i18n.t('chrome:home')}</a>
          </div>
        </div>
      </div>
    );
  }
}

export function OfflineBanner() {
  const { t } = useTranslation('chrome');
  const [online, setOnline] = useState(typeof navigator === 'undefined' ? true : navigator.onLine);
  useEffect(() => {
    const on = () => setOnline(true);
    const off = () => setOnline(false);
    window.addEventListener('online', on);
    window.addEventListener('offline', off);
    return () => {
      window.removeEventListener('online', on);
      window.removeEventListener('offline', off);
    };
  }, []);
  if (online) return null;
  return <div className="offline-banner" role="status">📡 {t('offline')}</div>;
}

/** Keyboard / screen-reader users jump past the navigation straight to the page content. */
export function SkipLink() {
  const { t } = useTranslation('chrome');
  return (
    <a
      href="#main"
      className="skip-link"
      onClick={(e) => {
        const main = document.querySelector('main');
        if (main) {
          e.preventDefault();
          main.setAttribute('tabindex', '-1');
          main.focus();
        }
      }}
    >
      {t('skip')}
    </a>
  );
}

/** Large text mode and theme — the spec asks for clear buttons and large text for all ages. */
export function DisplayPreferences() {
  const { t } = useTranslation('chrome');
  const [p, setP] = useState<Prefs>(loadPrefs);
  const update = (next: Partial<Prefs>) => {
    const merged = { ...p, ...next };
    setP(merged);
    savePrefs(merged);
  };
  const sizes: TextSize[] = ['normal', 'large', 'xlarge'];
  const themes: Theme[] = ['system', 'light', 'dark'];
  return (
    <div className="card stack">
      <div>
        <h3>{t('display')}</h3>
        <p className="muted small">{t('displayHint')}</p>
      </div>
      <div className="stack-sm" role="radiogroup" aria-label={t('textSize')}>
        <strong>{t('textSize')}</strong>
        <div className="chips">
          {sizes.map((v) => (
            <button key={v} type="button" role="radio" aria-checked={p.textSize === v} className={`chip ${p.textSize === v ? 'on' : ''}`} onClick={() => update({ textSize: v })}>
              {t(v)}
            </button>
          ))}
        </div>
      </div>
      <div className="stack-sm" role="radiogroup" aria-label={t('theme')}>
        <strong>{t('theme')}</strong>
        <div className="chips">
          {themes.map((v) => (
            <button key={v} type="button" role="radio" aria-checked={p.theme === v} className={`chip ${p.theme === v ? 'on' : ''}`} onClick={() => update({ theme: v })}>
              {t(v)}
            </button>
          ))}
        </div>
      </div>
      <div className="card flat tight">{t('preview')}</div>
    </div>
  );
}
