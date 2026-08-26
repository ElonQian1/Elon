(function () {
  'use strict';

  if (location.origin !== 'https://chatgpt.com') return;
  const conversations = window.__elonChatGptConversations;
  if (!conversations || typeof conversations.newConversation !== 'function') return;
  if (conversations.__elonWinNewConversationGuardWrapped === true) return;

  const CONVERSATION_PATH = /^(?:\/c\/[A-Za-z0-9_-]{1,160}|\/g\/g-p-[A-Za-z0-9_-]{1,160}\/c\/[A-Za-z0-9_-]{1,160})$/;
  const CONFIRM_STABLE_MS = 1_600;
  const CONFIRM_TIMEOUT_MS = 4_800;
  const originalNewConversation = conversations.newConversation.bind(conversations);

  function cleanText(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim();
  }

  function visibleTurnEvidence() {
    if (!document || typeof document.querySelectorAll !== 'function') return [];
    const seen = new Set();
    return Array.from(document.querySelectorAll(
      '[data-message-author-role], [data-testid^="conversation-turn"], main article'
    )).map((node) => {
      if (!node || seen.has(node)) return null;
      seen.add(node);
      const text = cleanText(node.textContent).slice(0, 320);
      return text ? { node, text } : null;
    }).filter(Boolean).slice(-16);
  }

  function turnStillAttached(entry) {
    const node = entry && entry.node;
    if (!node || node.isConnected === false) return false;
    if (document.documentElement && typeof document.documentElement.contains === 'function' &&
        !document.documentElement.contains(node)) return false;
    return cleanText(node.textContent).slice(0, 320) === entry.text;
  }

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

  function confirmFreshConversation(initialPath, initialSurface, initialTurns, inspect, onReady, onTimeout) {
    const started = Date.now();
    let stableSince = 0;
    function poll() {
      const current = surface(inspect);
      const routeChanged = location.pathname !== initialPath;
      const oldTurnsGone = initialTurns.every((entry) => !turnStillAttached(entry));
      const hadOldSurface = !!initialSurface && initialSurface.messageCount > 0;
      const boundaryObserved = CONVERSATION_PATH.test(initialPath)
        ? routeChanged && oldTurnsGone
        : oldTurnsGone && (!hadOldSurface || initialTurns.length > 0);
      const fresh = boundaryObserved
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
    const initialSurface = surface(inspect);
    const initialTurns = visibleTurnEvidence();
    return originalNewConversation(inspect, (action, ok, detail) => {
      if (action !== 'new_conversation' || !ok) return result(action, ok, detail);
      confirmFreshConversation(
        initialPath,
        initialSurface,
        initialTurns,
        inspect,
        () => result(action, true, detail),
        () => result(action, false, '官网未离开上一会话，已转入安全恢复。')
      );
    });
  }

  window.__elonChatGptConversations = Object.freeze(Object.assign({}, conversations, {
    __elonWinNewConversationGuardWrapped: true,
    newConversation
  }));
  window.__elonWinChatGptNewConversationGuard = Object.freeze({
    version: 3,
    conversations: window.__elonChatGptConversations,
    confirmStableMs: CONFIRM_STABLE_MS,
    confirmTimeoutMs: CONFIRM_TIMEOUT_MS
  });
})();
