(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 3, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateAttachmentLibrary = exported;
})(typeof window === 'object' ? window : null, function (root, options) {
  'use strict';
  const RUNTIME = 'https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js';
  const FLAGS = { enabled: '1342446482', experiment: '2354748696', treatment: '2711463943' };
  const now = () => root.performance?.now?.() ?? Date.now();
  let runtime;

  function eligible(file, context) {
    return context?.checkForReusableLibraryFile !== false && context?.storeInLibrary === true && context.isTemporaryChat !== true &&
      context.isProjectThread !== true && !context.projectScopeId && !context.gizmoId &&
      !context.libraryFileInfo && !context.directoryId && !context.uploadSource &&
      ['required', 'opportunistic'].includes(context.libraryPersistenceMode) &&
      ['ace_upload', 'multimodal', 'my_files'].includes(context.useCase) &&
      Number.isSafeInteger(file?.size) && file.size > 0 && file.size <= options.protocol.maxFileBytes &&
      typeof file.arrayBuffer === 'function' && typeof root.crypto?.subtle?.digest === 'function';
  }

  function recognized(value, name) {
    return value?.name === name && /^[A-Za-z]+:Recognized$/.test(value.details?.reason || '') &&
      !value.details?.warnings?.length;
  }

  async function enabled(assertCurrent) {
    assertCurrent();
    if (!runtime) {
      const bindings = root.__elonChatGptPrivateRuntimeBindings;
      const loaded = bindings ? bindings.observed(RUNTIME) :
        root.performance?.getEntriesByName?.(RUNTIME, 'resource')?.length > 0 ||
        !!root.document?.querySelector?.('link[rel="modulepreload"][href="' + RUNTIME + '"]');
      if (!loaded) return false;
      runtime = Promise.resolve().then(() =>
        (options.loadRuntime || (url => bindings ? bindings.load(url) : import(url)))(RUNTIME));
      runtime.catch(() => { runtime = null; });
    }
    const namespace = await runtime;
    assertCurrent();
    const client = namespace?.t6?.();
    if (client?.loadingStatus !== 'Ready') return false;
    const gate = name => {
      const result = client.getFeatureGate?.(name, { disableExposureLog: true });
      return recognized(result, name) && typeof result.value === 'boolean' ? result.value : null;
    };
    const direct = gate(FLAGS.enabled);
    if (direct === true) return true;
    if (direct !== false || gate(FLAGS.experiment) !== true) return false;
    const result = client.getExperiment?.(FLAGS.treatment, { disableExposureLog: true });
    return recognized(result, FLAGS.treatment) && typeof result.groupName === 'string' &&
      result.groupName.length > 0 && result.get?.('enable', false) === true;
  }

  function reusable(payload, file) {
    const value = payload?.reusable_library_file;
    const id = /^[A-Za-z0-9_-]{1,160}$/;
    if (!value || !id.test(value.file_id || '') || !id.test(value.library_file_id || '') ||
        typeof value.file_name !== 'string' || !value.file_name.trim() || value.file_name.length > 120 ||
        /[\x00-\x1f\x7f/\\]/.test(value.file_name) ||
        value.mime_type != null && value.mime_type !== file.type) return null;
    return Object.freeze({ fileId: value.file_id, libraryFileId: value.library_file_id,
      fileName: value.file_name, mimeType: value.mime_type ?? file.type });
  }

  async function lookup(file, headers, controller, owner) {
    const deadline = now() + 1500;
    const timer = root.setTimeout(() => controller.abort(), 1500);
    function assertCurrent() {
      owner.assertCurrent();
      if (controller.signal.aborted || now() >= deadline || root.location?.origin !== 'https://chatgpt.com') {
        throw new Error('cancelled');
      }
    }
    try {
      if (!await enabled(assertCurrent)) return null;
      const bytes = await file.arrayBuffer();
      assertCurrent();
      if (bytes.byteLength !== file.size) return null;
      // WebCrypto hashes off the JavaScript path; the existing 8 MiB file limit bounds the extra buffer.
      const digest = await root.crypto.subtle.digest('SHA-256', bytes);
      assertCurrent();
      if (digest.byteLength !== 32) return null;
      const hex = Array.from(new Uint8Array(digest), value => value.toString(16).padStart(2, '0')).join('');
      const response = await options.request(root, '/backend-api/files/library/reuse', {
        method: 'POST', credentials: 'include', redirect: 'error', headers, signal: controller.signal,
        body: JSON.stringify({ file_size_bytes: file.size, sha256_digest: hex }),
      }, { mode: 'json', timeoutMs: Math.max(1, Math.floor(deadline - now())), maxBytes: 16 * 1024 });
      assertCurrent();
      return reusable(response.payload, file);
    } catch (_) { return null; }
    finally { root.clearTimeout(timer); }
  }

  async function transfer(file, context, headers, owner) {
    if (!eligible(file, context)) return { kind: 'uploaded', result: await owner.upload(owner.signal) };
    const probe = new root.AbortController(), bytes = new root.AbortController();
    const abort = () => { probe.abort(); bytes.abort(); };
    owner.signal.addEventListener('abort', abort, { once: true });
    if (owner.signal.aborted) abort();
    let accepted = false;
    try {
      owner.assertCurrent();
      const pending = lookup(file, headers, probe, owner);
      const upload = Promise.resolve().then(() => owner.upload(bytes.signal)).then(result => {
        accepted = true; probe.abort(); return { kind: 'uploaded', result };
      });
      const hit = pending.then(value => value && !accepted && !probe.signal.aborted
        ? { kind: 'reused', file: value } : upload);
      const winner = await Promise.race([upload, hit]);
      owner.assertCurrent();
      if (winner.kind === 'uploaded') return winner;
      bytes.abort();
      try {
        // A hit alone is insufficient: only a confirmed cancellation may replace the byte transaction.
        return await upload;
      } catch (error) {
        owner.assertCurrent();
        if (owner.signal.aborted || error?.message !== 'cancelled') throw error;
        return winner;
      }
    } finally {
      abort();
      owner.signal.removeEventListener('abort', abort);
    }
  }

  return Object.freeze({ version: 3, transfer });
});
