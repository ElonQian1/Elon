(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 3, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptRspackMessages = factory(root);
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  let context, projection, warming = false, retryAt = 0, lastDiagnostic = '';
  const states = new Set(['idle', 'error', 'streaming', 'active-async-turn', 'active-tpp-turn']);
  function diagnostic(code) {
    if (page.__elonChatGptGroupReadDiagnosticsEnabled !== true ||
        page.__elonChatGptRspackSubmit?.state().code !== 'accepted' || code === lastDiagnostic) return;
    lastDiagnostic = code;
    try {
      page.elonChatGptNative?.postMessage(JSON.stringify({ type: 'browser_diagnostic',
        kind: 'rspack_read_' + code, detail: 'rspack_read_' + code }));
    } catch (_) { /* A diagnostic bridge failure cannot change the read result. */ }
  }
  const fail = code => { diagnostic(code); return null; };

  function warm(notify) {
    if (warming || Date.now() < retryAt) return;
    warming = true;
    Promise.resolve().then(() => page.__elonChatGptRspackRuntime.load()).then(runtime => {
      retryAt = runtime ? 0 : Date.now() + 10000;
      if (runtime && typeof notify === 'function') notify();
    }).catch(() => { retryAt = Date.now() + 10000; }).finally(() => { warming = false; });
  }

  function read(composer, notify) {
    try {
      const loader = page.__elonChatGptRspackRuntime;
      if (!loader?.observed()) return null;
      const runtime = loader.peek();
      if (!runtime) { warm(notify); return fail('runtime_pending'); }
      if (runtime.conversation.G?.scope !== runtime.scope.a ||
          !page.__elonChatGptRspackContext?.create || !page.__elonChatGptPrivateHistoryProjection?.create) return fail('projection_contract');
      context ||= page.__elonChatGptRspackContext.create(page);
      projection ||= page.__elonChatGptPrivateHistoryProjection.create({ streamPolicy: page.__elonChatGptPrivateStreamPolicy });
      const binding = context.read(composer);
      if (!binding) return fail('context_' + context.state().code);
      const mapping = binding.scope.get(runtime.conversation.G, binding.id);
      const currentNode = binding.scope.get(runtime.conversation.z, binding.id);
      const state = binding.scope.get(runtime.conversation.T, binding.id);
      if (!mapping || typeof mapping !== 'object' || Array.isArray(mapping)) return fail('mapping_missing');
      if (!states.has(state)) return fail('state_unknown');
      if (typeof currentNode !== 'string' || !Object.hasOwn(mapping, currentNode)) return fail('branch_missing');
      // Reuse the branch-aware rich-content projector; internal analysis/tool
      // messages and raw asset pointers must not enter the native message list.
      const payload = { mapping, current_node: currentNode };
      const rows = projection.sourceMessages(payload), messages = projection.project(payload);
      if (!rows.length) return fail('branch_empty');
      if (!messages.length) return fail('projection_empty');
      if (!context.owns(binding)) return fail('owner_changed');
      const submitted = page.__elonChatGptRspackSubmit?.state();
      diagnostic(submitted?.requestCurrent === false ? 'request_expired' : state === 'error' ? 'generation_error' : 'ready');
      return { messages, observedCount: rows.length, startIndex: Math.max(0, rows.length - 80),
        streaming: !['idle', 'error'].includes(state) };
    } catch (_) { return fail('exception'); }
  }
  return Object.freeze({ version: 3, read });
});
