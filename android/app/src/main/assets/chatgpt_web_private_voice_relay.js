(function () {
  'use strict';

  if (window.__elonChatGptPrivateResearchEnabled !== true) return;
  if (location.origin !== 'https://chatgpt.com') return;
  const existing = window.__elonChatGptPrivateVoiceRelay;
  if (existing && Number(existing.version) >= 2) return;

  const nativeFetch = window.fetch;
  const maxTemplateAgeMs = 2 * 60 * 1000;
  const maxExchangeMs = 15 * 1000;
  const resultLifetimeMs = 30 * 1000;
  const maxOfferLength = 240000;
  const maxAnswerLength = 320000;
  const requestIdPattern = /^relay_[a-z0-9]{8,32}$/;
  let template = null;
  let templateGeneration = 0;
  let dataChannelHint = null;
  let dataChannelGeneration = 0;
  let inFlightRequestId = null;
  let relayFetchDepth = 0;
  const results = new Map();

  function now() {
    return Date.now();
  }

  function isAudioSdp(value) {
    return typeof value === 'string' &&
      value.length >= 16 &&
      value.length <= maxAnswerLength &&
      /^v=0(?:\r?\n)/.test(value) &&
      /(?:\r?\n)m=audio\s/i.test(value);
  }

  function requestUrl(input) {
    try {
      return new URL(typeof input === 'string' ? input : input && input.url, location.href);
    } catch (_) {
      return null;
    }
  }

  function cloneHeaders(input, init) {
    try {
      const headers = new Headers(init && init.headers || input && input.headers || undefined);
      headers.delete('content-type');
      return headers;
    } catch (_) {
      return new Headers();
    }
  }

  function copyInitShape(input, init) {
    const source = init || {};
    const shape = {
      method: String(source.method || input && input.method || 'POST').toUpperCase(),
      headers: cloneHeaders(input, source)
    };
    ['credentials', 'mode', 'cache', 'redirect', 'referrer', 'referrerPolicy', 'integrity', 'keepalive']
      .forEach((key) => {
        if (source[key] !== undefined) shape[key] = source[key];
      });
    return shape;
  }

  function captureTemplate(input, init) {
    const url = requestUrl(input);
    const body = init && init.body;
    if (!url || url.origin !== location.origin || url.pathname !== '/realtime/wm') return;
    if (typeof FormData === 'undefined' || !(body instanceof FormData)) return;

    const fields = [];
    let offerCount = 0;
    let sessionCount = 0;
    try {
      body.forEach((value, key) => {
        const field = { key: String(key), value, offer: false };
        if (typeof value === 'string' && isAudioSdp(value)) {
          field.offer = true;
          offerCount += 1;
        }
        if (String(key).toLowerCase() === 'session' && typeof value === 'string') {
          JSON.parse(value);
          sessionCount += 1;
        }
        fields.push(field);
      });
    } catch (_) {
      return;
    }
    if (offerCount !== 1 || sessionCount !== 1 || fields.length > 16) return;

    templateGeneration += 1;
    template = {
      generation: templateGeneration,
      capturedAt: now(),
      used: false,
      url: url.href,
      init: copyInitShape(input, init),
      fields
    };
    pruneResults();
  }

  function validToken(value, maxLength) {
    return typeof value === 'string' &&
      value.length >= 1 &&
      value.length <= maxLength &&
      /^[\x20-\x7e]+$/.test(value);
  }

  function captureDataChannel(label, options) {
    if (!validToken(label, 64)) return;
    const source = options && typeof options === 'object' ? options : {};
    if (
      source.protocol !== undefined &&
      source.protocol !== '' &&
      !validToken(source.protocol, 64)
    ) return;
    const maxRetransmits = Number.isInteger(source.maxRetransmits)
      ? source.maxRetransmits
      : null;
    const id = Number.isInteger(source.id) ? source.id : null;
    if (maxRetransmits !== null && (maxRetransmits < 0 || maxRetransmits > 65535)) return;
    if (id !== null && (id < 0 || id > 65534)) return;
    dataChannelGeneration += 1;
    dataChannelHint = {
      generation: dataChannelGeneration,
      capturedAt: now(),
      label,
      ordered: source.ordered === undefined ? true : source.ordered === true,
      maxRetransmits,
      protocol: source.protocol === undefined ? '' : source.protocol,
      negotiated: source.negotiated === true,
      id
    };
  }

  function installPeerCapture() {
    const NativePeerConnection = window.RTCPeerConnection;
    if (typeof NativePeerConnection !== 'function') return;
    function RelayPeerConnection(configuration, constraints) {
      const peer = new NativePeerConnection(configuration, constraints);
      if (peer && typeof peer.createDataChannel === 'function') {
        const originalCreateDataChannel = peer.createDataChannel.bind(peer);
        peer.createDataChannel = function (label, options) {
          captureDataChannel(label, options);
          return originalCreateDataChannel(label, options);
        };
      }
      return peer;
    }
    RelayPeerConnection.prototype = NativePeerConnection.prototype;
    try { Object.setPrototypeOf(RelayPeerConnection, NativePeerConnection); } catch (_) {}
    window.RTCPeerConnection = RelayPeerConnection;
  }

  function pruneResults() {
    const cutoff = now() - resultLifetimeMs;
    results.forEach((entry, key) => {
      if (entry.savedAt < cutoff) results.delete(key);
    });
    while (results.size > 4) {
      results.delete(results.keys().next().value);
    }
  }

  function saveResult(requestId, value) {
    results.set(requestId, { savedAt: now(), value });
    pruneResults();
  }

  function failure(requestId, code) {
    saveResult(requestId, { status: 'failed', code });
  }

  function buildForm(value) {
    const body = new FormData();
    template.fields.forEach((field) => {
      if (field.offer) {
        body.append(field.key, value);
      } else {
        body.append(field.key, field.value);
      }
    });
    return body;
  }

  function buildInit(body) {
    const saved = template.init;
    const init = { method: saved.method, headers: new Headers(saved.headers), body };
    ['credentials', 'mode', 'cache', 'redirect', 'referrer', 'referrerPolicy', 'integrity', 'keepalive']
      .forEach((key) => {
        if (saved[key] !== undefined) init[key] = saved[key];
      });
    return init;
  }

  async function exchange(requestId, offer) {
    if (inFlightRequestId !== requestId) return;
    const activeTemplate = template;
    const controller = typeof AbortController === 'function' ? new AbortController() : null;
    let timeoutId = null;
    try {
      relayFetchDepth += 1;
      const init = buildInit(buildForm(offer));
      if (controller) init.signal = controller.signal;
      const response = await Promise.race([
        nativeFetch.call(window, activeTemplate.url, init),
        new Promise((_, reject) => {
          timeoutId = setTimeout(() => {
            if (controller) controller.abort();
            reject(new Error('timeout'));
          }, maxExchangeMs);
        })
      ]);
      if (!response || !response.ok) {
        failure(requestId, 'upstream_rejected');
        return;
      }
      const answer = await response.text();
      if (!isAudioSdp(answer) || answer.length > maxAnswerLength) {
        failure(requestId, 'invalid_answer');
        return;
      }
      saveResult(requestId, { status: 'ok', answer });
    } catch (error) {
      failure(requestId, error && error.message === 'timeout' ? 'timeout' : 'network_error');
    } finally {
      if (timeoutId !== null) clearTimeout(timeoutId);
      relayFetchDepth = Math.max(0, relayFetchDepth - 1);
      if (inFlightRequestId === requestId) inFlightRequestId = null;
      if (template === activeTemplate) {
        template = {
          generation: activeTemplate.generation,
          capturedAt: activeTemplate.capturedAt,
          used: true
        };
      }
    }
  }

  function startExchange(requestId, offer) {
    pruneResults();
    if (!requestIdPattern.test(String(requestId || ''))) return false;
    results.delete(requestId);
    if (!isAudioSdp(offer) || offer.length > maxOfferLength) {
      failure(requestId, 'invalid_offer');
      return false;
    }
    if (inFlightRequestId) {
      failure(requestId, 'busy');
      return false;
    }
    if (!template) {
      failure(requestId, 'template_unavailable');
      return false;
    }
    if (now() - template.capturedAt > maxTemplateAgeMs) {
      template = null;
      failure(requestId, 'template_expired');
      return false;
    }
    if (template.used) {
      failure(requestId, 'template_consumed');
      return false;
    }
    template.used = true;
    inFlightRequestId = requestId;
    exchange(requestId, offer);
    return true;
  }

  function takeResult(requestId) {
    pruneResults();
    const entry = results.get(String(requestId || ''));
    if (!entry) return null;
    results.delete(String(requestId || ''));
    return JSON.stringify(entry.value);
  }

  function bootstrap() {
    const age = template ? Math.max(0, now() - template.capturedAt) : 0;
    const dataChannelAge = dataChannelHint
      ? Math.max(0, now() - dataChannelHint.capturedAt)
      : 0;
    const dataChannelReady = Boolean(
      dataChannelHint && dataChannelAge <= maxTemplateAgeMs
    );
    const templateReady = Boolean(template && !template.used && age <= maxTemplateAgeMs);
    return JSON.stringify({
      version: 2,
      available: templateReady && dataChannelReady,
      templateGeneration,
      templateState: !template ? 'missing' : template.used ? 'consumed' : age > maxTemplateAgeMs ? 'expired' : 'ready',
      dataChannelGeneration,
      dataChannelState: !dataChannelHint ? 'missing' : dataChannelAge > maxTemplateAgeMs ? 'expired' : 'ready',
      dataChannel: dataChannelReady ? {
        label: dataChannelHint.label,
        ordered: dataChannelHint.ordered,
        maxRetransmits: dataChannelHint.maxRetransmits,
        protocol: dataChannelHint.protocol,
        negotiated: dataChannelHint.negotiated,
        id: dataChannelHint.id
      } : null,
      inFlight: Boolean(inFlightRequestId)
    });
  }

  function state() {
    const value = JSON.parse(bootstrap());
    delete value.dataChannel;
    return JSON.stringify(value);
  }

  function relayFetch(input, init) {
    if (relayFetchDepth === 0) captureTemplate(input, init || {});
    return nativeFetch.apply(this, arguments);
  }

  installPeerCapture();
  window.fetch = relayFetch;
  window.__elonChatGptPrivateVoiceRelay = Object.freeze({
    version: 2,
    bootstrap,
    state,
    startExchange,
    takeResult
  });
})();
