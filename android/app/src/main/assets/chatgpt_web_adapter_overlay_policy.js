(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptOverlayPolicy = Object.freeze(policy);
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  const EXPLICIT_SELECTOR = [
    '[role="dialog"]',
    '[role="menu"]',
    '[data-radix-menu-content]',
    '[data-headlessui-menu-items]',
    '[data-headlessui-portal]',
    '[data-slot="dropdown-menu-content"]',
    '[data-slot="menu-content"]'
  ].join(', ');
  const MANAGEMENT_ACTION = /view.*files.*chat|rename|unpin|pin.chat|unarchive|archive|share|delete|在聊天中查看文件|重命名|重新命名|取消置顶|置顶聊天|取消归档|归档|分享|删除/i;

  function signal(node) {
    return [
      node && node.id,
      node && node.getAttribute && node.getAttribute('data-testid'),
      node && node.getAttribute && node.getAttribute('aria-label'),
      node && node.getAttribute && node.getAttribute('title'),
      node && node.textContent
    ].filter(Boolean).join(' ').replace(/\s+/g, ' ').trim();
  }

  function isManagementAction(node) {
    return MANAGEMENT_ACTION.test(signal(node));
  }

  function managementSignature(root, isVisible, actionableNodes) {
    if (!root || typeof actionableNodes !== 'function') return '';
    return actionableNodes(root)
      .filter((node) => typeof isVisible !== 'function' || isVisible(node))
      .filter(isManagementAction)
      .map(signal)
      .filter(Boolean)
      .sort()
      .join('|');
  }

  function menuItemSignal(node) {
    const role = String(node && node.getAttribute && node.getAttribute('role') || '').toLowerCase();
    if (!/^menuitem(?:checkbox|radio)?$/.test(role) && !isManagementAction(node)) return '';
    return [
      role,
      node && node.getAttribute && node.getAttribute('data-testid'),
      node && node.getAttribute && node.getAttribute('aria-label'),
      node && node.getAttribute && node.getAttribute('title'),
      node && node.textContent
    ].filter(Boolean).join(':').replace(/\s+/g, ' ').trim().slice(0, 240);
  }

  function contextMenuSignature(root, isVisible, actionableNodes) {
    if (!root || typeof actionableNodes !== 'function') return '';
    return actionableNodes(root)
      .filter((node) => typeof isVisible !== 'function' || isVisible(node))
      .map(menuItemSignal)
      .filter(Boolean)
      .sort()
      .join('|');
  }

  function explicitMenuRoot(root) {
    if (!root || typeof root.getAttribute !== 'function') return false;
    const role = String(root.getAttribute('role') || '').toLowerCase();
    return role === 'menu' || [
      'data-radix-menu-content',
      'data-headlessui-menu-items',
      'data-slot'
    ].some((attribute) => root.hasAttribute && root.hasAttribute(attribute));
  }

  function rankedRoots(roots, isVisible, actionableNodes) {
    return Array.from(new Set(Array.isArray(roots) ? roots : []))
      .map((root, index) => {
        const actions = typeof actionableNodes === 'function' ? actionableNodes(root) : [];
        const visibleActions = actions.filter(
          (node) => typeof isVisible !== 'function' || isVisible(node)
        );
        const managementCount = visibleActions.filter(isManagementAction).length;
        const menuItemCount = visibleActions.map(menuItemSignal).filter(Boolean).length;
        return {
          root,
          index,
          score: managementCount * 1000 + menuItemCount * 100 +
            (explicitMenuRoot(root) ? 25 : 0) - Math.min(visibleActions.length, 24)
        };
      })
      .sort((left, right) => right.score - left.score || right.index - left.index)
      .map((candidate) => candidate.root);
  }

  function inferredRoot(node, isVisible, actionableNodes) {
    let current = node && node.parentElement;
    for (let depth = 0; current && depth < 7; depth += 1, current = current.parentElement) {
      if (!isVisible(current)) continue;
      const actions = actionableNodes(current);
      if (actions.length > 24) continue;
      if (actions.filter(isManagementAction).length >= 2) return current;
    }
    return null;
  }

  function visibleRoots(document, isVisible, actionableNodes) {
    if (!document || typeof document.querySelectorAll !== 'function') return [];
    const explicit = Array.from(document.querySelectorAll(EXPLICIT_SELECTOR)).filter(isVisible);
    const inferred = Array.from(document.querySelectorAll(
      'button, [role="button"], [role="menuitem"], a[href]'
    )).filter(isVisible).filter(isManagementAction)
      .map((node) => inferredRoot(node, isVisible, actionableNodes)).filter(Boolean);
    return rankedRoots(explicit.concat(inferred), isVisible, actionableNodes);
  }

  return Object.freeze({
    contextMenuSignature,
    isManagementAction,
    managementSignature,
    rankedRoots,
    visibleRoots
  });
});
