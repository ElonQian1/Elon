(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 15, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateAttachmentComposer = exported;
})(typeof window === 'object' ? window : null, function (root, options) {
  'use strict';
  options = options || {};
  let owned = null;
  const confirmed = new WeakSet();
  const projects = new WeakMap();
  const project = root.__elonChatGptPrivateAttachmentProject?.create(root);

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

  function composerPolicy() {
    const input = root.document.querySelector('#upload-files');
    if (!input?.isConnected) return null;
    const key = Object.keys(input).find(name => name.startsWith('__reactFiber$'));
    const candidates = new Map();
    // The host node can still reference React's previous alternate. Accept only
    // a branch that reaches its root's current pointer, never work-in-progress props.
    for (const start of [input[key], input[key]?.alternate]) {
      const ancestors = [];
      for (let fiber = start; fiber && ancestors.length < 90; fiber = fiber.return) ancestors.push(fiber);
      const top = ancestors.at(-1);
      if (!top || top.return || top.stateNode?.current !== top) continue;
      for (const fiber of ancestors) {
        const props = fiber.memoizedProps;
        if (!props?.conversation || typeof props.conversation !== 'object' ||
            typeof props.onCreateNewCompletion !== 'function') continue;
        // Official file-drop handler receives currentModelId ?? currentModelConfig.id.
        const slug = props.currentModelId ?? props.currentModelConfig?.id;
        if (typeof slug !== 'string' || !/^[a-z0-9][a-z0-9._-]{0,127}$/i.test(slug)) return null;
        const policy = { modelSlug: slug,
          libraryEnabled: props.entrySurface === 'chat_composer' && props.isLibraryEnabled === true };
        candidates.set(JSON.stringify(policy), policy);
      }
    }
    return candidates.size === 1 ? candidates.values().next().value : null;
  }

  function route() {
    const url = new URL(root.location.href);
    const existing = /^(?:\/g\/(g-p-[a-f0-9]{32})(?:-[A-Za-z0-9_-]{1,124})?)?\/c\/([a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12})$/i.exec(url.pathname);
    const projectId = project?.projectId(url.pathname) || existing?.[1] || null;
    const supported = projectId || url.pathname === '/' || existing;
    const query = Array.from(url.searchParams.entries());
    // This is the official temporary-chat signal's source, not a cached UI label.
    const isTemporaryChat = query.length === 1 && query[0][0] === 'temporary-chat' && query[0][1] === 'true';
    if (url.origin !== 'https://chatgpt.com' || url.username || url.password || !supported ||
        url.hash || query.length && (!isTemporaryChat || projectId)) throw new Error('composer_context_unavailable');
    return { path: url.pathname, conversationId: existing?.[2] || null,
      isTemporaryChat, projectId };
  }

  function available() {
    try {
      route();
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
    const policy = composerPolicy();
    const binding = Object.freeze({ store: resolveStore(), href: root.location.href, token, account,
      model: model(), modelSlug: policy?.modelSlug ?? null, libraryEnabled: policy?.libraryEnabled === true, ...route() });
    if (binding.conversationId === null && !binding.projectId) confirmed.add(binding);
    return binding;
  }

  async function prepare(binding, signal, descriptor, refreshScope = false) {
    if (!current(binding) || signal?.aborted) throw new Error('composer_changed');
    if (root.__elonChatGptPrivateAttachmentProtocol?.isPdf(descriptor) && !binding.modelSlug) return null;
    if (refreshScope && binding.conversationId) { confirmed.delete(binding); projects.delete(binding); }
    if (confirmed.has(binding)) return true;
    const read = binding.projectId && !binding.conversationId ? () => project?.read(binding, signal, descriptor)
      : root.__elonChatGptPrivateTransport?.readAttachmentContext;
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
      if (binding.projectId && !binding.conversationId) {
        if (!context || !project.supports(context, descriptor)) return context === false ? false : null;
        projects.set(binding, context);
        confirmed.add(binding);
        return true;
      }
      if (context?.conversationId !== binding.conversationId) throw new Error('composer_context_unavailable');
      if (context?.projectId) {
        if (!project || binding.isTemporaryChat || binding.projectId && binding.projectId !== context.projectId) return null;
        const thread = await project.captureThread(binding, context.projectId, signal);
        if (!Array.isArray(context.nodeIds) || !context.nodeIds.includes(thread.leafId)) return null;
        // Keep the branch guard active even when the permission request fails.
        projects.set(binding, Object.freeze({ projectId: context.projectId, thread }));
        const scope = await project.read({ ...binding, projectId: context.projectId }, signal, descriptor);
        if (!current(binding) || signal?.aborted || !available() || !thread.current()) throw new Error('composer_changed');
        if (!scope || !project.supports(scope, descriptor)) return scope === false ? false : null;
        projects.set(binding, Object.freeze({ ...scope, thread }));
        confirmed.add(binding);
        return true;
      }
      if (binding.projectId) return null;
      const supported = binding.isTemporaryChat ? context?.temporary : context?.ordinary;
      if (context?.conversationId !== binding.conversationId || typeof supported !== 'boolean') {
        throw new Error('composer_context_unavailable');
      }
      if (!supported) return false;
      confirmed.add(binding);
      return true;
    } catch (error) {
      if (error?.message === 'composer_changed' || !current(binding) || signal?.aborted || !available()) throw error;
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
      const policy = checkModel ? composerPolicy() : null;
      return !!binding && root.location.href === binding.href &&
        root.__elonChatGptDocumentToken === binding.token && identity() === binding.account &&
        (!checkModel || model() === binding.model && (policy?.modelSlug ?? null) === binding.modelSlug &&
          (policy?.libraryEnabled === true) === binding.libraryEnabled &&
          projects.get(binding)?.thread?.current() !== false) &&
        resolveStore() === binding.store;
    } catch (_) { return false; }
  }

  function uploadContext(binding, file, imageDimensions) {
    if (!current(binding) || !confirmed.has(binding)) throw new Error('composer_changed');
    const library = !binding.isTemporaryChat && binding.libraryEnabled;
    const context = projects.has(binding) ? project.uploadContext(projects.get(binding), file, imageDimensions)
      : { useCase: imageDimensions ? 'multimodal' : 'ace_upload', storeInLibrary: library,
      libraryPersistenceMode: binding.isTemporaryChat ? undefined : library ? 'opportunistic' : 'required',
      isTemporaryChat: binding.isTemporaryChat, indexForRetrieval: false, imageDimensions };
    return { ...context, modelSlug: binding.modelSlug ?? undefined };
  }

  function reservationContext(binding, descriptor) {
    if (!current(binding) || !confirmed.has(binding) || projects.has(binding) ||
        binding.projectId || !Number.isSafeInteger(descriptor?.size) || descriptor.size < 1 ||
        descriptor.size > root.__elonChatGptPrivateAttachmentProtocol.maxFileBytes) return null;
    const image = ['image/jpeg', 'image/png', 'image/webp'].includes(descriptor.type);
    if (!image && !root.__elonChatGptPrivateAttachmentProtocol.isDocument(descriptor)) return null;
    return pickerReservationContext(binding, image ? 'image' : 'document');
  }

  function pickerReservationContext(binding, kind) {
    if (!['image', 'document'].includes(kind) || !current(binding) || !confirmed.has(binding) ||
        projects.has(binding) || binding.projectId) return null;
    const library = !binding.isTemporaryChat && binding.libraryEnabled;
    return Object.freeze({ useCase: kind === 'image' ? 'multimodal' : 'ace_upload', storeInLibrary: library,
      libraryPersistenceMode: binding.isTemporaryChat ? undefined : library ? 'opportunistic' : 'required',
      isTemporaryChat: binding.isTemporaryChat, modelSlug: binding.modelSlug ?? undefined });
  }

  function associate(binding, file, result, leaseId) {
    const scope = projects.get(binding);
    const projectId = scope?.projectId || binding.projectId;
    const metadata = result?.metadata || {};
    const reused = result?.stage === 'reused';
    if (reused && (binding.isTemporaryChat || projectId || !binding.libraryEnabled ||
        metadata.libraryPersistenceResult !== 'library' ||
        !/^[A-Za-z0-9_-]{1,160}$/.test(metadata.libraryFileId || '') ||
        typeof result.reusedFileName !== 'string' || !result.reusedFileName.trim() ||
        result.reusedFileName.length > 120 || /[\x00-\x1f\x7f/\\]/.test(result.reusedFileName))) {
      throw new Error('association_invalid');
    }
    if (!current(binding) || !confirmed.has(binding) || result?.ok !== true || result.associated !== false ||
        result.binding !== binding || !['processed', 'reused'].includes(result.stage) || result.isTemporaryChat !== binding.isTemporaryChat ||
        (result.projectId || null) !== projectId ||
        (scope && result.projectWriteRequested !== scope.canWrite) ||
        !/^[A-Za-z0-9_-]{1,160}$/.test(result.fileId || '') || result.fileSize !== file.size ||
        result.fileName !== file.name || result.mimeType !== file.type) throw new Error('association_invalid');
    const store = binding.store;
    if (store.files$().length !== 0 || store.hasUploadInProgress$()) throw new Error('composer_changed');
    const tempId = 'native_upload_' + leaseId;
    const spec = { name: reused ? result.reusedFileName : file.name, id: result.fileId,
      size: file.size, isBigPaste: false, mimeType: file.type };
    if (/^image\//.test(file.type)) {
      Object.assign(spec, root.__elonChatGptPrivateAttachmentProtocol.imageDimensions(result.imageDimensions));
    }
    if (Number.isSafeInteger(metadata.fileTokenSize)) spec.fileTokenSize = metadata.fileTokenSize;
    if (['temporary', 'library'].includes(metadata.libraryPersistenceResult)) {
      spec.libraryPersistenceResult = metadata.libraryPersistenceResult;
    }
    if (metadata.libraryPersistenceResult !== 'temporary' && metadata.libraryFileId) spec.libraryFileId = metadata.libraryFileId;
    const libraryFileInfo = scope ? project.uploadContext(scope, file, result.imageDimensions).libraryFileInfo : undefined;
    const attached = {
      tempId, file, fileSignature: JSON.stringify({ name: file.name, size: file.size,
        lastModified: file.lastModified, type: file.type }),
      status: 'ready', progress: 100, fileId: result.fileId, cdnUrl: null, fileSpec: spec,
      source: reused ? 'library' : 'local', ...(reused ? { autoReused: true } : {}),
      storeInLibrary: !binding.isTemporaryChat && !projectId && binding.libraryEnabled,
      isTemporaryChat: binding.isTemporaryChat, isProjectThread: !!projectId,
      ...(scope?.canWrite ? { projectGizmoId: projectId } : {}),
      ...(libraryFileInfo ? { libraryFileInfo } : {}),
      ...(spec.libraryFileId ? { libraryFileId: spec.libraryFileId } : {}),
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

  function prepareSubmit(store) {
    try {
      const value = attachedNow();
      if (!value || store !== value.binding.store || !current(value.binding)) return null;
      const { attached, binding } = value, file = attached.file;
      const fingerprint = () => JSON.stringify({ ...attached, file: undefined });
      const metadata = fingerprint();
      // The official prepared_action accepts ready entries, not just file IDs.
      // Keep File identity, but detach and freeze its small metadata tree.
      function freeze(data) {
        for (const child of Object.values(data)) if (child && typeof child === 'object') freeze(child);
        return Object.freeze(data);
      }
      const readyFiles = Object.freeze([Object.freeze({ ...freeze(JSON.parse(metadata)), file })]);
      let consumed = false;
      function unchanged() { return attached.file === file && fingerprint() === metadata; }
      function currentLease() {
        try {
          const files = store.files$(), ready = store.readyFiles$();
          return !consumed && owned === value && current(binding) && unchanged() &&
            Array.isArray(files) && files.length === 1 && files[0] === attached &&
            Array.isArray(ready) && ready.length === 1 && ready[0] === attached &&
            store.hasUploadInProgress$() === false;
        } catch (_) { return false; }
      }
      function consumeAccepted() {
        try {
          // The caller must first confirm official dispatch and conversation ownership.
          // New-thread navigation can already have cleared the display-only owner.
          if (consumed || root.__elonChatGptDocumentToken !== binding.token ||
              identity() !== binding.account || resolveStore() !== store || !unchanged()) return false;
          const files = store.files$();
          if (!Array.isArray(files)) return false;
          if (files.includes(attached)) {
            store.files$.set(files.filter(item => item !== attached));
            if (store.files$().includes(attached)) return false;
          }
          consumed = true;
          if (owned === value) owned = null;
          return true;
        } catch (_) { return false; }
      }
      return currentLease() ? Object.freeze({ readyFiles, current: currentLease, consumeAccepted }) : null;
    } catch (_) { return null; }
  }

  return Object.freeze({ version: 15, available, capture, prepare, current, uploadContext, reservationContext, pickerReservationContext, associate, merge, remove, prepareSubmit });
});
