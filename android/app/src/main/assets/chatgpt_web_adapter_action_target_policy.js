(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module === 'object' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptActionTargetPolicy = Object.freeze(policy);
})(typeof window === 'object' ? window : null, function () {
  'use strict';

  const MIN_SIZE = 1;
  const MIN_OPACITY = 0.02;

  function number(value) {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : 0;
  }

  function rectOf(node) {
    if (!node || typeof node.getBoundingClientRect !== 'function') return null;
    const rect = node.getBoundingClientRect();
    const left = number(rect.left);
    const top = number(rect.top);
    const right = Number.isFinite(Number(rect.right)) ? Number(rect.right) : left + number(rect.width);
    const bottom = Number.isFinite(Number(rect.bottom)) ? Number(rect.bottom) : top + number(rect.height);
    return { left, top, right, bottom };
  }

  function intersect(left, right) {
    const value = {
      left: Math.max(left.left, right.left),
      top: Math.max(left.top, right.top),
      right: Math.min(left.right, right.right),
      bottom: Math.min(left.bottom, right.bottom)
    };
    return value.right - value.left >= MIN_SIZE && value.bottom - value.top >= MIN_SIZE
      ? value
      : null;
  }

  function styleBlocksAction(style) {
    if (!style) return true;
    return style.display === 'none' || style.visibility === 'hidden' ||
      style.visibility === 'collapse' || style.contentVisibility === 'hidden' ||
      style.pointerEvents === 'none' || number(style.opacity) < MIN_OPACITY;
  }

  function clipsChildren(style) {
    if (!style) return false;
    return [style.overflow, style.overflowX, style.overflowY].some((value) =>
      /^(auto|clip|hidden|scroll)$/i.test(String(value || ''))
    );
  }

  function actionableRect(node) {
    const documentRef = node && node.ownerDocument;
    const view = documentRef && documentRef.defaultView;
    if (!view || typeof view.getComputedStyle !== 'function') return null;
    let visible = intersect(rectOf(node) || {}, {
      left: 0,
      top: 0,
      right: number(view.innerWidth),
      bottom: number(view.innerHeight)
    });
    if (!visible) return null;

    let current = node;
    while (current && current.nodeType === 1) {
      const style = view.getComputedStyle(current);
      if (styleBlocksAction(style)) return null;
      if (current !== node && clipsChildren(style)) {
        visible = intersect(visible, rectOf(current) || {});
        if (!visible) return null;
      }
      current = current.parentElement;
    }
    return visible;
  }

  function isOwnedHit(node, hit) {
    return !!hit && (hit === node || (typeof node.contains === 'function' && node.contains(hit)));
  }

  function actionPoint(node) {
    const documentRef = node && node.ownerDocument;
    if (!documentRef || typeof documentRef.elementFromPoint !== 'function') return null;
    const rect = actionableRect(node);
    if (!rect) return null;
    const xValues = [0.5, 0.25, 0.75];
    const yValues = [0.5, 0.25, 0.75];
    for (const yRatio of yValues) {
      for (const xRatio of xValues) {
        const x = rect.left + (rect.right - rect.left) * xRatio;
        const y = rect.top + (rect.bottom - rect.top) * yRatio;
        if (isOwnedHit(node, documentRef.elementFromPoint(x, y))) return { x, y };
      }
    }
    return null;
  }

  function signature(node) {
    const rect = rectOf(node) || { left: 0, top: 0, right: 0, bottom: 0 };
    const attribute = (name) => String(node && node.getAttribute && node.getAttribute(name) || '');
    return [
      attribute('role'),
      attribute('aria-label'),
      attribute('aria-checked'),
      attribute('aria-selected'),
      attribute('aria-expanded'),
      attribute('data-state'),
      String(node && node.textContent || '').replace(/\s+/g, ' ').trim(),
      Math.round(rect.left),
      Math.round(rect.top),
      Math.round(rect.right),
      Math.round(rect.bottom)
    ].join('|');
  }

  return Object.freeze({ actionPoint, actionableRect, signature });
});
