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
    if (!payload || typeof payload !== 'object') return null;
    const message = payload.message || payload.data && payload.data.message;
    if (!message || typeof message !== 'object' ||
        !message.author || message.author.role !== 'assistant') return null;
    const text = contentText(message.content).slice(0, MAX_TEXT_LENGTH);
    if (!text) return null;
    const rawStatus = String(message.status || payload.status || '').toLowerCase();
    const completed = /^(completed|finished_successfully|finished)$/.test(rawStatus);
    return {
      id: String(message.id || '').slice(0, 180),
      conversationId: String(payload.conversation_id || payload.conversationId || '').slice(0, 180),
      text,
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
    const content = Array.isArray(message.content) ? message.content.slice() : [];
    const index = content.findIndex((item) => item && (item.type === 'markdown' || item.type === 'text'));
    const part = { type: 'markdown', text: stream.text };
    if (index >= 0) content[index] = Object.assign({}, content[index], part);
    else content.unshift(part);
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
      content: [{ type: 'markdown', text: stream.text }]
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
