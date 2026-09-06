(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateAttachmentProject = exported;
})(typeof window === 'object' ? window : null, function (root) {
  'use strict';
  const PROJECT = /^g-p-[a-f0-9]{32}$/i;
  const PATH = /^\/g\/(g-p-[a-f0-9]{32})(?:-[A-Za-z0-9_-]{1,124})?\/project$/i;

  function projectId(path) { return PATH.exec(path || '')?.[1] || null; }

  async function read(binding, signal) {
    const id = projectId(binding.path);
    const request = root.__elonChatGptPrivateJsonRequest?.request;
    if (!id || id !== binding.projectId || binding.isTemporaryChat || signal?.aborted ||
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
      libraryFileInfo: Object.freeze({ gizmo_id: scope.projectId, is_project: true, should_upload_to_project: true }),
      imageDimensions,
    });
  }

  return Object.freeze({ version: 1, projectId, read, supports, uploadContext });
});
