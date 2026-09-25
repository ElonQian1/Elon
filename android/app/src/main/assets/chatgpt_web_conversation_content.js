(function (root, factory) {
  'use strict';
  const common = typeof module === 'object' && module.exports;
  const blocks = common ? require('./chatgpt_web_text_blocks.js') : root?.__elonChatGptTextBlocks;
  const history = common ? require('./chatgpt_web_private_history_projection.js') : root?.__elonChatGptPrivateHistoryProjection;
  const api = Object.freeze(factory(blocks, history));
  if (common) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonConversationContent = api;
})(typeof window === 'object' ? window : null, function (textBlocks, historyFactory) {
  'use strict';
  const history = historyFactory?.create({});

  function describe(message, scope = {}) {
    if (!history || !textBlocks) throw new Error('reader_unavailable');
    const rich = textBlocks.project(message);
    const content = message.content;
    const parts = typeof content === 'string' ? [content] : Array.isArray(content) ? content :
      Array.isArray(content?.parts) ? content.parts : typeof content?.text === 'string' ? [content.text] :
        typeof content?.content === 'string' ? [content.content] : [];
    const text = [], gaps = new Set();
    for (const part of parts) {
      if (typeof part === 'string') text.push(part);
      else if (typeof part?.text === 'string') text.push(part.text);
      else if (typeof part?.content === 'string') text.push(part.content);
      else if (part?.content_type !== 'image_asset_pointer') gaps.add('unsupported_content_part');
    }
    if (!parts.length) gaps.add('unsupported_message_content');
    const single = { ...scope, messages: [message], mapping: undefined,
      linear_conversation: undefined, page_info: undefined };
    const index = history.files(single);
    if (!index || index.truncated) gaps.add('unsupported_attachment_content');
    const media = (index?.files || []).filter(file => file.kind !== 'source').map(file => ({
      name: file.name, kind: file.kind, mediaType: file.mediaType,
      // Raw provider descriptors remain in the reader's page-local snapshot.
      source: history.fileSource(single, file.id), scope: {
        gizmo_id: scope.gizmo_id, project_id: scope.project_id, context_scopes: scope.context_scopes,
      },
    }));
    // Existing history owner decides which generated tool images are visible.
    const generated = message.author?.role === 'tool';
    return { text: generated ? [] : rich ? [rich.text] : text,
      rich: (rich?.parts || []).map(part => ({ type: part.type,
        id: part.textBlock.id, title: part.textBlock.title, language: part.textBlock.language,
        text: part.textBlock.content, complete: part.textBlock.complete })),
      media, gaps: generated ? [] : [...gaps] };
  }
  return { version: 1, describe };
});
