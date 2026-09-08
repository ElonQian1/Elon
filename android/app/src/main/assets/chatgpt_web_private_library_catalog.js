(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com' &&
      Number(root.__elonChatGptPrivateLibraryCatalog?.version || 0) < exported.version) {
    root.__elonChatGptPrivateLibraryCatalog?.dispose?.();
    root.__elonChatGptPrivateLibraryCatalog = factory(root);
  }
})(typeof window === 'object' ? window : null, function (root) {
  'use strict';
  const ACTION = 'list_library_files';
  const HANDLE = /^library_[a-f0-9]{32}$/;
  const REQUEST = /^mcp_[a-z0-9]{1,32}$/;
  const TTL = 60000, MAX_ITEMS = 500, MAX_PAGES = 8;
  const pages = new Map(), directories = new Map();
  let identityKey = '', active = null, disposed = false, failures = 0, retryAt = 0;

  function identity() {
    const raw = root.__elonChatGptPrivateTransport?.copySameOriginRequestHeaders?.();
    const headers = Object.fromEntries(Object.entries(raw || {}).map(([k, v]) => [k.toLowerCase(), v]));
    if (!/^Bearer\s+\S{8,65536}$/.test(headers.authorization || '') ||
        !/^doc_[a-z0-9_]{3,80}$/.test(root.__elonChatGptDocumentToken || '')) return '';
    return JSON.stringify([root.__elonChatGptDocumentToken,
      ...['authorization', 'chatgpt-account-id', 'oai-device-id'].map(k => headers[k] || '')]);
  }

  function reset() {
    active?.controller.abort();
    active = null;
    pages.clear();
    directories.clear();
    failures = 0;
    retryAt = 0;
  }

  function opaque(value, max = 2048) {
    return typeof value === 'string' && value.length > 0 && value.length <= max && !/[\x00-\x20\x7f]/.test(value);
  }

  function label(value) {
    return typeof value === 'string' && value.trim() && value.length <= 1024 && !/[\x00-\x1f\x7f]/.test(value)
      ? value.trim().slice(0, 180) : '';
  }

  function handle() {
    return 'library_' + Array.from(root.crypto.getRandomValues(new Uint8Array(16)),
      value => value.toString(16).padStart(2, '0')).join('');
  }

  function current(job) {
    return !disposed && active === job && !job.controller.signal.aborted &&
      identity() === job.identity && root.location.href === job.href;
  }

  function snapshot(job, page, stale, emit) {
    if (!current(job)) return;
    // IDs, pagination tokens and credential material remain inside this page owner.
    const items = page.items.map(({ source, ...item }) => {
      const downloadHandle = item.kind === 'file'
        ? root.__elonChatGptPrivateFileDownload?.registerLibraryFile?.(source) || '' : '';
      return { ...item, downloadHandle };
    });
    emit('library_files_snapshot', { version: 1, requestId: job.requestId,
      directoryHandle: job.directoryHandle, query: job.query, breadcrumbs: job.breadcrumbs,
      items, hasMore: Boolean(page.cursor) && !page.capped, partial: page.partial || page.capped, stale });
  }

  function normalize(job, payload, previous) {
    if (!payload || !Array.isArray(payload.items) || payload.items.length > 1000 ||
        payload.cursor != null && payload.cursor !== '' && !opaque(payload.cursor, 8192)) {
      throw new Error('library_response_invalid');
    }
    const items = [...(previous?.items || [])], seen = new Set(items.map(item => item.source.id));
    let partial = previous?.partial || false;
    for (const source of payload.items) {
      const name = label(source?.name);
      if (!source || !opaque(source.id) || !name || !['file', 'directory'].includes(source.kind) ||
          source.parent_directory_id != null && !opaque(source.parent_directory_id) ||
          !job.query && job.directoryId != null && source.parent_directory_id != null && source.parent_directory_id !== job.directoryId ||
          source.mime_type != null && source.mime_type !== '' && !/^[A-Za-z0-9.+-]{1,63}\/[A-Za-z0-9.+-]{1,63}$/.test(source.mime_type) ||
          source.file_size_bytes != null && (!Number.isSafeInteger(source.file_size_bytes) || source.file_size_bytes < 0)) {
        partial = true;
        continue;
      }
      if (seen.has(source.id)) continue;
      seen.add(source.id);
      if (items.length >= MAX_ITEMS) { partial = true; break; }
      let id;
      if (source.kind === 'directory') {
        const old = Array.from(directories.entries()).find(([, value]) => value.id === source.id);
        id = old?.[0] || handle();
        if (!old && directories.size >= 1000 || job.breadcrumbs.length >= 32) { partial = true; continue; }
        // Search results may be descendants; this trail is navigation history, not a claimed server parent path.
        directories.set(id, { id: source.id, breadcrumbs: [...job.breadcrumbs, { handle: id, name }] });
      } else id = handle();
      items.push({ handle: id, kind: source.kind, name, mediaType: source.mime_type || '',
        sizeBytes: source.file_size_bytes ?? -1, source });
    }
    const cursors = [...(previous?.cursors || [])];
    const cursor = payload.cursor || '';
    const repeated = cursor && cursors.includes(cursor);
    if (cursor) cursors.push(cursor);
    return { items, cursor: repeated ? '' : cursor, cursors, partial: partial || Boolean(repeated),
      capped: Boolean(cursor) && (items.length >= MAX_ITEMS || cursors.length >= MAX_PAGES), savedAt: Date.now() };
  }

  async function list(command, emit, respond) {
    let input;
    try { input = JSON.parse(command.value || '{}'); } catch (_) { return respond(ACTION, false, 'invalid_library_request'); }
    const operation = input?.operation || 'open', directoryHandle = input?.directoryHandle || '', query = input?.query ?? '';
    if (!REQUEST.test(command.requestId || '') || !['open', 'refresh', 'next'].includes(operation) ||
        directoryHandle !== '' && !HANDLE.test(directoryHandle) ||
        typeof query !== 'string' || query.length > 200 || /[\x00-\x1f\x7f]/.test(query)) {
      return respond(ACTION, false, 'invalid_library_request');
    }
    const account = identity();
    if (account !== identityKey) { reset(); identityKey = account; }
    if (disposed || root.location.origin !== 'https://chatgpt.com' || !account) {
      return respond(ACTION, false, 'library_identity_not_ready');
    }
    const directory = directoryHandle ? directories.get(directoryHandle) : { id: null, breadcrumbs: [] };
    if (!directory) return respond(ACTION, false, 'library_selection_expired');
    active?.controller.abort();
    const job = { requestId: command.requestId, identity: account, href: root.location.href,
      directoryHandle, directoryId: directory.id, query: query.trim(), breadcrumbs: directory.breadcrumbs, controller: new root.AbortController() };
    active = job;
    const key = JSON.stringify([directory.id, job.query]), cached = pages.get(key);
    const more = operation === 'next';
    if (cached && !more) {
      const stale = operation === 'refresh' || Date.now() - cached.savedAt >= TTL;
      snapshot(job, cached, stale, emit);
      if (!stale) { active = null; return respond(ACTION, true, 'library_cached'); }
    }
    if (more && (!cached || !cached.cursor || cached.capped)) {
      active = null;
      return respond(ACTION, false, 'library_page_expired');
    }
    if (Date.now() < retryAt) { active = null; return respond(ACTION, false, 'library_retry_later'); }
    try {
      const url = new URL('/backend-api/files/library/nodes', root.location.origin);
      if (directory.id != null) url.searchParams.set('parent_directory_id', directory.id);
      if (more) url.searchParams.set('cursor', cached.cursor);
      if (job.query) url.searchParams.set('q', job.query);
      url.searchParams.set('hydrate_folder_thumbnails', 'true');
      url.searchParams.set('include_folder_counts', 'true');
      url.searchParams.set('include_saved_entities', 'true');
      const result = await root.__elonChatGptPrivateJsonRequest.request(root, url.href, {
        method: 'GET', credentials: 'same-origin', cache: 'no-store', redirect: 'error',
        headers: root.__elonChatGptPrivateTransport.copySameOriginRequestHeaders(), signal: job.controller.signal,
      }, { timeoutMs: 10000, maxBytes: 2 * 1024 * 1024 });
      if (!current(job)) return respond(ACTION, false, 'library_cancelled');
      const page = normalize(job, result.payload, more ? cached : null);
      pages.delete(key);
      pages.set(key, page);
      while (pages.size > 8) pages.delete(pages.keys().next().value);
      failures = 0;
      retryAt = 0;
      snapshot(job, page, false, emit);
      respond(ACTION, true, page.partial || page.capped ? 'library_partial' : 'library_ready');
    } catch (error) {
      if (!current(job)) return respond(ACTION, false, 'library_cancelled');
      failures += 1;
      if (failures >= 3) retryAt = Date.now() + 30000;
      const code = error?.message === 'http_401' ? 'library_identity_not_ready' :
        error?.message === 'library_response_invalid' ? 'library_response_invalid' : 'library_read_failed';
      respond(ACTION, false, code);
    } finally {
      job.controller.abort();
      if (active === job) active = null;
    }
  }

  function cancel(requestId) {
    if (!active || active.requestId !== requestId) return false;
    active.controller.abort();
    active = null;
    return true;
  }
  function dispose() { disposed = true; reset(); }
  return Object.freeze({ version: 1, list, cancel, dispose });
});
