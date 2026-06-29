// Service Worker
const CACHE_PREFIX = 'hivegame-cache-';
const CACHE_NAME = __HIVE_CACHE_NAME__;
const ASSETS_TO_CACHE = __HIVE_ASSETS__;

const CORE_ICONS = [
  '/assets/android-chrome-192x192.png',
  '/assets/favicon.ico',
];

function urlBase64ToUint8Array(base64String) {
  const padding = '='.repeat((4 - (base64String.length % 4)) % 4);
  const base64 = (base64String + padding).replace(/-/g, '+').replace(/_/g, '/');
  const raw = atob(base64);
  const output = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) {
    output[i] = raw.charCodeAt(i);
  }
  return output;
}

self.addEventListener('install', (event) => {
  event.waitUntil(
    (async () => {
      const cache = await caches.open(CACHE_NAME);
      const critical = ASSETS_TO_CACHE.filter((a) => a.startsWith('/pkg/')).concat(CORE_ICONS);
      await Promise.all(
        critical.map((url) =>
          cache.add(url).catch((error) => console.warn('precache miss:', url, error))
        )
      );
      await self.skipWaiting();
    })()
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    (async () => {
      const cacheNames = await caches.keys();
      await Promise.all(
        cacheNames
          .filter((cacheName) => cacheName.startsWith(CACHE_PREFIX) && cacheName !== CACHE_NAME)
          .map((cacheName) => caches.delete(cacheName))
      );
      await self.clients.claim();
    })()
  );
});

self.addEventListener('push', (event) => {
  if (!event.data) {
    return;
  }
  let payload;
  try {
    payload = event.data.json();
  } catch (e) {
    payload = { title: 'HiveGame', body: event.data.text() };
  }
  const tag = payload.link || payload.event_type || 'hivegame';
  const link = payload.link || '/';
  event.waitUntil(
    self.registration.showNotification(payload.title || 'HiveGame', {
      body: payload.body || '',
      icon: '/assets/android-chrome-192x192.png',
      badge: '/assets/android-chrome-192x192.png',
      tag: tag,
      renotify: true,
      data: { link: link },
      actions: [{ action: 'open', title: 'View game' }],
    })
  );
});

self.addEventListener('pushsubscriptionchange', (event) => {
  event.waitUntil(
    (async () => {
      try {
        const resp = await fetch('/api/push/vapid-public-key');
        if (!resp.ok) {
          return;
        }
        const vapid = (await resp.text()).trim();
        if (!vapid) {
          return;
        }
        const sub = await self.registration.pushManager.subscribe({
          userVisibleOnly: true,
          applicationServerKey: urlBase64ToUint8Array(vapid),
        });
        const json = sub.toJSON();
        const oldEndpoint =
          (event.oldSubscription && event.oldSubscription.endpoint) || null;
        await fetch('/api/push/web-subscription', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            endpoint: json.endpoint,
            p256dh: json.keys.p256dh,
            auth: json.keys.auth,
            locale: self.navigator.language || 'en',
            old_endpoint: oldEndpoint,
          }),
        });
      } catch (e) {
      }
    })()
  );
});

self.addEventListener('notificationclick', (event) => {
  event.notification.close();
  const raw = (event.notification.data && event.notification.data.link) || '/';
  const abs = new URL(raw, self.location.origin);
  const url = new URL(abs.pathname + abs.search + abs.hash, self.location.origin);
  event.waitUntil(
    (async () => {
      const windows = await self.clients.matchAll({ type: 'window', includeUncontrolled: true });
      for (const client of windows) {
        if (new URL(client.url).origin === url.origin) {
          await client.focus();
          client.postMessage({
            type: 'hive-navigate',
            path: url.pathname + url.search + url.hash,
          });
          return;
        }
      }
      await self.clients.openWindow(url.href);
    })()
  );
});

self.addEventListener('fetch', (event) => {
  const request = event.request;
  if (request.method !== 'GET') {
    return;
  }

  if (request.mode === 'navigate') {
    return;
  }

  const url = new URL(request.url);
  const isHashedAsset =
    url.origin === self.location.origin &&
    (url.pathname.startsWith('/pkg/') || url.pathname.startsWith('/assets/'));

  if (!isHashedAsset) {
    return;
  }

  event.respondWith(
    (async () => {
      const cache = await caches.open(CACHE_NAME);
      const cached = await cache.match(request);
      if (cached) {
        return cached;
      }
      const response = await fetch(request);
      if (response && response.ok) {
        cache.put(request, response.clone());
      }
      return response;
    })()
  );
});
