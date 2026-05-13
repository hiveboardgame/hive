// Service Worker
const CACHE_PREFIX = 'hivegame-cache-';
const CACHE_NAME = __HIVE_CACHE_NAME__;
const ASSETS_TO_CACHE = __HIVE_ASSETS__;

self.addEventListener('install', (event) => {
  event.waitUntil(
    (async () => {
      try {
        const cache = await caches.open(CACHE_NAME);
        await cache.addAll(ASSETS_TO_CACHE);
        await self.skipWaiting();
      } catch (error) {
        console.error('Cache installation failed:', error);
      }
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

self.addEventListener('fetch', (event) => {
  if (event.request.method !== 'GET') {
    return;
  }

  event.respondWith(
    (async () => {
      const cache = await caches.open(CACHE_NAME);
      const response = await cache.match(event.request);
      return response || fetch(event.request);
    })()
  );
});
