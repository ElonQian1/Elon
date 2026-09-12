(function (root, factory) {
  'use strict';
  const api = Object.freeze(factory());
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptWritingBlockPolicy = api;
})(typeof window === 'object' ? window : null, function () {
  'use strict';
  const ID = /^[A-Za-z0-9_-]{1,128}$/;
  const PATH = /^\/c\/([a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12})$/i;
  const TICKET = /^wb_[a-f0-9]{32}$/;
  const own = (object, key) => object && Object.prototype.hasOwnProperty.call(object, key) ? object[key] : null;
  const fail = code => { throw Error('writing_' + code); };
  function text(value) {
    if (typeof value !== 'string' || value.length > 120000) fail('request_invalid');
    for (let i = 0; i < value.length; i++) {
      const c = value.charCodeAt(i);
      if (c >= 0xd800 && c <= 0xdbff) {
        const next = value.charCodeAt(++i);
        if (!(next >= 0xdc00 && next <= 0xdfff)) fail('request_invalid');
      } else if (c >= 0xdc00 && c <= 0xdfff) fail('request_invalid');
    }
    return value;
  }
  function parse(raw) {
    if (typeof raw !== 'string' || raw.length > 900000) fail('request_invalid');
    const input = JSON.parse(raw), op = input?.operation;
    const keys = op === 'prepare' ? ['operation', 'path', 'messageId', 'id', 'content'] :
      op === 'save' ? ['operation', 'path', 'ticket', 'content'] : op === 'verify' ? ['operation', 'path', 'ticket'] : [];
    if (!keys.length || !input || Array.isArray(input) ||
        Object.keys(input).length !== keys.length || Object.keys(input).some(key => !keys.includes(key)) ||
        typeof input.path !== 'string' || !PATH.test(input.path)) fail('request_invalid');
    if (op === 'prepare' ? !ID.test(input.messageId || '') || !ID.test(input.id || '') : !TICKET.test(input.ticket || '')) fail('request_invalid');
    if (op !== 'verify') text(input.content);
    return input;
  }
  function source(payload, conversationId, messageId, id, parser) {
    if (!payload || payload.conversation_id !== conversationId || !payload.mapping || Array.isArray(payload.mapping) ||
        payload.is_do_not_remember !== false || payload.is_temporary_chat === true || payload.gizmo_id != null ||
        payload.shared_project_conversation_owner != null) fail('scope_unconfirmed');
    let nodeId = payload.current_node;
    const seen = new Set();
    let found = null;
    while (nodeId && seen.size < 4096) {
      if (seen.has(nodeId)) fail('branch_unconfirmed');
      seen.add(nodeId);
      const node = own(payload.mapping, nodeId);
      if (!node || node.id !== nodeId) fail('branch_unconfirmed');
      if (node.message?.id === messageId) {
        if (found) fail('selection_ambiguous');
        found = node.message;
      }
      nodeId = node.parent;
    }
    if (nodeId || !found) fail('branch_unconfirmed');
    const sources = parser.project(found, true)?.writeSources?.filter(row => row.id === id) || [];
    if (sources.length !== 1) fail('selection_unavailable');
    const value = sources[0];
    if (value.locallyEdited || typeof value.title !== 'string' || value.title.length > 512 ||
        !value.metadata || typeof value.metadata !== 'object' || Array.isArray(value.metadata) ||
        JSON.stringify(value.metadata).length > 16384) fail('selection_unavailable');
    text(value.content);
    return { ...value, metadata: JSON.parse(JSON.stringify(value.metadata)) };
  }
  function same(a, b) {
    // Metadata is preserved, not synthesized from labels or native form defaults.
    const canonical = value => Array.isArray(value) ? value.map(canonical) : value && typeof value === 'object'
      ? Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])])) : value;
    return JSON.stringify(canonical(a)) === JSON.stringify(canonical(b));
  }
  function body(conversationId, source, content, updatedAt) {
    return { message_id: source.messageId, conversation_id: conversationId, index: String(source.index), id: source.id,
      writing_block: { content: text(content), index: String(source.index), variant: source.variant,
        metadata: source.metadata, title: source.title, id: source.id }, updated_at: updatedAt };
  }
  return { version: 1, parse, source, same, body, PATH, TICKET };
});
