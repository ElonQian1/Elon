(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 3, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateDirectoryRefresh = exported;
})(typeof window === 'object' ? window : null, function (root, accept, fetch) {
  'use strict';
  const active = new Map();
  const now = () => root.performance?.now?.() ?? Date.now();
  let continuation = null;
  let lastDiagnostic = null;

  function sameContext(value, job) {
    return value && value.document === job.document && value.transport === job.transport && value.account === job.account;
  }

  function globalCycle(job) {
    if (!sameContext(continuation, job) || now() - continuation.started >= 60000) {
      continuation = { document: job.document, transport: job.transport, account: job.account,
        started: now(), reads: new Map() };
    }
    return continuation;
  }

  function identity(raw) {
    const headers = Object.fromEntries(Object.entries(raw || {}).map(([key, value]) => [key.toLowerCase(), value]));
    if (!/^Bearer\s+\S{8,65536}$/.test(headers.authorization || '')) return '';
    return JSON.stringify(['authorization', 'chatgpt-account-id', 'oai-device-id'].map(key => headers[key] || ''));
  }

  function owned(job) {
    return active.get(job.scope) === job && !job.controller.signal.aborted && root.location.origin === 'https://chatgpt.com' &&
      root.__elonChatGptDocumentToken === job.document && root.__elonChatGptPrivateTransport === job.transport;
  }

  function current(job) {
    try { return owned(job) && identity(job.transport.copySameOriginRequestHeaders?.()) === job.account; }
    catch (_) { return false; }
  }

  function cancel() { continuation = null; active.forEach(job => job.controller.abort()); }

  async function readPages(job, scope, headers) {
    const protocol = root.__elonChatGptPrivateDirectoryPages;
    const family = scope === 'projects' ? 'projects' : scope === 'conversations' ? 'conversations' : 'project_conversations';
    const projectId = family === 'project_conversations' ? scope : '';
    const state = job.cycle?.reads.get(scope) || { page: protocol.initial(scope), items: new Map(),
      visited: new Set(), pages: 0, complete: false, truncated: false, finished: false };
    job.cycle?.reads.set(scope, state);
    const { page, items, visited } = state;
    const started = now(), resumedPages = state.pages;
    let requests = 0, requestStarted = started, lastRequestMs = 0, result;
    try {
      while (!state.finished && state.pages < 10 && now() - started < 12000) {
        if (!current(job)) throw new Error('directory_context_changed');
        if (visited.has(page.token)) throw new Error('directory_cursor_stalled');
        requestStarted = now();
        requests += 1;
        const response = await root.__elonChatGptPrivateJsonRequest.request({
          fetch, AbortController: root.AbortController,
          setTimeout: root.setTimeout.bind(root), clearTimeout: root.clearTimeout.bind(root),
        }, protocol.path(page), { method: 'GET', credentials: 'same-origin', cache: 'no-store', headers,
          signal: job.controller.signal }, {
          // Admission is batch-bounded; an admitted page still gets its full request deadline.
          timeoutMs: 4000,
          maxBytes: 1024 * 1024, mode: 'text',
        });
        lastRequestMs = now() - requestStarted;
        if (!current(job)) throw new Error('directory_context_changed');
        const decoded = protocol.decode(page, response.text);
        visited.add(page.token);
        const before = items.size;
        decoded.items.forEach(item => {
          if (items.has(item.id) || items.size < page.maximum) items.set(item.id, item);
          else state.truncated = true;
        });
        state.pages += 1;
        state.complete = decoded.knownEnd && !state.truncated;
        state.finished = state.complete || decoded.next === null || state.truncated;
        if (state.finished) break;
        if (items.size >= page.maximum) { state.truncated = true; state.finished = true; break; }
        if (before === items.size) throw new Error('directory_cursor_stalled');
        page.token = decoded.next;
      }
      if (state.pages >= 10 && !state.finished) { state.truncated = true; state.finished = true; }
      if (!current(job)) throw new Error('directory_context_changed');
      // Only a terminal project page is authoritative enough to remove old membership rows.
      if (!accept({ family, projectId, replace: Boolean(projectId && state.complete) },
          JSON.stringify({ items: Array.from(items.values()) }), false)) throw new Error('directory_response_invalid');
      result = { ok: true, complete: state.complete, truncated: state.truncated, pages: state.pages,
        code: state.complete ? 'directory_ready' : 'directory_partial' };
      return result;
    } catch (error) {
      // A partial read must not clear cached rows, but already validated pages remain useful.
      if (current(job) && items.size) {
        accept({ family, projectId: family === 'project_conversations' ? scope : '', replace: false },
          JSON.stringify({ items: Array.from(items.values()) }), false);
      }
      const code = String(error?.message || '');
      // Retain only a retryable read, never a bad cursor/schema or a different identity.
      if (!current(job) || !(code === 'timeout' || /^http_(429|5\d\d)$/.test(code))) job.cycle?.reads.delete(scope);
      result = { ok: false, partial: items.size > 0 && current(job), complete: false, pages: state.pages,
        code: code === 'http_401' ? 'directory_identity_not_ready' : code === 'timeout' ? 'directory_timeout' :
          code.startsWith('directory_') ? code : 'directory_refresh_failed' };
      lastRequestMs = requests ? now() - requestStarted : 0;
      return result;
    } finally {
      const boundedMs = value => Math.min(60000, Math.max(0, Math.round(value)));
      job.diagnostics.reads.push({ family, pages: state.pages, resumedPages, requests,
        elapsedMs: boundedMs(now() - started), lastRequestMs: boundedMs(lastRequestMs),
        ok: result?.ok === true, complete: result?.complete === true, truncated: state.truncated,
        code: result?.code || 'directory_refresh_failed' });
    }
  }

  async function execute(job) {
    let timer, abort;
    try {
      const headers = await Promise.race([
        job.transport.acquireSameOriginRequestHeaders(),
        new Promise((_, reject) => {
          abort = () => reject(new Error('cancelled'));
          job.controller.signal.addEventListener('abort', abort, { once: true });
          timer = root.setTimeout(() => reject(new Error('identity_not_ready')), 7000);
        }),
      ]);
      root.clearTimeout(timer);
      job.diagnostics.identityMs = Math.min(60000, Math.max(0, Math.round(now() - job.started)));
      const account = identity(headers);
      if (!owned(job) || job.account && job.account !== account) return { ok: false, code: 'directory_context_changed' };
      job.account = account;
      if (!account) return { ok: false, code: 'directory_identity_not_ready' };
      if (!current(job)) return { ok: false, code: 'directory_context_changed' };
      if (job.scope !== 'global') return await readPages(job, job.scope, headers);
      job.cycle = globalCycle(job);
      const [conversations, projects] = await Promise.all([
        readPages(job, 'conversations', headers), readPages(job, 'projects', headers),
      ]);
      if (!current(job)) return { ok: false, code: 'directory_context_changed' };
      if (continuation === job.cycle && [...job.cycle.reads.values()].length === 2 &&
          [...job.cycle.reads.values()].every(read => read.finished)) continuation = null;
      const ok = conversations.ok && projects.ok;
      return { ok, partial: conversations.ok || conversations.partial || projects.ok || projects.partial,
        code: ok ? (conversations.complete && projects.complete ? 'directory_ready' : 'directory_partial') :
          (!conversations.ok ? conversations.code : projects.code),
        // The global cache includes separately fetched project contents and must never be replaced wholesale.
        complete: false, conversationsComplete: conversations.complete, projectsComplete: projects.complete,
        truncated: conversations.truncated || projects.truncated, pages: conversations.pages + projects.pages };
    } catch (error) {
      if (!owned(job) || job.account && !current(job)) return { ok: false, code: 'directory_cancelled' };
      const code = String(error?.message || '');
      return { ok: false, code: code === 'http_401' || code === 'identity_not_ready'
        ? 'directory_identity_not_ready' : code === 'timeout' ? 'directory_timeout' : 'directory_refresh_failed' };
    } finally {
      if (current(job)) {
        job.diagnostics.durationMs = Math.min(60000, Math.max(0, Math.round(now() - job.started)));
        lastDiagnostic = { document: job.document, transport: job.transport, account: job.account, value: job.diagnostics };
      }
      root.clearTimeout(timer);
      if (abort) job.controller.signal.removeEventListener('abort', abort);
      if (active.get(job.scope) === job) active.delete(job.scope);
    }
  }

  function refresh(scope = 'global') {
    if (scope !== 'global' && !/^g-p-[A-Za-z0-9_-]{1,160}$/.test(scope || '')) {
      return Promise.resolve({ ok: false, code: 'directory_scope_invalid' });
    }
    const previous = active.get(scope);
    if (previous && owned(previous) && (!previous.account || current(previous))) return previous.promise;
    previous?.controller.abort();
    const transport = root.__elonChatGptPrivateTransport;
    const document = root.__elonChatGptDocumentToken;
    if (root.location.origin !== 'https://chatgpt.com' || !/^doc_[a-z0-9_]{3,80}$/.test(document || '') ||
        typeof transport?.acquireSameOriginRequestHeaders !== 'function' ||
        !root.__elonChatGptPrivateJsonRequest || !root.__elonChatGptPrivateDirectoryPages ||
        typeof fetch !== 'function' || typeof root.AbortController !== 'function') {
      return Promise.resolve({ ok: false, code: 'directory_identity_not_ready' });
    }
    let account;
    try { account = identity(transport.copySameOriginRequestHeaders?.()); }
    catch (_) { return Promise.resolve({ ok: false, code: 'directory_identity_not_ready' }); }
    const job = { scope, document, transport, account, controller: new root.AbortController(), started: now(),
      diagnostics: { schema: 'elon.directory_refresh.v1', observed: true, durationMs: 0, identityMs: 0, reads: [] } };
    active.set(scope, job);
    job.promise = execute(job);
    return job.promise;
  }

  function diagnostics() {
    const context = { document: root.__elonChatGptDocumentToken, transport: root.__elonChatGptPrivateTransport };
    try { context.account = identity(context.transport?.copySameOriginRequestHeaders?.()); } catch (_) {}
    return sameContext(lastDiagnostic, context) ? JSON.parse(JSON.stringify(lastDiagnostic.value)) :
      { schema: 'elon.directory_refresh.v1', observed: false, durationMs: 0, identityMs: 0, reads: [] };
  }

  return Object.freeze({ refresh, cancel, diagnostics });
});
