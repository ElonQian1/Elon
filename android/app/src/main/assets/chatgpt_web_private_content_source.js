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

  function projectContentUrl(value, scope) {
    if (typeof value !== 'string' || value.length > 16384 || /[\\\x00-\x20\x7f]/.test(value) ||
        !/^(?:https:\/\/chatgpt\.com(?::443)?)?\/api\/library\/files\//.test(value) ||
        !/^libfile[_-][A-Za-z0-9_-]{1,152}$/.test(scope?.libraryFileId || '') ||
        !/^file[_-][A-Za-z0-9_-]{1,152}$/.test(scope?.fileId || '')) return null;
    try {
      const url = new URL(value, 'https://chatgpt.com');
      // CDt binds both the library row and concrete file; do not borrow current-chat scope.
      return url.origin === 'https://chatgpt.com' && !url.username && !url.password && !url.hash &&
        url.pathname === '/api/library/files/' + scope.libraryFileId + '/project-content' &&
        [...url.searchParams].length === 1 && url.searchParams.get('file_id') === scope.fileId ? url.href : null;
    } catch (_) { return null; }
  }

  function describe(value) {
    const result = { schema: 'elon.download_source.v1', observed: true, origin: 'invalid', path: '',
      relative: false, whitespace: false, credentials: false, port: false, fragment: false };
    if (typeof value !== 'string' || value.length > 16384) return result;
    result.relative = !/^[A-Za-z][A-Za-z0-9+.-]*:/.test(value);
    result.whitespace = /[\\\x00-\x20\x7f]/.test(value);
    try {
      const url = new URL(value, 'https://chatgpt.com');
      result.origin = url.origin === 'https://chatgpt.com' ? 'same_origin' :
        url.protocol !== 'https:' ? 'non_https' :
        /(^|\.)oaiusercontent\.com$/.test(url.hostname) ? 'oaiusercontent' :
        /\.blob\.core\.windows\.net$/.test(url.hostname) ? 'azure_blob' : 'other_https';
      if (result.origin === 'same_origin') {
        const vocabulary = new Set(['api', 'backend-api', 'files', 'library', 'download', 'content', 'project-content', 'estuary', 'attachment', 'attachments']);
        const parts = url.pathname.split('/').filter(Boolean).slice(0, 8);
        result.path = parts.length ? '/' + parts.map(part => vocabulary.has(part) ? part : '{id}').join('/') : '';
      }
      result.credentials = Boolean(url.username || url.password);
      result.port = Boolean(url.port);
      result.fragment = Boolean(url.hash);
    } catch (_) {}
    return result;
  }

  return Object.freeze({ version: 4, contentUrl, previewUrl, projectContentUrl, describe });
});
