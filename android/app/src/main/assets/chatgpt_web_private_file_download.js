(function (root, factory) {
  'use strict';
  const pointer = typeof module === 'object' && module.exports
    ? require('./chatgpt_web_private_image_pointer.js') : root?.__elonChatGptPrivateImagePointer;
  const citation = typeof module === 'object' && module.exports
    ? require('./chatgpt_web_private_file_citation.js') : root?.__elonChatGptPrivateFileCitation;
  const exported = Object.freeze({ version: 29, create: root => factory(root, pointer, citation) });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com' &&
      Number(root.__elonChatGptPrivateFileDownload?.version || 0) < exported.version) {
    root.__elonChatGptPrivateFileDownload?.dispose?.();
    root.__elonChatGptPrivateFileDownload = factory(root, pointer, citation);
  }
})(typeof window === 'object' ? window : null, function (root, pointerParser, citationParser) {
  'use strict';
  const entries = new Map();
  const PATH = /^(?:\/g\/(g-p-[a-f0-9]{32})(?:-[A-Za-z0-9_-]{1,124})?)?\/c\/([A-Za-z0-9_-]{1,160})$/i;
  const PROJECT = /^g-p-[a-f0-9]{32}$/i;
  const LIBRARY = /^libfile[_-][A-Za-z0-9_-]{1,152}$/;
  const HANDLE = /^download_[a-f0-9]{32}$/;
  const ACTION = 'download_conversation_file';
  let active = null;
  let disposed = false;
  let lastSource = null;

  function sourceDiagnostics() {
    if (disposed || !lastSource || !lastSource.value || lastSource.expiresAt <= Date.now() || lastSource.account !== identity() ||
        lastSource.token !== root.__elonChatGptDocumentToken) return null;
    return { ...lastSource.value };
  }

  function identity() {
    const headers = root.__elonChatGptPrivateTransport?.copySameOriginRequestHeaders?.();
    const values = {};
    for (const [key, value] of Object.entries(headers || {})) values[key.toLowerCase()] = value;
    if (!/^Bearer\s+\S{8,65536}$/.test(values.authorization || '')) return null;
    return JSON.stringify(['authorization', 'chatgpt-account-id', 'oai-device-id'].map(key => values[key] || ''));
  }

  function authorizationUrl(entry, projectId) {
    const url = new URL('/backend-api/files/download/' + encodeURIComponent(entry.downloadFileId || entry.fileId), root.location.origin);
    for (const [key, value] of entry.downloadQuery || []) url.searchParams.set(key, value);
    if (projectId) url.searchParams.set('gizmo_id', projectId);
    if (entry.conversationId) {
      url.searchParams.set(entry.image || entry.fileCitation || entry.projectId || entry.libraryFileId
        ? 'check_context_scopes_for_conversation_id' : 'conversation_id', entry.conversationId);
    }
    url.searchParams.set('download_intent', 'true');
    return url.href;
  }

  function connectorCopy(file) {
    const info = file.context_connector_info;
    if (info == null) return true;
    if (typeof info !== 'object' || Array.isArray(info) ||
        Object.keys(info).some(key => !['context_connector', 'source_url', 'synthetic_extension', 'type'].includes(key)) ||
        typeof info.context_connector !== 'string' || !/^[A-Za-z0-9_-]{1,96}$/.test(info.context_connector) ||
        file.source != null && !['connector', 'library', 'local'].includes(file.source)) return false;
    for (const key of ['synthetic_extension', 'type']) {
      if (info[key] != null && (typeof info[key] !== 'string' || info[key].length > 96 ||
          /[\x00-\x1f\x7f]/.test(info[key]))) return false;
    }
    if (info.source_url != null && info.source_url !== '') {
      if (typeof info.source_url !== 'string' || info.source_url.length > 8192 ||
          /[\x00-\x20\x7f]/.test(info.source_url)) return false;
      try {
        const url = new URL(info.source_url);
        if (!['https:', 'http:'].includes(url.protocol) || url.username || url.password) return false;
      } catch (_) { return false; }
    }
    // Gar emits this attribution for an uploaded connector copy. C4t and
    // jW/A5t download the ChatGPT file ID; source_url is display-only.
    return true;
  }

  function imageFile(source) {
    const image = source.image;
    if (image?.content_type !== 'image_asset_pointer' || typeof image.asset_pointer !== 'string' ||
        source.attachmentsUnconfirmed || !Array.isArray(source.attachments)) return null;
    const pointer = pointerParser?.parse(image.asset_pointer);
    if (!pointer || ['gizmo_id', 'project_id', 'library_file_id', 'shared_library_file_id',
      'library_download_id', 'context_scopes', 'source_url', 'context_connector', 'connector_id',
      'context_connector_info'].some(key => image[key] != null)) return null;
    // Official jW/A5t pair image pointers with attachment metadata before pEt/fEt.
    const matches = source.attachments.filter(file => file?.id === pointer.id);
    if (matches.length > 1 || matches[0]?.mime_type != null &&
        (typeof matches[0].mime_type !== 'string' || !/^image\/[A-Za-z0-9.+-]{1,63}$/.test(matches[0].mime_type))) return null;
    return { ...matches[0], ...pointer };
  }

  function target(path, source, scope) {
    const conversation = PATH.exec(path || '');
    const image = source?.image != null;
    const shared = source?.sharedLibraryReference != null;
    const mountedReference = source?.mountedLibraryReference != null;
    const fileCitation = source?.fileCitationReference != null;
    if (fileCitation && (image || shared || mountedReference || source.attachment != null)) return null;
    if (mountedReference && (image || shared || source.attachment != null)) return null;
    if (shared && (image || source.attachment != null)) return null;
    const file = image ? imageFile(source) : shared ? source.sharedLibraryReference :
      mountedReference ? source.mountedLibraryReference : fileCitation
        ? citationParser?.target(source.fileCitationReference) : source?.attachment;
    const mounted = !image && !shared && root.__elonChatGptPrivateLibraryDownload?.mountedTarget?.(file, mountedReference);
    if (mountedReference && !mounted) return null;
    const libraryReference = !image && (shared
      ? root.__elonChatGptPrivateLibraryDownload?.sharedReference?.(file)
      : root.__elonChatGptPrivateLibraryDownload?.target?.(file));
    if (shared && !libraryReference) return null;
    if (!conversation || !file || !image && !libraryReference && !mounted && !/^[A-Za-z0-9_-]{1,160}$/.test(file.id || '')) return null;
    // Cloud references and alternate preview targets have separate resolvers.
    if (['shared_library_file_id', 'library_download_id', 'source_url',
      'context_connector', 'connector_id', ...(!mounted ? ['mounted_library_file_id'] : []), 'shared_library_file_reference',
      'preview_file'].some(key => file[key] != null && file[key] !== '') || !connectorCopy(file)) return null;
    if (scope?.context_scopes != null && (!Array.isArray(scope.context_scopes) || scope.context_scopes.length)) return null;
    if (file.context_scopes != null && (!Array.isArray(file.context_scopes) || file.context_scopes.length)) return null;
    const projects = [conversation[1], source.projectId, scope?.gizmo_id, scope?.project_id,
      file.gizmo_id, file.project_id].filter(value => value != null && value !== '');
    if (projects.some(value => typeof value !== 'string' || !PROJECT.test(value)) || new Set(projects).size > 1) return null;
    const libraryFileId = mounted || file.library_file_id == null || file.library_file_id === '' ? null : file.library_file_id;
    if (libraryFileId !== null && (typeof libraryFileId !== 'string' || !LIBRARY.test(libraryFileId))) return null;
    return Object.freeze({ conversationId: conversation[2], fileId: file.id,
      projectId: projects[0] || null, libraryFileId,
      connectorCopy: file.context_connector_info != null,
      mediaType: typeof file.mime_type === 'string' ? file.mime_type : '',
      ...(fileCitation ? { fileCitation: true, metadataProjectId: file.gizmo_id || file.project_id || null } : {}),
      ...(mounted || {}),
      ...(libraryReference || {}),
      ...(libraryReference || mounted ? { name: file.name.replace(/\u00a0/g, ' ').trim().slice(0, 180), mediaType: file.mime_type || '' } : {}),
      ...(image ? { image: true,
        downloadFileId: file.downloadFileId, downloadQuery: file.downloadQuery,
        name: typeof file.name === 'string' && file.name.trim() ? file.name.replace(/\u00a0/g, ' ').trim().slice(0, 180) : 'image.png',
        mediaType: file.mime_type || '' } : {}) });
  }

  async function resolveDestination(job, request) {
    const entry = job.entry;
    if (!request?.request || !current(job)) throw new Error('download_cancelled');
    if (entry.mountedFileId) {
      const materialized = await root.__elonChatGptPrivateLibraryDownload.materialize(root, job, current);
      if (!current(job)) throw new Error('download_cancelled');
      job.entry = Object.freeze({ ...entry, ...materialized });
      return { url: authorizationUrl(job.entry, entry.projectId), fileId: materialized.fileId };
    }
    if (!entry.libraryFileId && !entry.fileCitation) return { url: authorizationUrl(entry, entry.projectId) };
    if (entry.fileCitation && entry.projectId && !entry.metadataProjectId && !entry.libraryFileId) {
      // Official fDt skips metadata without explicit file ownership. A project
      // conversation still supplies its access scope, not file ownership.
      return { url: authorizationUrl(entry, null) };
    }
    // FileCitationPreviewSheet uses cEt/OX even when the reference omits library identity.
    const url = new URL('/backend-api/files/' + encodeURIComponent(entry.fileId) + '/simple', root.location.origin);
    // Official V3t/w3n pass file-preview ownership, not the current chat's
    // project. Conversation scope is still sent separately for access checks.
    const metadataProjectId = entry.fileCitation ? entry.metadataProjectId : entry.projectId;
    if (metadataProjectId) url.searchParams.set('gizmo_id', metadataProjectId);
    if (entry.conversationId) url.searchParams.set('conversation_id', entry.conversationId);
    const result = await request.request(root, url.href, {
      method: 'GET', credentials: 'same-origin', cache: 'no-store', redirect: 'error',
      headers: root.__elonChatGptPrivateTransport.copySameOriginRequestHeaders(), signal: job.controller.signal,
    }, { timeoutMs: 6000, maxBytes: 65536 });
    if (!current(job)) throw new Error('download_cancelled');
    const info = result.payload;
    if (!info || typeof info !== 'object' || Array.isArray(info) ||
        info.is_library_file !== undefined && typeof info.is_library_file !== 'boolean' ||
        entry.libraryFileId && (info.is_library_file !== true || info.library_file_id !== entry.libraryFileId) ||
        info.file_id != null && info.file_id !== entry.fileId ||
        info.library_file_id != null && (typeof info.library_file_id !== 'string' || !LIBRARY.test(info.library_file_id)) ||
        info.is_project != null && typeof info.is_project !== 'boolean' ||
        info.gizmo_id != null && (typeof info.gizmo_id !== 'string' || !PROJECT.test(info.gizmo_id))) {
      throw new Error('download_scope_unconfirmed');
    }
    const isLibrary = info.is_library_file === true;
    if (entry.catalogProject && (!isLibrary || info.is_project !== true || !PROJECT.test(info.gizmo_id || '') ||
        entry.projectId && info.gizmo_id !== entry.projectId)) throw new Error('download_scope_unconfirmed');
    const projectId = isLibrary ? (info.is_project === true || PROJECT.test(info.gizmo_id || '')
      ? info.gizmo_id || entry.projectId : null) : entry.projectId;
    if (isLibrary && info.is_project === true && !projectId) throw new Error('download_scope_unconfirmed');
    const libraryFileId = entry.libraryFileId || (isLibrary ? info.library_file_id : null);
    // Official preview passes libraryDownloadId only after matching personal
    // ownership. Images and connector copies retain their separate resolvers.
    if (!entry.image && !entry.connectorCopy && libraryFileId && !projectId && info.is_project !== true) {
      return { libraryDownloadId: libraryFileId };
    }
    return { url: authorizationUrl(entry, projectId), ...(isLibrary && projectId && libraryFileId
      ? { projectContentScope: { libraryFileId, fileId: entry.fileId } } : {}) };
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
      (!job.entry.galleryCurrent || job.entry.galleryCurrent()) &&
      root.location.href === job.descriptor.href && root.__elonChatGptDocumentToken === job.entry.token &&
      identity() === job.entry.account && (Boolean(job.entry.sharedLibraryFileId) || job.byteTransfer || job.entry.expiresAt > Date.now());
  }

  function registerGalleryImage(item, galleryCurrent) {
    const account = identity(), token = root.__elonChatGptDocumentToken;
    const restricted = ['gizmo_id', 'project_id', 'post_id', 'library_file_id', 'shared_library_file_id',
      'library_download_id', 'context_scopes', 'source_url', 'context_connector', 'connector_id',
      'context_connector_info', 'shared', 'watermarked_url', 'watermarkedUrl'];
    const pointer = pointerParser?.parse(item?.asset_pointer);
    const originalUrl = root.__elonChatGptPrivateContentSource?.previewUrl?.(item?.url);
    if (disposed || !account || !/^doc_[a-z0-9_]{3,80}$/.test(token || '') ||
        !root.elonChatGptFileDownload || typeof galleryCurrent !== 'function' || !galleryCurrent() ||
        typeof item?.conversation_id !== 'string' || !/^[A-Za-z0-9_-]{1,160}$/.test(item.conversation_id) ||
        item.is_project != null && item.is_project !== false || restricted.some(key => item[key] != null) ||
        item.url != null && !originalUrl ||
        !pointer || pointer.downloadQuery.some(([key]) => restricted.includes(key.toLowerCase()))) return '';
    // The catalog URL is the original resource, never the resized JPEG export.
    const request = target('/c/' + item.conversation_id,
      { image: { content_type: 'image_asset_pointer', asset_pointer: item.asset_pointer }, attachments: [] }, null);
    if (!request) return '';
    const handle = 'download_' + Array.from(root.crypto.getRandomValues(new Uint8Array(16)),
      value => value.toString(16).padStart(2, '0')).join('');
    entries.set(handle, { ...request, path: '/images', account, token, galleryCurrent, originalUrl,
      expiresAt: Date.now() + 120000 });
    while (entries.size > 800) entries.delete(entries.keys().next().value);
    return handle;
  }

  function registerLibraryFile(file) {
    const account = identity(), token = root.__elonChatGptDocumentToken;
    const mounted = root.__elonChatGptPrivateLibraryDownload?.catalogTarget?.(file);
    if (disposed || !account || !/^doc_[a-z0-9_]{3,80}$/.test(token || '') ||
        !root.elonChatGptFileDownload || file?.kind !== 'file' || !mounted && !LIBRARY.test(file.id || '') ||
        typeof file.name !== 'string' || !file.name.trim() || /[\x00-\x1f\x7f]/.test(file.name) ||
        file.name.length > 1024 || file.external_account != null || !mounted && file.cloud_doc_url != null ||
        file.library_artifact_type != null || file.saved_entity != null || file.trashed_at != null) return '';
    let destination = mounted || { sharedLibraryFileId: file.id };
    if (!mounted && (file.is_project != null && file.is_project !== false || file.gizmo_id != null ||
        file.project_id != null || file.context_scopes != null)) {
      if (file.is_project !== true || !/^file[_-][A-Za-z0-9_-]{1,152}$/.test(file.file_id || '') ||
          file.gizmo_id != null && !PROJECT.test(file.gizmo_id) ||
          ['project_id', 'context_scopes', 'preview_file', 'mounted_library_file_id', 'shared_library_file_id',
            'library_download_id', 'context_connector_info'].some(key => file[key] != null)) return '';
      // Project nodes retain their backing file and own project scope. DDt/sDt
      // confirm metadata before EDt; the personal-library anchor cannot serve them.
      destination = { fileId: file.file_id, libraryFileId: file.id,
        projectId: file.gizmo_id || null, catalogProject: true };
    }
    const name = file.name.trim().slice(0, 180);
    for (const [key, entry] of entries) {
      if (entry.expiresAt <= Date.now()) entries.delete(key);
      else if (entry.path === '/library' && Object.keys(destination).every(key => entry[key] === destination[key]) &&
          entry.name === name && entry.mediaType === (file.mime_type || '') &&
          entry.token === token && entry.account === account) return key;
    }
    const bytes = root.crypto.getRandomValues(new Uint8Array(16));
    const handle = 'download_' + Array.from(bytes, value => value.toString(16).padStart(2, '0')).join('');
    // Standalone mounted files reuse materialization; neither route borrows the open chat's scope.
    entries.set(handle, { path: '/library', ...destination, name,
      mediaType: file.mime_type || '', account, token, expiresAt: Date.now() + 120000 });
    while (entries.size > 800) entries.delete(entries.keys().next().value);
    return handle;
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
          documentToken: job.entry.token, url,
          ...(job.entry.resolvedFile ? { resolvedFile: job.entry.resolvedFile } : {}) }));
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
        !/^[a-f0-9-]{36}$/.test(descriptor.leaseId || '') || entry.expiresAt <= Date.now() || identity() !== entry.account ||
        entry.galleryCurrent && !entry.galleryCurrent()) {
      abandon(descriptor);
      return respond(ACTION, false, 'download_selection_expired');
    }
    const job = { descriptor, entry, controller: new root.AbortController() };
    lastSource = null;
    if (entry.mountedFileId) entries.delete(descriptor.downloadHandle);
    active = job;
    let timer = root.setTimeout(() => job.controller.abort(), 15000);
    try {
      const request = root.__elonChatGptPrivateJsonRequest;
      const destination = entry.originalUrl ? { originalUrl: entry.originalUrl }
        : entry.sharedLibraryFileId ? { libraryDownloadId: entry.sharedLibraryFileId }
        : await resolveDestination(job, request);
      if (!current(job)) throw new Error('download_cancelled');
      if (destination.libraryDownloadId) {
        job.byteTransfer = true;
        root.clearTimeout(timer);
        timer = root.setTimeout(() => job.controller.abort(), 120000);
        const detail = await root.__elonChatGptPrivateLibraryDownload.run(root, job, current,
          downloadUrl, destination.libraryDownloadId);
        job.queued = true;
        respond(ACTION, true, detail);
        return;
      }
      let sourceUrl = destination.originalUrl;
      if (!sourceUrl) {
        const result = await request.request(root, destination.url, {
          method: 'GET', credentials: 'same-origin', cache: 'no-store', redirect: 'error',
          headers: root.__elonChatGptPrivateTransport.copySameOriginRequestHeaders(), signal: job.controller.signal,
        }, { timeoutMs: 8000, maxBytes: 65536 });
        if (!current(job)) throw new Error('download_cancelled');
        const payload = result.payload;
        if (payload?.status === 'retry') throw new Error('download_file_not_ready');
        if (payload?.status !== 'success' || (payload.file_id && payload.file_id !== (destination.fileId || entry.fileId) &&
          payload.file_id !== entry.downloadFileId)) {
          throw new Error('download_authorization_failed');
        }
        sourceUrl = payload.download_url;
      }
      const binary = root.__elonChatGptPrivateLibraryDownload;
      lastSource = { account: entry.account, token: entry.token, expiresAt: Date.now() + 120000,
        value: root.__elonChatGptPrivateContentSource?.describe?.(sourceUrl, destination.projectContentScope) };
      if (binary?.contentUrl?.(sourceUrl, destination.projectContentScope)) {
        // A fresh authorization may return the official same-origin content route,
        // not a signed external URL. Reuse the existing byte owner and save receipt.
        job.byteTransfer = true;
        root.clearTimeout(timer);
        timer = root.setTimeout(() => job.controller.abort(), 120000);
        const detail = await binary.runContent(root, job, current, sourceUrl, destination.projectContentScope);
        job.queued = true;
        respond(ACTION, true, detail);
      } else {
        await enqueue(job, downloadUrl(sourceUrl));
        job.queued = true;
        respond(ACTION, true, 'download_queued');
      }
    } catch (error) {
      const reason = job.cancelled ? 'download_cancelled' : String(error?.message || '');
      const code = ['download_file_not_ready', 'download_source_unsupported', 'download_confirmation_unknown',
        'download_enqueue_failed', 'download_cancelled', 'download_file_unavailable', 'download_storage_failed',
        'download_file_too_large', 'download_content_invalid', 'download_transfer_timeout'].includes(reason) ? reason :
        reason === 'http_404' ? 'download_file_unavailable' : 'download_prepare_failed';
      respond(ACTION, false, code);
    } finally {
      root.clearTimeout(timer);
      job.controller.abort();
      if (!job.queued) abandon(descriptor);
      if (active === job) active = null;
    }
  }

  function cancel(leaseId) {
    if (!active || leaseId != null && leaseId !== active.descriptor.leaseId) return false;
    active.cancelled = true;
    active.controller.abort();
    return true;
  }
  function dispose() { disposed = true; cancel(); entries.clear(); lastSource = null; }
  return Object.freeze({ version: 29, register, registerLibraryFile, registerGalleryImage, start, cancel, dispose, sourceDiagnostics });
});
