// App-wide UX safety nets: error boundary, offline banner, skip link, display preferences.
import { Component, ErrorInfo, ReactNode, useEffect, useState } from 'react';
import { Globe, Laptop, Moon, Radio, Sun } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import i18n, { setLanguage, SupportedLanguage } from '../i18n';
import { loadPrefs, Prefs, savePrefs, TextSize, Theme } from '../prefs';
import { hideSplash } from '../splash';
import { LogoMark } from './Brand';

export class ErrorBoundary extends Component<{ children: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('UI error', error, info.componentStack);
    hideSplash();
  }
  render() {
    if (!this.state.failed) return this.props.children;
    return (
      <div className="auth-page" role="alert">
        <div className="card auth-card stack">
          <LogoMark size={48} />
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
  return (
    <div className="offline-banner row nowrap" style={{ justifyContent: 'center', gap: 'var(--space-2)' }} role="status">
      <Radio size={16} aria-hidden="true" />
      <span>{t('offline')}</span>
    </div>
  );
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

/**
 * Display preferences: Theme, Text size, and Language controls.
 * Uses accessible segmented controls with a live "Aa" preview.
 */
export function DisplayPreferences() {
  const { t, i18n: i18nInstance } = useTranslation('chrome');
  const [p, setP] = useState<Prefs>(loadPrefs);
  const currentLang: SupportedLanguage = i18nInstance.language?.startsWith('zh') ? 'zh' : 'en';

  const update = (next: Partial<Prefs>) => {
    const merged = { ...p, ...next };
    setP(merged);
    savePrefs(merged);
  };

  const sizes: { value: TextSize; label: string }[] = [
    { value: 'normal', label: t('normal') },
    { value: 'large', label: t('large') },
    { value: 'xlarge', label: t('xlarge') },
  ];

  const themes: { value: Theme; label: string; icon: ReactNode }[] = [
    { value: 'system', label: t('system'), icon: <Laptop size={16} aria-hidden="true" /> },
    { value: 'light', label: t('light'), icon: <Sun size={16} aria-hidden="true" /> },
    { value: 'dark', label: t('dark'), icon: <Moon size={16} aria-hidden="true" /> },
  ];

  const languages: { value: SupportedLanguage; label: string }[] = [
    { value: 'zh', label: t('zh') },
    { value: 'en', label: t('en') },
  ];

  return (
    <div className="stack">
      <p className="muted small">{t('displayHint')}</p>

      {/* Theme control */}
      <div className="stack-sm" role="radiogroup" aria-label={t('theme')}>
        <span className="bold small">{t('theme')}</span>
        <div className="segmented-control">
          {themes.map((item) => (
            <button
              key={item.value}
              type="button"
              role="radio"
              aria-checked={p.theme === item.value}
              className={`segmented-item ${p.theme === item.value ? 'active' : ''}`}
              onClick={() => update({ theme: item.value })}
            >
              {item.icon}
              <span>{item.label}</span>
            </button>
          ))}
        </div>
      </div>

      {/* Text size control */}
      <div className="stack-sm" role="radiogroup" aria-label={t('textSize')}>
        <span className="bold small">{t('textSize')}</span>
        <div className="segmented-control">
          {sizes.map((item) => (
            <button
              key={item.value}
              type="button"
              role="radio"
              aria-checked={p.textSize === item.value}
              className={`segmented-item ${p.textSize === item.value ? 'active' : ''}`}
              onClick={() => update({ textSize: item.value })}
            >
              <span>{item.label}</span>
            </button>
          ))}
        </div>
      </div>

      {/* Live Aa preview */}
      <div className="text-preview card flat tight row nowrap">
        <span className="preview-aa" aria-hidden="true">Aa</span>
        <span className="preview-text">{t('preview')}</span>
      </div>

      {/* Language control */}
      <div className="stack-sm" role="radiogroup" aria-label={t('language')}>
        <span className="bold small">{t('language')}</span>
        <div className="segmented-control">
          {languages.map((item) => (
            <button
              key={item.value}
              type="button"
              role="radio"
              aria-checked={currentLang === item.value}
              className={`segmented-item ${currentLang === item.value ? 'active' : ''}`}
              onClick={() => setLanguage(item.value)}
            >
              <Globe size={16} aria-hidden="true" />
              <span>{item.label}</span>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
