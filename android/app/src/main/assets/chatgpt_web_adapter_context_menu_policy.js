(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptContextMenuPolicy = Object.freeze(policy);
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  function shouldArm(control) {
    return !!control && control.semantic === 'conversation_options' && !!control.contextId;
  }

  function activate(control, node) {
    if (!shouldArm(control) || !node || typeof node.click !== 'function') return false;
    try {
      node.click();
      return true;
    } catch {
      return false;
    }
  }

  return Object.freeze({ activate, shouldArm });
});
