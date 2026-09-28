// Service Worker
const CACHE_PREFIX = 'hivegame-cache-';
const CACHE_NAME = __HIVE_CACHE_NAME__;
const ASSETS_TO_CACHE = __HIVE_ASSETS__;

const CORE_ICONS = [
  '/assets/android-chrome-192x192.png',
  '/assets/favicon.ico',
];

async function postWithCsrf(path, body) {
  const url = new URL(path, self.location.origin);
  if (url.origin !== self.location.origin) {
    throw new Error('Worker requests must use the application origin');
  }
  const options = { credentials: 'same-origin', mode: 'same-origin', redirect: 'error' };
  const tokenResponse = await fetch('/api/csrf', { ...options, cache: 'no-store' });
  if (!tokenResponse.ok) {
    throw new Error(`CSRF token request failed (${tokenResponse.status})`);
  }
  const token = await tokenResponse.text();
  if (!token) {
    throw new Error('CSRF token response was empty');
  }
  const response = await fetch(url.href, {
    ...options,
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': token },
    body: JSON.stringify(body),
  });
  if (!response.ok) {
    throw new Error(`Push subscription update failed (${response.status})`);
  }
}

function isTileSvgRequest(request) {
  const url = new URL(request.url);
  return (
    url.origin === self.location.origin &&
    url.pathname.startsWith('/assets/tiles/') &&
    url.pathname.endsWith('.svg')
  );
}

async function deleteTileSvgEntries(cache) {
  const requests = await cache.keys();
  await Promise.all(
    requests
      .filter((request) => isTileSvgRequest(request))
      .map((request) => cache.delete(request))
  );
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
      const cache = await caches.open(CACHE_NAME);
      await deleteTileSvgEntries(cache);
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
  // Missing subscriptions can mean permission was revoked or ownership was lost.
  // Only an explicit subscription from the page may create a new registration.
  if (!event.oldSubscription || !event.newSubscription) {
    return;
  }
  event.waitUntil(
    (async () => {
      try {
        const json = event.newSubscription.toJSON();
        await postWithCsrf('/api/push/web-subscription', {
          endpoint: json.endpoint,
          p256dh: json.keys.p256dh,
          auth: json.keys.auth,
          locale: self.navigator.language || 'en',
          old_endpoint: event.oldSubscription.endpoint,
        });
      } catch (error) {
        // A changed login or failed connection ends this attempt; never replay it.
        console.warn('Push subscription renewal failed:', error.message);
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
  if (isTileSvgRequest(request)) {
    return;
  }

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
