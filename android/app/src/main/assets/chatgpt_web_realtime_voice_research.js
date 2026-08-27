(function () {
  'use strict';

  if (window.__elonChatGptPrivateResearchEnabled !== true) return;
  if (location.origin !== 'https://chatgpt.com') return;
  const existing = window.__elonChatGptRealtimeVoiceResearch;
  if (existing && Number(existing.version) >= 1) return;

  const startedAt = Date.now();
  const expiresAt = startedAt + (10 * 60 * 1000);
  const maxObservations = 160;
  const voicePathHint = /(voice|realtime|webrtc|rtc|audio|speech)/i;
  const sensitiveKeyHint = /(token|secret|credential|authorization|cookie|proof|sdp|candidate)/i;
  let observationCount = 0;
  let voiceWindowUntil = 0;

  function bridgeReady() {
    return window.elonChatGptNative &&
      typeof window.elonChatGptNative.postMessage === 'function' &&
      /^doc_[a-z0-9_]{3,80}$/.test(String(window.__elonChatGptDocumentToken || ''));
  }

  function safePart(value, fallback) {
    const normalized = String(value || '').toLowerCase().replace(/[^a-z0-9._:/{}-]/g, '-');
    return normalized.slice(0, 96) || fallback;
  }

  function emit(parts) {
    if (!bridgeReady() || Date.now() > expiresAt || observationCount >= maxObservations) return;
    const detail = ['v1'].concat(parts.map((part) => safePart(part, 'none'))).join('|').slice(0, 160);
    observationCount += 1;
    window.elonChatGptNative.postMessage(JSON.stringify({
      type: 'command_result',
      action: 'research_voice_observation',
      ok: true,
      detail,
      adapterVersion: Number(window.__elonChatGptAdapterTargetVersion || 0),
      documentToken: String(window.__elonChatGptDocumentToken || '')
    }));
  }

  function activateVoiceWindow() {
    voiceWindowUntil = Math.max(voiceWindowUntil, Date.now() + (2 * 60 * 1000));
  }

  function lengthBucket(value) {
    const length = Math.max(0, Number(value) || 0);
    if (length === 0) return 'b0';
    if (length <= 256) return 'b1';
    if (length <= 1024) return 'b2';
    if (length <= 4096) return 'b3';
    if (length <= 16384) return 'b4';
    return 'b5';
  }

  function hostFamily(url) {
    const host = String(url.hostname || '').toLowerCase();
    if (url.origin === location.origin) return 'chatgpt_origin';
    if (host === 'chatgpt.com' || host.endsWith('.chatgpt.com')) return 'chatgpt_subdomain';
    if (host === 'openai.com' || host.endsWith('.openai.com')) return 'openai_subdomain';
    return 'other';
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
    return (url.pathname || '/')
      .split('/')
      .map(safeSegment)
      .join('/')
      .slice(0, 96) || '/';
  }

  function responseKind(response) {
    try {
      const value = String(response.headers.get('content-type') || '').toLowerCase();
      if (value.includes('json')) return 'json';
      if (value.includes('sdp')) return 'session-description';
      if (value.startsWith('text/')) return 'text';
      return value ? 'other' : 'unknown';
    } catch (_) {
      return 'unknown';
    }
  }

  function safeResponseKeys(value) {
    if (!value || typeof value !== 'object' || Array.isArray(value)) return ['none'];
    return Object.keys(value)
      .filter((key) => /^[A-Za-z][A-Za-z0-9_-]{0,39}$/.test(key))
      .map((key) => sensitiveKeyHint.test(key) ? 'ephemeral-field' : key.toLowerCase())
      .filter((key, index, all) => all.indexOf(key) === index)
      .sort()
      .slice(0, 10);
  }

  function requestInfo(input, init) {
    try {
      const rawUrl = typeof input === 'string' || input instanceof URL
        ? String(input)
        : String(input && input.url || '');
      const url = new URL(rawUrl, location.href);
      const method = String(init && init.method || input && input.method || 'GET').toUpperCase();
      const family = hostFamily(url);
      const active = Date.now() <= voiceWindowUntil;
      const candidate = family !== 'other' && (active || voicePathHint.test(url.pathname));
      return candidate ? { url, method, family, path: safePath(url) } : null;
    } catch (_) {
      return null;
    }
  }

  function observeResponseShape(info, response) {
    if (!response || responseKind(response) !== 'json' || typeof response.clone !== 'function') return;
    Promise.resolve()
      .then(() => response.clone().json())
      .then((body) => emit([
        'network-shape', info.family, info.path, safeResponseKeys(body).join('.')
      ]))
      .catch(() => {});
  }

  function installFetchObserver() {
    if (typeof window.fetch !== 'function') return;
    const originalFetch = window.fetch;
    window.fetch = function (input, init) {
      const info = requestInfo(input, init || {});
      const requestStartedAt = Date.now();
      if (info) emit(['network-start', info.method, info.family, info.path]);
      return originalFetch.apply(this, arguments).then((response) => {
        if (info) {
          emit([
            'network-end', info.method, info.family, info.path,
            String(response.status || 0), responseKind(response),
            lengthBucket(Date.now() - requestStartedAt)
          ]);
          observeResponseShape(info, response);
        }
        return response;
      }, (error) => {
        if (info) emit(['network-error', info.method, info.family, info.path, safeError(error)]);
        throw error;
      });
    };
  }

  function xhrResponseKind(xhr) {
    try {
      const value = String(xhr.getResponseHeader('content-type') || '').toLowerCase();
      if (value.includes('json')) return 'json';
      if (value.includes('sdp')) return 'session-description';
      if (value.startsWith('text/')) return 'text';
      return value ? 'other' : 'unknown';
    } catch (_) {
      return 'unknown';
    }
  }

  function installXhrObserver() {
    const NativeXhr = window.XMLHttpRequest;
    if (!NativeXhr || !NativeXhr.prototype) return;
    const originalOpen = NativeXhr.prototype.open;
    const originalSend = NativeXhr.prototype.send;
    const requests = new WeakMap();
    NativeXhr.prototype.open = function (method, url) {
      requests.set(this, requestInfo(String(url || ''), { method }));
      return originalOpen.apply(this, arguments);
    };
    NativeXhr.prototype.send = function () {
      const info = requests.get(this);
      const requestStartedAt = Date.now();
      if (info) {
        emit(['network-start', info.method, info.family, info.path]);
        this.addEventListener('loadend', () => emit([
          'network-end', info.method, info.family, info.path,
          String(this.status || 0), xhrResponseKind(this),
          lengthBucket(Date.now() - requestStartedAt)
        ]));
        this.addEventListener('error', () => emit([
          'network-error', info.method, info.family, info.path, 'error'
        ]));
      }
      return originalSend.apply(this, arguments);
    };
  }

  function installSocketObserver() {
    const NativeSocket = window.WebSocket;
    if (typeof NativeSocket !== 'function') return;
    function ObservedSocket(url, protocols) {
      const info = requestInfo(String(url || ''), { method: 'WS' });
      const socket = protocols === undefined
        ? new NativeSocket(url)
        : new NativeSocket(url, protocols);
      if (info) {
        emit(['socket-start', info.family, info.path]);
        socket.addEventListener('open', () => emit(['socket-open', info.family, info.path]));
        socket.addEventListener('close', () => emit(['socket-close', info.family, info.path]));
        socket.addEventListener('error', () => emit(['socket-error', info.family, info.path]));
      }
      return socket;
    }
    ObservedSocket.prototype = NativeSocket.prototype;
    try { Object.setPrototypeOf(ObservedSocket, NativeSocket); } catch (_) {}
    window.WebSocket = ObservedSocket;
  }

  function safeError(error) {
    const name = String(error && error.name || 'error').toLowerCase();
    return /^(aborterror|notallowederror|notfounderror|notreadableerror|securityerror|typeerror)$/.test(name)
      ? name
      : 'error';
  }

  function mediaSummary(stream) {
    try {
      const tracks = stream && typeof stream.getTracks === 'function' ? stream.getTracks() : [];
      const audioCount = tracks.filter((track) => track && track.kind === 'audio').length;
      const videoCount = tracks.filter((track) => track && track.kind === 'video').length;
      return `a${Math.min(audioCount, 9)}v${Math.min(videoCount, 9)}`;
    } catch (_) {
      return 'a0v0';
    }
  }

  function installMediaObserver() {
    const mediaDevices = navigator.mediaDevices;
    if (!mediaDevices || typeof mediaDevices.getUserMedia !== 'function') return;
    const original = mediaDevices.getUserMedia.bind(mediaDevices);
    mediaDevices.getUserMedia = function (constraints) {
      activateVoiceWindow();
      const requested = constraints && constraints.audio ? 'audio' : 'other';
      emit(['media-request', requested]);
      return original(constraints).then((stream) => {
        emit(['media-granted', mediaSummary(stream)]);
        return stream;
      }, (error) => {
        emit(['media-error', safeError(error)]);
        throw error;
      });
    };
  }

  function descriptionSummary(description) {
    const type = safePart(description && description.type, 'unknown');
    const sdpLength = description && typeof description.sdp === 'string' ? description.sdp.length : 0;
    return [type, lengthBucket(sdpLength)];
  }

  function observePeerConnection(peer) {
    activateVoiceWindow();
    emit(['peer-created']);
    const eventNames = ['connectionstatechange', 'iceconnectionstatechange', 'signalingstatechange', 'track', 'datachannel'];
    eventNames.forEach((eventName) => {
      try {
        peer.addEventListener(eventName, (event) => {
          if (eventName === 'connectionstatechange') emit(['peer-connection', peer.connectionState || 'unknown']);
          if (eventName === 'iceconnectionstatechange') emit(['peer-ice', peer.iceConnectionState || 'unknown']);
          if (eventName === 'signalingstatechange') emit(['peer-signaling', peer.signalingState || 'unknown']);
          if (eventName === 'track') emit(['peer-track', event && event.track && event.track.kind || 'unknown']);
          if (eventName === 'datachannel') emit(['peer-data-channel', 'remote']);
        });
      } catch (_) {}
    });

    wrapPeerMethod(peer, 'createOffer', () => emit(['peer-create-offer']));
    wrapPeerMethod(peer, 'createAnswer', () => emit(['peer-create-answer']));
    wrapPeerMethod(peer, 'setLocalDescription', (args) => {
      emit(['peer-local-description'].concat(descriptionSummary(args[0])));
    });
    wrapPeerMethod(peer, 'setRemoteDescription', (args) => {
      emit(['peer-remote-description'].concat(descriptionSummary(args[0])));
    });
    wrapPeerMethod(peer, 'createDataChannel', () => emit(['peer-data-channel', 'local']));
  }

  function wrapPeerMethod(peer, name, before) {
    if (!peer || typeof peer[name] !== 'function') return;
    try {
      const original = peer[name].bind(peer);
      peer[name] = function () {
        before(Array.prototype.slice.call(arguments));
        return original.apply(null, arguments);
      };
    } catch (_) {}
  }

  function installPeerObserver() {
    const NativePeerConnection = window.RTCPeerConnection;
    if (typeof NativePeerConnection !== 'function') return;
    function ObservedPeerConnection(configuration, constraints) {
      const peer = new NativePeerConnection(configuration, constraints);
      observePeerConnection(peer);
      return peer;
    }
    ObservedPeerConnection.prototype = NativePeerConnection.prototype;
    try { Object.setPrototypeOf(ObservedPeerConnection, NativePeerConnection); } catch (_) {}
    window.RTCPeerConnection = ObservedPeerConnection;
  }

  installFetchObserver();
  installXhrObserver();
  installSocketObserver();
  installMediaObserver();
  installPeerObserver();

  window.__elonChatGptRealtimeVoiceResearch = Object.freeze({
    version: 1,
    activate: activateVoiceWindow,
    snapshot: function () {
      return Object.freeze({
        active: Date.now() <= voiceWindowUntil,
        observations: observationCount,
        expiresAt
      });
    }
  });
  emit(['observer-ready']);
})();
