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
            <button type="button" className="btn primary" onClick={() => window.location.reload()}>{i18n.t('chrome:reload')}</button>
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
    <div className="offline-banner row nowrap" role="status">
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

  const handleRadioKeyDown = <T,>(
    e: React.KeyboardEvent<HTMLButtonElement>,
    items: { value: T }[],
    index: number,
    onSelect: (val: T) => void,
  ) => {
    let nextIndex = -1;
    if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
      nextIndex = (index + 1) % items.length;
    } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
      nextIndex = (index - 1 + items.length) % items.length;
    } else if (e.key === 'Home') {
      nextIndex = 0;
    } else if (e.key === 'End') {
      nextIndex = items.length - 1;
    }
    if (nextIndex >= 0) {
      e.preventDefault();
      onSelect(items[nextIndex].value);
      const buttons = e.currentTarget.parentElement?.querySelectorAll<HTMLButtonElement>('[role="radio"]');
      buttons?.[nextIndex]?.focus();
    }
  };

  return (
    <div className="stack">
      <p className="muted small">{t('displayHint')}</p>

      {/* Theme control */}
      <div className="stack-sm" role="radiogroup" aria-label={t('theme')}>
        <span className="bold small">{t('theme')}</span>
        <div className="segmented-control">
          {themes.map((item, idx) => (
            <button
              key={item.value}
              type="button"
              role="radio"
              tabIndex={p.theme === item.value ? 0 : -1}
              aria-checked={p.theme === item.value}
              className={`segmented-item ${p.theme === item.value ? 'active' : ''}`}
              onClick={() => update({ theme: item.value })}
              onKeyDown={(e) => handleRadioKeyDown(e, themes, idx, (val) => update({ theme: val }))}
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
          {sizes.map((item, idx) => (
            <button
              key={item.value}
              type="button"
              role="radio"
              tabIndex={p.textSize === item.value ? 0 : -1}
              aria-checked={p.textSize === item.value}
              className={`segmented-item ${p.textSize === item.value ? 'active' : ''}`}
              onClick={() => update({ textSize: item.value })}
              onKeyDown={(e) => handleRadioKeyDown(e, sizes, idx, (val) => update({ textSize: val }))}
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
          {languages.map((item, idx) => (
            <button
              key={item.value}
              type="button"
              role="radio"
              tabIndex={currentLang === item.value ? 0 : -1}
              aria-checked={currentLang === item.value}
              className={`segmented-item ${currentLang === item.value ? 'active' : ''}`}
              onClick={() => setLanguage(item.value)}
              onKeyDown={(e) => handleRadioKeyDown(e, languages, idx, setLanguage)}
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

/**
 * Compact 2-button bar for public/onboarding screens (Login, Invite, Visit):
 * 1-tap text-size cycle (Standard -> Large -> Extra large) + Language toggle.
 */
export function PublicDisplayBar() {
  const { t, i18n: i18nInstance } = useTranslation('chrome');
  const [p, setP] = useState<Prefs>(loadPrefs);
  const currentLang: SupportedLanguage = i18nInstance?.language?.startsWith('zh') ? 'zh' : 'en';

  const cycleTextSize = () => {
    const order: TextSize[] = ['normal', 'large', 'xlarge'];
    const nextSize = order[(order.indexOf(p.textSize) + 1) % order.length];
    const next = { ...p, textSize: nextSize };
    setP(next);
    savePrefs(next);
  };

  const sizeBadge = p.textSize === 'xlarge' ? 'Aa++' : p.textSize === 'large' ? 'Aa+' : 'Aa';
  const nextLangLabel = currentLang === 'zh' ? t('en') : t('zh');

  return (
    <div className="row nowrap public-display-bar">
      <button
        type="button"
        className="btn sm ghost text-size-toggle"
        aria-label={t('quickTextSize', { size: t(p.textSize) })}
        onClick={cycleTextSize}
      >
        <span aria-hidden="true">{sizeBadge}</span>
      </button>
      <button
        type="button"
        className="btn sm ghost lang-toggle"
        aria-label={`${nextLangLabel} (${t('switchLang')})`}
        onClick={() => setLanguage(currentLang === 'zh' ? 'en' : 'zh')}
      >
        <Globe size={16} aria-hidden="true" />
        <span>{nextLangLabel}</span>
      </button>
    </div>
  );
}
