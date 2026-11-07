// App-wide UX safety nets: error boundary, offline banner, skip link, display preferences.
// Strings are bilingual inline so this file stays independent of the locale bundles.
import { Component, ErrorInfo, ReactNode, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { loadPrefs, Prefs, savePrefs, TextSize, Theme } from '../prefs';

type Lang = 'zh' | 'en';
const STR = {
  errTitle: { zh: '页面出了点问题', en: 'Something went wrong' },
  errBody: { zh: '请重新载入。如果问题持续，请到帮助中心告诉我们。', en: 'Please reload. If it keeps happening, tell us in the Help Center.' },
  reload: { zh: '重新载入', en: 'Reload' },
  home: { zh: '回到首页', en: 'Go home' },
  offline: { zh: '目前没有网络连接，显示的资料可能不是最新的。', en: 'You are offline. Information shown may be out of date.' },
  skip: { zh: '跳到主要内容', en: 'Skip to main content' },
  display: { zh: '显示设置', en: 'Display' },
  displayHint: { zh: '只保存在这部设备上。', en: 'Saved on this device only.' },
  textSize: { zh: '字体大小', en: 'Text size' },
  normal: { zh: '标准', en: 'Standard' },
  large: { zh: '大字', en: 'Large' },
  xlarge: { zh: '特大', en: 'Extra large' },
  theme: { zh: '外观', en: 'Appearance' },
  system: { zh: '跟随系统', en: 'System' },
  light: { zh: '浅色', en: 'Light' },
  dark: { zh: '深色', en: 'Dark' },
  preview: { zh: '预览：周二 上午 8:30 · 北门入口', en: 'Preview: Tue 8:30 AM · North gate' },
} satisfies Record<string, Record<Lang, string>>;

function useStr() {
  const { i18n } = useTranslation();
  const lang: Lang = i18n.language?.startsWith('zh') ? 'zh' : 'en';
  return (k: keyof typeof STR) => STR[k][lang];
}

function langNow(): Lang {
  return document.documentElement.lang.startsWith('zh') ? 'zh' : 'en';
}

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
    const l = langNow();
    return (
      <div className="auth-page" role="alert">
        <div className="card auth-card stack">
          <img src="/icon.svg" width={48} height={48} alt="" />
          <h2>{STR.errTitle[l]}</h2>
          <p className="muted">{STR.errBody[l]}</p>
          <div className="btn-row">
            <button className="btn primary" onClick={() => window.location.reload()}>{STR.reload[l]}</button>
            <a className="btn" href="/">{STR.home[l]}</a>
          </div>
        </div>
      </div>
    );
  }
}

export function OfflineBanner() {
  const s = useStr();
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
  return <div className="offline-banner" role="status">📡 {s('offline')}</div>;
}

/** Keyboard / screen-reader users jump past the navigation straight to the page content. */
export function SkipLink() {
  const s = useStr();
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
      {s('skip')}
    </a>
  );
}

/** 大字模式 and theme — the spec asks for clear buttons and large text for all ages. */
export function DisplayPreferences() {
  const s = useStr();
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
        <h3>{s('display')}</h3>
        <p className="muted small">{s('displayHint')}</p>
      </div>
      <div className="stack-sm" role="radiogroup" aria-label={s('textSize')}>
        <strong>{s('textSize')}</strong>
        <div className="chips">
          {sizes.map((v) => (
            <button key={v} type="button" role="radio" aria-checked={p.textSize === v} className={`chip ${p.textSize === v ? 'on' : ''}`} onClick={() => update({ textSize: v })}>
              {s(v)}
            </button>
          ))}
        </div>
      </div>
      <div className="stack-sm" role="radiogroup" aria-label={s('theme')}>
        <strong>{s('theme')}</strong>
        <div className="chips">
          {themes.map((v) => (
            <button key={v} type="button" role="radio" aria-checked={p.theme === v} className={`chip ${p.theme === v ? 'on' : ''}`} onClick={() => update({ theme: v })}>
              {s(v)}
            </button>
          ))}
        </div>
      </div>
      <div className="card flat tight">{s('preview')}</div>
    </div>
  );
}
