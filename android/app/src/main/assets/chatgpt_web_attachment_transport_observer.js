(function () {
  'use strict';

  if (location.origin !== 'https://chatgpt.com') return;
  const existing = window.__elonChatGptAttachmentTransportObserver;
  if (existing && Number(existing.version) >= 2) return;
  if (existing && typeof existing.cancel === 'function') existing.cancel();

  const transportVersion = 1;
  const armLifetimeMs = 120000;
  let operation = 0;
  let armedUntil = 0;
  let sequence = 0;
  let started = false;

  function isArmed() {
    return operation > 0 && Date.now() <= armedUntil;
  }

  function emit(state) {
    if (!isArmed()) return;
    const nativeBridge = window.elonChatGptNative;
    const adapterVersion = Number(
      window.__elonChatGptAdapterTargetVersion || window.__elonChatGptAdapterVersion || 0
    );
    const documentToken = String(window.__elonChatGptDocumentToken || '');
    if (!nativeBridge || typeof nativeBridge.postMessage !== 'function') return;
    if (!Number.isInteger(adapterVersion) || adapterVersion <= 0) return;
    if (!/^doc_[a-z0-9_]{3,80}$/.test(documentToken)) return;
    nativeBridge.postMessage(JSON.stringify({
      schema: 'yilong.ai.ui.v1',
      adapterVersion,
      documentToken,
      providerId: 'chatgpt',
      source: 'official_web',
      conversationId: location.pathname,
      sequence: ++sequence,
      emittedAt: new Date().toISOString(),
      event: {
        type: 'attachment_transport',
        transportVersion,
        sequence,
        state,
        completedCount: 0
      }
    }));
  }

  function arm() {
    operation += 1;
    armedUntil = Date.now() + armLifetimeMs;
    sequence = 0;
    started = false;
    emit('armed');
    return operation;
  }

  function cancel() {
    operation += 1;
    armedUntil = 0;
    started = false;
  }

  function requestMetadata(input, init) {
    let url;
    try {
      url = new URL(typeof input === 'string' ? input : input && input.url, location.href);
    } catch (_) {
      return null;
    }
    if (url.origin !== location.origin) return null;
    const method = String(
      (init && init.method) || (input && input.method) || 'GET'
    ).toUpperCase();
    if (method !== 'POST') return null;
    const path = url.pathname;
    if (/^\/backend-api\/sentinel\/[^/]+\/(?:prepare|finalize)\/?$/.test(path)) {
      return { kind: 'sentinel', path };
    }
    if (/^\/backend-api\/files\/[^/]+\/?$/.test(path)) {
      return { kind: 'file', path };
    }
    return null;
  }

  function observe(metadata, status, expectedOperation) {
    if (!metadata || !isArmed() || expectedOperation !== operation) return;
    const ok = Number(status) >= 200 && Number(status) < 300;
    if (!ok) {
      emit('failed');
      return;
    }
    // A successful reservation also matches /files/<segment>; it proves no file upload.
    if (!started) {
      started = true;
      emit('started');
    }
  }

  const delegateFetch = typeof window.fetch === 'function' ? window.fetch : null;
  if (delegateFetch) {
    window.fetch = function () {
      const args = arguments;
      const metadata = isArmed() ? requestMetadata(args[0], args[1] || {}) : null;
      const expectedOperation = operation;
      let result;
      try {
        result = delegateFetch.apply(this, args);
      } catch (error) {
        observe(metadata, 0, expectedOperation);
        throw error;
      }
      return Promise.resolve(result).then(
        (response) => {
          observe(metadata, response && response.status, expectedOperation);
          return response;
        },
        (error) => {
          observe(metadata, 0, expectedOperation);
          throw error;
        }
      );
    };
  }

  const xhrPrototype = window.XMLHttpRequest && window.XMLHttpRequest.prototype;
  const delegateOpen = xhrPrototype && xhrPrototype.open;
  const delegateSend = xhrPrototype && xhrPrototype.send;
  const xhrMetadata = new WeakMap();
  if (delegateOpen && delegateSend) {
    xhrPrototype.open = function (method, rawUrl) {
      const metadata = requestMetadata(rawUrl, { method });
      if (metadata) xhrMetadata.set(this, metadata);
      else xhrMetadata.delete(this);
      return delegateOpen.apply(this, arguments);
    };
    xhrPrototype.send = function () {
      const metadata = isArmed() ? xhrMetadata.get(this) : null;
      const expectedOperation = operation;
      if (metadata) {
        this.addEventListener('loadend', () => {
          observe(metadata, this.status, expectedOperation);
        }, { once: true });
      }
      return delegateSend.apply(this, arguments);
    };
  }

  window.__elonChatGptAttachmentTransportObserver = Object.freeze({
    version: 2,
    arm,
    cancel,
    isArmed
  });
})();
