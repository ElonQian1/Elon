const CACHE_NAME = 'elon-pc-workbench-v1';
const CACHE_PREFIX = 'elon-pc-workbench-';
const APP_SCOPE = '/pc/';

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME)
      .then((cache) => cache.add(APP_SCOPE).catch(() => undefined))
      .then(() => self.skipWaiting()),
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys()
      .then((keys) => Promise.all(
        keys
          .filter((key) => key.startsWith(CACHE_PREFIX) && key !== CACHE_NAME)
          .map((key) => caches.delete(key)),
      ))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener('message', (event) => {
  const data = event.data || {};
  if (data.type !== 'CACHE_URLS' || !Array.isArray(data.urls)) return;
  const urls = data.urls
    .map((raw) => sameOriginPcUrl(raw))
    .filter(Boolean);
  if (urls.length === 0) return;
  event.waitUntil(caches.open(CACHE_NAME).then((cache) => cache.addAll(urls).catch(() => undefined)));
});

self.addEventListener('fetch', (event) => {
  const request = event.request;
  if (request.method !== 'GET') return;

  const url = sameOriginPcUrl(request.url);
  if (!url) return;

  if (request.mode === 'navigate') {
    event.respondWith(networkFirstNavigation(request));
    return;
  }

  if (new URL(url).pathname.startsWith(`${APP_SCOPE}assets/`)) {
    event.respondWith(cacheFirst(request));
  }
});

async function networkFirstNavigation(request) {
  try {
    const response = await fetch(request);
    if (response && response.ok) {
      const cache = await caches.open(CACHE_NAME);
      await cache.put(APP_SCOPE, response.clone());
    }
    return response;
  } catch {
    const cached = await caches.match(request) || await caches.match(APP_SCOPE);
    if (cached) return cached;
    return new Response('<!doctype html><meta charset="utf-8"><title>一龙工作台</title><body>一龙 PC 工作台离线壳尚未缓存，请恢复网络后再打开。</body>', {
      headers: { 'content-type': 'text/html; charset=utf-8' },
      status: 503,
    });
  }
}

async function cacheFirst(request) {
  const cached = await caches.match(request);
  if (cached) return cached;
  const response = await fetch(request);
  if (response && response.ok) {
    const cache = await caches.open(CACHE_NAME);
    await cache.put(request, response.clone());
  }
  return response;
}

function sameOriginPcUrl(raw) {
  try {
    const url = new URL(raw, self.location.origin);
    if (url.origin !== self.location.origin) return '';
    if (!url.pathname.startsWith(APP_SCOPE)) return '';
    return url.toString();
  } catch {
    return '';
  }
}
