(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 2, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateCanvasContent = api;
})(typeof window === 'object' ? window : null, function (options) {
  'use strict';
  const cache = new Map(), now = options.now || Date.now;
  const ID = /^[A-Za-z0-9_-]{1,128}$/;
  const fail = code => { throw new Error('share_canvas_' + code); };

  function parse(payload, id) {
    const doc = payload?.shared_textdoc;
    if (!doc || doc.shared_textdoc_id !== id) fail('unconfirmed');
    if (doc.is_moderation_blocked === true || doc.is_anonify_api_key_detected === true) fail('restricted');
    if (doc.access !== 'public') fail('scope_unconfirmed');
    if (doc.is_moderation_blocked !== false || doc.is_anonify_api_key_detected != null &&
        doc.is_anonify_api_key_detected !== false) fail('unconfirmed');
    if (typeof doc.content !== 'string' || typeof doc.name !== 'string' || !doc.name.trim() ||
        doc.name.length > 512 || /[\u0000-\u001f\u007f]/.test(doc.name) ||
        typeof doc.textdoc_type !== 'string' ||
        !/^(document|webview|code\/[a-z0-9+#._-]{1,40})$/.test(doc.textdoc_type) ||
        doc.version != null && (!Number.isSafeInteger(doc.version) || doc.version < 1)) fail('unconfirmed');
    if (doc.content.length > 128 * 1024) fail('too_large');
    return Object.freeze({ id, title: doc.name, content: doc.content, documentType: doc.textdoc_type,
      documentVersion: doc.version ?? null, access: 'public' });
  }

  function remaining(deadline) {
    const budget = Math.min(7000, deadline - now());
    if (budget <= 0) fail('timeout');
    return budget;
  }

  async function read(binding, id, { force = false, deadline = Infinity } = {}) {
    if (!ID.test(id)) fail('unconfirmed');
    if (!options.current(binding)) throw new Error('share_context_changed');
    for (const [key, item] of cache) {
      if (!options.current(item.binding) || now() - item.at < 0 || now() - item.at >= 60000) cache.delete(key);
    }
    if (force) cache.delete(id);
    const cached = cache.get(id);
    if (cached) return cached.value;
    // This is the website's shared-textdoc read, not the editable source textdoc.
    const response = await options.request(binding, '/backend-api/textdoc/shared/' + id, 'GET', 'json', remaining(deadline));
    if (!options.current(binding)) throw new Error('share_context_changed');
    remaining(deadline);
    const value = parse(response.payload, id);
    cache.set(id, { binding, at: now(), value });
    while (cache.size > 2) cache.delete(cache.keys().next().value);
    return value;
  }

  async function prepareUpdate(binding, id, deadline) {
    const before = await read(binding, id, { force: true, deadline });
    if (before.documentVersion == null) fail('version_unconfirmed');
    return before;
  }

  async function update(binding, id, before, deadline) {
    cache.delete(id);
    try {
      if (!ID.test(id) || before.id !== id || before.documentVersion == null) fail('unconfirmed');
      // The website publishes the latest source into this existing share, not a new document.
      const response = await options.request(binding, '/backend-api/textdoc/shared/' + id + '/update_to_latest',
        'POST', 'json', remaining(deadline));
      const published = parse(response.payload, id);
      if (published.documentVersion == null || published.documentVersion < before.documentVersion) fail('unconfirmed');
      const observed = await read(binding, id, { force: true, deadline });
      if (['title', 'content', 'documentType', 'documentVersion'].some(key => published[key] !== observed[key])) fail('unconfirmed');
      return observed;
    } catch (error) {
      cache.delete(id);
      throw error;
    }
  }

  return Object.freeze({ version: 2, read, prepareUpdate, update, decode: parse, invalidate: () => cache.clear() });
});
