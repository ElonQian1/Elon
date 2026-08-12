(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module === 'object' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptDictationSessionPolicy = Object.freeze(policy);
})(typeof window === 'object' ? window : null, function () {
  'use strict';

  const CANCEL_SIGNAL = /cancel dictation|cancel recording|discard recording|stop dictation|取消听写|取消录音|放弃录音/i;
  const SUBMIT_SIGNAL = /submit dictation|submit recording|confirm dictation|accept recording|done recording|提交听写|确认听写|完成听写|确认录音|完成录音/i;

  function signal(node) {
    return [
      node && node.id,
      node && node.getAttribute && node.getAttribute('data-testid'),
      node && node.getAttribute && node.getAttribute('aria-label'),
      node && node.getAttribute && node.getAttribute('title'),
      node && node.textContent
    ].filter(Boolean).join(' ').replace(/\s+/g, ' ').trim();
  }

  function rect(node) {
    if (!node || typeof node.getBoundingClientRect !== 'function') return null;
    const value = node.getBoundingClientRect();
    const left = Number(value.left);
    const top = Number(value.top);
    const right = Number(value.right);
    const bottom = Number(value.bottom);
    if (![left, top, right, bottom].every(Number.isFinite)) return null;
    if (right <= left || bottom <= top) return null;
    return {
      left,
      top,
      right,
      bottom,
      width: right - left,
      height: bottom - top,
      centerX: (left + right) / 2,
      centerY: (top + bottom) / 2
    };
  }

  function explicitControl(nodes, kind, isActionable) {
    const pattern = kind === 'cancel' ? CANCEL_SIGNAL : SUBMIT_SIGNAL;
    return nodes.find((node) => isActionable(node) && pattern.test(signal(node))) || null;
  }

  function structuralControls(nodes, isActionable, composerPresent, viewportWidth, viewportHeight) {
    if (composerPresent || viewportWidth <= 0 || viewportHeight <= 0) return null;
    const bottomControls = nodes.map((node) => ({ node, rect: rect(node) }))
      .filter(({ node, rect: value }) => {
        if (!value || !isActionable(node)) return false;
        const ratio = value.width / value.height;
        return value.centerY >= viewportHeight * 0.72 &&
          value.width >= 28 && value.height >= 28 &&
          ratio >= 0.65 && ratio <= 1.55;
      });
    const candidates = bottomControls
      .filter(({ rect: value }) => value.centerX >= viewportWidth * 0.55)
      .sort((left, right) => left.rect.centerX - right.rect.centerX);
    if (candidates.length < 2) return null;

    const right = candidates[candidates.length - 1];
    const left = candidates[candidates.length - 2];
    const rowDelta = Math.abs(left.rect.centerY - right.rect.centerY);
    const sizeRatio = Math.max(left.rect.height, right.rect.height) /
      Math.min(left.rect.height, right.rect.height);
    const horizontalGap = right.rect.centerX - left.rect.centerX;
    const hasLeftCompanion = bottomControls.some(({ rect: value }) =>
      value.centerX <= viewportWidth * 0.3 &&
      Math.abs(value.centerY - left.rect.centerY) <= Math.max(value.height, left.rect.height) * 0.6
    );
    if (!hasLeftCompanion) return null;
    if (rowDelta > Math.max(left.rect.height, right.rect.height) * 0.6) return null;
    if (sizeRatio > 1.6 || horizontalGap <= 0 || horizontalGap > viewportWidth * 0.3) return null;
    return { cancel: left.node, submit: right.node };
  }

  function find(kind, options) {
    const nodes = Array.from(options.nodes || []);
    const isActionable = typeof options.isActionable === 'function'
      ? options.isActionable
      : () => false;
    const explicit = explicitControl(nodes, kind, isActionable);
    if (explicit) return explicit;
    const structural = structuralControls(
      nodes,
      isActionable,
      options.composerPresent === true,
      Number(options.viewportWidth) || 0,
      Number(options.viewportHeight) || 0
    );
    return structural ? structural[kind] : null;
  }

  function active(options) {
    return !!(find('cancel', options) || find('submit', options));
  }

  return Object.freeze({ active, find, signal });
});
