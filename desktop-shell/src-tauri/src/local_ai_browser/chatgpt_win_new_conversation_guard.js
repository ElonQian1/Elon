(function () {
  'use strict';

  if (location.origin !== 'https://chatgpt.com' || window.__elonWinChatGptNewConversationGuard) return;
  const conversations = window.__elonChatGptConversations;
  if (!conversations || typeof conversations.newConversation !== 'function') return;

  const CONVERSATION_PATH = /^(?:\/c\/[A-Za-z0-9_-]{1,160}|\/g\/g-p-[A-Za-z0-9_-]{1,160}\/c\/[A-Za-z0-9_-]{1,160})$/;
  const CONFIRM_STABLE_MS = 720;
  const CONFIRM_TIMEOUT_MS = 1_400;
  const originalNewConversation = conversations.newConversation.bind(conversations);

  function surface(inspect) {
    try {
      const value = typeof inspect === 'function' ? inspect() : null;
      return value && typeof value === 'object'
        ? {
            messageCount: Math.max(0, Number(value.messageCount) || 0),
            composerReady: value.composerReady === true
          }
        : null;
    } catch {
      return null;
    }
  }

  function confirmFreshConversation(initialPath, inspect, onReady, onTimeout) {
    const started = Date.now();
    let stableSince = 0;
    function poll() {
      const current = surface(inspect);
      const routeChanged = location.pathname !== initialPath;
      const oldConversationLeft = !CONVERSATION_PATH.test(initialPath) || routeChanged;
      const fresh = oldConversationLeft
        && !!current
        && current.messageCount === 0
        && current.composerReady;
      if (fresh) {
        if (!stableSince) stableSince = Date.now();
        if (Date.now() - stableSince >= CONFIRM_STABLE_MS) return onReady();
      } else {
        stableSince = 0;
      }
      if (Date.now() - started >= CONFIRM_TIMEOUT_MS) return onTimeout();
      window.setTimeout(poll, 80);
    }
    poll();
  }

  function newConversation(inspect, result) {
    const initialPath = location.pathname;
    return originalNewConversation(inspect, (action, ok, detail) => {
      if (action !== 'new_conversation' || !ok) return result(action, ok, detail);
      confirmFreshConversation(
        initialPath,
        inspect,
        () => result(action, true, detail),
        () => result(action, false, '官网未离开上一会话，已转入安全恢复。')
      );
    });
  }

  window.__elonChatGptConversations = Object.freeze(Object.assign({}, conversations, {
    newConversation
  }));
  window.__elonWinChatGptNewConversationGuard = Object.freeze({
    version: 1,
    confirmStableMs: CONFIRM_STABLE_MS,
    confirmTimeoutMs: CONFIRM_TIMEOUT_MS
  });
})();
