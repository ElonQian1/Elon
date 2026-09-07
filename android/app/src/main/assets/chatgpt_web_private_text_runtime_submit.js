(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 3, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com') {
    const existing = root.__elonChatGptPrivateTextRuntimeSubmit;
    if (!(Number(existing?.version) >= exported.version) && !existing?.state?.().pending) {
      root.__elonChatGptPrivateTextRuntimeSubmit = exported.create(root);
    }
  }
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options = options || {};
  const RUNTIME_URL = 'https://chatgpt.com/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js';
  const UUID = '[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}';
  const CONVERSATION = new RegExp('^(?:/g/g-p-[a-f0-9]{32}(?:-[A-Za-z0-9_-]{1,124})?)?/c/(' + UUID + ')$', 'i');
  const PROJECT = /^\/g\/g-p-[a-f0-9]{32}(?:-[A-Za-z0-9_-]{1,124})?\/project$/i;
  let active = null;

  function route() {
    const url = new URL(page.location.href);
    const conversationId = CONVERSATION.exec(url.pathname)?.[1] || null;
    if (url.origin !== 'https://chatgpt.com' || url.username || url.password || url.hash ||
        url.search && url.search !== '?temporary-chat=true' ||
        url.pathname.startsWith('/g/') && url.search ||
        url.pathname !== '/' && !conversationId && !PROJECT.test(url.pathname)) return null;
    return { href: url.href, conversationId, temporary: url.search === '?temporary-chat=true' };
  }

  function identity() {
    const headers = page.__elonChatGptPrivateTransport?.copySameOriginRequestHeaders?.();
    if (!headers || typeof headers !== 'object') return null;
    const values = {};
    for (const [key, value] of Object.entries(headers)) values[key.toLowerCase()] = value;
    // Kept inside this document. Neither request credentials nor text enter receipts.
    return JSON.stringify(['authorization', 'chatgpt-account-id', 'oai-device-id'].map(key => values[key] || ''));
  }

  function committedAncestors(node) {
    const key = Object.keys(node).find(name => name.startsWith('__reactFiber$'));
    for (const start of [node[key], node[key]?.alternate]) {
      const ancestors = [];
      for (let fiber = start; fiber && ancestors.length < 90; fiber = fiber.return) ancestors.push(fiber);
      const top = ancestors.at(-1);
      if (top && !top.return && top.stateNode?.current === top) return ancestors;
    }
    return [];
  }

  function stores(node) {
    const shared = new Set(), files = new Set();
    function accept(value) {
      const store = value?.store || value;
      if (typeof store?.getSharedProps === 'function' && typeof store.subscribeToSharedProps === 'function') shared.add(store);
      if (typeof value?.files$ === 'function' && typeof value.readyFiles$ === 'function' &&
          typeof value.hasUploadInProgress$ === 'function') files.add(value);
    }
    for (const fiber of committedAncestors(node)) {
      accept(fiber.memoizedProps?.value);
      let context = fiber.dependencies?.firstContext;
      for (let count = 0; context && count < 30; context = context.next, count++) accept(context.memoizedValue);
    }
    return shared.size === 1 && files.size === 1
      ? { shared: shared.values().next().value, files: files.values().next().value } : null;
  }

  function loaded() {
    return page.performance?.getEntriesByName?.(RUNTIME_URL, 'resource')?.length > 0 ||
      Array.from(page.document.querySelectorAll('link[rel="modulepreload"]')).some(node => node.href === RUNTIME_URL);
  }

  function capture(node, previousAttachment) {
    if (!node?.isConnected || !loaded()) return null;
    const token = page.__elonChatGptDocumentToken, account = identity(), currentRoute = route();
    if (!/^doc_[a-z0-9_]{3,80}$/.test(token || '') || account === null || !currentRoute) return null;
    const context = stores(node), props = context?.shared.getSharedProps();
    const conversation = props?.conversation, controller = props?.composerController;
    if (!conversation || !controller || controller.conversation !== conversation ||
        typeof conversation.serverId$ !== 'function' ||
        (conversation.serverId$() || null) !== currentRoute.conversationId ||
        typeof props.submitComposer !== 'function' || typeof props.isNewThread !== 'boolean' ||
        props.structuredInputHost != null || props.structuredInputMessageId != null ||
        props.isDisabled !== false || props.isComposerSubmissionReady !== true ||
        props.isConsumerLockdownModeLoadingForConversation !== false ||
        typeof props.shouldBlockConsumerLockdownModeActionsForConversation !== 'boolean') return null;
    const pending = context.files.files$(), ready = context.files.readyFiles$();
    if (!Array.isArray(pending) || !Array.isArray(ready) ||
        context.files.hasUploadInProgress$() !== false) return null;
    const attachment = previousAttachment || (pending.length || ready.length
      ? page.__elonChatGptPrivateAttachmentSend?.prepareSubmit?.(context.files) : null);
    if (attachment) {
      if (typeof attachment.current !== 'function' || !attachment.current() ||
          typeof attachment.consumeAccepted !== 'function' || !Array.isArray(attachment.readyFiles) ||
          attachment.readyFiles.length !== 1 || pending.length !== 1 || ready.length !== 1) return null;
    } else if (pending.length || ready.length) return null;
    return { ...context, ...currentRoute, token, account, node, conversation, controller,
      leaf: props.currentLeafId, submit: props.submitComposer, attachment };
  }

  function sameOwner(binding) {
    try {
      const currentRoute = route(), context = stores(binding.node), props = context?.shared.getSharedProps();
      return binding.node.isConnected && page.__elonChatGptDocumentToken === binding.token &&
        identity() === binding.account && currentRoute && context?.shared === binding.shared &&
        currentRoute.temporary === binding.temporary &&
        context.files === binding.files && props?.conversation === binding.conversation &&
        props.composerController === binding.controller &&
        (binding.conversation.serverId$() || null) === currentRoute.conversationId;
    } catch (_) { return false; }
  }

  function current(binding) {
    const next = capture(binding.node, binding.attachment);
    return next && Object.keys(binding).every(key => binding[key] === next[key]);
  }

  function submit(command) {
    if (page.__elonChatGptPrivateTextTransactionsEnabled !== true) return { handled: false, code: 'disabled' };
    if (active || page.__elonChatGptPrivateRegenerateRuntime?.state?.().pending) {
      return { handled: true, completion: Promise.resolve({ status: 'unknown', code: 'busy' }) };
    }
    const value = command?.prompt, expected = command?.expectedDraft;
    if (typeof value !== 'string' || !value.trim() || value.length > 20000 ||
        typeof expected !== 'string' || !/^mcp_[a-z0-9]{1,32}$/.test(command.requestId || '')) return { handled: false, code: 'invalid_command' };
    let binding;
    try {
      binding = capture(command.composer);
      if (!binding || command.requireNativeAttachment === true && !binding.attachment ||
          command.readDraft() !== expected || expected && expected !== value || !current(binding)) {
        return { handled: false, code: 'context_unavailable' };
      }
      command.beforeSubmit?.();
      if (!current(binding) || command.readDraft() !== expected) return { handled: false, code: 'context_changed' };
    } catch (_) { return { handled: false, code: 'context_unavailable' }; }

    const owned = { binding, requestId: command.requestId };
    active = owned;
    let receipt;
    try {
      // Official submitComposer owns readiness, fresh request preparation and React updates.
      const action = binding.attachment
        ? { kind: 'prepared_action', text: value, readyFiles: binding.attachment.readyFiles }
        : { kind: 'text_action', text: value };
      receipt = binding.submit(new page.Event('submit'), action,
        { requireDispatchAcceptance: true });
    } catch (_) {
      return { handled: true, completion: Promise.resolve({ status: 'unknown', code: 'invocation_failed' }) };
    }
    if (receipt?.accepted === false) {
      active = null;
      return { handled: true, completion: Promise.resolve({ status: 'rejected', code: 'not_ready' }) };
    }
    if (receipt?.accepted !== true || typeof receipt.completion?.then !== 'function') {
      // Unknown post-invocation results retain ownership until this document is replaced.
      return { handled: true, completion: Promise.resolve({ status: 'unknown', code: 'invalid_receipt' }) };
    }
    let timer;
    const settled = Promise.resolve(receipt.completion).then(accepted => {
      if (!sameOwner(binding)) return { status: 'unknown', code: 'context_changed' };
      if (accepted !== true) return { status: 'unknown', code: 'dispatch_not_confirmed' };
      // prepared_action does not reset ready files. Never let a failed local
      // cleanup accidentally include the accepted attachment in another send.
      if (binding.attachment) {
        owned.retain = true;
        if (!binding.attachment.consumeAccepted()) return { status: 'unknown', code: 'attachment_cleanup_unconfirmed' };
        owned.retain = false;
      }
      // Explicit actions do not reset the editor. Only clear our unchanged draft.
      if (expected && command.readDraft() === expected) command.clearDraft?.();
      return { status: 'accepted', code: 'accepted' };
    }).catch(() => ({ status: 'unknown', code: 'completion_failed' })).finally(() => {
      page.clearTimeout(timer);
      if (active === owned && !owned.retain) active = null;
    });
    const completion = Promise.race([settled, new Promise(resolve => {
      timer = page.setTimeout(() => resolve({ status: 'unknown', code: 'timeout' }), options.timeoutMs || 15000);
    })]);
    return { handled: true, completion };
  }

  return Object.freeze({ version: 3, submit, state: () => ({ pending: active !== null }) });
});
