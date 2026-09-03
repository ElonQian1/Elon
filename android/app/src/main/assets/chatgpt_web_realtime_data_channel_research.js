(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptRealtimeDataChannelResearch = Object.freeze(api);
})(typeof window === 'object' ? window : null, function () {
  'use strict';

  const observed = typeof WeakSet === 'function' ? new WeakSet() : null;
  const sensitiveKey = /(token|secret|credential|authorization|cookie|proof|sdp|candidate)/i;
  const eventType = /^(?:session|input_audio|conversation|response|rate_limits|transcript|speech|audio|error)[a-z0-9._-]{0,72}$/;

  function lengthBucket(value) {
    const length = Math.max(0, Number(value) || 0);
    if (length === 0) return 'b0';
    if (length <= 256) return 'b1';
    if (length <= 1024) return 'b2';
    if (length <= 4096) return 'b3';
    if (length <= 16384) return 'b4';
    return 'b5';
  }

  function safeKeys(value) {
    if (!value || typeof value !== 'object' || Array.isArray(value)) return 'none';
    const keys = Object.keys(value)
      .filter((key) => /^[A-Za-z][A-Za-z0-9_-]{0,39}$/.test(key))
      .map((key) => sensitiveKey.test(key) ? 'sensitive-field' : key.toLowerCase())
      .filter((key, index, all) => all.indexOf(key) === index)
      .sort()
      .slice(0, 12);
    return keys.length ? keys.join('.') : 'none';
  }

  function safeEventType(value) {
    const normalized = String(value || '').trim().toLowerCase();
    return eventType.test(normalized) ? normalized : 'other';
  }

  function payloadShape(data) {
    if (typeof data === 'string') {
      let parsed = null;
      if (data.length <= 262144) {
        try { parsed = JSON.parse(data); } catch (_) {}
      }
      if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
        return ['text', lengthBucket(data.length), 'none', 'other'];
      }
      const nestedType = parsed.event && parsed.event.type ||
        parsed.item && parsed.item.type ||
        parsed.response && parsed.response.type;
      return [
        'json',
        lengthBucket(data.length),
        safeKeys(parsed),
        safeEventType(parsed.type || nestedType)
      ];
    }
    if (typeof Blob !== 'undefined' && data instanceof Blob) {
      return ['blob', lengthBucket(data.size), 'none', 'other'];
    }
    if (typeof ArrayBuffer !== 'undefined' && data instanceof ArrayBuffer) {
      return ['bytes', lengthBucket(data.byteLength), 'none', 'other'];
    }
    if (typeof ArrayBuffer !== 'undefined' && ArrayBuffer.isView && ArrayBuffer.isView(data)) {
      return ['bytes', lengthBucket(data.byteLength), 'none', 'other'];
    }
    return ['other', 'unknown', 'none', 'other'];
  }

  function sessionProfile(body) {
    if (typeof FormData === 'undefined' || !(body instanceof FormData)) return [];
    let profile = [];
    try {
      body.forEach((value, key) => {
        if (profile.length || String(key || '').toLowerCase() !== 'session' ||
            typeof value !== 'string') return;
        let parsed;
        try { parsed = JSON.parse(value); } catch (_) { return; }
        const rawMode = String(parsed && parsed.chat_mode || '').toLowerCase();
        const mode = /^(?:voice|dictation|transcribe|conversation|chat)$/.test(rawMode)
          ? rawMode
          : rawMode ? 'other' : 'none';
        profile = [
          `chat-mode-${mode}`,
          `input-transcription-${parsed && parsed.input_audio_transcription ? 'present' : 'absent'}`,
          `turn-detection-${parsed && parsed.turn_detection ? 'present' : 'absent'}`,
          `modalities-${Array.isArray(parsed && parsed.modalities) ? parsed.modalities.length : 0}`
        ];
      });
    } catch (_) {
      return [];
    }
    return profile;
  }

  function observe(channel, ownership, emit) {
    if (!channel || typeof emit !== 'function') return channel;
    if (observed && observed.has(channel)) return channel;
    if (observed) observed.add(channel);
    const side = ownership === 'remote' ? 'remote' : 'local';
    emit(['data-channel-bind', side, channel.negotiated === true ? 'negotiated' : 'in-band']);
    if (typeof channel.send === 'function') {
      try {
        const originalSend = channel.send.bind(channel);
        channel.send = function (data) {
          emit(['data-channel-send', side].concat(payloadShape(data)));
          return originalSend(data);
        };
      } catch (_) {}
    }
    if (typeof channel.addEventListener === 'function') {
      try {
        channel.addEventListener('message', (event) => {
          emit(['data-channel-message', side].concat(payloadShape(event && event.data)));
        });
        channel.addEventListener('open', () => emit(['data-channel-open', side]));
        channel.addEventListener('close', () => emit(['data-channel-close', side]));
        channel.addEventListener('error', () => emit(['data-channel-error', side]));
      } catch (_) {}
    }
    return channel;
  }

  return Object.freeze({ observe, payloadShape, sessionProfile });
});
