(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateDirectoryRefresh = exported;
})(typeof window === 'object' ? window : null, function (root, accept, fetch) {
  'use strict';
  const PATH = '/backend-api/conversations?offset=0&limit=28';
  let active = null;

  function identity(raw) {
    const headers = Object.fromEntries(Object.entries(raw || {}).map(([key, value]) => [key.toLowerCase(), value]));
    if (!/^Bearer\s+\S{8,65536}$/.test(headers.authorization || '')) return '';
    return JSON.stringify(['authorization', 'chatgpt-account-id', 'oai-device-id'].map(key => headers[key] || ''));
  }

  function owned(job) {
    return active === job && !job.controller.signal.aborted && root.location.origin === 'https://chatgpt.com' &&
      root.__elonChatGptDocumentToken === job.document && root.__elonChatGptPrivateTransport === job.transport;
  }

  function current(job) {
    try { return owned(job) && identity(job.transport.copySameOriginRequestHeaders?.()) === job.account; }
    catch (_) { return false; }
  }

  function cancel() { active?.controller.abort(); }

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
      const response = await root.__elonChatGptPrivateJsonRequest.request({
        fetch, AbortController: root.AbortController,
        setTimeout: root.setTimeout.bind(root), clearTimeout: root.clearTimeout.bind(root),
      }, PATH, { method: 'GET', credentials: 'same-origin', cache: 'no-store', headers,
        signal: job.controller.signal }, { timeoutMs: 4000, maxBytes: 1024 * 1024, mode: 'text' });
      if (!current(job)) return { ok: false, code: 'directory_context_changed' };
      // This is the observed first page, not a replacement for all cached pages or project membership.
      if (!accept({ family: 'conversations', projectId: '' }, response.text, false)) {
        return { ok: false, code: 'directory_response_invalid' };
      }
      return { ok: true, code: 'directory_page_ready', complete: false };
    } catch (error) {
      if (!owned(job) || job.account && !current(job)) return { ok: false, code: 'directory_cancelled' };
      const code = String(error?.message || '');
      return { ok: false, code: code === 'http_401' || code === 'identity_not_ready'
        ? 'directory_identity_not_ready' : code === 'timeout' ? 'directory_timeout' : 'directory_refresh_failed' };
    } finally {
      root.clearTimeout(timer);
      if (abort) job.controller.signal.removeEventListener('abort', abort);
      if (active === job) active = null;
    }
  }

  function refresh() {
    if (active && owned(active) && (!active.account || current(active))) return active.promise;
    cancel();
    const transport = root.__elonChatGptPrivateTransport;
    const document = root.__elonChatGptDocumentToken;
    if (root.location.origin !== 'https://chatgpt.com' || !/^doc_[a-z0-9_]{3,80}$/.test(document || '') ||
        typeof transport?.acquireSameOriginRequestHeaders !== 'function' ||
        !root.__elonChatGptPrivateJsonRequest || typeof fetch !== 'function' || typeof root.AbortController !== 'function') {
      return Promise.resolve({ ok: false, code: 'directory_identity_not_ready' });
    }
    let account;
    try { account = identity(transport.copySameOriginRequestHeaders?.()); }
    catch (_) { return Promise.resolve({ ok: false, code: 'directory_identity_not_ready' }); }
    const job = { document, transport, account, controller: new root.AbortController() };
    active = job;
    job.promise = execute(job);
    return job.promise;
  }

  return Object.freeze({ refresh, cancel });
});
