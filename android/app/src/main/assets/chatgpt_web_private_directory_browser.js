(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateDirectoryBrowser = exported;
})(typeof window === 'object' ? window : null, function (root, fetch, normalize) {
  'use strict';
  const MAX_TICKETS = 32, MAX_CACHED_ROWS = 512, TTL_MS = 600000;
  const ERRORS = new Set(['directory_context_changed', 'directory_identity_not_ready', 'directory_scope_invalid',
    'directory_response_invalid', 'directory_cursor_stalled', 'directory_page_too_large', 'directory_ticket_unavailable']);
  const tickets = new Map();
  const now = () => root.performance?.now?.() ?? Date.now();
  const copy = value => JSON.parse(JSON.stringify(value));
  const failure = code => ({ ok: false, code });
  let active = null;

  function identity(headers) {
    const values = Object.fromEntries(Object.entries(headers || {}).map(([key, value]) => [key.toLowerCase(), value]));
    if (!/^Bearer\s+\S{8,65536}$/.test(values.authorization || '')) return '';
    return JSON.stringify(['authorization', 'chatgpt-account-id', 'oai-device-id'].map(key => values[key] || ''));
  }

  function context() {
    const document = root.__elonChatGptDocumentToken, transport = root.__elonChatGptPrivateTransport;
    if (root.location.origin !== 'https://chatgpt.com' || !/^doc_[a-z0-9_]{3,80}$/.test(document || '') ||
        typeof transport?.acquireSameOriginRequestHeaders !== 'function' ||
        typeof transport.copySameOriginRequestHeaders !== 'function') return null;
    return { document, transport, account: identity(transport.copySameOriginRequestHeaders()) };
  }

  function same(first, second) {
    return first && second && first.document === second.document && first.transport === second.transport &&
      first.account === second.account && Boolean(first.account);
  }

  function owned(job) {
    if (active !== job || job.controller.signal.aborted) return false;
    try {
      const value = context();
      return value && value.document === job.context.document && value.transport === job.context.transport &&
        (!job.context.account || value.account === job.context.account);
    } catch (_) { return false; }
  }

  function prune(value) {
    for (const [key, ticket] of tickets) {
      if (!same(value, ticket.context) || now() - ticket.created >= TTL_MS) tickets.delete(key);
    }
  }

  function saveTicket(value, page, lineage, parent = '') {
    const bytes = new Uint8Array(16);
    root.crypto.getRandomValues(bytes);
    const handle = 'dp_' + Array.from(bytes, byte => byte.toString(16).padStart(2, '0')).join('');
    if (tickets.has(handle)) throw new Error('directory_ticket_unavailable');
    // Eviction drops only a page cache/opaque waypoint, never the production recent index.
    while (tickets.size >= MAX_TICKETS) {
      const key = Array.from(tickets.keys()).find(key => key !== parent);
      if (!key) throw new Error('directory_ticket_unavailable');
      tickets.delete(key);
    }
    tickets.set(handle, { context: value, page, lineage, created: now(), result: null });
    return handle;
  }

  function cacheResult(handle, result) {
    const ticket = tickets.get(handle);
    if (!ticket) return;
    ticket.result = copy(result);
    let count = Array.from(tickets.values()).reduce((n, value) => n + (value.result?.items.length || 0), 0);
    for (const [key, value] of tickets) {
      if (count <= MAX_CACHED_ROWS) break;
      if (key !== handle && value.result) {
        count -= value.result.items.length;
        value.result = null;
      }
    }
  }

  async function acquire(job) {
    let timer, onAbort;
    try {
      const headers = await Promise.race([
        job.context.transport.acquireSameOriginRequestHeaders(),
        new Promise((_, reject) => {
          onAbort = () => reject(new Error('directory_cancelled'));
          job.controller.signal.addEventListener('abort', onAbort, { once: true });
          timer = root.setTimeout(() => reject(new Error('directory_identity_not_ready')), 7000);
        }),
      ]);
      if (!owned(job)) throw new Error('directory_context_changed');
      const account = identity(headers);
      if (!account) throw new Error('directory_identity_not_ready');
      job.context.account = account;
      if (!same(job.context, context())) throw new Error('directory_context_changed');
      return headers;
    } finally {
      root.clearTimeout(timer);
      if (onAbort) job.controller.signal.removeEventListener('abort', onAbort);
    }
  }

  async function execute(job) {
    try {
      const headers = await acquire(job);
      if (!owned(job)) return failure('directory_context_changed');
      prune(job.context);
      const source = job.handle ? tickets.get(job.handle) : null;
      if (job.handle && (!source || source.page.scope !== job.scope)) return failure('directory_page_expired');
      const page = source?.page || root.__elonChatGptPrivateDirectoryPages.initial(job.scope);
      const lineage = source?.lineage || [];
      const response = await root.__elonChatGptPrivateJsonRequest.request({
        fetch, AbortController: root.AbortController,
        setTimeout: root.setTimeout.bind(root), clearTimeout: root.clearTimeout.bind(root),
      }, root.__elonChatGptPrivateDirectoryPages.path(page), {
        method: 'GET', credentials: 'same-origin', cache: 'no-store', headers, signal: job.controller.signal,
      }, { timeoutMs: 8000, maxBytes: 1024 * 1024, mode: 'text' });
      if (!owned(job) || !same(job.context, context())) return failure('directory_context_changed');
      const decoded = root.__elonChatGptPrivateDirectoryPages.decode(page, response.text);
      if (decoded.items.length > 200) throw new Error('directory_page_too_large');
      if (decoded.next !== null && (decoded.next === page.token || lineage.includes(decoded.next))) {
        throw new Error('directory_cursor_stalled');
      }
      // A caller must use the existing canonical directory mapper. Never bridge raw provider rows.
      const items = normalize(job.scope, decoded.items);
      if (!Array.isArray(items) || items.length !== decoded.items.length || items.some((item, index) =>
        !item || item.id !== decoded.items[index].id || typeof item.title !== 'string' || !item.title.trim())) {
        throw new Error('directory_response_invalid');
      }
      if (!owned(job) || !same(job.context, context())) return failure('directory_context_changed');
      const handle = job.handle || saveTicket(job.context, page, []);
      const nextHandle = decoded.next === null ? null : saveTicket(job.context,
        { ...page, token: decoded.next }, [...lineage, page.token].slice(-64), handle);
      const result = { ok: true, code: decoded.knownEnd ? 'directory_page_end' :
        nextHandle ? 'directory_page_ready' : 'directory_page_unknown',
        scope: job.scope, handle, items, nextHandle, complete: decoded.knownEnd, cached: false };
      cacheResult(handle, result);
      return copy(result);
    } catch (error) {
      if (!owned(job)) return failure('directory_cancelled');
      const code = String(error?.message || '');
      return failure(ERRORS.has(code) ? code : code === 'http_401' ? 'directory_identity_not_ready' :
        code === 'timeout' ? 'directory_timeout' : 'directory_page_failed');
    } finally {
      if (active === job) active = null;
    }
  }

  function read(scope, handle = '') {
    if (scope !== 'conversations' && scope !== 'projects' && !/^g-p-[A-Za-z0-9_-]{1,160}$/.test(scope || '')) {
      return Promise.resolve(failure('directory_scope_invalid'));
    }
    if (typeof handle !== 'string' || handle && !/^dp_[a-f0-9]{32}$/.test(handle)) {
      return Promise.resolve(failure('directory_page_expired'));
    }
    let value;
    try { value = context(); } catch (_) { value = null; }
    if (!value || !root.__elonChatGptPrivateDirectoryPages || !root.__elonChatGptPrivateJsonRequest ||
        typeof normalize !== 'function' || typeof fetch !== 'function' || typeof root.AbortController !== 'function' ||
        typeof root.crypto?.getRandomValues !== 'function') return Promise.resolve(failure('directory_reader_unavailable'));
    prune(value);
    const ticket = handle ? tickets.get(handle) : null;
    if (handle && (!ticket || ticket.page.scope !== scope)) return Promise.resolve(failure('directory_page_expired'));
    const cached = ticket?.result;
    if (cached && (!cached.nextHandle || tickets.has(cached.nextHandle))) {
      // A cached selection is still a new user choice and supersedes any older pending choice.
      active?.controller.abort(); active = null;
      return Promise.resolve({ ...copy(cached), cached: true });
    }
    if (active && active.scope === scope && active.handle === handle && owned(active)) return active.promise;
    active?.controller.abort();
    const job = { scope, handle, context: value, controller: new root.AbortController() };
    active = job;
    job.promise = execute(job);
    return job.promise;
  }

  function cancel() {
    active?.controller.abort(); active = null;
    tickets.clear();
  }

  return Object.freeze({ read, cancel });
});
