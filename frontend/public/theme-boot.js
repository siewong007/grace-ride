// Runs before first paint (classic <script> in index.html <head>, allowed by the CSP's script-src 'self').
// Applies the theme and language saved in Settings so the boot splash, logo, and <html lang> match from frame 1:
// dark-theme users get the dark splash and white logo, light-theme users the light splash and colour logo,
// and "system" follows the device. Mirrors applyPrefs() in src/prefs.ts ('gr-prefs') and src/i18n.ts ('gr-lang').
(function () {
  var theme = 'system';
  var lang = null;
  try {
    var p = JSON.parse(localStorage.getItem('gr-prefs') || '{}');
    if (p && (p.theme === 'light' || p.theme === 'dark')) theme = p.theme;
    var savedLang = localStorage.getItem('gr-lang');
    if (savedLang === 'en' || savedLang === 'zh') lang = savedLang;
  } catch { /* storage blocked: follow the device */ }
  var root = document.documentElement;
  if (lang) root.lang = lang === 'zh' ? 'zh-CN' : 'en';
  if (theme !== 'system') root.setAttribute('data-theme', theme);
  var dark = theme === 'dark' || (theme === 'system' && !!window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches);
  var meta = document.querySelector('meta[name="theme-color"]');
  if (meta) meta.setAttribute('content', dark ? '#141517' : '#FFFFFF');
})();
