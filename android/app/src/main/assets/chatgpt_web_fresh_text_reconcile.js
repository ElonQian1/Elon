(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 14, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptFreshTextReconcile = api;
})(typeof window === 'object' ? window : null, function () {
  'use strict';
  const ownsKey = (object, key) => object && Object.prototype.hasOwnProperty.call(object, key);

  function ordinaryProjectHistory(payload, binding) {
    const owner = payload.owner, scopes = payload.context_scopes;
    return (owner == null || typeof owner === 'object' && !Array.isArray(owner) &&
      typeof owner.user_id === 'string' && /^[A-Za-z0-9_-]{1,160}$/.test(owner.user_id) &&
      owner.user_id === binding.projectUserId) &&
      (scopes == null || Array.isArray(scopes) && Array.from(scopes).every(value => value === 'GLOBAL'));
  }

  function conversationMatches(payload, binding) {
    const projectId = binding.projectId ?? null;
    return payload?.conversation_id === binding.conversationId && (payload.gizmo_id ?? null) === projectId &&
      (binding.temporary === true ? projectId === null && payload.is_do_not_remember === true &&
        payload.is_temporary_chat !== false && payload.shared_project_conversation_owner == null :
        binding.newConversation !== true || payload.is_do_not_remember === false &&
          payload.is_temporary_chat !== true && payload.shared_project_conversation_owner == null) &&
      (projectId === null || /^g-p-[a-f0-9]{32}$/i.test(projectId) &&
        payload.is_do_not_remember === false && payload.shared_project_conversation_owner == null &&
        ordinaryProjectHistory(payload, binding));
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
      (root.message == null || root.message.author?.role === 'root') && root.children?.includes(user.id) === true ||
      newParentVerifier(payload, binding, user) !== null;
  }

  function completePagination(payload, rootId) {
    const page = payload.__paginatedConversationPage, messages = page?.messagesLeafToRoot;
    if (!page || page.cursor !== null || page.serverCurrentLeafId !== payload.current_node ||
        !Array.isArray(messages) || !messages.length || messages.length > 4096 ||
        messages[0]?.id !== payload.current_node || page.oldestMessageId !== messages.at(-1)?.id ||
        payload.mapping[rootId]?.children?.[0] !== page.oldestMessageId) return false;
    const seen = new Set([rootId]);
    return messages.every((message, index) => {
      if (!message || typeof message.id !== 'string' || seen.has(message.id) ||
          !ownsKey(payload.mapping, message.id)) return false;
      seen.add(message.id);
      const node = payload.mapping[message.id], child = messages[index - 1]?.id;
      return node?.id === message.id && node.message === message &&
        node.parent === (messages[index + 1]?.id ?? rootId) && Array.isArray(node.children) &&
        node.children.length === (index === 0 ? 0 : 1) && (index === 0 || node.children[0] === child);
    });
  }

  function newParentVerifier(payload, binding, user) {
    if (binding.newConversation !== true || binding.operation === 'regenerate' ||
        binding.parentId !== 'client-created-root' || !user) return null;
    const mapping = payload.mapping, chain = [], seen = new Set([user.id]);
    const paginatedRoot = 'paginated-root:' + binding.conversationId;
    let cursor = user.parent, child = user.id;
    while (chain.length < 8) {
      if (typeof cursor !== 'string' || !/^[A-Za-z0-9_-]{1,160}$/.test(cursor) && cursor !== paginatedRoot ||
          seen.has(cursor) || !ownsKey(mapping, cursor)) return null;
      seen.add(cursor);
      const node = mapping[cursor];
      if (!node || node.id !== cursor || !Array.isArray(node.children) ||
          node.children.length !== 1 || node.children[0] !== child) return null;
      const root = node.message == null;
      if (root ? node.parent !== '' || ownsKey(mapping, '') ||
          (cursor === paginatedRoot ? !completePagination(payload, paginatedRoot) : chain.length === 0) :
          !/^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i.test(cursor) ||
          node.message.id !== cursor || node.message.author?.role !== 'system' ||
          node.message.metadata?.is_visually_hidden_from_conversation !== true ||
          node.message.content?.content_type !== 'text') return null;
      chain.push(Object.freeze({ id: cursor, parentId: node.parent, child, root }));
      if (root) {
        // Capture only structural identities before the official loader converts
        // null-message roots into root-role messages with parentId, not parent.
        Object.freeze(chain);
        return (reader, state) => {
          try {
            // VHt hydrates paginated messages into the existing Cx root. Only
            // this verified complete page may retain our original empty root;
            // full-history roots and all non-root identities must still match.
            const canonicalRoot = cursor === paginatedRoot && !reader.getNodeIfExists(state, cursor)
              ? binding.parentId : cursor;
            const canonicalId = id => id === cursor ? canonicalRoot : id;
            return chain.every(expected => {
              const id = canonicalId(expected.id), current = reader.getNodeIfExists(state, id), message = current?.message;
              return current?.id === id && current.parentId === canonicalId(expected.parentId) &&
                message?.id === id && reader.getParentNode(state, expected.child)?.id === id &&
                Array.isArray(current.children) && current.children.length === 1 && current.children[0] === expected.child &&
                (expected.root ? message.author?.role === 'root' : message.author?.role === 'system' &&
                  message.metadata?.is_visually_hidden_from_conversation === true && message.content?.content_type === 'text');
            });
          } catch (_) { return false; }
        };
      }
      child = cursor; cursor = node.parent;
    }
    return null;
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

  function awaitingUser(payload, binding, userMessageId) {
    // The request can precede its history row. Only an unchanged completed
    // parent is a propagation gap; another branch is never a reason to wait.
    if (binding.operation === 'regenerate' || binding.newConversation === true ||
        !conversationMatches(payload, binding) || !payload.mapping || Array.isArray(payload.mapping) ||
        ownsKey(payload.mapping, userMessageId) || payload.current_node !== binding.parentId ||
        !ownsKey(payload, 'async_status') || ![null, 3, 4].includes(payload.async_status)) return false;
    const parent = ownsKey(payload.mapping, binding.parentId) && payload.mapping[binding.parentId];
    return parent?.id === binding.parentId && parent.message?.id === binding.parentId &&
      parent.message.author?.role === 'assistant' &&
      (parent.message.status === 'finished_successfully' && parent.message.end_turn === true ||
        parent.message.status === 'finished_partial_completion');
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
    const firstRoute = binding.newConversation === true && binding.temporary !== true;
    // The official first-response handler binds and navigates its server ID
    // before reducing history into the tree. Hydrating while still on the empty
    // home route can replace that route's composer and retire our captured owner.
    if (firstRoute && (typeof binding.finalize !== 'function' ||
        await binding.finalize(signal) !== true || signal.aborted ||
        !binding.canReconcile(request.userMessageId))) { report('owner_changed'); return false; }
    report('reading');
    let verified = false, verifyNewParent = null, verifiedReplyId = null;
    // BEn performs official fetch + tree reconciliation. Do not replace the
    // website store, reload the document, or apply another branch's response.
    await binding.runtime.textHydrateHistory(binding.conversationId, {
      forceNetworkFetch: true, includeMessageId: request.userMessageId,
      signal, skipIfExisting: false, source: 'native_fresh_text_v1',
      onConversationLoadedFromNetwork(payload) {
        verified = !signal.aborted && ownsResponse(payload, binding, request.userMessageId, stopped, emptyStopped);
        verifiedReplyId = verified && binding.operation === 'regenerate' ? payload.current_node : null;
        verifyNewParent = verified ? newParentVerifier(payload, binding, payload.mapping?.[request.userMessageId]) : null;
        report(signal.aborted ? 'owner_changed' : rejection(payload, binding, request.userMessageId, stopped, emptyStopped));
      },
      shouldApplyResponse: () => verified && !signal.aborted && binding.canReconcile(request.userMessageId)
    });
    const selected = !signal.aborted && verified && (binding.operation !== 'regenerate' ||
      binding.selectVerifiedReply?.(request.userMessageId, verifiedReplyId, stopped) === true);
    let done = selected && binding.reconciled(request.userMessageId, stopped, emptyStopped, verifyNewParent);
    if (done && binding.finalize && !firstRoute) done = await binding.finalize(signal) === true &&
      !signal.aborted && binding.reconciled(request.userMessageId, stopped, emptyStopped, verifyNewParent);
    if (verified) report(done ? 'reconciled' : signal.aborted || !binding.canReconcile(request.userMessageId)
      ? 'owner_changed' : binding.reconciliationFailure?.() || 'store_not_reconciled');
    return done;
  }
  return Object.freeze({ reconcile, ownsResponse, branch, read, awaitingUser });
});
