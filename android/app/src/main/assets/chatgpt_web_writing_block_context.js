(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 2, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptWritingBlockContext = api;
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  const fail = code => { throw Error('writing_' + code); };
  const identity = page.__elonChatGptPrivateConversationShareContract.create(page).identity;
  const policy = page.__elonChatGptWritingBlockPolicy;
  async function capture(input, snapshot) {
    const binding = { path: input.path, href: page.location.href, document: page.document,
      token: page.__elonChatGptDocumentToken, account: identity(), id: policy.PATH.exec(input.path)?.[1] };
    const url = new URL(binding.href);
    if (url.origin !== 'https://chatgpt.com' || url.pathname !== input.path || url.search || url.hash ||
        !binding.account || !/^doc_[a-z0-9_]{3,80}$/.test(binding.token || '')) fail('context_unavailable');
    const bindings = page.__elonChatGptPrivateRuntimeBindings;
    if (bindings?.state?.().profile_id !== 'web_20260912') fail('runtime_unavailable');
    let timeout;
    const shared = await Promise.race([
      bindings.load('shared'),
      new Promise((_, reject) => { timeout = page.setTimeout(() => reject(Error('writing_runtime_unavailable')), 3000); })
    ]).finally(() => page.clearTimeout(timeout));
    if (typeof shared?.canvasConversations !== 'function' || typeof shared.XM !== 'function' ||
        typeof shared.HM?.getNodeIfExists !== 'function' || typeof shared.HM.getCurrentLeafId !== 'function' ||
        typeof shared.HM.getRequestId !== 'function' || typeof shared.Fl !== 'function' ||
        typeof shared.writingUpdateState !== 'function' ||
        typeof shared.writingTreeOwner?.updateTree !== 'function') fail('runtime_unavailable');
    const matches = shared.canvasConversations().filter(value => value?.serverId$?.() === binding.id);
    if (matches.length !== 1) fail('context_unavailable');
    const selected = matches[0];
    const state = () => shared.XM(selected.id);
    const leaf = shared.HM.getCurrentLeafId(state());
    function current() {
      try {
        const value = snapshot();
        return page.document === binding.document && page.location.href === binding.href &&
          page.__elonChatGptDocumentToken === binding.token && identity() === binding.account &&
          bindings.state().profile_id === 'web_20260912' && selected.serverId$() === binding.id &&
          shared.canvasConversations().filter(item => item?.serverId$?.() === binding.id).length === 1 &&
          shared.canvasConversations().includes(selected) && shared.HM.getCurrentLeafId(state()) === leaf &&
          value?.url === binding.href && value.streaming === false && !value.dictationActive &&
          !value.dictationCaptureActive && !value.dictationCapturePending &&
          shared.Fl(shared.HM.getRequestId(state())) === false &&
          !page.__elonChatGptPrivateTextRuntimeSubmit?.state?.().pending &&
          !page.__elonChatGptPrivateTextTransactionRelay?.state?.().active &&
          !page.__elonChatGptPrivateRegenerateRuntime?.state?.().pending &&
          !page.__elonChatGptPrivateConversationDelete?.busy?.() &&
          page.__elonChatGptPrivateConversationMutation?.state?.().state !== 'busy';
      } catch (_) { return false; }
    }
    let guardedSource;
    function messageMatches(message, source) {
      if (!source || message?.id !== source.messageId) return false;
      const rows = page.__elonChatGptTextBlocks.project(message, true)?.writeSources?.filter(row => row.id === source.id) || [];
      return rows.length === 1 && policy.same(rows[0], source);
    }
    function localMatches(source) {
      if (!source || !current()) return false;
      const message = shared.HM.getNodeIfExists(state(), source.messageId)?.message;
      return messageMatches(message, source);
    }
    function local(source) {
      if (!current()) fail('context_changed');
      if (!localMatches(source)) fail('web_edit_pending');
      guardedSource = source;
    }
    async function reconcile(source, deadline) {
      try {
        if (!current() || Date.now() >= deadline || !guardedSource ||
            !policy.same({ ...source, content: guardedSource.content }, guardedSource)) return false;
        if (!localMatches(source)) {
          if (!localMatches(guardedSource)) return false;
          // The official writing action updates one node through the observable tree owner.
          // The caller has already confirmed the POST with an authoritative server readback.
          shared.writingUpdateState(selected.id, value => shared.writingTreeOwner.updateTree(value, tree => {
            if (!current() || Date.now() >= deadline || !tree.containsNode(source.messageId)) return;
            const message = tree.getMaybeMessage(source.messageId);
            if (!messageMatches(message, guardedSource)) return;
            const blocks = message.metadata?.writing_blocks || {};
            tree.updateNodeMessageMetadata(source.messageId, { writing_blocks: { ...blocks,
              [source.id]: { ...blocks[source.id], id: source.id, index: source.index,
                content: source.content, variant: source.variant, title: source.title, metadata: source.metadata }
            } });
          }));
        }
        local(source);
        return true;
      } catch (_) { return false; }
    }
    if (!current()) fail('context_changed');
    return { ...binding, current, local, reconcile };
  }
  return Object.freeze({ capture });
});
