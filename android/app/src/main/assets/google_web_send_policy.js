(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root && !root.__elonGoogleWebSendPolicy) {
    root.__elonGoogleWebSendPolicy = Object.freeze(api);
  }
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  function reconcile(currentDraft, expectedDraft, prompt) {
    if (!prompt) return Object.freeze({ allowed: false, write: false, staged: false });
    if (currentDraft === prompt) {
      return Object.freeze({ allowed: true, write: false, staged: true });
    }
    if (currentDraft === expectedDraft) {
      return Object.freeze({ allowed: true, write: true, staged: false });
    }
    return Object.freeze({ allowed: false, write: false, staged: false });
  }

  function confirmed(observation) {
    if (!observation) return false;
    return observation.hrefChanged === true || observation.streaming === true ||
      observation.queryMatches === true ||
      observation.currentDraft !== observation.prompt;
  }

  return Object.freeze({ version: 1, reconcile, confirmed });
});
