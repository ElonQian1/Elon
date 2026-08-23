(function () {
  'use strict';

  const existingTransport = window.__elonChatGptPrivateTransport;
  if ((existingTransport && Number(existingTransport.version) >= 6) ||
      window.__elonChatGptPrivateResearchEnabled !== true ||
      location.origin !== 'https://chatgpt.com') return;

  const MAX_MESSAGES = 80;
  const inheritedHeaders = new Map();
  const delegateFetch = typeof window.fetch === 'function' ? window.fetch.bind(window) : null;
  let privateFetchDepth = 0;

  function requestUrl(input) {
    try {
      return new URL(typeof input === 'string' ? input : input && input.url, location.href);
    } catch (_) {
      return null;
    }
  }

  function isConversationDetail(url) {
    return Boolean(url && url.origin === location.origin &&
      /^\/backend-api\/conversations\/[A-Za-z0-9_-]{1,160}$/.test(url.pathname));
  }

  function headerEntries(input, init) {
    const entries = new Map();
    function read(headers) {
      if (!headers) return;
      try {
        if (typeof headers.forEach === 'function') {
          headers.forEach((value, name) => entries.set(String(name), String(value)));
        } else if (Array.isArray(headers)) {
          headers.forEach((entry) => {
            if (Array.isArray(entry) && entry.length >= 2) entries.set(String(entry[0]), String(entry[1]));
          });
        } else if (typeof headers === 'object') {
          Object.keys(headers).forEach((name) => entries.set(name, String(headers[name])));
        }
      } catch (_) {
        // Browser-managed headers that cannot be read remain browser-managed.
      }
    }
    read(input && input.headers);
    read(init && init.headers);
    return entries;
  }

  if (delegateFetch) {
    window.fetch = function () {
      const input = arguments[0];
      const init = arguments[1] || {};
      if (privateFetchDepth === 0 && isConversationDetail(requestUrl(input))) {
        const entries = headerEntries(input, init);
        if (entries.size) inheritedHeaders.set('conversation_detail', entries);
      }
      return delegateFetch.apply(window, arguments);
    };
  }

  function cleanText(value, maximum) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim()
      .slice(0, maximum);
  }

  function copiedRequestHeaders() {
    const local = inheritedHeaders.get('conversation_detail');
    if (local) return local;
    const probe = window.__elonChatGptPrivateResearchProbe;
    const copied = probe && typeof probe.copyRequestContext === 'function'
      ? probe.copyRequestContext('conversation_detail')
      : null;
    return copied && typeof copied === 'object' && Object.keys(copied).length
      ? new Map(Object.entries(copied))
      : null;
  }

  async function fetchConversation(id) {
    const inherited = copiedRequestHeaders();
    if (!inherited) throw new Error('missing_context');
    const controller = typeof AbortController === 'function' ? new AbortController() : null;
    const timeout = controller ? window.setTimeout(() => controller.abort(), 1500) : null;
    const headers = { Accept: 'application/json' };
    inherited.forEach((value, name) => { headers[name] = value; });
    try {
      privateFetchDepth += 1;
      const response = await window.fetch('/backend-api/conversations/' + encodeURIComponent(id), {
        method: 'GET',
        credentials: 'include',
        cache: 'default',
        headers,
        signal: controller ? controller.signal : undefined,
        __elonPrivateResearch: 'conversation_prefetch'
      });
      if (!response || !response.ok) throw new Error('http_' + Number(response && response.status));
      return await response.json();
    } finally {
      privateFetchDepth = Math.max(0, privateFetchDepth - 1);
      if (timeout !== null) window.clearTimeout(timeout);
    }
  }

  function textParts(content) {
    if (!content || typeof content !== 'object') return [];
    const values = Array.isArray(content.parts) ? content.parts : [];
    return values.map((value) => {
      if (typeof value === 'string') return value;
      if (!value || typeof value !== 'object') return '';
      return typeof value.text === 'string' ? value.text :
        (typeof value.content === 'string' ? value.content : '');
    }).map((value) => String(value || '').trim()).filter(Boolean);
  }

  function messageFrom(node) {
    const message = node && node.message;
    if (!message || typeof message !== 'object') return null;
    const role = cleanText(message.author && message.author.role, 20).toLowerCase();
    if (role !== 'user' && role !== 'assistant') return null;
    const content = textParts(message.content).join('\n').trim().slice(0, 20000);
    if (!content) return null;
    return {
      id: cleanText(message.id || node.id, 160) || role,
      role,
      content,
      state: message.status === 'in_progress' ? 'streaming' : 'completed',
      parts: []
    };
  }

  function conversationMessages(payload) {
    const mapping = payload && payload.mapping;
    if (!mapping || typeof mapping !== 'object') return [];
    const ordered = [];
    const seen = new Set();
    let cursor = cleanText(payload.current_node || payload.currentNode, 180);
    while (cursor && mapping[cursor] && !seen.has(cursor) && ordered.length < MAX_MESSAGES * 3) {
      seen.add(cursor);
      ordered.push(mapping[cursor]);
      cursor = cleanText(mapping[cursor].parent, 180);
    }
    const nodes = ordered.length
      ? ordered.reverse()
      : Object.values(mapping).sort((left, right) =>
        Number(left && left.message && left.message.create_time || 0) -
        Number(right && right.message && right.message.create_time || 0)
      );
    return nodes.map(messageFrom).filter(Boolean).slice(-MAX_MESSAGES);
  }

  function conversationPrefetchReady() {
    return Boolean(copiedRequestHeaders());
  }

  function prefetchConversation(path, emitEvent, navigate) {
    const match = String(path || '').match(/\/c\/([A-Za-z0-9_-]{1,160})$/);
    if (!match) return false;
    if (!conversationPrefetchReady()) return false;
    fetchConversation(match[1]).then((payload) => {
      const messages = conversationMessages(payload);
      if (!messages.length) return;
      emitEvent({
        type: 'message_snapshot',
        title: cleanText(payload && payload.title, 120),
        url: location.origin + path,
        draft: '',
        messages,
        observedMessageCount: messages.length,
        messageWindowStart: 0,
        authenticated: true,
        pageKind: 'conversation',
        loginRequired: false,
        composerReady: false,
        streaming: false,
        currentModel: cleanText(payload && (payload.default_model_slug || payload.model), 80),
        attachments: [],
        dictationActive: false,
        capabilities: ['conversation_history']
      });
    }).catch(() => undefined).finally(navigate);
    return true;
  }

  window.__elonChatGptPrivateTransport = Object.freeze({
    version: 6,
    conversationPrefetchEnabled: false,
    experimentalConversationPrefetchAvailable: true,
    conversationPrefetchReady,
    prefetchConversation
  });
})();
