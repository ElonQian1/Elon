(function () {
  'use strict';

  if (window.__elonChatGptPrivateStreamObserverEnabled !== true) return;
  if (location.origin !== 'https://chatgpt.com') return;
  const existing = window.__elonChatGptPrivateStreamTransport;
  if (existing && Number(existing.version) >= 2) return;
  const policy = window.__elonChatGptPrivateStreamPolicy;
  const originalFetch = typeof window.fetch === 'function' ? window.fetch : null;
  if (!policy || !originalFetch || typeof TextDecoder !== 'function') return;

  const session = policy.createSession({ now: Date.now });
  const researchProbe = window.__elonChatGptPrivateResearchProbe;
  const listeners = new Set();
  let disposed = false;

  function notify() {
    listeners.forEach((listener) => {
      try { listener(); }
      catch (_) { /* Native snapshot fallback remains active. */ }
    });
  }

  function schemaKeys(value) {
    if (!value || typeof value !== 'object' || Array.isArray(value)) return 'none';
    const keys = Object.keys(value)
      .filter((key) => /^[A-Za-z][A-Za-z0-9_]{0,39}$/.test(key))
      .sort()
      .slice(0, 10);
    return keys.length ? keys.join('.') : 'none';
  }

  function schemaType(value) {
    const token = String(value || '').toLowerCase();
    return /^[a-z][a-z0-9_-]{0,39}$/.test(token) ? token : 'none';
  }

  function reportShape(payload) {
    if (!researchProbe || typeof researchProbe.recordPrivateStreamShape !== 'function') return;
    const data = payload && typeof payload.data === 'object' ? payload.data : null;
    const message = payload && typeof payload.message === 'object' ? payload.message :
      (data && typeof data.message === 'object' ? data.message : null);
    const content = message && typeof message.content === 'object' ? message.content : null;
    researchProbe.recordPrivateStreamShape([
      't:' + schemaType(payload && payload.type),
      'k:' + schemaKeys(payload),
      'dt:' + schemaType(data && data.type),
      'dk:' + schemaKeys(data),
      'mk:' + schemaKeys(message),
      'ck:' + schemaKeys(content)
    ].join('/'));
  }

  function isOfficialConversationStream(method, url, response) {
    if (String(method || 'GET').toUpperCase() !== 'POST') return false;
    if (url.origin !== location.origin ||
        !/^\/backend-api\/(?:f\/)?conversation(?:\/|$)/.test(url.pathname)) return false;
    if (!response || !response.ok || !response.headers || typeof response.headers.get !== 'function') return false;
    return String(response.headers.get('content-type') || '').toLowerCase()
      .includes('text/event-stream');
  }

  async function observe(response) {
    const startedAt = Date.now();
    let frames = 0;
    let firstReported = false;
    function report(outcome) {
      if (researchProbe && typeof researchProbe.recordPrivateStreamOutcome === 'function') {
        researchProbe.recordPrivateStreamOutcome(outcome, frames, Date.now() - startedAt);
      }
    }
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
        reportShape(payload);
        if (!session.accept(payload)) return;
        frames += 1;
        if (!firstReported) {
          firstReported = true;
          report('first');
        }
        notify();
      },
      () => {
        const completed = session.finish();
        report(completed ? 'success' : 'empty');
        if (completed) notify();
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
      report('error');
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
    version: 2,
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
