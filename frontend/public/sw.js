/* global self, caches, fetch, URL */
const SHELL_CACHE = 'grace-ride-shell-v1';
const STATIC_ASSETS = ['/', '/index.html'];

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
      /^\/(care|visit|invite|recover)\//.test(url.pathname) || url.searchParams.has('code');
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

  if (url.pathname.startsWith('/assets/')) {
    event.respondWith(
      caches.match(request).then(
        (cached) =>
          cached ||
          fetch(request).then((res) => {
            if (res.ok) {
              const copy = res.clone();
              caches.open(SHELL_CACHE).then((c) => c.put(request, copy));
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
  event.waitUntil(
    self.registration.showNotification(payload.title || 'Grace Ride', {
      body: payload.body || '',
      icon: '/icon-192.png',
      badge: '/icon-192.png',
      data: { url: payload.url || '/notifications' },
    }),
  );
});

self.addEventListener('message', (event) => {
  const msg = event.data;
  if (msg && msg.type === 'SHOW_NOTIFICATION') {
    event.waitUntil(
      self.registration.showNotification(msg.title || 'Grace Ride', {
        body: msg.body || '',
        icon: '/icon-192.png',
        badge: '/icon-192.png',
        data: { url: msg.url || '/notifications' },
      }),
    );
  }
});

function safeTargetUrl(raw) {
  try {
    const resolved = new URL(typeof raw === 'string' && raw ? raw : '/notifications', self.location.origin);
    if (
      resolved.origin === self.location.origin &&
      !resolved.pathname.startsWith('//') &&
      !resolved.pathname.startsWith('/api/')
    ) {
      return resolved.href;
    }
  } catch {
    // Fall back to /notifications below.
  }
  return `${self.location.origin}/notifications`;
}

self.addEventListener('notificationclick', (event) => {
  event.notification.close();
  const targetUrl = safeTargetUrl(event.notification.data && event.notification.data.url);
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
