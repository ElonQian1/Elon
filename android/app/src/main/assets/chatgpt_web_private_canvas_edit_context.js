(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateCanvasEditContext = api;
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  const fail = code => { throw Error('canvas_' + code); };

  async function capture(binding, id) {
    const bindings = page.__elonChatGptPrivateRuntimeBindings;
    if (!bindings?.observed('conversation') || !bindings.observed('shared')) fail('runtime_unavailable');
    const [conversation, shared] = await Promise.all([bindings.load('conversation'), bindings.load('shared')]);
    if (!options.current(binding)) fail('context_changed');
    if (typeof conversation?.canvasEdits?.getState !== 'function' || typeof shared?.canvasQueryClient !== 'function') {
      fail('runtime_unavailable');
    }
    // Z0 is a session-scoped singleton getter, not React's useQueryClient hook.
    const client = shared.canvasQueryClient();
    if (!client || typeof client.invalidateQueries !== 'function') fail('runtime_unavailable');
    const profile = bindings.state?.().profile_id;
    if (!profile) fail('runtime_unavailable');
    const context = { binding, id, edits: conversation.canvasEdits, client, bindings, profile };
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
    const state = context.edits.getState();
    if (!state?.userEdits || !state.timestamps) fail('runtime_unavailable');
    const edits = state.userEdits[context.id] ?? [], time = state.timestamps[context.id];
    if (!Array.isArray(edits) || edits.some(edit => !edit || typeof edit.isPending !== 'boolean') ||
        time && ['lastTriggeredAt', 'lastFlushedAt'].some(key => time[key] !== null && !Number.isFinite(time[key]))) fail('runtime_unavailable');
    if (edits.some(edit => edit.isPending) || time && (time.lastTriggeredAt ?? 0) > (time.lastFlushedAt ?? 0)) fail('web_edit_pending');
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

  return Object.freeze({ version: 1, capture, check, reconcile });
});
