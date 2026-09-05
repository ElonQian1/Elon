(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptSidebarControlPolicy = Object.freeze(policy);
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  const SIDEBAR_SELECTOR = [
    'aside',
    'nav',
    '[data-testid*="sidebar" i]',
    '[role="navigation"]',
    '[role="dialog"]'
  ].join(', ');
  const ACTION_SELECTOR = 'button, [role="button"], [role="menuitem"], a[href]';
  const ACCOUNT_SIGNAL = /(?:^|[\s_-])(?:accounts?-profile|profile(?:-button|-menu)?|account(?:-button|-menu)?|user-menu|avatar)(?:$|[\s_-])|个人资料|账户|帐户|账号|用户菜单|头像/i;
  const NON_ACCOUNT_SIGNAL = /new[\s_-]?chat|compose|conversation|project|workspace|more|options?|新建|会话|聊天|项目|工作区|更多|选项/i;
  let trackedAccountNodes = new WeakSet();

  function clean(value) {
    return String(value || '').replace(/\s+/g, ' ').trim();
  }

  function attributeSignal(node) {
    if (!node || typeof node.getAttribute !== 'function') return '';
    return clean([
      node.id,
      node.getAttribute('data-testid'),
      node.getAttribute('aria-label'),
      node.getAttribute('title'),
      node.getAttribute('name')
    ].filter(Boolean).join(' '));
  }

  function sameOriginPath(node) {
    if (!node || typeof node.getAttribute !== 'function') return '';
    const href = node.getAttribute('href');
    if (!href) return '';
    try {
      const url = new URL(href, 'https://chatgpt.com');
      return url.origin === 'https://chatgpt.com' ? url.pathname : '';
    } catch {
      return '';
    }
  }

  function isConversationPath(path) {
    return /^\/(?:c\/|g\/g-p-[a-z0-9_-]+\/c\/)/i.test(String(path || ''));
  }

  function isSidebarScope(node, isVisible, viewportWidth, viewportHeight) {
    if (!node || (typeof isVisible === 'function' && !isVisible(node))) return false;
    const rect = node.getBoundingClientRect();
    const width = Math.max(1, Number(viewportWidth) || 1);
    const height = Math.max(1, Number(viewportHeight) || 1);
    return rect.right > 0 && rect.bottom > 0 &&
      rect.left <= width * 0.1 && rect.top <= height * 0.25 &&
      rect.width >= width * 0.35 && rect.height >= height * 0.6;
  }

  function findSidebarScopes(document, isVisible, viewportWidth, viewportHeight) {
    if (!document || typeof document.querySelectorAll !== 'function') return [];
    return Array.from(document.querySelectorAll(SIDEBAR_SELECTOR))
      .filter((node) => isSidebarScope(node, isVisible, viewportWidth, viewportHeight));
  }

  function nearSidebarBottom(node, scope) {
    if (!node || !scope) return false;
    const nodeRect = node.getBoundingClientRect();
    const scopeRect = scope.getBoundingClientRect();
    return nodeRect.top + nodeRect.height / 2 >= scopeRect.top + scopeRect.height * 0.72;
  }

  function opensMenu(node) {
    if (!node || typeof node.getAttribute !== 'function') return false;
    return /^(?:menu|dialog)$/i.test(clean(node.getAttribute('aria-haspopup')));
  }

  function isAccountTrigger(node, scope) {
    if (!node || isConversationPath(sameOriginPath(node))) return false;
    const signal = attributeSignal(node);
    if (ACCOUNT_SIGNAL.test(signal)) return true;
    return !!scope && opensMenu(node) && nearSidebarBottom(node, scope) &&
      !sameOriginPath(node) && !NON_ACCOUNT_SIGNAL.test(signal);
  }

  function candidateRank(node, scope) {
    const signal = attributeSignal(node);
    return (ACCOUNT_SIGNAL.test(signal) ? 100 : 0) +
      (opensMenu(node) ? 20 : 0) +
      (scope && nearSidebarBottom(node, scope) ? 10 : 0);
  }

  function findAccountTriggers(document, isVisible, viewportWidth, viewportHeight) {
    const scopes = findSidebarScopes(document, isVisible, viewportWidth, viewportHeight);
    const candidates = [];
    const seen = new Set();
    scopes.forEach((scope) => {
      Array.from(scope.querySelectorAll(ACTION_SELECTOR)).forEach((node) => {
        if (seen.has(node) || (typeof isVisible === 'function' && !isVisible(node))) return;
        seen.add(node);
        if (isAccountTrigger(node, scope)) candidates.push({ node, scope });
      });
    });
    const selected = candidates
      .sort((left, right) => candidateRank(right.node, right.scope) - candidateRank(left.node, left.scope))
      .slice(0, 2)
      .map((candidate) => candidate.node);
    trackedAccountNodes = new WeakSet(selected);
    return selected;
  }

  function isTrackedAccountTrigger(node) {
    return trackedAccountNodes.has(node);
  }

  return Object.freeze({
    attributeSignal,
    findAccountTriggers,
    findSidebarScopes,
    isAccountTrigger,
    isSidebarScope,
    isTrackedAccountTrigger
  });
});
