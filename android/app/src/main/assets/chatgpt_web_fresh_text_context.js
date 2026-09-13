(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 3, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptFreshTextContext = api;
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  const PROFILE = 'web_20260912';
  const fail = code => { throw Error(code); };
  const idPattern = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;

  function stamp() {
    try {
      const bindings = page.__elonChatGptPrivateRuntimeBindings;
      if (bindings?.state?.().profile_id !== PROFILE) return null;
      const shared = bindings.peek('shared');
      const account = page.__elonChatGptPrivateModelContract?.create(page).withRuntimeIdentity({}, shared)?.account;
      return account ? JSON.stringify([page.__elonChatGptDocumentToken, page.location.href, account]) : null;
    } catch (_) { return null; }
  }

  async function capture(composer, stoppedParent, options = {}) {
    const bindings = page.__elonChatGptPrivateRuntimeBindings;
    if (bindings?.state?.().profile_id !== PROFILE) fail('runtime_unavailable');
    const token = page.__elonChatGptDocumentToken, href = page.location.href, document = page.document;
    const url = new URL(href);
    if (url.origin !== 'https://chatgpt.com' || url.search || url.hash || url.username || url.password ||
        !/^\/c\/[a-f0-9-]{36}$/i.test(url.pathname)) fail('scope_unsupported');
    const submit = page.__elonChatGptPrivateTextRuntimeSubmit;
    const binding = submit?.captureConversation?.(composer);
    if (!binding || binding.newThread || binding.temporary || binding.href !== href) fail('context_unavailable');
    const [shared, conversation, editor] = await Promise.all([
      bindings.load('shared'), bindings.load('conversation'), bindings.load('composer')
    ]);
    if (document !== page.document || token !== page.__elonChatGptDocumentToken || href !== page.location.href) fail('context_changed');
    const afterLoad = submit.captureConversation(composer);
    if (!afterLoad || ['token', 'account', 'conversation', 'controller', 'shared', 'files', 'serverId', 'href']
      .some(key => binding[key] !== afterLoad[key])) fail('context_changed');
    const identity = page.__elonChatGptPrivateModelContract?.create(page);
    const account = () => identity?.withRuntimeIdentity({}, shared)?.account;
    const ownerAccount = account();
    if (!ownerAccount || shared.wV?.(shared.SV?.isPersonalWorkspace) !== true) fail('identity_unavailable');
    if (typeof shared.textApi?.safePost !== 'function' || typeof shared.textSecurityHeaders !== 'function' ||
        typeof shared.textHistoryDisabled !== 'function' || typeof shared.textModelOverride !== 'function' ||
        typeof conversation.textSecurity !== 'function' || typeof conversation.textStream !== 'function' ||
        typeof conversation.textPrepareEnabled !== 'function' || typeof conversation.textReviewAck !== 'function' ||
        typeof conversation.textHydrateHistory !== 'function' ||
        typeof editor.Ng !== 'function') fail('runtime_unavailable');
    const selected = binding.conversation, tree = () => shared.XM(selected.id);
    const selectedTool = editor.Ng(binding.controller)?.activeSystemHintType;
    let toolOwner = null;
    if (selectedTool !== null) {
      if (options.allowTools !== true || !['search', 'picture_v2'].includes(selectedTool)) fail('tools_active');
      // Reuse the account/model-filtered tool menu once for admission. The owned
      // transaction subsequently observes the in-memory selection, not DOM layout.
      toolOwner = page.__elonChatGptPrivateComposerToolContext?.capture(page, [
        { hint: 'search', semantic: 'web_search' }, { hint: 'picture_v2', semantic: 'image_generation' }
      ]);
      if (!toolOwner || ['controller', 'conversation', 'shared', 'serverId', 'href', 'token', 'account']
        .some(key => toolOwner[key] !== afterLoad[key]) || toolOwner.document !== document ||
          !toolOwner.allowed?.split(',').includes(selectedTool)) fail('tools_active');
    }
    function registeredOwner() {
      const conversations = shared.canvasConversations?.();
      if (!Array.isArray(conversations) || conversations.length > 512) return false;
      const matches = conversations.filter(item => item?.serverId$?.() === binding.serverId);
      return matches.length === 1 && matches[0] === selected;
    }

    function read() {
      const state = tree(), props = binding.shared.getSharedProps();
      if (!state || !registeredOwner() || props.conversation !== selected || props.composerController !== binding.controller ||
          props.isDisabled !== false || props.isConsumerLockdownModeLoadingForConversation !== false ||
          props.shouldBlockConsumerLockdownModeActionsForConversation !== false ||
          props.structuredInputMessageId != null || selected.serverId$() !== binding.serverId ||
          !idPattern.test(binding.serverId || '') || shared.HM.getGizmoId(state) != null ||
          state.mode?.kind !== 'primary_assistant' || shared.cX?.() !== false || shared.uo(selected) !== false ||
          conversation.textPrepareEnabled() !== true || conversation.textReviewAck(selected) != null) fail('scope_unsupported');
      if (['continuingFromSharedConversationId', 'continuingFromSharedProjectConversationId', 'continuingFromSharedPostId',
        'forkFromSharedPost', 'branchingFromMessageId', 'branchingFromConversationId', 'continuationBranch',
        'hideFromHistory', 'conversationOrigin'].some(key => state[key] != null && state[key] !== false) ||
          Object.keys(selected.config || {}).some(key => selected.config[key] != null && selected.config[key] !== false)) fail('scope_unsupported');
      const files = binding.files.files$(), ready = binding.files.readyFiles$();
      if (!Array.isArray(files) || !Array.isArray(ready) || files.length || ready.length ||
          binding.files.hasUploadInProgress$() !== false) fail('attachments_active');
      const hints = editor.Ng(binding.controller);
      if (hints?.locked !== false || hints.activeSystemHintType !== selectedTool ||
          !(hints.activeConnectorSystemHintTypes instanceof Set) || hints.activeConnectorSystemHintTypes.size ||
          hints.activeCustomAgentSystemHintType !== null || hints.coldStartCampaignCreativeId != null) fail('tools_active');
      const request = shared.HM.getRequestId(state), status = shared.Fx(selected);
      if (shared.Fl(request) !== false || status != null && status.value !== shared.v7.UNREAD ||
          page.__elonChatGptPrivateTextRuntimeSubmit?.state?.().pending ||
          page.__elonChatGptPrivateTextTransactionRelay?.state?.().active ||
          page.__elonChatGptPrivateRegenerateRuntime?.state?.().pending ||
          page.__elonChatGptPrivateStopRuntime?.state?.().pending ||
          page.__elonChatGptCanvasDocumentActions?.generationPending?.()) fail('conversation_busy');
      const parent = shared.HM.getCurrentMessage(state), model = conversation.Nrn(selected);
      if (toolOwner && toolOwner.model !== model?.id) fail('context_changed');
      const completedAssistant = parent?.author?.role === 'assistant' &&
        (parent.status === 'finished_partial_completion' || parent.status === 'finished_successfully' && parent.end_turn === true);
      const confirmedStoppedUser = parent?.author?.role === 'user' && stoppedParent?.id === parent.id &&
        stoppedParent.current() === true;
      if (!parent || parent.id !== props.currentLeafId || !idPattern.test(parent.id) ||
          !(completedAssistant || confirmedStoppedUser) ||
          shared.textModelOverride()?.model_slug === model?.id) fail('parent_unavailable');
      return { conversationId: binding.serverId, parentId: parent.id, parentRole: parent.author.role, model: model?.id,
        tool: selectedTool,
        effort: conversation.yRt(selected).conversationThinkingEffort$() ?? null,
        serviceTier: conversation.l0(selected).getServiceTierForSubmission$() ?? null,
        historyDisabled: shared.textHistoryDisabled(), doNotRemember: state.is_do_not_remember === true };
    }
    const snapshot = read(), fingerprint = JSON.stringify(snapshot);
    function owns() {
      return document === page.document && token === page.__elonChatGptDocumentToken && href === page.location.href &&
        bindings.state().profile_id === PROFILE && account() === ownerAccount && registeredOwner() &&
        selected.serverId$() === snapshot.conversationId;
    }
    function current() {
      try { return owns() && JSON.stringify(read()) === fingerprint; } catch (_) { return false; }
    }
    function historyAllowed() {
      const status = shared.Fx(selected);
      return shared.Fl(shared.HM.getRequestId(tree())) === false &&
        (status == null || status.value === shared.v7.UNREAD || status.value === shared.v7.STREAMING) &&
        !submit.state?.().pending && !page.__elonChatGptPrivateRegenerateRuntime?.state?.().pending &&
        !page.__elonChatGptPrivateTextTransactionRelay?.state?.().active &&
        !page.__elonChatGptPrivateStopRuntime?.state?.().pending &&
        !page.__elonChatGptCanvasDocumentActions?.generationPending?.();
    }
    if (!current()) fail('context_changed');
    return Object.freeze({ ...snapshot, token, current, owns, shared, runtime: conversation,
      canReconcile(userMessageId) {
        if (!owns() || !historyAllowed()) return false;
        const state = tree(), leaf = shared.HM.getCurrentMessage(state);
        return leaf?.id === snapshot.parentId || leaf?.id === userMessageId ||
          shared.HM.getParentPromptNode(state, leaf?.id)?.id === userMessageId;
      },
      canStop(userMessageId) {
        return this.canReconcile(userMessageId);
      },
      reconciled(userMessageId, stopped = false, emptyStopped = false) {
        if (!owns() || !historyAllowed()) return false;
        const status = shared.Fx(selected);
        if (status != null && status.value !== shared.v7.UNREAD) return false;
        const state = tree(), user = shared.HM.getNodeIfExists(state, userMessageId);
        const parent = shared.HM.getParentNode(state, userMessageId);
        const leaf = shared.HM.getCurrentMessage(state);
        if (stopped && emptyStopped && user?.message?.author?.role === 'user' &&
            parent?.id === snapshot.parentId && leaf?.id === userMessageId && leaf.author?.role === 'user') return true;
        return user?.message?.author?.role === 'user' && parent?.id === snapshot.parentId &&
          leaf?.id !== snapshot.parentId && leaf?.id !== userMessageId &&
          leaf?.author?.role === 'assistant' && (leaf.status === 'finished_successfully' && leaf.end_turn === true ||
            stopped && leaf.status === 'finished_partial_completion') &&
          shared.HM.getParentPromptNode(state, leaf.id)?.id === userMessageId;
      }
    });
  }
  return Object.freeze({ capture, stamp });
});
