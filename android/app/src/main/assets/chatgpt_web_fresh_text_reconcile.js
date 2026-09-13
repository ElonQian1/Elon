(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 6, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptFreshTextReconcile = api;
})(typeof window === 'object' ? window : null, function () {
  'use strict';
  const ownsKey = (object, key) => object && Object.prototype.hasOwnProperty.call(object, key);

  function conversationMatches(payload, binding) {
    const projectId = binding.projectId ?? null;
    return payload?.conversation_id === binding.conversationId && (payload.gizmo_id ?? null) === projectId &&
      (binding.temporary === true ? projectId === null && payload.is_do_not_remember === true &&
        payload.is_temporary_chat !== false && payload.shared_project_conversation_owner == null :
        binding.newConversation !== true || payload.is_do_not_remember === false &&
          payload.is_temporary_chat !== true && payload.shared_project_conversation_owner == null) &&
      (projectId === null || /^g-p-[a-f0-9]{32}$/i.test(projectId) &&
        payload.is_do_not_remember === false && payload.shared_project_conversation_owner == null);
  }

  function branch(payload, binding, userMessageId) {
    if (!conversationMatches(payload, binding) ||
        !payload.mapping || Array.isArray(payload.mapping)) return false;
    const mapping = payload.mapping, user = ownsKey(mapping, userMessageId) && mapping[userMessageId];
    if (user?.id !== userMessageId || user.message?.id !== userMessageId ||
        user.message.author?.role !== 'user' || !parentMatches(payload, binding, user) ||
        binding.attachments && !binding.attachments.matchesHistory(user.message)) return false;
    if (binding.operation === 'regenerate' && binding.matchesOriginalUser?.(user.message) !== true) return false;
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

  function parentMatches(payload, binding, user) {
    if (binding.operation === 'regenerate') return user.id === binding.parentId &&
      typeof binding.historyParentId === 'string' && user.parent === binding.historyParentId;
    if (user.parent === binding.parentId) return true;
    // hy's raw callback precedes oQe's legacy empty-root normalization. This
    // exception cannot authorize another UUID parent or a non-root message.
    const root = payload.mapping?.[''];
    return binding.newConversation === true && binding.parentId === 'client-created-root' &&
      user.parent === '' && ownsKey(payload.mapping, '') && root?.id === '' && !root.parent &&
      (root.message == null || root.message.author?.role === 'root') && root.children?.includes(user.id) === true;
  }

  function ownsResponse(payload, binding, userMessageId, stopped = false, emptyStopped = false) {
    const owned = branch(payload, binding, userMessageId);
    if (!owned) return false;
    if (binding.operation === 'regenerate' && binding.isOwnedResponse?.(owned.leaf) !== true) return false;
    // Reviewed provider async enum: 3 streaming, 4 completed but unread.
    if (ownsKey(payload, 'async_status') && owned.asyncStatus !== null && owned.asyncStatus !== 4) return false;
    if (stopped && owned.asyncStatus !== null && owned.asyncStatus !== 4) return false;
    if (owned.leaf.author.role === 'user') return stopped && emptyStopped;
    return owned.leaf.status === 'finished_successfully' && owned.leaf.end_turn === true ||
      stopped && owned.leaf.status === 'finished_partial_completion';
  }

  function rejection(payload, binding, userMessageId, stopped, emptyStopped) {
    if (!payload) return 'payload_missing';
    if (!conversationMatches(payload, binding)) return 'conversation_mismatch';
    const user = payload.mapping?.[userMessageId];
    if (!user || user.id !== userMessageId || user.message?.id !== userMessageId ||
        user.message?.author?.role !== 'user') return 'user_missing';
    if (!parentMatches(payload, binding, user)) return 'parent_mismatch';
    const owned = branch(payload, binding, userMessageId);
    if (!owned) return 'branch_mismatch';
    if (ownsKey(payload, 'async_status') && owned.asyncStatus !== null && owned.asyncStatus !== 4) return 'server_active';
    return ownsResponse(payload, binding, userMessageId, stopped, emptyStopped) ? 'verified' : 'not_terminal';
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

  async function reconcile(binding, request, signal, stopped = false, emptyStopped = false, onObservation) {
    const report = code => { try { onObservation?.(code); } catch (_) {} };
    if (signal.aborted || !binding.canReconcile(request.userMessageId) ||
        typeof binding.runtime.textHydrateHistory !== 'function') { report('owner_changed'); return false; }
    report('reading');
    let verified = false;
    // BEn performs official fetch + tree reconciliation. Do not replace the
    // website store, reload the document, or apply another branch's response.
    await binding.runtime.textHydrateHistory(binding.conversationId, {
      forceNetworkFetch: true, includeMessageId: request.userMessageId,
      signal, skipIfExisting: false, source: 'native_fresh_text_v1',
      onConversationLoadedFromNetwork(payload) {
        verified = !signal.aborted && ownsResponse(payload, binding, request.userMessageId, stopped, emptyStopped);
        report(signal.aborted ? 'owner_changed' : rejection(payload, binding, request.userMessageId, stopped, emptyStopped));
      },
      shouldApplyResponse: () => verified && !signal.aborted && binding.canReconcile(request.userMessageId)
    });
    let done = !signal.aborted && verified && binding.reconciled(request.userMessageId, stopped, emptyStopped);
    if (done && binding.finalize) done = await binding.finalize(signal) === true &&
      !signal.aborted && binding.reconciled(request.userMessageId, stopped, emptyStopped);
    if (verified) report(done ? 'reconciled' : signal.aborted || !binding.canReconcile(request.userMessageId)
      ? 'owner_changed' : 'store_not_reconciled');
    return done;
  }
  return Object.freeze({ reconcile, ownsResponse, branch, read });
});
