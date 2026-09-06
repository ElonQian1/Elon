(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptPrivateTemporaryChat = api;
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options = options || {};
  const SHARED = 'https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js';
  const COMPOSER = 'https://chatgpt.com/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js';
  const REACT = 'https://chatgpt.com/cdn/assets/2340486e-dyt4epctwx2pn2sj.js';
  // The inspected AKt callback owns attachment/personalization cleanup and router navigation.
  // Invoke that transaction, not a guessed route setter or a replayed DOM click.
  const ACTION = '()=>{cg.logEvent(`Temporary Chat Move: Temporary Chat Button Clicked`),a?(gB.reset(c),qg()&&$p.delete(n),!o&&!$p(n)&&OKt(s),u(jKt,{replace:!0})):oD(l,{params:o?void 0:new URLSearchParams({[zm]:`true`})})}';
  const now = options.now || (() => Date.now());
  let runtime, loading, cooldown = 0, pending = null, uncertain = null;

  function identity() {
    const raw = page.__elonChatGptPrivateTransport?.copySameOriginRequestHeaders?.();
    const headers = Object.fromEntries(Object.entries(raw || {}).map(([key, value]) => [key.toLowerCase(), value]));
    if (!/^Bearer\s+\S{8,65536}$/.test(headers.authorization || '')) return null;
    return JSON.stringify(['authorization', 'chatgpt-account-id', 'oai-device-id'].map(key => headers[key] || ''));
  }

  function observed(url) {
    return page.performance?.getEntriesByName?.(url, 'resource')?.length > 0 ||
      !!page.document.querySelector('link[rel="modulepreload"][href="' + url + '"]');
  }

  function owner(node) {
    if (!node?.isConnected) return null;
    const key = Object.keys(node).find(key => key.startsWith('__reactFiber$'));
    const matches = [];
    for (const start of [node[key], node[key]?.alternate]) {
      const chain = [];
      for (let fiber = start; fiber && chain.length < 90; fiber = fiber.return) chain.push(fiber);
      const root = chain.at(-1);
      if (!root || root.return || root.stateNode?.current !== root) continue;
      const index = chain.findIndex(fiber => fiber.type?.name === 'AKt' &&
        typeof fiber.memoizedProps?.clientThreadId === 'string');
      if (index < 0) continue;
      const id = chain[index].memoizedProps.clientThreadId;
      if (!/^[a-zA-Z0-9_-]{1,160}$/.test(id)) continue;
      // AKt's single useMemoCache(30) records the closure's isNew/temp inputs.
      // Checking these prevents a committed but stale empty-chat callback from rewriting a saved chat.
      const data = chain[index].updateQueue?.memoCache?.data;
      const memo = Array.isArray(data) && data.length === 1 ? data[0] : null;
      if (!Array.isArray(memo) || memo.length !== 30 || memo[0] !== id ||
          typeof memo[3] !== 'boolean' || typeof memo[4] !== 'boolean' ||
          typeof memo[7] !== 'function' || Function.prototype.toString.call(memo[7]) !== ACTION ||
          memo[19] !== memo[7] || memo[20] !== (memo[4] && !memo[3]) || memo[21] !== memo[4]) continue;
      const conversations = new Set(chain.map(fiber => fiber.memoizedProps?.conversation)
        .filter(conversation => conversation?.id === id && typeof conversation.serverId$ === 'function'));
      const callbacks = chain.slice(0, index).map(fiber => fiber.memoizedProps?.onClick)
        .filter(action => typeof action === 'function');
      const actions = new Set(callbacks.filter(action => Function.prototype.toString.call(action) === ACTION));
      if (conversations.size !== 1 || actions.size > 1 || callbacks.length && !actions.size ||
          actions.size && !actions.has(memo[7]) || !actions.size && !memo[20]) continue;
      matches.push({ id, conversation: conversations.values().next().value,
        capturedIsNew: memo[3], capturedSelected: memo[4],
        action: memo[20] || node.disabled === true || node.getAttribute('aria-disabled') === 'true'
          ? null : actions.values().next().value });
    }
    return matches.length === 1 ? matches[0] : null;
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
      if (!current || (current.conversation.serverId$() || null) !== cid) return null;
      return { ...current, node, token, account, href: url.href, pathname: url.pathname, selected: !!url.search };
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
    const importer = options.loadRuntime || (url => import(url));
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
    return { ...binding, isNew, current: binding.capturedIsNew === isNew && binding.capturedSelected === selected,
      privacy: thread.is_do_not_remember || binding.conversation.config?.startDoNotRemember === true };
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
    return after && sameSession(after) && after.selected === operation.desired && after.privacy === operation.desired &&
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
      if (!runtime) { void load(); return null; }
      const transition = pending || uncertain;
      const state = live(binding);
      if (transition && sameSession(transition.before) && expectedLocation(transition)) {
        if (transition.initial && confirmed(transition, state)) {
          if (!pending) uncertain = null;
        } else return { selected: transition.before.selected, stateSettable: false };
      }
      return state && state.privacy === state.selected
        ? { selected: state.selected, stateSettable: state.current && !!state.action } : null;
    } catch (_) { return null; }
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
      if (!initial || !initial.current || initial.privacy !== initial.selected) {
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

  return Object.freeze({ version: 1, observe, setSelected });
});
