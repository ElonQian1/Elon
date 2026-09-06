(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 3, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com' &&
      Number(root.__elonChatGptPrivateFileDownload?.version || 0) < exported.version) {
    root.__elonChatGptPrivateFileDownload?.dispose?.();
    root.__elonChatGptPrivateFileDownload = factory(root);
  }
})(typeof window === 'object' ? window : null, function (root) {
  'use strict';
  const entries = new Map();
  const PATH = /^(?:\/g\/(g-p-[a-f0-9]{32})(?:-[A-Za-z0-9_-]{1,124})?)?\/c\/([A-Za-z0-9_-]{1,160})$/i;
  const PROJECT = /^g-p-[a-f0-9]{32}$/i;
  const LIBRARY = /^libfile[_-][A-Za-z0-9_-]{1,152}$/;
  const HANDLE = /^download_[a-f0-9]{32}$/;
  const ACTION = 'download_conversation_file';
  let active = null;
  let disposed = false;

  function identity() {
    const headers = root.__elonChatGptPrivateTransport?.copySameOriginRequestHeaders?.();
    const values = {};
    for (const [key, value] of Object.entries(headers || {})) values[key.toLowerCase()] = value;
    if (!/^Bearer\s+\S{8,65536}$/.test(values.authorization || '')) return null;
    return JSON.stringify(['authorization', 'chatgpt-account-id', 'oai-device-id'].map(key => values[key] || ''));
  }

  function authorizationUrl(entry, projectId) {
    const url = new URL('/backend-api/files/download/' + encodeURIComponent(entry.fileId), root.location.origin);
    if (projectId) url.searchParams.set('gizmo_id', projectId);
    url.searchParams.set(entry.image || entry.projectId || entry.libraryFileId
      ? 'check_context_scopes_for_conversation_id' : 'conversation_id', entry.conversationId);
    url.searchParams.set('download_intent', 'true');
    return url.href;
  }

  function imageFile(source) {
    const image = source.image;
    if (image?.content_type !== 'image_asset_pointer' || typeof image.asset_pointer !== 'string' ||
        source.attachmentsUnconfirmed || !Array.isArray(source.attachments)) return null;
    const pointer = /^(?:file-service|sediment):\/\/([A-Za-z0-9_-]{1,160})$/.exec(image.asset_pointer);
    if (!pointer || ['gizmo_id', 'project_id', 'library_file_id', 'shared_library_file_id',
      'library_download_id', 'context_scopes', 'source_url', 'context_connector', 'connector_id',
      'context_connector_info'].some(key => image[key] != null)) return null;
    // Official jW/A5t pair image pointers with attachment metadata before pEt/fEt.
    const matches = source.attachments.filter(file => file?.id === pointer[1]);
    if (matches.length > 1 || matches[0]?.mime_type != null &&
        (typeof matches[0].mime_type !== 'string' || !/^image\/[A-Za-z0-9.+-]{1,63}$/.test(matches[0].mime_type))) return null;
    return { ...matches[0], id: pointer[1] };
  }

  function target(path, source, scope) {
    const conversation = PATH.exec(path || '');
    const image = source?.image != null;
    const file = image ? imageFile(source) : source?.attachment;
    if (!conversation || !/^[A-Za-z0-9_-]{1,160}$/.test(file?.id || '')) return null;
    // Shared-library and connector lanes have separate content resolvers.
    if (['shared_library_file_id', 'library_download_id', 'source_url',
      'context_connector', 'connector_id', 'context_connector_info'].some(key => file[key] != null && file[key] !== '')) return null;
    if (scope?.context_scopes != null && (!Array.isArray(scope.context_scopes) || scope.context_scopes.length)) return null;
    if (file.context_scopes != null && (!Array.isArray(file.context_scopes) || file.context_scopes.length)) return null;
    const projects = [conversation[1], source.projectId, scope?.gizmo_id, scope?.project_id,
      file.gizmo_id, file.project_id].filter(value => value != null && value !== '');
    if (projects.some(value => typeof value !== 'string' || !PROJECT.test(value)) || new Set(projects).size > 1) return null;
    const libraryFileId = file.library_file_id == null || file.library_file_id === '' ? null : file.library_file_id;
    if (libraryFileId !== null && (typeof libraryFileId !== 'string' || !LIBRARY.test(libraryFileId))) return null;
    return Object.freeze({ conversationId: conversation[2], fileId: file.id,
      projectId: projects[0] || null, libraryFileId,
      ...(image ? { image: true,
        name: typeof file.name === 'string' && file.name.trim() ? file.name.replace(/\u00a0/g, ' ').trim().slice(0, 180) : 'image.png',
        mediaType: file.mime_type || '' } : {}) });
  }

  async function resolveAuthorization(job, request) {
    const entry = job.entry;
    if (!entry.libraryFileId) return authorizationUrl(entry, entry.projectId);
    // Official WTt/KTt/DX resolve a library file's effective project before dEt.
    const url = new URL('/backend-api/files/' + encodeURIComponent(entry.fileId) + '/simple', root.location.origin);
    if (entry.projectId) url.searchParams.set('gizmo_id', entry.projectId);
    url.searchParams.set('conversation_id', entry.conversationId);
    const result = await request.request(root, url.href, {
      method: 'GET', credentials: 'same-origin', cache: 'no-store', redirect: 'error',
      headers: root.__elonChatGptPrivateTransport.copySameOriginRequestHeaders(), signal: job.controller.signal,
    }, { timeoutMs: 6000, maxBytes: 65536 });
    if (!current(job)) throw new Error('download_cancelled');
    const info = result.payload;
    if (info?.is_library_file !== true || info.library_file_id !== entry.libraryFileId ||
        info.file_id != null && info.file_id !== entry.fileId ||
        info.is_project !== undefined && typeof info.is_project !== 'boolean' ||
        info.gizmo_id != null && (typeof info.gizmo_id !== 'string' || !PROJECT.test(info.gizmo_id))) {
      throw new Error('download_scope_unconfirmed');
    }
    const projectId = info.is_project === true || PROJECT.test(info.gizmo_id || '')
      ? info.gizmo_id || entry.projectId : null;
    if (info.is_project === true && !projectId) throw new Error('download_scope_unconfirmed');
    return authorizationUrl(entry, projectId);
  }

  function register(path, payload, index) {
    for (const [key, entry] of entries) {
      if (entry.path === path || entry.expiresAt <= Date.now()) entries.delete(key);
    }
    const account = identity(), token = root.__elonChatGptDocumentToken;
    const projection = root.__elonChatGptPrivateHistoryProjection?.create({});
    const scope = projection?.normalize?.(payload);
    return index.files.map(row => {
      if (disposed || !account || !/^doc_[a-z0-9_]{3,80}$/.test(token || '') ||
          !root.elonChatGptFileDownload || !projection?.fileSource || !projection?.normalize) return row;
      const request = target(path, projection.fileSource(payload, row.id), scope);
      if (!request) return row;
      const bytes = root.crypto.getRandomValues(new Uint8Array(16));
      const handle = 'download_' + Array.from(bytes, value => value.toString(16).padStart(2, '0')).join('');
      const name = request.name || row.name;
      entries.set(handle, { ...request, path, name, account, token, expiresAt: Date.now() + 120000 });
      while (entries.size > 800) entries.delete(entries.keys().next().value);
      return { ...row, name, ...(request.image ? { mediaType: request.mediaType } : {}), downloadHandle: handle };
    });
  }

  function current(job) {
    return !disposed && active === job && !job.controller.signal.aborted &&
      root.location.href === job.descriptor.href && root.__elonChatGptDocumentToken === job.entry.token &&
      identity() === job.entry.account && job.entry.expiresAt > Date.now();
  }

  function downloadUrl(value) {
    try {
      const url = new URL(value);
      if (url.protocol === 'https:' && /^[a-z0-9][a-z0-9.-]*\.oaiusercontent\.com$/.test(url.hostname) &&
          !url.port && !url.username && !url.password && !url.hash && url.href.length <= 16384) return url.href;
    } catch (_) {}
    throw new Error('download_source_unsupported');
  }

  function enqueue(job, url) {
    const bridge = root.elonChatGptFileDownload;
    if (!bridge?.postMessage) return Promise.reject(new Error('download_bridge_unavailable'));
    return new Promise((resolve, reject) => {
      const previous = bridge.onmessage;
      let settled = false;
      const finish = (error, value) => {
        if (settled) return;
        settled = true;
        root.clearTimeout(timer);
        job.controller.signal.removeEventListener('abort', cancelled);
        if (bridge.onmessage === listener) bridge.onmessage = previous;
        if (error) reject(new Error(error)); else resolve(value);
      };
      const cancelled = () => finish('download_cancelled');
      const listener = event => {
        let data;
        try { data = JSON.parse(event.data); } catch (_) { return; }
        if (data?.leaseId !== job.descriptor.leaseId) return;
        if (!current(job)) return finish('download_cancelled');
        finish(data.state === 'queued' ? '' : 'download_enqueue_failed', data);
      };
      const timer = root.setTimeout(() => finish('download_confirmation_unknown'), 5000);
      bridge.onmessage = listener;
      job.controller.signal.addEventListener('abort', cancelled, { once: true });
      try {
        bridge.postMessage(JSON.stringify({ leaseId: job.descriptor.leaseId,
          documentToken: job.entry.token, url }));
      } catch (_) { finish('download_enqueue_failed'); }
    });
  }

  function abandon(descriptor) {
    if (!/^[a-f0-9-]{36}$/.test(descriptor?.leaseId || '')) return;
    try {
      root.elonChatGptFileDownload?.postMessage(JSON.stringify({ leaseId: descriptor.leaseId,
        documentToken: descriptor.documentToken, cancel: true }));
    } catch (_) {}
  }

  async function start(raw, respond) {
    let descriptor;
    try { descriptor = JSON.parse(raw); } catch (_) { return respond(ACTION, false, 'invalid_file_request'); }
    if (active && !current(active)) { active.controller.abort(); active = null; }
    if (active) {
      if (descriptor?.leaseId !== active.descriptor.leaseId) abandon(descriptor);
      return respond(ACTION, false, 'download_busy');
    }
    const entry = entries.get(descriptor?.downloadHandle);
    if (disposed || !HANDLE.test(descriptor?.downloadHandle || '') || !entry ||
        descriptor.version !== 1 || descriptor.path !== entry.path || descriptor.name !== entry.name ||
        descriptor.documentToken !== entry.token || descriptor.href !== root.location.href ||
        !/^[a-f0-9-]{36}$/.test(descriptor.leaseId || '') || entry.expiresAt <= Date.now() || identity() !== entry.account) {
      abandon(descriptor);
      return respond(ACTION, false, 'download_selection_expired');
    }
    const job = { descriptor, entry, controller: new root.AbortController() };
    active = job;
    const timer = root.setTimeout(() => job.controller.abort(), 15000);
    try {
      const request = root.__elonChatGptPrivateJsonRequest;
      if (!request?.request || !current(job)) throw new Error('download_cancelled');
      const url = await resolveAuthorization(job, request);
      if (!current(job)) throw new Error('download_cancelled');
      const result = await request.request(root, url, {
        method: 'GET', credentials: 'same-origin', cache: 'no-store', redirect: 'error',
        headers: root.__elonChatGptPrivateTransport.copySameOriginRequestHeaders(), signal: job.controller.signal,
      }, { timeoutMs: 8000, maxBytes: 65536 });
      if (!current(job)) throw new Error('download_cancelled');
      const payload = result.payload;
      if (payload?.status === 'retry') throw new Error('download_file_not_ready');
      if (payload?.status !== 'success' || (payload.file_id && payload.file_id !== entry.fileId)) {
        throw new Error('download_authorization_failed');
      }
      await enqueue(job, downloadUrl(payload.download_url));
      job.queued = true;
      respond(ACTION, true, 'download_queued');
    } catch (error) {
      const reason = String(error?.message || '');
      const code = ['download_file_not_ready', 'download_source_unsupported', 'download_confirmation_unknown',
        'download_enqueue_failed', 'download_cancelled'].includes(reason) ? reason :
        reason === 'http_404' ? 'download_file_unavailable' : 'download_prepare_failed';
      respond(ACTION, false, code);
    } finally {
      root.clearTimeout(timer);
      job.controller.abort();
      if (!job.queued) abandon(descriptor);
      if (active === job) active = null;
    }
  }

  function cancel() { active?.controller.abort(); }
  function dispose() { disposed = true; cancel(); entries.clear(); }
  return Object.freeze({ version: 3, register, start, cancel, dispose });
});
