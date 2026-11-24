const SHELL_CACHE = 'grace-ride-shell-v4';
const STATIC_ASSETS = [
  '/',
  '/index.html',
  '/theme-boot.js',
  '/manifest.webmanifest',
  '/icon.svg?v=2',
  '/icon-192.png?v=2',
  '/apple-touch-icon.png?v=2',
  '/badge-monochrome.png',
];
const MAX_RUNTIME_ASSETS = 200;
const NOTIFICATION_ICON = '/icon-192.png?v=2';
const NOTIFICATION_BADGE = '/badge-monochrome.png';

function isCacheableStaticPath(pathname) {
  return (
    pathname.startsWith('/assets/') ||
    pathname.startsWith('/icons/') ||
    pathname === '/theme-boot.js' ||
    pathname === '/manifest.webmanifest' ||
    pathname.startsWith('/icon') ||
    pathname === '/apple-touch-icon.png' ||
    pathname === '/badge-monochrome.png'
  );
}

function trimRuntimeCache(cache) {
  if (typeof cache.keys !== 'function' || typeof cache.delete !== 'function') return;
  const pinned = new Set(
    STATIC_ASSETS.map((p) => new URL(p, self.location.origin).href),
  );
  cache.keys().then((requests) => {
    const runtime = requests.filter((req) => !pinned.has(req.url));
    const excess = runtime.length - MAX_RUNTIME_ASSETS;
    for (let i = 0; i < excess; i++) {
      cache.delete(runtime[i]);
    }
  });
}

function urlBase64ToUint8Array(base64String) {
  const pad = '='.repeat((4 - (base64String.length % 4)) % 4);
  const b64 = (base64String + pad).replace(/-/g, '+').replace(/_/g, '/');
  const raw = atob(b64);
  const bytes = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) {
    bytes[i] = raw.charCodeAt(i);
  }
  return bytes;
}

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches
      .open(SHELL_CACHE)
      .then((cache) => cache.addAll(STATIC_ASSETS))
      .then(() => self.skipWaiting()),
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(keys.filter((k) => k !== SHELL_CACHE).map((k) => caches.delete(k))),
      )
      .then(() => self.clients.claim()),
  );
});

self.addEventListener('fetch', (event) => {
  const { request } = event;
  if (request.method !== 'GET') return;
  const url = new URL(request.url);
  // Never cache /api/* responses in CacheStorage: private ride, member, and medical
  // data must stay governed by authenticated requests and lock-cleared snapshots.
  if (url.origin !== self.location.origin || url.pathname.startsWith('/api/')) {
    return;
  }

  if (request.mode === 'navigate') {
    const isSensitiveRoute =
      /^\/(care|visit|invite)\//.test(url.pathname) || url.searchParams.has('code');
    event.respondWith(
      fetch(request)
        .then((res) => {
          const ct = res.headers.get('content-type') || '';
          if (!isSensitiveRoute && res.ok && ct.includes('text/html')) {
            const copy = res.clone();
            // Store under canonical '/index.html' rather than request.url so bearer
            // capability URLs (/care/:token, /visit/:token, /invite/:token, ?code=)
            // are never persisted as CacheStorage keys.
            caches.open(SHELL_CACHE).then((c) => c.put('/index.html', copy));
          }
          return res;
        })
        .catch(() => caches.match('/index.html').then((r) => r || caches.match('/'))),
    );
    return;
  }

  if (isCacheableStaticPath(url.pathname)) {
    event.respondWith(
      caches.match(request).then(
        (cached) =>
          cached ||
          fetch(request).then((res) => {
            if (res.ok) {
              const copy = res.clone();
              caches.open(SHELL_CACHE).then((c) => {
                c.put(request, copy);
                trimRuntimeCache(c);
              });
            }
            return res;
          }),
      ),
    );
  }
});

self.addEventListener('push', (event) => {
  let payload = { title: 'Grace Ride', body: '', url: '/notifications' };
  try {
    if (event.data) {
      payload = { ...payload, ...event.data.json() };
    }
  } catch {
    if (event.data) payload.body = event.data.text();
  }
  const options = {
    body: payload.body || '',
    icon: NOTIFICATION_ICON,
    badge: NOTIFICATION_BADGE,
    data: { url: payload.url || '/notifications' },
  };
  if (typeof payload.tag === 'string' && payload.tag) {
    options.tag = payload.tag;
    options.renotify = payload.priority === 'urgent';
  }
  if (Array.isArray(payload.actions) && payload.actions.length > 0) {
    options.actions = payload.actions;
  }
  const showPromise = self.registration.showNotification(payload.title || 'Grace Ride', options);
  const badgePromise =
    self.navigator && typeof self.navigator.setAppBadge === 'function'
      ? (typeof payload.badge_count === 'number' && payload.badge_count > 0
          ? self.navigator.setAppBadge(payload.badge_count)
          : typeof payload.badge_count === 'number' && payload.badge_count === 0 && typeof self.navigator.clearAppBadge === 'function'
            ? self.navigator.clearAppBadge()
            : self.navigator.setAppBadge()
        ).catch(() => {})
      : Promise.resolve();
  event.waitUntil(Promise.all([showPromise, badgePromise]));
});

self.addEventListener('pushsubscriptionchange', (event) => {
  event.waitUntil(
    fetch('/api/push/config', { credentials: 'same-origin' })
      .then((res) => (res.ok ? res.json() : null))
      .then(async (cfg) => {
        if (!cfg || !cfg.vapid_public_key) return;
        const appKey = urlBase64ToUint8Array(cfg.vapid_public_key);
        const sub = await self.registration.pushManager.subscribe({
          userVisibleOnly: true,
          applicationServerKey: appKey,
        });
        const json = sub.toJSON();
        if (!json.endpoint) return;
        await fetch('/api/push/subscribe', {
          method: 'POST',
          credentials: 'same-origin',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({
            endpoint: json.endpoint,
            p256dh: (json.keys && json.keys.p256dh) || '',
            auth: (json.keys && json.keys.auth) || '',
          }),
        });
      })
      .catch(() => {}),
  );
});

self.addEventListener('message', (event) => {
  const msg = event.data;
  if (msg && msg.type === 'SHOW_NOTIFICATION') {
    event.waitUntil(
      self.registration.showNotification(msg.title || 'Grace Ride', {
        body: msg.body || '',
        icon: NOTIFICATION_ICON,
        badge: NOTIFICATION_BADGE,
        data: { url: msg.url || '/notifications' },
      }),
    );
  }
});

function safeTargetUrl(raw, action) {
  let resolved;
  try {
    resolved = new URL(typeof raw === 'string' && raw ? raw : '/notifications', self.location.origin);
    if (
      resolved.origin !== self.location.origin ||
      resolved.pathname.startsWith('//') ||
      resolved.pathname.startsWith('/api/')
    ) {
      resolved = new URL('/notifications', self.location.origin);
    }
  } catch {
    resolved = new URL('/notifications', self.location.origin);
  }
  if (action === 'ready' || action === 'delayed_5m') {
    resolved.searchParams.set('quick', action);
  }
  return resolved.href;
}

self.addEventListener('notificationclick', (event) => {
  event.notification.close();
  const targetUrl = safeTargetUrl(
    event.notification.data && event.notification.data.url,
    event.action,
  );
  event.waitUntil(
    self.clients.matchAll({ type: 'window', includeUncontrolled: true }).then((clients) => {
      for (const client of clients) {
        if ('focus' in client && 'navigate' in client) {
          return Promise.resolve(client.navigate(targetUrl))
            .catch(() => undefined)
            .then(() => client.focus());
        }
      }
      if (self.clients.openWindow) {
        return self.clients.openWindow(targetUrl);
      }
      return undefined;
    }),
  );
});
