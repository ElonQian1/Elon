(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 2, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateDirectoryRefresh = exported;
})(typeof window === 'object' ? window : null, function (root, accept, fetch) {
  'use strict';
  const active = new Map();

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

  function cancel() { active.forEach(job => job.controller.abort()); }

  async function readPages(job, scope, headers) {
    const protocol = root.__elonChatGptPrivateDirectoryPages;
    const page = protocol.initial(scope), items = new Map(), visited = new Set();
    const started = Date.now();
    let pages = 0, complete = false, truncated = false;
    try {
      while (pages < 10 && Date.now() - started < 12000) {
        if (!current(job)) throw new Error('directory_context_changed');
        if (visited.has(page.token)) throw new Error('directory_cursor_stalled');
        visited.add(page.token);
        const response = await root.__elonChatGptPrivateJsonRequest.request({
          fetch, AbortController: root.AbortController,
          setTimeout: root.setTimeout.bind(root), clearTimeout: root.clearTimeout.bind(root),
        }, protocol.path(page), { method: 'GET', credentials: 'same-origin', cache: 'no-store', headers,
          signal: job.controller.signal }, {
          timeoutMs: Math.min(4000, Math.max(1, 12000 - (Date.now() - started))),
          maxBytes: 1024 * 1024, mode: 'text',
        });
        if (!current(job)) throw new Error('directory_context_changed');
        const decoded = protocol.decode(page, response.text);
        const before = items.size;
        decoded.items.forEach(item => {
          if (items.has(item.id) || items.size < page.maximum) items.set(item.id, item);
          else truncated = true;
        });
        pages += 1;
        complete = decoded.knownEnd && !truncated;
        if (complete || decoded.next === null || truncated) break;
        if (items.size >= page.maximum) { truncated = true; break; }
        if (before === items.size) throw new Error('directory_cursor_stalled');
        page.token = decoded.next;
      }
      if (!current(job)) throw new Error('directory_context_changed');
      const family = scope === 'projects' ? 'projects' : scope === 'conversations' ? 'conversations' : 'project_conversations';
      const projectId = family === 'project_conversations' ? scope : '';
      // Only a terminal project page is authoritative enough to remove old membership rows.
      if (!accept({ family, projectId, replace: Boolean(projectId && complete) },
          JSON.stringify({ items: Array.from(items.values()) }), false)) throw new Error('directory_response_invalid');
      return { ok: true, complete, truncated, pages, code: complete ? 'directory_ready' : 'directory_partial' };
    } catch (error) {
      // A partial read must not clear cached rows, but already validated pages remain useful.
      if (current(job) && items.size) {
        const family = scope === 'projects' ? 'projects' : scope === 'conversations' ? 'conversations' : 'project_conversations';
        accept({ family, projectId: family === 'project_conversations' ? scope : '', replace: false },
          JSON.stringify({ items: Array.from(items.values()) }), false);
      }
      const code = String(error?.message || '');
      return { ok: false, partial: items.size > 0 && current(job), complete: false, pages,
        code: code === 'http_401' ? 'directory_identity_not_ready' : code === 'timeout' ? 'directory_timeout' :
          code.startsWith('directory_') ? code : 'directory_refresh_failed' };
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
      const account = identity(headers);
      if (!owned(job) || job.account && job.account !== account) return { ok: false, code: 'directory_context_changed' };
      job.account = account;
      if (!account) return { ok: false, code: 'directory_identity_not_ready' };
      if (!current(job)) return { ok: false, code: 'directory_context_changed' };
      if (job.scope !== 'global') return await readPages(job, job.scope, headers);
      const [conversations, projects] = await Promise.all([
        readPages(job, 'conversations', headers), readPages(job, 'projects', headers),
      ]);
      if (!current(job)) return { ok: false, code: 'directory_context_changed' };
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
    const job = { scope, document, transport, account, controller: new root.AbortController() };
    active.set(scope, job);
    job.promise = execute(job);
    return job.promise;
  }

  return Object.freeze({ refresh, cancel });
});
