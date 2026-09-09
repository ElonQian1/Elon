(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 20, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com' &&
      !(Number(root.__elonChatGptPrivateAttachmentSend?.version) >= exported.version)) {
    root.__elonChatGptPrivateAttachmentSend?.cancel();
    root.__elonChatGptPrivateAttachmentSend = factory(root);
  }
})(typeof window === 'object' ? window : null, function (root, options) {
  'use strict';
  options = options || {};
  const composer = options.composer || root.__elonChatGptPrivateAttachmentComposer?.create(root);
  const source = options.source || root.__elonChatGptNativeAttachmentSource?.create(root);
  const image = options.image || root.__elonChatGptPrivateAttachmentImage?.create(root);
  const createTransport = options.createTransport || (config => root.__elonChatGptPrivateAttachmentTransport.create(root, config));
  let active = null;
  const library = root.__elonChatGptPrivateLibraryAttachment?.create(root, { composer, busy: () => !!active });
  const selections = root.__elonChatGptPrivateAttachmentSelection?.create(root, {
    composer, createTransport, busy: () => !!active || library?.busy() === true,
  });

  function suspend() {
    library?.cancel();
    if (!active) return;
    active.controller.abort();
    active.transport?.cancel();
  }

  function cancel() { selections?.cancel(); suspend(); }

  async function start(raw, respond, changed, fallback) {
    if (active || library?.busy()) return respond('request_attachment_upload', false, '附件上传尚未结束。');
    let descriptor;
    try { descriptor = JSON.parse(raw); } catch (_) {}
    const batch = descriptor?.version === 2;
    const descriptors = batch ? descriptor.files : [descriptor];
    if (batch && (!Array.isArray(descriptors) || !descriptors.length || descriptors.length > 9 ||
        descriptors.some(file => file?.version !== 1 || file.href !== descriptor.href ||
          file.documentToken !== descriptor.documentToken || !/^[a-f0-9-]{36}$/.test(file.leaseId || '') ||
          !Number.isSafeInteger(file.size) || file.size < 1 || file.size > 8 * 1024 * 1024 ||
          typeof file.name !== 'string' || !file.name.trim() || file.name.length > 120 ||
          /[\x00-\x1f\x7f/\\]/.test(file.name)) ||
        new Set(descriptors.map(file => file.leaseId)).size !== descriptors.length) ||
        Array.isArray(descriptors) && descriptors.some(file => file && typeof file === 'object' &&
          'uploadCopy' in file && typeof file.uploadCopy !== 'boolean')) {
      selections?.cancel();
      return respond('request_attachment_upload', false, '附件上传选项无效，请重新选择。');
    }
    const uploadCopy = descriptors.some(file => file?.uploadCopy === true);
    const unavailable = () => {
      selections?.cancel();
      return uploadCopy ? respond('request_attachment_upload', false,
        '尚未能重新上传这份附件，文字未发送，请重试。') : fallback();
    };
    if (uploadCopy) selections?.cancel();
    // Compatibility selection is before any private write, never an automatic replay.
    if (!descriptor || !composer?.available() || !source || !root.__elonChatGptPrivateTransport ||
        !root.__elonChatGptPrivateAttachmentTransport || batch && typeof composer.associateMany !== 'function') return unavailable();
    if (batch && descriptors.some(file => !root.__elonChatGptPrivateAttachmentProtocol?.isDocument(file) &&
        !['image/jpeg', 'image/png', 'image/webp'].includes(file.type))) return unavailable();
    if (descriptors.some(file => /^image\//.test(file?.type) && !image?.available(file))) return unavailable();
    if (batch) selections?.cancel();
    const job = { controller: new root.AbortController(), transport: null, attempted: false };
    active = job;
    let timer;
    try {
      let abortListener;
      let authTimer;
      try {
        await Promise.race([
          root.__elonChatGptPrivateTransport.acquireSameOriginRequestHeaders(),
          new Promise((_, reject) => {
            abortListener = () => reject(new Error('cancelled'));
            job.controller.signal.addEventListener('abort', abortListener, { once: true });
            authTimer = root.setTimeout(() => reject(new Error('auth_timeout')), 7000);
          }),
        ]);
      } finally {
        root.clearTimeout(authTimer);
        if (abortListener) job.controller.signal.removeEventListener('abort', abortListener);
      }
      if (job.controller.signal.aborted) throw new Error('cancelled');
      const selected = uploadCopy || batch ? null : selections?.take(descriptor);
      if (selected) { job.controller.abort(); job.controller = selected.controller; job.transport = selected.transport; }
      const binding = selected?.binding || composer.capture();
      if (descriptor.documentToken !== binding.token || descriptor.href !== binding.href) throw new Error('context_changed');
      // One low-frequency guard only while an explicit upload is in flight.
      timer = root.setInterval(() => { if (!composer.current(binding)) cancel(); }, 500);
      // Compatibility selection for unknown/unsupported scope precedes byte reads
      // and private writes. Cancelled or stale bindings throw instead of replaying.
      for (const item of descriptors) {
        if (!await composer.prepare(binding, job.controller.signal, item, !!selected)) return unavailable();
      }
      const completed = [];
      for (const item of descriptors) {
        if (job.controller.signal.aborted || !composer.current(binding) || !composer.available()) throw new Error('context_changed');
        job.transport = job.transport || createTransport({ isCurrent: candidate => candidate === binding &&
          !job.controller.signal.aborted && composer.current(binding) });
        if (!selected && !item.uploadCopy) job.transport.prefetch?.(composer.reservationContext?.(binding, item), binding, job.controller.signal);
        let file = await source.read(item, job.controller.signal);
        let imageDimensions;
        if (/^image\//.test(file.type)) {
          const prepared = await image.prepare(file, item, job.controller.signal);
          file = prepared.file;
          imageDimensions = prepared.dimensions;
        }
        if (job.controller.signal.aborted || !composer.current(binding)) throw new Error('context_changed');
        job.attempted = true;
        const context = { ...composer.uploadContext(binding, file, imageDimensions),
          ...(item.uploadCopy ? { checkForReusableLibraryFile: false } : {}) };
        const result = await job.transport.upload(file, context, binding);
        if (!result.ok) throw new Error(result.code);
        if (job.controller.signal.aborted) throw new Error('cancelled');
        completed.push({ file, result, leaseId: item.leaseId });
        job.transport.dispose();
        job.transport = null;
      }
      if (batch) composer.associateMany(binding, completed);
      else composer.associate(binding, completed[0].file, completed[0].result, descriptor.leaseId);
      respond('request_attachment_upload', true, 'private_attachment_associated');
      changed(true);
    } catch (_) {
      respond('request_attachment_upload', false, job.attempted
        ? '附件未能确认关联到当前会话，文字尚未自动重发，请检查附件后重试。'
        : '附件连接尚未就绪或会话已变化，请重试。');
    } finally {
      root.clearInterval(timer);
      selections?.cancel(descriptor.selectionId);
      job.controller.abort();
      job.transport?.dispose();
      if (active === job) active = null;
    }
  }

  function remove(id, respond, changed) {
    if (!String(id).startsWith('private_attachment_')) return false;
    let ok = false;
    try { ok = composer.remove(id); } catch (_) {}
    respond('remove_attachment', ok, ok ? '' : '附件状态已变化，请刷新后重试。');
    changed(true);
    return true;
  }

  return Object.freeze({ version: 20, start, cancel, suspend, remove,
    attachLibrary: (command, respond, changed) => {
      selections?.cancel();
      return library ? library.attach(command, respond, changed) : respond('attach_library_file', false, 'library_not_ready');
    },
    prepareSubmit: store => composer?.prepareSubmit?.(store) || null,
    beginSelection: raw => selections?.begin(raw) === true, cancelSelection: id => selections?.cancel(id),
    merge: dom => composer?.merge(dom) || dom });
});
