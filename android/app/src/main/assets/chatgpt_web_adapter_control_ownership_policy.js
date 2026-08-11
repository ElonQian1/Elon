(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module === 'object' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptControlOwnershipPolicy = Object.freeze(policy);
})(typeof window === 'object' ? window : null, function () {
  'use strict';

  const COMPOSER_SELECTORS = Object.freeze([
    '[data-testid="prompt-textarea"]',
    '#prompt-textarea',
    'form [contenteditable="true"]',
    'form textarea',
    'main [contenteditable="true"]',
    'textarea[placeholder]'
  ]);

  function contains(owner, candidate) {
    return !!owner && typeof owner.contains === 'function' && owner.contains(candidate);
  }

  function findVisibleComposer(root, isVisible) {
    if (!root || typeof root.querySelectorAll !== 'function' || typeof isVisible !== 'function') {
      return null;
    }
    for (const selector of COMPOSER_SELECTORS) {
      const match = Array.from(root.querySelectorAll(selector)).find(isVisible);
      if (match) return match;
    }
    return null;
  }

  function isPrimaryComposerTextControl(node, region, composer, describe) {
    if (region !== 'composer' || !node || !composer || typeof describe !== 'function') return false;
    const details = describe(node);
    if (!details || details.role !== 'textbox') return false;
    return node === composer || contains(node, composer) || contains(composer, node);
  }

  function createOverlayOwnershipTracker(now, pendingTimeoutMs) {
    const currentTime = typeof now === 'function' ? now : Date.now;
    const timeoutMs = Number.isFinite(pendingTimeoutMs) && pendingTimeoutMs > 0
      ? pendingTimeoutMs
      : 1500;
    let currentPageKey = '';
    let pending = null;
    let active = null;

    function usePage(pageKey) {
      const nextPageKey = String(pageKey || '');
      if (nextPageKey === currentPageKey) return;
      currentPageKey = nextPageKey;
      pending = null;
      active = null;
    }

    function pendingIsUsable() {
      if (!pending) return false;
      if (currentTime() - pending.requestedAt > timeoutMs) {
        pending = null;
        return false;
      }
      if (pending.sourceNode && pending.sourceNode.isConnected === false) {
        pending = null;
        return false;
      }
      return true;
    }

    function rememberMessageTrigger(control, sourceNode, visibleOverlays, pageKey) {
      usePage(pageKey);
      if (
        !control || control.region !== 'message' || control.semantic !== 'more' ||
        !control.contextId || !sourceNode
      ) {
        pending = null;
        return false;
      }
      pending = {
        contextId: String(control.contextId),
        sourceNode,
        existingOverlays: new Set(Array.isArray(visibleOverlays) ? visibleOverlays : []),
        requestedAt: currentTime()
      };
      active = null;
      return true;
    }

    function resolveOverlayContext(overlay, pageKey) {
      usePage(pageKey);
      if (!overlay) {
        active = null;
        pendingIsUsable();
        return '';
      }
      if (
        active && active.overlay === overlay &&
        (!active.overlay || active.overlay.isConnected !== false)
      ) return active.contextId;
      active = null;
      if (!pendingIsUsable() || pending.existingOverlays.has(overlay)) return '';
      active = { overlay, contextId: pending.contextId };
      pending = null;
      return active.contextId;
    }

    function observeNoOverlay(pageKey) {
      usePage(pageKey);
      active = null;
      pendingIsUsable();
    }

    function cancelPending(pageKey) {
      usePage(pageKey);
      pending = null;
    }

    return Object.freeze({
      cancelPending,
      observeNoOverlay,
      rememberMessageTrigger,
      resolveOverlayContext
    });
  }

  return Object.freeze({
    createOverlayOwnershipTracker,
    findVisibleComposer,
    isPrimaryComposerTextControl
  });
});
