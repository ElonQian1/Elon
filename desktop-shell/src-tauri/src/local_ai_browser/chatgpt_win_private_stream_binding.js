(function (root, factory) {
  'use strict';

  if (!root) return;
  const api = factory();
  root.__elonWinChatGptPrivateStreamBindingLifecycle = Object.freeze(api);
  api.prepare(root);
})(typeof window === 'object' ? window : null, function () {
  'use strict';

  const VERSION = 1;
  const MIN_TRANSPORT_VERSION = 11;

  function prepare(root) {
    const transport = root && root.__elonChatGptPrivateStreamTransport;
    if (!transport) return false;
    const policy = root.__elonChatGptPrivateStreamPolicy;
    const binding = root.__elonWinChatGptPrivateStreamBinding;
    const reusable = binding && binding.transport === transport &&
      binding.policy === policy && Number(transport.version || 0) >= MIN_TRANSPORT_VERSION;
    if (reusable) return false;
    try {
      if (typeof transport.dispose === 'function') transport.dispose();
    } catch (_) {
      // A stale observer must not prevent the replacement from being installed.
    }
    try { delete root.__elonChatGptPrivateStreamTransport; }
    catch (_) { root.__elonChatGptPrivateStreamTransport = undefined; }
    return true;
  }

  function commit(root) {
    const policy = root && root.__elonChatGptPrivateStreamPolicy;
    const transport = root && root.__elonChatGptPrivateStreamTransport;
    if (!policy || !transport || typeof transport.mergeMessages !== 'function') return false;
    root.__elonWinChatGptPrivateStreamBinding = Object.freeze({
      version: VERSION,
      policy,
      transport,
    });
    return true;
  }

  return Object.freeze({ version: VERSION, prepare, commit });
});
