(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 2, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptFreshTextRecoveryContext = api;
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  const profiles = ['web_20260912', 'web_20260915', 'web_20260915_b'];
  const uuid = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;
  const project = /^g-p-[a-f0-9]{32}$/i;
  const fail = code => { throw Error(code); };

  async function capture(signal) {
    const document = page.document, token = page.__elonChatGptDocumentToken, href = page.location.href;
    if (signal?.aborted) fail('context_changed');
    if (href === 'https://chatgpt.com/?temporary-chat=true') return null;
    const bindings = page.__elonChatGptPrivateRuntimeBindings, profile = bindings?.state?.().profile_id;
    if (!profiles.includes(profile) || !bindings.observed?.('shared')) fail('runtime_unavailable');
    const shared = bindings.peek('shared');
    if (!shared || typeof shared.canvasConversations !== 'function' || typeof shared.XM !== 'function' ||
        typeof shared.HM?.getGizmoId !== 'function') fail('recovery_identity_unavailable');
    const identity = page.__elonChatGptPrivateModelContract?.create(page);
    const account = () => identity?.withRuntimeIdentity({}, shared)?.account;
    const accountId = account();
    if (typeof accountId !== 'string' || !accountId) fail('recovery_identity_unavailable');
    // The committed provider owns routing, but its editor, draft, selected model
    // and current parent are not prerequisites for reading an old send result.
    const owner = page.__elonChatGptPrivateTextRuntimeSubmit?.captureConversation?.(null, false, true);
    if (!owner || owner.current?.() !== true || owner.href !== href) fail('context_unavailable');
    const selected = owner.conversation, state = () => shared.XM(selected.id);
    const privateMode = () => owner.temporary === true || shared.cX?.() === true ||
      shared.textHistoryDisabled?.() === true || state()?.is_do_not_remember === true;
    if (privateMode()) return null;
    const projectId = shared.HM.getGizmoId(state()) ?? null, conversationId = owner.serverId;
    if (conversationId !== null && !uuid.test(conversationId) ||
        conversationId === null && owner.newThread !== true ||
        projectId !== null && (typeof projectId !== 'string' || !project.test(projectId))) fail('context_unavailable');
    const projectMatch = new URL(href).pathname.match(/^\/g\/(g-p-[a-f0-9]{32})(?:-|\/)/i);
    if (projectMatch && projectMatch[1] !== projectId) fail('context_unavailable');
    function owns() {
      try {
        if (signal?.aborted || document !== page.document || token !== page.__elonChatGptDocumentToken ||
            href !== page.location.href || bindings.state().profile_id !== profile ||
            bindings.peek('shared') !== shared || account() !== accountId || owner.current() !== true ||
            (selected.serverId$() || null) !== conversationId) return false;
        const entries = shared.canvasConversations(), tree = state();
        if (!Array.isArray(entries) || entries.length > 512 || entries.filter(item =>
          conversationId === null ? item?.id === selected.id : item?.serverId$?.() === conversationId).length !== 1 ||
            !entries.includes(selected) || !tree || shared.wV?.(shared.SV?.isPersonalWorkspace) !== true ||
            shared.cX?.() !== false || shared.uo?.(selected) !== false || shared.textHistoryDisabled?.() !== false ||
            tree.is_do_not_remember !== false || (shared.HM.getGizmoId(tree) ?? null) !== projectId) return false;
        if (projectId === null ? tree.mode?.kind !== 'primary_assistant' :
            tree.mode?.kind !== 'gizmo_interaction' || tree.mode.gizmo_id !== projectId ||
            tree.sharedProjectConversationOwner != null &&
              tree.sharedProjectConversationOwner.id !== shared.mq?.()?.normalizedAccountUserId) return false;
        if (tree.continuingFromSharedConversationId != null || tree.continuingFromSharedProjectConversationId != null) return false;
        // Do not hydrate across a new official write that started during the read.
        const status = shared.Fx?.(selected);
        const voiceState = page.__elonChatGptPrivateVoiceRelay?.state?.();
        const voice = voiceState ? JSON.parse(voiceState) : null;
        if (voice?.armed || voice?.inFlight || voice?.takeoverActive) return false;
        return shared.Fl?.(shared.HM.getRequestId?.(tree)) === false &&
          (status == null || status.value === shared.v7?.UNREAD) &&
          !page.__elonChatGptPrivateTextRuntimeSubmit?.state?.().pending &&
          !page.__elonChatGptPrivateTextTransactionRelay?.state?.().active &&
          !page.__elonChatGptPrivateRegenerateRuntime?.state?.().pending &&
          !page.__elonChatGptPrivateStopRuntime?.state?.().pending &&
          !page.__elonChatGptCanvasDocumentActions?.generationPending?.();
      } catch (_) { return false; }
    }
    if (!owns()) fail('context_unavailable');
    if (!bindings.observed('conversation')) fail('runtime_unavailable');
    const runtime = await bindings.load('conversation');
    if (!owns()) fail('context_changed');
    if (typeof runtime?.textHydrateHistory !== 'function') fail('runtime_unavailable');
    return Object.freeze({ conversationId, projectId, newConversation: owner.newThread === true,
      temporary: false, historyDisabled: false, doNotRemember: false,
      runtime, owns, recoveryIdentity: () => owns() ? accountId : null });
  }
  function stamp() {
    try {
      const bindings = page.__elonChatGptPrivateRuntimeBindings;
      if (!profiles.includes(bindings?.state?.().profile_id)) return null;
      const shared = bindings.peek('shared');
      const account = page.__elonChatGptPrivateModelContract?.create(page).withRuntimeIdentity({}, shared)?.account;
      const owner = page.__elonChatGptPrivateTextRuntimeSubmit?.captureConversation?.(null, false, true);
      return account && owner?.current?.() === true ? JSON.stringify([
        account, owner.conversation.id, owner.serverId, owner.temporary, bindings.state().profile_id,
      ]) : null;
    } catch (_) { return null; }
  }
  return Object.freeze({ capture, stamp });
});
