(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root) root.__elonChatGptPrivateHistoryProjection = exported;
})(typeof window === 'object' ? window : null, function (dependencies) {
  'use strict';

  const MAX_MESSAGES = 80;
  const MAX_NODES = 4096;
  const MAX_PARTS = 20;
  const MAX_TEXT = 20000;
  const streamPolicy = dependencies && dependencies.streamPolicy;

  function object(value) {
    return value && typeof value === 'object' && !Array.isArray(value) ? value : null;
  }

  function clean(value, limit) {
    return typeof value === 'string' ? value.replace(/\u00a0/g, ' ').trim().slice(0, limit) : '';
  }

  function normalize(payload) {
    return [payload, payload && payload.conversation, payload && payload.data,
      payload && payload.data && payload.data.conversation, payload && payload.result,
      payload && payload.result && payload.result.conversation].find((value) => object(value) &&
        (object(value.mapping) || Array.isArray(value.messages) ||
          Array.isArray(value.linear_conversation) || Array.isArray(value.items))) || {};
  }

  function orderedNodes(payload) {
    const mapping = object(payload.mapping);
    if (!mapping) {
      return (payload.messages || payload.linear_conversation || payload.items || [])
        .slice(-MAX_NODES).map((node, index) => ({ node, fallbackId: 'private-history-' + index }));
    }
    const keys = Object.keys(mapping);
    if (keys.length > MAX_NODES) return [];
    let cursor = clean(payload.current_node || payload.currentNode, 180);
    if (!cursor) {
      // Only a unique leaf identifies a branch without guessing which regeneration was selected.
      const parents = new Set(keys.map((key) => clean(mapping[key] && mapping[key].parent, 180)));
      const leaves = keys.filter((key) => !parents.has(key));
      if (leaves.length !== 1) return [];
      cursor = leaves[0];
    }
    const seen = new Set();
    const nodes = [];
    while (cursor) {
      if (!Object.prototype.hasOwnProperty.call(mapping, cursor) || seen.has(cursor)) return [];
      seen.add(cursor);
      const node = object(mapping[cursor]);
      if (!node) return [];
      nodes.push({ node, fallbackId: cursor });
      cursor = clean(node.parent, 180);
    }
    return nodes.reverse();
  }

  function visible(message) {
    const role = message.author && message.author.role || message.role;
    if (role !== 'assistant' && role !== 'user') return false;
    if (message.metadata && message.metadata.is_visually_hidden_from_conversation === true) return false;
    if (role === 'assistant') {
      if (message.channel && message.channel !== 'final') return false;
      if (message.recipient && message.recipient !== 'all') return false;
    }
    return true;
  }

  function textContent(content) {
    if (typeof content === 'string') return clean(content, MAX_TEXT);
    if (!object(content)) return '';
    const parts = Array.isArray(content.parts) ? content.parts : [content.text || content.content];
    return parts.slice(0, 128).map((part) => typeof part === 'string' ? part :
      part && (part.text || (typeof part.content === 'string' ? part.content : '')) || '')
      .map((value) => clean(value, MAX_TEXT)).filter(Boolean).join('\n').slice(0, MAX_TEXT);
  }

  function mediaParts(message) {
    const parts = [];
    const content = object(message.content);
    (content && Array.isArray(content.parts) ? content.parts : []).slice(0, MAX_PARTS)
      .forEach((part) => {
        if (!object(part) || part.content_type !== 'image_asset_pointer') return;
        const value = { type: 'image', text: '\u56fe\u7247', kind: 'image' };
        ['width', 'height'].forEach((key) => {
          const dimension = Number(part[key]);
          if (Number.isInteger(dimension) && dimension > 0 && dimension <= 4096) {
            value[key === 'width' ? 'imageWidth' : 'imageHeight'] = dimension;
          }
        });
        // File identifiers and signed URLs stay in the page. DOM may later attach a preview handle.
        parts.push(value);
      });
    const attachments = message.metadata && message.metadata.attachments;
    (Array.isArray(attachments) ? attachments : []).slice(0, MAX_PARTS).forEach((attachment) => {
      const name = clean(attachment && attachment.name, 180);
      if (!name) return;
      const value = { type: 'file', text: name, kind: 'file' };
      const mime = clean(attachment.mime_type, 96);
      if (/^[A-Za-z0-9.+-]{1,63}\/[A-Za-z0-9.+-]{1,63}$/.test(mime)) value.mediaType = mime;
      parts.push(value);
    });
    return parts.slice(0, MAX_PARTS - 1);
  }

  function projectMessage(entry) {
    const node = object(entry.node);
    const message = node && object(node.message || node);
    if (!message || !visible(message)) return null;
    const role = message.author && message.author.role || message.role;
    let text = textContent(message.content);
    let citations = [];
    let cards = [];
    if (role === 'assistant' && streamPolicy) {
      if (typeof streamPolicy.visibleContentText === 'function') text = streamPolicy.visibleContentText(text);
      const projected = streamPolicy.assistantFrame({ message: Object.assign({}, message, {
        author: { role }, content: { content_type: 'text', parts: [text] }
      }) });
      if (projected) { text = projected.text.slice(0, MAX_TEXT); citations = projected.citations || []; }
      cards = streamPolicy.financePartsFromMetadata(message.metadata);
      const chart = streamPolicy.clientChartPartFromMetadata(message.metadata);
      if (chart) cards.push(chart);
    }
    const content = [];
    if (text) content.push({ type: role === 'assistant' ? 'markdown' : 'text', text });
    content.push(...mediaParts(message), ...citations, ...cards);
    if (!content.length) return null;
    return {
      id: clean(message.id || node.id, 180) || entry.fallbackId,
      role,
      state: message.status === 'in_progress' ? 'streaming' : 'completed',
      content: content.slice(0, MAX_PARTS)
    };
  }

  function project(payload) {
    return orderedNodes(normalize(payload)).map(projectMessage).filter(Boolean).slice(-MAX_MESSAGES);
  }

  return Object.freeze({ normalize, project });
});
