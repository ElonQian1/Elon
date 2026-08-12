(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module === 'object' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptComposerToolStatePolicy = Object.freeze(policy);
})(typeof window === 'object' ? window : null, function () {
  'use strict';

  function clean(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim().toLowerCase();
  }

  function isConversationSearch(signal) {
    return /search[\s_-]*(?:chat|conversation|history)|(?:chat|conversation|history)[\s_-]*search|搜索(?:聊天|会话|历史)/i.test(
      clean(signal)
    );
  }

  function isWebSearchSignal(signal) {
    const value = clean(signal);
    if (!value || isConversationSearch(value)) return false;
    return /web[\s_-]*search|search[\s_-]*(?:the[\s_-]*)?web|browse|网页搜索|联网搜索|^搜索$/.test(value);
  }

  function semantic(input) {
    if (!input || clean(input.region) !== 'composer') return '';
    const label = clean(input.label);
    if (isWebSearchSignal(label)) return 'web_search';
    return isWebSearchSignal(input.signal) ? 'web_search' : '';
  }

  function controlSelected(input) {
    const directSelected = !!(input && input.directSelected);
    return directSelected || !!(
      input && input.semantic === 'web_search' && clean(input.region) === 'composer'
    );
  }

  function optionSelected(input) {
    const directSelected = !!(input && input.directSelected);
    return directSelected || !!(
      input && input.semantic === 'web_search' && input.activeInComposer
    );
  }

  function createSelectionTracker() {
    const values = new Map();
    return Object.freeze({
      observe(semantic, selected) {
        const key = clean(semantic);
        if (key) values.set(key, selected === true);
        return selected === true;
      },
      value(semantic, fallback) {
        const key = clean(semantic);
        return key && values.has(key) ? values.get(key) : fallback === true;
      }
    });
  }

  return Object.freeze({
    controlSelected,
    createSelectionTracker,
    isWebSearchSignal,
    optionSelected,
    semantic
  });
});
