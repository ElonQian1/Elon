(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root && (!root.__elonGoogleWebSendPolicy ||
      Number(root.__elonGoogleWebSendPolicy.version || 0) < api.version)) {
    root.__elonGoogleWebSendPolicy = Object.freeze(api);
  }
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  function cleanText(value) {
    return String(value || '')
      .replace(/\u00a0/g, ' ')
      .replace(/[ \t]+\n/g, '\n')
      .replace(/\n{3,}/g, '\n\n')
      .trim();
  }

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

  function latestUserQueryMatches(messages, prompt) {
    if (!Array.isArray(messages)) return false;
    for (let index = messages.length - 1; index >= 0; index -= 1) {
      const message = messages[index];
      if (!message || message.role !== 'user') continue;
      const content = Array.isArray(message.content) ? message.content : [];
      const text = content.map((part) => part && part.text || '').join('\n');
      return cleanText(text) === cleanText(prompt);
    }
    return false;
  }

  return Object.freeze({ version: 2, reconcile, confirmed, latestUserQueryMatches });
});
