(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 4, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateAttachmentReservation = exported;
})(typeof window === 'object' ? window : null, function (root, options) {
  'use strict';
  const RUNTIME_URL = 'https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js';
  const EXPERIMENT = '3119290944';
  const protocol = options.protocol;
  const bytes = options.bytes;
  const now = options.now || Date.now;
  let slot = null;

  function eligible(context) {
    const temporary = context?.isTemporaryChat === true;
    // Official temporary uploads omit legacy persistence; only the reservation
    // selection/claim defaults it to required, with store_in_library still false.
    return !!context && ['ace_upload', 'my_files', 'multimodal'].includes(context.useCase) &&
      typeof context.storeInLibrary === 'boolean' &&
      (context.libraryPersistenceMode === (temporary ? undefined : 'required') ||
        !temporary && context.storeInLibrary && context.libraryPersistenceMode === 'opportunistic') &&
      (context.isTemporaryChat === undefined || typeof context.isTemporaryChat === 'boolean') &&
      (!temporary || context.storeInLibrary === false &&
        (context.indexForRetrieval === undefined || context.indexForRetrieval === false)) &&
      (context.isProjectThread === undefined || context.isProjectThread === false) &&
      !context.projectScopeId && !context.gizmoId && !context.libraryFileInfo &&
      !context.directoryId && !context.uploadSource;
  }

  function current(value) {
    try {
      return slot === value && !value.controller.signal.aborted &&
        root.location?.origin === 'https://chatgpt.com' && root.location.href === value.href &&
        options.isCurrent(value.binding) === true;
    } catch (_) { return false; }
  }

  function release(value) {
    if (!value) return;
    root.clearTimeout(value.timer);
    value.signal?.removeEventListener('abort', value.onAbort);
    value.controller.abort();
    value.result = null;
    if (slot === value) slot = null;
  }

  function cancel() { release(slot); }

  function destination(payload, mimeType) {
    if (payload?.eligible !== true || typeof payload.upload_url !== 'string' ||
        typeof payload.upload_url_expires_at !== 'string' || typeof payload.reservation_expires_at !== 'string' ||
        !Number.isFinite(Date.parse(payload.upload_url_expires_at)) ||
        !Number.isFinite(Date.parse(payload.reservation_expires_at)) ||
        payload.upload_headers != null || payload.direct_library_upload_strategy != null) return null;
    const entry = { status: 'success', file_id: payload.reservation_id,
      upload_url: payload.upload_url };
    if (bytes?.version === 1) bytes.plan(entry, { type: mimeType }, protocol);
    else protocol.destination(entry, mimeType);
    return Object.freeze(entry);
  }

  async function enabled(value) {
    const bindings = root.__elonChatGptPrivateRuntimeBindings;
    const loaded = bindings ? bindings.observed(RUNTIME_URL) :
      root.performance?.getEntriesByName?.(RUNTIME_URL, 'resource')?.length > 0 ||
      !!root.document?.querySelector?.('link[rel="modulepreload"][href="' + RUNTIME_URL + '"]');
    if (!loaded || !current(value)) return false;
    // Import only the inspected module already used by this page. Its experiment
    // client is authoritative; a missing value is not permission to enable it.
    const namespace = await (options.loadRuntime || (url => bindings ? bindings.load(url) : import(url)))(RUNTIME_URL);
    if (!current(value)) return false;
    const client = namespace?.t6?.();
    if (client?.loadingStatus !== 'Ready') return false;
    const experiment = client.getExperiment?.(EXPERIMENT, { disableExposureLog: true });
    return experiment?.name === EXPERIMENT && typeof experiment.groupName === 'string' &&
      experiment.groupName.length > 0 && /^[A-Za-z]+:Recognized$/.test(experiment.details?.reason || '') &&
      !experiment.details?.warnings?.length && experiment.get?.('enable', false) === true;
  }

  async function allocate(value) {
    try {
      if (!await enabled(value)) return;
      const headers = await options.acquireHeaders();
      if (!current(value)) return;
      const response = await options.request(root, '/backend-api/files/upload_reservations', {
        method: 'POST', credentials: 'include', redirect: 'error', headers,
        signal: value.controller.signal,
        body: JSON.stringify({ intended_use_case: value.context.useCase, entry_surface: 'chat_composer',
          requires_gizmo_id: false, store_in_library: value.context.storeInLibrary,
          library_persistence_mode: value.context.libraryPersistenceMode ?? 'required' }),
      }, { mode: 'json', timeoutMs: 3000, maxBytes: 16 * 1024 });
      if (!current(value)) return;
      const payload = response.payload;
      const entry = destination(payload, 'application/octet-stream');
      if (entry) value.result = Object.freeze({ entry,
        uploadExpires: Date.parse(payload.upload_url_expires_at),
        reservationExpires: Date.parse(payload.reservation_expires_at) });
    } catch (_) {
      // Allocation has not uploaded any user bytes. A missing/failed slot only
      // selects the existing private create route; it does not trigger a replay.
    } finally {
      root.clearTimeout(value.timer);
    }
  }

  function start(context, binding, signal) {
    cancel();
    if (!eligible(context) || !binding || signal?.aborted || typeof options.request !== 'function' ||
        typeof options.acquireHeaders !== 'function' || typeof options.isCurrent !== 'function' || !protocol) return;
    const value = { context: Object.freeze({ ...context }), binding, href: root.location?.href,
      controller: new root.AbortController(), signal, result: null };
    slot = value;
    value.onAbort = () => release(value);
    signal?.addEventListener('abort', value.onAbort, { once: true });
    value.timer = root.setTimeout(() => release(value), 5000);
    void allocate(value);
  }

  function claim(entry, file, context) {
    const created = protocol.prepare(file, context);
    const processing = protocol.processBody(entry.file_id, file, context);
    const useCase = context.useCase === 'ace_upload' && file.name.lastIndexOf('.') > 0 &&
      /\.(?:hwp|hwpx)$/i.test(file.name) ? 'my_files' : context.useCase;
    return Object.freeze({
      url: '/backend-api/files/upload_reservations/' + encodeURIComponent(entry.file_id) + '/claim_and_finish',
      body: JSON.stringify({ file_name: file.name, file_size: file.size, use_case: useCase,
        index_for_retrieval: context.indexForRetrieval || context.useCase === 'ace_upload' && useCase === 'my_files',
        store_in_library: context.storeInLibrary, library_persistence_mode: context.libraryPersistenceMode ?? 'required',
        mime_type: created.mime_type, entry_surface: 'chat_composer', metadata: processing.metadata,
        ...(context.imageDimensions ? protocol.imageDimensions(context.imageDimensions) : {}) }),
    });
  }

  function take(file, context, binding) {
    const value = slot;
    if (!value) return null;
    try {
      const result = value.result;
      if (!current(value) || binding !== value.binding || !eligible(context) ||
          context.useCase !== value.context.useCase || context.storeInLibrary !== value.context.storeInLibrary ||
          context.libraryPersistenceMode !== value.context.libraryPersistenceMode ||
          (context.isTemporaryChat === true) !== (value.context.isTemporaryChat === true) ||
          context.modelSlug !== value.context.modelSlug || context.storeInLibrary && file.size > 2 * 1024 * 1024 ||
          !result || result.uploadExpires - now() < 60000 || result.reservationExpires - now() < 60000) return null;
      return Object.freeze({ entry: result.entry, claim: claim(result.entry, file, context) });
    } catch (_) { return null; }
    finally { release(value); }
  }

  // Selection never waits for allocation. Taking a pending slot abandons it, so
  // a late response cannot replace an already selected upload transaction.
  return Object.freeze({ version: 4, start, take, cancel });
});
