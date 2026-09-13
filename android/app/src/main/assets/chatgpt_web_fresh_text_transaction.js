(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 17, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com' &&
      !(root.__elonChatGptFreshTextTransaction?.version >= api.version) && !root.__elonChatGptFreshTextTransaction?.state?.().pending) {
    root.__elonChatGptFreshTextTransaction?.dispose?.();
    root.__elonChatGptFreshTextTransaction = factory(root);
  }
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options ||= {};
  const context = options.context || page.__elonChatGptFreshTextContext.create(page);
  const regeneration = options.regeneration || page.__elonChatGptFreshRegenerateContext?.create(page, context);
  const requests = (options.requests || page.__elonChatGptFreshTextRequest).create(page);
  const reconciliation = options.reconciliation || page.__elonChatGptFreshTextReconcile.create();
  const stopping = options.stopping || page.__elonChatGptFreshTextStop?.create(page, { reconciliation });
  const recovery = options.recovery || page.__elonChatGptFreshTextRecovery?.create(page, {
    reconciliation, timeoutMs: options.reconcileTimeoutMs || 15000
  });
  const records = (options.receipts || page.__elonChatGptFreshTextReceipts).create();
  let document = page.document, token = page.__elonChatGptDocumentToken, active = null, disposed = false;
  let trial = null, last = null, stoppedParent = null;
  const now = options.now || Date.now;
  const eventTypes = new Set(['delta_encoding', 'message', 'input_message', 'message_stream_complete',
    'stream_handoff', 'resume_conversation_token', 'conversation_async_status', 'server_ste_metadata',
    'stream-message-start', 'stream-message-patch', 'stream-message-done', 'delta', 'other']);
  const knownCodes = new Set(['context_changed', 'context_invalid', 'command_invalid', 'scope_unsupported',
    'runtime_unavailable', 'identity_unavailable', 'context_unavailable', 'attachments_active', 'tools_active',
    'conversation_busy', 'parent_unavailable', 'prepare_unconfirmed', 'security_unavailable', 'security_invalid',
    'login_required', 'draft_changed', 'stream_unavailable', 'preparation_timeout', 'stream_open_timeout',
    'stream_timeout', 'reconciliation_timeout', 'cancelled', 'stream_item_invalid', 'stream_server_error',
    'stream_handoff_unavailable', 'stream_topic_owned', 'stream_topic_timeout', 'stream_item_gap',
    'stream_item_limit', 'stream_queue_limit', 'stream_subscribe_failed', 'stream_owner_changed', 'stream_decode_failed']);

  function boundary() {
    if (document === page.document && token === page.__elonChatGptDocumentToken) return;
    active?.controller.abort();
    active?.stopBoundaryController.abort();
    active = null; trial = null; last = null; stoppedParent = null; records.clear();
    document = page.document; token = page.__elonChatGptDocumentToken;
  }

  function state() {
    boundary();
    if (active?.dispatched && active.finished && !active.stopping && !active.recovering && !active.recoveryCompletion) {
      try { if ((active.stopConfirmed || active.recoveryConfirmed ||
        active.binding.reconciled(active.request.userMessageId, active.stopAcknowledged === true) &&
          active.binding.navigationReady?.() !== false) && retireAttachments(active)) active = null; } catch (_) {}
    }
    return { version: 6, transport: 'fresh_page_http_v1', pending: active !== null,
      operation: active?.operation || '',
      phase: active?.phase || 'idle', dispatched: active?.dispatched === true,
      accepted: active?.accepted === true, code: active?.code || '', recovering: active?.recovering === true };
  }

  function retireAttachments(owner) {
    if (!owner.binding?.attachments || owner.attachmentsRetired) return true;
    if (!owner.accepted && !owner.stopConfirmed && !owner.recoveryConfirmed) return false;
    try { owner.attachmentsRetired = owner.binding.attachments.consumeAccepted() === true; } catch (_) {}
    if (!owner.attachmentsRetired) owner.code = 'attachment_cleanup_pending';
    return owner.attachmentsRetired === true;
  }

  function trialArmed() {
    if (trial && (now() >= trial.expiresAt || context.stamp() !== trial.stamp)) trial = null;
    return trial !== null;
  }

  function trialControl(mode) {
    state();
    let control = 'state';
    if (mode === 'start') {
      const stamp = context.stamp();
      control = disposed ? 'disposed' : active ? 'busy' :
        page.__elonChatGptPrivateTextTransactionsEnabled !== true ? 'disabled' :
          !stamp ? 'identity_unavailable' : 'armed';
      if (control === 'armed' && !trialArmed()) trial = { stamp, expiresAt: now() + 120000 };
    } else if (mode === 'end') {
      trial = null; control = 'ended';
    } else if (mode !== 'state') control = 'invalid_mode';
    const armed = trialArmed();
    const safeCode = value => /^[a-z_]{0,64}$/.test(value || '') ? value || '' : 'unknown';
    return { schema: 'elon.fresh_text_trial.v1', version: 6, control, armed,
      operation: last?.operation || '',
      remaining_ms: armed ? Math.max(0, Math.min(120000, trial.expiresAt - now())) : 0,
      attempts: records.attempts(), pending: active !== null, phase: last?.phase || 'idle',
      code: safeCode(last?.code), dispatched: last?.dispatched === true,
      accepted: last?.accepted === true, reconciled: last?.recoveryConfirmed === true || last?.stopConfirmed === true,
      parent_role: ['user', 'assistant'].includes(last?.binding?.parentRole) ? last.binding.parentRole : 'unknown',
      stream_events: last?.streamEvents || 0, event_types: Array.from(last?.eventTypes || []),
      history: last?.historyCode || 'not_observed' };
  }

  function dispatch(command, operation) {
    if (disposed) return { handled: false, code: 'disposed' };
    state();
    const previous = records.get(command?.requestId);
    if (previous) return previous.operation === operation && previous.prompt === command.prompt && previous.expectedDraft === command.expectedDraft &&
      previous.href === page.location.href && previous.stamp === context.stamp() ? previous.transaction :
      { handled: true, completion: Promise.resolve({ status: 'rejected', code: 'request_id_conflict' }) };
    if (records.retired(command?.requestId)) return { handled: true,
      completion: Promise.resolve({ status: 'rejected', code: 'request_retired' }) };
    // A stopped/uncertain write still owns the ledger even if the trial is disabled.
    if (active) return { handled: true, completion: Promise.resolve({ status: 'unknown', code: 'busy' }) };
    const regenerate = operation === 'regenerate';
    if ((regenerate ? page.__elonChatGptFreshRegenerationEnabled !== true : page.__elonChatGptFreshTextDispatchEnabled === false) && !trialArmed() ||
        page.__elonChatGptPrivateTextTransactionsEnabled !== true) {
      return { handled: false, code: 'disabled' };
    }
    // Cold/unknown identity keeps the accepted sender; no UI-blocking import is
    // started merely to discover whether the independent candidate is eligible.
    const stamp = context.stamp();
    if (!stamp) return { handled: false, code: 'identity_unavailable' };
    if (!/^mcp_[a-z0-9]{1,32}$/.test(command?.requestId || '') || typeof command.prompt !== 'string' ||
        !regenerate && !command.prompt.trim() && page.__elonChatGptFreshTextAttachmentsEnabled !== true && !trialArmed() ||
        command.prompt.length > 20000 || typeof command.expectedDraft !== 'string' ||
        !regenerate && command.expectedDraft && command.expectedDraft !== command.prompt) return { handled: false, code: 'invalid_command' };
    const admission = records.admit(command.requestId);
    if (admission) return { handled: true, completion: Promise.resolve({ status: 'rejected', code: admission }) };
    if (stoppedParent && !stoppedParent.current()) stoppedParent = null;
    const continuation = stoppedParent;
    const allowTools = page.__elonChatGptFreshTextToolsEnabled === true || trialArmed();
    const allowProjects = page.__elonChatGptFreshTextProjectsEnabled === true || trialArmed();
    const allowNewConversations = page.__elonChatGptFreshTextNewConversationsEnabled === true || trialArmed();
    const allowTemporary = page.__elonChatGptFreshTextTemporaryEnabled === true || trialArmed();
    const allowAttachments = page.__elonChatGptFreshTextAttachmentsEnabled === true || trialArmed();
    trial = null;
    let resolve;
    const owner = { operation, token, document, stamp, controller: new page.AbortController(), phase: 'preparing',
      streamEvents: 0, eventTypes: new Set(), historyCode: 'not_observed',
      stopBoundaryController: new page.AbortController(),
      dispatched: false, accepted: false, finished: false, settled: false, fallback: false };
    owner.stopBoundary = owner.stopBoundaryController.signal;
    owner.stopCurrent = () => active === owner && owner.document === page.document &&
      owner.token === page.__elonChatGptDocumentToken && owner.binding?.owns() === true;
    owner.stopReading = () => owner.controller.abort();
    owner.notify = () => { try { command.onSettled?.(); } catch (_) {} };
    const completion = new Promise(done => { resolve = done; });
    function receipt(value) { if (!owner.settled) { owner.settled = true; resolve(value); } }
    const transaction = Object.freeze({ handled: true, completion,
      claimFallback() {
        if (!owner.fallback || owner.fallbackClaimed || owner.dispatched || active && active !== owner ||
            owner.document !== page.document || owner.token !== page.__elonChatGptDocumentToken ||
            context.stamp() !== owner.stamp) return false;
        if (command.readDraft() !== command.expectedDraft) return false;
        owner.fallbackClaimed = true;
        return true;
      }
    });
    records.set(command.requestId, { operation, prompt: command.prompt, expectedDraft: command.expectedDraft,
      href: page.location.href, stamp, transaction,
      retirable: () => owner.finished && !owner.stopping && !owner.recovering && !owner.recoveryCompletion &&
        (!owner.dispatched || (owner.stopConfirmed === true || owner.recoveryConfirmed === true) && retireAttachments(owner)) });
    active = owner;
    last = owner;
    function current() {
      return active === owner && !owner.controller.signal.aborted && owner.document === page.document &&
        owner.token === page.__elonChatGptDocumentToken && owner.binding?.current() === true &&
        command.readDraft() === command.expectedDraft;
    }
    function check() { if (!current()) throw Error('context_changed'); }
    let deadline;
    function timeout(ms, code) {
      page.clearTimeout(deadline);
      deadline = page.setTimeout(() => {
        owner.code = code;
        owner.controller.abort();
      }, ms);
    }
    async function abortable(promise) {
      const signal = owner.controller.signal;
      if (signal.aborted) throw Error(owner.code || 'cancelled');
      let listener;
      try {
        return await Promise.race([promise, new Promise((_, reject) => {
          listener = () => reject(Error(owner.code || 'cancelled'));
          signal.addEventListener('abort', listener, { once: true });
        })]);
      } finally { signal.removeEventListener('abort', listener); }
    }

    async function run() {
      timeout(options.prepareTimeoutMs || 15000, 'preparation_timeout');
      if (regenerate && !regeneration) throw Error('runtime_unavailable');
      owner.binding = await abortable(regenerate ? regeneration.capture(command) :
        context.capture(command.composer, continuation,
          { allowTools, allowProjects, allowNewConversations, allowTemporary, allowAttachments }));
      check();
      owner.request = requests.create(owner.binding, command);
      const { shared, runtime } = owner.binding;
      if (typeof shared.textTopic !== 'function') throw Error('runtime_unavailable');
      // This is a new preparation request for this exact parent/model/command,
      // not a template captured from a previous website send.
      const prepared = await abortable(shared.textApi.safePost('/f/conversation/prepare', {
        requestBody: owner.request.preparationBody(), signal: owner.controller.signal,
        additionalHeaders: owner.request.preparationHeaders(),
        disableAutomaticRetry: true
      }));
      check();
      const security = await abortable(Promise.resolve(runtime.textSecurity(owner.request.securityMetadata())));
      check();
      const request = owner.request.consume(prepared, security, shared.textSecurityHeaders, current);
      const stream = page.__elonChatGptPrivateStreamTransport;
      if (typeof stream?.[regenerate ? 'preparePrivateRegeneration' : 'preparePrivateSend'] !== 'function' ||
          typeof stream.beginPrivateStream !== 'function' ||
          !page.__elonChatGptFreshTextStream) throw Error('stream_unavailable');
      timeout(options.openTimeoutMs || 15000, 'stream_open_timeout');
      // DM, not its retrying MGt wrapper, retains official auth and response
      // integrity. The native command owns body construction and dispatch.
      const source = runtime.textStream('https://chatgpt.com/backend-api/f/conversation', {
        method: 'POST', headers: request.headers, body: request.body,
        targetBaseUrl: 'https://chatgpt.com/backend-api', routeName: '/f/conversation',
        signal: owner.controller.signal, initialOpenTimeoutMs: 15000, idleTimeoutMs: 45000,
        onBeforeRequestStart() {
          check();
          if (owner.dispatched) throw Error('preparation_consumed');
          owner.binding.beforeDispatch?.();
          // The provider invokes fetch immediately after this hook. Crossing
          // this boundary is uncertain even if fetch subsequently throws.
          owner.dispatched = true; owner.phase = 'dispatching';
          stoppedParent = null;
          const ready = regenerate ? stream.preparePrivateRegeneration(owner.request.userMessageId) :
            stream.preparePrivateSend(command.prompt, owner.request.userMessageId,
              !!owner.binding.attachments && !command.prompt.trim());
          if (!ready) throw Error('stream_unavailable');
          owner.sink = stream.beginPrivateStream({ conversationId: owner.binding.conversationId,
            userMessageId: owner.request.userMessageId, current: owner.stopCurrent,
            observePayload: owner.binding.observePayload,
            adoptConversation: owner.binding.newConversation ? (id, payload) =>
              owner.binding.adoptConversation(id, payload, owner.request.userMessageId) : undefined });
          if (!owner.sink) throw Error('stream_unavailable');
          command.onDispatch?.();
          return {};
        }
      });
      const rootIterator = source?.[Symbol.asyncIterator]?.();
      if (!rootIterator) throw Error('runtime_unavailable');
      const iterator = page.__elonChatGptFreshTextStream.create({
        getTopic: shared.textTopic, current: owner.stopCurrent, signal: owner.controller.signal,
        setTimeout: page.setTimeout.bind(page), clearTimeout: page.clearTimeout.bind(page)
      }).follow(rootIterator);
      owner.iterator = iterator;
      for (;;) {
        const next = await abortable(iterator.next());
        if (next.done) break;
        if (next.value && 'data' in next.value) {
          owner.streamEvents = Math.min(65535, owner.streamEvents + 1);
          const data = next.value.data;
          const type = next.value.event === 'delta_encoding' ? 'delta_encoding' :
            data?.type || (data?.message ? 'message' : data?.o ? 'delta' : 'other');
          owner.eventTypes.add(eventTypes.has(type) ? type : 'other');
          owner.sink.push(next.value);
        }
        if (next.value?.response) {
          if (!owner.dispatched) throw Error('runtime_unavailable');
          if (next.value.response.ok !== true ||
              !next.value.response.headers?.get('content-type')?.includes('text/event-stream')) throw Error('stream_unavailable');
          owner.accepted = true; owner.phase = 'streaming';
          retireAttachments(owner);
          timeout(options.streamTimeoutMs || 600000, 'stream_timeout');
          // Draft cleanup must never delay or change acceptance of the write.
          try { if (!regenerate && owner.binding.owns() && command.readDraft() === command.expectedDraft && command.expectedDraft) command.clearDraft?.(); } catch (_) {}
          receipt({ status: 'accepted', code: 'accepted', current: owner.binding.owns() });
        }
      }
      if (!owner.accepted) throw Error('stream_unavailable');
      owner.sink.finish();
      owner.phase = 'reconciling'; owner.code = 'history_reconciliation_required';
    }
    void run().catch(error => {
      owner.code ||= knownCodes.has(error?.message) ? error.message : 'request_failed';
      owner.phase = owner.dispatched ? 'uncertain' : 'rejected';
      owner.controller.abort();
      // Only pre-preparation runtime/context absence may offer the accepted
      // sender. No auth refusal, timeout or post-dispatch failure is replayed.
      owner.fallback = !owner.binding && !owner.dispatched &&
        ['runtime_unavailable', 'context_unavailable', 'scope_unsupported',
          'attachments_active', 'tools_active', 'parent_unavailable', 'identity_unavailable'].includes(owner.code);
      receipt({ status: owner.dispatched ? 'unknown' : owner.fallback ? 'unavailable' : 'rejected', code: owner.code });
    }).finally(() => {
      page.clearTimeout(deadline); owner.finished = true;
      if (owner.controller.signal.aborted) {
        // An iterator may have an outstanding next(); never await its cleanup.
        try { Promise.resolve(owner.iterator?.return?.()).catch(() => {}); } catch (_) {}
      }
      if (!owner.dispatched && active === owner) active = null;
      owner.notify();
      if (active === owner && owner.dispatched && !owner.stopping) recover(true);
    });
    return transaction;
  }

  function cancel() {
    state();
    if (!active || active.finished) return false;
    active.code = 'cancelled'; active.controller.abort();
    // This cancels our HTTP reader; it does not falsely confirm server stop.
    return true;
  }
  function stop() {
    state();
    const owner = active;
    if (!owner) return { handled: false };
    if (!owner.dispatched) {
      cancel();
      return { handled: true, completion: Promise.resolve({ status: 'accepted', code: 'cancelled_before_dispatch' }) };
    }
    if (!stopping) return { handled: true, completion: Promise.resolve({ status: 'unknown', code: 'server_stop_unconfirmed' }) };
    owner.recoveryController?.abort();
    owner.phase = 'stopping';
    const completion = stopping.stop(owner).then(receipt => {
      if ((active === owner || active === null) && owner.document === page.document &&
          owner.token === page.__elonChatGptDocumentToken && owner.binding.owns()) {
        owner.phase = receipt.status === 'accepted' ? 'completed' : 'uncertain';
        owner.code = receipt.status === 'accepted' ? '' : receipt.code;
        if (owner.operation === 'send' && owner.stopConfirmed === true && owner.stopAcknowledged === true) {
          const binding = owner.binding, id = owner.request.userMessageId;
          stoppedParent = { id, current() {
            try { return binding.reconciled(id, true, true) === true; } catch (_) { return false; }
          } };
        }
      }
      return receipt;
    });
    return { handled: true, completion };
  }
  function recover(automatic = false) {
    state();
    const owner = active;
    if (!owner || !recovery || disposed) return { handled: false };
    if (owner.recoveryCompletion) return { handled: true, completion: owner.recoveryCompletion };
    const completion = recovery.recover(owner, automatic).then(receipt => {
      if (active !== owner || owner.stopping || owner.stopConfirmed || !owner.stopCurrent()) return receipt;
      if (receipt.status === 'accepted') {
        owner.phase = 'completed'; owner.code = ''; owner.accepted = true;
        page.__elonChatGptPrivateStreamTransport?.finishPrivateSend?.();
      } else if (!['recovery_not_ready', 'recovery_deferred', 'recovery_cancelled'].includes(receipt.code)) {
        owner.code = receipt.code;
      }
      owner.notify();
      return receipt;
    }).catch(() => ({ status: 'unknown', code: 'history_unavailable' }))
      .finally(() => { owner.recoveryCompletion = null; });
    owner.recoveryCompletion = completion;
    return { handled: true, completion };
  }
  const visibility = () => { if (page.document.visibilityState !== 'hidden') recover(true); };
  const resume = () => recover(true);
  const eventDocument = page.document;
  eventDocument.addEventListener?.('visibilitychange', visibility);
  page.addEventListener?.('online', resume);
  page.addEventListener?.('pageshow', resume);
  function dispose() {
    if (active) return false;
    disposed = true;
    eventDocument.removeEventListener?.('visibilitychange', visibility);
    page.removeEventListener?.('online', resume);
    page.removeEventListener?.('pageshow', resume);
    return true;
  }
  const hasCurrentWriter = () => !!active?.dispatched && !active.stopConfirmed &&
    !active.recoveryConfirmed && active.stopCurrent();
  return Object.freeze({ version: 17, send: command => dispatch(command, 'send'),
    regenerate: command => dispatch({ ...command, prompt: '' }, 'regenerate'),
    state, cancel, stop, recover, dispose, trialControl, hasCurrentWriter });
});
