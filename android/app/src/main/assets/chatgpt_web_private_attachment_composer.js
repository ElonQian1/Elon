(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateAttachmentComposer = exported;
})(typeof window === 'object' ? window : null, function (root, options) {
  'use strict';
  options = options || {};
  let owned = null;

  function storeFromInput() {
    const input = root.document.querySelector('#upload-files');
    if (!input?.isConnected) return null;
    const key = Object.keys(input).find(name => name.startsWith('__reactFiber$'));
    const stores = new Set();
    function accept(value) {
      if (typeof value?.files$ === 'function' && typeof value.files$.set === 'function' &&
          typeof value.readyFiles$ === 'function' && typeof value.hasUploadInProgress$ === 'function') {
        stores.add(value);
      }
    }
    // The observed official FilePickerContext uses a callable signal, not React setState.
    for (let fiber = input[key], depth = 0; fiber && depth < 90; fiber = fiber.return, depth++) {
      accept(fiber.memoizedProps?.value);
      let dependency = fiber.dependencies?.firstContext;
      for (let count = 0; dependency && count < 30; dependency = dependency.next, count++) {
        accept(dependency.memoizedValue);
      }
    }
    return stores.size === 1 ? stores.values().next().value : null;
  }
  const resolveStore = options.resolveStore || storeFromInput;

  function identity() {
    const headers = root.__elonChatGptPrivateTransport?.copySameOriginRequestHeaders?.();
    const normalized = {};
    for (const [key, value] of Object.entries(headers || {})) normalized[key.toLowerCase()] = value;
    if (!/^Bearer\s+\S{8,65536}$/.test(normalized.authorization || '')) return null;
    // This value stays in this page's closure. It is never part of a native receipt or log.
    return JSON.stringify(['authorization', 'chatgpt-account-id', 'oai-device-id'].map(key => normalized[key] || ''));
  }

  function model() {
    return root.__elonChatGptComposer?.currentModel?.(root.document.querySelector('#prompt-textarea')) || '';
  }

  function available() {
    try {
      const url = new URL(root.location.href);
      // First integration covers the verified plain new-chat context only. Project,
      // temporary and existing-thread upload contexts need their own confirmed contract.
      if (url.origin !== 'https://chatgpt.com' || url.pathname !== '/' || url.search || url.hash) return false;
      const store = resolveStore();
      const files = store?.files$();
      return !!store && store.skipChatAttachmentLimits !== true && Array.isArray(files) &&
        files.length === 0 && store.hasUploadInProgress$() === false;
    } catch (_) { return false; }
  }

  function capture() {
    if (!available()) throw new Error('composer_context_unavailable');
    const token = root.__elonChatGptDocumentToken;
    const account = identity();
    if (!/^doc_[a-z0-9_]{3,80}$/.test(token || '') || !account) throw new Error('composer_context_unavailable');
    return Object.freeze({ store: resolveStore(), href: root.location.href, token, account, model: model() });
  }

  function current(binding, checkModel = true) {
    try {
      return !!binding && root.location.href === binding.href &&
        root.__elonChatGptDocumentToken === binding.token && identity() === binding.account &&
        (!checkModel || model() === binding.model) && resolveStore() === binding.store;
    } catch (_) { return false; }
  }

  function associate(binding, file, result, leaseId) {
    if (!current(binding) || result?.ok !== true || result.associated !== false ||
        result.binding !== binding || result.stage !== 'processed' ||
        !/^[A-Za-z0-9_-]{1,160}$/.test(result.fileId || '') || result.fileSize !== file.size ||
        result.fileName !== file.name || result.mimeType !== file.type) throw new Error('association_invalid');
    const store = binding.store;
    if (store.files$().length !== 0 || store.hasUploadInProgress$()) throw new Error('composer_changed');
    const tempId = 'native_upload_' + leaseId;
    const metadata = result.metadata || {};
    const spec = { name: file.name, id: result.fileId, size: file.size, isBigPaste: false, mimeType: file.type };
    if (Number.isSafeInteger(metadata.fileTokenSize)) spec.fileTokenSize = metadata.fileTokenSize;
    const attached = {
      tempId, file, fileSignature: JSON.stringify({ name: file.name, size: file.size,
        lastModified: file.lastModified, type: file.type }),
      status: 'ready', progress: 100, fileId: result.fileId, cdnUrl: null, fileSpec: spec,
      source: 'local', storeInLibrary: false, isTemporaryChat: false, isProjectThread: false,
    };
    store.files$.set([attached]);
    const ready = store.readyFiles$();
    if (!current(binding) || !Array.isArray(ready) || !ready.some(item =>
      item === attached && item.fileSpec?.id === result.fileId && item.status === 'ready'
    )) {
      // Roll back only our exact local object. Never overwrite concurrent user files.
      const files = store.files$();
      if (Array.isArray(files)) store.files$.set(files.filter(item => item !== attached));
      throw new Error('association_unconfirmed');
    }
    owned = { binding, attached, id: 'private_attachment_' + leaseId };
    return { associated: true };
  }

  function attachedNow() {
    if (!owned) return null;
    if (!current(owned.binding, false)) { owned = null; return null; }
    const ready = owned.binding.store.readyFiles$();
    if (!Array.isArray(ready) || !ready.some(item => item === owned.attached &&
      item.fileSpec?.id === owned.attached.fileId && item.status === 'ready')) owned = null;
    return owned;
  }

  function merge(dom) {
    try {
      const value = attachedNow();
      if (!value) return dom;
      const name = value.attached.file.name;
      return [...dom.filter(item => !String(item.name || '').includes(name)),
        { id: value.id, name, state: 'ready', removable: true }];
    } catch (_) { return dom; }
  }

  function remove(id) {
    const value = attachedNow();
    if (!value || value.id !== id) return false;
    const store = value.binding.store;
    store.files$.set(store.files$().filter(item => item !== value.attached));
    if (store.files$().includes(value.attached)) throw new Error('attachment_remove_unconfirmed');
    owned = null;
    return true;
  }

  return Object.freeze({ version: 1, available, capture, current, associate, merge, remove });
});
