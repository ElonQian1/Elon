(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptFreshTextReconcile = api;
})(typeof window === 'object' ? window : null, function () {
  'use strict';
  const ownsKey = (object, key) => object && Object.prototype.hasOwnProperty.call(object, key);

  function branch(payload, binding, userMessageId) {
    if (!payload || payload.conversation_id !== binding.conversationId ||
        !payload.mapping || Array.isArray(payload.mapping)) return false;
    const mapping = payload.mapping, user = ownsKey(mapping, userMessageId) && mapping[userMessageId];
    if (user?.id !== userMessageId || user.message?.id !== userMessageId ||
        user.message.author?.role !== 'user' || user.parent !== binding.parentId) return false;
    let id = payload.current_node;
    const leaf = ownsKey(mapping, id) && mapping[id]?.message;
    if (!leaf || leaf.id !== id || !(leaf.author?.role === 'assistant' ||
        id === userMessageId && leaf.author?.role === 'user')) return null;
    const seen = new Set();
    while (id && seen.size < 4096) {
      if (id === userMessageId) return { leaf, asyncStatus: payload.async_status };
      if (seen.has(id) || !ownsKey(mapping, id)) return false;
      seen.add(id);
      const node = mapping[id];
      // A later user turn or a sibling branch is not our completion.
      if (!node || node.id !== id || node.message?.id !== id || node.message?.author?.role === 'user') return null;
      id = node.parent;
    }
    return false;
  }

  function ownsResponse(payload, binding, userMessageId, stopped = false, emptyStopped = false) {
    const owned = branch(payload, binding, userMessageId);
    if (!owned) return false;
    // Reviewed provider async enum: 3 streaming, 4 completed but unread.
    if (ownsKey(payload, 'async_status') && owned.asyncStatus !== null && owned.asyncStatus !== 4) return false;
    if (stopped && owned.asyncStatus !== null && owned.asyncStatus !== 4) return false;
    if (owned.leaf.author.role === 'user') return stopped && emptyStopped;
    return owned.leaf.status === 'finished_successfully' && owned.leaf.end_turn === true ||
      stopped && owned.leaf.status === 'finished_partial_completion';
  }

  async function read(binding, request, signal) {
    if (signal.aborted || !binding.canReconcile(request.userMessageId)) return null;
    let value = null;
    await binding.runtime.textHydrateHistory(binding.conversationId, {
      forceNetworkFetch: true, includeMessageId: request.userMessageId, signal,
      skipIfExisting: false, source: 'native_fresh_stop_v1', shouldApplyResponse: () => false,
      onConversationLoadedFromNetwork(payload) { if (!signal.aborted) value = payload; }
    });
    return !signal.aborted && binding.canReconcile(request.userMessageId) ? value : null;
  }

  async function reconcile(binding, request, signal, stopped = false, emptyStopped = false) {
    if (signal.aborted || !binding.canReconcile(request.userMessageId) ||
        typeof binding.runtime.textHydrateHistory !== 'function') return false;
    let verified = false;
    // BEn performs official fetch + tree reconciliation. Do not replace the
    // website store, reload the document, or apply another branch's response.
    await binding.runtime.textHydrateHistory(binding.conversationId, {
      forceNetworkFetch: true, includeMessageId: request.userMessageId,
      signal, skipIfExisting: false, source: 'native_fresh_text_v1',
      onConversationLoadedFromNetwork(payload) {
        verified = !signal.aborted && ownsResponse(payload, binding, request.userMessageId, stopped, emptyStopped);
      },
      shouldApplyResponse: () => verified && !signal.aborted && binding.canReconcile(request.userMessageId)
    });
    return !signal.aborted && verified && binding.reconciled(request.userMessageId, stopped, emptyStopped);
  }
  return Object.freeze({ reconcile, ownsResponse, branch, read });
});
