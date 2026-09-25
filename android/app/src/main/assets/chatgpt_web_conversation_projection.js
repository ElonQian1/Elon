(function (root, factory) {
  'use strict';
  const content = typeof module === 'object' && module.exports
    ? require('./chatgpt_web_conversation_content.js') : root?.__elonConversationContent;
  const api = factory(content);
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonConversationProjection = api;
})(typeof window === 'object' ? window : null, function (contentProjection) {
  'use strict';
  const ID = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;
  const fail = code => { throw new Error(code); };
  const object = value => value && typeof value === 'object' && !Array.isArray(value);

  function normalize(raw, id, allowIncomplete = false) {
    if (!ID.test(id)) fail('invalid_conversation_id');
    const value = [raw, raw?.conversation, raw?.data, raw?.data?.conversation,
      raw?.result, raw?.result?.conversation].find(value => object(value) &&
      (object(value.mapping) || Array.isArray(value.messages) || Array.isArray(value.linear_conversation)));
    if (!value) fail('unsupported_conversation');
    const ids = [value.id, value.conversation_id].filter(value => value != null);
    if (!ids.length || ids.some(value => value !== id)) fail('conversation_mismatch');
    if (value.has_more === true || value.truncated === true) fail('source_incomplete');
    if (value.page_info != null && messagePage(value, id).cursor && !allowIncomplete) fail('source_incomplete');
    return value;
  }

  function messagePage(value, id) {
    const info = value?.page_info, rows = value?.messages;
    if (!object(info) || typeof info.has_previous_page !== 'boolean' || !Array.isArray(rows)) fail('source_incomplete');
    if ([value.id, value.conversation_id].some(value => value != null && value !== id)) fail('conversation_mismatch');
    if (value.has_more === true || value.truncated === true) fail('source_incomplete');
    if (rows.length > 20000) fail('source_limit');
    const seen = new Set();
    for (const row of rows) {
      if (!object(row) || typeof row.id !== 'string' || !row.id || row.id.length > 180 || seen.has(row.id)) fail('invalid_branch');
      seen.add(row.id);
    }
    const cursor = info.has_previous_page ? info.start_cursor : null;
    if (info.has_previous_page && (typeof cursor !== 'string' || !cursor || cursor.length > 2048)) fail('source_incomplete');
    return { rows, cursor };
  }

  function ordered(value) {
    let mapping = value.mapping;
    if (!object(mapping)) {
      const rows = value.linear_conversation || value.messages;
      if (rows.length > 20000 || rows.some(row => !object(row))) fail('source_limit');
      if (value.page_info != null) {
        // The official paginated endpoint returns the selected branch oldest first.
        // Metadata parent IDs can point outside this representation, including hidden roots.
        messagePage(value, value.conversation_id || value.id);
        if (rows.length && value.current_node !== rows.at(-1).id) fail('invalid_branch');
        return rows;
      }
      // A declared linear conversation is already ordered. Flat Message[] can be a tree.
      const parent = row => row.parent ?? row.parent_id ?? row.metadata?.parent_id;
      if (value.linear_conversation || !rows.some(row => parent(row) != null)) return rows.map(row => row.message || row);
      mapping = {};
      for (const row of rows) {
        if (typeof row.id !== 'string' || Object.hasOwn(mapping, row.id)) fail('invalid_branch');
        Object.defineProperty(mapping, row.id, { value: { message: row.message || row, parent: parent(row) }, enumerable: true });
      }
    }
    const keys = Object.keys(mapping);
    if (keys.length > 20000) fail('source_limit');
    if (!keys.length) return [];
    let cursor = value.current_node || value.currentNode;
    if (!cursor) {
      const parents = new Set(keys.map(key => mapping[key]?.parent));
      const leaves = keys.filter(key => !parents.has(key));
      if (leaves.length !== 1) fail('branch_ambiguous');
      cursor = leaves[0];
    }
    const seen = new Set(), rows = [];
    while (cursor) {
      if (!Object.hasOwn(mapping, cursor) || seen.has(cursor) || !object(mapping[cursor])) fail('invalid_branch');
      seen.add(cursor);
      const node = mapping[cursor];
      rows.push(node.message || node);
      cursor = node.parent;
    }
    return rows.reverse();
  }

  function project(raw, id, onAsset) {
    const value = normalize(raw, id), blocks = [], gaps = new Set();
    let messageCount = 0, attachmentCount = 0;
    for (const message of ordered(value)) {
      let role = message.author?.role || message.role;
      const generated = role === 'tool' ? contentProjection?.describe(message, value) : null;
      if (generated?.media.length) role = 'assistant';
      if (!['user', 'assistant'].includes(role) || message.metadata?.is_visually_hidden_from_conversation === true ||
          role === 'assistant' && (message.channel && message.channel !== 'final' || message.recipient && message.recipient !== 'all')) continue;
      const messageId = typeof message.id === 'string' ? message.id : `message-${messageCount}`;
      const base = { message_id: messageId, role, message_index: messageCount++ };
      let partIndex = 0;
      const addText = (text, attributes = {}) => {
        const chars = Array.from(text);
        if (chars.length > 4 * 1024 * 1024) fail('source_limit');
        for (let offset = 0; offset < chars.length; offset += 3000) {
          blocks.push({ ...base, type: 'text', ...attributes, part_index: partIndex, char_offset: offset,
            char_count: chars.length, text: chars.slice(offset, offset + 3000).join('') });
        }
        partIndex++;
      };
      const addAttachment = part => {
        const assetIndex = attachmentCount;
        attachmentCount++;
        blocks.push({ ...base, type: 'attachment', attachment_id: `${base.message_index}:${partIndex++}`,
          kind: part.kind, name: part.name.slice(0, 180), media_type: part.mediaType || null,
          state: part.source ? 'asset_read_available' : 'native_download_required', bytes_available: false,
          ...(part.source ? { asset_index: assetIndex } : {}) });
        if (part.source && typeof onAsset === 'function') onAsset(assetIndex, part);
        gaps.add('attachment_bytes_not_read');
      };
      if (!contentProjection) fail('reader_unavailable');
      const projected = generated || contentProjection.describe(message, value);
      for (const text of projected.text) addText(text, { format: role === 'assistant' ? 'markdown' : 'plain' });
      for (const rich of projected.rich) addText(rich.text, { type: rich.type,
        block_id: rich.id, title: rich.title, language: rich.language, complete: rich.complete,
        representation: 'structured_content' });
      for (const gap of projected.gaps) gaps.add(gap);
      for (const media of projected.media) addAttachment(media);
    }
    if (blocks.length > 100000) fail('source_limit');
    return { schema: 'yilong.web-conversation.snapshot.v1', conversation_id: id,
      title: typeof value.title === 'string' ? value.title.slice(0, 500) : '',
      branch: 'current', message_count: messageCount, attachment_count: attachmentCount,
      text_complete: !gaps.has('unsupported_content_part') && !gaps.has('unsupported_message_content'),
      multimodal_complete: gaps.size === 0, gaps: [...gaps], blocks };
  }

  function page(snapshot, revision, offset = 0) {
    if (!Number.isSafeInteger(offset) || offset < 0 || offset > snapshot.blocks.length) fail('invalid_cursor');
    const blocks = snapshot.blocks.slice(offset, offset + 2), next = offset + blocks.length;
    return { ...snapshot, blocks, revision, block_offset: offset, total_blocks: snapshot.blocks.length,
      has_more: next < snapshot.blocks.length, next_cursor: next < snapshot.blocks.length ? `${revision}.${next}` : null,
      source_is_untrusted: true };
  }
  return Object.freeze({ version: 3, project, page, normalize, messagePage, ID });
});
