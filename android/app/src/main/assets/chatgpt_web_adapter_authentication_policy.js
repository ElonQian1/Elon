(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root && (!root.__elonChatGptAuthenticationPolicy ||
      Number(root.__elonChatGptAuthenticationPolicy.version || 0) < api.version)) {
    root.__elonChatGptAuthenticationPolicy = Object.freeze(api);
  }
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  function normalized(value) {
    return String(value || '')
      .replace(/\u00a0/g, ' ')
      .replace(/\s+/g, ' ')
      .trim()
      .toLowerCase();
  }

  function isLoginEntry(candidate) {
    const label = normalized(candidate && candidate.label);
    const href = normalized(candidate && candidate.href);
    if (href.includes('/auth/login')) return true;
    return /^(?:log\s*in|login|sign\s*in)(?:\s+(?:to|with)\b.*)?$/.test(label) ||
      /^(?:登录|登入)(?:\s*chatgpt)?$/i.test(label);
  }

  function isAuthenticated(signals) {
    if (!signals || signals.loginRequired === true || signals.hasLoginEntry === true) return false;
    if (signals.hasProfileEntry === true) return true;
    return signals.composerReady === true;
  }

  return Object.freeze({
    version: 1,
    isLoginEntry,
    isAuthenticated
  });
});
