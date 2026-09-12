(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 20, create: factory });
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

  function guestIdentity(warm = false) {
    const bindings = page.__elonChatGptPrivateRuntimeBindings;
    if (typeof bindings?.peek !== 'function' || !bindings.observed('shared')) return null;
    const shared = bindings.peek('shared');
    if (!shared) {
      // Warm the shared cache without holding or replaying a user command.
      if (warm) bindings.load('shared').catch(() => {});
      return null;
    }
    // Request credentials can exist on the logged-out homepage as well.
    // Only the official bootstrap and live session establish guest mode.
    return typeof shared.R5 === 'function' && typeof shared.F5 === 'function' &&
      shared.R5()?.authStatus === 'logged_out' && shared.F5() === null ? shared : null;
  }

  function identity(allowGuest = false) {
    const headers = page.__elonChatGptPrivateTransport?.copySameOriginRequestHeaders?.();
    if (!headers || typeof headers !== 'object') return allowGuest ? guestIdentity(true) : null;
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
    const guestProof = allowGuest ? guestIdentity() : null;
    if (!matchesRoute(conversation, currentRoute, guestProof)) return unavailable('conversation_route_mismatch');
    if (typeof props.isNewThread !== 'boolean') return unavailable('composer_mode_unsupported');
    if (props.structuredInputMessageId != null) return unavailable('structured_input_active');
    // The official composer always creates this capability host, even with no
    // active structured input. Its existence is not an active-mode signal.
    const host = props.structuredInputHost;
    if (host != null && (typeof host !== 'object' ||
        Object.keys(host).sort().join(',') !== 'canOpen$,tryOpen$' ||
        typeof host.canOpen$ !== 'function' || typeof host.tryOpen$ !== 'function' ||
        props.structuredInputMessageId !== null)) return unavailable('structured_host_unrecognized');
    return { ...context, ...currentRoute, token, account, guestProof, node, conversation, controller,
      serverId: conversation.serverId$() || null, requestId: props.currentRequestId,
      structuredHost: host, newThread: props.isNewThread };
  }

  function matchesRoute(conversation, currentRoute, guestProof) {
    if (typeof conversation.serverId$ !== 'function') return false;
    const serverId = conversation.serverId$() || null;
    if (serverId !== null && (typeof serverId !== 'string' || !new RegExp('^' + UUID + '$', 'i').test(serverId))) return false;
    if (serverId === currentRoute.conversationId) return true;
    if (currentRoute.href === 'https://chatgpt.com/?temporary-chat=true' && serverId !== null) {
      const bindings = page.__elonChatGptPrivateRuntimeBindings;
      const shared = bindings?.observed('shared') ? bindings.peek('shared') : null;
      if (!shared) {
        bindings?.load('shared').catch(() => {});
        return false;
      }
      // Temporary threads can acquire a server ID without a /c/ navigation.
      // The URL alone is insufficient: require the committed official control.
      const thread = typeof conversation.id === 'string' && shared.XM?.(conversation.id);
      return !!thread && page.__elonChatGptTemporaryChat?.ownsSelectedConversation?.(conversation) === true && shared.cX?.() === true &&
        shared.HM?.getIsNewConversation?.(thread) === false && shared.uo?.(conversation) === false &&
        typeof shared.HM?.getGizmoId === 'function' && shared.HM.getGizmoId(thread) == null;
    }
    // The confirmed logged-out homepage can keep the same official conversation
    // after it receives a server ID, without navigating to /c/<id>.
    return guestProof !== null &&
      currentRoute.href === 'https://chatgpt.com/' && currentRoute.conversationId === null;
  }

  function draftEditor(binding) {
    const bindings = page.__elonChatGptPrivateRuntimeBindings;
    if (!bindings?.observed('composer')) return null;
    const runtime = bindings.peek('composer');
    if (!runtime) { bindings.load('composer').catch(() => {}); return null; }
    if (typeof runtime.t_ !== 'function' || typeof runtime.AS !== 'function' || typeof runtime.VS !== 'function') return null;
    const view = runtime.t_(binding.controller);
    if (view?.dom !== binding.node || view.isDestroyed ||
        typeof view.state?.doc?.toJSON !== 'function') return null;
    const doc = view.state.doc.toJSON();
    if (doc.type !== 'doc' || !Array.isArray(doc.content) || doc.content.length > 1000 ||
        !doc.content.every(p => p.type === 'paragraph' && !p.marks?.length &&
          (!p.content || p.content.every(t => t.type === 'text' && !t.marks?.length)))) return null;
    return { view, read: runtime.AS, replace: runtime.VS };
  }

  function capture(node, previousAttachment, draftMode = false) {
    const binding = captureConversation(node, true);
    if (!binding) return null;
    const context = binding, props = context.shared.getSharedProps();
    if (typeof props.submitComposer !== 'function') return unavailable('submit_owner_unavailable');
    if (props.isDisabled !== false) return unavailable('composer_disabled');
    if (typeof props.isComposerSubmissionReady !== 'boolean') return unavailable('submission_not_ready');
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
    const useDraft = draftMode || !props.isComposerSubmissionReady;
    const editor = useDraft && !attachment ? draftEditor(binding) : null;
    if (useDraft && (!editor || attachment)) return unavailable('submission_not_ready');
    captureCode = 'ready';
    return { ...binding, leaf: props.currentLeafId, submit: props.submitComposer, attachment,
      draftView: editor?.view || null, readEditor: editor?.read || null, replaceEditor: editor?.replace || null };
  }

  function sameOwner(binding, acknowledged = false) {
    try {
      const currentRoute = route(), context = stores(binding.node), props = context?.shared.getSharedProps();
      // A confirmed first dispatch can publish its server ID before the router
      // commits /c/<id>. This is only the captured ordinary homepage owner,
      // never permission to capture another command during that transition.
      const homeHandoff = acknowledged && binding.newThread && !binding.temporary && binding.serverId === null &&
        binding.href === 'https://chatgpt.com/' && currentRoute?.href === binding.href &&
        new RegExp('^' + UUID + '$', 'i').test(binding.conversation.serverId$() || '');
      // A committed provider wrapper can be replaced after first dispatch;
      // the actual conversation, controller and file store must still be ours.
      return binding.node.isConnected && page.__elonChatGptDocumentToken === binding.token &&
        identity(true) === binding.account && !!currentRoute && !!context &&
        (!binding.guestProof || guestIdentity() === binding.guestProof) &&
        currentRoute.temporary === binding.temporary &&
        context.files === binding.files && props?.conversation === binding.conversation &&
        props.composerController === binding.controller &&
        (!binding.serverId || binding.conversation.serverId$() === binding.serverId) &&
        (matchesRoute(binding.conversation, currentRoute, binding.guestProof) || homeHandoff);
    } catch (_) { return false; }
  }

  function current(binding) {
    const next = capture(binding.node, binding.attachment, !!binding.draftView);
    return next && Object.keys(binding).every(key => binding[key] === next[key]);
  }

  function retireAcceptedFiles(owned) {
    try { owned.retain = !owned.binding.attachment.consumeAccepted(); } catch (_) { owned.retain = true; }
    return !owned.retain;
  }

  function submit(command) {
    if (page.__elonChatGptPrivateTextTransactionsEnabled !== true) return { handled: false, code: 'disabled' };
    // Retry only local retirement on a later user action, never the network send.
    if (active?.retain && retireAcceptedFiles(active)) active = null;
    if (active || page.__elonChatGptCanvasDocumentActions?.generationPending?.() ||
        page.__elonChatGptPrivateRegenerateRuntime?.state?.().pending ||
        page.__elonChatGptPrivateStopRuntime?.state?.().pending) {
      return { handled: true, completion: Promise.resolve({ status: 'unknown', code: 'busy' }) };
    }
    const value = command?.prompt, expected = command?.expectedDraft;
    if (typeof value !== 'string' || !value.trim() || value.length > 20000 ||
        typeof expected !== 'string' || !/^mcp_[a-z0-9]{1,32}$/.test(command.requestId || '')) return { handled: false, code: 'invalid_command' };
    let binding, draftMutationAttempted = false;
    try {
      captureCode = 'context_unavailable';
      binding = capture(command.composer);
      if (!binding) return { handled: false, code: captureCode };
      if (command.requireNativeAttachment === true && !binding.attachment) return { handled: false, code: 'attachment_not_owned' };
      if (command.readDraft() !== expected || expected && expected !== value) return { handled: false, code: 'draft_mismatch' };
      if (!current(binding)) return { handled: false, code: 'context_changed' };
      command.beforeSubmit?.();
      if (!current(binding) || command.readDraft() !== expected) return { handled: false, code: 'context_changed' };
      if (binding.draftView) {
        if (binding.readEditor(binding.draftView.state.doc)?.content !== expected) return { handled: false, code: 'draft_mismatch' };
        // Use the official editor transaction, without focusing it or waiting for
        // a DOM button. submitComposer still owns all current-draft constraints.
        if (expected !== value) {
          draftMutationAttempted = true;
          binding.replaceEditor(binding.draftView, value, { scrollIntoView: false });
        }
        if (!current(binding) || binding.readEditor(binding.draftView.state.doc)?.content !== value || command.readDraft() !== value) {
          return { handled: true, completion: Promise.resolve({ status: 'rejected', code: 'draft_handoff' }) };
        }
      }
    } catch (_) {
      return draftMutationAttempted
        ? { handled: true, completion: Promise.resolve({ status: 'rejected', code: 'draft_handoff' }) }
        : { handled: false, code: 'context_unavailable' };
    }

    const owned = { binding, requestId: command.requestId };
    active = owned;
    let receipt;
    try {
      // Official submitComposer owns readiness, fresh request preparation and React updates.
      const action = binding.attachment
        ? { kind: 'prepared_action', text: value, readyFiles: binding.attachment.readyFiles }
        : binding.draftView ? { kind: 'current_draft' } : { kind: 'text_action', text: value };
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
      const currentContext = sameOwner(binding, accepted === true);
      if (accepted !== true) return { status: 'unknown', code: binding.attachment ? 'dispatch_not_confirmed'
        : accepted === false ? 'dispatch_unconfirmed_false'
        : accepted == null ? 'dispatch_unconfirmed_void' : 'dispatch_unconfirmed_shape' };
      // ACK belongs to the captured dispatch, not the current editor. Retire
      // its exact files independently; failed local cleanup blocks replay only.
      if (binding.attachment) {
        owned.retain = true;
        retireAcceptedFiles(owned);
      }
      if (currentContext && !binding.draftView && expected) {
        try { if (command.readDraft() === expected) command.clearDraft?.(); } catch (_) {}
      }
      return { status: 'accepted', code: 'accepted', current: currentContext,
        ...(binding.attachment ? { cleanup: owned.retain ? 'pending' : 'completed' } : {}) };
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
    if (page.__elonChatGptPrivateTextTransactionsEnabled === true && route()) {
      identity(true);
      if (route().href === 'https://chatgpt.com/') guestIdentity(true);
      const bindings = page.__elonChatGptPrivateRuntimeBindings;
      if (bindings?.observed('composer')) bindings.load('composer').catch(() => {});
    }
  } catch (_) {}
  return Object.freeze({ version: 20, submit, captureConversation, state: () => ({ pending: active !== null }) });
});
