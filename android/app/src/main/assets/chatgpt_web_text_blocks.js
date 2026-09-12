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

  function project(message) {
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
    const parts = [], edits = [];
    let codeIndex = 0, writingIndex = 0;
    const finished = /^(finished_successfully|completed|finished)$/.test(message.status || '');
    for (let i = 0; i < lines.length && parts.length < MAX_BLOCKS; i++) {
      const open = fence(lines[i].line);
      const writing = !open && lines[i].line.match(/^ {0,3}:::writing\{([^}]*)\}\s*$/);
      if (!open && !writing) continue;
      const attrs = writing && attributes(writing[1]);
      if (writing && !attrs) continue;
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
        // Only replace complete, supported blocks. Incomplete wrappers remain visible and read-only.
        if (value && end >= 0) edits.push({ start: lines[i].start, end: lines[end].end,
          text: saved + (saved.endsWith('\n') ? '' : '\n') });
      }
      const projected = part(value);
      if (projected) parts.push(projected);
      i = end < 0 ? lines.length : end;
    }
    let text = raw;
    for (const edit of edits.reverse()) text = text.slice(0, edit.start) + edit.text + text.slice(edit.end);
    // The official widget representation is also used without a :::writing wrapper.
    for (const [index, reference] of (Array.isArray(message.metadata?.content_references)
      ? message.metadata.content_references.slice(0, 64) : []).entries()) {
      if (parts.length >= MAX_BLOCKS) break;
      if (reference?.type !== 'client_defined_widget' || reference.category !== 'writing_block' ||
          typeof reference.data?.content !== 'string') continue;
      const data = reference.data;
      const id = token(data.id) || 'writing-reference-' + index;
      if (parts.some(item => item.textBlock.id === id)) continue;
      const value = part(block('writing', id, data.title || data.subject || '', '', data.content, finished));
      if (value) parts.push(value);
    }
    return { text, parts };
  }

  function domCode(content, language, index) {
    return block('code', 'code-' + index, '', language, content, true);
  }

  return { version: 1, project, domCode, MAX_CONTENT };
});
