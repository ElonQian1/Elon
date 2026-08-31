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

  function snapshotRoots(roots, signatureFor) {
    const snapshot = new Map();
    (Array.isArray(roots) ? roots : []).forEach((root) => {
      snapshot.set(root, typeof signatureFor === 'function' ? String(signatureFor(root) || '') : '');
    });
    return snapshot;
  }

  function hasNewOrChangedRoot(before, after, signatureFor) {
    const current = Array.isArray(after) ? after : [];
    if (current.some((root) => !before.has(root))) return true;
    if (typeof signatureFor !== 'function') return false;
    return current.some((root) => {
      const next = String(signatureFor(root) || '');
      return next && next !== before.get(root);
    });
  }

  function prepare(
    control,
    visibleRoots,
    scheduleTask,
    pollIntervalMs,
    signatureFor,
    timeoutMs
  ) {
    if (!shouldArm(control) || typeof visibleRoots !== 'function') return null;
    const before = snapshotRoots(visibleRoots(), signatureFor);
    const schedule = typeof scheduleTask === 'function' ? scheduleTask : setTimeout;
    const interval = Number.isFinite(pollIntervalMs) && pollIntervalMs > 0
      ? pollIntervalMs
      : 100;
    const timeout = Number.isFinite(timeoutMs) && timeoutMs >= interval
      ? timeoutMs
      : 1800;
    const opened = () => hasNewOrChangedRoot(before, visibleRoots(), signatureFor);
    return function observe(onOpened, onTimedOut) {
      if (typeof onOpened !== 'function' || typeof onTimedOut !== 'function') return false;
      let elapsed = 0;
      function poll() {
        elapsed += interval;
        if (opened()) return onOpened();
        if (elapsed >= timeout) return onTimedOut();
        schedule(poll, interval);
      }
      schedule(poll, interval);
      return true;
    };
  }

  return Object.freeze({ hasNewOrChangedRoot, hasNewRoot, prepare, shouldArm, snapshotRoots });
});
