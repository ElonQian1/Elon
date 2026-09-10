(function (root, prepare) {
  'use strict';
  const api = Object.freeze({ version: 1, prepare });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateLibraryAttachmentPolicy = api;
})(typeof window === 'object' ? window : null, async function (page, binding) {
  'use strict';
  const runtime = page.__elonChatGptPrivateRuntimeBindings;
  const ownerPath = page.__elonChatGptCommittedOwnerPath ||
    (typeof module === 'object' && module.exports ? require('./chatgpt_web_committed_owner_path') : null);
  const document = page.document, token = page.__elonChatGptDocumentToken;
  let namespace;
  try { namespace = runtime?.peek('composer') || await runtime?.load('composer'); }
  catch (_) { return null; }
  if (typeof namespace?.fh?.validateChatAttachment !== 'function') return null;

  function limits() {
    if (page.document !== document || page.__elonChatGptDocumentToken !== token ||
        runtime.peek('composer') !== namespace || binding.store.skipChatAttachmentLimits === true) return null;
    const spec = runtime.tools(), node = document.querySelector('#composer-plus-btn') ||
      document.querySelector('[data-testid="composer-plus-btn"]');
    if (!spec || !node?.isConnected) return null;
    const key = Object.keys(node).find(name => name.startsWith('__reactFiber$'));
    const path = ownerPath?.resolve(node[key])?.ancestors || [];
    const owners = path.filter(fiber => fiber.type?.name === spec.owner);
    if (owners.length !== 1) return null;
    const owner = owners[0], props = owner.memoizedProps;
    let dependency = owner.dependencies?.firstContext, matches = 0;
    for (let count = 0; dependency && count < 30; dependency = dependency.next, count++) {
      if (dependency.memoizedValue === binding.store) matches++;
    }
    if (matches !== 1 || !props?.conversation || props.composerDisabled !== false ||
        props.isTemporaryChat !== false || props.isProjectThread === true || props.gizmoEditorMode === true ||
        props.loginModalGate?.shouldGateToLoginModal ||
        (props.currentModelId ?? props.currentModelConfig?.id) !== binding.modelSlug) return null;
    const upload = props.maxLibraryAttachmentCount, total = props.maxTotalLibraryAttachmentCount;
    if (![upload, total].every(value => Number.isSafeInteger(value) && value >= 0)) return null;
    return { upload, total };
  }

  if (!limits()) return null;
  // Reuse the official validator and current computed menu limits. No modal is opened.
  return descriptor => {
    try {
      const policy = limits(), files = binding.store.files$();
      if (!policy || !Array.isArray(files) || files.length >= 9) return false;
      return namespace.fh.validateChatAttachment(binding.store, descriptor.size, undefined,
        policy.upload, policy.total, files) === true;
    } catch (_) { return false; }
  };
});
