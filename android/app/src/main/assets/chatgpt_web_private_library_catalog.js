(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 8, create: factory });
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
  const ATTACHMENT_FIELDS = ['id', 'kind', 'name', 'file_id', 'mime_type', 'file_size_bytes',
    'parent_directory_id', 'external_account', 'cloud_doc_url', 'library_artifact_type',
    'saved_entity', 'trashed_at', 'is_project', 'gizmo_id', 'project_id', 'context_scopes',
    'preview_file', 'mounted_library_file_id', 'library_file_id', 'shared_library_file_id',
    'library_download_id', 'context_connector_info', 'library_provider'];
  const pages = new Map(), directories = new Map(), renamed = new Map();
  let identityKey = '', active = null, disposed = false, failures = 0, retryAt = 0;

  function sameAttachmentMetadata(left, right) {
    let remaining = 4096;
    function equal(a, b, depth) {
      if (--remaining < 0 || depth > 32) return false;
      if (Object.is(a, b)) return true;
      if (!a || !b || typeof a !== 'object' || typeof b !== 'object' ||
          Array.isArray(a) !== Array.isArray(b)) return false;
      const keys = Object.keys(a);
      if (keys.length !== Object.keys(b).length) return false;
      return keys.every(key => Object.prototype.hasOwnProperty.call(b, key) && equal(a[key], b[key], depth + 1));
    }
    // A fresh JSON response has new object identities; compare only attachment-relevant values.
    return ATTACHMENT_FIELDS.every(key => equal(left[key], right[key], 0));
  }

  function identity(raw = root.__elonChatGptPrivateTransport?.copySameOriginRequestHeaders?.()) {
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
    renamed.clear();
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

  function rememberRename(source, name) {
    const previous = renamed.get(source.id);
    const oldNames = new Set([...(previous?.oldNames || []), source.name]);
    oldNames.delete(name);
    while (oldNames.size > 16) oldNames.delete(oldNames.values().next().value);
    renamed.delete(source.id);
    renamed.set(source.id, { name, oldNames, expiresAt: Date.now() + 10 * 60000 });
    while (renamed.size > 128) renamed.delete(renamed.keys().next().value);
  }

  function reconcileName(source, query) {
    const known = renamed.get(source?.id);
    if (!known) return source;
    if (Date.now() >= known.expiresAt || source.name !== known.name &&
        (!query || !known.oldNames.has(source.name))) {
      renamed.delete(source.id);
      return source;
    }
    // Search indexing can lag a successful rename. Directory reads remain authoritative.
    return query && known.oldNames.has(source.name) ? { ...source, name: known.name } : source;
  }

  function owned(job) {
    return !disposed && active === job && !job.controller.signal.aborted &&
      root.location.origin === 'https://chatgpt.com' && root.location.href === job.href &&
      root.__elonChatGptDocumentToken === job.token && root.__elonChatGptPrivateTransport === job.transport;
  }

  function current(job) { return owned(job) && identity() === job.identity; }

  async function prepareIdentity(job) {
    let timer, abort;
    try {
      // Join the shared identity owner; closing this browser must not abort other consumers' identity reads.
      const headers = await Promise.race([
        job.transport?.acquireSameOriginRequestHeaders?.(),
        new Promise((_, reject) => {
          abort = () => reject(new Error('library_cancelled'));
          job.controller.signal.addEventListener('abort', abort, { once: true });
          timer = root.setTimeout(() => reject(new Error('library_identity_not_ready')), 7000);
        }),
      ]);
      if (!owned(job)) throw new Error('library_cancelled');
      job.identity = identity(headers);
      if (!job.identity) throw new Error('library_identity_not_ready');
      if (!current(job)) throw new Error('library_cancelled');
      identityKey = job.identity;
    } finally {
      root.clearTimeout(timer);
      if (abort) job.controller.signal.removeEventListener('abort', abort);
    }
  }

  function snapshot(job, page, stale, emit) {
    if (!current(job)) return;
    const attachments = root.__elonChatGptPrivateLibraryAttachment?.create(root);
    // IDs, pagination tokens and credential material remain inside this page owner.
    const items = page.items.map(({ source, lookupUrl, observedAt, ...item }) => {
      const downloadHandle = item.kind === 'file'
        ? root.__elonChatGptPrivateFileDownload?.registerLibraryFile?.(source) || '' : '';
      const canAttach = !!attachments?.descriptor(source);
      return { ...item, downloadHandle, canAttach, ...root.__elonChatGptPrivateLibraryMutations?.capabilities?.(source) };
    });
    emit({ type: 'library_files_snapshot', version: 1, requestId: job.requestId,
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
    for (const raw of payload.items) {
      const source = reconcileName(raw, job.query);
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
        sizeBytes: source.file_size_bytes ?? -1, source, lookupUrl: job.lookupUrl, observedAt: Date.now() });
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
    if (disposed || root.location.origin !== 'https://chatgpt.com' ||
        !/^doc_[a-z0-9_]{3,80}$/.test(root.__elonChatGptDocumentToken || '')) {
      return respond(ACTION, false, 'library_identity_not_ready');
    }
    if (root.__elonChatGptPrivateLibraryMutations?.busy?.()) return respond(ACTION, false, 'library_mutation_busy');
    active?.controller.abort();
    const job = { requestId: command.requestId, identity: account, href: root.location.href,
      token: root.__elonChatGptDocumentToken, transport: root.__elonChatGptPrivateTransport,
      directoryHandle, query: query.trim(), deadline: Date.now() + 14000, controller: new root.AbortController() };
    active = job;
    try {
      if (!account) await prepareIdentity(job);
      if (!current(job)) return respond(ACTION, false, 'library_cancelled');
      if (root.__elonChatGptPrivateLibraryMutations?.busy?.()) return respond(ACTION, false, 'library_mutation_busy');
      const directory = directoryHandle ? directories.get(directoryHandle) : { id: null, breadcrumbs: [] };
      if (!directory) return respond(ACTION, false, 'library_selection_expired');
      job.directoryId = directory.id;
      job.breadcrumbs = directory.breadcrumbs;
      const key = JSON.stringify([directory.id, job.query]), cached = pages.get(key);
      const more = operation === 'next';
      if (cached && !more) {
        const stale = operation === 'refresh' || Date.now() - cached.savedAt >= TTL;
        snapshot(job, cached, stale, emit);
        if (!stale) return respond(ACTION, true, 'library_cached');
      }
      if (more && (!cached || !cached.cursor || cached.capped)) return respond(ACTION, false, 'library_page_expired');
      if (Date.now() < retryAt) return respond(ACTION, false, 'library_retry_later');
      const url = new URL('/backend-api/files/library/nodes', root.location.origin);
      if (directory.id != null) url.searchParams.set('parent_directory_id', directory.id);
      if (more) url.searchParams.set('cursor', cached.cursor);
      if (job.query) url.searchParams.set('q', job.query);
      url.searchParams.set('hydrate_folder_thumbnails', 'true');
      url.searchParams.set('include_folder_counts', 'true');
      url.searchParams.set('include_saved_entities', 'true');
      job.lookupUrl = url.href;
      const remaining = job.deadline - Date.now();
      if (remaining <= 0) throw new Error('timeout');
      const result = await root.__elonChatGptPrivateJsonRequest.request(root, url.href, {
        method: 'GET', credentials: 'same-origin', cache: 'no-store', redirect: 'error',
        headers: root.__elonChatGptPrivateTransport.copySameOriginRequestHeaders(), signal: job.controller.signal,
      }, { timeoutMs: Math.min(10000, remaining), maxBytes: 2 * 1024 * 1024 });
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
      if (!owned(job) || job.identity && !current(job)) return respond(ACTION, false, 'library_cancelled');
      if (!job.identity) return respond(ACTION, false, 'library_identity_not_ready');
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
  function selectMutation(fileHandle) {
    const account = identity(), href = root.location.href;
    if (disposed || !account || account !== identityKey || root.location.origin !== 'https://chatgpt.com') return null;
    let item;
    for (const page of pages.values()) {
      if (Date.now() - page.savedAt >= TTL) continue;
      item = page.items.find(row => row.handle === fileHandle);
      if (item) break;
    }
    if (!item || item.kind !== 'file') return null;
    const current = () => !disposed && identity() === account && identityKey === account && root.location.href === href;
    return { source: { ...item.source }, current,
      fresh: () => current() && Array.from(pages.values()).some(page => Date.now() - page.savedAt < TTL && page.items.includes(item)),
      settle(confirmed, operation, name) {
      if (!current()) return;
      if (confirmed && operation === 'rename') rememberRename(item.source, name);
      if (confirmed && operation === 'trash') renamed.delete(item.source.id);
      for (const page of pages.values()) {
        if (confirmed && operation === 'trash') page.items = page.items.filter(row => row.source.id !== item.source.id);
        if (confirmed && operation === 'rename') page.items = page.items.map(row => row.source.id !== item.source.id ? row :
          { ...row, name, source: { ...row.source, name } });
        page.savedAt = 0;
      }
    } };
  }
  function cancelActiveRead() { active?.controller.abort(); active = null; }
  function selectAttachment(fileHandle) {
    const account = identity(), href = root.location.href, document = root.document;
    const transport = root.__elonChatGptPrivateTransport;
    if (disposed || !account || account !== identityKey || root.location.origin !== 'https://chatgpt.com') return null;
    let item;
    for (const page of pages.values()) {
      if (!page.savedAt) continue;
      item = page.items.find(row => row.handle === fileHandle);
      if (item) break;
    }
    if (!item || item.kind !== 'file') return null;
    const owned = () => !disposed && root.location.origin === 'https://chatgpt.com' &&
      identity() === account && identityKey === account && root.location.href === href &&
      root.document === document && root.__elonChatGptPrivateTransport === transport &&
      !root.__elonChatGptPrivateLibraryMutations?.busy?.() &&
      Array.from(pages.values()).some(page => page.savedAt && page.items.includes(item));
    const current = () => owned() && Date.now() - item.observedAt < TTL;
    return { source: { ...item.source }, current, owned,
      async refresh(signal) {
        if (!owned() || signal?.aborted) return false;
        if (current()) return true;
        // Re-read the exact observed page; neither a guessed per-file endpoint nor a renewed write ticket.
        const response = await root.__elonChatGptPrivateJsonRequest.request(root, item.lookupUrl, {
          method: 'GET', credentials: 'same-origin', cache: 'no-store', redirect: 'error',
          headers: transport.copySameOriginRequestHeaders(), signal,
        }, { timeoutMs: 10000, maxBytes: 2 * 1024 * 1024 });
        if (!owned() || signal?.aborted) return false;
        const rows = response.payload?.items;
        if (!Array.isArray(rows) || rows.length > 1000) return false;
        const matches = rows.filter(row => row?.id === item.source.id);
        if (matches.length !== 1 || !sameAttachmentMetadata(matches[0], item.source)) return false;
        item.observedAt = Date.now();
        return true;
      },
    };
  }
  return Object.freeze({ version: 8, list, cancel, dispose, selectMutation, selectAttachment, cancelActiveRead });
});
