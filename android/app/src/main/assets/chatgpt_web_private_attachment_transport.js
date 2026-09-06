(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 4, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root && root.location?.origin === 'https://chatgpt.com') {
    root.__elonChatGptPrivateAttachmentTransport = exported;
  }
})(typeof window === 'object' ? window : null, function (root, options) {
  'use strict';
  const protocol = options?.protocol || root.__elonChatGptPrivateAttachmentProtocol;
  const request = options?.request || root.__elonChatGptPrivateJsonRequest?.request;
  const acquire = options?.acquireHeaders || (() =>
    root.__elonChatGptPrivateTransport.acquireSameOriginRequestHeaders());
  const current = options?.isCurrent;
  const HEADER_NAMES = new Set([
    'authorization', 'chatgpt-account-id', 'oai-device-id', 'oai-language',
    'oai-client-version', 'oai-client-build-number',
  ]);
  let active = null;
  let cooldownUntil = 0;

  function allowedHeaders(source) {
    const result = { Accept: 'application/json', 'Content-Type': 'application/json' };
    for (const [key, value] of Object.entries(source || {})) {
      if (HEADER_NAMES.has(key.toLowerCase()) && typeof value === 'string') result[key.toLowerCase()] = value;
    }
    if (!/^Bearer\s+\S{8,65536}$/.test(result.authorization || '')) throw new Error('auth_unavailable');
    return result;
  }

  function assertCurrent(job) {
    if (job.controller.signal.aborted || active !== job) throw new Error('cancelled');
    if (root.location?.origin !== 'https://chatgpt.com' || root.location.href !== job.href ||
        typeof current !== 'function' || current(job.binding) !== true) throw new Error('context_changed');
  }

  async function dispatch(job, url, init, mode, timeoutMs) {
    assertCurrent(job);
    job.dispatched += 1;
    const result = await request(root, url, {
      ...init, redirect: 'error', signal: job.controller.signal,
    }, { mode, timeoutMs, maxBytes: 256 * 1024 });
    assertCurrent(job);
    return result;
  }

  function change(job, stage) {
    assertCurrent(job);
    job.stage = stage;
    try { options?.onProgress?.({ stage }); } catch (_) {}
  }

  async function upload(file, context, binding) {
    if (active) return { ok: false, code: 'busy', mayHaveSideEffects: false };
    if (cooldownUntil > Date.now()) return { ok: false, code: 'cooldown', mayHaveSideEffects: false };
    if (!protocol || typeof request !== 'function' || typeof current !== 'function' || !binding ||
        typeof root.AbortController !== 'function' || root.location?.origin !== 'https://chatgpt.com') {
      return { ok: false, code: 'unavailable', mayHaveSideEffects: false };
    }
    const job = {
      href: root.location?.href, binding, controller: new root.AbortController(),
      stage: 'validating', dispatched: 0, fileId: null,
    };
    active = job;
    let abortListener;
    let authTimer;
    try {
      assertCurrent(job);
      // Snapshot caller-owned options before any await; model/project changes cannot rewrite a pending upload.
      const selected = Object.freeze({ ...context,
        ...(context.libraryFileInfo == null ? {} : { libraryFileInfo: protocol.projectInfo(context) }),
        ...(context.imageDimensions == null ? {} : { imageDimensions: protocol.imageDimensions(context.imageDimensions) }) });
      const body = protocol.prepare(file, selected);
      const abort = new Promise((_, reject) => {
        abortListener = () => reject(new Error('cancelled'));
        job.controller.signal.addEventListener('abort', abortListener, { once: true });
        authTimer = root.setTimeout(() => reject(new Error('auth_timeout')), 7000);
      });
      const headers = allowedHeaders(await Promise.race([acquire(), abort]));
      root.clearTimeout(authTimer);
      authTimer = null;
      assertCurrent(job);
      change(job, 'preparing');
      const prepared = await dispatch(job, '/backend-api/files', {
        method: 'POST', credentials: 'include', headers, body: JSON.stringify(body),
      }, 'json', 15000);
      job.fileId = prepared.payload?.file_id || null;
      const destination = protocol.destination(prepared.payload, file.type);
      job.fileId = destination.fileId;
      change(job, 'uploading');
      await dispatch(job, destination.url, {
        method: 'PUT', credentials: 'omit', headers: destination.headers, body: file,
      }, 'none', 30000);
      change(job, 'processing');
      const processed = await dispatch(job, '/backend-api/files/process_upload_stream', {
        method: 'POST', credentials: 'include', headers,
        body: JSON.stringify(protocol.processBody(destination.fileId, file, selected)),
      }, 'text', 30000);
      const result = protocol.processed(processed.text, destination.fileId);
      if (selected.imageDimensions && result.metadata.mimeType && result.metadata.mimeType !== file.type ||
          selected.isTemporaryChat === true && result.metadata.libraryPersistenceResult === 'library') {
        throw new Error('processing_metadata_mismatch');
      }
      change(job, 'processed');
      assertCurrent(job);
      return {
        ok: true, stage: 'processed', binding, fileId: destination.fileId,
        fileName: file.name, fileSize: file.size, mimeType: file.type,
        isTemporaryChat: selected.isTemporaryChat === true,
        projectId: selected.libraryFileInfo?.gizmo_id || null,
        metadata: result.metadata, eventCount: result.eventCount, events: result.events,
        ...(selected.imageDimensions ? { imageDimensions: selected.imageDimensions } : {}),
        // Upload completion is not composer association or message-send acknowledgement.
        associated: false,
      };
    } catch (error) {
      const raw = String(error?.message || 'request_failed');
      const code = /^(?:http_\d{3}|invalid_(?:file|file_name|mime_type|prepare_response|upload_url|file_id|process_stream)|unsupported_(?:upload_context|upload_route)|processing_(?:failed|unconfirmed|metadata_mismatch)|process_file_mismatch|auth_(?:unavailable|timeout)|response_too_large|invalid_json|cancelled|timeout|context_changed)$/.test(raw)
        ? raw : 'request_failed';
      if (job.dispatched && !['cancelled', 'context_changed'].includes(code)) cooldownUntil = Date.now() + 45000;
      return { ok: false, code, stage: job.stage, mayHaveSideEffects: job.dispatched > 0, hasFileId: !!job.fileId };
    } finally {
      if (authTimer != null) root.clearTimeout(authTimer);
      if (abortListener) job.controller.signal.removeEventListener('abort', abortListener);
      job.controller.abort();
      if (active === job) active = null;
    }
  }

  function cancel() { if (active) active.controller.abort(); }
  function snapshot() { return { version: 4, stage: active?.stage || 'idle', cooldown: cooldownUntil > Date.now() }; }
  return Object.freeze({ version: 4, upload, cancel, dispose: cancel, snapshot });
});
