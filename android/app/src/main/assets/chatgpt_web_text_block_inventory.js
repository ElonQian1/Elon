(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptTextBlockInventory = factory(root);
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  let active = false;
  const empty = status => ({ schema: 'elon.text_block_inventory.v2', status, messages: 0, code_blocks: 0,
    writing_blocks: 0, writable_blocks: 0, metadata_messages: 0, unparsed_messages: 0, bounded: true,
    dom_id_matches: 0, dom_code_matches: 0, dom_writing_matches: 0, dom_line_ending_matches: 0 });
  async function read() {
    if (active) return empty('busy');
    active = true;
    let current;
    try {
      const url = new URL(page.location.href), document = page.document, token = page.__elonChatGptDocumentToken;
      const match = /^\/c\/([a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12})$/i.exec(url.pathname);
      const identity = page.__elonChatGptPrivateConversationShareContract?.create(page).identity;
      const account = identity?.();
      if (url.origin !== 'https://chatgpt.com' || !match || url.search || url.hash || url.username || url.password ||
          !document || !/^doc_[a-z0-9_]{3,80}$/.test(token || '') || !account) return empty('context_unavailable');
      current = () => {
        try { return page.document === document && page.location.href === url.href &&
          page.__elonChatGptDocumentToken === token && identity() === account; }
        catch (_) { return false; }
      };
      const response = await page.__elonChatGptPrivateJsonRequest.request(page,
        '/backend-api/conversation/' + match[1], { method: 'GET', credentials: 'same-origin',
          cache: 'no-store', redirect: 'error', __elonPrivateTransport: 'text_block_inventory_v1',
          headers: { ...page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders(), Accept: 'application/json' } },
        { timeoutMs: 8000, maxBytes: 4 * 1024 * 1024, mode: 'json' });
      if (!current()) return empty('context_changed');
      const projection = page.__elonChatGptPrivateHistoryProjection.create({});
      const payload = projection.normalize(response.payload);
      if ((payload.conversation_id || payload.id) !== match[1]) return empty('invalid_response');
      const rows = projection.sourceMessages(payload).slice(-80);
      if (!rows.length && Object.keys(payload.mapping || {}).length) return empty('invalid_response');
      const result = empty('ready');
      result.messages = rows.length;
      const dom = page.__elonChatGptMessages?.readMessages?.(false) || [];
      const domBodies = dom.flatMap(row => (row.content || []).flatMap(part =>
        typeof part.textBlock?.content === 'string' ? [part.textBlock.content] : []));
      const lines = value => value.replace(/\r\n/g, '\n').replace(/\n$/, '');
      for (const { message } of rows) {
        if (dom.some(row => row.id === message.id && row.role === message.author?.role)) result.dom_id_matches++;
        if (message.metadata?.writing_blocks && Object.keys(message.metadata.writing_blocks).length ||
            message.metadata?.content_references?.some?.(value => value?.category === 'writing_block')) result.metadata_messages++;
        const parsed = page.__elonChatGptTextBlocks.project(message);
        if (!parsed) { result.unparsed_messages++; continue; }
        for (const part of parsed.parts) {
          if (part.textBlock.kind === 'code') result.code_blocks++;
          else if (part.textBlock.kind === 'writing') result.writing_blocks++;
          if (part.textBlock.sourceMessageId) result.writable_blocks++;
          if (domBodies.includes(part.textBlock.content)) {
            result[part.textBlock.kind === 'code' ? 'dom_code_matches' : 'dom_writing_matches']++;
          } else if (domBodies.some(body => lines(body) === lines(part.textBlock.content))) result.dom_line_ending_matches++;
        }
      }
      return current() ? result : empty('context_changed');
    } catch (_) { return empty(current && !current() ? 'context_changed' : 'read_failed'); }
    finally { active = false; }
  }
  function handle(action, command, respond) {
    if (action !== 'private_protocol_probe' || command.value !== 'text_block_inventory') return false;
    read().then(result => respond(action, result.status === 'ready', JSON.stringify(result)));
    return true;
  }
  return Object.freeze({ read, handle });
});
