(function () {
  'use strict';

  if (window.__elonChatGptPrivateStreamObserverEnabled !== true) return;
  if (location.origin !== 'https://chatgpt.com') return;
  const existing = window.__elonChatGptPrivateStreamTransport;
  if (existing && Number(existing.version) >= 6) return;
  if (existing && typeof existing.dispose === 'function') {
    try { existing.dispose(); }
    catch (_) { /* A stale transport must not block the upgraded observer. */ }
  }
  const policy = window.__elonChatGptPrivateStreamPolicy;
  const originalFetch = typeof window.fetch === 'function' ? window.fetch : null;
  const socketTap = window.__elonChatGptPrivateSocketTap;
  if (!policy || !originalFetch || typeof TextDecoder !== 'function') return;

  const session = policy.createSession({ now: Date.now });
  const researchProbe = window.__elonChatGptPrivateResearchProbe;
  const listeners = new Set();
  const MAX_SOCKET_DEPTH = 6;
  const MAX_SOCKET_VALUES = 80;
  const MAX_SOCKET_TEXT_LENGTH = 65536;
  const MAX_PACKED_WIDGET_BYTES = 768 * 1024;
  const MAX_UNPACKED_WIDGET_BYTES = 2 * 1024 * 1024;
  const observedWidgets = new Set();
  let disposed = false;
  let socketUnsubscribe = null;
  let socketFrames = 0;
  let socketFirstReported = false;
  let socketSuccessReported = false;
  let accessSignal = null;
  const ACCESS_SIGNAL_TTL_MS = 2 * 60 * 1000;

  function notify() {
    listeners.forEach((listener) => {
      try { listener(); }
      catch (_) { /* Native snapshot fallback remains active. */ }
    });
  }

  function packedWidgetKey(widget) {
    const value = String(widget && widget.compressed || '');
    return [
      String(widget && widget.messageId || ''),
      String(widget && widget.widgetId || ''),
      value.length,
      value.slice(0, 16),
      value.slice(-16)
    ].join(':').slice(0, 460);
  }

  function base64UrlBytes(value) {
    const source = String(value || '');
    if (!source || source.length > 1024 * 1024 || !/^[A-Za-z0-9_-]+$/.test(source) ||
        typeof atob !== 'function') return null;
    const padding = source.length % 4 ? '='.repeat(4 - source.length % 4) : '';
    let decoded;
    try { decoded = atob(source.replace(/-/g, '+').replace(/_/g, '/') + padding); }
    catch (_) { return null; }
    if (!decoded || decoded.length > MAX_PACKED_WIDGET_BYTES) return null;
    const bytes = new Uint8Array(decoded.length);
    for (let index = 0; index < decoded.length; index += 1) bytes[index] = decoded.charCodeAt(index);
    return bytes;
  }

  async function decodePackedFinance(widget) {
    if (!widget || widget.encoding !== 'gzip-json-base64url-v1' ||
        typeof DecompressionStream !== 'function' || typeof Blob !== 'function') return null;
    const compressed = base64UrlBytes(widget.compressed);
    if (!compressed) return null;
    let reader;
    try {
      reader = new Blob([compressed]).stream()
        .pipeThrough(new DecompressionStream('gzip')).getReader();
      const decoder = new TextDecoder();
      let json = '';
      let total = 0;
      while (true) {
        const chunk = await reader.read();
        if (chunk.done) break;
        total += Number(chunk.value && chunk.value.byteLength || 0);
        if (total > MAX_UNPACKED_WIDGET_BYTES) {
          try { await reader.cancel(); }
          catch (_) { /* The bounded decoder may already be closed. */ }
          return null;
        }
        json += decoder.decode(chunk.value, { stream: true });
      }
      json += decoder.decode();
      const value = JSON.parse(json);
      return typeof policy.financePartFromWidget === 'function'
        ? policy.financePartFromWidget(value)
        : null;
    } catch (_) {
      return null;
    } finally {
      if (reader) {
        try { reader.releaseLock(); }
        catch (_) { /* The decompression stream may already be closed. */ }
      }
    }
  }

  function observePackedFinance(payload) {
    if (typeof policy.packedFinanceWidgets !== 'function' ||
        typeof session.acceptRichParts !== 'function') return;
    const widgets = policy.packedFinanceWidgets(payload).concat(
      typeof session.packedWidgets === 'function' ? session.packedWidgets() : []
    );
    widgets.forEach((widget) => {
      const key = packedWidgetKey(widget);
      if (!key || observedWidgets.has(key)) return;
      if (observedWidgets.size >= 32) observedWidgets.clear();
      observedWidgets.add(key);
      Promise.resolve(decodePackedFinance(widget)).then((part) => {
        if (disposed || !part || !session.acceptRichParts([part], widget)) return;
        recordShape('widget/finance/decoded');
        notify();
      }).catch(() => recordShape('widget/finance/decode_error'));
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

  function compactPath(value) {
    const path = String(value || '');
    if (!path || path.length > 160) return 'none';
    return path.split('/').map((segment) => {
      if (!segment) return '';
      if (/^[a-z][a-z0-9_-]{0,31}$/i.test(segment) && segment.length < 17) return segment;
      if (/^[0-9]{1,4}$/.test(segment)) return '{index}';
      return '{id}';
    }).join('/').slice(0, 96) || 'none';
  }

  function compactShape(payload) {
    if (!payload || typeof payload !== 'object' || Array.isArray(payload)) return '';
    const keys = ['c', 'o', 'p', 'v'].filter((key) =>
      Object.prototype.hasOwnProperty.call(payload, key));
    if (!keys.length) return '';
    const value = payload.v;
    const valueKind = Array.isArray(value) ? 'array' :
      (value === null ? 'null' : typeof value);
    return [
      'compact',
      'c:' + schemaType(typeof payload.c === 'string' ? payload.c : typeof payload.c),
      'o:' + schemaType(typeof payload.o === 'string' ? payload.o : typeof payload.o),
      'p:' + compactPath(payload.p),
      'v:' + valueKind,
      'vk:' + schemaKeys(value)
    ].join('/');
  }

  function recordShape(shape) {
    if (!researchProbe || typeof researchProbe.recordPrivateStreamShape !== 'function') return;
    researchProbe.recordPrivateStreamShape(String(shape || '').slice(0, 120));
  }

  function reportShape(payload, source) {
    const data = payload && typeof payload.data === 'object' ? payload.data : null;
    const message = payload && typeof payload.message === 'object' ? payload.message :
      (data && typeof data.message === 'object' ? data.message : null);
    const content = message && typeof message.content === 'object' ? message.content : null;
    const prefix = source ? source + '/' : '';
    recordShape(prefix + [
      't:' + schemaType(payload && payload.type),
      'k:' + schemaKeys(payload),
      'dt:' + schemaType(data && data.type),
      'dk:' + schemaKeys(data),
      'mk:' + schemaKeys(message),
      'ck:' + schemaKeys(content)
    ].join('/'));
    const compact = compactShape(payload);
    if (compact) recordShape(prefix + compact);
  }

  function reportSocketOutcome(outcome) {
    if (researchProbe && typeof researchProbe.recordPrivateStreamOutcome === 'function') {
      researchProbe.recordPrivateStreamOutcome(outcome, socketFrames, 0);
    }
  }

  function reportAcceptedRealtimeSnapshot() {
    socketFrames += 1;
    if (!socketFirstReported) {
      socketFirstReported = true;
      reportSocketOutcome('first');
    }
    const active = session.current(location.pathname);
    if (active && active.state === 'completed' && !socketSuccessReported) {
      socketSuccessReported = true;
      reportSocketOutcome('success');
    }
    notify();
  }

  function socketLengthBucket(length) {
    if (length < 256) return 'small';
    if (length < 4096) return 'medium';
    return 'large';
  }

  function nestedFieldShape(value) {
    if (value === null) return 'null';
    if (Array.isArray(value)) {
      const types = value.slice(0, 8).map((item) => item === null ? 'null' : typeof item);
      return 'array/count:' + Math.min(value.length, 99) + '/types:' +
        (Array.from(new Set(types)).map(schemaType).join('.') || 'none');
    }
    if (typeof value === 'object') return 'object/k:' + schemaKeys(value);
    if (typeof value === 'string') return 'string/size:' + socketLengthBucket(value.length);
    return schemaType(typeof value);
  }

  function parseSocketText(text) {
    const value = String(text || '').trim();
    if (!value || value.length > MAX_SOCKET_TEXT_LENGTH) return [];
    const candidates = [value];
    if (value.startsWith('data:')) candidates.push(value.slice(5).trim());
    if (value.includes('\n')) {
      value.split(/\r?\n/).map((line) => line.trim()).filter(Boolean)
        .forEach((line) => candidates.push(line.startsWith('data:') ? line.slice(5).trim() : line));
    }
    const parsed = [];
    const seen = new Set();
    candidates.forEach((candidate) => {
      if (!candidate || candidate === '[DONE]' || seen.has(candidate)) return;
      seen.add(candidate);
      try { parsed.push(JSON.parse(candidate)); }
      catch (_) { /* Unknown frames remain owned by the official page. */ }
    });
    return parsed;
  }

  function isCompletedPayload(payload) {
    if (!payload || typeof payload !== 'object') return false;
    const values = [
      payload.type,
      payload.status,
      payload.event,
      payload.data && payload.data.type,
      payload.data && payload.data.status,
      payload.v && payload.v.status,
      payload.v && payload.v.message && payload.v.message.status
    ].map((value) => String(value || '').toLowerCase());
    return values.some((value) => /^(completed|finished|finished_successfully|done|message_stream_complete)$/.test(value));
  }

  function acceptObservedPayload(root, source, exploreAll) {
    const sourcePrefix = source ? source + '/' : '';
    const queue = [{ value: root, depth: 0, exploreAll: exploreAll === true }];
    const seen = new Set();
    let accepted = false;
    let inspected = 0;
    while (queue.length && inspected < MAX_SOCKET_VALUES) {
      const entry = queue.shift();
      const value = entry.value;
      if (!value || seen.has(value)) continue;
      seen.add(value);
      inspected += 1;
      if (typeof value === 'string') {
        if (entry.depth < MAX_SOCKET_DEPTH) {
          parseSocketText(value).forEach((item) =>
            queue.push({ value: item, depth: entry.depth + 1, exploreAll: entry.exploreAll }));
        }
        continue;
      }
      if (typeof value !== 'object') continue;
      reportShape(value, source);
      if (session.accept(value)) accepted = true;
      observePackedFinance(value);
      if (value.author && value.author.role === 'assistant' &&
          session.accept({ message: value })) {
        accepted = true;
        observePackedFinance({ message: value });
      }
      if (entry.depth >= MAX_SOCKET_DEPTH) continue;
      if (Array.isArray(value)) {
        value.slice(0, 24).forEach((item) => queue.push({
          value: item,
          depth: entry.depth + 1,
          exploreAll: entry.exploreAll
        }));
        continue;
      }
      [
        'data',
        'payload',
        'body',
        'event',
        'reply',
        'response',
        'update_content',
        'message',
        'content',
        'v'
      ].forEach((key) => {
        const nested = value[key];
        if ((key === 'reply' || key === 'update_content') && nested != null) {
          const kind = Array.isArray(nested) ? 'array' : typeof nested;
          const size = typeof nested === 'string' ? socketLengthBucket(nested.length) : 'none';
          recordShape(sourcePrefix + 'field:' + key + '/type:' + schemaType(kind) + '/size:' + size);
          if (key === 'update_content') {
            recordShape(sourcePrefix + 'field:update_content/shape:' + nestedFieldShape(nested));
          }
        }
        if (nested && typeof nested === 'object') {
          queue.push({
            value: nested,
            depth: entry.depth + 1,
            exploreAll: entry.exploreAll || key === 'update_content'
          });
        } else if (typeof nested === 'string') {
          queue.push({
            value: nested,
            depth: entry.depth + 1,
            exploreAll: entry.exploreAll || key === 'update_content'
          });
        }
      });
      if (entry.exploreAll) {
        Object.keys(value).slice(0, 24).forEach((key) => {
          const nested = value[key];
          if ((nested && typeof nested === 'object') || typeof nested === 'string') {
            queue.push({ value: nested, depth: entry.depth + 1, exploreAll: true });
          }
        });
      }
      if (isCompletedPayload(value) && session.finish()) accepted = true;
    }
    return accepted;
  }

  function observeSocket(text) {
    if (disposed) return;
    const parsed = parseSocketText(text);
    if (!parsed.length) {
      recordShape('socket/unparsed/size:' + socketLengthBucket(String(text || '').length));
      return;
    }
    let accepted = false;
    parsed.forEach((payload) => {
      if (acceptObservedPayload(payload, 'socket')) accepted = true;
    });
    if (!accepted) return;
    reportAcceptedRealtimeSnapshot();
  }

  function isOfficialConversationStream(method, url, response) {
    if (String(method || 'GET').toUpperCase() !== 'POST') return false;
    if (url.origin !== location.origin ||
        !/^\/(?:backend-api|backend-anon)\/(?:f\/)?conversation(?:\/|$)/.test(url.pathname)) return false;
    if (!response || !response.ok || !response.headers || typeof response.headers.get !== 'function') return false;
    return String(response.headers.get('content-type') || '').toLowerCase()
      .includes('text/event-stream');
  }

  function isOfficialConversationRequest(method, url) {
    return String(method || 'GET').toUpperCase() === 'POST' &&
      url.origin === location.origin &&
      /^\/(?:backend-api|backend-anon)\/(?:f\/)?conversation(?:\/|$)/.test(url.pathname);
  }

  function observeAccessResponse(method, url, response) {
    if (!isOfficialConversationRequest(method, url) || !response) return;
    const status = Number(response.status) || 0;
    const reason = status === 401 || status === 403
      ? 'login_required'
      : status === 429 ? 'rate_limited' : '';
    const next = reason ? { reason, status, observedAt: Date.now() } : null;
    const changed = JSON.stringify(next) !== JSON.stringify(accessSignal);
    accessSignal = next;
    if (changed) notify();
  }

  function currentAccess() {
    if (!accessSignal) return null;
    if (Date.now() - accessSignal.observedAt > ACCESS_SIGNAL_TTL_MS) {
      accessSignal = null;
      return null;
    }
    return Object.assign({}, accessSignal);
  }

  function isOfficialStreamStatus(method, url, response) {
    if (String(method || 'GET').toUpperCase() !== 'GET') return false;
    if (url.origin !== location.origin ||
        !/^\/backend-api\/conversation\/[A-Za-z0-9_-]{1,160}\/stream_status$/.test(url.pathname)) {
      return false;
    }
    if (!response || !response.ok || !response.headers || typeof response.headers.get !== 'function') {
      return false;
    }
    return String(response.headers.get('content-type') || '').toLowerCase().includes('json');
  }

  function observeStreamStatus(response) {
    let clone;
    try { clone = response.clone(); }
    catch (_) { return; }
    if (!clone || typeof clone.json !== 'function') return;
    Promise.resolve(clone.json()).then((payload) => {
      if (disposed || !acceptObservedPayload(payload, 'status', true)) return;
      reportAcceptedRealtimeSnapshot();
    }).catch(function () {
      recordShape('status/parse_error');
    });
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
        observePackedFinance(payload);
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
      observeAccessResponse(method, url, response);
      if (isOfficialConversationStream(method, url, response)) observe(response);
      else if (isOfficialStreamStatus(method, url, response)) observeStreamStatus(response);
      return response;
    });
  };
  window.fetch = wrappedFetch;

  if (socketTap && typeof socketTap.subscribe === 'function') {
    socketUnsubscribe = socketTap.subscribe(observeSocket, true);
  }

  window.__elonChatGptPrivateStreamTransport = Object.freeze({
    version: 6,
    enabled: true,
    current: (pathname) => session.current(pathname),
    access: currentAccess,
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
      observedWidgets.clear();
      if (typeof socketUnsubscribe === 'function') socketUnsubscribe();
      socketUnsubscribe = null;
      accessSignal = null;
      session.reset();
      if (window.fetch === wrappedFetch) window.fetch = originalFetch;
    }
  });
})();
