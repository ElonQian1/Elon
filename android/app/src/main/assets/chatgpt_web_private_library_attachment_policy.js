(function (root, prepare) {
  'use strict';
  const states = new WeakMap();
  const api = Object.freeze({ version: 2,
    prepare(page, binding) {
      const document = page.document, token = page.__elonChatGptDocumentToken;
      return prepare(page, binding, code => states.set(page, { document, token, code }));
    },
    state(page) {
      const value = states.get(page);
      return value?.document === page.document && value.token === page.__elonChatGptDocumentToken
        ? value.code : 'not_observed';
    }
  });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateLibraryAttachmentPolicy = api;
})(typeof window === 'object' ? window : null, async function (page, binding, report) {
  'use strict';
  const runtime = page.__elonChatGptPrivateRuntimeBindings;
  const ownerPath = page.__elonChatGptCommittedOwnerPath ||
    (typeof module === 'object' && module.exports ? require('./chatgpt_web_committed_owner_path') : null);
  const document = page.document, token = page.__elonChatGptDocumentToken;
  const reject = code => { report(code); return null; };
  let namespace;
  try { namespace = runtime?.peek('composer') || await runtime?.load('composer'); }
  catch (_) { return reject('runtime_unavailable'); }
  if (typeof namespace?.fh?.validateChatAttachment !== 'function') return reject('validator_unavailable');

  function limits() {
    if (page.document !== document || page.__elonChatGptDocumentToken !== token) return reject('document_changed');
    if (runtime.peek('composer') !== namespace) return reject('runtime_changed');
    if (binding.store.skipChatAttachmentLimits === true) return reject('limits_bypassed');
    const spec = runtime.tools(), node = document.querySelector('#composer-plus-btn') ||
      document.querySelector('[data-testid="composer-plus-btn"]');
    if (!spec) return reject('runtime_unavailable');
    if (!node?.isConnected) return reject('composer_detached');
    const key = Object.keys(node).find(name => name.startsWith('__reactFiber$'));
    const path = ownerPath?.resolve(node[key])?.ancestors || [];
    const owners = path.filter(fiber => fiber.type?.name === spec.owner);
    if (owners.length !== 1) return reject('owner_unavailable');
    const owner = owners[0], props = owner.memoizedProps;
    let dependency = owner.dependencies?.firstContext, matches = 0;
    for (let count = 0; dependency && count < 30; dependency = dependency.next, count++) {
      if (dependency.memoizedValue === binding.store) matches++;
    }
    if (matches !== 1) return reject('store_mismatch');
    if (!props?.conversation || props.composerDisabled !== false ||
        props.isTemporaryChat !== false || props.isProjectThread === true || props.gizmoEditorMode === true ||
        props.loginModalGate?.shouldGateToLoginModal) return reject('scope_mismatch');
    if ((props.currentModelId ?? props.currentModelConfig?.id) !== binding.modelSlug) return reject('model_mismatch');
    const upload = props.maxLibraryAttachmentCount, total = props.maxTotalLibraryAttachmentCount;
    if (!['maxLibraryAttachmentCount', 'maxTotalLibraryAttachmentCount'].every(key =>
      Object.prototype.hasOwnProperty.call(props, key))) return reject('limits_missing');
    // The official composer explicitly passes undefined when its quota banner is absent.
    // Preserve that value for the official validator; do not invent a replacement quota.
    if (![upload, total].every(value => value === undefined || Number.isSafeInteger(value) && value >= 0)) {
      return reject('limits_invalid');
    }
    report('ready');
    return { upload, total };
  }

  if (!limits()) return null;
  // Reuse the official validator and current computed menu limits. No modal is opened.
  return descriptor => {
    try {
      const policy = limits(), files = binding.store.files$();
      if (!policy) return false;
      if (!Array.isArray(files) || files.length >= 9) { report('attachment_limit'); return false; }
      const accepted = namespace.fh.validateChatAttachment(binding.store, descriptor.size, undefined,
        policy.upload, policy.total, files) === true;
      report(accepted ? 'ready' : 'attachment_limit');
      return accepted;
    } catch (_) { report('validator_error'); return false; }
  };
});
