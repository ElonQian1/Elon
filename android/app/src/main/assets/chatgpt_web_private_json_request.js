(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 1, request: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (!root || !root.location || root.location.origin !== 'https://chatgpt.com') return;
  if (Number(root.__elonChatGptPrivateJsonRequest?.version) >= exported.version) return;
  root.__elonChatGptPrivateJsonRequest = exported;
})(typeof window === 'object' ? window : null, function (root, url, init, limits) {
  'use strict';
  const timeoutMs = Math.max(1, Math.min(30000, Number(limits?.timeoutMs) || 5000));
  const maxBytes = Math.max(1, Math.min(8 * 1024 * 1024, Number(limits?.maxBytes) || 1024 * 1024));
  const mode = limits?.mode || 'json';
  const controller = typeof root.AbortController === 'function' ? new root.AbortController() : null;
  let reader = null;
  let response = null;
  let settled = false;
  let timer = null;
  let rejectRequest;
  const inputSignal = init?.signal;
  const expired = () => { if (settled) throw new Error('cancelled'); };

  function releaseBody() {
    try {
      const body = reader || response?.body;
      if (body && typeof body.cancel === 'function') Promise.resolve(body.cancel()).catch(() => {});
    } catch (_) {}
  }

  function cleanup() {
    if (timer !== null) root.clearTimeout(timer);
    if (inputSignal) inputSignal.removeEventListener('abort', cancel);
    if (controller) controller.abort();
    releaseBody();
  }

  function fail(code) {
    if (settled) return;
    settled = true;
    rejectRequest(new Error(code));
    cleanup();
  }

  function cancel() { fail('cancelled'); }

  async function readText() {
    const declared = Number(response.headers?.get('content-length') || 0);
    if (declared > maxBytes) throw new Error('response_too_large');
    // Real Fetch responses stream bytes; stop before buffering an oversized body.
    if (response.body && typeof response.body.getReader === 'function') {
      reader = response.body.getReader();
      const decoder = new TextDecoder('utf-8', { fatal: true });
      const parts = [];
      let size = 0;
      let chunks = 0;
      try {
        while (true) {
          const item = await reader.read();
          expired();
          if (item.done) break;
          size += item.value.byteLength;
          if (size > maxBytes || ++chunks > 65536) throw new Error('response_too_large');
          parts.push(decoder.decode(item.value, { stream: true }));
        }
        parts.push(decoder.decode());
        return parts.join('');
      } finally {
        releaseBody();
        try { reader.releaseLock(); } catch (_) {}
        reader = null;
      }
    }
    // Compatibility with non-streaming Fetch implementations, with the same deadline.
    if (typeof response.text !== 'function') throw new Error('response_body_unavailable');
    const text = await response.text();
    expired();
    if (text.length > maxBytes || new TextEncoder().encode(text).byteLength > maxBytes) {
      throw new Error('response_too_large');
    }
    return text;
  }

  async function execute() {
    expired();
    response = await root.fetch(url, Object.assign({}, init, {
      signal: controller ? controller.signal : inputSignal,
    }));
    if (settled) { releaseBody(); throw new Error('cancelled'); }
    const status = Number(response?.status) || 0;
    if (!response || !response.ok) throw new Error('http_' + status);
    if (mode === 'none') return { status, ok: true };
    const text = await readText();
    expired();
    if (mode === 'text') return { status, ok: true, text };
    let payload;
    try { payload = JSON.parse(text); }
    catch (_) { throw new Error('invalid_json'); }
    return { status, ok: true, payload };
  }

  return new Promise((resolve, reject) => {
    rejectRequest = reject;
    if (inputSignal?.aborted) { cancel(); return; }
    if (inputSignal) inputSignal.addEventListener('abort', cancel, { once: true });
    // Settle independently of AbortController: some wrappers ignore cancellation.
    timer = root.setTimeout(() => fail('timeout'), timeoutMs);
    execute().then((value) => {
      if (settled) return;
      settled = true;
      cleanup();
      resolve(value);
    }, (error) => {
      if (settled) return;
      settled = true;
      cleanup();
      reject(error);
    });
  });
});
