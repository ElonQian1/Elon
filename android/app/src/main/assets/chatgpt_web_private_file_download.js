(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com' &&
      Number(root.__elonChatGptPrivateFileDownload?.version || 0) < exported.version) {
    root.__elonChatGptPrivateFileDownload?.dispose?.();
    root.__elonChatGptPrivateFileDownload = factory(root);
  }
})(typeof window === 'object' ? window : null, function (root) {
  'use strict';
  const entries = new Map();
  const PATH = /^\/c\/([A-Za-z0-9_-]{1,160})$/;
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

  function target(path, source) {
    const conversation = PATH.exec(path || '');
    const file = source?.attachment;
    if (!conversation || source?.projectId || !/^file-[A-Za-z0-9_-]{1,155}$/.test(file?.id || '')) return null;
    // Library, project, shared and connector files have additional official scope resolution.
    if (['gizmo_id', 'project_id', 'library_file_id', 'shared_library_file_id', 'source_url',
      'context_connector', 'connector_id', 'context_connector_info'].some(key => file[key] != null && file[key] !== '')) return null;
    const url = new URL('/backend-api/files/download/' + encodeURIComponent(file.id), root.location.origin);
    url.searchParams.set('conversation_id', conversation[1]);
    url.searchParams.set('download_intent', 'true');
    return { url: url.href, fileId: file.id };
  }

  function register(path, payload, index) {
    for (const [key, entry] of entries) {
      if (entry.path === path || entry.expiresAt <= Date.now()) entries.delete(key);
    }
    const account = identity(), token = root.__elonChatGptDocumentToken;
    const projection = root.__elonChatGptPrivateHistoryProjection?.create({});
    return index.files.map(row => {
      if (disposed || !account || !/^doc_[a-z0-9_]{3,80}$/.test(token || '') ||
          !root.elonChatGptFileDownload || !projection?.fileSource) return row;
      const request = target(path, projection.fileSource(payload, row.id));
      if (!request) return row;
      const bytes = root.crypto.getRandomValues(new Uint8Array(16));
      const handle = 'download_' + Array.from(bytes, value => value.toString(16).padStart(2, '0')).join('');
      entries.set(handle, { ...request, path, name: row.name, account, token, expiresAt: Date.now() + 120000 });
      while (entries.size > 800) entries.delete(entries.keys().next().value);
      return { ...row, downloadHandle: handle };
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

  async function start(raw, respond) {
    let descriptor;
    try { descriptor = JSON.parse(raw); } catch (_) { return respond(ACTION, false, 'invalid_file_request'); }
    if (active) return respond(ACTION, false, 'download_busy');
    const entry = entries.get(descriptor?.downloadHandle);
    if (disposed || !HANDLE.test(descriptor?.downloadHandle || '') || !entry ||
        descriptor.version !== 1 || descriptor.path !== entry.path || descriptor.name !== entry.name ||
        descriptor.documentToken !== entry.token || descriptor.href !== root.location.href ||
        !/^[a-f0-9-]{36}$/.test(descriptor.leaseId || '') || entry.expiresAt <= Date.now() || identity() !== entry.account) {
      return respond(ACTION, false, 'download_selection_expired');
    }
    const job = { descriptor, entry, controller: new root.AbortController() };
    active = job;
    const timer = root.setTimeout(() => job.controller.abort(), 15000);
    try {
      const request = root.__elonChatGptPrivateJsonRequest;
      if (!request?.request || !current(job)) throw new Error('download_cancelled');
      const result = await request.request(root, entry.url, {
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
      if (active === job) active = null;
    }
  }

  function cancel() { active?.controller.abort(); }
  function dispose() { disposed = true; cancel(); entries.clear(); }
  return Object.freeze({ version: 1, register, start, cancel, dispose });
});
