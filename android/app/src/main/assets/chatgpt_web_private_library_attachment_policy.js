(function (root, prepare) {
  'use strict';
  const states = new WeakMap();
  const api = Object.freeze({ version: 4,
    prepare(page, binding) {
      const document = page.document, token = page.__elonChatGptDocumentToken;
      return prepare(page, binding, code => states.set(page, { document, token, code }));
    },
    prepareUpload(page, binding) {
      const document = page.document, token = page.__elonChatGptDocumentToken;
      return prepare(page, binding, code => states.set(page, { document, token, code }), true);
    },
    state(page) {
      const value = states.get(page);
      return value?.document === page.document && value.token === page.__elonChatGptDocumentToken
        ? value.code : 'not_observed';
    }
  });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateLibraryAttachmentPolicy = api;
})(typeof window === 'object' ? window : null, async function (page, binding, report, localUpload) {
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
  let conversation, shared;
  if (localUpload) {
    try {
      conversation = runtime.peek('conversation') || await runtime.load('conversation');
      shared = runtime.peek('shared') || await runtime.load('shared');
    } catch (_) { return reject('runtime_unavailable'); }
    if (!['attachmentBaseLimit', 'attachmentMaxUploads', 'attachmentPendingCount', 'attachmentConfiguredLimit']
      .every(key => typeof conversation?.[key] === 'function') ||
      !['Multimodal', 'Interpreter'].every(key => Number.isSafeInteger(shared?.attachmentUploadType?.[key]))) {
      return reject('validator_unavailable');
    }
  }

  function limits() {
    if (page.document !== document || page.__elonChatGptDocumentToken !== token) return reject('document_changed');
    if (runtime.peek('composer') !== namespace) return reject('runtime_changed');
    if (localUpload && (runtime.peek('conversation') !== conversation || runtime.peek('shared') !== shared)) {
      return reject('runtime_changed');
    }
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
        props.isTemporaryChat !== false || props.gizmoEditorMode === true ||
        props.loginModalGate?.shouldGateToLoginModal) return reject('scope_mismatch');
    // Project recall is conditional in the official composer, not universally disabled.
    if (binding.libraryProjectId ? props.isProjectThread !== true || props.libraryEligibilityReason !== 'eligible'
      : props.isProjectThread === true) return reject('scope_mismatch');
    if ((props.currentModelId ?? props.currentModelConfig?.id) !== binding.modelSlug) return reject('model_mismatch');
    if (localUpload && props.isFileUploadEnabled !== true) return reject('upload_unavailable');
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
      if (localUpload) {
        const selected = Array.isArray(descriptor) ? descriptor : [descriptor];
        if (!Array.isArray(files) || !selected.length || files.length + selected.length > 9) {
          report('attachment_limit'); return false;
        }
        const prospective = files.slice();
        for (const file of selected) {
          const type = /^image\//.test(file.type) ? shared.attachmentUploadType.Multimodal : shared.attachmentUploadType.Interpreter;
          const base = conversation.attachmentBaseLimit(type), max = conversation.attachmentMaxUploads(type);
          const configured = conversation.attachmentConfiguredLimit(), count = conversation.attachmentPendingCount(prospective);
          if (![base, max, count].every(value => Number.isSafeInteger(value) && value >= 0) ||
              configured != null && (!Number.isSafeInteger(configured) || configured < 0)) {
            report('limits_invalid'); return false;
          }
          // Same checks as official uploadFile: live remaining total, then counted per-turn slots.
          const cap = policy.upload === undefined || configured == null ? null : Math.min(base, configured);
          if (prospective.length >= max || cap != null && count >= Math.min(max, cap) ||
              namespace.fh.validateChatAttachment(binding.store, file.size, undefined, undefined, undefined, prospective) !== true) {
            report('attachment_limit'); return false;
          }
          // Count only; never insert placeholders into the official FilePicker store.
          prospective.push({ isBigPaste: false });
        }
        report('ready'); return true;
      }
      if (!Array.isArray(files) || files.length >= 9) { report('attachment_limit'); return false; }
      const accepted = namespace.fh.validateChatAttachment(binding.store, descriptor.size, undefined,
        policy.upload, policy.total, files) === true;
      report(accepted ? 'ready' : 'attachment_limit');
      return accepted;
    } catch (_) { report('validator_error'); return false; }
  };
});
