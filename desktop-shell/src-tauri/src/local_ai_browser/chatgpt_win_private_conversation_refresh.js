(function () {
  'use strict';

  const VERSION = 2;
  if (location.origin !== 'https://chatgpt.com') return;
  const candidate = window.__elonChatGptPrivateTransport;
  if (!candidate || typeof candidate.prefetchConversation !== 'function') return;

  const existing = window.__elonWinChatGptConversationRefresh;
  if (existing && Number(existing.version || 0) >= VERSION &&
      typeof existing.rebind === 'function') {
    existing.rebind(candidate);
    return;
  }

  let delegate = candidate.__elonWinConversationRefreshWrapped === true &&
    candidate.baseTransport && typeof candidate.baseTransport.prefetchConversation === 'function'
    ? candidate.baseTransport
    : candidate;
  let bindingRevision = 1;
  let wrapper = null;

  function conversationIdentity(path) {
    const match = String(path || '').match(/(?:^|\/)c\/([A-Za-z0-9_-]{1,160})$/);
    return match ? match[1] : '';
  }

  function safeConversationPath(path) {
    const value = String(path || '');
    return conversationIdentity(value) && (
      /^\/c\/[A-Za-z0-9_-]{1,160}$/.test(value) ||
      /^\/g\/g-p-[A-Za-z0-9_-]{1,160}\/c\/[A-Za-z0-9_-]{1,160}$/.test(value)
    ) ? value : '';
  }

  function normalizedContent(message) {
    if (Array.isArray(message && message.content)) return message.content;
    const values = [];
    const direct = typeof (message && message.content) === 'string' ? message.content : '';
    if (direct.trim()) values.push(direct);
    if (Array.isArray(message && message.parts)) {
      message.parts.forEach((part) => {
        const text = typeof part === 'string' ? part :
          (part && typeof part.text === 'string' ? part.text : '');
        if (text.trim() && !values.includes(text)) values.push(text);
      });
    }
    const text = values.join('\n').trim();
    return text ? [{ type: 'markdown', text }] : [];
  }

  function normalizeSnapshot(event, requestedPath) {
    if (!event || event.type !== 'message_snapshot') return event;
    const path = safeConversationPath(requestedPath);
    const richCache = window.__elonWinChatGptConversationRichCache;
    const messages = Array.isArray(event.messages) ? event.messages.map((message) => ({
      ...message,
      content: normalizedContent(message),
    })).map((message) => richCache && typeof richCache.enrichMessage === 'function'
      ? richCache.enrichMessage(message, path)
      : message).filter((message) => message.content.length > 0) : [];
    return {
      ...event,
      url: path ? location.origin + path : event.url,
      messages,
      observedMessageCount: Math.max(Number(event.observedMessageCount) || 0, messages.length),
      messageWindowStart: Math.max(0, Number(event.messageWindowStart) || 0),
      accessSource: 'private_response',
    };
  }

  function normalizedEmitter(emitEvent, requestedPath) {
    if (typeof emitEvent !== 'function') return emitEvent;
    return (event) => emitEvent(normalizeSnapshot(event, requestedPath));
  }

  function prefetchConversation(path, emitEvent, navigate) {
    const requestedPath = safeConversationPath(path);
    if (!requestedPath) return false;
    return delegate.prefetchConversation(
      requestedPath,
      normalizedEmitter(emitEvent, requestedPath),
      navigate,
    );
  }

  function refreshCurrentConversation(path, emitEvent) {
    const requestedPath = safeConversationPath(path);
    if (!requestedPath || conversationIdentity(requestedPath) !== conversationIdentity(location.pathname)) {
      return false;
    }
    const emit = normalizedEmitter(emitEvent, requestedPath);
    if (/^\/c\/[A-Za-z0-9_-]{1,160}$/.test(requestedPath)
        && requestedPath === location.pathname &&
        typeof delegate.refreshCurrentConversation === 'function') {
      return delegate.refreshCurrentConversation(requestedPath, emit);
    }
    // The shared APK transport canonicalizes a project route to /c/{id} before
    // its current-route equality check. Prefetch is the same bounded GET without
    // that lossy equality gate, so Win keeps the official project path here.
    return delegate.prefetchConversation(requestedPath, emit, null);
  }

  function createWrapper() {
    return Object.freeze({
      ...delegate,
      __elonWinConversationRefreshWrapped: true,
      winConversationRefreshVersion: VERSION,
      baseTransport: delegate,
      prefetchConversation,
      refreshCurrentConversation,
    });
  }

  function rebind(nextTransport) {
    if (nextTransport === wrapper) {
      window.__elonChatGptPrivateTransport = wrapper;
      return false;
    }
    const next = nextTransport && nextTransport.__elonWinConversationRefreshWrapped === true &&
      nextTransport.baseTransport &&
      typeof nextTransport.baseTransport.prefetchConversation === 'function'
      ? nextTransport.baseTransport
      : nextTransport;
    if (!next || typeof next.prefetchConversation !== 'function') return false;
    if (next === delegate) {
      window.__elonChatGptPrivateTransport = wrapper;
      return false;
    }
    delegate = next;
    bindingRevision += 1;
    wrapper = createWrapper();
    window.__elonChatGptPrivateTransport = wrapper;
    return true;
  }

  wrapper = createWrapper();
  window.__elonChatGptPrivateTransport = wrapper;
  window.__elonWinChatGptConversationRefresh = Object.freeze({
    version: VERSION,
    rebind,
    diagnostics: function () {
      return 'v' + VERSION + '|bindings=' + bindingRevision;
    },
  });
})();
