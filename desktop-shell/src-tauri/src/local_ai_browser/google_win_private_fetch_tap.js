(function () {
  'use strict';

  var VERSION = 1;
  var allowedOrigins = new Set(['https://google.com', 'https://www.google.com']);
  if (!allowedOrigins.has(location.origin)) return;
  var existing = window.__elonWinGooglePrivateFetchTap;
  if (existing && Number(existing.version || 0) >= VERSION) return;
  var originalFetch = typeof window.fetch === 'function' ? window.fetch : null;
  if (!originalFetch) return;

  var listeners = new Set();
  var disposed = false;

  function requestMeta(input, init) {
    var url;
    try {
      url = new URL(typeof input === 'string' ? input : input && input.url, location.href);
    } catch (_) {
      return null;
    }
    if (url.origin !== location.origin) return null;
    var method = String(init && init.method || input && input.method || 'GET').toUpperCase();
    if (method !== 'POST') return null;
    var path = url.pathname;
    var privateRpc = path.indexOf('/_/') === 0 ||
      path.indexOf('batchexecute') >= 0 ||
      path.indexOf('/async/') >= 0;
    return privateRpc ? { method: method, url: url.href } : null;
  }

  function publish(meta, response) {
    if (disposed || !meta || !response || typeof response.clone !== 'function') return;
    var clone;
    try { clone = response.clone(); }
    catch (_) { return; }
    var event = Object.freeze({ method: meta.method, url: meta.url, response: clone });
    listeners.forEach(function (listener) {
      try { listener(event); }
      catch (_) { /* The official response remains authoritative. */ }
    });
  }

  var wrappedFetch = function () {
    var args = arguments;
    var meta = requestMeta(args[0], args[1] || {});
    return Promise.resolve(originalFetch.apply(this, args)).then(function (response) {
      publish(meta, response);
      return response;
    });
  };
  window.fetch = wrappedFetch;

  window.__elonWinGooglePrivateFetchTap = Object.freeze({
    version: VERSION,
    subscribe: function (listener) {
      if (typeof listener !== 'function') return function () {};
      listeners.add(listener);
      return function () { listeners.delete(listener); };
    },
    dispose: function () {
      if (disposed) return;
      disposed = true;
      listeners.clear();
      if (window.fetch === wrappedFetch) window.fetch = originalFetch;
    }
  });
})();
