(function () {
  'use strict';

  var VERSION = 11;
  var PROVIDER_ID = '__PROVIDER_ID__';
  var MAX_BODY_BYTES = 2 * 1024 * 1024;
  var ANALYSIS_SCHEMA = 'yilong.web-ai.capture-analysis.v1';
  var RUNTIME_SLOT = '__elonWinWebResponseResearchCaptureRuntime';
  var FETCH_TAP_SUBSCRIPTION_SLOT = '__elonWinWebResponseResearchCaptureFetchTapSubscription';

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

  function visibleMessage(payload) {
    if (!payload || typeof payload !== 'object') return null;
    var envelope = Number.isFinite(payload.c) && payload.v && typeof payload.v === 'object'
      ? payload.v
      : payload;
    var message = envelope.message || envelope.data && envelope.data.message;
    return message && typeof message === 'object' ? message : null;
  }

  function safeStructureToken(value, limit) {
    var text = String(value || '').toLowerCase();
    return /^[a-z0-9_.:-]+$/.test(text) ? text.slice(0, limit || 48) : '';
  }

  function unsupportedRichSignature(reference, policy) {
    if (!reference || typeof reference !== 'object') return '';
    var initialState = reference.dil && reference.dil.initialState;
    if (reference.type === 'dil' && initialState && typeof initialState === 'object' &&
        !Array.isArray(initialState)) {
      if (policy && typeof policy.financePartFromWidget === 'function' &&
          policy.financePartFromWidget(initialState)) return '';
      return 'dil:' + Object.keys(initialState).filter(function (key) {
        return /^[A-Za-z_][A-Za-z0-9_]{0,63}$/.test(key);
      }).sort().slice(0, 12).join(',');
    }
    var data = reference.data;
    if (reference.type !== 'client_defined_widget' || !data || typeof data !== 'object') return '';
    if (policy && typeof policy.clientChartPartFromMetadata === 'function' &&
        policy.clientChartPartFromMetadata({ content_references: [reference] })) return '';
    var content = data.content && typeof data.content === 'object' ? data.content : {};
    return [
      'client',
      safeStructureToken(reference.category, 32),
      safeStructureToken(data.widget_type, 48),
      safeStructureToken(data.language, 32),
      safeStructureToken(content.chartType, 32)
    ].join(':');
  }

  function rendererUpgradePart() {
    return {
      type: 'interactive',
      text: '官网富内容已升级',
      kind: 'renderer_upgrade_required'
    };
  }

  function chatGptStreamAnalysis(body, format, recoveryGeneration) {
    if (PROVIDER_ID !== 'chatgpt' || format !== 'sse') return null;
    var policy = window.__elonChatGptPrivateStreamPolicy;
    var base = {
      schema: ANALYSIS_SCHEMA,
      analyzerVersion: 2,
      policyAvailable: Boolean(policy),
      decodedFrameCount: 0,
      acceptedFrameCount: 0,
      assistantFrameCount: 0,
      progressFrameCount: 0,
      textLength: 0,
      richKinds: [],
      contentTypes: [],
      unsupportedRichCount: 0,
      completed: false,
      parseError: false
    };
    if (!policy || typeof policy.createSession !== 'function' ||
        typeof policy.createSseDecoder !== 'function') return base;
    var session = policy.createSession({ now: function () { return Date.now(); } });
    var conversationId = '';
    var contentTypes = new Set();
    var unsupportedRich = new Set();
    try {
      session.begin();
      var decoder = policy.createSseDecoder(function (payload) {
        base.decodedFrameCount += 1;
        var contentType = contentTypeFrom(payload);
        if (contentType && contentTypes.size < 16) contentTypes.add(contentType);
        var visible = visibleMessage(payload);
        var references = visible && visible.metadata && Array.isArray(visible.metadata.content_references)
          ? visible.metadata.content_references
          : [];
        references.slice(0, 32).forEach(function (reference) {
          var signature = unsupportedRichSignature(reference, policy);
          if (signature && unsupportedRich.size < 32) unsupportedRich.add(signature);
        });
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
        var recoveredParts = Array.isArray(snapshot.richParts) ? snapshot.richParts.slice() : [];
        if (unsupportedRich.size) recoveredParts.push(rendererUpgradePart());
        base.unsupportedRichCount = unsupportedRich.size;
        base.richKinds = Array.from(new Set(recoveredParts.map(function (part) {
            return String(part && part.kind || '').toLowerCase();
          }).filter(function (kind) {
            return /^[a-z][a-z0-9_-]{0,31}$/.test(kind);
          }))).slice(0, 16);
        var recovery = window.__elonWinChatGptPrivateStreamRecovery;
        if (recovery && typeof recovery.accept === 'function' &&
            recoveredParts.length) {
          recovery.accept({
            messageId: snapshot.id || '',
            turnId: snapshot.turnId || '',
            conversationId: snapshot.conversationId || conversationId || '',
            text: snapshot.text || '',
            generation: recoveryGeneration,
            richParts: recoveredParts
          });
        }
      }
      base.unsupportedRichCount = unsupportedRich.size;
      base.contentTypes = Array.from(contentTypes).slice(0, 16);
      return base;
    } catch (_) {
      base.parseError = true;
      base.contentTypes = Array.from(contentTypes).slice(0, 16);
      return base;
    }
  }

  function googleRpcAnalysis(body, format) {
    if (PROVIDER_ID !== 'google-ai-mode') return null;
    var base = {
      schema: ANALYSIS_SCHEMA,
      analyzerVersion: 2,
      policyAvailable: true,
      decodedFrameCount: 0,
      acceptedFrameCount: 0,
      assistantFrameCount: 0,
      progressFrameCount: 0,
      textLength: 0,
      richKinds: [],
      contentTypes: [],
      unsupportedRichCount: 0,
      completed: true,
      parseError: false
    };
    var source = String(body || '').replace(/^\)\]\}'\s*/, '').trim();
    if (!source) return base;

    var candidates = [];
    var seenCandidates = new Set();
    function addCandidate(value) {
      var candidate = String(value || '').trim();
      if (!candidate || !/^[\[{]/.test(candidate) || seenCandidates.has(candidate)) return;
      seenCandidates.add(candidate);
      candidates.push(candidate);
    }
    addCandidate(source);
    source.split(/\r?\n/).forEach(addCandidate);

    var queue = [];
    candidates.slice(0, 64).forEach(function (candidate) {
      try {
        queue.push({ value: JSON.parse(candidate), depth: 0 });
        base.decodedFrameCount += 1;
        base.acceptedFrameCount += 1;
      } catch (_) {}
    });
    if (!queue.length) {
      base.parseError = format === 'json';
      return base;
    }

    var types = new Set(['google_rpc']);
    if (source.indexOf('wrb.fr') >= 0) types.add('batched_json');
    var inspected = 0;
    var nestedJson = 0;
    while (queue.length && inspected < 4096) {
      var entry = queue.shift();
      inspected += 1;
      if (typeof entry.value === 'string') {
        var nested = entry.value.trim();
        if (entry.depth < 5 && /^[\[{]/.test(nested) && nested.length <= MAX_BODY_BYTES) {
          try {
            queue.push({ value: JSON.parse(nested), depth: entry.depth + 1 });
            nestedJson += 1;
          } catch (_) {}
        }
      } else if (Array.isArray(entry.value)) {
        entry.value.slice(0, 128).forEach(function (value) {
          queue.push({ value: value, depth: entry.depth + 1 });
        });
      } else if (entry.value && typeof entry.value === 'object') {
        Object.keys(entry.value).slice(0, 128).forEach(function (key) {
          queue.push({ value: entry.value[key], depth: entry.depth + 1 });
        });
      }
    }
    if (nestedJson) types.add('nested_json');
    if (inspected >= 4096) types.add('bounded_walk');
    base.contentTypes = Array.from(types).slice(0, 16);
    return base;
  }

  function publish(meta, transport, status, contentType, body, truncated) {
    var bounded = boundedUtf8(body, truncated);
    if (!bounded.body) return;
    var format = responseFormat(contentType, bounded.body.slice(0, 80));
    var capture = {
      providerId: PROVIDER_ID,
      captureRuntimeVersion: VERSION,
      method: meta.method,
      endpointFamily: meta.endpointFamily,
      transport: transport,
      status: Number(status || 0),
      format: format,
      capturedAtMs: Date.now(),
      body: bounded.body,
      truncated: bounded.truncated
    };
    var analysis = chatGptStreamAnalysis(bounded.body, format, meta.recoveryGeneration) ||
      googleRpcAnalysis(bounded.body, format);
    if (analysis) capture.analysis = analysis;
    invokeCapture(capture);
    if (PROVIDER_ID === 'chatgpt') {
      var financeRecovery = window.__elonWinChatGptCapturedFinanceRecovery;
      if (financeRecovery && typeof financeRecovery.recover === 'function') {
        Promise.resolve(financeRecovery.recover(
          bounded.body,
          format,
          meta.recoveryGeneration
        )).catch(function () {});
      }
    }
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

  function requestMetadata(input, init) {
    var meta = classify(requestUrl(input), requestMethod(input, init));
    if (!meta) return null;
    var recovery = window.__elonWinChatGptPrivateStreamRecovery;
    if (recovery && typeof recovery.generation === 'function') {
      try { meta.recoveryGeneration = recovery.generation(); } catch (_) {}
    }
    return meta;
  }

  function observeXhrResponse(xhr, meta) {
    if (!xhr || !meta || xhr.status < 100 || xhr.status > 599) return;
    var contentType = '';
    var body = '';
    try { contentType = xhr.getResponseHeader('content-type') || ''; } catch (_) {}
    try {
      body = !xhr.responseType || xhr.responseType === 'text'
        ? xhr.responseText
        : xhr.responseType === 'json' ? JSON.stringify(xhr.response) : '';
    } catch (_) {}
    publish(meta, 'xhr', xhr.status, contentType, body, false);
  }

  var runtime = Object.freeze({
    version: VERSION,
    providerId: PROVIDER_ID,
    requestMetadata: requestMetadata,
    observeFetchResponse: observeFetchResponse,
    observeXhrResponse: observeXhrResponse,
  });
  window[RUNTIME_SLOT] = runtime;
  window.__elonWinWebResponseResearchCaptureVersion = VERSION;

  function installFetchTapObserver() {
    var tap = window.__elonChatGptPrivateFetchTap;
    if (!tap || typeof tap.subscribe !== 'function') return false;
    var installed = window[FETCH_TAP_SUBSCRIPTION_SLOT];
    if (installed && installed.tap === tap) return true;
    if (installed && typeof installed.unsubscribe === 'function') {
      try { installed.unsubscribe(); } catch (_) {}
    }
    var unsubscribe = tap.subscribe(function (event) {
      var activeRuntime = window[RUNTIME_SLOT];
      if (!activeRuntime || !event || !event.response ||
          typeof activeRuntime.requestMetadata !== 'function' ||
          typeof activeRuntime.observeFetchResponse !== 'function') return;
      var meta = activeRuntime.requestMetadata(String(event.url || ''), {
        method: String(event.method || 'GET').toUpperCase()
      });
      if (meta) void activeRuntime.observeFetchResponse(event.response, meta);
    });
    window[FETCH_TAP_SUBSCRIPTION_SLOT] = Object.freeze({
      tap: tap,
      unsubscribe: typeof unsubscribe === 'function' ? unsubscribe : null
    });
    return true;
  }

  // The shared APK/Win fetch tap is installed before the page application and
  // remains the canonical interception point even when later adapters replace
  // window.fetch. Prefer that stable bus; retain the legacy wrapper only for
  // providers or test environments where the tap is unavailable.
  var observingFetchTap = installFetchTapObserver();

  if (!observingFetchTap && typeof window.fetch === 'function' &&
      !window.fetch.__elonResearchCaptureWrapped) {
    var originalFetch = window.fetch;
    var wrappedFetch = function (input, init) {
      var activeRuntime = window[RUNTIME_SLOT];
      var meta = activeRuntime && typeof activeRuntime.requestMetadata === 'function'
        ? activeRuntime.requestMetadata(input, init)
        : null;
      return originalFetch.apply(this, arguments).then(function (response) {
        var latestRuntime = window[RUNTIME_SLOT] || activeRuntime;
        if (meta && latestRuntime && typeof latestRuntime.observeFetchResponse === 'function') {
          void latestRuntime.observeFetchResponse(response, meta);
        }
        return response;
      });
    };
    Object.defineProperty(wrappedFetch, '__elonResearchCaptureWrapped', { value: true });
    Object.defineProperty(wrappedFetch, '__elonResearchCaptureRuntimeProxyVersion', { value: 1 });
    window.fetch = wrappedFetch;
  }

  if (window.XMLHttpRequest && !XMLHttpRequest.prototype.__elonResearchCaptureWrapped) {
    var xhrMeta = new WeakMap();
    var originalOpen = XMLHttpRequest.prototype.open;
    var originalSend = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.open = function (method, url) {
      var activeRuntime = window[RUNTIME_SLOT];
      xhrMeta.set(this, activeRuntime && typeof activeRuntime.requestMetadata === 'function'
        ? activeRuntime.requestMetadata(String(url || ''), {
          method: String(method || 'GET').toUpperCase()
        })
        : null);
      return originalOpen.apply(this, arguments);
    };
    XMLHttpRequest.prototype.send = function () {
      var xhr = this;
      var meta = xhrMeta.get(xhr);
      if (meta) {
        xhr.addEventListener('loadend', function () {
          var activeRuntime = window[RUNTIME_SLOT];
          if (activeRuntime && typeof activeRuntime.observeXhrResponse === 'function') {
            activeRuntime.observeXhrResponse(xhr, meta);
          }
        }, { once: true });
      }
      return originalSend.apply(this, arguments);
    };
    Object.defineProperty(XMLHttpRequest.prototype, '__elonResearchCaptureWrapped', { value: true });
    Object.defineProperty(
      XMLHttpRequest.prototype,
      '__elonResearchCaptureRuntimeProxyVersion',
      { value: 1 }
    );
  }
})();
