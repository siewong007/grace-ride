// Runs before first paint (classic <script> in index.html <head>, allowed by the CSP's script-src 'self').
// Applies the theme saved in Settings so the boot splash and logo match it from the very first frame:
// dark-theme users get the dark splash and white logo, light-theme users the light splash and colour logo,
// and "system" follows the device. Mirrors applyPrefs() in src/prefs.ts (same 'gr-prefs' key).
(function () {
  var theme = 'system';
  try {
    var p = JSON.parse(localStorage.getItem('gr-prefs') || '{}');
    if (p && (p.theme === 'light' || p.theme === 'dark')) theme = p.theme;
  } catch (e) { /* storage blocked: follow the device */ }
  var root = document.documentElement;
  if (theme !== 'system') root.setAttribute('data-theme', theme);
  var dark = theme === 'dark' || (theme === 'system' && !!window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches);
  var meta = document.querySelector('meta[name="theme-color"]');
  if (meta) meta.setAttribute('content', dark ? '#141517' : '#FFFFFF');
})();
