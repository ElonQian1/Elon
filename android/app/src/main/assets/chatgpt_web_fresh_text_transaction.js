(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 2, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com' &&
      !(root.__elonChatGptFreshTextTransaction?.version >= api.version) && !root.__elonChatGptFreshTextTransaction?.state?.().pending) {
    root.__elonChatGptFreshTextTransaction = factory(root);
  }
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options ||= {};
  const context = options.context || page.__elonChatGptFreshTextContext.create(page);
  const requests = (options.requests || page.__elonChatGptFreshTextRequest).create(page);
  const reconciliation = options.reconciliation || page.__elonChatGptFreshTextReconcile.create();
  const stopping = options.stopping || page.__elonChatGptFreshTextStop?.create(page, { reconciliation });
  const records = new Map();
  let document = page.document, token = page.__elonChatGptDocumentToken, active = null;
  const knownCodes = new Set(['context_changed', 'context_invalid', 'command_invalid', 'scope_unsupported',
    'runtime_unavailable', 'identity_unavailable', 'context_unavailable', 'attachments_active', 'tools_active',
    'conversation_busy', 'parent_unavailable', 'prepare_unconfirmed', 'security_unavailable', 'security_invalid',
    'login_required', 'draft_changed', 'stream_unavailable', 'preparation_timeout', 'stream_open_timeout',
    'stream_timeout', 'reconciliation_timeout', 'cancelled']);

  function boundary() {
    if (document === page.document && token === page.__elonChatGptDocumentToken) return;
    active?.controller.abort();
    active?.stopBoundaryController.abort();
    active = null; records.clear();
    document = page.document; token = page.__elonChatGptDocumentToken;
  }

  function state() {
    boundary();
    if (active?.dispatched && active.finished && !active.stopping) {
      try { if (active.stopConfirmed || active.binding.reconciled(active.request.userMessageId, active.stopAcknowledged === true)) active = null; } catch (_) {}
    }
    return { version: 2, transport: 'fresh_page_http_v1', pending: active !== null,
      phase: active?.phase || 'idle', dispatched: active?.dispatched === true,
      accepted: active?.accepted === true, code: active?.code || '' };
  }

  function send(command) {
    state();
    const previous = records.get(command?.requestId);
    if (previous) return previous.prompt === command.prompt && previous.expectedDraft === command.expectedDraft &&
      previous.href === page.location.href && previous.stamp === context.stamp() ? previous.transaction :
      { handled: true, completion: Promise.resolve({ status: 'rejected', code: 'request_id_conflict' }) };
    // A stopped/uncertain write still owns the ledger even if the trial is disabled.
    if (active) return { handled: true, completion: Promise.resolve({ status: 'unknown', code: 'busy' }) };
    if (page.__elonChatGptFreshTextDispatchEnabled !== true || page.__elonChatGptPrivateTextTransactionsEnabled !== true) {
      return { handled: false, code: 'disabled' };
    }
    // Cold/unknown identity keeps the accepted sender; no UI-blocking import is
    // started merely to discover whether the independent candidate is eligible.
    const stamp = context.stamp();
    if (!stamp) return { handled: false, code: 'identity_unavailable' };
    if (!/^mcp_[a-z0-9]{1,32}$/.test(command?.requestId || '') || typeof command.prompt !== 'string' ||
        !command.prompt.trim() || command.prompt.length > 20000 || typeof command.expectedDraft !== 'string' ||
        command.expectedDraft && command.expectedDraft !== command.prompt) return { handled: false, code: 'invalid_command' };
    // No eviction of write receipts within this document: exceeding this trial
    // budget requires a fresh document, not permission to reuse an old command ID.
    if (records.size >= 32) return { handled: false, code: 'trial_budget' };
    let resolve;
    const owner = { token, document, stamp, controller: new page.AbortController(), phase: 'preparing',
      stopBoundaryController: new page.AbortController(),
      dispatched: false, accepted: false, finished: false, settled: false, fallback: false };
    owner.stopBoundary = owner.stopBoundaryController.signal;
    owner.stopCurrent = () => active === owner && owner.document === page.document &&
      owner.token === page.__elonChatGptDocumentToken && owner.binding?.owns() === true;
    owner.stopReading = () => owner.controller.abort();
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
    records.set(command.requestId, { prompt: command.prompt, expectedDraft: command.expectedDraft,
      href: page.location.href, stamp, transaction });
    active = owner;
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
      owner.binding = await abortable(context.capture(command.composer));
      check();
      owner.request = requests.create(owner.binding, command);
      const { shared, runtime } = owner.binding;
      // This is a new preparation request for this exact parent/model/command,
      // not a template captured from a previous website send.
      const prepared = await abortable(shared.textApi.safePost('/f/conversation/prepare', {
        requestBody: owner.request.preparationBody(), signal: owner.controller.signal,
        disableAutomaticRetry: true
      }));
      check();
      const security = await abortable(Promise.resolve(runtime.textSecurity({ systemHints: [] })));
      check();
      const request = owner.request.consume(prepared, security, shared.textSecurityHeaders, current);
      const stream = page.__elonChatGptPrivateStreamTransport;
      if (typeof stream?.preparePrivateSend !== 'function') throw Error('stream_unavailable');
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
          // The provider invokes fetch immediately after this hook. Crossing
          // this boundary is uncertain even if fetch subsequently throws.
          owner.dispatched = true; owner.phase = 'dispatching';
          if (!stream.preparePrivateSend(command.prompt, owner.request.userMessageId)) throw Error('stream_unavailable');
          command.onDispatch?.();
          return {};
        }
      });
      const iterator = source?.[Symbol.asyncIterator]?.();
      if (!iterator) throw Error('runtime_unavailable');
      owner.iterator = iterator;
      for (;;) {
        const next = await abortable(iterator.next());
        if (next.done) break;
        if (next.value?.response) {
          if (!owner.dispatched) throw Error('runtime_unavailable');
          if (next.value.response.ok !== true ||
              !next.value.response.headers?.get('content-type')?.includes('text/event-stream')) throw Error('stream_unavailable');
          owner.accepted = true; owner.phase = 'streaming';
          timeout(options.streamTimeoutMs || 600000, 'stream_timeout');
          // Draft cleanup must never delay or change acceptance of the write.
          try { if (owner.binding.owns() && command.readDraft() === command.expectedDraft && command.expectedDraft) command.clearDraft?.(); } catch (_) {}
          receipt({ status: 'accepted', code: 'accepted', current: owner.binding.owns() });
        }
      }
      if (!owner.accepted) throw Error('stream_unavailable');
      owner.phase = 'reconciling'; owner.code = 'history_reconciliation_required';
      // Iterator completion is not evidence that the official selected branch
      // has incorporated this turn. Keep the write barrier until it has.
      timeout(options.reconcileTimeoutMs || 15000, 'reconciliation_timeout');
      if (await abortable(reconciliation.reconcile(owner.binding, owner.request, owner.controller.signal))) {
        owner.phase = 'completed'; owner.code = '';
      }
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
      try { command.onSettled?.(); } catch (_) {}
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
    owner.phase = 'stopping';
    const completion = stopping.stop(owner).then(receipt => {
      if (active === owner) {
        owner.phase = receipt.status === 'accepted' ? 'completed' : 'uncertain';
        owner.code = receipt.status === 'accepted' ? '' : receipt.code;
      }
      return receipt;
    });
    return { handled: true, completion };
  }
  return Object.freeze({ version: 2, send, state, cancel, stop });
});
