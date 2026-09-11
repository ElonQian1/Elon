(function () {
  'use strict';

  if (window.__elonChatGptPrivateStreamObserverEnabled !== true) return;
  if (location.origin !== 'https://chatgpt.com') return;
  const existing = window.__elonChatGptPrivateFetchTap;
  // Keep an already captured v2 fetch alive until the next document load.
  if (existing && Number(existing.version) >= 2) return;
  const originalFetch = typeof window.fetch === 'function' ? window.fetch : null;
  if (!originalFetch) return;

  const listeners = new Set();
  let disposed = false;
  let conversationRequestId = 0;

  function resumeMeta(init) {
    try {
      // Only the official resume cursor is inspected, never send bodies or headers.
      const body = init && init.body;
      if (typeof body !== 'string' || body.length > 1024) return null;
      const value = JSON.parse(body);
      if (!value || Object.keys(value).length !== 2 ||
          typeof value.conversation_id !== 'string' ||
          !/^[A-Za-z0-9_-]{1,160}$/.test(value.conversation_id) ||
          !Number.isSafeInteger(value.offset) || value.offset < 0) return null;
      return Object.freeze({ conversationId: value.conversation_id, offset: value.offset });
    } catch (_) { return null; }
  }

  function requestMeta(input, init) {
    let url;
    try { url = new URL(typeof input === 'string' ? input : input && input.url, location.href); }
    catch (_) { return null; }
    if (url.origin !== location.origin) return null;
    const method = String(init && init.method || input && input.method || 'GET').toUpperCase();
    const conversation = method === 'POST' &&
      /^\/(?:backend-api|backend-anon)\/(?:f\/)?conversation\/?$/.test(url.pathname);
    const streamStatus = method === 'GET' &&
      /^\/backend-api\/conversation\/[A-Za-z0-9_-]{1,160}\/stream_status$/.test(url.pathname);
    const isResume = method === 'POST' &&
      /^\/(?:backend-api|backend-anon)\/f\/conversation\/resume$/.test(url.pathname);
    const resume = isResume ? resumeMeta(init) : null;
    if (!conversation && !streamStatus && !resume) return null;
    if (conversation) conversationRequestId += 1;
    return { method, url: url.href, resume, requestId: conversationRequestId,
      documentToken: String(window.__elonChatGptDocumentToken || ''), pathname: location.pathname };
  }

  function publish(meta, response) {
    if (disposed || !meta || !response || typeof response.clone !== 'function') return;
    let clone;
    try { clone = response.clone(); }
    catch (_) { return; }
    const event = Object.freeze(Object.assign({}, meta, { response: clone }));
    listeners.forEach((listener) => {
      try { listener(event); }
      catch (_) { /* The official response remains authoritative. */ }
    });
  }

  const wrappedFetch = function () {
    const args = arguments;
    const meta = requestMeta(args[0], args[1] || {});
    return Promise.resolve(originalFetch.apply(this, args)).then((response) => {
      publish(meta, response);
      return response;
    });
  };
  window.fetch = wrappedFetch;

  window.__elonChatGptPrivateFetchTap = Object.freeze({
    version: 3,
    subscribe: (listener) => {
      if (typeof listener !== 'function') return function () {};
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    dispose: () => {
      if (disposed) return;
      disposed = true;
      listeners.clear();
      if (window.fetch === wrappedFetch) window.fetch = originalFetch;
    }
  });
})();
