(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com' && !root.__elonChatGptCanvasDocumentActions) {
    root.__elonChatGptCanvasDocumentActions = factory(root);
  }
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  let core;
  const TOKEN = /^cd_[a-f0-9]{32}_[a-z0-9]{1,12}$/;
  function parse(raw) {
    if (typeof raw !== 'string' || raw.length > 2 * 1024 * 1024) throw Error();
    const input = JSON.parse(raw), op = input?.operation;
    const keys = op === 'list' ? ['operation', 'path', 'force'] :
      op === 'save' ? ['operation', 'path', 'ticket', 'scope', 'id', 'content', 'comments'] :
      op === 'verify' ? ['operation', 'path', 'ticket', 'scope', 'id'] : [];
    if (!keys.length || !input || typeof input !== 'object' || Array.isArray(input) ||
        Object.keys(input).some(key => !keys.includes(key)) || typeof input.path !== 'string' ||
        input.path.length > 256 || op === 'list' && typeof input.force !== 'boolean' ||
        op !== 'list' && (!TOKEN.test(input.ticket || '') || !TOKEN.test(input.scope || '') ||
          typeof input.id !== 'string' || !/^[A-Za-z0-9_-]{1,128}$/.test(input.id))) throw Error();
    if (op === 'save' && (typeof input.content !== 'string' || input.content.length > 128 * 1024 ||
        !Array.isArray(input.comments) || input.comments.length > 1000)) throw Error();
    return input;
  }
  function handle(action, command, respond, readSnapshot, emit) {
    if (action !== 'canvas_document') return false;
    let input;
    try {
      input = parse(command?.value);
      if (typeof emit !== 'function' || !/^mcp_[a-z0-9]{1,32}$/.test(command?.requestId || '')) throw Error();
    } catch (_) { respond(action, false, 'canvas_request_invalid'); return true; }
    if (input.operation === 'save' && command.selected !== true) {
      respond(action, false, 'canvas_confirmation_required'); return true;
    }
    try {
      core ||= page.__elonChatGptPrivateCanvasDocuments.create(page);
      core.run(input, command.selected === true, readSnapshot).then(result => {
        if (result.ok) emit({ type: 'canvas_documents', version: 1, requestId: command.requestId,
          path: result.path, ticket: result.ticket, scope: result.scope,
          documents: result.documents, unconfirmedWrite: result.unconfirmedWrite });
        respond(action, result.ok, /^canvas_(?:[a-z_]{1,64}|http_\d{3})$/.test(result.code) ? result.code : 'canvas_unavailable');
      }).catch(() => respond(action, false, 'canvas_write_unconfirmed'));
    } catch (_) { respond(action, false, 'canvas_unavailable'); }
    return true;
  }
  return Object.freeze({ version: 1, handle });
});
