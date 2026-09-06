(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 2, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptNativeAttachmentSource = exported;
})(typeof window === 'object' ? window : null, function (root) {
  'use strict';
  let active = null;
  let sequence = 0;
  const bridge = root.elonChatGptAttachmentSource;

  function current(descriptor) {
    return root.location?.href === descriptor.href &&
      root.__elonChatGptDocumentToken === descriptor.documentToken;
  }

  async function read(descriptor, signal) {
    if (active || typeof bridge?.postMessage !== 'function') throw new Error('native_source_unavailable');
    if (descriptor?.version !== 1 || !/^[a-f0-9-]{36}$/.test(descriptor.leaseId || '') ||
        !Number.isSafeInteger(descriptor.size) || descriptor.size < 1 || descriptor.size > 8 * 1024 * 1024 ||
        !['text/plain', 'image/jpeg', 'image/png', 'image/webp'].includes(descriptor.type) ||
        !current(descriptor)) throw new Error('native_source_invalid');
    const job = {};
    active = job;
    const previous = bridge.onmessage;
    const parts = [];
    let pending = null;
    const abort = () => pending?.reject(new Error('cancelled'));
    bridge.onmessage = event => {
      let value;
      try { value = JSON.parse(event.data); } catch (_) { return; }
      if (!value || typeof value !== 'object' || !pending || value.requestId !== pending.id) return;
      if (value.code || !current(descriptor)) pending.reject(new Error('native_source_expired'));
      else pending.resolve(value);
    };
    signal?.addEventListener('abort', abort);
    try {
      for (let offset = 0; offset < descriptor.size;) {
        if (signal?.aborted || !current(descriptor)) throw new Error('cancelled');
        const id = 'attachment_' + (++sequence).toString(36);
        let timer;
        try {
          const response = await new Promise((resolve, reject) => {
            pending = { id, resolve, reject };
            timer = root.setTimeout(() => reject(new Error('native_source_timeout')), 5000);
            bridge.postMessage(JSON.stringify({ requestId: id, leaseId: descriptor.leaseId,
              documentToken: descriptor.documentToken, offset }));
          });
          const expected = Math.min(64 * 1024, descriptor.size - offset);
          if (response.offset !== offset || typeof response.data !== 'string' ||
              response.data.length > 88 * 1024) throw new Error('native_source_invalid');
          const binary = root.atob(response.data);
          if (binary.length !== expected) throw new Error('native_source_invalid');
          const bytes = new Uint8Array(expected);
          for (let index = 0; index < expected; index++) bytes[index] = binary.charCodeAt(index);
          parts.push(bytes);
          offset += expected;
        } finally {
          root.clearTimeout(timer);
          pending = null;
        }
      }
      if (signal?.aborted || !current(descriptor)) throw new Error('cancelled');
      return new root.File(parts, descriptor.name, { type: descriptor.type });
    } finally {
      pending = null;
      parts.length = 0;
      signal?.removeEventListener('abort', abort);
      bridge.onmessage = previous;
      if (active === job) active = null;
    }
  }

  return Object.freeze({ version: 2, read });
});
