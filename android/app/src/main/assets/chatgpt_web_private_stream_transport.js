(function () {
  'use strict';

  if (window.__elonChatGptPrivateStreamObserverEnabled !== true) return;
  if (location.origin !== 'https://chatgpt.com') return;
  const existing = window.__elonChatGptPrivateStreamTransport;
  if (existing && Number(existing.version) >= 1) return;
  const policy = window.__elonChatGptPrivateStreamPolicy;
  const originalFetch = typeof window.fetch === 'function' ? window.fetch : null;
  if (!policy || !originalFetch || typeof TextDecoder !== 'function') return;

  const session = policy.createSession({ now: Date.now });
  const listeners = new Set();
  let disposed = false;

  function notify() {
    listeners.forEach((listener) => {
      try { listener(); }
      catch (_) { /* Native snapshot fallback remains active. */ }
    });
  }

  function isOfficialConversationStream(method, url, response) {
    if (String(method || 'GET').toUpperCase() !== 'POST') return false;
    if (url.origin !== location.origin || url.pathname !== '/backend-api/f/conversation') return false;
    if (!response || !response.ok || !response.headers || typeof response.headers.get !== 'function') return false;
    return String(response.headers.get('content-type') || '').toLowerCase()
      .includes('text/event-stream');
  }

  async function observe(response) {
    let clone;
    try { clone = response.clone(); }
    catch (_) { return; }
    const reader = clone && clone.body && typeof clone.body.getReader === 'function'
      ? clone.body.getReader()
      : null;
    if (!reader) return;
    session.begin();
    const decoder = new TextDecoder();
    const sse = policy.createSseDecoder(
      (payload) => {
        if (session.accept(payload)) notify();
      },
      () => {
        if (session.finish()) notify();
      }
    );
    try {
      while (!disposed) {
        const value = await reader.read();
        if (value.done) break;
        sse.push(decoder.decode(value.value, { stream: true }));
      }
      sse.push(decoder.decode());
      sse.finish();
    } catch (_) {
      session.reset();
      notify();
    } finally {
      try { reader.releaseLock(); }
      catch (_) { /* The stream may already be closed. */ }
    }
  }

  const wrappedFetch = function () {
    const args = arguments;
    const input = args[0];
    const init = args[1] || {};
    let url;
    try { url = new URL(typeof input === 'string' ? input : input.url, location.href); }
    catch (_) { return originalFetch.apply(this, args); }
    const method = init.method || input && input.method || 'GET';
    return Promise.resolve(originalFetch.apply(this, args)).then((response) => {
      if (isOfficialConversationStream(method, url, response)) observe(response);
      return response;
    });
  };
  window.fetch = wrappedFetch;

  window.__elonChatGptPrivateStreamTransport = Object.freeze({
    version: 1,
    enabled: true,
    current: (pathname) => session.current(pathname),
    mergeMessages: (messages, pathname) => session.merge(messages, pathname),
    subscribe: (listener) => {
      if (typeof listener !== 'function') return function () {};
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    dispose: () => {
      if (disposed) return;
      disposed = true;
      listeners.clear();
      session.reset();
      if (window.fetch === wrappedFetch) window.fetch = originalFetch;
    }
  });
})();
