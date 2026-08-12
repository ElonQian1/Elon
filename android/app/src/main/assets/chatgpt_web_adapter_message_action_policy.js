(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptMessageActionPolicy = Object.freeze(policy);
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  function clean(value) {
    return String(value || '').replace(/\s+/g, ' ').trim().toLowerCase();
  }

  function nodeSignal(node) {
    if (!node) return '';
    return clean([
      node.getAttribute && node.getAttribute('data-testid'),
      node.getAttribute && node.getAttribute('aria-label'),
      node.getAttribute && node.getAttribute('title'),
      node.textContent
    ].filter(Boolean).join(' '));
  }

  function isRegenerateSignal(value) {
    return /regenerate|try again|\bretry\b|重新生成|重新回答|重试/.test(clean(value));
  }

  function isOverflowSignal(value) {
    return /more actions?|message actions?|更多操作|更多选项|更多/.test(clean(value));
  }

  function isModelRetryTriggerSignal(value) {
    return /switch model|change model|切换模型|更换模型/.test(clean(value));
  }

  function isRegenerateControl(node) {
    return isRegenerateSignal(nodeSignal(node));
  }

  function isOverflowControl(node) {
    return isOverflowSignal(nodeSignal(node));
  }

  function isModelRetryTriggerControl(node) {
    return isModelRetryTriggerSignal(nodeSignal(node));
  }

  return Object.freeze({
    clean,
    nodeSignal,
    isRegenerateSignal,
    isOverflowSignal,
    isModelRetryTriggerSignal,
    isRegenerateControl,
    isOverflowControl,
    isModelRetryTriggerControl
  });
});
