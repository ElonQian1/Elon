(function (root, factory) {
  'use strict';
  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateContentSource = api;
})(typeof window === 'object' ? window : null, function () {
  'use strict';

  function contentUrl(value) {
    if (typeof value !== 'string' || value.length > 16384 || /[\\\x00-\x20\x7f]/.test(value) ||
        !/^(?:https:\/\/chatgpt\.com(?::443)?)?\/(?:backend-api|api)\/estuary\/content(?:\?[^#]*)?$/.test(value)) return null;
    try { return new URL(value, 'https://chatgpt.com').href; } catch (_) { return null; }
  }

  function previewUrl(value) {
    const content = contentUrl(value);
    if (content) return content;
    if (typeof value !== 'string' || value.length > 16384 || /[\\\x00-\x20\x7f]/.test(value) ||
        !value.startsWith('https://')) return null;
    try {
      const parsed = new URL(value);
      return parsed.protocol === 'https:' && !parsed.username && !parsed.password && !parsed.port &&
        !parsed.hash && /(^|\.)oaiusercontent\.com$/.test(parsed.hostname) ? parsed.href : null;
    } catch (_) { return null; }
  }

  return Object.freeze({ version: 2, contentUrl, previewUrl });
});
