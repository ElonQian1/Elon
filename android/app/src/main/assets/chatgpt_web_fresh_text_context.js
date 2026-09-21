(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 29, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptFreshTextContext = api;
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  const PROFILES = ['web_20260912', 'web_20260915', 'web_20260915_b', 'web_20260921', 'web_20260922', 'web_20260922_b'];
  const fail = (code, admissionStage) => { throw Object.assign(Error(code), { admissionStage }); };
  const idPattern = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;
  const modelPattern = /^[a-z0-9][a-z0-9._-]{0,127}$/i;
  const projectPattern = /^g-p-[a-f0-9]{32}$/i;
  const routePattern = /^(?:\/g\/(g-p-[a-f0-9]{32})(?:-[A-Za-z0-9_-]{1,124})?)?\/c\/([a-f0-9-]{36})$/i;
  const newProjectPattern = /^\/g\/(g-p-[a-f0-9]{32})(?:-[A-Za-z0-9_-]{1,124})?\/project$/i;
  const sameOwner = (before, after) => !!before && !!after &&
    ['token', 'account', 'conversation', 'controller', 'shared', 'files', 'serverId', 'href', 'newThread', 'temporary']
      .every(key => before[key] === after[key]);

  function stamp() {
    try {
      const bindings = page.__elonChatGptPrivateRuntimeBindings;
      if (!PROFILES.includes(bindings?.state?.().profile_id)) return null;
      const shared = bindings.peek('shared');
      const account = page.__elonChatGptPrivateModelContract?.create(page).withRuntimeIdentity({}, shared)?.account;
      return account ? JSON.stringify([page.__elonChatGptDocumentToken, page.location.href, account]) : null;
    } catch (_) { return null; }
  }

  function identityBootstrap() {
    try {
      const bindings = page.__elonChatGptPrivateRuntimeBindings;
      const profile = bindings?.state?.().profile_id;
      const token = page.__elonChatGptDocumentToken, href = page.location.href, document = page.document;
      const url = new URL(href);
      if (!PROFILES.includes(profile) || !/^doc_[a-z0-9_]{3,80}$/.test(token || '') ||
          url.origin !== 'https://chatgpt.com' || url.username || url.password || url.hash ||
          url.search && !(url.pathname === '/' && url.search === '?temporary-chat=true') ||
          url.pathname !== '/' && !routePattern.test(url.pathname) && !newProjectPattern.test(url.pathname) ||
          bindings.peek('shared') || bindings.observed?.('shared') !== true) return null;
      const current = () => document === page.document && token === page.__elonChatGptDocumentToken &&
        href === page.location.href && page.__elonChatGptPrivateRuntimeBindings === bindings &&
        bindings.state().profile_id === profile && page.document.hidden !== true && page.navigator?.onLine !== false;
      // This imports a reviewed module only. It captures no draft/command and
      // grants no send permission; the next snapshot validates the live account.
      return Object.freeze({ key: JSON.stringify([token, href, profile]), current,
        async load() {
          if (!current()) return false;
          await bindings.load('shared');
          return current() && stamp() !== null;
        } });
    } catch (_) { return null; }
  }

  async function capture(composer, stoppedParent, options = {}) {
    const bindings = page.__elonChatGptPrivateRuntimeBindings;
    const profile = bindings?.state?.().profile_id;
    if (!PROFILES.includes(profile)) fail('runtime_unavailable');
    const token = page.__elonChatGptDocumentToken, href = page.location.href, document = page.document;
    const url = new URL(href);
    const temporary = url.pathname === '/' && url.search === '?temporary-chat=true';
    const existingRoute = routePattern.exec(url.pathname);
    const newRoute = url.pathname === '/' ? [url.pathname, null, null] : newProjectPattern.exec(url.pathname);
    const route = existingRoute || newRoute;
    if (url.origin !== 'https://chatgpt.com' || url.search && !temporary || url.hash || url.username || url.password ||
        !route || temporary && options.allowTemporary !== true ||
        !existingRoute && !temporary && options.allowNewConversations !== true) fail('scope_unsupported', 'base_route');
    const submit = page.__elonChatGptPrivateTextRuntimeSubmit;
    const captureOwner = composer?.isConnected ? submit?.captureConversation : submit?.capturePrivateConversation;
    let binding = captureOwner?.(composer);
    if (!binding && !composer?.isConnected && bindings.observed?.('composer') === true && !bindings.peek('composer')) {
      // Pin the committed owner before joining the known module import. A late
      // import must never redirect this command to another conversation's draft.
      const before = submit?.captureConversation?.(composer, false, true);
      if (!before || before.current?.() !== true) fail('context_unavailable');
      await bindings.load('composer');
      if (document !== page.document || token !== page.__elonChatGptDocumentToken || href !== page.location.href ||
          bindings.state().profile_id !== profile) fail('context_changed');
      binding = captureOwner?.(composer);
      if (!sameOwner(before, binding) || before.current?.() !== true) fail('context_changed');
    }
    const newConversation = binding?.newThread === true;
    if (!binding || binding.temporary !== temporary || binding.href !== href ||
        (temporary ? typeof binding.newThread !== 'boolean' ||
          (newConversation ? binding.serverId !== null : !idPattern.test(binding.serverId || '')) :
          existingRoute ? binding.newThread || !idPattern.test(route[2]) : !binding.newThread || binding.serverId !== null)) {
      fail('context_unavailable');
    }
    if (newConversation && options.allowNewConversations !== true) fail('scope_unsupported', 'base_new');
    const [shared, conversation, editor] = await Promise.all([
      bindings.load('shared'), bindings.load('conversation'), bindings.load('composer')
    ]);
    if (document !== page.document || token !== page.__elonChatGptDocumentToken || href !== page.location.href) fail('context_changed');
    const afterLoad = captureOwner(composer);
    if (!sameOwner(binding, afterLoad)) fail('context_changed');
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
    let serverId = binding.serverId, navigating = false, navigated = false, targetNavigationKey = null, navigationTask = null;
    if (newConversation && (['textBindConversationId', 'textClientConversation', 'textResolvedConversationId',
      ...(temporary ? [] : ['textNavigate', 'canvasQueryClient'])].some(key => typeof shared[key] !== 'function') ||
      ['textRequestedDefaultModel', 'textRememberFirstModel', ...(temporary ? [] : ['textNavigateConversation'])]
        .some(key => typeof conversation[key] !== 'function') ||
      shared.textClientConversation(selected.id) !== true || shared.textResolvedConversationId(selected.id) != null)) {
      fail('runtime_unavailable');
    }
    if ((newConversation || temporary) && typeof shared.textNavigationKey !== 'function') fail('runtime_unavailable');
    const navigationKey = newConversation || temporary ? shared.textNavigationKey() : null;
    if ((newConversation || temporary) && (typeof navigationKey !== 'string' || !navigationKey || navigationKey.length > 256)) {
      fail('context_unavailable');
    }
    const selectedTool = editor.Ng(binding.controller)?.activeSystemHintType;
    const selectedFiles = binding.files.files$(), readyFiles = binding.files.readyFiles$();
    let attachments = null;
    if (!Array.isArray(selectedFiles) || !Array.isArray(readyFiles)) fail('attachments_active');
    const personalAttachmentsAllowed = state => options.allowPersonalAttachments === true &&
      !newConversation && !temporary && !route[1] && selectedTool === null &&
      shared.HM.getGizmoId(state) == null && state?.mode?.kind === 'primary_assistant' &&
      selectedFiles.length > 0 && selectedFiles.length <= 9 && selectedFiles.every(file =>
        file.source === 'local' && file.libraryFileId == null && file.mountedLibraryFileId == null &&
        ['text/plain', 'application/pdf', 'image/png'].includes(file.fileSpec?.mimeType?.toLowerCase()));
    if (selectedFiles.length || readyFiles.length) {
      if (options.allowAttachments !== true && !personalAttachmentsAllowed(tree()) ||
          !page.__elonChatGptFreshTextAttachments) fail('attachments_active');
      attachments = page.__elonChatGptFreshTextAttachments.capture(page, binding, conversation);
    }
    if (options.requireNativeAttachment === true && !attachments) fail('attachments_active');
    let toolOwner = null;
    const personalToolAllowed = state =>
      (options.allowPersonalSearch === true && selectedTool === 'search' ||
        options.allowPersonalImage === true && selectedTool === 'picture_v2') &&
      !newConversation && !temporary && !attachments && !route[1] &&
      shared.HM.getGizmoId(state) == null && state?.mode?.kind === 'primary_assistant';
    if (selectedTool !== null) {
      if (options.allowTools !== true && !personalToolAllowed(tree()) || !['search', 'picture_v2'].includes(selectedTool)) fail('tools_active');
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

    function projectUserId() {
      const user = shared.mq?.()?.normalizedAccountUserId;
      if (typeof user !== 'string' || !/^[A-Za-z0-9_-]{1,160}$/.test(user)) fail('identity_unavailable', 'project_shared');
      return user;
    }

    function scope(state) {
      if (!state || shared.wV?.(shared.SV?.isPersonalWorkspace) !== true) fail('scope_unsupported', 'base_workspace');
      const projectId = shared.HM.getGizmoId(state) ?? null;
      if (temporary && projectId !== null) fail('scope_unsupported', 'base_project');
      if (projectId === null) {
        if (route[1] || state.mode?.kind !== 'primary_assistant' || state.mode.gizmo_id != null) fail('scope_unsupported', 'base_mode');
        return null;
      }
      const existingPlainProject = options.allowExistingProjects === true && !newConversation &&
        !temporary && !attachments && selectedTool === null;
      if (options.allowProjects !== true && !existingPlainProject ||
          typeof projectId !== 'string' || !projectPattern.test(projectId)) fail('scope_unsupported', 'base_project');
      if (route[1] && route[1] !== projectId) fail('scope_unsupported', 'project_route');
      if (state.mode?.kind !== 'gizmo_interaction' || state.mode.gizmo_id !== projectId ||
          Object.keys(state.mode).some(key => !['kind', 'gizmo_id', 'gizmo'].includes(key))) fail('scope_unsupported', 'project_mode');
      if (state.isLoading !== false) fail('scope_unsupported', 'project_loading');
      if (state.is_do_not_remember !== false) fail('scope_unsupported', 'project_privacy');
      if (state.continuingFromSharedProjectConversationId != null ||
          state.sharedProjectConversationOwner != null && (typeof state.sharedProjectConversationOwner !== 'object' ||
            Array.isArray(state.sharedProjectConversationOwner) ||
            state.sharedProjectConversationOwner.id !== projectUserId())) fail('scope_unsupported', 'project_shared');
      // Official hydration retains GLOBAL on ordinary projects. Restricted scopes
      // still require their own transport contract; sparse arrays are not evidence.
      if (state.contextScopes != null && (!Array.isArray(state.contextScopes) ||
          Array.from(state.contextScopes).some(value => value !== 'GLOBAL'))) fail('scope_unsupported', 'project_scopes');
      return projectId;
    }

    function projectHeaders(state, projectId) {
      if (projectId === null) return Object.freeze({});
      if (['textBusinessContext', 'textProjectHeaders', 'textLockedProjectId', 'textLockedChatPin', 'canvasQueryClient']
        .some(key => typeof shared[key] !== 'function') ||
          typeof shared.HM.getConversationTurns !== 'function') fail('runtime_unavailable');
      if (shared.textBusinessContext({ turns: shared.HM.getConversationTurns(state),
        gizmoId: projectId, conversationId: binding.serverId }) !== null) fail('scope_unsupported', 'project_business');
      // Match OB: reuse the current account's authorized project PIN only inside
      // the page. No credential or project instruction is copied into native UI.
      const headers = shared.textProjectHeaders(projectId,
        shared.textLockedProjectId(shared.canvasQueryClient()), shared.textLockedChatPin());
      if (headers === undefined) return Object.freeze({});
      if (options.allowProjects !== true) fail('scope_unsupported', 'project_headers');
      if (!headers || typeof headers !== 'object' || Array.isArray(headers) ||
          Object.keys(headers).length !== 1 || typeof headers['x-openai-locked-chats-pin'] !== 'string' ||
          !headers['x-openai-locked-chats-pin'] || headers['x-openai-locked-chats-pin'].length > 65536 ||
          /[\r\n]/.test(headers['x-openai-locked-chats-pin'])) fail('scope_unsupported', 'project_headers');
      return Object.freeze({ 'x-openai-locked-chats-pin': headers['x-openai-locked-chats-pin'] });
    }

    function read() {
      const state = tree(), props = binding.shared.getSharedProps();
      if (!state || !registeredOwner() || props.conversation !== selected || props.composerController !== binding.controller) fail('scope_unsupported', 'base_owner');
      if (props.isDisabled !== false || props.isConsumerLockdownModeLoadingForConversation !== false ||
          props.shouldBlockConsumerLockdownModeActionsForConversation !== false ||
          props.structuredInputMessageId != null) fail('scope_unsupported', 'base_composer');
      if ((selected.serverId$() ?? null) !== binding.serverId ||
          (newConversation ? serverId !== null || props.isNewThread !== true :
            !idPattern.test(binding.serverId || '') || !temporary && route[2] !== binding.serverId ||
              temporary && (props.isNewThread !== false || shared.HM.getIsNewConversation?.(state) !== false))) fail('scope_unsupported', 'base_route_state');
      if (shared.cX?.() !== temporary || shared.uo(selected) !== false ||
          temporary && (typeof state.is_do_not_remember !== 'boolean' || shared.textHistoryDisabled() !== true)) fail('scope_unsupported', 'base_privacy');
      if (conversation.textPrepareEnabled() !== true || conversation.textReviewAck(selected) != null) fail('scope_unsupported', 'base_prepare');
      const projectId = scope(state);
      if (attachments && options.allowAttachments !== true && !personalAttachmentsAllowed(state)) fail('attachments_active');
      if (toolOwner && options.allowTools !== true && !personalToolAllowed(state)) fail('tools_active');
      if (['continuingFromSharedConversationId', 'continuingFromSharedProjectConversationId', 'continuingFromSharedPostId',
        'forkFromSharedPost', 'branchingFromMessageId', 'branchingFromConversationId', 'continuationBranch',
        'hideFromHistory', 'conversationOrigin'].some(key => state[key] != null && state[key] !== false)) fail('scope_unsupported', 'base_branch');
      if (Object.entries(selected.config || {}).some(([key, value]) => value != null && value !== false &&
          !(projectId !== null && key === 'urlGizmoId' && value === projectId))) fail('scope_unsupported', 'base_config');
      const files = binding.files.files$(), ready = binding.files.readyFiles$();
      if (!Array.isArray(files) || !Array.isArray(ready) || binding.files.hasUploadInProgress$() !== false ||
          (attachments ? !attachments.current() || files.length !== selectedFiles.length || ready.length !== files.length
            : files.length || ready.length)) fail('attachments_active');
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
      if (typeof model?.id !== 'string' || !modelPattern.test(model.id)) fail('context_unavailable', 'base_model');
      if (toolOwner && toolOwner.model !== model?.id) fail('context_changed');
      const completedAssistant = parent?.author?.role === 'assistant' &&
        (parent.status === 'finished_partial_completion' || parent.status === 'finished_successfully' && parent.end_turn === true);
      const confirmedStoppedUser = parent?.author?.role === 'user' && stoppedParent?.id === parent.id &&
        stoppedParent.current() === true;
      const rootParent = newConversation && parent?.id === 'client-created-root' && parent.author?.role === 'root' &&
        parent.content?.content_type === 'text' && Array.isArray(parent.content.parts) && parent.content.parts.length === 0 &&
        shared.HM.getIsNewConversation?.(state) === true && shared.HM.getAllMessages?.(state)?.length === 1 &&
        state.isLoading === false && typeof state.is_do_not_remember === 'boolean' &&
        (temporary || state.is_do_not_remember === false) && shared.textHistoryDisabled() === temporary;
      if (!parent || parent.id !== props.currentLeafId ||
          (newConversation ? !rootParent : !idPattern.test(parent.id) || !(completedAssistant || confirmedStoppedUser)) ||
          shared.textModelOverride()?.model_slug === model?.id) fail('parent_unavailable');
      const requestedDefaultModel = newConversation ? conversation.textRequestedDefaultModel(selected, model?.id) ?? null : null;
      if (requestedDefaultModel !== null && (typeof requestedDefaultModel !== 'string' ||
          !modelPattern.test(requestedDefaultModel))) fail('context_invalid');
      let temporaryPersonalization = null;
      if (temporary && newConversation) {
        if (['textTemporaryPersonalizationEnabled', 'textTemporaryPersonalization', 'textReadUntracked']
          .some(key => typeof shared[key] !== 'function')) fail('runtime_unavailable');
        const enabled = shared.textTemporaryPersonalizationEnabled();
        if (typeof enabled !== 'boolean') fail('context_invalid');
        if (enabled) {
          temporaryPersonalization = shared.textReadUntracked(() => shared.textTemporaryPersonalization(selected.id));
          if (typeof temporaryPersonalization !== 'boolean') fail('context_invalid');
        }
      }
      return { conversationId: binding.serverId, parentId: parent.id, parentRole: parent.author.role, model: model?.id,
        newConversation, requestedDefaultModel, temporary, temporaryPersonalization,
        tool: selectedTool, projectId, projectUserId: projectId === null ? null : projectUserId(),
        projectHeaders: projectHeaders(state, projectId),
        effort: conversation.yRt(selected).conversationThinkingEffort$() ?? null,
        serviceTier: conversation.l0(selected).getServiceTierForSubmission$() ?? null,
        historyDisabled: shared.textHistoryDisabled(), doNotRemember: state.is_do_not_remember === true };
    }
    const snapshot = read(), fingerprint = JSON.stringify(snapshot);
    attachments?.prepare(snapshot);
    function ownedRoute() {
      if (temporary) return page.location.href === href && shared.textNavigationKey() === navigationKey &&
        binding.shared.getSharedProps().conversation === selected;
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
    let ownershipStage = 'not_observed', reconciliationStage = 'not_observed', storeStage = 'store_not_reconciled';
    function owns() {
      try {
        ownershipStage = 'document'; if (document !== page.document) return false;
        ownershipStage = 'document_token'; if (token !== page.__elonChatGptDocumentToken) return false;
        ownershipStage = 'route'; if (!ownedRoute()) return false;
        ownershipStage = 'runtime'; if (bindings.state().profile_id !== profile) return false;
        ownershipStage = 'account'; if (account() !== ownerAccount) return false;
        ownershipStage = 'registry'; if (!registeredOwner()) return false;
        ownershipStage = 'history_scope';
        if (!(temporary ? shared.cX?.() === true && shared.uo(selected) === false && shared.textHistoryDisabled() === true &&
            typeof tree()?.is_do_not_remember === 'boolean' : !newConversation || shared.cX?.() === false &&
              shared.uo(selected) === false && shared.textHistoryDisabled() === false && tree()?.is_do_not_remember === false)) return false;
        ownershipStage = 'server_id'; if ((selected.serverId$() ?? null) !== serverId) return false;
        ownershipStage = 'project_scope'; if (scope(tree()) !== snapshot.projectId) return false;
        if (snapshot.projectId !== null && projectUserId() !== snapshot.projectUserId) return false;
        ownershipStage = 'owned'; return true;
      } catch (_) { ownershipStage = 'context_error'; return false; }
    }
    function current() {
      try { return owns() && (!binding.memoryOwner || binding.current()) && JSON.stringify(read()) === fingerprint; } catch (_) { return false; }
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
    async function finalizeRoute(signal) {
      if (temporary) return !signal.aborted && !!serverId && owns();
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
    }
    if (!current()) fail('context_changed');
    return Object.freeze({ ...snapshot, get conversationId() { return serverId; }, token, current, owns, shared, runtime: conversation,
      recoveryIdentity: () => owns() ? ownerAccount : null,
      attachments, draft: binding.draft,
      diagnostics: () => ({ ownership: ownershipStage, reconciliation: reconciliationStage }),
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
      finalize(signal) {
        if (signal.aborted) return Promise.resolve(false);
        if (!navigationTask) {
          const task = finalizeRoute(signal);
          navigationTask = task;
          const clear = () => { if (navigationTask === task) navigationTask = null; };
          void task.then(clear, clear);
        }
        return navigationTask.then(value => !signal.aborted && value);
      },
      navigationReady: () => temporary || !newConversation || navigated,
      reconciliationFailure: () => storeStage,
      canReconcile(userMessageId) {
        reconciliationStage = 'identity'; if (!serverId || !owns()) return false;
        reconciliationStage = 'history_busy'; if (!historyAllowed()) return false;
        const state = tree(), leaf = shared.HM.getCurrentMessage(state);
        const ready = leaf?.id === snapshot.parentId || leaf?.id === userMessageId ||
          shared.HM.getParentPromptNode(state, leaf?.id)?.id === userMessageId;
        reconciliationStage = ready ? 'ready' : 'leaf_mismatch';
        return ready;
      },
      canStop(userMessageId) {
        return this.canReconcile(userMessageId);
      },
      reconciled(userMessageId, stopped = false, emptyStopped = false, verifyNewParent = null) {
        storeStage = 'store_owner_changed'; if (!owns()) return false;
        storeStage = 'store_history_busy'; if (!historyAllowed()) return false;
        const status = shared.Fx(selected);
        storeStage = 'store_status_unsettled';
        if (status != null && status.value !== shared.v7.UNREAD) return false;
        const state = tree(), user = shared.HM.getNodeIfExists(state, userMessageId);
        const parent = shared.HM.getParentNode(state, userMessageId);
        const leaf = shared.HM.getCurrentMessage(state);
        const parentMatches = parent?.id === snapshot.parentId || newConversation &&
          typeof verifyNewParent === 'function' && verifyNewParent(shared.HM, state) === true;
        storeStage = 'store_attachments_mismatch';
        if (attachments && !attachments.matchesHistory(user?.message)) return false;
        storeStage = 'store_user_missing'; if (user?.message?.author?.role !== 'user') return false;
        storeStage = 'store_parent_mismatch'; if (!parentMatches) return false;
        if (stopped && emptyStopped && user?.message?.author?.role === 'user' &&
            parentMatches && leaf?.id === userMessageId && leaf.author?.role === 'user') {
          storeStage = 'reconciled'; return true;
        }
        storeStage = 'store_leaf_mismatch';
        if (!(leaf?.id !== snapshot.parentId && leaf?.id !== userMessageId &&
          leaf?.author?.role === 'assistant' && (leaf.status === 'finished_successfully' && leaf.end_turn === true ||
            stopped && leaf.status === 'finished_partial_completion'))) return false;
        storeStage = 'store_prompt_mismatch';
        if (shared.HM.getParentPromptNode(state, leaf.id)?.id !== userMessageId) return false;
        storeStage = 'reconciled'; return true;
      }
    });
  }
  let inspection = null;
  function inspect(composer) {
    if (inspection) return inspection;
    const result = (code, stage) => ({ schema: 'elon.fresh_text_admission.v1', code, stage });
    const codes = ['scope_unsupported', 'runtime_unavailable', 'identity_unavailable', 'context_unavailable',
      'context_changed', 'context_invalid', 'attachments_active', 'tools_active', 'conversation_busy', 'parent_unavailable'];
    const stages = ['base_route', 'base_new', 'base_owner', 'base_composer', 'base_route_state', 'base_privacy',
      'base_prepare', 'base_model', 'base_workspace', 'base_project', 'base_mode', 'base_branch', 'base_config',
      'project_business', 'project_headers', 'project_route', 'project_mode', 'project_loading',
      'project_privacy', 'project_shared', 'project_scopes'];
    let timer;
    // Inspect the same owner as a send, without arming a trial or creating a request.
    inspection = Promise.race([capture(composer, null, {
      allowProjects: true, allowNewConversations: true, allowTemporary: true
    }).then(() => result('ready', 'ready'), error => result(
      codes.includes(error?.message) ? error.message : 'read_failed',
      stages.includes(error?.admissionStage) ? error.admissionStage : 'base_context')),
    new Promise(resolve => { timer = page.setTimeout(() => resolve(result('timeout', 'timeout')), 5000); })])
      .finally(() => { page.clearTimeout(timer); inspection = null; });
    return inspection;
  }
  return Object.freeze({ capture, stamp, inspect, identityBootstrap });
});
