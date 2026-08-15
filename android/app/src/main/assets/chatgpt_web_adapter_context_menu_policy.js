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
    delayMs,
    signatureFor,
    confirmationDelayMs,
    isOpen
  ) {
    if (!shouldArm(control) || typeof visibleRoots !== 'function') return null;
    const before = snapshotRoots(visibleRoots(), signatureFor);
    const schedule = typeof scheduleTask === 'function' ? scheduleTask : setTimeout;
    const opened = () => (
      (typeof isOpen === 'function' && isOpen()) ||
      hasNewOrChangedRoot(before, visibleRoots(), signatureFor)
    );
    return function arm(retry) {
      if (typeof retry !== 'function') return false;
      schedule(() => {
        if (opened()) return;
        schedule(() => {
          if (!opened()) retry();
        }, Number.isFinite(confirmationDelayMs) ? confirmationDelayMs : 220);
      }, Number.isFinite(delayMs) ? delayMs : 260);
      return true;
    };
  }

  return Object.freeze({ hasNewOrChangedRoot, hasNewRoot, prepare, shouldArm, snapshotRoots });
});
