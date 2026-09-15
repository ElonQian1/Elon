// Public app shell only. Authenticated APIs, media and writes never enter CacheStorage.
'use strict';
const SHELL = 'elon-mobile-shell-v1';
self.addEventListener('install', event => event.waitUntil((async () => {
  try {
    const cache = await caches.open(SHELL), response = await fetch('/', { cache: 'reload' });
    if (response.ok && !response.redirected && (response.headers.get('content-type') || '').includes('text/html')) {
      const html = await response.clone().text(); await cache.put('/', response);
      const assets = [...new Set(Array.from(html.matchAll(/(?:src|href)="(\/assets\/[\w.-]+\.(?:js|css))"/g), match => match[1]))];
      await Promise.allSettled(assets.map(async path => {
        const asset = await fetch(path, { cache: 'reload' }); if (asset.ok) await cache.put(path, asset);
      }));
    }
  } catch {}
  await self.skipWaiting();
})()));
self.addEventListener('activate', event => event.waitUntil(self.clients.claim()));
self.addEventListener('fetch', event => {
  const request = event.request, url = new URL(request.url);
  if (request.method !== 'GET' || url.origin !== self.location.origin) return;
  const navigation = request.mode === 'navigate' && url.pathname === '/';
  const asset = /^\/assets\/[\w.-]+\.(js|css)$/.test(url.pathname);
  if (!navigation && !asset) return;
  const key = navigation ? '/' : url.pathname;
  event.respondWith((async () => {
    let cache;
    try { cache = await caches.open(SHELL); } catch { return fetch(request); }
    const saved = await cache.match(key);
    const network = fetch(request).then(async response => {
      const type = response.headers.get('content-type') || '';
      if (response.ok && !response.redirected && (navigation ? type.includes('text/html') : /javascript|text\/css/.test(type))) {
        const copy = response.clone();
        event.waitUntil((async () => {
          try {
            await cache.put(key, copy);
            const keys = await cache.keys();
            for (const old of keys.slice(0, Math.max(0, keys.length - 80))) if (new URL(old.url).pathname !== '/') await cache.delete(old);
          } catch {}
        })());
      }
      return response;
    });
    event.waitUntil(network.catch(() => {}));
    if (!saved) return network;
    let timer;
    try {
      return await Promise.race([network.catch(() => saved), new Promise(resolve => { timer = setTimeout(() => resolve(saved), 1200); })]);
    } finally { clearTimeout(timer); }
  })());
});
