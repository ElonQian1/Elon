(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptRspackMessages = factory(root);
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  let context, projection, warming = false, retryAt = 0;
  const states = new Set(['idle', 'error', 'streaming', 'active-async-turn', 'active-tpp-turn']);

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
      if (!runtime) { warm(notify); return null; }
      if (runtime.conversation.G?.scope !== runtime.scope.a ||
          !page.__elonChatGptRspackContext?.create || !page.__elonChatGptPrivateHistoryProjection?.create) return null;
      context ||= page.__elonChatGptRspackContext.create(page);
      projection ||= page.__elonChatGptPrivateHistoryProjection.create({ streamPolicy: page.__elonChatGptPrivateStreamPolicy });
      const binding = context.read(composer);
      if (!binding) return null;
      const mapping = binding.scope.get(runtime.conversation.G, binding.id);
      const currentNode = binding.scope.get(runtime.conversation.z, binding.id);
      const state = binding.scope.get(runtime.conversation.T, binding.id);
      if (!mapping || typeof mapping !== 'object' || Array.isArray(mapping) || !states.has(state) ||
          typeof currentNode !== 'string' || !Object.hasOwn(mapping, currentNode)) return null;
      // Reuse the branch-aware rich-content projector; internal analysis/tool
      // messages and raw asset pointers must not enter the native message list.
      const payload = { mapping, current_node: currentNode };
      const rows = projection.sourceMessages(payload), messages = projection.project(payload);
      if (!rows.length || !messages.length || !context.owns(binding)) return null;
      return { messages, observedCount: rows.length, startIndex: Math.max(0, rows.length - 80),
        streaming: !['idle', 'error'].includes(state) };
    } catch (_) { return null; }
  }
  return Object.freeze({ version: 1, read });
});
