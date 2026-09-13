(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 5, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptFreshTextContext = api;
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  const PROFILE = 'web_20260912';
  const fail = code => { throw Error(code); };
  const idPattern = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;
  const projectPattern = /^g-p-[a-f0-9]{32}$/i;
  const routePattern = /^(?:\/g\/(g-p-[a-f0-9]{32})(?:-[A-Za-z0-9_-]{1,124})?)?\/c\/([a-f0-9-]{36})$/i;
  const newProjectPattern = /^\/g\/(g-p-[a-f0-9]{32})(?:-[A-Za-z0-9_-]{1,124})?\/project$/i;

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
    const existingRoute = routePattern.exec(url.pathname);
    const newRoute = url.pathname === '/' ? [url.pathname, null, null] : newProjectPattern.exec(url.pathname);
    const route = existingRoute || newRoute;
    if (url.origin !== 'https://chatgpt.com' || url.search || url.hash || url.username || url.password ||
        !route || !existingRoute && options.allowNewConversations !== true) fail('scope_unsupported');
    const submit = page.__elonChatGptPrivateTextRuntimeSubmit;
    const binding = submit?.captureConversation?.(composer);
    if (!binding || binding.temporary || binding.href !== href ||
        (existingRoute ? binding.newThread || !idPattern.test(route[2]) : !binding.newThread || binding.serverId !== null)) {
      fail('context_unavailable');
    }
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
    const newConversation = !existingRoute;
    let serverId = binding.serverId, navigating = false, navigated = false, targetNavigationKey = null;
    if (newConversation && (['textBindConversationId', 'textClientConversation', 'textResolvedConversationId',
      'textNavigationKey', 'textNavigate', 'canvasQueryClient'].some(key => typeof shared[key] !== 'function') ||
      ['textRequestedDefaultModel', 'textRememberFirstModel', 'textNavigateConversation']
        .some(key => typeof conversation[key] !== 'function') ||
      shared.textClientConversation(selected.id) !== true || shared.textResolvedConversationId(selected.id) != null)) {
      fail('runtime_unavailable');
    }
    const navigationKey = newConversation ? shared.textNavigationKey() : null;
    if (newConversation && (typeof navigationKey !== 'string' || !navigationKey || navigationKey.length > 256)) {
      fail('context_unavailable');
    }
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
      const matches = conversations.filter(item => serverId === null ? item?.id === selected.id : item?.serverId$?.() === serverId);
      return matches.length === 1 && matches[0] === selected;
    }

    function scope(state) {
      if (!state || shared.wV?.(shared.SV?.isPersonalWorkspace) !== true) fail('scope_unsupported');
      const projectId = shared.HM.getGizmoId(state) ?? null;
      if (projectId === null) {
        if (route[1] || state.mode?.kind !== 'primary_assistant' || state.mode.gizmo_id != null) fail('scope_unsupported');
        return null;
      }
      if (options.allowProjects !== true || typeof projectId !== 'string' || !projectPattern.test(projectId) ||
          route[1] && route[1] !== projectId || state.mode?.kind !== 'gizmo_interaction' ||
          state.mode.gizmo_id !== projectId ||
          Object.keys(state.mode).some(key => !['kind', 'gizmo_id', 'gizmo'].includes(key)) ||
          state.isLoading !== false || state.is_do_not_remember !== false ||
          state.sharedProjectConversationOwner != null || state.continuingFromSharedProjectConversationId != null ||
          state.contextScopes != null && (!Array.isArray(state.contextScopes) || state.contextScopes.length)) fail('scope_unsupported');
      return projectId;
    }

    function projectHeaders(state, projectId) {
      if (projectId === null) return Object.freeze({});
      if (['textBusinessContext', 'textProjectHeaders', 'textLockedProjectId', 'textLockedChatPin', 'canvasQueryClient']
        .some(key => typeof shared[key] !== 'function') ||
          typeof shared.HM.getConversationTurns !== 'function') fail('runtime_unavailable');
      if (shared.textBusinessContext({ turns: shared.HM.getConversationTurns(state),
        gizmoId: projectId, conversationId: binding.serverId }) !== null) fail('scope_unsupported');
      // Match OB: reuse the current account's authorized project PIN only inside
      // the page. No credential or project instruction is copied into native UI.
      const headers = shared.textProjectHeaders(projectId,
        shared.textLockedProjectId(shared.canvasQueryClient()), shared.textLockedChatPin());
      if (headers === undefined) return Object.freeze({});
      if (!headers || typeof headers !== 'object' || Array.isArray(headers) ||
          Object.keys(headers).length !== 1 || typeof headers['x-openai-locked-chats-pin'] !== 'string' ||
          !headers['x-openai-locked-chats-pin'] || headers['x-openai-locked-chats-pin'].length > 65536 ||
          /[\r\n]/.test(headers['x-openai-locked-chats-pin'])) fail('scope_unsupported');
      return Object.freeze({ 'x-openai-locked-chats-pin': headers['x-openai-locked-chats-pin'] });
    }

    function read() {
      const state = tree(), props = binding.shared.getSharedProps();
      if (!state || !registeredOwner() || props.conversation !== selected || props.composerController !== binding.controller ||
          props.isDisabled !== false || props.isConsumerLockdownModeLoadingForConversation !== false ||
          props.shouldBlockConsumerLockdownModeActionsForConversation !== false ||
          props.structuredInputMessageId != null || (selected.serverId$() ?? null) !== binding.serverId ||
          (newConversation ? serverId !== null || props.isNewThread !== true :
            !idPattern.test(binding.serverId || '') || route[2] !== binding.serverId) ||
          shared.cX?.() !== false || shared.uo(selected) !== false ||
          conversation.textPrepareEnabled() !== true || conversation.textReviewAck(selected) != null) fail('scope_unsupported');
      const projectId = scope(state);
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
      const rootParent = newConversation && parent?.id === 'client-created-root' && parent.author?.role === 'root' &&
        parent.content?.content_type === 'text' && Array.isArray(parent.content.parts) && parent.content.parts.length === 0 &&
        shared.HM.getIsNewConversation?.(state) === true && shared.HM.getAllMessages?.(state)?.length === 1 &&
        state.isLoading === false && state.is_do_not_remember === false && shared.textHistoryDisabled() === false;
      if (!parent || parent.id !== props.currentLeafId ||
          (newConversation ? !rootParent : !idPattern.test(parent.id) || !(completedAssistant || confirmedStoppedUser)) ||
          shared.textModelOverride()?.model_slug === model?.id) fail('parent_unavailable');
      const requestedDefaultModel = newConversation ? conversation.textRequestedDefaultModel(selected, model?.id) ?? null : null;
      if (requestedDefaultModel !== null && (typeof requestedDefaultModel !== 'string' ||
          !/^[a-z0-9][a-z0-9._-]{0,127}$/i.test(requestedDefaultModel))) fail('context_invalid');
      return { conversationId: binding.serverId, parentId: parent.id, parentRole: parent.author.role, model: model?.id,
        newConversation, requestedDefaultModel,
        tool: selectedTool, projectId, projectHeaders: projectHeaders(state, projectId),
        effort: conversation.yRt(selected).conversationThinkingEffort$() ?? null,
        serviceTier: conversation.l0(selected).getServiceTierForSubmission$() ?? null,
        historyDisabled: shared.textHistoryDisabled(), doNotRemember: state.is_do_not_remember === true };
    }
    const snapshot = read(), fingerprint = JSON.stringify(snapshot);
    function ownedRoute() {
      if (!newConversation) return page.location.href === href;
      if (navigating && serverId) {
        const currentUrl = new URL(page.location.href), target = routePattern.exec(currentUrl.pathname);
        if (currentUrl.origin === url.origin && !currentUrl.search && !currentUrl.hash &&
            !currentUrl.username && !currentUrl.password && target?.[2] === serverId &&
            (!target[1] || target[1] === snapshot.projectId)) {
          const key = shared.textNavigationKey();
          if (!navigated) { navigated = true; targetNavigationKey = key; }
          return typeof key === 'string' && !!key && key === targetNavigationKey;
        }
      }
      return !navigated && page.location.href === href && shared.textNavigationKey() === navigationKey &&
        binding.shared.getSharedProps().conversation === selected;
    }
    function owns() {
      try {
        return document === page.document && token === page.__elonChatGptDocumentToken && ownedRoute() &&
          bindings.state().profile_id === PROFILE && account() === ownerAccount && registeredOwner() &&
          (!newConversation || shared.cX?.() === false && shared.uo(selected) === false &&
            shared.textHistoryDisabled() === false && tree()?.is_do_not_remember === false) &&
          (selected.serverId$() ?? null) === serverId && scope(tree()) === snapshot.projectId;
      } catch (_) { return false; }
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
    return Object.freeze({ ...snapshot, get conversationId() { return serverId; }, token, current, owns, shared, runtime: conversation,
      beforeDispatch() {
        if (!current()) fail('context_changed');
        if (newConversation) conversation.textRememberFirstModel(selected, snapshot.requestedDefaultModel ?? snapshot.model);
      },
      adoptConversation(id, payload, userMessageId) {
        if (!owns() || !idPattern.test(id || '')) return false;
        if (serverId !== null) return serverId === id;
        const input = payload?.input_message || (payload?.message?.author?.role === 'user' ? payload.message : null);
        const message = input || payload?.message;
        // This callback is invoked only by this command's decoded HTTP/topic
        // stream, never by passive observers or a directory response.
        if (input && input.id !== userMessageId || !message && payload?.type !== 'resume_conversation_token' ||
            message && (!idPattern.test(message.id || '') || !['user', 'assistant'].includes(message.author?.role))) return false;
        if (shared.canvasConversations().some(item => item?.serverId$?.() === id) ||
            shared.textResolvedConversationId(selected.id) != null) return false;
        shared.textBindConversationId(selected.id, id);
        serverId = id;
        return owns() && shared.textResolvedConversationId(selected.id) === id;
      },
      async finalize(signal) {
        if (!newConversation) return true;
        if (signal.aborted || !owns() || !serverId) return false;
        let active = true, navigationFailed = false;
        try {
          if (!navigated) {
            navigating = true;
            // Project route resolution may finish after the user leaves. Expire
            // its callback with this attempt, not just with the page lifetime.
            const navigate = (target, options) => {
              if (active && !signal.aborted && owns()) shared.textNavigate(target, options);
            };
            Promise.resolve(conversation.textNavigateConversation(navigate, shared.canvasQueryClient(), serverId,
              snapshot.projectId !== null, undefined, true)).catch(() => { navigationFailed = true; });
          }
          for (let attempt = 0; attempt < 30; attempt++) {
            if (signal.aborted || navigationFailed || !owns()) return false;
            if (navigated) return true;
            await new Promise(resolve => page.setTimeout(resolve, 100));
          }
          return false;
        } finally { active = false; }
      },
      navigationReady: () => !newConversation || navigated,
      canReconcile(userMessageId) {
        if (!serverId || !owns() || !historyAllowed()) return false;
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
