(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  if (root) root.__elonChatGptContextMenuInvocation = Object.freeze(api);
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  function createCoordinator() {
    let pending = null;

    function complete(candidate, ok) {
      if (!candidate || pending !== candidate) return false;
      pending = null;
      candidate.complete(ok === true);
      return true;
    }

    function reconcile() {
      const candidate = pending;
      if (!candidate) return false;
      if (candidate.observation.isOpen()) return complete(candidate, true);
      if (!candidate.retried) {
        candidate.retried = true;
        if (candidate.emitTouch()) return true;
      }
      return complete(candidate, false);
    }

    function start(observation, emitTouch, onComplete) {
      if (typeof observation !== 'function' || typeof observation.isOpen !== 'function' ||
          typeof emitTouch !== 'function' || typeof onComplete !== 'function') return false;
      if (pending) complete(pending, false);
      const candidate = {
        observation,
        emitTouch,
        complete: onComplete,
        retried: false
      };
      pending = candidate;
      observation(
        () => complete(candidate, true),
        () => reconcile()
      );
      if (!candidate.emitTouch()) complete(candidate, false);
      return true;
    }

    return Object.freeze({ reconcile, start });
  }

  return Object.freeze({ createCoordinator });
});
