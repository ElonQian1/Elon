(function (root, factory) {
  'use strict';
  const api = Object.freeze(factory());
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptTextBlocks = api;
})(typeof window === 'object' ? window : null, function () {
  'use strict';
  const MAX_CONTENT = 120000;
  const MAX_MESSAGE = 240000;
  const MAX_BLOCKS = 16;
  const own = (value, key) => value && Object.prototype.hasOwnProperty.call(value, key) ? value[key] : undefined;
  const token = value => typeof value === 'string' && /^[A-Za-z0-9_.:+#-]{1,160}$/.test(value) ? value : '';

  function attributes(value) {
    const result = Object.create(null);
    const pattern = /\s*([A-Za-z_][\w:-]*)\s*=\s*(?:"((?:\\.|[^"\\])*)"|'((?:\\.|[^'\\])*)'|([^\s}"',]+))/y;
    let offset = 0;
    while (offset < value.length) {
      if (!value.slice(offset).trim()) break;
      pattern.lastIndex = offset;
      const match = pattern.exec(value);
      if (!match || own(result, match[1]) !== undefined) return null;
      result[match[1]] = (match[2] ?? match[3] ?? match[4]).replace(/\\(["'\\])/g, '$1');
      offset = pattern.lastIndex;
    }
    return result;
  }

  function fence(line) {
    const match = line.match(/^ {0,3}(`{3,}|~{3,})(.*)$/);
    if (!match || match[1][0] === '`' && match[2].includes('`')) return null;
    return { marker: match[1][0], length: match[1].length, info: match[2].trim() };
  }

  function fenceEnd(lines, start, open) {
    for (let i = start + 1; i < lines.length; i++) {
      const close = fence(lines[i].line);
      if (close && close.marker === open.marker && close.length >= open.length && !close.info) return i;
    }
    return -1;
  }

  function writingEnd(lines, start) {
    let depth = 1;
    for (let i = start + 1; i < lines.length; i++) {
      const line = lines[i].line;
      const open = fence(line);
      if (open) {
        const end = fenceEnd(lines, i, open);
        if (end < 0) return -1;
        i = end;
      } else if (/^ {0,3}:::\s*$/.test(line)) {
        if (--depth === 0) return i;
      } else {
        const nested = line.match(/^ {0,3}:::([A-Za-z][A-Za-z0-9_-]*)(?:\s|\{|$)/);
        if (nested && !['writing', 'contextList'].includes(nested[1])) return -1;
        if (nested) depth++;
      }
    }
    return -1;
  }

  function block(kind, id, title, language, content, complete) {
    if (typeof content !== 'string' || content.length > MAX_CONTENT) return null;
    return { version: 1, kind, id: token(id), title: typeof title === 'string' ? title.slice(0, 160) : '',
      language: /^[A-Za-z0-9_+.#-]{1,32}$/.test(language || '') ? language : '', content, complete: complete === true };
  }

  function part(value) {
    if (!value) return null;
    return { type: value.kind === 'writing' ? 'writing_block' : 'code',
      text: value.title || (value.kind === 'writing' ? 'Writing Block' : value.language || 'Code'),
      kind: value.kind === 'writing' ? 'writing_block' : 'code_block', language: value.language,
      lineCount: value.content.split('\n').length, textBlock: value };
  }

  function widgetSource(message, data, saved, index, id, content) {
    const object = value => value && typeof value === 'object' && !Array.isArray(value);
    const variant = saved?.variant ?? data.variant;
    if (!/^[A-Za-z0-9_-]{1,128}$/.test(data.id || '') ||
        !/^[A-Za-z0-9_-]{1,128}$/.test(message.id || '') || message.author?.role !== 'assistant' ||
        message.clientMetadata?.writingBlockOwners || data.library_file_id != null || saved?.library_file_id != null ||
        !['standard', 'document', 'email', 'creative', 'chat_message', 'social_post', 'slides'].includes(variant) ||
        saved != null && (!object(saved) || saved.id != null && saved.id !== id ||
          saved.index != null && String(saved.index) !== String(index))) return null;
    let initial = {};
    if (data.metadata != null) {
      // The official widget accepts JSON or URI-encoded JSON string metadata.
      if (typeof data.metadata !== 'string' || data.metadata.length > 16384) return null;
      let decoded = data.metadata;
      try { decoded = decodeURIComponent(decoded); } catch (_) {}
      let parsed;
      for (const value of [data.metadata, decoded]) {
        try {
          const candidate = JSON.parse(value);
          if (object(candidate) && Object.values(candidate).every(item => typeof item === 'string')) {
            parsed = candidate; break;
          }
        } catch (_) {}
      }
      if (!parsed) return null;
      initial = parsed;
    }
    if (saved?.metadata != null && !object(saved.metadata)) return null;
    for (const key of ['recipient', 'cc', 'bcc', 'subject'])
      if (data[key] != null && (typeof data[key] !== 'string' || data[key].length > 16384)) return null;
    const metadata = { recipient: data.recipient ?? null, cc: data.cc ?? null,
      bcc: data.bcc ?? null, subject: data.subject ?? null, ...initial, ...saved?.metadata };
    const title = saved?.title ?? data.title ?? '';
    if (typeof title !== 'string' || title.length > 512 || JSON.stringify(metadata).length > 16384) return null;
    return { id, messageId: message.id, index, variant, title, metadata, content,
      locallyEdited: saved?.locallyEdited === true, representation: 'widget' };
  }

  function project(message, includeWriteSources = false) {
    const content = message && message.content;
    if (!content || content.content_type !== 'text' || !Array.isArray(content.parts) ||
        content.parts.some(value => typeof value !== 'string')) return null;
    const raw = content.parts.join('');
    if (raw.length > MAX_MESSAGE) return null;
    const lines = [];
    for (const match of raw.matchAll(/[^\r\n]*(?:\r\n|\n|\r|$)/g)) {
      if (!match[0]) continue;
      lines.push({ line: match[0].replace(/[\r\n]+$/, ''), start: match.index, end: match.index + match[0].length });
    }
    const parts = [], edits = [], writeSources = [], writingIds = new Map();
    let scanned = 0, ambiguousWriting = false;
    let codeIndex = 0, writingIndex = 0;
    const finished = /^(finished_successfully|completed|finished)$/.test(message.status || '');
    for (let i = 0; i < lines.length && parts.length < MAX_BLOCKS; i++) {
      scanned = i + 1;
      const open = fence(lines[i].line);
      const writing = !open && lines[i].line.match(/^ {0,3}:::writing\{([^}]*)\}\s*$/);
      if (!open && !writing) continue;
      const attrs = writing && attributes(writing[1]);
      if (writing && !attrs) { ambiguousWriting = true; continue; }
      if (writing) writingIds.set(attrs.id, (writingIds.get(attrs.id) || 0) + 1);
      const end = open ? fenceEnd(lines, i, open) : writingEnd(lines, i);
      const bodyEnd = end < 0 ? raw.length : lines[end].start;
      const original = raw.slice(lines[i].end, bodyEnd);
      let value;
      if (open) {
        const language = open.info.split(/\s/)[0];
        value = block('code', 'code-' + codeIndex++, '', language, original, end >= 0 || finished);
      } else {
        const id = token(attrs.id) || 'writing-' + writingIndex;
        writingIndex++;
        const metadata = token(attrs.id) && own(message.metadata?.writing_blocks, attrs.id);
        const saved = metadata && typeof metadata.content === 'string' ? metadata.content : original;
        value = block('writing', id, metadata?.title || attrs.title || attrs.subject || '', '', saved, end >= 0);
        const variant = metadata?.variant ?? attrs.variant;
        // Only explicit provider IDs on original messages can authorize a later read-check-save.
        if (value && end >= 0 && finished && token(attrs.id) &&
            /^[A-Za-z0-9_-]{1,128}$/.test(message.id || '') &&
            message.author?.role === 'assistant' && !message.clientMetadata?.writingBlockOwners &&
            !metadata?.library_file_id && typeof variant === 'string' && /^[A-Za-z0-9_-]{1,48}$/.test(variant)) {
          value.sourceMessageId = message.id;
          if (includeWriteSources) writeSources.push({ id, messageId: message.id, index: writingIndex - 1,
            variant, title: metadata?.title ?? attrs.title ?? attrs.subject ?? '',
            metadata: metadata?.metadata ?? {}, content: saved, locallyEdited: metadata?.locallyEdited === true });
        }
        // Only replace complete, supported blocks. Incomplete wrappers remain visible and read-only.
        if (value && end >= 0) edits.push({ start: lines[i].start, end: lines[end].end,
          text: saved + (saved.endsWith('\n') ? '' : '\n') });
      }
      const projected = part(value);
      if (projected) parts.push(projected);
      i = end < 0 ? lines.length : end;
      scanned = end < 0 ? lines.length : end + 1;
    }
    let text = raw;
    for (const edit of edits.reverse()) text = text.slice(0, edit.start) + edit.text + text.slice(edit.end);
    // Widget indexes belong to the original reference list, not the filtered card list.
    const references = Array.isArray(message.metadata?.content_references) ? message.metadata.content_references : [];
    if (references.length > 64) ambiguousWriting = true;
    for (const [index, reference] of references.slice(0, 64).entries()) {
      if (reference?.type !== 'client_defined_widget' || reference.category !== 'writing_block' ||
          typeof reference.data?.content !== 'string') continue;
      const data = reference.data;
      const id = token(data.id) || 'writing-reference-' + index;
      writingIds.set(id, (writingIds.get(id) || 0) + 1);
      if (parts.some(item => item.textBlock.id === id)) continue;
      if (parts.length >= MAX_BLOCKS) { ambiguousWriting = true; continue; }
      const saved = token(data.id) && own(message.metadata?.writing_blocks, data.id);
      const current = saved && typeof saved.content === 'string' ? saved.content : data.content;
      const value = block('writing', id, saved?.title ?? data.title ?? data.subject ?? '', '', current, finished);
      if (!value) continue;
      const source = finished && widgetSource(message, data, saved || null, index, id, current);
      if (source) {
        value.sourceMessageId = message.id;
        if (includeWriteSources) writeSources.push(source);
      }
      parts.push(part(value));
    }
    const canOwn = id => !ambiguousWriting && scanned >= lines.length && writingIds.get(id) === 1;
    for (const row of parts) if (!canOwn(row.textBlock.id)) delete row.textBlock.sourceMessageId;
    return { text, parts, ...(includeWriteSources ? { writeSources: writeSources.filter(row => canOwn(row.id)) } : {}) };
  }

  function domCode(content, language, index) {
    return block('code', 'code-' + index, '', language, content, true);
  }

  function runtimeProjection(page, messageId) {
    try {
      const url = new URL(page.location.href);
      const route = /^(?:\/g\/(g-p-[a-f0-9]{32})(?:-[A-Za-z0-9_-]{1,124})?)?\/c\/([a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12})$/i.exec(url.pathname);
      const id = route?.[2], projectId = route?.[1];
      const bindings = page.__elonChatGptPrivateRuntimeBindings;
      if (url.origin !== 'https://chatgpt.com' || !id || url.search || url.hash || url.username || url.password ||
          !/^doc_[a-z0-9_]{3,80}$/.test(page.__elonChatGptDocumentToken || '') ||
          bindings?.state?.().profile_id !== 'web_20260912' ||
          !page.__elonChatGptPrivateConversationShareContract?.create(page).identity()) return null;
      // Only read an already loaded, reviewed module. Rendering must never fetch or wait.
      const shared = bindings.peek('shared');
      const owners = shared?.canvasConversations?.().filter(item => item?.serverId$?.() === id) || [];
      if (owners.length !== 1) return null;
      const thread = shared.XM(owners[0].id);
      if (projectId && shared.HM.getGizmoId?.(thread) !== projectId) return null;
      const message = shared.HM.getNodeIfExists(thread, messageId)?.message;
      if (message?.id !== messageId || message.author?.role !== 'assistant' ||
          !/^(finished_successfully|completed|finished)$/.test(message.status || '')) return null;
      const value = project(message);
      return value?.parts.some(part => part.type === 'writing_block') &&
        value.parts.every(part => part.textBlock?.complete === true) ? value : null;
    } catch (_) { return null; }
  }

  return { version: 3, project, domCode, runtimeProjection, MAX_CONTENT };
});
