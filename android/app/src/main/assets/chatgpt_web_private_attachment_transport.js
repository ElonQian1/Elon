(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 11, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root && root.location?.origin === 'https://chatgpt.com') {
    root.__elonChatGptPrivateAttachmentTransport = exported;
  }
})(typeof window === 'object' ? window : null, function (root, options) {
  'use strict';
  const protocol = options?.protocol || root.__elonChatGptPrivateAttachmentProtocol;
  const bytes = options?.bytes || root.__elonChatGptPrivateAttachmentBytes;
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
  const reservationModule = options?.reservation || root.__elonChatGptPrivateAttachmentReservation;
  const reservation = [1, 2, 3].includes(reservationModule?.version) ? reservationModule.create(root, {
    protocol, bytes, request, isCurrent: current, acquireHeaders: async () => allowedHeaders(await acquire()),
  }) : null;
  const libraryModule = options?.library || root.__elonChatGptPrivateAttachmentLibrary;
  const library = libraryModule?.version === 1 ? libraryModule.create(root, { protocol, request }) : null;

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

  async function dispatch(job, url, init, mode, timeoutMs, signal = job.controller.signal) {
    assertCurrent(job);
    if (signal.aborted) throw new Error('cancelled');
    job.dispatched += 1;
    const result = await request(root, url, {
      ...init, redirect: 'error', signal,
    }, { mode, timeoutMs, maxBytes: 256 * 1024 });
    assertCurrent(job);
    return result;
  }

  async function uploadBytes(job, destination, file, headers, signal) {
    const controller = new root.AbortController();
    const abort = () => controller.abort();
    signal.addEventListener('abort', abort, { once: true });
    if (signal.aborted) abort();
    try {
      if (bytes?.version === 1) return await bytes.upload(root, destination, file, headers, {
        assertCurrent: () => {
          assertCurrent(job);
          if (controller.signal.aborted) throw new Error('cancelled');
        }, abort,
        dispatch: (url, init, mode, timeout) => dispatch(job, url, init, mode, timeout, controller.signal),
      });
      return await dispatch(job, destination.url, {
        method: 'PUT', credentials: 'omit', headers: destination.headers, body: file,
      }, 'none', 30000, controller.signal);
    } finally {
      signal.removeEventListener('abort', abort);
      abort();
    }
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
      if (bytes?.version === 1) body.supports_direct_azure_multipart = true;
      const creationHeaders = protocol.creationHeaders(file, selected);
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
      const reserved = reservation?.take(file, selected, binding);
      const prepared = reserved?.entry || (await dispatch(job, '/backend-api/files', {
        method: 'POST', credentials: 'include', headers: { ...headers, ...creationHeaders }, body: JSON.stringify(body),
      }, 'json', 15000)).payload;
      job.fileId = prepared?.file_id || null;
      const destination = bytes?.version === 1 ? bytes.plan(prepared, file, protocol)
        : protocol.destination(prepared, file.type);
      job.fileId = destination.fileId;
      change(job, 'uploading');
      const transfer = library ? await library.transfer(file, selected, headers, {
        signal: job.controller.signal, assertCurrent: () => assertCurrent(job),
        upload: signal => uploadBytes(job, destination, file, headers, signal),
      }) : { kind: 'uploaded', result: await uploadBytes(job, destination, file, headers, job.controller.signal) };
      let result;
      if (transfer.kind === 'reused') {
        job.fileId = transfer.file.fileId;
        result = { metadata: { libraryFileId: transfer.file.libraryFileId, libraryPersistenceResult: 'library',
          mimeType: transfer.file.mimeType }, eventCount: 0, events: [] };
      } else {
        change(job, 'processing');
        const processing = reserved?.claim || { url: '/backend-api/files/process_upload_stream',
          body: JSON.stringify(protocol.processBody(destination.fileId, file, selected)) };
        const processed = await dispatch(job, processing.url, {
          method: 'POST', credentials: 'include', headers: reserved ? { ...headers, ...creationHeaders } : headers,
          body: processing.body,
        }, 'text', 30000);
        result = protocol.processed(processed.text, destination.fileId);
      }
      if (selected.imageDimensions && result.metadata.mimeType && result.metadata.mimeType !== file.type ||
          selected.isTemporaryChat === true && result.metadata.libraryPersistenceResult === 'library') {
        throw new Error('processing_metadata_mismatch');
      }
      const stage = transfer.kind === 'reused' ? 'reused' : 'processed';
      change(job, stage);
      assertCurrent(job);
      return {
        ok: true, stage, binding, fileId: job.fileId,
        ...(stage === 'reused' ? { reusedFileName: transfer.file.fileName } : {}),
        fileName: file.name, fileSize: file.size, mimeType: file.type,
        isTemporaryChat: selected.isTemporaryChat === true,
        projectId: selected.projectScopeId || selected.libraryFileInfo?.gizmo_id || null,
        projectWriteRequested: selected.libraryFileInfo?.should_upload_to_project === true,
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
      reservation?.cancel();
      if (active === job) active = null;
    }
  }

  function prefetch(context, binding, signal) {
    if (!active && cooldownUntil <= Date.now()) reservation?.start(context, binding, signal);
  }
  function cancel() { reservation?.cancel(); if (active) active.controller.abort(); }
  function snapshot() { return { version: 11, stage: active?.stage || 'idle', cooldown: cooldownUntil > Date.now() }; }
  return Object.freeze({ version: 11, prefetch, upload, cancel, dispose: cancel, snapshot });
});
