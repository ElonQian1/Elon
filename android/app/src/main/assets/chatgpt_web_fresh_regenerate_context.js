(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptFreshRegenerateContext = api;
})(typeof window === 'object' ? window : null, function (page, baseContext) {
  'use strict';
  const UUID = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;
  const SLUG = /^[a-z0-9][a-z0-9._-]{0,127}$/i;
  const fail = code => { throw Error(code); };
  const contract = page.__elonChatGptPrivateRegenerateContract?.create(page);

  async function capture(command) {
    if (!contract) fail('runtime_unavailable');
    const seed = contract.capture(command.turn, command.getModelTrigger);
    if (!seed) fail('context_unavailable');
    const base = await baseContext.capture(command.composer);
    if (base.newConversation || base.temporary || base.projectId != null || base.attachments || base.tool) fail('scope_unsupported');
    const { shared, runtime } = base;
    const owner = contract.prepare(seed, { shared, conversation: runtime });
    if (!owner || owner.cid !== base.conversationId || owner.token !== base.token ||
        owner.message.id !== base.parentId) fail('context_changed');
    if (typeof runtime.textResolveRequestedModel !== 'function' || typeof shared.canvasQueryClient !== 'function' ||
        ['getNodeIfExists', 'getParentNode'].some(key => typeof shared.HM[key] !== 'function')) fail('runtime_unavailable');
    const model = await runtime.textResolveRequestedModel({ conversation: owner.conversation,
      queryClient: shared.canvasQueryClient(), requestedModelId: owner.modelSlug });
    if (model !== owner.modelSlug || !SLUG.test(model || '') || shared.textModelOverride()?.model_slug === model) fail('scope_unsupported');
    const tree = () => shared.XM(owner.conversation.id);
    const user = () => shared.HM.getNodeIfExists(tree(), owner.parentId);
    const original = user();
    if (original?.id !== owner.parentId || original.message?.id !== owner.parentId ||
        original.message.author?.role !== 'user' || original.message.content?.content_type !== 'text' ||
        !Array.isArray(original.message.content.parts) || !original.message.content.parts.length ||
        original.message.content.parts.length > 128 ||
        !original.message.content.parts.every(part => typeof part === 'string' && part.length <= 40000) ||
        original.message.content.parts.reduce((size, part) => size + part.length, 0) > 40000 ||
        !(UUID.test(original.parent || '') || original.parent === 'client-created-root' || original.parent === '') ||
        original.message.channel != null || original.message.recipient != null && original.message.recipient !== 'all') fail('scope_unsupported');
    const metadata = original.message.metadata || {};
    if (['attachments', 'system_hints', 'contextual_retry_message', 'is_contextual_retry_user_message',
      'is_visually_hidden_from_conversation', 'is_visually_hidden_reasoning_group', 'debug_internal_only']
      .some(key => metadata[key] != null && metadata[key] !== false &&
        (!Array.isArray(metadata[key]) || metadata[key].length)) ||
        owner.message.metadata?.map_search_parameters != null || owner.message.metadata?.image_gen_async) fail('scope_unsupported');
    const userSignature = message => JSON.stringify([message?.id, message?.author?.role,
      message?.channel ?? null, message?.recipient ?? 'all', message?.content?.content_type, message?.content?.parts,
      ...['attachments', 'system_hints', 'contextual_retry_message', 'is_contextual_retry_user_message',
        'is_visually_hidden_from_conversation', 'is_visually_hidden_reasoning_group', 'debug_internal_only']
        .map(key => { const value = message?.metadata?.[key];
          return value == null || value === false || Array.isArray(value) && !value.length ? null : value; })]);
    const originalUser = userSignature(original.message), historyParentId = original.parent;
    const variants = new Set(owner.variants), observed = new Set();
    function effort() {
      const selected = owner.model.menu.modelsData?.models?.get(model);
      if (!selected) fail('context_unavailable');
      const value = owner.message.metadata?.thinking_effort ?? selected.defaultThinkingEffort ?? null;
      if (value != null && !SLUG.test(value)) fail('context_invalid');
      return value;
    }
    const thinkingEffort = effort();
    function owns() { return base.owns() && contract.ownerCurrent(owner); }
    function sameUser() {
      const value = user();
      return value?.parent === historyParentId && value.id === owner.parentId && userSignature(value.message) === originalUser;
    }
    function current() {
      try {
        const ids = shared.HM.getVariantIds(tree(), owner.message.id);
        return owns() && base.current() && sameUser() && effort() === thinkingEffort &&
          owner.menu.retryOption?.value === model && owner.menu.retryOption.disabled !== true &&
          owner.menu.retryOption.shouldShowUpsell !== true && owner.menu.canRegenerateResponse === true &&
          runtime.f8t({ modelSlug: model, modelSwitcherDenialsBySlug: owner.model.menu.modelSwitcherDenialsBySlug }) === true &&
          Array.isArray(ids) && ids.length === variants.size && ids.every(id => variants.has(id));
      } catch (_) { return false; }
    }
    function isOwnedResponse(message) {
      return message?.author?.role === 'assistant' && UUID.test(message.id || '') &&
        observed.has(message.id) && !variants.has(message.id);
    }
    if (!current()) fail('context_changed');
    return Object.freeze({ ...base, operation: 'regenerate', model, effort: thinkingEffort, serviceTier: null,
      parentId: owner.parentId, parentRole: 'user', historyParentId,
      variantPurpose: variants.size === 1 ? 'comparison_implicit' : 'none', current, owns, isOwnedResponse,
      matchesOriginalUser: message => userSignature(message) === originalUser,
      beforeDispatch() { if (!current()) fail('context_changed'); base.beforeDispatch(); },
      observePayload(payload) {
        if (!owns()) return false;
        const input = payload?.input_message;
        if (input && input.id !== owner.parentId) return false;
        const message = payload?.message;
        if (!message) return true;
        if (message.author?.role === 'user') return message.id === owner.parentId;
        if (message.author?.role !== 'assistant') return true;
        if (!UUID.test(message.id || '') || variants.has(message.id) || observed.size >= 64 && !observed.has(message.id)) return false;
        observed.add(message.id);
        return true;
      },
      canReconcile(id) { return id === owner.parentId && owns() && sameUser() && base.canReconcile(id); },
      canStop(id) { return this.canReconcile(id); },
      reconciled(id, stopped = false) {
        if (!this.canReconcile(id)) return false;
        const status = shared.Fx(owner.conversation), leaf = shared.HM.getCurrentMessage(tree());
        return (status == null || status.value === shared.v7.UNREAD) && isOwnedResponse(leaf) &&
          (leaf.status === 'finished_successfully' && leaf.end_turn === true ||
            stopped && leaf.status === 'finished_partial_completion') &&
          shared.HM.getParentPromptNode(tree(), leaf.id)?.id === owner.parentId;
      }
    });
  }
  return Object.freeze({ capture });
});
