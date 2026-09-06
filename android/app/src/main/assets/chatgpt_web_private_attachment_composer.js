(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 3, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateAttachmentComposer = exported;
})(typeof window === 'object' ? window : null, function (root, options) {
  'use strict';
  options = options || {};
  let owned = null;
  const confirmed = new WeakSet();

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
      const ordinaryRoute = url.pathname === '/' || /^\/c\/[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i.test(url.pathname);
      if (url.origin !== 'https://chatgpt.com' || !ordinaryRoute || url.search || url.hash) return false;
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
    const path = new URL(root.location.href).pathname;
    const binding = Object.freeze({ store: resolveStore(), href: root.location.href, token, account,
      model: model(), path, conversationId: path === '/' ? null : path.slice(3) });
    if (binding.conversationId === null) confirmed.add(binding);
    return binding;
  }

  async function prepare(binding, signal) {
    if (!current(binding) || signal?.aborted) throw new Error('composer_changed');
    if (confirmed.has(binding)) return true;
    const read = root.__elonChatGptPrivateTransport?.readAttachmentContext;
    if (typeof read !== 'function') return null;
    let timer, abort;
    try {
      const context = await Promise.race([
        read(binding.path),
        new Promise((_, reject) => {
          abort = () => reject(new Error('cancelled'));
          signal?.addEventListener('abort', abort, { once: true });
          timer = root.setTimeout(() => reject(new Error('composer_context_timeout')), 10000);
        }),
      ]);
      if (!current(binding) || signal?.aborted || !available()) throw new Error('composer_changed');
      if (context?.conversationId !== binding.conversationId || typeof context.ordinary !== 'boolean') {
        throw new Error('composer_context_unavailable');
      }
      if (!context.ordinary) return false;
      confirmed.add(binding);
      return true;
    } catch (error) {
      if (!current(binding) || signal?.aborted || !available()) throw error;
      // Unknown metadata cannot authorize a private write, nor remove the
      // existing upload capability. The caller may select compatibility now.
      return null;
    } finally {
      root.clearTimeout(timer);
      if (abort) signal?.removeEventListener('abort', abort);
    }
  }

  function current(binding, checkModel = true) {
    try {
      return !!binding && root.location.href === binding.href &&
        root.__elonChatGptDocumentToken === binding.token && identity() === binding.account &&
        (!checkModel || model() === binding.model) && resolveStore() === binding.store;
    } catch (_) { return false; }
  }

  function associate(binding, file, result, leaseId) {
    if (!current(binding) || !confirmed.has(binding) || result?.ok !== true || result.associated !== false ||
        result.binding !== binding || result.stage !== 'processed' ||
        !/^[A-Za-z0-9_-]{1,160}$/.test(result.fileId || '') || result.fileSize !== file.size ||
        result.fileName !== file.name || result.mimeType !== file.type) throw new Error('association_invalid');
    const store = binding.store;
    if (store.files$().length !== 0 || store.hasUploadInProgress$()) throw new Error('composer_changed');
    const tempId = 'native_upload_' + leaseId;
    const metadata = result.metadata || {};
    const spec = { name: file.name, id: result.fileId, size: file.size, isBigPaste: false, mimeType: file.type };
    if (/^image\//.test(file.type)) {
      Object.assign(spec, root.__elonChatGptPrivateAttachmentProtocol.imageDimensions(result.imageDimensions));
    }
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

  return Object.freeze({ version: 3, available, capture, prepare, current, associate, merge, remove });
});
