(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 10, operationTimeoutMs: 24000, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateLibraryAttachment = exported;
})(typeof window === 'object' ? window : null, function (root, options) {
  'use strict';
  const composer = options?.composer;
  const mounted = root.__elonChatGptPrivateMountedLibraryAttachment?.create(root);
  const policy = root.__elonChatGptPrivateLibraryAttachmentPolicy ||
    (typeof module === 'object' && module.exports ? require('./chatgpt_web_private_library_attachment_policy') : null);
  const raster = root.__elonChatGptPrivateLibraryRasterPolicy ||
    (typeof module === 'object' && module.exports ? require('./chatgpt_web_private_library_raster_policy') : null);
  const receipts = new Map();
  const consumed = new Map();
  const OPERATION_TIMEOUT_MS = 24000;
  let active = null;

  function descriptor(source) {
    const remote = mounted?.descriptor(source);
    if (remote) return remote;
    if (source?.kind !== 'file' || !/^libfile[_-][A-Za-z0-9_-]{1,152}$/.test(source.id || '') ||
        !/^file[_-][A-Za-z0-9_-]{1,155}$/.test(source.file_id || '') ||
        source.external_account != null || source.cloud_doc_url != null ||
        source.library_artifact_type != null && raster?.matches(source) !== true ||
        source.saved_entity != null || source.trashed_at != null || source.is_project === true ||
        typeof source.name !== 'string' || !source.name.trim() || source.name.length > 120 ||
        /[\x00-\x1f\x7f/\\]/.test(source.name) || !Number.isSafeInteger(source.file_size_bytes) ||
        source.file_size_bytes < 1 || source.file_size_bytes > 8 * 1024 * 1024) return null;
    const file = { name: source.name, type: source.mime_type, size: source.file_size_bytes };
    if (!['image/jpeg', 'image/png', 'image/webp'].includes(file.type) &&
        root.__elonChatGptPrivateAttachmentProtocol?.isDocument(file) !== true) return null;
    return file;
  }

  function ready(source, requestId) {
    const value = descriptor(source);
    if (!value || !/^mcp_[a-z0-9]{1,32}$/.test(requestId || '')) throw new Error('library_file_unsupported');
    // Official SB.attachLibraryFile uses a metadata-only File and the existing backing ID.
    const file = new root.File([], value.name, { type: value.type });
    return { id: 'private_attachment_' + requestId, attached: {
      tempId: 'native_library_' + requestId, status: 'ready', file, fileId: source.file_id,
      cdnUrl: null, progress: 100, source: 'library', libraryFileId: source.id,
      ...(source.library_artifact_type == null ? {} : { libraryArtifactType: source.library_artifact_type }),
      libraryProvider: 'native', libraryEntrypoint: 'composer_library_picker', isBigPaste: false,
      fileSpec: { id: source.file_id, name: value.name, size: value.size, mimeType: value.type,
        isBigPaste: false, ...(/^image\//.test(value.type) ? { width: 512, height: 512 } : {}) },
    } };
  }

  async function attach(command, respond, changed) {
    const action = 'attach_library_file';
    let input;
    try { input = JSON.parse(command.value || '{}'); } catch (_) {}
    const id = command.requestId;
    if (command.selected !== true || !/^mcp_[a-z0-9]{1,32}$/.test(id || '') ||
        !/^library_[a-f0-9]{32}$/.test(input?.fileHandle || '')) {
      return respond(action, false, 'invalid_library_attachment');
    }
    const previous = receipts.get(id);
    if (previous) {
      const same = previous.handle === input.fileHandle && previous.current();
      const result = same ? await previous.result : [false, 'library_selection_expired'];
      return respond(action, ...result);
    }
    if (active || options?.busy?.() || root.__elonChatGptPrivateLibraryMutations?.busy?.()) {
      return respond(action, false, 'library_attachment_busy');
    }
    const selection = root.__elonChatGptPrivateLibraryCatalog?.selectAttachment?.(input.fileHandle);
    const file = descriptor(selection?.source);
    if (!file) return respond(action, false, 'library_selection_expired');
    const remote = !!mounted?.descriptor(selection.source);
    for (const [handle, time] of consumed) if (Date.now() - time >= 60000) consumed.delete(handle);
    let context;
    try { context = composer.captureLibrary(); } catch (error) {
      const code = error?.message === 'library_attachment_scope_unconfirmed'
        ? 'library_attachment_scope_unconfirmed' : 'composer_context_unavailable';
      return respond(action, false, code);
    }
    const { binding } = context;
    if (!binding.libraryEnabled || binding.isTemporaryChat) {
      return respond(action, false, 'library_attachment_scope_unconfirmed');
    }
    const job = { controller: new root.AbortController() };
    const now = () => root.performance?.now?.() ?? Date.now();
    const started = now();
    let selectionVerified = false;
    const current = () => (selection.owned?.() ?? selection.current()) && composer.current(binding) &&
      !root.__elonChatGptPrivateLibraryMutations?.busy?.();
    const canAssociate = () => !job.controller.signal.aborted && now() - started < OPERATION_TIMEOUT_MS &&
      selectionVerified && current() && context.current();
    active = job;
    const entry = { handle: input.fileHandle, current };
    receipts.set(id, entry);
    while (receipts.size > 64) receipts.delete(receipts.keys().next().value);
    let timer, onAbort, associated = false;
    const aborted = new Promise(resolve => {
      onAbort = () => resolve([false, 'library_attachment_unconfirmed']);
      job.controller.signal.addEventListener('abort', onAbort, { once: true });
      timer = root.setTimeout(() => job.controller.abort(), OPERATION_TIMEOUT_MS);
    });
    const execute = async () => {
      try {
        if (!selection.current() && (!selection.refresh || !await selection.refresh(job.controller.signal))) {
          return [false, 'library_selection_expired'];
        }
        selectionVerified = true;
        if (canAssociate() && context.contains(selection.source)) return [true, 'library_attachment_associated'];
        if (remote && (consumed.has(input.fileHandle) || consumed.size >= 512)) {
          return [false, 'library_selection_expired'];
        }
        const admit = context.count || binding.libraryProjectId ? await policy?.prepare(root, binding) : () => true;
        if (!canAssociate()) return [false, 'library_attachment_context_changed'];
        if (!admit) return [false, 'library_attachment_policy_unconfirmed'];
        if (!admit(file)) return [false, 'library_attachment_limit'];
        if (!await context.prepare(job.controller.signal, file)) return [false, context.failureDetail()];
        if (!canAssociate()) return [false, 'library_attachment_context_changed'];
        let item;
        if (remote) {
          // Consume this selected handle before the write; an unknown outcome is not replayed.
          consumed.set(input.fileHandle, Date.now());
          const prepared = await mounted.prepare(selection.source, id, job.controller.signal, canAssociate);
          if (!await context.prepare(job.controller.signal, prepared.descriptor)) return [false, context.failureDetail()];
          if (!canAssociate()) return [false, 'library_attachment_context_changed'];
          if (!admit(prepared.descriptor)) return [false, 'library_attachment_limit'];
          item = prepared.item;
        } else item = ready(selection.source, id);
        if (!canAssociate() || !admit(file)) return [false, 'library_attachment_context_changed'];
        context.associate(item);
        associated = true;
        return [true, 'library_attachment_associated'];
      } catch (_) {
        return [false, 'library_attachment_unconfirmed'];
      }
    };
    // The caller owns one deadline across scope read and remote preparation. Late work cannot publish.
    entry.result = Promise.race([execute(), aborted]).finally(() => {
      root.clearTimeout(timer);
      job.controller.signal.removeEventListener('abort', onAbort);
      job.controller.abort();
      if (active === job) active = null;
    });
    const result = await entry.result;
    respond(action, ...result);
    // Snapshot invalidation cancels attachment work. Notify only after this receipt has settled.
    if (associated && result[0]) changed(true);
  }

  function cancel() { active?.controller.abort(); }
  return Object.freeze({ descriptor, ready, attach, cancel, busy: () => !!active });
});
