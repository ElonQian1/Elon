(function (root, factory) {
  'use strict';
  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateImagePointer = api;
})(typeof window === 'object' ? window : null, function () {
  'use strict';
  const RESERVED_QUERY = new Set(['gizmo_id', 'project_id', 'conversation_id', 'post_id',
    'check_context_scopes_for_conversation_id', 'context_scopes', 'download_intent', 'inline',
    'authorization', 'cookie', 'access_token']);

  function parse(value) {
    if (typeof value !== 'string' || value.length > 4096 || /[\x00-\x1f\x7f]/.test(value)) return null;
    const match = /^(?:file-service|sediment):\/\/([^?]+)(?:\?(.*))?$/.exec(value);
    if (!match || !/^[A-Za-z0-9_-]{1,160}(?:#[A-Za-z0-9_-]{1,160}){0,4}$/.test(match[1])) return null;
    const query = new Map();
    try {
      // The official resolver preserves the complete ID for metadata lookup,
      // uses the last query value and maps '#' to '*', not a URL fragment.
      decodeURIComponent(match[2] || '');
      let count = 0;
      for (const [key, value] of new URLSearchParams(match[2] || '')) {
        if (++count > 32 || !/^[A-Za-z][A-Za-z0-9_-]{0,63}$/.test(key) ||
            RESERVED_QUERY.has(key.toLowerCase()) || value.length > 1024 || /[\x00-\x1f\x7f]/.test(value)) return null;
        query.set(key, value);
      }
    } catch (_) { return null; }
    return { id: value.slice(value.indexOf('://') + 3), downloadFileId: match[1].replaceAll('#', '*'),
      downloadQuery: Object.freeze(Array.from(query, entry => Object.freeze(entry))) };
  }

  return Object.freeze({ version: 1, parse });
});
