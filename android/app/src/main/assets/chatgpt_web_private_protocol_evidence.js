(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root) root.__elonChatGptPrivateProtocolEvidence = exported;
})(typeof window === 'object' ? window : null, function (root, safePath) {
  'use strict';
  const SCHEMA = 'elon.private_protocol_probe.v1';
  const MAX_RECORDS = 12;
  const MAX_FIELDS = 12;
  const MAX_BODY = 65536;
  const MAX_OUTPUT = 12000;
  const records = [];
  const readers = new Set();
  let epoch = 0;
  let timer = null;
  let running = false;
  let dropped = 0;
  let expiresAt = 0;
  const now = () => (root.Date || Date).now();

  function active() {
    if (running && now() >= expiresAt) stop();
    return running;
  }

  function fields(value) {
    const result = [];
    const seen = new Set();
    function visit(node, path, depth) {
      if (result.length >= MAX_FIELDS || depth > 3 || path.length > 72) return;
      const kind = node === null ? 'null' : Array.isArray(node) ? 'array' : typeof node;
      if (!/^(null|array|object|string|number|boolean)$/.test(kind)) return;
      result.push(path + ':' + kind);
      if (!node || typeof node !== 'object' || seen.has(node)) return;
      seen.add(node);
      if (Array.isArray(node)) {
        if (node.length) visit(node[0], path + '[]', depth + 1);
      } else {
        Object.keys(node).slice(0, 32).forEach((key) => {
          if (!/^[a-zA-Z][a-zA-Z0-9_]{0,39}$/.test(key) ||
              /^(?:__proto__|constructor|prototype)$/.test(key)) return;
          const next = path + '.' + key;
          if (next.length <= 72) visit(node[key], next, depth + 1);
        });
      }
    }
    visit(value, '$', 0);
    return result;
  }

  function kind(contentType) {
    const value = String(contentType || '').toLowerCase();
    if (value.includes('json')) return 'json';
    if (value.includes('multipart/form-data')) return 'multipart';
    if (value.includes('text/event-stream')) return 'stream';
    return value ? 'other' : 'unknown';
  }

  function current(record) {
    return active() && record.epoch === epoch && records.includes(record);
  }

  function textFields(record, side, text) {
    if (!current(record)) return;
    if (typeof text !== 'string') { record[side + 'State'] = 'unavailable'; return; }
    if (text.length > MAX_BODY ||
        new TextEncoder().encode(text).byteLength > MAX_BODY) {
      record[side + 'State'] = 'oversize';
      return;
    }
    try {
      record[side + 'Fields'] = fields(JSON.parse(text));
      record[side + 'State'] = 'ready';
    } catch (_) { record[side + 'State'] = 'invalid'; }
  }

  function readBody(record, side, response) {
    if (!current(record)) return;
    const reader = root.__elonChatGptPrivateJsonRequest;
    if (!reader || typeof root.AbortController !== 'function') {
      record[side + 'State'] = 'unavailable';
      try { Promise.resolve(response.body?.cancel()).catch(() => {}); } catch (_) {}
      return;
    }
    const controller = new root.AbortController();
    readers.add(controller);
    record[side + 'State'] = 'pending';
    // Consume only a clone. The original request and response remain the website's.
    const bodyOnly = { ok: true, status: response.status, headers: response.headers,
      body: response.body, text: response.text?.bind(response) };
    reader.request({ fetch: async () => bodyOnly, AbortController: root.AbortController,
      setTimeout: root.setTimeout.bind(root), clearTimeout: root.clearTimeout.bind(root)
    }, '/protocol-observation', { signal: controller.signal }, {
      timeoutMs: 2000, maxBytes: MAX_BODY, mode: 'text'
    }).then((value) => textFields(record, side, value.text)).catch((error) => {
      if (!current(record)) return;
      const reason = String(error && error.message || '');
      record[side + 'State'] = reason === 'response_too_large' ? 'oversize' :
        reason === 'timeout' ? 'timeout' : 'unavailable';
    }).finally(() => readers.delete(controller));
  }

  function stop() {
    running = false;
    epoch += 1;
    if (timer !== null) root.clearTimeout(timer);
    timer = null;
    readers.forEach((controller) => controller.abort());
    readers.clear();
    records.forEach((record) => ['request', 'response'].forEach((side) => {
      if (record[side + 'State'] === 'pending') record[side + 'State'] = 'cancelled';
    }));
  }

  function start() {
    stop();
    records.length = 0;
    dropped = 0;
    running = true;
    expiresAt = now() + 60000;
    timer = root.setTimeout(stop, 60000);
  }

  function begin(input, init, url, method, transport) {
    if (!active()) return null;
    // Observed telemetry bursts must not consume the bounded business capture.
    if (/^\/ces\/v1\/(?:rgstr|t|telemetry\/intake)\/?$/.test(url.pathname)) return null;
    if (records.length >= MAX_RECORDS) { dropped = Math.min(999, dropped + 1); return null; }
    const verb = String(method || 'GET').toUpperCase();
    if (!/^(GET|POST|PUT|PATCH|DELETE)$/.test(verb)) return null;
    const record = { epoch, id: records.length + 1, method: verb, path: safePath(url),
      transport, status: 0, requestKind: 'unknown', responseKind: 'unknown',
      requestState: 'skipped', responseState: 'pending', requestFields: [], responseFields: [] };
    records.push(record);
    const body = init && init.body;
    if (typeof body === 'string') {
      record.requestKind = 'json';
      textFields(record, 'request', body);
    } else if (typeof root.FormData === 'function' && body instanceof root.FormData) {
      record.requestKind = 'multipart';
      record.requestState = 'ready';
      let visited = 0;
      for (const [key, value] of body.entries()) {
        if (++visited > 32 || record.requestFields.length >= MAX_FIELDS) break;
        if (/^[a-zA-Z][a-zA-Z0-9_]{0,39}$/.test(key)) {
          record.requestFields.push('$.' + key + ':' + (typeof value === 'string' ? 'string' : 'file'));
        }
      }
    } else if (input && typeof input.clone === 'function') {
      record.requestKind = kind(input.headers && input.headers.get('content-type'));
      if (record.requestKind === 'json' && typeof root.Response === 'function') {
        try { readBody(record, 'request', new root.Response(input.clone().body)); }
        catch (_) { record.requestState = 'unavailable'; }
      }
    }
    return record;
  }

  function response(record, value) {
    if (!record || !current(record)) return;
    record.status = Math.max(0, Math.min(599, Number(value && value.status) || 0));
    record.responseKind = kind(value && value.headers && value.headers.get('content-type'));
    record.responseState = 'skipped';
    if (record.responseKind !== 'json' || !value || typeof value.clone !== 'function') return;
    try { readBody(record, 'response', value.clone()); }
    catch (_) { record.responseState = 'unavailable'; }
  }

  function xhrResponse(record, status, contentType, text) {
    if (!record || !current(record)) return;
    record.status = Math.max(0, Math.min(599, Number(status) || 0));
    record.responseKind = kind(contentType);
    record.responseState = 'skipped';
    if (record.responseKind === 'json') textFields(record, 'response', text);
  }

  function snapshot() {
    const value = { schema: SCHEMA, active: active(), dropped,
      records: records.map(({ epoch: ignored, ...record }) => record) };
    // Preserve whole records rather than cutting JSON or exposing raw protocol data.
    while (JSON.stringify(value).length > MAX_OUTPUT && value.records.length) {
      value.records.pop();
      value.dropped = Math.min(999, value.dropped + 1);
    }
    return value;
  }

  function command(value) {
    if (value === 'start') start();
    else if (value === 'stop') stop();
    else if (value === 'clear') { stop(); records.length = 0; dropped = 0; }
    else if (value !== 'read') return null;
    return JSON.stringify(snapshot());
  }

  return Object.freeze({ active, begin, response, xhrResponse, command, stop });
});
