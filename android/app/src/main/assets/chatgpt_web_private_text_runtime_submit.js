(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 10, create: factory });
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
  const ownerPath = page.__elonChatGptCommittedOwnerPath ||
    (typeof module === 'object' && module.exports ? require('./chatgpt_web_committed_owner_path') : null);
  let active = null, captureCode = 'not_observed';

  function unavailable(code) {
    captureCode = code;
    return null;
  }

  function route() {
    const url = new URL(page.location.href);
    const conversationId = CONVERSATION.exec(url.pathname)?.[1] || null;
    if (url.origin !== 'https://chatgpt.com' || url.username || url.password || url.hash ||
        url.search && url.search !== '?temporary-chat=true' ||
        url.pathname.startsWith('/g/') && url.search ||
        url.pathname !== '/' && !conversationId && !PROJECT.test(url.pathname)) return null;
    return { href: url.href, conversationId, temporary: url.search === '?temporary-chat=true' };
  }

  function identity(allowGuest = false) {
    const headers = page.__elonChatGptPrivateTransport?.copySameOriginRequestHeaders?.();
    if (!headers || typeof headers !== 'object') {
      if (!allowGuest) return null;
      const bindings = page.__elonChatGptPrivateRuntimeBindings;
      if (typeof bindings?.peek !== 'function' || !bindings.observed('shared')) return null;
      const shared = bindings.peek('shared');
      if (!shared) {
        // Warm the shared cache without holding or replaying a user command.
        bindings.load('shared').catch(() => {});
        return null;
      }
      // Exact public getters: bootstrap must say logged_out and the live session
      // must still be null. Missing headers alone are not a guest identity.
      return typeof shared.R5 === 'function' && typeof shared.F5 === 'function' &&
        shared.R5()?.authStatus === 'logged_out' && shared.F5() === null ? shared : null;
    }
    const values = {};
    for (const [key, value] of Object.entries(headers)) values[key.toLowerCase()] = value;
    if (typeof values.authorization !== 'string' || !/^Bearer \S+$/i.test(values.authorization)) return null;
    // Kept inside this document. Neither request credentials nor text enter receipts.
    return JSON.stringify(['authorization', 'chatgpt-account-id', 'oai-device-id'].map(key => values[key] || ''));
  }

  function committedAncestors(node) {
    // The current official composer appends its ProseMirror DOM imperatively.
    // Use its nearest React host, never a sibling editor or an uncommitted root.
    for (let host = node, depth = 0; host?.isConnected && depth < 12; host = host.parentElement, depth++) {
      if (host === page.document.body || host === page.document.documentElement) break;
      const key = Object.keys(host).find(name => name.startsWith('__reactFiber$'));
      if (!key) continue;
      return currentOwnerPath(host[key]);
    }
    captureCode = 'react_owner_unavailable';
    return [];
  }

  function currentOwnerPath(start) {
    const result = ownerPath?.resolve(start);
    if (result?.code) captureCode = result.code;
    return result?.ancestors || [];
  }

  function stores(node) {
    const shared = new Set(), files = new Set();
    function accept(value) {
      const store = value?.store || value;
      if (typeof store?.getSharedProps === 'function' && typeof store.subscribeToSharedProps === 'function') shared.add(store);
      if (typeof value?.files$ === 'function' && typeof value.readyFiles$ === 'function' &&
          typeof value.hasUploadInProgress$ === 'function') files.add(value);
    }
    const ancestors = committedAncestors(node);
    if (!ancestors.length) return null;
    for (const fiber of ancestors) {
      accept(fiber.memoizedProps?.value);
      let context = fiber.dependencies?.firstContext;
      for (let count = 0; context && count < 30; context = context.next, count++) accept(context.memoizedValue);
    }
    if (shared.size !== 1) return unavailable(shared.size ? 'shared_store_ambiguous' : 'shared_store_unavailable');
    if (files.size !== 1) return unavailable(files.size ? 'file_store_ambiguous' : 'file_store_unavailable');
    return { shared: shared.values().next().value, files: files.values().next().value };
  }

  function loaded() {
    if (page.__elonChatGptPrivateRuntimeBindings) {
      return page.__elonChatGptPrivateRuntimeBindings.observed(RUNTIME_URL);
    }
    return page.performance?.getEntriesByName?.(RUNTIME_URL, 'resource')?.length > 0 ||
      Array.from(page.document.querySelectorAll('link[rel="modulepreload"]')).some(node => node.href === RUNTIME_URL);
  }

  function captureConversation(node, allowGuest = false) {
    if (!node?.isConnected) return unavailable('composer_detached');
    if (!loaded()) return unavailable('runtime_not_observed');
    const token = page.__elonChatGptDocumentToken, account = identity(allowGuest), currentRoute = route();
    if (!/^doc_[a-z0-9_]{3,80}$/.test(token || '')) return unavailable('document_unavailable');
    if (account === null) return unavailable('identity_unavailable');
    if (!currentRoute) return unavailable('route_unsupported');
    const context = stores(node);
    if (!context) return null;
    const props = context.shared.getSharedProps();
    const conversation = props?.conversation, controller = props?.composerController;
    if (!conversation || !controller || controller.conversation !== conversation) return unavailable('conversation_owner_unavailable');
    if (typeof conversation.serverId$ !== 'function' ||
        (conversation.serverId$() || null) !== currentRoute.conversationId) return unavailable('conversation_route_mismatch');
    if (typeof props.isNewThread !== 'boolean' || props.structuredInputHost != null ||
        props.structuredInputMessageId != null) return unavailable('composer_mode_unsupported');
    return { ...context, ...currentRoute, token, account, node, conversation, controller,
      requestId: props.currentRequestId };
  }

  function capture(node, previousAttachment) {
    const binding = captureConversation(node, true);
    if (!binding) return null;
    const context = binding, props = context.shared.getSharedProps();
    if (typeof props.submitComposer !== 'function') return unavailable('submit_owner_unavailable');
    if (props.isDisabled !== false) return unavailable('composer_disabled');
    if (props.isComposerSubmissionReady !== true) return unavailable('submission_not_ready');
    if (props.isConsumerLockdownModeLoadingForConversation !== false ||
        typeof props.shouldBlockConsumerLockdownModeActionsForConversation !== 'boolean') return unavailable('lockdown_not_ready');
    const pending = context.files.files$(), ready = context.files.readyFiles$();
    if (!Array.isArray(pending) || !Array.isArray(ready)) return unavailable('file_state_unavailable');
    if (context.files.hasUploadInProgress$() !== false) return unavailable('upload_in_progress');
    const attachment = previousAttachment || (pending.length || ready.length
      ? page.__elonChatGptPrivateAttachmentSend?.prepareSubmit?.(context.files) : null);
    if (attachment) {
      if (typeof attachment.current !== 'function' || !attachment.current() ||
          typeof attachment.consumeAccepted !== 'function' || !Array.isArray(attachment.readyFiles) ||
          attachment.readyFiles.length < 1 || attachment.readyFiles.length > 9 ||
          pending.length !== attachment.readyFiles.length || ready.length !== pending.length) return unavailable('attachment_lease_invalid');
    } else if (pending.length || ready.length) return unavailable('attachment_not_owned');
    captureCode = 'ready';
    return { ...binding, leaf: props.currentLeafId, submit: props.submitComposer, attachment };
  }

  function sameOwner(binding) {
    try {
      const currentRoute = route(), context = stores(binding.node), props = context?.shared.getSharedProps();
      return binding.node.isConnected && page.__elonChatGptDocumentToken === binding.token &&
        identity(true) === binding.account && currentRoute && context?.shared === binding.shared &&
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
    if (active || page.__elonChatGptPrivateRegenerateRuntime?.state?.().pending ||
        page.__elonChatGptPrivateStopRuntime?.state?.().pending) {
      return { handled: true, completion: Promise.resolve({ status: 'unknown', code: 'busy' }) };
    }
    const value = command?.prompt, expected = command?.expectedDraft;
    if (typeof value !== 'string' || !value.trim() || value.length > 20000 ||
        typeof expected !== 'string' || !/^mcp_[a-z0-9]{1,32}$/.test(command.requestId || '')) return { handled: false, code: 'invalid_command' };
    let binding;
    try {
      captureCode = 'context_unavailable';
      binding = capture(command.composer);
      if (!binding) return { handled: false, code: captureCode };
      if (command.requireNativeAttachment === true && !binding.attachment) return { handled: false, code: 'attachment_not_owned' };
      if (command.readDraft() !== expected || expected && expected !== value) return { handled: false, code: 'draft_mismatch' };
      if (!current(binding)) return { handled: false, code: 'context_changed' };
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

  try {
    if (page.__elonChatGptPrivateTextTransactionsEnabled === true && route()) identity(true);
  } catch (_) {}
  return Object.freeze({ version: 10, submit, captureConversation, state: () => ({ pending: active !== null }) });
});
