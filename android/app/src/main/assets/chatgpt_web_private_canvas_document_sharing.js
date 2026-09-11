(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateCanvasDocumentSharing = api;
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  const decoder = page.__elonChatGptPrivateCanvasContent.create(options);
  const fail = () => { throw Error('canvas_share_unconfirmed'); };
  function parse(payload) {
    if (!payload || !Object.hasOwn(payload, 'shared_textdoc')) fail();
    const value = payload.shared_textdoc;
    if (value === null) return { state: 'missing', id: '', documentVersion: null };
    if (!value || typeof value.shared_textdoc_id !== 'string' || !/^[A-Za-z0-9_-]{1,128}$/.test(value.shared_textdoc_id)) fail();
    if (['private', 'workspace'].includes(value.access) || value.is_moderation_blocked === true ||
        value.is_anonify_api_key_detected === true) return { state: 'restricted', id: '', documentVersion: null };
    let snapshot;
    try { snapshot = decoder.decode(payload, value.shared_textdoc_id); } catch (_) { fail(); }
    if (snapshot.documentVersion == null) fail();
    return { state: 'public', id: snapshot.id, documentVersion: snapshot.documentVersion, snapshot };
  }
  const lookup = async (binding, id, deadline) => parse(await options.request(binding,
    '/backend-api/textdoc/' + id + '/share', 'GET', deadline));
  function matches(value, document) {
    return value.state === 'public' && value.snapshot?.title === document.title &&
      value.snapshot.documentType === document.documentType && value.snapshot.content === document.content &&
      value.snapshot.documentVersion === document.documentVersion;
  }
  async function create(binding, document, deadline, dispatch) {
    const payload = await options.request(binding, '/backend-api/textdoc/' + document.id + '/share',
      'POST', deadline, undefined, dispatch);
    const created = parse(payload), observed = await lookup(binding, document.id, deadline);
    if (!matches(created, document) || !matches(observed, document) || created.id !== observed.id) fail();
    return observed;
  }
  function display(documentId, value) {
    return { documentId, state: value.state, id: value.id, documentVersion: value.documentVersion };
  }
  return Object.freeze({ lookup, create, matches, display });
});
