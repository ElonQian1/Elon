(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com' && !root.__elonChatGptPrivateImageGallery) {
    root.__elonChatGptPrivateImageGallery = factory(root);
  }
})(typeof window === 'object' ? window : null, function (root) {
  'use strict';
  const ACTION = 'sync_private_image_gallery';
  const CANCEL = 'cancel_private_image_gallery';
  const HANDLE = /^image_[a-f0-9]{16}$/;
  const ID = /^[A-Za-z0-9_-]{1,160}$/;
  const TTL_MS = 120000;
  const pages = new Map();
  let cursors = [null], pageIndex = 0, cacheIdentity = '', cacheTime = 0;
  let active = null, disposed = false;

  function identity(headers) {
    const values = {};
    for (const [name, value] of Object.entries(headers || {})) values[name.toLowerCase()] = value;
    if (!/^Bearer\s+\S{8,65536}$/.test(values.authorization || '')) return null;
    return JSON.stringify(['authorization', 'chatgpt-account-id', 'oai-device-id'].map(key => values[key] || ''));
  }

  function current(job) {
    return !disposed && active === job && !job.controller.signal.aborted &&
      root.location.origin === 'https://chatgpt.com' && root.location.href === job.href &&
      root.__elonChatGptDocumentToken === job.token &&
      (!job.account || identity(root.__elonChatGptPrivateTransport?.copySameOriginRequestHeaders?.()) === job.account);
  }

  function check(job) { if (!current(job)) throw new Error('context_changed'); }

  function emit(job, state, page) {
    if (!current(job)) return;
    job.emit({ type: 'image_gallery_snapshot', source: 'private_image_gallery_v1',
      requestId: job.id, state, observedCount: page?.items.length || 0,
      ...(page ? { handles: page.handles, unavailableCount: page.unavailable,
        pageIndex: job.index, hasPrevious: job.index > 0, hasNext: page.cursor !== null } : {}) });
  }

  function release(job) {
    root.clearTimeout(job.timer);
    job.controller.abort();
    for (const handle of job.pending) job.assets?.cancel?.(handle, job.assetListener);
    job.pending.clear();
  }

  function cancel(requestId) {
    if (!active || requestId && active.id !== requestId) return;
    const job = active;
    active = null;
    release(job);
  }

  function clearCache() { pages.clear(); cursors = [null]; pageIndex = 0; cacheTime = 0; }

  async function headersFor(job) {
    const transport = root.__elonChatGptPrivateTransport;
    let source = transport?.copySameOriginRequestHeaders?.();
    if (!identity(source)) {
      let timer;
      try {
        source = await Promise.race([transport?.acquireSameOriginRequestHeaders?.(), new Promise((_, reject) => {
          timer = root.setTimeout(() => reject(new Error('identity_timeout')), 7000);
        })]);
      } finally { root.clearTimeout(timer); }
    }
    check(job);
    job.account = identity(source);
    if (!job.account) throw new Error('identity_unavailable');
    check(job);
    const headers = { Accept: 'application/json' };
    for (const [name, value] of Object.entries(source)) {
      if (['authorization', 'chatgpt-account-id', 'oai-device-id', 'oai-language', 'oai-client-version',
        'oai-client-build-number'].includes(name.toLowerCase())) headers[name] = value;
    }
    return headers;
  }

  async function read(job, url, signal = job.controller.signal) {
    check(job);
    const result = await root.__elonChatGptPrivateJsonRequest.request(root, url, {
      method: 'GET', credentials: 'include', redirect: 'error', cache: 'no-store',
      headers: job.headers, signal, __elonPrivateTransport: 'image_gallery',
    }, { timeoutMs: 6000, maxBytes: 512 * 1024 });
    check(job);
    return result.payload;
  }

  function parsePage(payload, cursor) {
    if (!payload || !Array.isArray(payload.items) || payload.items.length > 25 ||
        payload.items.some(item => !item || typeof item !== 'object' || Array.isArray(item)) ||
        payload.cursor != null && (typeof payload.cursor !== 'string' || !payload.cursor ||
          payload.cursor.length > 2048 || /[\u0000-\u001f\u007f]/.test(payload.cursor)) ||
        payload.cursor != null && (payload.cursor === cursor || !payload.items.length)) {
      throw new Error('catalog_unrecognized');
    }
    return { items: payload.items, cursor: payload.cursor ?? null };
  }

  function imageTarget(item) {
    const pointer = /^(?:file-service|sediment):\/\/([A-Za-z0-9_-]{1,160})$/.exec(item.asset_pointer || '');
    if (!pointer || item.conversation_id != null && !ID.test(item.conversation_id)) return null;
    // Shared/library pointer parameters require their own inspected scope resolver.
    return { fileId: pointer[1], conversationId: item.conversation_id ?? null };
  }

  function register(job, page) {
    const handles = [], seen = new Set();
    let unavailable = 0;
    for (const item of page.items) {
      const target = imageTarget(item);
      if (!target) { unavailable++; continue; }
      const stable = JSON.stringify([job.account, target.fileId, target.conversationId]);
      const handle = job.assets.registerPrivate(stable, async signal => {
        const url = new URL('/backend-api/files/download/' + encodeURIComponent(target.fileId), root.location.origin);
        if (target.conversationId) url.searchParams.set('conversation_id', target.conversationId);
        url.searchParams.set('inline', 'true');
        url.searchParams.set('download_intent', 'false');
        const payload = await read(job, url.href, signal);
        if (payload?.status !== 'success' || typeof payload.download_url !== 'string') {
          throw new Error('image_not_ready');
        }
        return payload.download_url;
      }, () => current(job));
      if (!HANDLE.test(handle)) { unavailable++; continue; }
      if (!seen.has(handle)) { seen.add(handle); handles.push(handle); }
    }
    return { ...page, handles, unavailable };
  }

  async function exportPage(job, page) {
    const missing = page.handles.filter(handle => !job.cached.has(handle));
    let next = 0, failed = page.unavailable;
    async function worker() {
      while (next < missing.length) {
        check(job);
        const handle = missing[next++];
        job.pending.add(handle);
        const result = await job.assets.request(handle, job.assetListener);
        job.pending.delete(handle);
        check(job);
        if (!result?.ok) failed++;
      }
    }
    await Promise.all([worker(), worker()]);
    return failed;
  }

  async function execute(job) {
    try {
      job.headers = await headersFor(job);
      if (!job.assets?.registerPrivate || !root.__elonChatGptPrivateJsonRequest?.request) {
        throw new Error('transport_unavailable');
      }
      const scope = JSON.stringify([job.account, job.token]);
      if (cacheIdentity !== scope || job.operation === 'refresh') {
        clearCache(); cacheIdentity = scope;
      } else if (Date.now() - cacheTime >= TTL_MS) {
        // Keep the user's cursor position; only cached page payloads have expired.
        pages.clear(); cacheTime = 0;
      }
      job.index = job.operation === 'next' ? pageIndex + 1 : job.operation === 'previous' ? pageIndex - 1 : pageIndex;
      if (job.index < 0 || job.index >= cursors.length || job.index >= 256) throw new Error('page_unavailable');
      let page = pages.get(job.index);
      if (!page) {
        const url = new URL('/backend-api/my/recent/image_gen', root.location.origin);
        url.searchParams.set('limit', '25');
        if (cursors[job.index] !== null) url.searchParams.set('after', cursors[job.index]);
        page = parsePage(await read(job, url.href), cursors[job.index]);
        if (page.cursor !== null && cursors.slice(0, job.index + 1).includes(page.cursor)) {
          throw new Error('catalog_cursor_cycle');
        }
        pages.set(job.index, page);
        while (pages.size > 3) pages.delete(pages.keys().next().value);
        if (!cacheTime) cacheTime = Date.now();
      }
      check(job);
      cursors.length = job.index + 1;
      if (page.cursor !== null) cursors.push(page.cursor);
      pageIndex = job.index;
      page = register(job, page);
      job.page = page;
      emit(job, 'loading', page);
      const failed = await exportPage(job, page);
      check(job);
      emit(job, failed ? 'partial' : 'ready', page);
      return { ok: failed === 0, code: failed ? 'image_preview_partial' : 'private_image_gallery_ready' };
    } catch (_) {
      emit(job, job.page ? 'partial' : 'failed', job.page);
      return { ok: false, code: current(job) ? 'private_image_gallery_unavailable' : 'gallery_cancelled' };
    } finally {
      release(job);
      if (active === job) active = null;
    }
  }

  function request(command, emitEvent) {
    if (disposed || typeof emitEvent !== 'function' || !/^mcp_[a-z0-9]{1,32}$/.test(command.requestId || '') ||
        !/^doc_[a-z0-9_]{3,80}$/.test(root.__elonChatGptDocumentToken || '') ||
        root.location.origin !== 'https://chatgpt.com') {
      return Promise.resolve({ ok: false, code: 'gallery_invalid_request' });
    }
    let args;
    try { args = JSON.parse(command.value || '{}'); } catch (_) {}
    if (!['open', 'refresh', 'next', 'previous'].includes(args?.operation) ||
        !Array.isArray(args.cachedHandles) || args.cachedHandles.length > 96 ||
        args.cachedHandles.some(handle => typeof handle !== 'string' || !HANDLE.test(handle))) {
      return Promise.resolve({ ok: false, code: 'gallery_invalid_request' });
    }
    cancel();
    const job = { id: command.requestId, operation: args.operation, cached: new Set(args.cachedHandles),
      href: root.location.href, token: root.__elonChatGptDocumentToken, account: null,
      controller: new root.AbortController(), pending: new Set(), emit: emitEvent,
      assets: root.__elonChatGptImageAssets };
    job.assetListener = event => {
      if (current(job)) emitEvent({ ...event, source: 'private_image_gallery_v1', requestId: job.id });
    };
    active = job;
    job.timer = root.setTimeout(() => { emit(job, job.page ? 'partial' : 'failed', job.page); cancel(job.id); }, 35000);
    emit(job, 'loading');
    return execute(job);
  }

  function handle(action, command, respond, emitEvent) {
    if (action === CANCEL) { cancel(String(command.value || '')); respond(action, true, ''); return true; }
    if (action !== ACTION) return false;
    request(command, emitEvent).then(result => respond(action, result.ok, result.code));
    return true;
  }

  function dispose() { cancel(); disposed = true; clearCache(); cacheIdentity = ''; }
  return Object.freeze({ version: 1, request, handle, cancel, dispose });
});
