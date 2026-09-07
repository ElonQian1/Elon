(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 2, create: factory });
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
  let modules, loading, active = null;

  function capture(node) {
    return page.__elonChatGptPrivateTextRuntimeSubmit?.captureConversation?.(node);
  }

  function current(binding) {
    const now = capture(binding.node);
    return now && ['token', 'account', 'href', 'shared', 'files', 'conversation', 'controller'].every(key =>
      now[key] === binding[key]) ? now : null;
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
    if (active === owner) active = null;
    owner.resolve(receipt);
  }

  function state() {
    if (active && page.__elonChatGptDocumentToken !== active.binding.token) {
      release(active, { status: 'unknown', code: 'document_changed' });
    }
    return { pending: active !== null };
  }

  function observation(binding) {
    const s = modules.shared, tree = s.XM(binding.conversation.id);
    return { requestId: s.HM.getRequestId(tree), active: s.Fl(binding.requestId),
      asyncStatus: s.Fx(binding.conversation) };
  }

  function stop(command) {
    if (page.__elonChatGptPrivateTextTransactionsEnabled !== true ||
        !/^mcp_[a-z0-9]{1,32}$/.test(command?.requestId || '')) return { handled: false };
    if (state().pending) return active.transaction;
    let binding;
    try {
      binding = capture(command.composer);
      if (!binding || !/^[a-z0-9_-]{1,128}$/i.test(binding.requestId || '') ||
          !Object.values(URLS).every(url => page.__elonChatGptPrivateRuntimeBindings
            ? page.__elonChatGptPrivateRuntimeBindings.observed(url) :
            page.performance?.getEntriesByName?.(url, 'resource')?.length > 0 ||
            page.document.querySelector('link[rel="modulepreload"][href="' + url + '"]'))) return { handled: false };
    } catch (_) { return { handled: false }; }
    let resolve;
    const completion = new Promise(done => { resolve = done; });
    const owner = { binding, completion, resolve, subscriptions: [], invoked: false, settled: false, retries: 0 };
    owner.transaction = { handled: true, completion, claimFallback() {
      if (!owner.allowFallback || owner.fallbackClaimed || owner.invoked) return false;
      try { if (current(binding)?.requestId !== binding.requestId) return false; } catch (_) { return false; }
      owner.fallbackClaimed = true;
      return true;
    } };
    active = owner;

    function unavailable(code) {
      let same = false;
      try { same = current(binding)?.requestId === binding.requestId; } catch (_) {}
      owner.allowFallback = same && code === 'runtime_unavailable';
      release(owner, { status: owner.allowFallback ? 'unavailable' : 'rejected', code });
    }

    function observe() {
      if (active !== owner || !owner.invoked || !owner.settled) return;
      page.clearTimeout(owner.retry); owner.retry = null;
      try {
        if (!current(binding)) return release(owner, { status: 'unknown', code: 'context_changed' });
        const value = observation(binding);
        if (value.requestId != null && value.requestId !== binding.requestId) {
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
        if (current(binding)?.requestId !== binding.requestId) return unavailable('context_changed');
        const value = observation(binding), mode = value.asyncStatus?.value;
        if ([modules.shared.v7.REALTIME, modules.shared.v7.REALTIME_BUSY,
          modules.shared.v7.REALTIME_BACKGROUND].includes(mode)) {
          return unavailable('voice_active');
        }
        if (value.requestId !== binding.requestId) return unavailable('request_changed');
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
        if (current(binding)?.requestId !== binding.requestId) return unavailable('context_changed');
        const final = observation(binding);
        if (final.requestId !== binding.requestId || final.asyncStatus?.value !== modules.shared.v7.STREAMING ||
            typeof final.active !== 'boolean') return unavailable('context_changed');
        owner.deadline = page.setTimeout(() => {
          owner.expired = true;
          const receipt = { status: 'unknown', code: 'timeout' };
          if (owner.settled) release(owner, receipt);
          else owner.resolve(receipt); // Never release an in-flight stop into another writer.
        }, options.timeoutMs || 15000);
        owner.invoked = true;
        // Pinned official FVt owns stop_conversation, fresh conduit state,
        // request abortion and tree cleanup. No copied proof or guessed POST.
        const receipt = modules.conversation.FVt(binding.conversation.id, binding.requestId,
          { clientInitiated: true, clientStopReason: 'user_stop_mouse' });
        if (typeof receipt?.then !== 'function') {
          owner.resolve({ status: 'unknown', code: 'invalid_receipt' });
          return;
        }
        Promise.resolve(receipt).then(() => { owner.settled = true; observe(); }).catch(() => {
          release(owner, { status: 'unknown', code: 'stop_failed' });
        });
      } catch (_) {
        if (!owner.invoked) return unavailable('context_unavailable');
        owner.resolve({ status: 'unknown', code: 'invocation_failed' });
      }
    }).catch(() => unavailable('runtime_unavailable'));
    return owner.transaction;
  }

  return Object.freeze({ version: 2, stop, state });
});
