(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 4, create: factory });
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

  function cancel() {
    if (!active) return;
    active.controller.abort();
    active.transport?.cancel();
  }

  async function start(raw, respond, changed, fallback) {
    if (active) return respond('request_attachment_upload', false, '附件上传尚未结束。');
    let descriptor;
    try { descriptor = JSON.parse(raw); } catch (_) {}
    // Compatibility selection is before any private write, never an automatic replay.
    if (!descriptor || !composer?.available() || !source || !root.__elonChatGptPrivateTransport ||
        !root.__elonChatGptPrivateAttachmentTransport) return fallback();
    if (/^image\//.test(descriptor.type) && !image?.available(descriptor)) return fallback();
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
      const binding = composer.capture();
      if (descriptor.documentToken !== binding.token || descriptor.href !== binding.href) throw new Error('context_changed');
      // One low-frequency guard only while an explicit upload is in flight.
      timer = root.setInterval(() => { if (!composer.current(binding)) cancel(); }, 500);
      // Compatibility selection for unknown/unsupported scope precedes byte reads
      // and private writes. Cancelled or stale bindings throw instead of replaying.
      if (!await composer.prepare(binding, job.controller.signal)) return fallback();
      let file = await source.read(descriptor, job.controller.signal);
      let imageDimensions;
      if (/^image\//.test(file.type)) {
        const prepared = await image.prepare(file, descriptor, job.controller.signal);
        file = prepared.file;
        imageDimensions = prepared.dimensions;
      }
      if (job.controller.signal.aborted || !composer.current(binding)) throw new Error('context_changed');
      job.transport = createTransport({ isCurrent: candidate => candidate === binding &&
        !job.controller.signal.aborted && composer.current(binding) });
      job.attempted = true;
      const result = await job.transport.upload(file, {
        useCase: imageDimensions ? 'multimodal' : 'ace_upload', storeInLibrary: false,
        libraryPersistenceMode: binding.isTemporaryChat ? undefined : 'required',
        isTemporaryChat: binding.isTemporaryChat, indexForRetrieval: false, imageDimensions,
      }, binding);
      if (!result.ok) throw new Error(result.code);
      if (job.controller.signal.aborted) throw new Error('cancelled');
      composer.associate(binding, file, result, descriptor.leaseId);
      respond('request_attachment_upload', true, 'private_attachment_associated');
      changed(true);
    } catch (_) {
      respond('request_attachment_upload', false, job.attempted
        ? '附件未能确认关联到当前会话，文字尚未自动重发，请检查附件后重试。'
        : '附件连接尚未就绪或会话已变化，请重试。');
    } finally {
      root.clearInterval(timer);
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

  return Object.freeze({ version: 4, start, cancel, remove,
    merge: dom => composer?.merge(dom) || dom });
});
