(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptMessagePortalPolicy = Object.freeze(policy);
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  const MESSAGE_ACTIONS = new Set([
    'branch',
    'copy',
    'edit',
    'feedback',
    'model',
    'more',
    'next_response',
    'previous_response',
    'read_aloud',
    'regenerate',
    'share'
  ]);

  function finiteNumber(value) {
    const number = Number(value);
    return Number.isFinite(number) ? number : null;
  }

  function normalizedRect(value) {
    const top = finiteNumber(value && value.top);
    const bottom = finiteNumber(value && value.bottom);
    if (top === null || bottom === null || bottom <= top) return null;
    return Object.freeze({ top, bottom });
  }

  function isPortalButtonRole(value) {
    const role = String(value || '').trim().toLowerCase();
    return !['menuitem', 'menuitemcheckbox', 'menuitemradio', 'option', 'slider'].includes(role);
  }

  function maximumGap(viewportHeight) {
    const height = finiteNumber(viewportHeight);
    return Math.max(48, Math.min(144, (height === null ? 800 : height) * 0.16));
  }

  function inferMessageIndex(input) {
    const semantic = String(input && input.semantic || '').trim().toLowerCase();
    if (!MESSAGE_ACTIONS.has(semantic) || !isPortalButtonRole(input && input.role)) return -1;
    const actionRect = normalizedRect(input && input.actionRect);
    const messageRects = Array.isArray(input && input.messageRects) ? input.messageRects : [];
    if (!actionRect || !messageRects.length) return -1;

    const center = (actionRect.top + actionRect.bottom) / 2;
    const maxGap = maximumGap(input && input.viewportHeight);
    let winner = -1;
    let winnerScore = Number.POSITIVE_INFINITY;
    messageRects.forEach((candidate, index) => {
      const rect = normalizedRect(candidate);
      if (!rect) return;
      const gap = center < rect.top ? rect.top - center : center > rect.bottom ? center - rect.bottom : 0;
      if (gap > maxGap) return;
      const aboveMessagePenalty = center < rect.top ? maxGap * 0.5 : 0;
      const nearestEdge = Math.min(Math.abs(center - rect.top), Math.abs(center - rect.bottom));
      const score = gap * 4 + nearestEdge + aboveMessagePenalty;
      if (score < winnerScore) {
        winner = index;
        winnerScore = score;
      }
    });
    return winner;
  }

  function findMessageNodes(documentObject, limit) {
    const main = documentObject && documentObject.querySelector('main');
    if (!main) return [];
    const turns = Array.from(main.querySelectorAll('[data-testid^="conversation-turn-"]'));
    const nodes = turns.length ? turns : Array.from(main.querySelectorAll('[data-message-author-role]'));
    return Number.isInteger(limit) && limit > 0 ? nodes.slice(-limit) : nodes;
  }

  function messageContextId(node, index) {
    return String(
      node && (node.getAttribute('data-message-id') || node.getAttribute('data-testid') || node.id)
      || 'message-' + index
    ).replace(/[^A-Za-z0-9_.:-]/g, '_').slice(0, 160);
  }

  function inferMessageContext(input) {
    const messages = Array.isArray(input && input.messages) ? input.messages : [];
    const index = inferMessageIndex({
      semantic: input && input.semantic,
      role: input && input.role,
      actionRect: input && input.actionRect,
      messageRects: messages.map((node) => node.getBoundingClientRect()),
      viewportHeight: input && input.viewportHeight
    });
    return index >= 0 ? messageContextId(messages[index], index) : '';
  }

  return Object.freeze({
    findMessageNodes,
    inferMessageContext,
    inferMessageIndex,
    messageContextId
  });
});
