(function () {
  'use strict';

  if (window.__elonGoogleWebPrivateResearchEnabled !== true) return;
  if (location.origin !== 'https://google.com' && location.origin !== 'https://www.google.com') return;
  const existing = window.__elonGoogleWebPrivateResponseTap;
  if (existing && Number(existing.version) >= 1) return;

  const originalFetch = typeof window.fetch === 'function' ? window.fetch : null;
  const xhrPrototype = window.XMLHttpRequest && window.XMLHttpRequest.prototype;
  const originalOpen = xhrPrototype && xhrPrototype.open;
  const originalSend = xhrPrototype && xhrPrototype.send;
  const xhrMetadata = new WeakMap();
  const observations = [];
  const seen = new Set();
  let expectedMarker = '';
  let threadDirectoryPairs = [];
  let disposed = false;
  const MAX_OBSERVATIONS = 64;
  const MAX_RESPONSE_BYTES = 1024 * 1024;

  function safeSegment(value) {
    const segment = String(value || '');
    if (!segment) return '';
    if (/^(?:_|[A-Za-z][A-Za-z0-9._-]{0,31})$/.test(segment)) return segment;
    if (/^[0-9]{1,5}$/.test(segment)) return '{index}';
    return '{id}';
  }

  function safePath(url) {
    return url.pathname.split('/').map(safeSegment).join('/').slice(0, 120) || '/';
  }

  function contentKind(value) {
    const type = String(value || '').toLowerCase();
    if (type.includes('json')) return 'json';
    if (type.includes('protobuf')) return 'protobuf';
    if (type.includes('event-stream')) return 'sse';
    if (type.startsWith('text/')) return 'text';
    return type ? 'other' : 'unknown';
  }

  function lengthBucket(length) {
    if (length < 256) return 'xs';
    if (length < 4096) return 's';
    if (length < 65536) return 'm';
    if (length < MAX_RESPONSE_BYTES) return 'l';
    return 'xl';
  }

  function shapeOf(text) {
    const value = String(text || '');
    let arrays = 0;
    let objects = 0;
    let strings = 0;
    let maxDepth = 0;
    let parsed = null;
    const normalized = value.replace(/^\)\]\}'\s*/, '').trim();
    try { parsed = JSON.parse(normalized); }
    catch (_) { /* Batched or streamed payloads remain structurally opaque. */ }
    if (parsed !== null) {
      const queue = [{ value: parsed, depth: 0 }];
      let inspected = 0;
      while (queue.length && inspected < 120) {
        const entry = queue.shift();
        inspected += 1;
        maxDepth = Math.max(maxDepth, entry.depth);
        if (typeof entry.value === 'string') {
          strings += 1;
        } else if (Array.isArray(entry.value)) {
          arrays += 1;
          entry.value.slice(0, 24).forEach((item) => queue.push({
            value: item,
            depth: entry.depth + 1
          }));
        } else if (entry.value && typeof entry.value === 'object') {
          objects += 1;
          Object.keys(entry.value).slice(0, 24).forEach((key) => queue.push({
            value: entry.value[key],
            depth: entry.depth + 1
          }));
        }
      }
    }
    return [
      parsed === null ? 'opaque' : 'json',
      'a' + Math.min(arrays, 99),
      'o' + Math.min(objects, 99),
      's' + Math.min(strings, 99),
      'd' + Math.min(maxDepth, 12),
      'm' + (expectedMarker && value.includes(expectedMarker) ? '1' : '0')
    ].join('.');
  }

  function threadDirectoryShape(text) {
    const normalized = String(text || '').replace(/^\)\]\}'\s*/, '').trim();
    let parsed;
    try { parsed = JSON.parse(normalized); }
    catch (_) { return 'opaque'; }
    const leaves = [];
    const pairs = [];
    let inspected = 0;
    let markerPath = '';

    function stringClass(value) {
      if (/^https:\/\/(?:www\.)?google\.com\//i.test(value)) return 'url';
      if (/^[A-Za-z0-9_-]{12,160}$/.test(value)) return 'id';
      if (/^\d{4}-\d{2}-\d{2}(?:[T ][^ ]+)?$/.test(value)) return 'date';
      return 'text';
    }

    function visit(value, path, depth) {
      if (inspected >= 180 || depth > 6) return;
      inspected += 1;
      if (typeof value === 'string') {
        const controlledMarker = value.match(/ELONGOOGLE[A-Z0-9_-]{8,120}/);
        if (((expectedMarker && value.includes(expectedMarker)) || controlledMarker) && !markerPath) {
          markerPath = path.join('.');
        }
        if (leaves.length < 18) {
          leaves.push(path.join('.') + ':' + stringClass(value) + ':' + lengthBucket(value.length));
        }
        return;
      }
      if (Array.isArray(value)) {
        if (value.length >= 2 && typeof value[0] === 'string' && typeof value[1] === 'string') {
          const id = String(value[0] || '').trim();
          const title = String(value[1] || '').trim();
          if (title && title.length <= 200 && id && id.length <= 500) {
            pairs.push({ title, id });
          }
        }
        value.slice(0, 48).forEach((item, index) => visit(item, path.concat(index), depth + 1));
        return;
      }
      if (value && typeof value === 'object') {
        Object.keys(value).sort().slice(0, 24).forEach((key) => {
          const safeKey = safeSegment(key);
          visit(value[key], path.concat(safeKey || 'key'), depth + 1);
        });
      }
    }

    visit(parsed, [], 0);
    threadDirectoryPairs = pairs.slice(0, 80);
    return [
      'json',
      'm' + (markerPath || '0'),
      'v' + leaves.join(',')
    ].join('|').slice(0, 130);
  }

  function threadLinkShape() {
    if (!threadDirectoryPairs.length || !document.body) return 'm0';
    const links = Array.from(document.querySelectorAll('a[href]'));
    for (const pair of threadDirectoryPairs) {
      const link = links.find((candidate) =>
        String(candidate.textContent || '').replace(/\s+/g, ' ').trim() === pair.title
      );
      if (!link) continue;
      let url;
      try { url = new URL(link.href, location.href); }
      catch (_) { continue; }
      if (url.origin !== location.origin) continue;
      const keys = Array.from(url.searchParams.keys())
        .map(safeSegment)
        .filter(Boolean)
        .sort()
        .slice(0, 12);
      const idKey = Array.from(url.searchParams.entries())
        .find((entry) => entry[1] === pair.id);
      return [
        'm1',
        'k' + (keys.join('.') || '0'),
        'i' + (idKey ? safeSegment(idKey[0]) : (url.pathname.includes(pair.id) ? 'path' : '0')),
        'a' + (url.searchParams.get('udm') === '50' ? 1 : 0)
      ].join('|').slice(0, 120);
    }
    return 'm0';
  }

  function threadIdShape() {
    const keys = new Set();
    let urls = 0;
    let relative = 0;
    let aiMode = 0;
    let exactLocationKey = '';
    let containedLocationKey = '';
    let locationEntries = [];
    try {
      locationEntries = Array.from(new URL(location.href).searchParams.entries());
    } catch (_) { /* Keep the research observation structural only. */ }
    threadDirectoryPairs.forEach((pair) => {
      if (!exactLocationKey || !containedLocationKey) {
        locationEntries.forEach((entry) => {
          const key = safeSegment(entry[0]);
          if (!key || !entry[1]) return;
          if (!exactLocationKey && entry[1] === pair.id) exactLocationKey = key;
          if (!containedLocationKey &&
              (entry[1].includes(pair.id) || pair.id.includes(entry[1]))) {
            containedLocationKey = key;
          }
        });
      }
      if (pair.id.startsWith('/')) relative += 1;
      let url;
      try { url = new URL(pair.id, location.href); }
      catch (_) { return; }
      if (url.origin !== location.origin || !pair.id.includes('/')) return;
      urls += 1;
      if (url.pathname === '/aimode' || url.searchParams.get('udm') === '50' ||
          url.searchParams.get('aep') === '11') aiMode += 1;
      Array.from(url.searchParams.keys()).forEach((key) => {
        const safe = safeSegment(key);
        if (safe) keys.add(safe);
      });
    });
    return [
      'n' + Math.min(threadDirectoryPairs.length, 99),
      'u' + Math.min(urls, 99),
      'r' + Math.min(relative, 99),
      'a' + Math.min(aiMode, 99),
      'k' + (Array.from(keys).sort().slice(0, 12).join('.') || '0'),
      'e' + (exactLocationKey || '0'),
      'c' + (containedLocationKey || '0')
    ].join('|');
  }

  function locationShape() {
    let url;
    try { url = new URL(location.href); }
    catch (_) { return 'invalid'; }
    const keys = Array.from(url.searchParams.keys())
      .map(safeSegment)
      .filter(Boolean)
      .sort()
      .slice(0, 12);
    return 'p' + safePath(url) + '|k' + (keys.join('.') || '0');
  }

  function scheduleThreadLinkObservation() {
    [100, 500, 1500, 3000].forEach((delay) => {
      setTimeout(() => {
        if (disposed || !threadDirectoryPairs.length) return;
        const detail = 'v1|schema|threadlink|' + threadLinkShape();
        if (!seen.has(detail) && observations.length < MAX_OBSERVATIONS) {
          seen.add(detail);
          observations.push(detail);
        }
      }, delay);
    });
  }

  function recordThreadDirectorySchema(url, text) {
    if (!url.pathname.endsWith('/AimThreadsService/ListThreads')) return;
    const detail = 'v1|schema|threads|' + threadDirectoryShape(text);
    if (seen.has(detail) || observations.length >= MAX_OBSERVATIONS) return;
    seen.add(detail);
    observations.push(detail.slice(0, 160));
    const idDetail = 'v1|schema|threadids|' + threadIdShape();
    if (!seen.has(idDetail) && observations.length < MAX_OBSERVATIONS) {
      seen.add(idDetail);
      observations.push(idDetail.slice(0, 160));
    }
    const locationDetail = 'v1|schema|location|' + locationShape();
    if (!seen.has(locationDetail) && observations.length < MAX_OBSERVATIONS) {
      seen.add(locationDetail);
      observations.push(locationDetail.slice(0, 160));
    }
    scheduleThreadLinkObservation();
  }

  function markerDomShape() {
    if (!expectedMarker || !document.body) return 'm0';
    const nodes = Array.from(document.querySelectorAll('body *')).filter((node) => {
      if (!(node instanceof Element)) return false;
      return Array.from(node.childNodes).some((child) =>
        child.nodeType === Node.TEXT_NODE && String(child.nodeValue || '').includes(expectedMarker)
      );
    }).slice(0, 8);
    if (!nodes.length) return 'm0';
    const exactNodes = nodes.filter((candidate) =>
      String(candidate.textContent || '').trim() === expectedMarker
    );
    const node = exactNodes[exactNodes.length - 1] || nodes[nodes.length - 1];
    const queryNode = nodes.find((candidate) => candidate !== node &&
      String(candidate.textContent || '').trim() !== expectedMarker
    ) || null;
    const rect = node.getBoundingClientRect();
    const path = [];
    for (let current = node; current && current instanceof Element && path.length < 6;
        current = current.parentElement) {
      path.push(String(current.tagName || '').toLowerCase().replace(/[^a-z0-9-]/g, '').slice(0, 12));
    }
    const visible = rect.width > 0 && rect.height > 0 && node.getClientRects().length > 0;
    const controls = node.querySelectorAll('button, [role="button"], input, textarea').length;
    const links = node.querySelectorAll('a[href]').length;
    const afterQuery = !!(queryNode && typeof Node !== 'undefined' &&
      (queryNode.compareDocumentPosition(node) & Node.DOCUMENT_POSITION_FOLLOWING));
    const excluded = node.closest(
      'header, nav, footer, form, [role="navigation"], [role="dialog"], ' +
      '[role="tablist"], [role="toolbar"]'
    );
    return [
      'm' + Math.min(nodes.length, 8),
      'e' + Math.min(exactNodes.length, 8),
      'v' + (visible ? 1 : 0),
      'a' + (afterQuery ? 1 : 0),
      'main' + (node.closest('main, [role="main"]') ? 1 : 0),
      'i' + (node.closest('a[href], button, [role="button"], [role="link"]') ? 1 : 0),
      'x' + (excluded ? 1 : 0),
      'li' + (node.closest('li, [role="listitem"]') ? 1 : 0),
      'live' + (node.closest('[aria-live], [role="status"], [role="alert"]') ? 1 : 0),
      'c' + Math.min(controls, 99),
      'l' + Math.min(links, 99),
      'w' + lengthBucket(Math.round(rect.width)),
      'p' + path.filter(Boolean).join('.')
    ].join('|').slice(0, 140);
  }

  function scheduleMarkerDomObservation() {
    [100, 500, 1500, 3000].forEach((delay) => {
      setTimeout(() => {
        if (disposed || !expectedMarker) return;
        const detail = 'v1|dom|marker|' + markerDomShape();
        if (!seen.has(detail) && observations.length < MAX_OBSERVATIONS) {
          seen.add(detail);
          observations.push(detail);
        }
      }, delay);
    });
  }

  function record(transport, method, url, status, kind, length, shape) {
    if (disposed || observations.length >= MAX_OBSERVATIONS || url.origin !== location.origin) return;
    const detail = [
      'v1',
      transport,
      String(method || 'GET').toUpperCase().replace(/[^A-Z]/g, '').slice(0, 8) || 'GET',
      safePath(url),
      String(Math.max(0, Math.min(999, Number(status) || 0))),
      kind,
      lengthBucket(length),
      shape
    ].join('|').slice(0, 160);
    if (seen.has(detail)) return;
    seen.add(detail);
    observations.push(detail);
  }

  function inspectFetchResponse(method, url, response) {
    if (!response || !response.ok || url.origin !== location.origin) return;
    let clone;
    try { clone = response.clone(); }
    catch (_) { return; }
    const kind = contentKind(response.headers && response.headers.get('content-type'));
    if (!clone || typeof clone.text !== 'function') return;
    Promise.resolve(clone.text()).then((text) => {
      if (disposed || text.length > MAX_RESPONSE_BYTES) return;
      record('fetch', method, url, response.status, kind, text.length, shapeOf(text));
      recordThreadDirectorySchema(url, text);
      if (expectedMarker && text.includes(expectedMarker)) scheduleMarkerDomObservation();
    }).catch(function () {});
  }

  if (originalFetch) {
    window.fetch = function () {
      const args = arguments;
      const input = args[0];
      const init = args[1] || {};
      let url;
      try { url = new URL(typeof input === 'string' ? input : input.url, location.href); }
      catch (_) { return originalFetch.apply(this, args); }
      const method = init.method || input && input.method || 'GET';
      return Promise.resolve(originalFetch.apply(this, args)).then((response) => {
        inspectFetchResponse(method, url, response);
        return response;
      });
    };
  }

  if (originalOpen && originalSend) {
    xhrPrototype.open = function (method, rawUrl) {
      try { xhrMetadata.set(this, { method, url: new URL(rawUrl, location.href) }); }
      catch (_) { xhrMetadata.delete(this); }
      return originalOpen.apply(this, arguments);
    };
    xhrPrototype.send = function () {
      const metadata = xhrMetadata.get(this);
      if (metadata && metadata.url.origin === location.origin) {
        this.addEventListener('loadend', () => {
          if (disposed || this.status < 200 || this.status >= 300) return;
          const kind = contentKind(this.getResponseHeader('content-type'));
          let text = '';
          try { text = typeof this.responseText === 'string' ? this.responseText : ''; }
          catch (_) { return; }
          if (text.length > MAX_RESPONSE_BYTES) return;
          record('xhr', metadata.method, metadata.url, this.status, kind, text.length, shapeOf(text));
          recordThreadDirectorySchema(metadata.url, text);
          if (expectedMarker && text.includes(expectedMarker)) scheduleMarkerDomObservation();
        }, { once: true });
      }
      return originalSend.apply(this, arguments);
    };
  }

  function observePrompt(prompt) {
    const value = String(prompt || '');
    const match = value.match(/(?:reply exactly with:|请只回复[:：]?|只回复[:：]?)[\s"“”']*([A-Za-z0-9_-]{8,120})/i);
    expectedMarker = match ? match[1] : '';
  }

  window.__elonGoogleWebPrivateResponseTap = Object.freeze({
    version: 1,
    enabled: true,
    observePrompt,
    drain: () => observations.splice(0, observations.length),
    dispose: () => {
      disposed = true;
      if (originalFetch && window.fetch !== originalFetch) window.fetch = originalFetch;
      if (originalOpen && xhrPrototype.open !== originalOpen) xhrPrototype.open = originalOpen;
      if (originalSend && xhrPrototype.send !== originalSend) xhrPrototype.send = originalSend;
    }
  });
})();
