(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateAttachmentSelection = exported;
})(typeof window === 'object' ? window : null, function (root, options) {
  'use strict';
  const composer = options.composer;
  let pending = null;

  function release(value) {
    if (!value) return;
    root.clearTimeout(value.setupTimer);
    root.clearTimeout(value.expiryTimer);
    value.controller.abort();
    value.transport?.dispose();
    if (pending === value) pending = null;
  }

  function cancel(id) {
    if (id == null || pending?.id === id) release(pending);
  }

  function current(value) {
    return !value.controller.signal.aborted && composer.current(value.binding);
  }

  async function prepare(value, kind) {
    try {
      if (!await composer.prepare(value.binding, value.controller.signal) ||
          pending !== value || !current(value)) return release(value);
      const context = composer.pickerReservationContext(value.binding, kind);
      if (!context) return release(value);
      value.transport = options.createTransport({
        isCurrent: binding => binding === value.binding && current(value),
      });
      value.transport.prefetch(context, value.binding, value.controller.signal);
      value.ready = true;
      root.clearTimeout(value.setupTimer);
    } catch (_) { release(value); }
  }

  function begin(raw) {
    cancel();
    let intent;
    try { intent = JSON.parse(raw); } catch (_) { return false; }
    if (options.busy() || !/^selection_[a-f0-9]{32}$/.test(intent?.id || '') ||
        !['image', 'document'].includes(intent.kind) || !composer?.available()) return false;
    try {
      // Only already available page identity is used. Opening a picker never waits for login or hydration.
      const binding = composer.capture();
      if (intent.documentToken !== binding.token || intent.href !== binding.href ||
          binding.isTemporaryChat || binding.projectId) return false;
      const value = { id: intent.id, binding, controller: new root.AbortController(), transport: null, ready: false };
      pending = value;
      value.setupTimer = root.setTimeout(() => release(value), 5000);
      value.expiryTimer = root.setTimeout(() => release(value), 120000);
      void prepare(value, intent.kind);
      return true;
    } catch (_) { return false; }
  }

  function take(descriptor) {
    const value = pending;
    if (!value) return null;
    if (!value.ready || descriptor.selectionId !== value.id || descriptor.documentToken !== value.binding.token ||
        descriptor.href !== value.binding.href || !current(value)) {
      release(value);
      return null;
    }
    // Transfer the same binding and transport exactly once; taking never waits for allocation.
    pending = null;
    root.clearTimeout(value.setupTimer);
    root.clearTimeout(value.expiryTimer);
    return value;
  }

  return Object.freeze({ version: 1, begin, take, cancel });
});
