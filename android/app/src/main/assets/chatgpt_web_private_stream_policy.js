(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module === 'object' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptPrivateStreamPolicy = Object.freeze(policy);
})(typeof window === 'object' ? window : null, function () {
  'use strict';

  const MAX_TEXT_LENGTH = 40000;
  const MAX_BUFFER_LENGTH = 524288;
  const MAX_AGE_MS = 5 * 60 * 1000;

  function cleanText(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\n{3,}/g, '\n\n').trim();
  }

  function compactEnvelope(payload) {
    if (!payload || typeof payload !== 'object') return payload;
    return Number.isFinite(payload.c) && payload.v && typeof payload.v === 'object'
      ? payload.v
      : payload;
  }

  function publicSourceUrl(value) {
    try {
      const url = new URL(String(value || ''));
      if (!/^https?:$/.test(url.protocol) || url.username || url.password) return '';
      url.search = '';
      url.hash = '';
      return url.toString();
    } catch (_) {
      return '';
    }
  }

  function sourceHost(value) {
    try { return new URL(value).hostname.toLowerCase().replace(/^www\./, ''); }
    catch (_) { return ''; }
  }

  function sourceLabel(item) {
    return cleanText(item && (item.attribution || item.title)).replace(/[\[\]()]/g, '').slice(0, 80);
  }

  function citationRecords(metadata) {
    const references = metadata && Array.isArray(metadata.content_references)
      ? metadata.content_references
      : [];
    return references.slice(0, 32).flatMap((reference, referenceIndex) => {
      if (!reference || reference.type !== 'grouped_webpages' || !Array.isArray(reference.items)) return [];
      const primary = reference.items[0];
      const url = publicSourceUrl(primary && primary.url);
      const label = sourceLabel(primary);
      if (!url || !label) return [];
      const refs = primary && Array.isArray(primary.refs) ? primary.refs.length : 0;
      const supporting = primary && Array.isArray(primary.supporting_websites)
        ? primary.supporting_websites.length
        : 0;
      const groupSize = Math.max(1, refs, supporting + 1);
      return [{
        start: Number(reference.start_idx),
        end: Number(reference.end_idx),
        matchedText: String(reference.matched_text || ''),
        part: {
          type: 'citation',
          text: label,
          url,
          markerText: label + (groupSize > 1 ? ' +' + (groupSize - 1) : ''),
          citationId: 'private-ref-' + referenceIndex,
          groupSize,
          targetHost: sourceHost(url)
        }
      }];
    });
  }

  function linkedText(value, citations) {
    let text = String(value || '');
    const replacements = citations.map((citation) => {
      let start = citation.start;
      let end = citation.end;
      const marker = citation.matchedText;
      const exact = Number.isInteger(start) && Number.isInteger(end) && start >= 0 && end >= start &&
        end <= text.length && text.slice(start, end) === marker;
      if (!exact) {
        start = marker && text.indexOf(marker) === text.lastIndexOf(marker) ? text.indexOf(marker) : -1;
        end = start >= 0 ? start + marker.length : -1;
      }
      return { citation, start, end };
    }).filter((item) => item.start >= 0 && item.end > item.start)
      .sort((left, right) => right.start - left.start);
    replacements.forEach(({ citation, start, end }) => {
      const label = citation.part.markerText.replace(/[\[\]()]/g, '');
      text = text.slice(0, start) + '[' + label + '](' + citation.part.url + ')' + text.slice(end);
    });
    return cleanText(text);
  }

  function contentText(content) {
    if (!content || typeof content !== 'object') return '';
    if (Array.isArray(content.parts)) {
      return cleanText(content.parts.map((part) => {
        if (typeof part === 'string') return part;
        if (!part || typeof part !== 'object') return '';
        return typeof part.text === 'string' ? part.text :
          (typeof part.content === 'string' ? part.content : '');
      }).filter(Boolean).join('\n'));
    }
    if (typeof content.text === 'string') return cleanText(content.text);
    if (typeof content.content === 'string') return cleanText(content.content);
    return '';
  }

  function assistantFrame(payload) {
    const envelope = compactEnvelope(payload);
    if (!envelope || typeof envelope !== 'object') return null;
    const message = envelope.message || envelope.data && envelope.data.message;
    if (!message || typeof message !== 'object' ||
        !message.author || message.author.role !== 'assistant') return null;
    const contentType = String(message.content && message.content.content_type || '').toLowerCase();
    if (contentType && contentType !== 'text') return null;
    const citations = citationRecords(message.metadata);
    const text = linkedText(contentText(message.content), citations).slice(0, MAX_TEXT_LENGTH);
    if (!text) return null;
    const rawStatus = String(message.status || envelope.status || '').toLowerCase();
    const completed = /^(completed|finished_successfully|finished)$/.test(rawStatus);
    return {
      id: String(message.id || '').slice(0, 180),
      conversationId: String(envelope.conversation_id || envelope.conversationId || '').slice(0, 180),
      text,
      citations: citations.map((citation) => citation.part),
      state: completed ? 'completed' : 'streaming'
    };
  }

  function createSseDecoder(onPayload, onDone) {
    let buffer = '';
    let closed = false;

    function processEvent(rawEvent) {
      const data = String(rawEvent || '').split(/\r?\n/)
        .filter((line) => line.startsWith('data:'))
        .map((line) => line.slice(5).trimStart())
        .join('\n')
        .trim();
      if (!data) return;
      if (data === '[DONE]') {
        closed = true;
        if (typeof onDone === 'function') onDone();
        return;
      }
      try {
        const payload = JSON.parse(data);
        if (typeof onPayload === 'function') onPayload(payload);
      } catch (_) {
        // Unknown frames remain owned by the official page and are ignored.
      }
    }

    function push(chunk) {
      if (closed || !chunk) return;
      buffer += String(chunk);
      if (buffer.length > MAX_BUFFER_LENGTH) {
        buffer = buffer.slice(-MAX_BUFFER_LENGTH);
      }
      let boundary = buffer.search(/\r?\n\r?\n/);
      while (boundary >= 0) {
        const separator = buffer.slice(boundary).match(/^\r?\n\r?\n/)[0];
        const event = buffer.slice(0, boundary);
        buffer = buffer.slice(boundary + separator.length);
        processEvent(event);
        if (closed) return;
        boundary = buffer.search(/\r?\n\r?\n/);
      }
    }

    function finish() {
      if (closed) return;
      if (buffer.trim()) processEvent(buffer);
      buffer = '';
      if (!closed && typeof onDone === 'function') onDone();
      closed = true;
    }

    return Object.freeze({ push, finish });
  }

  function conversationMatches(pathname, conversationId) {
    const path = String(pathname || '');
    if (!conversationId || path === '/' || path === '') return true;
    const match = path.match(/^\/c\/([^/?#]+)/);
    return !match || match[1] === conversationId;
  }

  function primaryText(message) {
    const content = message && Array.isArray(message.content) ? message.content : [];
    const part = content.find((item) => item && (item.type === 'markdown' || item.type === 'text'));
    return cleanText(part && part.text);
  }

  function mergedMessage(message, stream) {
    const content = Array.isArray(message.content)
      ? message.content.filter((part) => part && part.type !== 'citation').slice()
      : [];
    const index = content.findIndex((item) => item && (item.type === 'markdown' || item.type === 'text'));
    const part = { type: 'markdown', text: stream.text };
    if (index >= 0) content[index] = Object.assign({}, content[index], part);
    else content.unshift(part);
    (stream.citations || []).forEach((citation) => content.push(Object.assign({}, citation)));
    return Object.assign({}, message, { state: stream.state, content });
  }

  function mergeMessages(messages, stream) {
    const values = Array.isArray(messages) ? messages : [];
    if (!stream || !stream.text) return values;
    let assistantIndex = -1;
    for (let index = values.length - 1; index >= 0; index -= 1) {
      if (values[index] && values[index].role === 'assistant') {
        assistantIndex = index;
        break;
      }
    }
    const assistant = assistantIndex >= 0 ? values[assistantIndex] : null;
    const text = primaryText(assistant);
    const sameMessage = !!assistant && (
      (stream.id && (assistant.id === stream.id || assistant.id === 'private-stream:' + stream.id)) ||
      (text && (stream.text.startsWith(text) || text.startsWith(stream.text)))
    );
    if (sameMessage) {
      if (text.length > stream.text.length) {
        stream = Object.assign({}, stream, { text });
      }
      const result = values.slice();
      result[assistantIndex] = mergedMessage(assistant, stream);
      return result;
    }
    return values.concat([{
      id: stream.id ? 'private-stream:' + stream.id : 'private-stream:assistant',
      role: 'assistant',
      state: stream.state,
      content: [{ type: 'markdown', text: stream.text }].concat(
        (stream.citations || []).map((citation) => Object.assign({}, citation))
      )
    }]);
  }

  function createSession(options) {
    const now = typeof options.now === 'function' ? options.now : Date.now;
    let stream = null;

    function begin() {
      stream = { id: '', conversationId: '', text: '', state: 'streaming', updatedAt: now() };
    }

    function accept(payload) {
      const frame = assistantFrame(payload);
      if (!frame) return false;
      if (!stream) begin();
      stream = Object.assign({}, stream, frame, { updatedAt: now() });
      return true;
    }

    function finish() {
      if (!stream || !stream.text) return false;
      stream = Object.assign({}, stream, { state: 'completed', updatedAt: now() });
      return true;
    }

    function reset() {
      stream = null;
    }

    function current(pathname) {
      if (!stream || !stream.text || now() - stream.updatedAt > MAX_AGE_MS ||
          !conversationMatches(pathname, stream.conversationId)) return null;
      return Object.assign({}, stream);
    }

    function merge(values, pathname) {
      const active = current(pathname);
      if (!active) return values;
      const result = mergeMessages(values, active);
      if (active.state === 'completed') {
        const lastAssistant = Array.from(values || []).reverse()
          .find((message) => message && message.role === 'assistant');
        if (primaryText(lastAssistant).length >= active.text.length) reset();
      }
      return result;
    }

    return Object.freeze({ begin, accept, finish, reset, current, merge });
  }

  return Object.freeze({ assistantFrame, createSession, createSseDecoder, mergeMessages });
});
