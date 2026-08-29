(function () {
  'use strict';

  if (window.__elonChatGptPrivateResearchEnabled !== true) return;
  if (location.origin !== 'https://chatgpt.com') return;
  const existing = window.__elonChatGptPrivateVoiceRelay;
  if (existing && Number(existing.version) >= 4) return;

  const nativeFetch = window.fetch;
  const maxTemplateAgeMs = 2 * 60 * 1000;
  const maxExchangeMs = 15 * 1000;
  const resultLifetimeMs = 30 * 1000;
  const maxOfferLength = 240000;
  const maxAnswerLength = 320000;
  const requestIdPattern = /^relay_[a-z0-9]{8,32}$/;
  const presetDataChannel = Object.freeze({
    label: '',
    ordered: true,
    maxRetransmits: null,
    protocol: '',
    negotiated: false,
    id: null
  });
  let template = null;
  let templateGeneration = 0;
  let dataChannelHint = null;
  let dataChannelGeneration = 0;
  let officialPeer = null;
  let officialMediaEnabled = true;
  let officialTakeoverActive = false;
  let takeoverAnswer = null;
  let pendingExchange = null;
  let pendingTimeoutId = null;
  let inFlightRequestId = null;
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

  function validDataChannelLabel(value) {
    return typeof value === 'string' &&
      value.length <= 64 &&
      /^[\x20-\x7e]*$/.test(value);
  }

  function captureDataChannel(label, options) {
    // WebRTC permits an empty data-channel label. ChatGPT Web currently uses
    // that shape on some page generations, so it must not be confused with a
    // missing or malformed bootstrap hint.
    if (!validDataChannelLabel(label)) return false;
    const source = options && typeof options === 'object' ? options : {};
    if (
      source.protocol !== undefined &&
      source.protocol !== '' &&
      !validToken(source.protocol, 64)
    ) return false;
    const maxRetransmits = Number.isInteger(source.maxRetransmits)
      ? source.maxRetransmits
      : null;
    const id = Number.isInteger(source.id) ? source.id : null;
    if (maxRetransmits !== null && (maxRetransmits < 0 || maxRetransmits > 65535)) return false;
    if (id !== null && (id < 0 || id > 65534)) return false;
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
    return true;
  }

  function audioTracks(peer, method) {
    try {
      if (!peer || typeof peer[method] !== 'function') return [];
      return peer[method]()
        .map((entry) => entry && entry.track)
        .filter((track) => track && track.kind === 'audio' && track.readyState !== 'ended');
    } catch (_) {
      return [];
    }
  }

  function setTracksEnabled(tracks, enabled) {
    let changed = 0;
    tracks.forEach((track) => {
      try {
        track.enabled = enabled;
        if (track.enabled === enabled) changed += 1;
      } catch (_) {}
    });
    return changed;
  }

  function guardSender(peer, sender) {
    if (!sender || sender.__elonPrivateVoiceGuarded === true) return sender;
    try {
      const originalReplaceTrack = typeof sender.replaceTrack === 'function'
        ? sender.replaceTrack.bind(sender)
        : null;
      if (originalReplaceTrack) {
        sender.replaceTrack = function (track) {
          if (
            peer === officialPeer &&
            !officialMediaEnabled &&
            track &&
            track.kind === 'audio'
          ) {
            try { track.enabled = false; } catch (_) {}
          }
          return originalReplaceTrack(track);
        };
      }
      Object.defineProperty(sender, '__elonPrivateVoiceGuarded', { value: true });
    } catch (_) {}
    return sender;
  }

  function setOfficialMediaEnabled(enabled) {
    const nextEnabled = enabled === true;
    if (!officialPeer) {
      return JSON.stringify({ version: 4, applied: false, code: 'peer_unavailable' });
    }
    officialTakeoverActive = !nextEnabled;
    if (nextEnabled) takeoverAnswer = null;
    officialMediaEnabled = nextEnabled;
    const senderTracks = setTracksEnabled(audioTracks(officialPeer, 'getSenders'), nextEnabled);
    const receiverTracks = setTracksEnabled(audioTracks(officialPeer, 'getReceivers'), nextEnabled);
    return JSON.stringify({
      version: 4,
      applied: true,
      enabled: nextEnabled,
      senderTracks,
      receiverTracks,
      code: null
    });
  }

  function closeOfficialPeer() {
    if (!officialPeer) {
      return JSON.stringify({ version: 4, applied: false, code: 'peer_unavailable' });
    }
    const peer = officialPeer;
    const senderTracks = setTracksEnabled(audioTracks(peer, 'getSenders'), false);
    const receiverTracks = setTracksEnabled(audioTracks(peer, 'getReceivers'), false);
    officialMediaEnabled = false;
    officialTakeoverActive = true;
    officialPeer = null;
    try { peer.close(); } catch (_) {}
    return JSON.stringify({
      version: 4,
      applied: true,
      enabled: false,
      senderTracks,
      receiverTracks,
      closed: true
    });
  }

  function resetTakeover() {
    clearPendingExchange();
    const peer = officialPeer;
    const senderTracks = setTracksEnabled(audioTracks(peer, 'getSenders'), false);
    const receiverTracks = setTracksEnabled(audioTracks(peer, 'getReceivers'), false);
    officialPeer = null;
    officialMediaEnabled = true;
    officialTakeoverActive = false;
    takeoverAnswer = null;
    if (peer) {
      try { peer.close(); } catch (_) {}
    }
    return JSON.stringify({
      version: 4,
      applied: true,
      enabled: true,
      senderTracks,
      receiverTracks,
      closed: Boolean(peer)
    });
  }

  function installPeerCapture() {
    const NativePeerConnection = window.RTCPeerConnection;
    if (typeof NativePeerConnection !== 'function') return;
    function RelayPeerConnection(configuration, constraints) {
      const peer = new NativePeerConnection(configuration, constraints);
      if (peer && typeof peer.setRemoteDescription === 'function') {
        const originalSetRemoteDescription = peer.setRemoteDescription.bind(peer);
        peer.setRemoteDescription = function () {
          if (peer === officialPeer && officialTakeoverActive && takeoverAnswer) {
            return Promise.resolve();
          }
          return originalSetRemoteDescription.apply(peer, arguments);
        };
      }
      if (peer && typeof peer.addTrack === 'function') {
        const originalAddTrack = peer.addTrack.bind(peer);
        peer.addTrack = function (track) {
          if (
            peer === officialPeer &&
            !officialMediaEnabled &&
            track &&
            track.kind === 'audio'
          ) {
            try { track.enabled = false; } catch (_) {}
          }
          return guardSender(peer, originalAddTrack.apply(peer, arguments));
        };
      }
      if (peer && typeof peer.addTransceiver === 'function') {
        const originalAddTransceiver = peer.addTransceiver.bind(peer);
        peer.addTransceiver = function () {
          const transceiver = originalAddTransceiver.apply(peer, arguments);
          if (transceiver) guardSender(peer, transceiver.sender);
          return transceiver;
        };
      }
      if (peer && typeof peer.addEventListener === 'function') {
        peer.addEventListener('track', (event) => {
          if (peer !== officialPeer || officialMediaEnabled || !event || !event.track) return;
          try { event.track.enabled = false; } catch (_) {}
        });
      }
      if (peer && typeof peer.createDataChannel === 'function') {
        const originalCreateDataChannel = peer.createDataChannel.bind(peer);
        peer.createDataChannel = function (label, options) {
          if (captureDataChannel(label, options)) {
            const previousPeer = officialPeer;
            officialPeer = peer;
            officialMediaEnabled = !officialTakeoverActive;
            if (
              officialTakeoverActive &&
              previousPeer &&
              previousPeer !== peer
            ) {
              setTracksEnabled(audioTracks(previousPeer, 'getSenders'), false);
              setTracksEnabled(audioTracks(previousPeer, 'getReceivers'), false);
              try { previousPeer.close(); } catch (_) {}
            }
            try {
              if (typeof peer.getSenders === 'function') {
                peer.getSenders().forEach((sender) => guardSender(peer, sender));
              }
            } catch (_) {}
            setTracksEnabled(audioTracks(peer, 'getSenders'), officialMediaEnabled);
            setTracksEnabled(audioTracks(peer, 'getReceivers'), officialMediaEnabled);
          }
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

  function clearPendingExchange(requestId) {
    if (requestId && (!pendingExchange || pendingExchange.requestId !== requestId)) return false;
    if (pendingTimeoutId !== null) clearTimeout(pendingTimeoutId);
    pendingTimeoutId = null;
    pendingExchange = null;
    return true;
  }

  function releaseTakeoverForFallback() {
    officialTakeoverActive = false;
    officialMediaEnabled = true;
    takeoverAnswer = null;
    setTracksEnabled(audioTracks(officialPeer, 'getSenders'), true);
    setTracksEnabled(audioTracks(officialPeer, 'getReceivers'), true);
  }

  function armExchange(requestId, offer) {
    pruneResults();
    const id = String(requestId || '');
    if (!requestIdPattern.test(id)) {
      return JSON.stringify({ version: 4, armed: false, code: 'invalid_request' });
    }
    results.delete(id);
    if (!isAudioSdp(offer) || offer.length > maxOfferLength) {
      failure(id, 'invalid_offer');
      return JSON.stringify({ version: 4, armed: false, code: 'invalid_offer' });
    }
    if (pendingExchange || inFlightRequestId) {
      failure(id, 'busy');
      return JSON.stringify({ version: 4, armed: false, code: 'busy' });
    }
    template = null;
    pendingExchange = { requestId: id, offer, armedAt: now() };
    officialTakeoverActive = true;
    officialMediaEnabled = false;
    takeoverAnswer = null;
    setTracksEnabled(audioTracks(officialPeer, 'getSenders'), false);
    setTracksEnabled(audioTracks(officialPeer, 'getReceivers'), false);
    pendingTimeoutId = setTimeout(() => {
      if (!pendingExchange || pendingExchange.requestId !== id) return;
      clearPendingExchange(id);
      releaseTakeoverForFallback();
      failure(id, 'timeout');
    }, maxExchangeMs);
    return JSON.stringify({ version: 4, armed: true, code: null });
  }

  function cancelExchange(requestId) {
    const id = String(requestId || '');
    if (!clearPendingExchange(id)) return false;
    releaseTakeoverForFallback();
    return true;
  }

  function upstreamCall(receiver, input, init) {
    return init === undefined
      ? nativeFetch.call(receiver, input)
      : nativeFetch.call(receiver, input, init);
  }

  function takeoverResponse() {
    return new Response(takeoverAnswer, {
      status: 201,
      statusText: 'Created',
      headers: { 'content-type': 'application/sdp' }
    });
  }

  async function exchangeOnOfficialRequest(receiver, input, originalInit, active, activeTemplate) {
    const requestId = active.requestId;
    const controller = typeof AbortController === 'function' ? new AbortController() : null;
    let timeoutId = null;
    clearPendingExchange(requestId);
    inFlightRequestId = requestId;
    activeTemplate.used = true;
    try {
      const replacementInit = buildInit(buildForm(active.offer));
      if (controller) replacementInit.signal = controller.signal;
      const response = await Promise.race([
        nativeFetch.call(window, activeTemplate.url, replacementInit),
        new Promise((_, reject) => {
          timeoutId = setTimeout(() => {
            if (controller) controller.abort();
            reject(new Error('timeout'));
          }, maxExchangeMs);
        })
      ]);
      if (!response || !response.ok) {
        failure(requestId, 'upstream_rejected');
        releaseTakeoverForFallback();
        return upstreamCall(receiver, input, originalInit);
      }
      const answer = await response.clone().text();
      if (!isAudioSdp(answer) || answer.length > maxAnswerLength) {
        failure(requestId, 'invalid_answer');
        releaseTakeoverForFallback();
        return upstreamCall(receiver, input, originalInit);
      }
      takeoverAnswer = answer;
      officialTakeoverActive = true;
      officialMediaEnabled = false;
      setTracksEnabled(audioTracks(officialPeer, 'getSenders'), false);
      setTracksEnabled(audioTracks(officialPeer, 'getReceivers'), false);
      saveResult(requestId, { status: 'ok', answer });
      return response;
    } catch (error) {
      failure(requestId, error && error.message === 'timeout' ? 'timeout' : 'network_error');
      releaseTakeoverForFallback();
      return upstreamCall(receiver, input, originalInit);
    } finally {
      if (timeoutId !== null) clearTimeout(timeoutId);
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
    const observedDataChannelReady = Boolean(
      dataChannelHint && dataChannelAge <= maxTemplateAgeMs
    );
    const channel = observedDataChannelReady ? dataChannelHint : presetDataChannel;
    return JSON.stringify({
      version: 4,
      available: !pendingExchange && !inFlightRequestId,
      templateGeneration,
      templateState: !template ? 'missing' : template.used ? 'consumed' : age > maxTemplateAgeMs ? 'expired' : 'ready',
      dataChannelGeneration,
      dataChannelState: observedDataChannelReady ? 'ready' : 'preset',
      dataChannel: {
        label: channel.label,
        ordered: channel.ordered,
        maxRetransmits: channel.maxRetransmits,
        protocol: channel.protocol,
        negotiated: channel.negotiated,
        id: channel.id
      },
      armed: Boolean(pendingExchange),
      inFlight: Boolean(inFlightRequestId),
      takeoverActive: officialTakeoverActive && Boolean(takeoverAnswer)
    });
  }

  function state() {
    const value = JSON.parse(bootstrap());
    delete value.dataChannel;
    return JSON.stringify(value);
  }

  function relayFetch(input, init) {
    const url = requestUrl(input);
    const isVoiceRequest = Boolean(
      url && url.origin === location.origin && url.pathname === '/realtime/wm'
    );
    if (!isVoiceRequest) return nativeFetch.apply(this, arguments);
    if (officialTakeoverActive && takeoverAnswer) {
      return Promise.resolve(takeoverResponse());
    }
    captureTemplate(input, init || {});
    if (!pendingExchange) return nativeFetch.apply(this, arguments);
    const active = pendingExchange;
    const activeTemplate = template;
    if (!activeTemplate || activeTemplate.used) {
      clearPendingExchange(active.requestId);
      releaseTakeoverForFallback();
      failure(active.requestId, 'template_unavailable');
      return nativeFetch.apply(this, arguments);
    }
    return exchangeOnOfficialRequest(this, input, init, active, activeTemplate);
  }

  installPeerCapture();
  window.fetch = relayFetch;
  window.__elonChatGptPrivateVoiceRelay = Object.freeze({
    version: 4,
    bootstrap,
    state,
    armExchange,
    cancelExchange,
    takeResult,
    setOfficialMediaEnabled,
    closeOfficialPeer,
    resetTakeover
  });
})();
