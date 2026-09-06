(function (root) {
  'use strict';
  if (root.__binanceGridObserverV1) return;
  const S = root.BinanceGridSanitizer;
  if (!S) return;
  const apply = Reflect.apply;
  const then = Promise.prototype.then;
  const now = Date.now.bind(Date);
  const tasks = new Set();
  const xhrState = new WeakMap();
  let generation = 0;
  let session = null;
  let reason = 'stopped';
  let expiryTimer = null;

  function eligible() {
    try { return root.top === root && location.origin === S.ORIGIN && /^\/(?:[a-z]{2}(?:-[A-Za-z]{2})?\/)?(?:futures(?:\/|$)|trading-bots\/futures\/grid(?:\/|$))/.test(location.pathname); }
    catch (_) { return false; }
  }
  function cancelReader(task) {
    if (task.reader) {
      try { const result = task.reader.cancel(); apply(then, result, [undefined, function () {}]); } catch (_) {}
    }
  }
  function finish(task) {
    if (task.done) return;
    task.done = true;
    clearTimeout(task.timer);
    tasks.delete(task);
    if (task.remove) { try { task.remove(); } catch (_) {} }
  }
  function stop(nextReason) {
    generation++;
    session = null;
    reason = nextReason || 'stopped';
    clearTimeout(expiryTimer);
    for (const task of Array.from(tasks)) { cancelReader(task); finish(task); }
    return status();
  }
  function status() {
    if (session && (!eligible() || location.href !== session.href)) stop('navigated');
    if (session && now() >= session.until) stop('expired');
    return { active: Boolean(session), reason };
  }
  function current(task) {
    status();
    return !task.done && session && task.generation === generation && task.sessionId === session.id;
  }
  function emit(task, outcome, responseShape, responseStatus) {
    if (!current(task)) { finish(task); return; }
    const observation = S.sanitizeObservation({ schema_version: S.SCHEMA_VERSION, method: task.method,
      path: task.path, status: Number.isInteger(responseStatus) ? responseStatus : 0,
      requestShape: task.requestShape, responseShape: responseShape || null, outcome });
    if (observation) {
      try { root.postMessage({ channel: S.CHANNEL, sessionId: task.sessionId, observation }, S.ORIGIN); } catch (_) {}
    }
    finish(task);
  }
  function reserve(meta, requestShape) {
    if (!status().active || !meta || tasks.size >= S.limits.concurrency) return null;
    const task = { ...meta, requestShape, sessionId: session.id, generation, done: false, reader: null, remove: null, responseStatus: 0 };
    tasks.add(task);
    task.timer = setTimeout(function () { cancelReader(task); emit(task, 'timeout', null, task.responseStatus); }, S.limits.readMs);
    return task;
  }
  function dataProperty(object, key) {
    if (!object || (typeof object !== 'object' && typeof object !== 'function')) return { found: false };
    for (let depth = 0; object && depth < 8; depth++, object = Object.getPrototypeOf(object)) {
      const descriptor = Object.getOwnPropertyDescriptor(object, key);
      if (descriptor) return Object.hasOwn(descriptor, 'value') ? { found: true, value: descriptor.value } : { found: true, unsafe: true };
    }
    return { found: false };
  }
  function metadata(url, method) {
    if (typeof method !== 'string' || typeof url !== 'string') return null;
    method = method.toUpperCase();
    if (!['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'].includes(method)) return null;
    const path = S.normalizePath(new URL(url, document.baseURI || location.href).href);
    return path ? { method, path } : null;
  }
  function fetchMeta(args) {
    let url = args[0];
    let method = 'GET';
    if (typeof Request !== 'undefined' && url instanceof Request) {
      method = Object.getOwnPropertyDescriptor(Request.prototype, 'method').get.call(url);
      url = Object.getOwnPropertyDescriptor(Request.prototype, 'url').get.call(url);
    } else if (url instanceof URL) url = url.href;
    const option = dataProperty(args[1], 'method');
    if (option.unsafe) return null;
    if (option.found && option.value !== undefined) method = option.value;
    return metadata(url, method);
  }
  function fetchRequestShape(args) {
    const body = dataProperty(args[1], 'body');
    return body.found && !body.unsafe && typeof body.value === 'string' ? S.shapeFromJson(body.value) : null;
  }
  async function readResponse(task, response) {
    try {
      if (!current(task)) { finish(task); return; }
      task.responseStatus = response.status;
      if (response.url && !S.normalizePath(response.url)) { emit(task, 'unreadable', null, response.status); return; }
      const contentType = response.headers.get('content-type') || '';
      if (!/^(?:application\/json|application\/[a-z0-9.+-]+\+json)(?:\s*;|$)/i.test(contentType)) {
        emit(task, 'non_json', null, response.status); return;
      }
      const clone = response.clone();
      if (!clone.body || typeof clone.body.getReader !== 'function') { emit(task, 'unreadable', null, response.status); return; }
      task.reader = clone.body.getReader();
      let length = 0;
      let text = '';
      const decoder = new TextDecoder('utf-8', { fatal: true });
      while (current(task)) {
        const chunk = await task.reader.read();
        if (!current(task)) { finish(task); return; }
        if (chunk.done) break;
        if (!(chunk.value instanceof Uint8Array)) throw new Error('invalid_chunk');
        length += chunk.value.byteLength;
        if (length > S.limits.bodyBytes) { cancelReader(task); emit(task, 'too_large', null, response.status); return; }
        text += decoder.decode(chunk.value, { stream: true });
      }
      text += decoder.decode();
      const shape = S.shapeFromJson(text);
      emit(task, shape ? 'json' : 'unreadable', shape, response.status);
    } catch (_) { cancelReader(task); emit(task, 'unreadable', null, task.responseStatus); }
  }

  if (typeof root.fetch === 'function') {
    const original = root.fetch;
    root.fetch = function () {
      const args = arguments;
      const startedGeneration = session ? generation : null;
      const result = apply(original, this, args);
      try {
        if (startedGeneration === null || startedGeneration !== generation) return result;
        const meta = fetchMeta(args);
        if (!meta || !status().active || tasks.size >= S.limits.concurrency) return result;
        const task = reserve(meta, fetchRequestShape(args));
        if (task) {
          try {
            apply(then, result, [function (response) { void readResponse(task, response); },
              function () { emit(task, 'network_error', null, 0); }]);
          } catch (_) { finish(task); }
        }
      } catch (_) {}
      return result;
    };
  }
  if (typeof root.XMLHttpRequest === 'function') {
    const prototype = root.XMLHttpRequest.prototype;
    const open = prototype.open;
    const send = prototype.send;
    prototype.open = function () {
      const result = apply(open, this, arguments);
      try {
        const previous = xhrState.get(this);
        if (previous && previous.task) finish(previous.task);
        if (!status().active) { xhrState.delete(this); return result; }
        xhrState.set(this, { meta: metadata(arguments[1], arguments[0]), task: null });
      } catch (_) {}
      return result;
    };
    prototype.send = function () {
      if (!status().active) return apply(send, this, arguments);
      let task = null;
      try {
        const state = xhrState.get(this);
        if (state && state.meta && status().active && tasks.size < S.limits.concurrency) {
          const requestShape = typeof arguments[0] === 'string' ? S.shapeFromJson(arguments[0]) : null;
          task = reserve(state.meta, requestShape);
        }
        if (task) {
          const xhr = this;
          const complete = function () {
            try {
              if (!current(task)) { finish(task); return; }
              task.responseStatus = xhr.status;
              if (xhr.responseURL && !S.normalizePath(xhr.responseURL)) { emit(task, 'unreadable', null, xhr.status); return; }
              if (xhr.responseType !== '' && xhr.responseType !== 'text') { emit(task, 'unreadable', null, xhr.status); return; }
              const type = xhr.getResponseHeader('content-type') || '';
              if (!/^(?:application\/json|application\/[a-z0-9.+-]+\+json)(?:\s*;|$)/i.test(type)) { emit(task, 'non_json', null, xhr.status); return; }
              const text = xhr.responseText;
              if (text.length > S.limits.bodyBytes || new TextEncoder().encode(text).length > S.limits.bodyBytes) { emit(task, 'too_large', null, xhr.status); return; }
              const shape = S.shapeFromJson(text);
              emit(task, shape ? 'json' : 'unreadable', shape, xhr.status);
            } catch (_) { emit(task, 'unreadable', null, task.responseStatus); }
          };
          task.remove = function () { xhr.removeEventListener('loadend', complete); };
          xhr.addEventListener('loadend', complete, { once: true });
          state.task = task;
        }
      } catch (_) { if (task) finish(task); }
      try { return apply(send, this, arguments); }
      catch (error) { if (task) finish(task); throw error; }
    };
  }
  for (const event of ['pagehide', 'popstate', 'hashchange']) {
    root.addEventListener(event, function () { if (session) stop('navigated'); });
  }
  if (root.history) {
    for (const key of ['pushState', 'replaceState']) {
      const original = root.history[key];
      if (typeof original !== 'function') continue;
      root.history[key] = function () {
        const result = apply(original, this, arguments);
        try { status(); } catch (_) {}
        return result;
      };
    }
  }
  root.__binanceGridObserverV1 = Object.freeze({
    start: function (sessionId) {
      stop('stopped');
      if (!eligible()) { reason = 'ineligible_page'; return status(); }
      if (typeof sessionId !== 'string' || !/^[A-Za-z0-9_-]{16,128}$/.test(sessionId)) { reason = 'invalid_session'; return status(); }
      session = { id: sessionId, href: location.href, until: now() + S.limits.lifetimeMs };
      reason = 'capturing';
      expiryTimer = setTimeout(function () { stop('expired'); }, S.limits.lifetimeMs);
      return status();
    },
    stop: function () { return stop('stopped'); },
    status,
  });
})(globalThis);
