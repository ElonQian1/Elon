(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 7, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptPrivateTemporaryChat = api;
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options = options || {};
  const ownerPath = page.__elonChatGptCommittedOwnerPath ||
    (typeof module === 'object' && module.exports ? require('./chatgpt_web_committed_owner_path') : null);
  const SHARED = 'https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js';
  const COMPOSER = 'https://chatgpt.com/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js';
  const REACT = 'https://chatgpt.com/cdn/assets/2340486e-dyt4epctwx2pn2sj.js';
  // The inspected AKt callback owns attachment/personalization cleanup and router navigation.
  // Invoke that transaction, not a guessed route setter or a replayed DOM click.
  const ACTION = '()=>{cg.logEvent(`Temporary Chat Move: Temporary Chat Button Clicked`),a?(gB.reset(c),qg()&&$p.delete(n),!o&&!$p(n)&&OKt(s),u(jKt,{replace:!0})):oD(l,{params:o?void 0:new URLSearchParams({[zm]:`true`})})}';
  const now = options.now || (() => Date.now());
  let runtime, loading, cooldown = 0, pending = null, uncertain = null, observedNode = null;

  function identity() {
    const raw = page.__elonChatGptPrivateTransport?.copySameOriginRequestHeaders?.();
    const headers = Object.fromEntries(Object.entries(raw || {}).map(([key, value]) => [key.toLowerCase(), value]));
    if (!/^Bearer\s+\S{8,65536}$/.test(headers.authorization || '')) return null;
    return JSON.stringify(['authorization', 'chatgpt-account-id', 'oai-device-id'].map(key => headers[key] || ''));
  }

  function observed(url) {
    if (page.__elonChatGptPrivateRuntimeBindings) return page.__elonChatGptPrivateRuntimeBindings.observed(url);
    return page.performance?.getEntriesByName?.(url, 'resource')?.length > 0 ||
      !!page.document.querySelector('link[rel="modulepreload"][href="' + url + '"]');
  }

  function owner(node) {
    if (!node?.isConnected) return null;
    const spec = page.__elonChatGptPrivateRuntimeBindings
      ? page.__elonChatGptPrivateRuntimeBindings.temporary() : { owner: 'AKt', action: ACTION };
    if (!spec) return null;
    const key = Object.keys(node).find(key => key.startsWith('__reactFiber$'));
    const chain = ownerPath?.resolve(node[key])?.ancestors || [];
    const index = chain.findIndex(fiber => fiber.type?.name === spec.owner &&
      typeof fiber.memoizedProps?.clientThreadId === 'string');
    if (index < 0) return null;
    const id = chain[index].memoizedProps.clientThreadId;
    if (!/^[a-zA-Z0-9_-]{1,160}$/.test(id) &&
        !/^WEB:[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i.test(id)) return null;
    // The owner records isNew/temp inputs in its first useMemoCache(30).
    // The current build also allocates a separate two-slot cache inside a hook.
    // Checking these prevents a committed but stale empty-chat callback from rewriting a saved chat.
    const data = chain[index].updateQueue?.memoCache?.data;
    const memo = Array.isArray(data) && (data.length === 1 ||
      data.length === 2 && Array.isArray(data[1]) && data[1].length === 2) ? data[0] : null;
    if (!Array.isArray(memo) || memo.length !== 30 || memo[0] !== id ||
        typeof memo[3] !== 'boolean' || typeof memo[4] !== 'boolean' ||
        typeof memo[7] !== 'function' || Function.prototype.toString.call(memo[7]) !== spec.action ||
        memo[19] !== memo[7] || memo[20] !== (memo[4] && !memo[3]) || memo[21] !== memo[4]) return null;
    const conversations = new Set(chain.map(fiber => fiber.memoizedProps?.conversation)
      .filter(conversation => conversation?.id === id && typeof conversation.serverId$ === 'function'));
    const callbacks = chain.slice(0, index).map(fiber => fiber.memoizedProps?.onClick)
      .filter(action => typeof action === 'function');
    const actions = new Set(callbacks.filter(action => Function.prototype.toString.call(action) === spec.action));
    // Read-only indicators retain tooltip handlers, but no privacy action.
    // Only a mutable control must expose the exact captured transaction.
    if (conversations.size !== 1 || (memo[20]
      ? actions.size !== 0 : actions.size !== 1 || !actions.has(memo[7]))) return null;
    return { id, conversation: conversations.values().next().value,
      capturedIsNew: memo[3], capturedSelected: memo[4],
      action: memo[20] || node.disabled === true || node.getAttribute('aria-disabled') === 'true'
        ? null : actions.values().next().value };
  }

  function capture(node) {
    try {
      const url = new URL(page.location.href), account = identity(), token = page.__elonChatGptDocumentToken;
      const cid = /^\/c\/([a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12})$/i.exec(url.pathname)?.[1] || null;
      if (url.origin !== 'https://chatgpt.com' || url.username || url.password || url.hash ||
          url.pathname !== '/' && !cid || url.search && url.search !== '?temporary-chat=true' ||
          !account || !/^doc_[a-z0-9_]{3,80}$/.test(token || '') ||
          !observed(SHARED) || !observed(COMPOSER) || !observed(REACT)) return null;
      const current = owner(node);
      if (!current) return null;
      const serverId = current.conversation.serverId$() || null;
      const persistedHome = cid === null && url.search === '?temporary-chat=true' &&
        typeof serverId === 'string' && /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i.test(serverId);
      if (serverId !== cid && !persistedHome) return null;
      return { ...current, node, token, account, href: url.href, pathname: url.pathname,
        selected: !!url.search, persistedHome };
    } catch (_) { return null; }
  }

  function validate(value) {
    return typeof value?.cX === 'function' && typeof value?.XM === 'function' && typeof value?.uo === 'function' &&
      typeof value?.HM?.getIsNewConversation === 'function';
  }

  function load() {
    if (runtime) return Promise.resolve();
    if (loading) return loading;
    if (now() < cooldown) return Promise.resolve();
    const bindings = page.__elonChatGptPrivateRuntimeBindings;
    const importer = options.loadRuntime || (url => bindings ? bindings.load(url) : import(url));
    let timer;
    loading = Promise.race([
      Promise.resolve().then(() => importer(SHARED)),
      new Promise((_, reject) => { timer = page.setTimeout(() => reject(new Error('temporary_runtime_timeout')), 1500); })
    ]).then(value => {
      if (!validate(value)) throw new Error('temporary_runtime_unknown');
      runtime = value;
    }).catch(() => { cooldown = now() + 10000; }).finally(() => { page.clearTimeout(timer); loading = null; });
    return loading;
  }

  function live(binding) {
    if (!binding || !runtime || runtime.uo(binding.conversation) !== false) return null;
    const thread = runtime.XM(binding.id), selected = runtime.cX();
    if (!thread || typeof thread.is_do_not_remember !== 'boolean' || typeof selected !== 'boolean' ||
        selected !== binding.selected) return null;
    const isNew = runtime.HM.getIsNewConversation(thread);
    if (typeof isNew !== 'boolean') return null;
    if (binding.persistedHome && (isNew || binding.capturedIsNew || !binding.capturedSelected)) return null;
    // The official control reads the router signal, not legacy thread metadata.
    return { ...binding, isNew, current: binding.capturedIsNew === isNew && binding.capturedSelected === selected };
  }

  function sameSession(binding) {
    return binding.token === page.__elonChatGptDocumentToken && binding.account === identity();
  }

  function sameBefore(before, after) {
    return after && ['token', 'account', 'href', 'id', 'conversation'].every(key => before[key] === after[key]);
  }

  function expectedLocation(operation) {
    const url = new URL(page.location.href);
    const target = new URL(operation.before.href);
    target.pathname = operation.initial?.isNew === false ? '/' : target.pathname;
    target.search = operation.desired ? '?temporary-chat=true' : '';
    return url.href === operation.before.href || url.href === target.href;
  }

  function currentAfter(operation) {
    const preferred = capture(operation.before.node);
    if (preferred) return live(preferred);
    const nodes = page.document.querySelectorAll('button[aria-label], [role="button"][aria-label]');
    if (nodes.length > 200) return null;
    const candidates = Array.from(nodes).map(capture).filter(Boolean);
    return candidates.length === 1 ? live(candidates[0]) : null;
  }

  function confirmed(operation, after) {
    return after && after.current && sameSession(after) && after.selected === operation.desired &&
      (operation.initial.isNew
        ? after.conversation === operation.before.conversation && after.id === operation.before.id
        : after.isNew && after.pathname === '/' && after.id !== operation.before.id);
  }

  function finish(operation, ok, detail) {
    if (pending !== operation) return;
    pending = null;
    page.clearTimeout(operation.timer);
    uncertain = !ok && operation.started ? operation : null;
    for (const values of operation.listeners) {
      values.result('set_ui_control_selected', ok, detail || '');
      values.emitSnapshot();
    }
  }

  function check(operation) {
    if (pending !== operation) return;
    try {
      if (!sameSession(operation.before) || !expectedLocation(operation)) {
        return finish(operation, false, '会话已经变化，请重新确认临时聊天状态。');
      }
      const after = currentAfter(operation);
      if (confirmed(operation, after)) return finish(operation, true, '');
      if (after && operation.initial.isNew && after.conversation !== operation.before.conversation) {
        return finish(operation, false, '会话已经变化，请重新确认临时聊天状态。');
      }
    } catch (_) { /* Incomplete router/thread initialization is not a confirmed toggle. */ }
    if (now() >= operation.deadline) return finish(operation, false, '临时聊天状态尚未确认，请稍后重试。');
    operation.timer = page.setTimeout(() => check(operation), 100);
  }

  function observe(node) {
    try {
      const binding = capture(node);
      if (!binding) return null;
      observedNode = node;
      if (!runtime) { void load(); return null; }
      const transition = pending || uncertain;
      const state = live(binding);
      if (transition && sameSession(transition.before) && expectedLocation(transition)) {
        if (transition.initial && confirmed(transition, state)) {
          if (!pending) uncertain = null;
        } else return { selected: transition.before.selected, stateSettable: false };
      }
      return state
        ? { selected: state.selected, stateSettable: state.current && !!state.action } : null;
    } catch (_) { return null; }
  }

  function ownsSelectedConversation(conversation) {
    try {
      const state = live(capture(observedNode));
      return !!state && state.current && state.selected && state.conversation === conversation;
    } catch (_) { return false; }
  }

  function setSelected(values, fallback) {
    if (typeof values?.desiredSelected !== 'boolean') return false;
    if (pending) {
      if (pending.desired === values.desiredSelected && sameSession(pending.before) &&
          expectedLocation(pending) && pending.listeners.length < 8) pending.listeners.push(values);
      else values.result('set_ui_control_selected', false, '临时聊天正在切换，请等待确认。');
      return true;
    }
    const before = capture(values.node);
    if (!before || !runtime && now() < cooldown) return false;
    const operation = { before, desired: values.desiredSelected, listeners: [values], started: false };
    pending = operation;
    function apply() {
      if (pending !== operation) return;
      let initial, outsideScope = false;
      try {
        const current = capture(values.node);
        if (!sameBefore(before, current)) return finish(operation, false, '会话已经变化，请重新确认临时聊天状态。');
        outsideScope = runtime?.uo(current.conversation) === true;
        initial = live(current);
      } catch (_) { /* Pre-write unknown schema may retain the existing control path. */ }
      if (!runtime || outsideScope) {
        pending = null;
        if (operation.listeners.length > 1) {
          for (const duplicate of operation.listeners.slice(1)) duplicate.result('set_ui_control_selected', false, '请等待当前临时聊天操作完成。');
        }
        fallback(); return;
      }
      if (!initial || !initial.current) {
        operation.prepareDeadline ??= now() + 1500;
        if (now() >= operation.prepareDeadline) return finish(operation, false, '临时聊天状态仍在同步，请稍后重试。');
        operation.timer = page.setTimeout(apply, 100);
        return;
      }
      operation.initial = initial;
      if (initial.selected === operation.desired) return finish(operation, true, '');
      if (!initial.action) return finish(operation, false, initial.selected && !initial.isNew
        ? '当前临时会话不可直接转换，请新建普通会话。' : '临时聊天入口当前不可操作，请等待状态同步。');
      operation.started = true;
      operation.deadline = now() + 2400;
      try { Reflect.apply(initial.action, undefined, []); }
      catch (_) { /* The route may have changed before the callback threw; confirm, never replay. */ }
      check(operation);
    }
    if (runtime) apply(); else void load().then(apply);
    return true;
  }

  return Object.freeze({ version: 7, observe, setSelected, ownsSelectedConversation });
});
