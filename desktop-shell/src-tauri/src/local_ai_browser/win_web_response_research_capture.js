(function () {
  'use strict';

  var VERSION = 6;
  var PROVIDER_ID = '__PROVIDER_ID__';
  var MAX_BODY_BYTES = 2 * 1024 * 1024;
  var ANALYSIS_SCHEMA = 'yilong.web-ai.capture-analysis.v1';
  if (window.__elonWinWebResponseResearchCaptureVersion >= VERSION) return;
  window.__elonWinWebResponseResearchCaptureVersion = VERSION;

  function invokeCapture(capture) {
    var internalInvoke = window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke;
    var publicInvoke = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
    var call = internalInvoke || publicInvoke;
    if (typeof call !== 'function') return;
    Promise.resolve(call('publish_local_ai_web_research_capture', { capture: capture }))
      .catch(function () {});
  }

  function requestUrl(input) {
    if (typeof input === 'string') return input;
    if (input instanceof URL) return input.href;
    return input && typeof input.url === 'string' ? input.url : '';
  }

  function requestMethod(input, init) {
    var method = init && init.method;
    if (!method && input && typeof input.method === 'string') method = input.method;
    return String(method || 'GET').toUpperCase();
  }

  function classify(urlValue, method) {
    var url;
    try { url = new URL(urlValue, location.href); } catch (_) { return null; }
    if (url.origin !== location.origin || (method !== 'GET' && method !== 'POST')) return null;
    var path = url.pathname;
    if (PROVIDER_ID === 'chatgpt') {
      if (method === 'POST' && /^\/(?:backend-api|backend-anon)\/(?:f\/)?conversation(?:\/|$)/.test(path)) {
        return { endpointFamily: 'conversation_stream', method: method };
      }
      if (method === 'GET' && (/^\/(?:backend-api|backend-anon)\/(?:f\/)?conversations?\/[^/]+$/.test(path))) {
        return { endpointFamily: 'conversation_detail', method: method };
      }
      return null;
    }
    if (PROVIDER_ID === 'google-ai-mode' && method === 'POST' && (
      path.indexOf('/_/') === 0 || path.indexOf('batchexecute') >= 0 || path.indexOf('/async/') >= 0
    )) {
      return { endpointFamily: 'ai_rpc', method: method };
    }
    return null;
  }

  function responseFormat(contentType, bodyPrefix) {
    var value = String(contentType || '').toLowerCase();
    if (value.indexOf('text/event-stream') >= 0) return 'sse';
    var prefix = String(bodyPrefix || '').trimStart();
    // Some official fetch layers preserve the SSE body while normalizing the
    // response header to JSON or plain text. The framed body is authoritative.
    if (prefix.indexOf('data:') === 0 || prefix.indexOf('event:') === 0) return 'sse';
    if (value.indexOf('ndjson') >= 0 || value.indexOf('jsonl') >= 0) return 'ndjson';
    if (value.indexOf('json') >= 0) return 'json';
    if (prefix.charAt(0) === '{' || prefix.charAt(0) === '[') return 'json';
    return 'text';
  }

  function boundedUtf8(text, alreadyTruncated) {
    var value = String(text || '');
    var bytes = new TextEncoder().encode(value);
    if (bytes.byteLength <= MAX_BODY_BYTES) {
      return { body: value, truncated: Boolean(alreadyTruncated) };
    }
    return {
      body: new TextDecoder().decode(bytes.slice(0, MAX_BODY_BYTES)),
      truncated: true
    };
  }

  function contentTypeFrom(payload) {
    if (!payload || typeof payload !== 'object') return '';
    var candidates = [
      payload.message,
      payload.v && payload.v.message,
      payload.data && payload.data.message,
      payload.result && payload.result.message
    ];
    for (var index = 0; index < candidates.length; index += 1) {
      var message = candidates[index];
      var value = message && message.content && message.content.content_type;
      if (/^[a-z][a-z0-9_-]{0,39}$/i.test(String(value || ''))) {
        return String(value).toLowerCase();
      }
    }
    return '';
  }

  function chatGptStreamAnalysis(body, format, recoveryGeneration) {
    if (PROVIDER_ID !== 'chatgpt' || format !== 'sse') return null;
    var policy = window.__elonChatGptPrivateStreamPolicy;
    var base = {
      schema: ANALYSIS_SCHEMA,
      analyzerVersion: 1,
      policyAvailable: Boolean(policy),
      decodedFrameCount: 0,
      acceptedFrameCount: 0,
      assistantFrameCount: 0,
      progressFrameCount: 0,
      textLength: 0,
      richKinds: [],
      contentTypes: [],
      completed: false,
      parseError: false
    };
    if (!policy || typeof policy.createSession !== 'function' ||
        typeof policy.createSseDecoder !== 'function') return base;
    var session = policy.createSession({ now: function () { return Date.now(); } });
    var conversationId = '';
    var contentTypes = new Set();
    try {
      session.begin();
      var decoder = policy.createSseDecoder(function (payload) {
        base.decodedFrameCount += 1;
        var contentType = contentTypeFrom(payload);
        if (contentType && contentTypes.size < 16) contentTypes.add(contentType);
        if (typeof policy.assistantFrame === 'function') {
          var assistant = policy.assistantFrame(payload);
          if (assistant) {
            base.assistantFrameCount += 1;
            base.textLength = Math.max(base.textLength, String(assistant.text || '').length);
            conversationId = String(assistant.conversationId || conversationId || '').slice(0, 180);
          }
        }
        if (typeof policy.progressFrame === 'function' && policy.progressFrame(payload)) {
          base.progressFrameCount += 1;
        }
        if (session.accept(payload)) base.acceptedFrameCount += 1;
      }, function () {
        base.completed = Boolean(session.finish());
      });
      decoder.push(String(body || ''));
      decoder.finish();
      var snapshot = session.current(location.pathname);
      if (!snapshot && conversationId) snapshot = session.current('/c/' + conversationId);
      if (snapshot) {
        base.textLength = Math.max(base.textLength, String(snapshot.text || '').length);
        base.completed = base.completed || snapshot.state === 'completed';
        base.richKinds = Array.from(new Set((Array.isArray(snapshot.richParts)
          ? snapshot.richParts : []).map(function (part) {
            return String(part && part.kind || '').toLowerCase();
          }).filter(function (kind) {
            return /^[a-z][a-z0-9_-]{0,31}$/.test(kind);
          }))).slice(0, 16);
        var recovery = window.__elonWinChatGptPrivateStreamRecovery;
        if (recovery && typeof recovery.accept === 'function' &&
            Array.isArray(snapshot.richParts) && snapshot.richParts.length) {
          recovery.accept({
            messageId: snapshot.id || '',
            turnId: snapshot.turnId || '',
            conversationId: snapshot.conversationId || conversationId || '',
            text: snapshot.text || '',
            generation: recoveryGeneration,
            richParts: snapshot.richParts
          });
        }
      }
      base.contentTypes = Array.from(contentTypes).slice(0, 16);
      return base;
    } catch (_) {
      base.parseError = true;
      base.contentTypes = Array.from(contentTypes).slice(0, 16);
      return base;
    }
  }

  function publish(meta, transport, status, contentType, body, truncated) {
    var bounded = boundedUtf8(body, truncated);
    if (!bounded.body) return;
    var format = responseFormat(contentType, bounded.body.slice(0, 80));
    var capture = {
      providerId: PROVIDER_ID,
      method: meta.method,
      endpointFamily: meta.endpointFamily,
      transport: transport,
      status: Number(status || 0),
      format: format,
      capturedAtMs: Date.now(),
      body: bounded.body,
      truncated: bounded.truncated
    };
    var analysis = chatGptStreamAnalysis(bounded.body, format, meta.recoveryGeneration);
    if (analysis) capture.analysis = analysis;
    invokeCapture(capture);
  }

  async function observeFetchResponse(response, meta) {
    var clone;
    try { clone = response.clone(); } catch (_) { return; }
    var contentType = '';
    try { contentType = clone.headers.get('content-type') || ''; } catch (_) {}
    if (!clone.body || typeof clone.body.getReader !== 'function') {
      try {
        publish(meta, 'fetch', clone.status, contentType, await clone.text(), false);
      } catch (_) {}
      return;
    }
    var reader = clone.body.getReader();
    var decoder = new TextDecoder();
    var chunks = [];
    var bytesRead = 0;
    var truncated = false;
    try {
      while (true) {
        var next = await reader.read();
        if (next.done) break;
        var chunk = next.value || new Uint8Array(0);
        if (bytesRead + chunk.byteLength > MAX_BODY_BYTES) {
          var remaining = Math.max(0, MAX_BODY_BYTES - bytesRead);
          if (remaining) chunks.push(decoder.decode(chunk.slice(0, remaining), { stream: true }));
          truncated = true;
          try { await reader.cancel(); } catch (_) {}
          break;
        }
        bytesRead += chunk.byteLength;
        chunks.push(decoder.decode(chunk, { stream: true }));
      }
      chunks.push(decoder.decode());
      publish(meta, 'fetch', clone.status, contentType, chunks.join(''), truncated);
    } catch (_) {
      if (chunks.length) publish(meta, 'fetch', clone.status, contentType, chunks.join(''), true);
    }
  }

  if (typeof window.fetch === 'function' && !window.fetch.__elonResearchCaptureWrapped) {
    var originalFetch = window.fetch;
    var wrappedFetch = function (input, init) {
      var meta = classify(requestUrl(input), requestMethod(input, init));
      var recovery = window.__elonWinChatGptPrivateStreamRecovery;
      if (meta && recovery && typeof recovery.generation === 'function') {
        try { meta.recoveryGeneration = recovery.generation(); } catch (_) {}
      }
      return originalFetch.apply(this, arguments).then(function (response) {
        if (meta) void observeFetchResponse(response, meta);
        return response;
      });
    };
    Object.defineProperty(wrappedFetch, '__elonResearchCaptureWrapped', { value: true });
    window.fetch = wrappedFetch;
  }

  if (window.XMLHttpRequest && !XMLHttpRequest.prototype.__elonResearchCaptureWrapped) {
    var xhrMeta = new WeakMap();
    var originalOpen = XMLHttpRequest.prototype.open;
    var originalSend = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.open = function (method, url) {
      xhrMeta.set(this, classify(String(url || ''), String(method || 'GET').toUpperCase()));
      return originalOpen.apply(this, arguments);
    };
    XMLHttpRequest.prototype.send = function () {
      var xhr = this;
      var meta = xhrMeta.get(xhr);
      if (meta) {
        var recovery = window.__elonWinChatGptPrivateStreamRecovery;
        if (recovery && typeof recovery.generation === 'function') {
          try { meta.recoveryGeneration = recovery.generation(); } catch (_) {}
        }
        xhr.addEventListener('loadend', function () {
          if (xhr.status < 100 || xhr.status > 599) return;
          var contentType = '';
          var body = '';
          try { contentType = xhr.getResponseHeader('content-type') || ''; } catch (_) {}
          try {
            body = !xhr.responseType || xhr.responseType === 'text'
              ? xhr.responseText
              : xhr.responseType === 'json' ? JSON.stringify(xhr.response) : '';
          } catch (_) {}
          publish(meta, 'xhr', xhr.status, contentType, body, false);
        }, { once: true });
      }
      return originalSend.apply(this, arguments);
    };
    Object.defineProperty(XMLHttpRequest.prototype, '__elonResearchCaptureWrapped', { value: true });
  }
})();
