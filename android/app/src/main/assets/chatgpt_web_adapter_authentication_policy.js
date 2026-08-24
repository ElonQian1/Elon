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

  function hasExplicitLoginRequirement(value) {
    const text = normalized(value);
    if (!text) return false;
    return /(?:sign\s*in|log\s*in)\s+(?:is\s+required\s+)?to\s+continue\b/.test(text) ||
      /(?:登录|登入)(?:以|后)?继续/.test(text) ||
      /(?:需要|请).{0,12}(?:登录|登入).{0,12}继续/.test(text);
  }

  function accessDecision(signals) {
    const value = signals || {};
    if (value.pageKind === 'auth') {
      return Object.freeze({ blocked: true, loginRequired: true, reason: 'login_required', source: 'visible_page' });
    }
    if (value.composerReady !== true && value.hasLoginEntry === true &&
        hasExplicitLoginRequirement(value.visibleText)) {
      return Object.freeze({ blocked: true, loginRequired: true, reason: 'login_required', source: 'visible_page' });
    }
    const privateStatus = Number(value.privateStatus) || 0;
    if (privateStatus === 401 || privateStatus === 403) {
      return Object.freeze({ blocked: true, loginRequired: true, reason: 'login_required', source: 'private_response' });
    }
    if (privateStatus === 429) {
      return Object.freeze({ blocked: true, loginRequired: false, reason: 'rate_limited', source: 'private_response' });
    }
    return Object.freeze({ blocked: false, loginRequired: false, reason: '', source: '' });
  }

  function isAuthenticated(signals) {
    if (!signals || signals.loginRequired === true || signals.hasLoginEntry === true) return false;
    if (signals.hasProfileEntry === true) return true;
    return signals.composerReady === true;
  }

  return Object.freeze({
    version: 2,
    isLoginEntry,
    hasExplicitLoginRequirement,
    accessDecision,
    isAuthenticated
  });
});
