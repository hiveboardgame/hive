// Service Worker

// Change CACHE_NAME for major upgrades
const CACHE_NAME = 'hivegame-cache-v1';

// Fetch and cache assets list from JSON
async function getAssetsToCache() {
  try {
    const response = await fetch('/pwa-cache');
    const data = await response.json();
    return data;
  } catch (error) {
    console.error('Failed to load cache list:', error);
    return []; // Return empty array as fallback
  }
}

self.addEventListener('install', async (event) => {
  event.waitUntil(
    (async () => {
      try {
        const assetsToCache = await getAssetsToCache();
        const cache = await caches.open(CACHE_NAME);
        await cache.addAll(assetsToCache);
        await self.skipWaiting();
      } catch (error) {
        console.error('Cache installation failed:', error);
      }
    })()
  );
});

self.addEventListener('fetch', (event) => {
  event.respondWith(
    caches.match(event.request)
      .then((response) => response || fetch(event.request))
  );
});
