(function (root, factory) {
  'use strict';
  const api = Object.freeze(factory());
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateCanvasDocumentPolicy = api;
})(typeof window === 'object' ? window : null, function () {
  'use strict';
  const ID = /^[A-Za-z0-9_-]{1,128}$/, LIMIT = 128 * 1024;
  const fail = code => { throw Error('canvas_' + code); };
  const object = value => value && typeof value === 'object' && !Array.isArray(value);

  function offsets(text) {
    if (typeof text !== 'string' || text.length > LIMIT) fail('content_invalid');
    const points = [0];
    for (const char of text) {
      if (char.length === 1 && /[\ud800-\udfff]/.test(char)) fail('content_invalid');
      points.push(points[points.length - 1] + char.length);
    }
    return points;
  }

  function comments(rows, content, utf16 = false) {
    if (!Array.isArray(rows) || rows.length > 1000) fail('comments_invalid');
    const points = offsets(content), reverse = new Map(points.map((offset, index) => [offset, index]));
    const ids = new Set();
    return Object.freeze(rows.map(row => {
      if (!object(row) || typeof row.id !== 'string' || !ID.test(row.id) || ids.has(row.id) ||
          typeof row.content !== 'string' || row.content.length > 16384 ||
          !Number.isSafeInteger(row.start) || !Number.isSafeInteger(row.end) || row.start < 0 || row.end < row.start ||
          (utf16 ? !reverse.has(row.start) || !reverse.has(row.end) : row.end >= points.length)) fail('comments_invalid');
      ids.add(row.id);
      return Object.freeze({ id: row.id, content: row.content,
        start: utf16 ? reverse.get(row.start) : points[row.start],
        end: utf16 ? reverse.get(row.end) : points[row.end] });
    }));
  }

  function parse(payload) {
    if (!Array.isArray(payload) || payload.length > 200) fail('list_unconfirmed');
    const ids = new Set();
    return Object.freeze(payload.map(row => {
      if (!object(row) || typeof row.id !== 'string' || !ID.test(row.id) || row.id.startsWith('temp-') || ids.has(row.id) ||
          typeof row.title !== 'string' || !row.title.trim() || row.title.length > 512 ||
          /[\u0000-\u001f\u007f]/.test(row.title) || typeof row.textdoc_type !== 'string' ||
          !/^(document|webview|code\/[a-z0-9+#._-]{1,40})$/.test(row.textdoc_type) ||
          !Number.isSafeInteger(row.version) || row.version < 1) fail('list_unconfirmed');
      ids.add(row.id);
      return Object.freeze({ id: row.id, title: row.title, content: row.content,
        documentType: row.textdoc_type, documentVersion: row.version,
        comments: comments(row.comments, row.content) });
    }));
  }

  function draft(before, value) {
    if (!object(value)) fail('draft_invalid');
    const serialized = comments(value.comments, value.content, true);
    const previous = new Map(before.comments.map(comment => [comment.id, comment]));
    if (serialized.length !== previous.size) fail('comment_removal_unconfirmed');
    for (const comment of value.comments) {
      const old = previous.get(comment.id);
      if (!old || comment.content !== old.content) fail('comments_changed');
      // A deleted anchor needs an explicit comment decision, never silent removal on text save.
      if (old.end > old.start && comment.start === comment.end) fail('comment_anchor_removed');
    }
    return Object.freeze({ content: value.content, comments: serialized });
  }

  function equalComments(left, right) {
    if (left.length !== right.length) return false;
    const byId = new Map(right.map(value => [value.id, value]));
    return left.every(value => {
      const other = byId.get(value.id);
      return other && ['start', 'end', 'content'].every(key => value[key] === other[key]);
    });
  }

  function same(left, right) {
    return ['id', 'title', 'content', 'documentType', 'documentVersion'].every(key => left[key] === right[key]) &&
      equalComments(left.comments, right.comments);
  }

  function matches(saved, before, expected, version) {
    return saved?.id === before.id && saved.documentType === before.documentType && saved.title === before.title &&
      saved.documentVersion === version && saved.content === expected.content &&
      equalComments(saved.comments, comments(expected.comments, expected.content));
  }

  function renameTitle(value) {
    if (typeof value !== 'string' || !value || value !== value.trim() || value.length > 512 ||
        /[\u0000-\u001f\u007f]/.test(value)) fail('title_invalid');
    offsets(value);
    return value;
  }

  function renamed(saved, before, title) {
    return !!saved && saved.title === title && saved.documentVersion >= before.documentVersion &&
      same({ ...saved, title: before.title, documentVersion: before.documentVersion }, before);
  }

  function dismissComment(before, id) {
    if (typeof id !== 'string' || !ID.test(id) || !before.comments.some(value => value.id === id)) fail('comment_invalid');
    return Object.freeze({ content: before.content,
      comments: comments(before.comments.filter(value => value.id !== id), before.content, true) });
  }

  return { version: 3, parse, draft, same, matches, renamed, renameTitle, dismissComment, equalComments, comments, offsets };
});
