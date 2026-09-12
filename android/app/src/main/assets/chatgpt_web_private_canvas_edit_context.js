(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 2, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateCanvasEditContext = api;
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  const fail = code => { throw Error('canvas_' + code); };
  let retained;

  async function capture(binding, id) {
    const bindings = page.__elonChatGptPrivateRuntimeBindings;
    if (!bindings?.observed('conversation') || !bindings.observed('shared')) fail('runtime_unavailable');
    const [conversation, shared, react] = await Promise.all([
      bindings.load('conversation'), bindings.load('shared'), bindings.load('react')]);
    if (!options.current(binding)) fail('context_changed');
    if (typeof page.__elonChatGptPrivateCanvasEditObserver?.create !== 'function' ||
        typeof shared?.canvasQueryClient !== 'function') {
      fail('runtime_unavailable');
    }
    // A session-scoped singleton getter, not React's useQueryClient hook.
    const client = shared.canvasQueryClient();
    if (!client || typeof client.invalidateQueries !== 'function') fail('runtime_unavailable');
    const profile = bindings.state?.().profile_id;
    if (!profile) fail('runtime_unavailable');
    const same = retained && retained.client === client && retained.profile === profile &&
      retained.binding.document === binding.document && retained.binding.token === binding.token &&
      retained.binding.account === binding.account;
    if (!same) {
      retained?.observer.dispose();
      retained = { binding, client, profile,
        observer: page.__elonChatGptPrivateCanvasEditObserver.create(page, react, conversation, client) };
    }
    const observer = retained.observer;
    const context = { binding, id, observer, client, bindings, profile };
    check(context);
    return context;
  }

  function check(context) {
    if (!options.current(context.binding)) fail('context_changed');
    if (context.bindings.state?.().profile_id !== context.profile) fail('runtime_unavailable');
    const snapshot = context.binding.readSnapshot?.();
    if (snapshot?.url !== context.binding.href || snapshot.streaming !== false || snapshot.dictationActive ||
        snapshot.dictationCaptureActive || snapshot.dictationCapturePending ||
        page.__elonChatGptPrivateConversationDelete?.busy?.() ||
        page.__elonChatGptPrivateConversationMutation?.state?.().state === 'busy') fail('conversation_busy');
    context.observer.check(context.id);
  }

  async function reconcile(context, kind = 'document') {
    try {
      check(context);
      // Invalidate only the affected query; do not reload the page, erase local edits,
      // instantiate a second cache, or optimistically acknowledge someone else's save queue.
      const queryKey = kind === 'share' ? ['canvas', 'textdoc', 'share', context.id] : [context.binding.id, 'textdocs'];
      await context.client.invalidateQueries({ queryKey, exact: true, refetchType: 'active' });
      return true;
    } catch (_) { return false; }
  }

  return Object.freeze({ version: 2, capture, check, reconcile });
});
