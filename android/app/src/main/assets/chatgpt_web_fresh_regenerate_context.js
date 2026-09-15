(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 8, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptFreshRegenerateContext = api;
})(typeof window === 'object' ? window : null, function (page, baseContext) {
  'use strict';
  const UUID = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;
  const SLUG = /^[a-z0-9][a-z0-9._-]{0,127}$/i;
  const fail = (code, admissionStage) => { throw Object.assign(Error(code), { admissionStage }); };
  const contract = page.__elonChatGptPrivateRegenerateContract?.create(page);
  const userSignature = (page.__elonChatGptFreshTextUserIdentity ||
    (typeof module === 'object' && module.exports ? require('./chatgpt_web_fresh_text_user_identity') : null))?.signature;

  async function capture(command, options = {}) {
    if (!contract || !userSignature) fail('runtime_unavailable', 'contract');
    const seed = contract.capture(command.turn, command.getModelTrigger);
    if (!seed) fail('context_unavailable', 'menu');
    const allowExistingProjects = options.allowExistingProjects === true ||
      page.__elonChatGptFreshRegenerationProjectsEnabled === true;
    const base = await baseContext.capture(command.composer, null, { allowExistingProjects });
    if (base.newConversation || base.temporary || base.projectId != null && !allowExistingProjects ||
        base.attachments || base.tool) fail('scope_unsupported', 'base_scope');
    const { shared, runtime } = base;
    const owner = contract.prepare(seed, { shared, conversation: runtime });
    if (!owner || owner.cid !== base.conversationId || owner.token !== base.token ||
        owner.message.id !== base.parentId) fail('context_changed', 'owner');
    if (typeof runtime.textResolveRequestedModel !== 'function' || typeof shared.canvasQueryClient !== 'function' ||
        typeof shared.writingUpdateState !== 'function' || typeof shared.writingTreeOwner?.setCurrentLeafId !== 'function' ||
        ['getNodeIfExists', 'getParentNode'].some(key => typeof shared.HM[key] !== 'function')) fail('runtime_unavailable', 'resolver');
    const resolvedModel = await runtime.textResolveRequestedModel({ conversation: owner.conversation,
      queryClient: shared.canvasQueryClient(), requestedModelId: owner.modelSlug });
    // A1t uses requestedModelId when the optional work-model resolution is null.
    const model = resolvedModel ?? owner.modelSlug;
    if (model !== owner.modelSlug || !SLUG.test(model || '') || shared.textModelOverride()?.model_slug === model) fail('scope_unsupported', 'model');
    const tree = () => shared.XM(owner.conversation.id);
    const user = () => shared.HM.getNodeIfExists(tree(), owner.parentId);
    const original = user();
    if (original?.id !== owner.parentId || original.message?.id !== owner.parentId ||
        original.message.author?.role !== 'user') fail('scope_unsupported', 'user_identity');
    if (original.message.content?.content_type !== 'text' ||
        !Array.isArray(original.message.content.parts) || !original.message.content.parts.length ||
        original.message.content.parts.length > 128 ||
        !original.message.content.parts.every(part => typeof part === 'string' && part.length <= 40000) ||
        original.message.content.parts.reduce((size, part) => size + part.length, 0) > 40000) fail('scope_unsupported', 'user_content');
    if (typeof original.parentId !== 'string' ||
        !(UUID.test(original.parentId) || original.parentId === 'client-created-root' || original.parentId === '')) fail('scope_unsupported', 'user_parent');
    if (original.message.channel != null) fail('scope_unsupported', 'user_channel');
    if (original.message.recipient != null && original.message.recipient !== 'all') fail('scope_unsupported', 'user_recipient');
    const metadata = original.message.metadata || {};
    if (['attachments', 'system_hints', 'contextual_retry_message', 'is_contextual_retry_user_message',
      'is_visually_hidden_from_conversation', 'is_visually_hidden_reasoning_group', 'debug_internal_only']
      .some(key => metadata[key] != null && metadata[key] !== false &&
        (!Array.isArray(metadata[key]) || metadata[key].length))) fail('scope_unsupported', 'user_metadata');
    if (owner.message.metadata?.map_search_parameters != null || owner.message.metadata?.image_gen_async) fail('scope_unsupported', 'reply_metadata');
    // Official in-memory nodes use parentId; only the history response uses parent.
    const originalUser = userSignature(original.message), historyParentId = original.parentId;
    const variants = new Set(owner.variants), observed = new Set();
    function effort() {
      const selected = owner.model.menu.modelsData?.models?.get(model);
      if (!selected) fail('context_unavailable', 'effort');
      const value = owner.message.metadata?.thinking_effort ??
        (resolvedModel != null ? selected.defaultThinkingEffort : null) ?? null;
      if (value != null && !SLUG.test(value)) fail('context_invalid', 'effort');
      return value;
    }
    const thinkingEffort = effort();
    function owns() { return base.owns() && contract.ownerCurrent(owner); }
    function sameUser(state = tree()) {
      const value = shared.HM.getNodeIfExists(state, owner.parentId);
      return value?.parentId === historyParentId && value.id === owner.parentId && userSignature(value.message) === originalUser;
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
    const terminal = (message, stopped) => message?.status === 'finished_successfully' && message.end_turn === true ||
      stopped && message?.status === 'finished_partial_completion';
    const settled = () => { const status = shared.Fx(owner.conversation);
      return status == null || status.value === shared.v7.UNREAD; };
    let storeStage = 'store_not_reconciled';
    if (!current()) fail('context_changed', 'current');
    return Object.freeze({ ...base, operation: 'regenerate', model, effort: thinkingEffort, serviceTier: null,
      parentId: owner.parentId, parentRole: 'user', historyParentId,
      variantPurpose: variants.size === 1 ? 'comparison_implicit' : 'none', current, owns, isOwnedResponse,
      matchesOriginalUser: message => userSignature(message) === originalUser,
      recoveryUserSignature: () => owns() ? originalUser : null,
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
      canReconcile(id) {
        if (id !== owner.parentId || !owns() || !sameUser()) return false;
        const leaf = shared.HM.getCurrentMessage(tree());
        return (leaf?.id === owner.message.id || isOwnedResponse(leaf)) && base.canReconcile(id);
      },
      canStop(id) { return this.canReconcile(id); },
      reconciliationFailure: () => storeStage,
      selectVerifiedReply(id, replyId, stopped = false) {
        try {
          storeStage = 'store_owner_changed'; if (!this.canReconcile(id)) return false;
          storeStage = 'store_status_unsettled'; if (!settled()) return false;
          const eligible = state => {
            const selected = shared.HM.getCurrentLeafId(state);
            const reply = shared.HM.getNodeIfExists(state, replyId)?.message;
            return (selected === owner.message.id || selected === replyId) && sameUser(state) &&
              reply?.id === replyId && isOwnedResponse(reply) && terminal(reply, stopped) &&
              shared.HM.getParentPromptNode(state, replyId)?.id === owner.parentId;
          };
          storeStage = 'store_leaf_mismatch'; if (!eligible(tree())) return false;
          if (shared.HM.getCurrentLeafId(tree()) !== replyId) {
            // fy may preserve the old sibling. Reuse the reviewed sY/KJ state
            // transaction only after this request's terminal history is verified.
            shared.writingUpdateState(owner.conversation.id, state => {
              if (this.canReconcile(id) && settled() && eligible(state) &&
                  shared.HM.getCurrentLeafId(state) === owner.message.id) {
                shared.writingTreeOwner.setCurrentLeafId(state, replyId);
              }
            });
          }
          return this.reconciled(id, stopped);
        } catch (_) { return false; }
      },
      reconciled(id, stopped = false) {
        storeStage = 'store_owner_changed'; if (!this.canReconcile(id)) return false;
        storeStage = 'store_status_unsettled'; if (!settled()) return false;
        const leaf = shared.HM.getCurrentMessage(tree());
        storeStage = 'store_leaf_mismatch'; if (!isOwnedResponse(leaf) || !terminal(leaf, stopped)) return false;
        storeStage = 'store_prompt_mismatch';
        if (shared.HM.getParentPromptNode(tree(), leaf.id)?.id !== owner.parentId) return false;
        storeStage = 'reconciled'; return true;
      }
    });
  }
  let inspection = null;
  function inspect(command, options = {}) {
    if (inspection) return inspection;
    const allowExistingProjects = options.allowExistingProjects === true ||
      page.__elonChatGptFreshTextTransaction?.trialControl?.('state')?.armed === true;
    const result = (code, stage) => ({ schema: 'elon.fresh_regenerate_admission.v1', code, stage });
    const codes = ['scope_unsupported', 'runtime_unavailable', 'identity_unavailable', 'context_unavailable',
      'context_changed', 'context_invalid', 'attachments_active', 'tools_active', 'conversation_busy', 'parent_unavailable'];
    const stages = ['contract', 'menu', 'base_scope', 'owner', 'resolver', 'model', 'user_identity', 'user_content',
      'user_parent', 'user_channel', 'user_recipient', 'user_metadata', 'reply_metadata', 'effort', 'current',
      'base_route', 'base_new', 'base_owner', 'base_composer', 'base_route_state', 'base_privacy', 'base_prepare',
      'base_model', 'base_workspace', 'base_project', 'base_mode', 'base_branch', 'base_config',
      'project_route', 'project_mode', 'project_loading', 'project_privacy', 'project_shared',
      'project_scopes', 'project_business', 'project_headers'];
    let timer;
    // Admission only: no prepare request, stream, draft mutation or writer ledger.
    inspection = Promise.race([capture(command, { allowExistingProjects }).then(() => result('ready', 'ready'), error =>
      result(codes.includes(error?.message) ? error.message : 'read_failed',
        stages.includes(error?.admissionStage) ? error.admissionStage : 'base_context')),
    new Promise(resolve => { timer = page.setTimeout(() => resolve(result('timeout', 'timeout')), 5000); })])
      .finally(() => { page.clearTimeout(timer); inspection = null; });
    return inspection;
  }
  return Object.freeze({ capture, inspect });
});
