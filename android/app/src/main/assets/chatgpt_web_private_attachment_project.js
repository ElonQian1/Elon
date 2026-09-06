(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 2, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateAttachmentProject = exported;
})(typeof window === 'object' ? window : null, function (root, options) {
  'use strict';
  const PROJECT = /^g-p-[a-f0-9]{32}$/i;
  const PATH = /^\/g\/(g-p-[a-f0-9]{32})(?:-[A-Za-z0-9_-]{1,124})?\/project$/i;
  const UUID = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;
  const RUNTIME_URL = 'https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js';
  let runtime;

  function projectId(path) { return PATH.exec(path || '')?.[1] || null; }

  async function read(binding, signal) {
    const id = binding.projectId;
    const request = root.__elonChatGptPrivateJsonRequest?.request;
    if (!PROJECT.test(id || '') || binding.isTemporaryChat || signal?.aborted ||
        root.location.href !== binding.href || typeof request !== 'function') return null;
    const headers = root.__elonChatGptPrivateTransport?.copySameOriginRequestHeaders?.();
    const result = await request(root, '/backend-api/gizmos/' + encodeURIComponent(id), {
      method: 'GET', credentials: 'same-origin', cache: 'no-store', redirect: 'error', headers, signal,
    }, { timeoutMs: 7000, maxBytes: 1024 * 1024 });
    if (signal?.aborted || root.location.href !== binding.href) throw new Error('composer_changed');
    const gizmo = result.payload?.gizmo;
    if (!gizmo || gizmo.id !== id || typeof gizmo.current_user_permission?.can_write !== 'boolean' ||
        gizmo.use_injest_path !== undefined && typeof gizmo.use_injest_path !== 'boolean') return null;
    // Do not turn an unreadable permission or a read-only member into a project write.
    if (!gizmo.current_user_permission.can_write) return false;
    return Object.freeze({ projectId: id, usesInjestPath: gizmo.use_injest_path === true });
  }

  async function captureThread(binding, expectedProjectId, signal) {
    const samePage = () => root.location.href === binding.href &&
      root.__elonChatGptDocumentToken === binding.token;
    const unchanged = () => !signal?.aborted && samePage();
    if (!UUID.test(binding.conversationId || '') || !PROJECT.test(expectedProjectId || '') ||
        binding.isTemporaryChat || !unchanged()) throw new Error('composer_context_unavailable');
    let timer, abort;
    try {
      if (!runtime) {
        const loaded = root.performance?.getEntriesByName?.(RUNTIME_URL, 'resource')?.length > 0 ||
          !!root.document?.querySelector?.('link[rel="modulepreload"][href="' + RUNTIME_URL + '"]');
        if (!loaded) throw new Error('composer_context_unavailable');
        // Bind only the inspected, already-loaded official module; never import a
        // guessed replacement or create a second state store from copied code.
        const load = options?.loadRuntime || (url => import(url));
        runtime = Promise.resolve().then(() => load(RUNTIME_URL));
        runtime.catch(() => { runtime = null; });
      }
      const namespace = await Promise.race([runtime, new Promise((_, reject) => {
        abort = () => reject(new Error('cancelled'));
        signal?.addEventListener('abort', abort, { once: true });
        timer = root.setTimeout(() => reject(new Error('composer_context_timeout')), 1500);
      })]);
      if (!unchanged()) throw new Error('composer_changed');
      const selectors = namespace?.HM;
      if (typeof namespace?.XM !== 'function' || typeof selectors?.getGizmoId !== 'function' ||
          typeof selectors.getCurrentLeafId !== 'function' || typeof selectors.hasNode !== 'function') {
        throw new Error('composer_context_unavailable');
      }
      const selectedLeaf = () => {
        if (!samePage()) return null;
        const thread = namespace.XM(binding.conversationId);
        if (!thread || thread.isLoading !== false || thread.is_do_not_remember !== false ||
            selectors.getGizmoId(thread) !== expectedProjectId) return null;
        const id = selectors.getCurrentLeafId(thread);
        return UUID.test(id || '') && selectors.hasNode(thread, id) === true ? id : null;
      };
      const leafId = selectedLeaf();
      if (!leafId) throw new Error('composer_context_unavailable');
      return Object.freeze({ conversationId: binding.conversationId, projectId: expectedProjectId, leafId,
        current: () => { try { return selectedLeaf() === leafId; } catch (_) { return false; } } });
    } finally {
      root.clearTimeout(timer);
      if (abort) signal?.removeEventListener('abort', abort);
    }
  }

  function supports(scope, file) {
    if (!PROJECT.test(scope?.projectId || '')) return false;
    if (file?.type === 'text/plain') return true;
    // Image indexing under the ingest path is controlled by an account feature flag.
    // Until that flag is bound to current identity, retain the existing upload path.
    return ['image/jpeg', 'image/png', 'image/webp'].includes(file?.type) && !scope.usesInjestPath;
  }

  function uploadContext(scope, file, imageDimensions) {
    if (!supports(scope, file)) throw new Error('unsupported_upload_context');
    return Object.freeze({
      useCase: imageDimensions ? 'multimodal' : 'gizmo',
      gizmoId: imageDimensions ? undefined : scope.projectId,
      isProjectThread: true, isTemporaryChat: false, storeInLibrary: false,
      libraryPersistenceMode: 'required',
      indexForRetrieval: scope.usesInjestPath && /\.(?:xls|xlsx|csv)$/i.test(file.name),
      libraryFileInfo: Object.freeze({ gizmo_id: scope.projectId, is_project: true, should_upload_to_project: true,
        ...(scope.thread ? { origination_thread_id: scope.thread.conversationId,
          origination_message_id: scope.thread.leafId } : {}) }),
      imageDimensions,
    });
  }

  return Object.freeze({ version: 2, projectId, read, captureThread, supports, uploadContext });
});
