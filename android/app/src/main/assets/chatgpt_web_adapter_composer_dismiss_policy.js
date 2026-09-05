(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module === 'object' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptComposerDismissPolicy = Object.freeze(policy);
})(typeof window === 'object' ? window : null, function () {
  'use strict';

  const OVERLAY_SELECTOR = [
    '[role="dialog"]', '[role="menu"]', '[role="listbox"]',
    '[data-radix-popper-content-wrapper]', '[data-headlessui-portal]', '[popover]'
  ].join(', ');
  const INTERACTIVE_SELECTOR = [
    'button', 'a[href]', 'input', 'textarea', 'select', '[contenteditable="true"]',
    '[role="button"]', '[role="menuitem"]', '[role="option"]', '[role="slider"]'
  ].join(', ');
  const CANDIDATE_RATIOS = [
    [0.5, 0.24], [0.25, 0.3], [0.75, 0.3], [0.5, 0.45],
    [0.12, 0.5], [0.88, 0.5], [0.5, 0.12]
  ];

  function blocked(hit) {
    if (!hit || typeof hit.closest !== 'function') return true;
    return !!hit.closest(OVERLAY_SELECTOR) || !!hit.closest(INTERACTIVE_SELECTOR);
  }

  function safePoint(documentRef, view) {
    if (!documentRef || typeof documentRef.elementFromPoint !== 'function' || !view) return null;
    const width = Math.max(1, Number(view.innerWidth) || 0);
    const height = Math.max(1, Number(view.innerHeight) || 0);
    for (const [xRatio, yRatio] of CANDIDATE_RATIOS) {
      if (!blocked(documentRef.elementFromPoint(width * xRatio, height * yRatio))) {
        return { xRatio, yRatio };
      }
    }
    return null;
  }

  function emitTouch(documentRef, view, emitEvent, purpose) {
    if (typeof emitEvent !== 'function') return false;
    const point = safePoint(documentRef, view);
    if (!point) return false;
    emitEvent({
      type: 'web_touch_request',
      purpose: purpose || 'dismiss_composer_menu',
      xRatio: point.xRatio,
      yRatio: point.yRatio
    });
    return true;
  }

  return Object.freeze({ emitTouch, safePoint });
});
