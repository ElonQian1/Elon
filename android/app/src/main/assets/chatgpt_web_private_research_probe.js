(function () {
  'use strict';

  const legacyEnabled = window.__elonChatGptPrivateResearchEnabled === true;
  if (location.origin !== 'https://chatgpt.com') return;
  const existingProbe = window.__elonChatGptPrivateResearchProbe;
  if (existingProbe && Number(existingProbe.version) >= 12) return;

  const nativeBridge = window.elonChatGptNative;
  const adapterVersion = Number(window.__elonChatGptAdapterTargetVersion || 0);
  const documentToken = String(window.__elonChatGptDocumentToken || '');
  if (!nativeBridge || typeof nativeBridge.postMessage !== 'function') return;
  if (!/^doc_[a-z0-9_]{3,80}$/.test(documentToken)) return;

  const startedAt = Date.now();
  const expiresAt = startedAt + (10 * 60 * 1000);
  const maxObservations = 200;
  let observationCount = 0;
  let privateObservationCount = 0;
  const privateStreamShapes = new Set();
  const requestContexts = new Map();

  function nowMs() {
    return window.performance && typeof window.performance.now === 'function'
      ? window.performance.now()
      : Date.now();
  }

  function endpointCandidate(url) {
    return url.origin === location.origin && (
      url.pathname.startsWith('/backend-api/') ||
      url.pathname.startsWith('/api/') ||
      url.pathname.startsWith('/ces/')
    );
  }

  function safeSegment(segment) {
    if (!segment) return '';
    if (segment.includes('%')) return '{segment}';
    if (/^[0-9a-f]{8}-[0-9a-f-]{20,}$/i.test(segment)) return '{id}';
    if (/^[0-9]{7,}$/.test(segment)) return '{id}';
    if (/^[A-Za-z0-9_-]{17,}$/.test(segment)) return '{id}';
    if (!/^[A-Za-z0-9._-]{1,40}$/.test(segment)) return '{segment}';
    return segment;
  }

  function safePath(url) {
    const value = url.pathname
      .split('/')
      .map(safeSegment)
      .join('/')
      .slice(0, 96);
    return value || '/';
  }

  const evidence = window.__elonChatGptPrivateProtocolEvidence?.create(window, safePath);
  if (!legacyEnabled && !evidence) return;

  function responseKind(response) {
    if (!response || !response.headers || typeof response.headers.get !== 'function') return 'unknown';
    const contentType = String(response.headers.get('content-type') || '').toLowerCase();
    if (contentType.includes('text/event-stream')) return 'sse';
    if (contentType.includes('json')) return 'json';
    if (contentType.startsWith('text/')) return 'text';
    return contentType ? 'other' : 'unknown';
  }

  function safeKeys(value, maximum) {
    if (!value || typeof value !== 'object') return [];
    const limit = Math.max(1, Math.min(80, Number(maximum) || 24));
    return Object.keys(value)
      .filter((key) => /^[A-Za-z][A-Za-z0-9_]{0,39}$/.test(key))
      .sort()
      .slice(0, limit);
  }

  function safeHeaderNames(headers) {
    if (!headers) return [];
    const names = [];
    try {
      if (typeof headers.forEach === 'function') {
        headers.forEach((_, name) => names.push(String(name).toLowerCase()));
      } else if (Array.isArray(headers)) {
        headers.forEach((entry) => names.push(String(entry && entry[0] || '').toLowerCase()));
      } else if (typeof headers === 'object') {
        Object.keys(headers).forEach((name) => names.push(String(name).toLowerCase()));
      }
    } catch (_) {
      return [];
    }
    return Array.from(new Set(names))
      .filter((name) => /^[a-z][a-z0-9-]{0,50}$/.test(name))
      .sort()
      .slice(0, 16);
  }

  function safeValueTypes(value) {
    return safeKeys(value, 24).map((key) => {
      const current = value[key];
      const type = Array.isArray(current)
        ? 'array'
        : current === null ? 'null' : typeof current;
      return key + ':' + (/^(?:array|null|boolean|number|string|object)$/.test(type)
        ? type
        : 'other');
    });
  }

  function mutationCandidate(url, method) {
    const verb = String(method || 'GET').toUpperCase();
    if (!/^(?:POST|PATCH|PUT|DELETE)$/.test(verb)) return false;
    return /^\/backend-api\/conversation\/[A-Za-z0-9_-]{1,160}(?:\/[A-Za-z0-9_-]{1,40})?$/.test(
      url.pathname
    ) || /^\/backend-api\/gizmos\/g-p-[A-Za-z0-9_-]{1,160}\/conversations$/.test(
      url.pathname
    );
  }

  function observeMutationBody(url, body) {
    if (typeof body !== 'string' || !body || body.length > 65536) return;
    try {
      const parsed = JSON.parse(body);
      emitShape('mutation_body', url, safeKeys(parsed, 24));
      emitShape('mutation_types', url, safeValueTypes(parsed));
    } catch (_) {
      // Only JSON key names and primitive types are eligible for research output.
    }
  }

  function requestFamily(url) {
    if (/^\/backend-api\/conversations\/[A-Za-z0-9_-]{1,160}$/.test(url.pathname)) {
      return 'conversation_content';
    }
    if (/^\/backend-api\/gizmos\/g-p-[A-Za-z0-9_-]{1,160}\/conversations$/.test(url.pathname)) {
      return 'project_conversations';
    }
    if (url.pathname === '/backend-api/gizmos/snorlax/sidebar') return 'project_sidebar';
    return '';
  }

  function captureRequestContext(input, init, url) {
    const family = requestFamily(url);
    if (!family) return;
    const values = {};
    function read(headers) {
      if (!headers) return;
      try {
        if (typeof headers.forEach === 'function') {
          headers.forEach((value, name) => { values[String(name)] = String(value); });
        } else if (Array.isArray(headers)) {
          headers.forEach((entry) => {
            if (Array.isArray(entry) && entry.length >= 2) values[String(entry[0])] = String(entry[1]);
          });
        } else if (typeof headers === 'object') {
          Object.keys(headers).forEach((name) => { values[name] = String(headers[name]); });
        }
      } catch (_) {
        // Browser-managed values that cannot be read are intentionally ignored.
      }
    }
    read(input && input.headers);
    read(init && init.headers);
    if (Object.keys(values).length) requestContexts.set(family, values);
  }

  function emitShape(kind, url, names) {
    if (!legacyEnabled) return;
    if (Date.now() > expiresAt || observationCount >= maxObservations) return;
    if (!endpointCandidate(url) || !names.length) return;
    observationCount += 1;
    nativeBridge.postMessage(JSON.stringify({
      type: 'command_result',
      adapterVersion,
      documentToken,
      action: 'research_network_observation',
      ok: true,
      detail: ['v1', kind, safePath(url), names.join('.')].join('|').slice(0, 160)
    }));
  }

  function observeRequestShape(input, init, url, method) {
    captureRequestContext(input, init, url);
    const headers = safeHeaderNames(init && init.headers || input && input.headers);
    if (mutationCandidate(url, method)) {
      emitShape('headers', url, headers);
      if (init && init.body != null) {
        observeMutationBody(url, init.body);
      } else if (input && typeof input.clone === 'function') {
        try {
          Promise.resolve(input.clone().text())
            .then((body) => observeMutationBody(url, body))
            .catch(() => {});
        } catch (_) {}
      }
      return;
    }
    if (/^\/backend-api\/[A-Za-z0-9_-]{17,160}$/.test(url.pathname) ||
        /^\/backend-api\/conversations\/[A-Za-z0-9_-]{1,160}$/.test(url.pathname) ||
        /^\/backend-api\/gizmos\/.+\/conversations$/.test(url.pathname) ||
        url.pathname === '/backend-api/gizmos/snorlax/sidebar') {
      emitShape('headers', url, headers);
      return;
    }
    if (url.pathname !== '/backend-api/f/conversation') return;
    emitShape('headers', url, headers);
    const body = init && init.body;
    if (typeof body !== 'string' || body.length > 262144) return;
    try {
      const parsed = JSON.parse(body);
      emitShape('body', url, safeKeys(parsed));
      const message = Array.isArray(parsed.messages) ? parsed.messages[0] : null;
      emitShape('message', url, safeKeys(message));
      emitShape('content', url, safeKeys(message && message.content));
    } catch (_) {
      // A non-JSON body is intentionally not inspected or reported.
    }
  }

  function emit(transport, method, url, status, kind, elapsedMs) {
    if (!legacyEnabled) return;
    if (Date.now() > expiresAt || observationCount >= maxObservations) return;
    if (!endpointCandidate(url)) return;
    observationCount += 1;
    const fields = [
      'v1',
      transport,
      String(method || 'GET').toUpperCase().replace(/[^A-Z]/g, '').slice(0, 10) || 'GET',
      safePath(url),
      String(Number.isFinite(status) ? Math.max(0, Math.min(999, Math.round(status))) : 0),
      String(kind || 'unknown').replace(/[^a-z]/g, '').slice(0, 10) || 'unknown',
      String(Math.max(0, Math.min(999999, Math.round(elapsedMs || 0))))
    ];
    nativeBridge.postMessage(JSON.stringify({
      type: 'command_result',
      adapterVersion,
      documentToken,
      action: 'research_network_observation',
      ok: true,
      detail: fields.join('|').slice(0, 160)
    }));
  }

  function emitPrivate(kind, method, url, status, responseType, elapsedMs) {
    if (!legacyEnabled) return;
    if (Date.now() > expiresAt || privateObservationCount >= 64) return;
    if (kind !== 'conversation_prefetch' || !endpointCandidate(url)) return;
    privateObservationCount += 1;
    nativeBridge.postMessage(JSON.stringify({
      type: 'command_result',
      adapterVersion,
      documentToken,
      action: 'research_network_observation',
      ok: true,
      detail: [
        'v1',
        'private_prefetch',
        String(method || 'GET').toUpperCase().replace(/[^A-Z]/g, '').slice(0, 10) || 'GET',
        safePath(url),
        String(Math.max(0, Math.min(999, Math.round(status || 0)))),
        String(responseType || 'unknown').replace(/[^a-z]/g, '').slice(0, 10) || 'unknown',
        String(Math.max(0, Math.min(999999, Math.round(elapsedMs || 0))))
      ].join('|').slice(0, 160)
    }));
  }

  function emitPrivateOutcome(outcome, messageCount, elapsedMs) {
    if (!legacyEnabled) return;
    if (Date.now() > expiresAt || privateObservationCount >= 64) return;
    const safeOutcome = String(outcome || '').toLowerCase();
    if (!/^(success|empty|timeout|auth|context|http|network|parse)$/.test(safeOutcome)) return;
    privateObservationCount += 1;
    nativeBridge.postMessage(JSON.stringify({
      type: 'command_result',
      adapterVersion,
      documentToken,
      action: 'research_network_observation',
      ok: true,
      detail: [
        'v1',
        'private_outcome',
        safeOutcome,
        String(Math.max(0, Math.min(80, Math.round(messageCount || 0)))),
        String(Math.max(0, Math.min(999999, Math.round(elapsedMs || 0))))
      ].join('|')
    }));
  }

  function emitPrivateStreamOutcome(outcome, frameCount, elapsedMs) {
    if (!legacyEnabled) return;
    if (Date.now() > expiresAt || privateObservationCount >= 64) return;
    const safeOutcome = String(outcome || '').toLowerCase();
    if (!/^(first|success|empty|error)$/.test(safeOutcome)) return;
    privateObservationCount += 1;
    nativeBridge.postMessage(JSON.stringify({
      type: 'command_result',
      adapterVersion,
      documentToken,
      action: 'research_network_observation',
      ok: true,
      detail: [
        'v1',
        'private_stream',
        safeOutcome,
        String(Math.max(0, Math.min(9999, Math.round(frameCount || 0)))),
        String(Math.max(0, Math.min(999999, Math.round(elapsedMs || 0))))
      ].join('|')
    }));
  }

  function emitPrivateStreamShape(shape) {
    if (!legacyEnabled) return;
    if (Date.now() > expiresAt || privateObservationCount >= 64 ||
        privateStreamShapes.size >= 16) return;
    const safeShape = String(shape || '').toLowerCase();
    if (!/^[a-z0-9._:{}\/|-]{1,120}$/.test(safeShape) || privateStreamShapes.has(safeShape)) return;
    privateStreamShapes.add(safeShape);
    privateObservationCount += 1;
    nativeBridge.postMessage(JSON.stringify({
      type: 'command_result',
      adapterVersion,
      documentToken,
      action: 'research_network_observation',
      ok: true,
      detail: ['v1', 'private_stream_shape', safeShape].join('|')
    }));
  }

  function emitPrivatePayloadShape(payload) {
    if (!legacyEnabled) return;
    if (Date.now() > expiresAt || privateObservationCount >= 32) return;
    const candidates = [
      ['root', payload],
      ['conversation', payload && payload.conversation],
      ['data', payload && payload.data],
      ['data_conversation', payload && payload.data && payload.data.conversation],
      ['result', payload && payload.result],
      ['result_conversation', payload && payload.result && payload.result.conversation]
    ];
    const selected = candidates.find((entry) => entry[1] && typeof entry[1] === 'object' &&
      entry[1].mapping && typeof entry[1].mapping === 'object');
    const arrayCandidates = [
      ['root_messages', payload && payload.messages],
      ['root_linear', payload && payload.linear_conversation],
      ['root_items', payload && payload.items],
      ['data_messages', payload && payload.data && payload.data.messages],
      ['data_linear', payload && payload.data && payload.data.linear_conversation],
      ['result_messages', payload && payload.result && payload.result.messages]
    ];
    const selectedArray = arrayCandidates.find((entry) => Array.isArray(entry[1]));
    const mapping = selected ? selected[1].mapping : null;
    const nodes = mapping ? Object.values(mapping) : (selectedArray ? selectedArray[1] : []);
    const messages = nodes.map((node) => node && (
      mapping ? node.message : (node.message || node)
    )).filter(Boolean);
    const roleMessages = messages.filter((message) =>
      message.author && (message.author.role === 'user' || message.author.role === 'assistant'));
    const stringPartMessages = roleMessages.filter((message) =>
      message.content && Array.isArray(message.content.parts) &&
      message.content.parts.some((part) => typeof part === 'string'));
    const objectPartMessages = roleMessages.filter((message) =>
      message.content && Array.isArray(message.content.parts) &&
      message.content.parts.some((part) => part && typeof part === 'object'));
    const directTextMessages = roleMessages.filter((message) => message.content && (
      typeof message.content.text === 'string' || typeof message.content.content === 'string'));
    const rootKeys = safeKeys(payload, 60);
    for (let index = 0; index < rootKeys.length && privateObservationCount < 32; index += 10) {
      privateObservationCount += 1;
      nativeBridge.postMessage(JSON.stringify({
        type: 'command_result',
        adapterVersion,
        documentToken,
        action: 'research_network_observation',
        ok: true,
        detail: ['v1', 'private_keys', String(index / 10), rootKeys.slice(index, index + 10).join('.')]
          .join('|').slice(0, 160)
      }));
    }
    if (privateObservationCount >= 32) return;
    privateObservationCount += 1;
    nativeBridge.postMessage(JSON.stringify({
      type: 'command_result',
      adapterVersion,
      documentToken,
      action: 'research_network_observation',
      ok: true,
      detail: [
        'v1',
        'private_shape',
        selected ? selected[0] : (selectedArray ? selectedArray[0] : 'none'),
        String(Math.min(9999, nodes.length)),
        String(Math.min(9999, messages.length)),
        String(Math.min(9999, roleMessages.length)),
        String(Math.min(9999, stringPartMessages.length)),
        String(Math.min(9999, objectPartMessages.length)),
        String(Math.min(9999, directTextMessages.length))
      ].join('|')
    }));
  }

  const originalFetch = typeof window.fetch === 'function' ? window.fetch : null;
  if (originalFetch) {
    window.fetch = function () {
      const args = arguments;
      if (!legacyEnabled && !evidence?.active()) return originalFetch.apply(this, args);
      const input = args[0];
      const init = args[1] || {};
      let url;
      try {
        url = new URL(typeof input === 'string' ? input : input.url, location.href);
      } catch (_) {
        return originalFetch.apply(this, args);
      }
      if (!endpointCandidate(url)) return originalFetch.apply(this, args);
      const method = init.method || (input && input.method) || 'GET';
      const privateKind = String(
        init.__elonPrivateTransport || init.__elonPrivateResearch || ''
      );
      if (legacyEnabled) observeRequestShape(input, init, url, method);
      let observation = null;
      try { observation = evidence?.begin(input, init, url, method, 'fetch'); } catch (_) {}
      const requestStartedAt = nowMs();
      try {
        return Promise.resolve(originalFetch.apply(this, args)).then(
          (response) => {
            try { evidence?.response(observation, response); } catch (_) {}
            if (legacyEnabled) emit('fetch', method, url, response.status, responseKind(response), nowMs() - requestStartedAt);
            if (legacyEnabled) emitPrivate(
              privateKind,
              method,
              url,
              response.status,
              responseKind(response),
              nowMs() - requestStartedAt
            );
            return response;
          },
          (error) => {
            try { evidence?.xhrResponse(observation, 0, '', null); } catch (_) {}
            if (legacyEnabled) emit('fetch', method, url, 0, 'error', nowMs() - requestStartedAt);
            if (legacyEnabled) emitPrivate(privateKind, method, url, 0, 'error', nowMs() - requestStartedAt);
            throw error;
          }
        );
      } catch (error) {
        try { evidence?.xhrResponse(observation, 0, '', null); } catch (_) {}
        if (legacyEnabled) emit('fetch', method, url, 0, 'error', nowMs() - requestStartedAt);
        if (legacyEnabled) emitPrivate(privateKind, method, url, 0, 'error', nowMs() - requestStartedAt);
        throw error;
      }
    };
  }

  const xhrPrototype = window.XMLHttpRequest && window.XMLHttpRequest.prototype;
  const originalOpen = xhrPrototype && xhrPrototype.open;
  const originalSend = xhrPrototype && xhrPrototype.send;
  const xhrMetadata = new WeakMap();
  if (originalOpen && originalSend) {
    xhrPrototype.open = function (method, rawUrl) {
      if (!legacyEnabled && !evidence?.active()) {
        xhrMetadata.delete(this);
        return originalOpen.apply(this, arguments);
      }
      try {
        xhrMetadata.set(this, { method, url: new URL(rawUrl, location.href) });
      } catch (_) {
        xhrMetadata.delete(this);
      }
      return originalOpen.apply(this, arguments);
    };
    xhrPrototype.send = function () {
      const metadata = xhrMetadata.get(this);
      if ((legacyEnabled || evidence?.active()) && metadata && endpointCandidate(metadata.url)) {
        if (legacyEnabled && mutationCandidate(metadata.url, metadata.method)) {
          observeMutationBody(metadata.url, arguments[0]);
        }
        let observation = null;
        try { observation = evidence?.begin(null, { body: arguments[0] }, metadata.url, metadata.method, 'xhr'); } catch (_) {}
        const requestStartedAt = nowMs();
        this.addEventListener('loadend', () => {
          const contentType = String(this.getResponseHeader('content-type') || '').toLowerCase();
          const kind = contentType.includes('text/event-stream')
            ? 'sse'
            : contentType.includes('json') ? 'json' : contentType.startsWith('text/') ? 'text' : 'unknown';
          try {
            evidence?.xhrResponse(observation, this.status, contentType,
              contentType.includes('json') && (!this.responseType || this.responseType === 'text')
                ? this.responseText : null);
          } catch (_) {}
          if (legacyEnabled) emit('xhr', metadata.method, metadata.url, this.status, kind, nowMs() - requestStartedAt);
        }, { once: true });
      }
      return originalSend.apply(this, arguments);
    };
  }

  window.__elonChatGptPrivateResearchProbe = Object.freeze({
    version: 12,
    enabled: legacyEnabled,
    handle: (action, command, respond) => {
      if (action !== 'private_protocol_probe') return false;
      const detail = evidence?.command(String(command.value || ''));
      respond(action, typeof detail === 'string', detail || 'protocol_probe_unavailable');
      return true;
    },
    expiresAt,
    observationCount: () => observationCount,
    privateObservationCount: () => privateObservationCount,
    recordPrivateOutcome: emitPrivateOutcome,
    recordPrivateStreamOutcome: emitPrivateStreamOutcome,
    recordPrivateStreamShape: emitPrivateStreamShape,
    recordPrivatePayloadShape: emitPrivatePayloadShape,
    copyRequestContext: (family) => Object.assign({}, requestContexts.get(String(family || '')) || {})
  });
})();
