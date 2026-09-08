(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 7, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') {
    const old = root.__elonChatGptPrivateStopRuntime;
    if (!(Number(old?.version) >= api.version) && !old?.state?.().pending) {
      root.__elonChatGptPrivateStopRuntime = api.create(root);
    }
  }
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options = options || {};
  const URLS = Object.freeze({
    shared: 'https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js',
    conversation: 'https://chatgpt.com/cdn/assets/conversation-small-hiw4wce20lu6te81.js'
  });
  let modules, loading, active = null, code = 'not_observed';
  let diagnosticToken, lastDiagnostic;
  const validId = value => typeof value === 'string' && /^[a-z0-9_-]{1,128}$/i.test(value);
  // Official request IDs interpolate an opaque client-thread key and counter;
  // the thread component is not restricted to message/server ID characters.
  const validRequestId = value => validId(value) || typeof value === 'string' && value.length <= 512 &&
    !/[\u0000-\u001f\u007f]/.test(value) && /^request-.+-[0-9]{1,16}$/.test(value);
  const emptyDiagnostic = () => ({ cached: false, request: 'missing', tree: false,
    generation: false, mode: 'unknown' });

  function diagnostics() {
    return { ...(diagnosticToken === page.__elonChatGptDocumentToken && lastDiagnostic || emptyDiagnostic()) };
  }

  function generation(s, binding, tree, status) {
    if (!tree || tree.interruptionInProgress === true || !binding.serverId || typeof s.HM.getCurrentLeafId !== 'function' ||
        typeof s.HM.getConversationLastTurn !== 'function') return null;
    const leaf = s.HM.getCurrentLeafId(tree), turn = s.HM.getConversationLastTurn(tree)?.id;
    return validId(leaf) && validId(turn) ? { leaf, turn, status } : null;
  }

  function decline(reason) {
    code = reason;
    return { handled: false };
  }

  function capture(node, report = false) {
    if (report) { diagnosticToken = page.__elonChatGptDocumentToken; lastDiagnostic = emptyDiagnostic(); }
    const binding = page.__elonChatGptPrivateTextRuntimeSubmit?.captureConversation?.(node, true);
    if (!binding) return null;
    const shared = page.__elonChatGptPrivateRuntimeBindings?.peek?.('shared');
    if (typeof shared?.XM !== 'function' || typeof shared.HM?.getRequestId !== 'function') return binding;
    // The composer property is a render snapshot. Pin the live official tree
    // synchronously, before loading or subscribing can expose a later request.
    const tree = shared.XM(binding.conversation.id), requestId = shared.HM.getRequestId(tree);
    const status = shared.Fx?.(binding.conversation);
    const scope = requestId == null ? generation(shared, binding, tree, status) : null;
    if (report) lastDiagnostic = { cached: true, request: requestId == null ? 'missing' :
      validRequestId(requestId) ? 'valid' : 'invalid', tree: !!tree, generation: !!scope,
      mode: typeof shared.Fx !== 'function' ? 'unknown' : status == null ? 'idle' :
        ({ 3: 'streaming', 4: 'unread', 5: 'voice', 6: 'voice', 7: 'voice' })[status.value] || 'unknown' };
    return { ...binding, requestId, generation: scope };
  }

  function current(binding) {
    const now = capture(binding.node);
    return now && ['token', 'account', 'guestProof', 'href', 'shared', 'files', 'conversation', 'controller'].every(key =>
      now[key] === binding[key]) ? now : null;
  }

  function sameGeneration(expected, actual, preparing = false) {
    // Before dispatch the signal identity is fixed; only terminal cleanup may
    // replace it afterward without changing the captured turn and leaf.
    return !!actual && expected.leaf === actual.leaf && expected.turn === actual.turn &&
      ((!preparing && (actual.status == null || actual.status.value === 4)) || expected.status === actual.status);
  }

  function currentOwner(binding) {
    const value = current(binding);
    return !!value && (binding.generation ? value.requestId == null &&
      sameGeneration(binding.generation, value.generation, true) : value.requestId === binding.requestId);
  }

  function validate(value) {
    const s = value?.shared;
    return typeof value?.conversation?.FVt === 'function' && typeof s?.XM === 'function' &&
      typeof s.HM?.getRequestId === 'function' && typeof s.Fx === 'function' && typeof s.Fl === 'function' &&
      s.v7 && ['STREAMING', 'UNREAD', 'REALTIME', 'REALTIME_BUSY', 'REALTIME_BACKGROUND'].every((key, index) =>
        s.v7[key] === index + 3);
  }

  function load() {
    if (modules) return Promise.resolve(modules);
    if (loading) return loading;
    const bindings = page.__elonChatGptPrivateRuntimeBindings;
    const importer = options.loadRuntime || (url => bindings ? bindings.load(url) : import(url));
    let timer;
    loading = Promise.race([
      Promise.all(Object.entries(URLS).map(async ([key, url]) => [key, await importer(url)])),
      new Promise((_, reject) => { timer = page.setTimeout(() => reject(Error('runtime_timeout')),
        options.loadTimeoutMs || 1500); })
    ]).then(entries => {
      const value = Object.fromEntries(entries);
      if (!validate(value)) throw Error('runtime_unknown');
      modules = value;
      return value;
    }).finally(() => { page.clearTimeout(timer); loading = null; });
    return loading;
  }

  function release(owner, receipt) {
    page.clearTimeout(owner.deadline); page.clearTimeout(owner.retry);
    for (const unsubscribe of owner.subscriptions) { try { unsubscribe(); } catch (_) {} }
    owner.subscriptions = [];
    if (active === owner) { active = null; code = receipt.code; }
    owner.resolve(receipt);
  }

  function state() {
    if (active && page.__elonChatGptDocumentToken !== active.binding.token) {
      release(active, { status: 'unknown', code: 'document_changed' });
    }
    return { pending: active !== null, code };
  }

  function observation(binding) {
    const s = modules.shared, tree = s.XM(binding.conversation.id);
    const asyncStatus = s.Fx(binding.conversation);
    return { requestId: s.HM.getRequestId(tree), active: s.Fl(binding.requestId),
      asyncStatus, generation: binding.generation ? generation(s, binding, tree, asyncStatus) : null };
  }

  function observedOwner(binding, value, preparing = false) {
    return binding.generation ? value.requestId == null &&
      sameGeneration(binding.generation, value.generation, preparing) : value.requestId === binding.requestId;
  }

  function stop(command) {
    if (page.__elonChatGptPrivateTextTransactionsEnabled !== true) return decline('disabled');
    if (!/^mcp_[a-z0-9]{1,32}$/.test(command?.requestId || '')) return decline('invalid_command');
    if (state().pending) return active.transaction;
    let binding;
    try {
      if (!command.composer?.isConnected) return decline('composer_unavailable');
      binding = capture(command.composer, true);
      if (!binding) return decline('context_unavailable');
      if (lastDiagnostic?.mode === 'voice') {
        code = 'voice_active';
        return { handled: true, completion: Promise.resolve({ status: 'rejected', code }) };
      }
      if (!validRequestId(binding.requestId) && !(binding.requestId == null &&
          binding.generation?.status?.value === 3)) return decline('request_unavailable');
      if (!Object.values(URLS).every(url => page.__elonChatGptPrivateRuntimeBindings
            ? page.__elonChatGptPrivateRuntimeBindings.observed(url) :
            page.performance?.getEntriesByName?.(url, 'resource')?.length > 0 ||
            page.document.querySelector('link[rel="modulepreload"][href="' + url + '"]'))) return decline('runtime_not_observed');
    } catch (_) { return decline('context_unavailable'); }
    let resolve;
    const completion = new Promise(done => { resolve = done; });
    const owner = { binding, completion, resolve, subscriptions: [], invoked: false, settled: false, retries: 0 };
    owner.transaction = { handled: true, completion, claimFallback() {
      if (!owner.allowFallback || owner.fallbackClaimed || owner.invoked) return false;
      try { if (!currentOwner(binding)) return false; } catch (_) { return false; }
      owner.fallbackClaimed = true;
      return true;
    } };
    active = owner;
    code = 'preparing';

    function unavailable(code) {
      let same = false;
      try { same = currentOwner(binding); } catch (_) {}
      owner.allowFallback = same && code === 'runtime_unavailable';
      release(owner, { status: owner.allowFallback ? 'unavailable' : 'rejected', code });
    }

    function observe() {
      if (active !== owner || !owner.invoked || !owner.settled) return;
      page.clearTimeout(owner.retry); owner.retry = null;
      try {
        if (!current(binding)) return release(owner, { status: 'unknown', code: 'context_changed' });
        const value = observation(binding);
        if (binding.generation ? !observedOwner(binding, value) :
            value.requestId != null && value.requestId !== binding.requestId) {
          return release(owner, { status: 'unknown', code: 'request_changed' });
        }
        if (value.active === false && (value.asyncStatus == null || value.asyncStatus.value === modules.shared.v7.UNREAD)) {
          return release(owner, { status: 'accepted', code: 'stop_observed' });
        }
      } catch (_) { /* Only observed completion may mark the native command stopped. */ }
      if (owner.expired) return release(owner, { status: 'unknown', code: 'timeout' });
      // Official cleanup may follow the stop response. Retry memory state for
      // at most three seconds, then rely on subscriptions and the total deadline.
      if (owner.retries++ < 30) owner.retry = page.setTimeout(observe, 100);
    }

    void load().then(() => {
      if (active !== owner) return;
      try {
        if (!currentOwner(binding)) return unavailable('context_changed');
        const value = observation(binding), mode = value.asyncStatus?.value;
        if ([modules.shared.v7.REALTIME, modules.shared.v7.REALTIME_BUSY,
          modules.shared.v7.REALTIME_BACKGROUND].includes(mode)) {
          return unavailable('voice_active');
        }
        if (!observedOwner(binding, value, true)) return unavailable('request_changed');
        if (value.active === false && value.asyncStatus == null) {
          return release(owner, { status: 'accepted', code: 'already_stopped' });
        }
        if (mode !== modules.shared.v7.STREAMING || typeof value.active !== 'boolean') {
          return unavailable('generation_not_ready');
        }
        const sharedUnsubscribe = binding.shared.subscribeToSharedProps(observe);
        if (typeof sharedUnsubscribe === 'function') owner.subscriptions.push(sharedUnsubscribe);
        const streamUnsubscribe = page.__elonChatGptPrivateStreamTransport?.subscribe?.(observe);
        if (typeof streamUnsubscribe === 'function') owner.subscriptions.push(streamUnsubscribe);
        if (!currentOwner(binding)) return unavailable('context_changed');
        const final = observation(binding);
        if (!observedOwner(binding, final, true) || final.asyncStatus?.value !== modules.shared.v7.STREAMING ||
            typeof final.active !== 'boolean') return unavailable('context_changed');
        owner.deadline = page.setTimeout(() => {
          owner.expired = true;
          code = 'timeout';
          const receipt = { status: 'unknown', code: 'timeout' };
          if (owner.settled) release(owner, receipt);
          else owner.resolve(receipt); // Never release an in-flight stop into another writer.
        }, options.timeoutMs || 15000);
        owner.invoked = true;
        code = 'invoked';
        // Pinned official FVt owns stop_conversation, fresh conduit state,
        // request abortion and tree cleanup. No copied proof or guessed POST.
        // The official overload also stops a server-owned streaming generation
        // without a request entry. Its exact turn/leaf/status were pinned above.
        const receipt = modules.conversation.FVt(binding.conversation.id, binding.generation ? undefined : binding.requestId,
          { clientInitiated: true, clientStopReason: 'user_stop_mouse' });
        if (typeof receipt?.then !== 'function') {
          code = 'invalid_receipt';
          owner.resolve({ status: 'unknown', code: 'invalid_receipt' });
          return;
        }
        Promise.resolve(receipt).then(() => { owner.settled = true; observe(); }).catch(() => {
          release(owner, { status: 'unknown', code: 'stop_failed' });
        });
      } catch (_) {
        if (!owner.invoked) return unavailable('context_unavailable');
        code = 'invocation_failed';
        owner.resolve({ status: 'unknown', code: 'invocation_failed' });
      }
    }).catch(() => unavailable('runtime_unavailable'));
    return owner.transaction;
  }

  return Object.freeze({ version: 7, stop, state, diagnostics });
});
