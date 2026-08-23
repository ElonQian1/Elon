(function () {
  'use strict';

  const existing = window.__elonGoogleWebPrivateThreadDirectory;
  if (existing && Number(existing.version) >= 1) return;
  if (location.origin !== 'https://google.com' && location.origin !== 'https://www.google.com') return;

  const xhrPrototype = window.XMLHttpRequest && window.XMLHttpRequest.prototype;
  const originalOpen = xhrPrototype && xhrPrototype.open;
  const originalSend = xhrPrototype && xhrPrototype.send;
  const metadata = new WeakMap();
  let conversations = [];
  let listener = null;
  const MAX_CONVERSATIONS = 200;
  const MAX_RESPONSE_BYTES = 1024 * 1024;
  const SAFE_ID = /^[A-Za-z0-9_-]{12,160}$/;

  function parseRows(text) {
    const normalized = String(text || '').replace(/^\)\]\}'\s*/, '').trim();
    if (!normalized || normalized.length > MAX_RESPONSE_BYTES) return [];
    let payload;
    try { payload = JSON.parse(normalized); }
    catch (_) { return []; }
    const rows = [];
    const seen = new Set();
    let inspected = 0;

    function visit(value, depth) {
      if (rows.length >= MAX_CONVERSATIONS || inspected >= 1200 || depth > 7) return;
      inspected += 1;
      if (!Array.isArray(value)) return;
      if (value.length >= 2 && typeof value[0] === 'string' && typeof value[1] === 'string') {
        const id = value[0].trim();
        const title = value[1].replace(/\s+/g, ' ').trim().slice(0, 160);
        if (SAFE_ID.test(id) && title && !seen.has(id)) {
          seen.add(id);
          rows.push({ id, title });
        }
      }
      value.slice(0, 80).forEach((item) => visit(item, depth + 1));
    }

    visit(payload, 0);
    return rows;
  }

  function restorableRows(rows) {
    let current;
    try { current = new URL(location.href); }
    catch (_) { return []; }
    const csuir = current.searchParams.get('csuir') || '';
    const active = rows.find((row) => csuir.includes(row.id));
    if (!active) return [];
    return rows.map((row) => {
      const url = new URL(current.href);
      url.searchParams.set('csuir', csuir.replace(active.id, row.id));
      if (url.searchParams.has('q')) url.searchParams.set('q', row.title);
      return Object.freeze({
        id: row.id,
        title: row.title,
        path: '/c/' + row.id,
        providerUrl: url.href.slice(0, 8192)
      });
    });
  }

  function acceptResponse(text) {
    const next = restorableRows(parseRows(text));
    if (!next.length) return;
    conversations = next;
    if (typeof listener === 'function') listener();
  }

  if (originalOpen && originalSend) {
    xhrPrototype.open = function (method, rawUrl) {
      let url = null;
      try { url = new URL(String(rawUrl || ''), location.href); }
      catch (_) { /* Leave unrelated requests untouched. */ }
      metadata.set(this, {
        eligible: String(method || 'GET').toUpperCase() === 'GET' && !!url &&
          url.origin === location.origin &&
          url.pathname.endsWith('/AimThreadsService/ListThreads')
      });
      return originalOpen.apply(this, arguments);
    };
    xhrPrototype.send = function () {
      const state = metadata.get(this);
      if (state && state.eligible && typeof this.addEventListener === 'function') {
        this.addEventListener('load', () => {
          if (this.status < 200 || this.status >= 300) return;
          let text = '';
          try { text = String(this.responseText || ''); }
          catch (_) { return; }
          acceptResponse(text);
        }, { once: true });
      }
      return originalSend.apply(this, arguments);
    };
  }

  window.__elonGoogleWebPrivateThreadDirectory = Object.freeze({
    version: 1,
    snapshot: () => conversations.slice(),
    setListener: (value) => { listener = typeof value === 'function' ? value : null; }
  });
})();
