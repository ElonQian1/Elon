(function () {
  'use strict';

  var VERSION = 2;
  var PROVIDER_ID = '__PROVIDER_ID__';
  var MAX_BODY_BYTES = 2 * 1024 * 1024;
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
      if (method === 'POST' && /^\/backend-api\/(?:f\/)?conversation(?:\/|$)/.test(path)) {
        return { endpointFamily: 'conversation_stream', method: method };
      }
      if (method === 'GET' && (/^\/backend-api\/(?:f\/)?conversations?\/[^/]+$/.test(path))) {
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
    if (value.indexOf('ndjson') >= 0 || value.indexOf('jsonl') >= 0) return 'ndjson';
    if (value.indexOf('json') >= 0) return 'json';
    var prefix = String(bodyPrefix || '').trimStart();
    if (prefix.indexOf('data:') === 0 || prefix.indexOf('event:') === 0) return 'sse';
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

  function publish(meta, transport, status, contentType, body, truncated) {
    var bounded = boundedUtf8(body, truncated);
    if (!bounded.body) return;
    invokeCapture({
      providerId: PROVIDER_ID,
      method: meta.method,
      endpointFamily: meta.endpointFamily,
      transport: transport,
      status: Number(status || 0),
      format: responseFormat(contentType, bounded.body.slice(0, 80)),
      capturedAtMs: Date.now(),
      body: bounded.body,
      truncated: bounded.truncated
    });
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
