(function () {
  'use strict';

  const existingTransport = window.__elonChatGptPrivateTransport;
  const prefetchEnabled = window.__elonChatGptPrivateConversationPrefetchEnabled === true;
  const researchEnabled = window.__elonChatGptPrivateResearchEnabled === true;
  if ((existingTransport && Number(existingTransport.version) >= 10) ||
      (!prefetchEnabled && !researchEnabled) ||
      location.origin !== 'https://chatgpt.com') return;

  const policyModule = window.__elonChatGptPrivateTransportPolicy;
  if (!policyModule || typeof policyModule.create !== 'function') return;
  const MAX_MESSAGES = 80;
  const inheritedHeaders = new Map();
  const delegateFetch = typeof window.fetch === 'function' ? window.fetch.bind(window) : null;
  let privateFetchDepth = 0;

  function optionalSessionStorage() {
    try {
      return window.sessionStorage || null;
    } catch (_) {
      return null;
    }
  }

  const policy = policyModule.create({
    enabled: prefetchEnabled,
    now: Date.now,
    storage: optionalSessionStorage()
  });

  function requestUrl(input) {
    try {
      return new URL(typeof input === 'string' ? input : input && input.url, location.href);
    } catch (_) {
      return null;
    }
  }

  function isConversationContent(url) {
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
      const officialDetail = privateFetchDepth === 0 && isConversationContent(requestUrl(input));
      const startedAt = officialDetail ? Date.now() : 0;
      if (officialDetail) {
        const entries = headerEntries(input, init);
        if (entries.size) inheritedHeaders.set('conversation_content', entries);
      }
      let result;
      try {
        result = delegateFetch.apply(window, arguments);
      } catch (error) {
        if (officialDetail) policy.recordOfficial(0, Date.now() - startedAt);
        throw error;
      }
      if (!officialDetail) return result;
      return Promise.resolve(result).then(
        (response) => {
          policy.recordOfficial(Number(response && response.status), Date.now() - startedAt);
          return response;
        },
        (error) => {
          policy.recordOfficial(0, Date.now() - startedAt);
          throw error;
        }
      );
    };
  }

  function cleanText(value, maximum) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim()
      .slice(0, maximum);
  }

  function copiedRequestHeaders() {
    const local = inheritedHeaders.get('conversation_content');
    if (local) return local;
    const probe = window.__elonChatGptPrivateResearchProbe;
    const copied = probe && typeof probe.copyRequestContext === 'function'
      ? probe.copyRequestContext('conversation_content')
      : null;
    return copied && typeof copied === 'object' && Object.keys(copied).length
      ? new Map(Object.entries(copied))
      : null;
  }

  async function fetchConversation(id) {
    const inherited = copiedRequestHeaders();
    if (!inherited) throw new Error('missing_context');
    const startedAt = Date.now();
    let timedOut = false;
    const controller = typeof AbortController === 'function' ? new AbortController() : null;
    const timeout = controller ? window.setTimeout(() => {
      timedOut = true;
      controller.abort();
    }, policy.attemptBudgetMs()) : null;
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
        __elonPrivateTransport: 'conversation_prefetch'
      });
      if (!response || !response.ok) throw new Error('http_' + Number(response && response.status));
      return { payload: await response.json(), elapsedMs: Date.now() - startedAt };
    } catch (error) {
      if (timedOut) throw new Error('timeout');
      throw error;
    } finally {
      privateFetchDepth = Math.max(0, privateFetchDepth - 1);
      if (timeout !== null) window.clearTimeout(timeout);
    }
  }

  function textParts(content) {
    if (typeof content === 'string') return [content];
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
    const message = node && (node.message || node);
    if (!message || typeof message !== 'object') return null;
    const role = cleanText(message.author && message.author.role || message.role, 20).toLowerCase();
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
    if (!mapping || typeof mapping !== 'object') {
      const arrays = [
        payload && payload.messages,
        payload && payload.linear_conversation,
        payload && payload.items,
        payload && payload.data && payload.data.messages,
        payload && payload.data && payload.data.linear_conversation,
        payload && payload.result && payload.result.messages
      ];
      const messages = arrays.find(Array.isArray) || [];
      return messages.map(messageFrom).filter(Boolean).slice(-MAX_MESSAGES);
    }
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

  function normalizedConversationPayload(payload) {
    const values = [
      payload,
      payload && payload.conversation,
      payload && payload.data,
      payload && payload.data && payload.data.conversation,
      payload && payload.result,
      payload && payload.result && payload.result.conversation
    ];
    return values.find((value) => value && typeof value === 'object' &&
      value.mapping && typeof value.mapping === 'object') || payload;
  }

  function conversationPrefetchReady() {
    return policy.canAttempt(Boolean(copiedRequestHeaders()));
  }

  function failureKind(error) {
    const message = String(error && error.message || 'network');
    if (message === 'timeout') return 'timeout';
    if (message === 'missing_context') return 'context';
    if (/^http_(401|403)$/.test(message)) return 'auth';
    if (/^http_/.test(message)) return 'http';
    if (/json|parse/i.test(message)) return 'parse';
    return 'network';
  }

  function recordPrivateOutcome(outcome, messageCount, elapsedMs) {
    const probe = window.__elonChatGptPrivateResearchProbe;
    if (probe && typeof probe.recordPrivateOutcome === 'function') {
      probe.recordPrivateOutcome(outcome, messageCount, elapsedMs);
    }
  }

  function recordPrivatePayloadShape(payload) {
    const probe = window.__elonChatGptPrivateResearchProbe;
    if (probe && typeof probe.recordPrivatePayloadShape === 'function') {
      probe.recordPrivatePayloadShape(payload);
    }
  }

  function prefetchConversation(path, emitEvent, navigate) {
    const match = String(path || '').match(/\/c\/([A-Za-z0-9_-]{1,160})$/);
    if (!match) return false;
    if (!conversationPrefetchReady()) return false;
    fetchConversation(match[1]).then((result) => {
      recordPrivatePayloadShape(result.payload);
      const payload = normalizedConversationPayload(result.payload);
      const messages = conversationMessages(payload);
      if (!messages.length) {
        policy.recordFailure('empty');
        recordPrivateOutcome('empty', 0, result.elapsedMs);
        return;
      }
      policy.recordSuccess(result.elapsedMs);
      recordPrivateOutcome('success', messages.length, result.elapsedMs);
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
    }).catch((error) => {
      const outcome = failureKind(error);
      policy.recordFailure(outcome);
      recordPrivateOutcome(outcome, 0, 0);
    }).finally(navigate);
    return true;
  }

  window.__elonChatGptPrivateTransport = Object.freeze({
    version: 10,
    conversationPrefetchEnabled: prefetchEnabled,
    conversationPrefetchAvailable: true,
    experimentalConversationPrefetchAvailable: true,
    conversationPrefetchReady,
    prefetchConversation,
    health: policy.snapshot
  });
})();
