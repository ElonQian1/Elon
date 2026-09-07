(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') {
    const old = root.__elonChatGptPrivateRegenerateRuntime;
    if (!(Number(old?.version) >= api.version) && !old?.state?.().pending) {
      root.__elonChatGptPrivateRegenerateRuntime = api.create(root);
    }
  }
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options = options || {};
  const contract = (options.contract || page.__elonChatGptPrivateRegenerateContract)?.create(page);
  let modules, loading, active = null;

  function load() {
    if (modules) return Promise.resolve(modules);
    if (loading) return loading;
    const importer = options.loadRuntime || (url => import(url));
    let timer;
    loading = Promise.race([
      Promise.all(['shared', 'conversation'].map(async key => [key, await importer(contract.urls[key])])),
      new Promise((_, reject) => { timer = page.setTimeout(() => reject(new Error('runtime_timeout')),
        options.loadTimeoutMs || 1500); })
    ]).then(entries => {
      const value = Object.fromEntries(entries);
      if (!contract.validate(value)) throw new Error('runtime_unknown');
      modules = value;
      return value;
    }).finally(() => { page.clearTimeout(timer); loading = null; });
    return loading;
  }

  function release(owner) {
    page.clearTimeout(owner.deadline); page.clearTimeout(owner.retry);
    owner.unsubscribe?.(); owner.unsubscribe = null;
    if (active === owner) active = null;
  }

  function state() {
    if (active && page.__elonChatGptDocumentToken !== active.binding.token) {
      active.resolve({ status: 'unknown', code: 'document_changed' });
      release(active);
    }
    return { pending: active !== null };
  }

  function available(turn, getModelTrigger) {
    try { return !!contract?.capture(turn, getModelTrigger); } catch (_) { return false; }
  }

  function regenerate(command) {
    if (page.__elonChatGptPrivateTextTransactionsEnabled !== true || !contract) return { handled: false };
    if (state().pending || page.__elonChatGptPrivateTextRuntimeSubmit?.state?.().pending ||
        page.__elonChatGptPrivateTextTransactionRelay?.state?.().active) {
      return { handled: true, completion: Promise.resolve({ status: 'unknown', code: 'busy' }) };
    }
    if (!/^mcp_[a-z0-9]{1,32}$/.test(command?.requestId || '')) return { handled: false };
    const stream = page.__elonChatGptPrivateStreamTransport;
    if (typeof stream?.subscribe !== 'function' || typeof stream.current !== 'function' ||
        typeof stream.prepareSend !== 'function') return { handled: false };
    let binding;
    try { binding = contract.capture(command.turn, command.getModelTrigger); } catch (_) {}
    if (!binding) return { handled: false };
    let resolve;
    const completion = new Promise(done => { resolve = done; });
    const owner = { binding, resolve, invoked: false, retries: 0 }; active = owner;

    function unavailable(code) {
      if (owner.invoked) return resolve({ status: 'unknown', code: 'invocation_failed' });
      let same = false;
      try { same = !!contract.current(binding); } catch (_) {}
      release(owner);
      // Only a pre-invocation failure may select the existing sender/menu route.
      resolve({ status: same && code === 'runtime_unavailable' ? 'unavailable' : 'rejected', code });
    }

    function observe() {
      if (active !== owner || !owner.invoked) return;
      page.clearTimeout(owner.retry); owner.retry = null;
      try {
        const value = stream.current(page.location.pathname);
        if (!contract.ownerCurrent(owner.binding)) return;
        let confirmed = false;
        try { confirmed = contract.observed(owner.binding, modules, value); }
        catch (_) { /* The official tree throws until a streamed node is committed. */ }
        if (confirmed) {
          release(owner);
          return resolve({ status: 'accepted', code: 'regenerate_observed' });
        }
        // A stream event may precede the official tree commit. Retry only that
        // event, bounded in memory; do not poll the DOM or issue another request.
        if (value?.conversationId === binding.cid && value.id && value.text &&
            !owner.binding.variants.has(value.id) && owner.retries++ < 20) {
          owner.retry = page.setTimeout(observe, 100);
        }
      } catch (_) { /* Unknown runtime state is not permission to replay. */ }
    }

    void load().then(() => {
      if (active !== owner) return;
      try {
        const prepared = contract.prepare(binding, modules);
        if (!prepared) return unavailable('context_changed');
        stream.prepareSend();
        command.beforeSubmit?.();
        const final = contract.prepare(prepared, modules);
        if (!final || final.parentId !== prepared.parentId) return unavailable('context_changed');
        owner.binding = final;
        owner.unsubscribe = stream.subscribe(observe);
        owner.deadline = page.setTimeout(() => resolve({ status: 'unknown', code: 'timeout' }),
          options.timeoutMs || 15000);
        owner.invoked = true;
        // CUn's committed callback invokes the official regeneration hook. It
        // retains parent prompt, effort, tool hints, request proof and React state.
        prepared.callback({ sourceEvent: new page.Event('click'), requestedModelId: prepared.modelSlug });
        observe();
      } catch (_) {
        if (!owner.invoked) return unavailable('context_unavailable');
        resolve({ status: 'unknown', code: 'invocation_failed' });
      }
    }).catch(() => unavailable('runtime_unavailable'));
    return { handled: true, completion };
  }

  return Object.freeze({ version: 1, regenerate, available, state });
});
