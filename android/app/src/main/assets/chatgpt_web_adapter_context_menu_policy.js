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

  function hasNewRoot(before, after) {
    const existing = new Set(Array.isArray(before) ? before : []);
    return (Array.isArray(after) ? after : []).some((root) => !existing.has(root));
  }

  function prepare(control, visibleRoots, scheduleTask, delayMs) {
    if (!shouldArm(control) || typeof visibleRoots !== 'function') return null;
    const before = visibleRoots();
    const schedule = typeof scheduleTask === 'function' ? scheduleTask : setTimeout;
    return function arm(retry) {
      if (typeof retry !== 'function') return false;
      schedule(() => {
        if (!hasNewRoot(before, visibleRoots())) retry();
      }, Number.isFinite(delayMs) ? delayMs : 260);
      return true;
    };
  }

  return Object.freeze({ hasNewRoot, prepare, shouldArm });
});
