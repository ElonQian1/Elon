(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com' && !root.__elonChatGptPrivateNewConversation) {
    root.__elonChatGptPrivateNewConversation = factory(root);
  }
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options = options || {};
  const now = options.now || (() => Date.now());
  const SAVED_CHAT = /^\/c\/[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;
  let pending = null;

  function snapshot(inspect) {
    try {
      const value = inspect();
      return value?.composerReady === true && Number.isSafeInteger(value.messageCount) && value.messageCount >= 0
        ? { messageCount: value.messageCount } : null;
    } catch (_) { return null; }
  }

  function context(inspect) {
    try {
      const url = ordinaryRoute(), token = page.__elonChatGptDocumentToken;
      if (!url || !/^doc_[a-z0-9_]{3,80}$/.test(token || '')) return null;
      const observed = snapshot(inspect);
      return observed ? { ...observed, href: url.href, token, document: page.document } : null;
    } catch (_) { return null; }
  }

  function ordinaryRoute() {
    try {
      const url = new URL(page.location.href);
      return url.origin === 'https://chatgpt.com' && !url.username && !url.password && !url.search && !url.hash &&
        (url.pathname === '/' || SAVED_CHAT.test(url.pathname)) ? url : null;
    } catch (_) { return null; }
  }

  function sameDocument(owner) {
    return owner.before.document === page.document && owner.before.token === page.__elonChatGptDocumentToken &&
      page.location.origin === 'https://chatgpt.com';
  }

  function release(owner) {
    page.clearTimeout(owner.timer);
    if (pending === owner) pending = null;
  }

  function finish(owner, ok, code) {
    if (pending !== owner) return;
    release(owner);
    const messages = {
      ready: '新会话已就绪。',
      confirmation_required: '新建访客会话需要确认清除当前聊天，请在官网确认。',
      context_changed: '当前会话已经变化，请确认后重试。',
      invocation_unconfirmed: '尚未确认官网执行新会话操作，请稍后检查。',
      timeout: '新会话尚未就绪，请稍后检查。'
    };
    owner.result('new_conversation', ok, messages[code] + ' [runtime_new_chat:' + code + ']');
  }

  function unavailable(owner) {
    if (pending !== owner) return;
    const current = context(owner.inspect);
    if (!sameDocument(owner) || current?.href !== owner.before.href ||
        current.messageCount !== owner.before.messageCount) return finish(owner, false, 'context_changed');
    release(owner);
    owner.fallback();
  }

  function observe(owner) {
    if (pending !== owner) return;
    if (!sameDocument(owner)) return finish(owner, false, 'context_changed');
    // This is the exact guest-discard modal in both inspected official builds.
    // Never press its destructive confirmation or pretend a modal is a new chat.
    let modal;
    try { modal = page.document.querySelector('[data-testid="modal-no-auth-new-chat"]'); }
    catch (_) { return finish(owner, false, 'invocation_unconfirmed'); }
    if (modal) return finish(owner, false, 'confirmation_required');
    const route = ordinaryRoute();
    if (!route || route.href !== owner.before.href && route.href !== 'https://chatgpt.com/') {
      return finish(owner, false, 'context_changed');
    }
    const current = context(owner.inspect);
    if (current?.href === 'https://chatgpt.com/' && current.messageCount === 0) {
      if (owner.freshSince == null) owner.freshSince = now();
      if (now() - owner.freshSince >= 160) return finish(owner, true, 'ready');
    } else owner.freshSince = null;
    if (now() - owner.startedAt >= 5_000) return finish(owner, false, 'timeout');
    owner.timer = page.setTimeout(() => observe(owner), 80);
  }

  function invoke(owner, shared) {
    if (pending !== owner) return;
    const current = context(owner.inspect);
    if (!sameDocument(owner) || current?.href !== owner.before.href ||
        current.messageCount !== owner.before.messageCount) return finish(owner, false, 'context_changed');
    try {
      if (typeof shared?.Ur !== 'function' || typeof shared.zr !== 'function') return unavailable(owner);
      const action = shared.Ur('newChat');
      if (action?.id !== 'newChat' || action.isAvailable !== true || action.disabled !== false ||
          action.scope !== 'global' || action.label?.id !== 'keyboardActions.newChat') return unavailable(owner);
      const event = new page.KeyboardEvent('keydown', { key: 'o', code: 'KeyO', ctrlKey: true,
        shiftKey: true, bubbles: true, cancelable: true });
      // The official registry owns the current React action, router state,
      // disabled groups and guest confirmation. This is not an HTTP reset.
      owner.invoked = true;
      if (shared.zr('newChat', event) !== true) return finish(owner, false, 'invocation_unconfirmed');
      observe(owner);
    } catch (_) {
      if (owner.invoked) finish(owner, false, 'invocation_unconfirmed');
      else unavailable(owner);
    }
  }

  function start(inspect, result, fallback) {
    if (typeof inspect !== 'function' || typeof result !== 'function' || typeof fallback !== 'function') return false;
    if (pending) {
      result('new_conversation', false, '新会话操作正在进行，请稍候。 [runtime_new_chat:busy]');
      return true;
    }
    const bindings = page.__elonChatGptPrivateRuntimeBindings, before = context(inspect);
    let cached;
    try {
      if (!before || typeof bindings?.observed !== 'function' || typeof bindings.peek !== 'function' ||
          typeof bindings.load !== 'function' || !bindings.observed('shared')) return false;
      cached = bindings.peek('shared');
    } catch (_) { return false; }
    const owner = { before, inspect, result, fallback, startedAt: now(), invoked: false, timer: null };
    pending = owner;
    if (cached) invoke(owner, cached);
    else Promise.resolve().then(() => bindings.load('shared')).then(
      shared => invoke(owner, shared), () => unavailable(owner)
    );
    return true;
  }

  return Object.freeze({ version: 1, start, state: () => ({ pending: pending !== null }) });
});
